//! Cranelift JIT backend.
//!
//! Compiles a [`Graph`] to native code: one Cranelift block per state, each
//! transition guard lowered to a short-circuit chain of branches, each action
//! to a call. The control flow (state dispatch, guard branching, the `finished`
//! flag reads, the `m`-is-none test) is emitted as machine code; the parts that
//! touch the trie (zipper moves, byte-slice compares, `prefix_of`, `argmin`)
//! are `extern "C"` helpers operating on raw pointers — the same raw-pointer
//! model as the interpreter, with no `RefCell`.
//!
//! The JIT must produce identical results to [`Graph::run`]; that is the
//! contract the tests check.

use core::ffi::c_void;

use cranelift_codegen::ir::{types, AbiParam, InstBuilder, MemFlags, Signature, Value};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

use pathmap::alloc::Allocator;
use pathmap::zipper::{ReadZipperUntracked, Zipper, ZipperMoving};

use crate::{cmp_ok, common_prefix, descend_or_next, next_at, Cmp, Cond, Graph, Ref, Transition};

// ---------------------------------------------------------------------------
// Runtime context shared between the JIT'd code and the helpers
// ---------------------------------------------------------------------------

/// Passed by pointer to the compiled function and to every helper. `#[repr(C)]`
/// so the JIT can address its fields by fixed offset.
#[repr(C)]
pub struct JitCtx {
    /// array of `num_sources` opaque zipper pointers.
    zippers: *const *mut c_void,
    /// array of `num_sources` finished flags (1 = finished/None).
    finished: *mut u8,
    /// current `m`: a source index, or -1 for None.
    m: i64,
    sink: *mut Vec<Vec<u8>>,
}

// Field byte offsets (64-bit, repr(C)).
const OFF_FINISHED: i32 = 8;
const OFF_M: i32 = 16;

impl JitCtx {
    unsafe fn zipper<'a, 'p, V, A>(&self, i: i64) -> *mut ReadZipperUntracked<'a, 'p, V, A>
    where
        V: Clone + Send + Sync,
        A: Allocator,
    {
        unsafe { *self.zippers.add(i as usize) as *mut ReadZipperUntracked<'a, 'p, V, A> }
    }
    unsafe fn fin(&self, i: i64) -> bool {
        unsafe { *self.finished.add(i as usize) != 0 }
    }
    unsafe fn set_fin(&self, i: i64, v: bool) {
        unsafe { *self.finished.add(i as usize) = v as u8 }
    }
}

// ---------------------------------------------------------------------------
// extern "C" helpers (monomorphic per <V, A>)
// ---------------------------------------------------------------------------

unsafe fn path<'c, V, A>(ctx: &'c JitCtx, i: i64) -> &'c [u8]
where
    V: Clone + Send + Sync + Unpin + 'c,
    A: Allocator + 'c,
{
    unsafe { (*ctx.zipper::<V, A>(i)).path() }
}

extern "C" fn h_cond_ineq<V, A>(ctx: *const JitCtx, op: i64, a: i64, b: i64) -> i8
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &*ctx };
    cmp_ok(Cmp::from_i64(op), unsafe { path::<V, A>(c, a) }, unsafe {
        path::<V, A>(c, b)
    }) as i8
}

extern "C" fn h_cond_op_or_not<V, A>(ctx: *const JitCtx, op: i64, a: i64, b: i64) -> i8
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &*ctx };
    let r = unsafe { c.fin(a) }
        || cmp_ok(Cmp::from_i64(op), unsafe { path::<V, A>(c, a) }, unsafe {
            path::<V, A>(c, b)
        });
    r as i8
}

extern "C" fn h_cond_ne_if_value<V, A>(ctx: *const JitCtx, op: i64, a: i64, b: i64) -> i8
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &*ctx };
    let r = unsafe { c.fin(a) }
        || (cmp_ok(Cmp::from_i64(op), unsafe { path::<V, A>(c, a) }, unsafe {
            path::<V, A>(c, b)
        }) || !unsafe { (*c.zipper::<V, A>(a)).is_val() });
    r as i8
}

extern "C" fn h_cond_op_or_eq_not_value<V, A>(ctx: *const JitCtx, op: i64, a: i64, b: i64) -> i8
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &*ctx };
    let pa = unsafe { path::<V, A>(c, a) };
    let pb = unsafe { path::<V, A>(c, b) };
    let r = unsafe { c.fin(a) }
        || cmp_ok(Cmp::from_i64(op), pa, pb)
        || (pa == pb && !unsafe { (*c.zipper::<V, A>(a)).is_val() });
    r as i8
}

extern "C" fn h_cond_is_value<V, A>(ctx: *const JitCtx, a: i64) -> i8
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &*ctx };
    unsafe { (*c.zipper::<V, A>(a)).is_val() as i8 }
}

/// `rhs_kind`: 0 = source `rhs_idx`, 1 = `m`.
extern "C" fn h_cond_prefix<V, A>(ctx: *const JitCtx, a: i64, rhs_kind: i64, rhs_idx: i64) -> i8
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &*ctx };
    if unsafe { c.fin(a) } {
        return 0;
    }
    let rj = if rhs_kind == 1 {
        if c.m < 0 {
            return 0;
        }
        c.m
    } else {
        if unsafe { c.fin(rhs_idx) } {
            return 0;
        }
        rhs_idx
    };
    let pa = unsafe { path::<V, A>(c, a) };
    let pr = unsafe { path::<V, A>(c, rj) };
    pr.starts_with(pa) as i8
}

/// A `define` descriptor kept alive by the [`Jit`] artifact; its address is
/// baked into the compiled code as a constant.
pub struct DefineDesc {
    groups: Vec<Vec<usize>>,
}

extern "C" fn h_act_define<V, A>(ctx: *mut JitCtx, desc: *const DefineDesc)
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &mut *ctx };
    let d = unsafe { &*desc };
    let group_winner = |g: &[usize]| -> Option<usize> {
        g.iter()
            .copied()
            .filter(|&i| !unsafe { c.fin(i as i64) })
            .max_by(|&x, &y| unsafe { path::<V, A>(c, x as i64).cmp(path::<V, A>(c, y as i64)) })
    };
    let chosen = if d.groups.len() == 1 {
        group_winner(&d.groups[0])
    } else {
        d.groups
            .iter()
            .filter_map(|g| group_winner(g))
            .min_by(|&x, &y| unsafe { path::<V, A>(c, x as i64).cmp(path::<V, A>(c, y as i64)) })
    };
    c.m = match chosen {
        Some(i) => i as i64,
        None => -1,
    };
}

extern "C" fn h_act_push<V, A>(ctx: *mut JitCtx, i: i64)
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &mut *ctx };
    let p = unsafe { path::<V, A>(c, i) }.to_vec();
    unsafe { (*c.sink).push(p) };
}

extern "C" fn h_act_descend<V, A>(ctx: *mut JitCtx, i: i64)
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &mut *ctx };
    let active = unsafe { descend_or_next(c.zipper::<V, A>(i)) };
    unsafe { c.set_fin(i, !active) };
}

extern "C" fn h_act_next_i<V, A>(ctx: *mut JitCtx, i: i64, ds_ptr: *const usize, ds_len: i64)
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &mut *ctx };
    let ds = unsafe { core::slice::from_raw_parts(ds_ptr, ds_len as usize) };
    let pi = unsafe { path::<V, A>(c, i) };
    let level = ds
        .iter()
        .map(|&d| common_prefix(pi, unsafe { path::<V, A>(c, d as i64) }))
        .max()
        .unwrap();
    let active = unsafe { next_at(c.zipper::<V, A>(i), level) };
    unsafe { c.set_fin(i, !active) };
}

extern "C" fn h_act_next_i_var<V, A>(ctx: *mut JitCtx, i: i64)
where
    V: Clone + Send + Sync + Unpin,
    A: Allocator,
{
    let c = unsafe { &mut *ctx };
    let mj = c.m;
    debug_assert!(mj >= 0, "next_i_var with m = None");
    let level = common_prefix(unsafe { path::<V, A>(c, i) }, unsafe { path::<V, A>(c, mj) });
    let active = unsafe { next_at(c.zipper::<V, A>(i), level) };
    unsafe { c.set_fin(i, !active) };
}

extern "C" fn h_act_end(ctx: *mut JitCtx, i: i64) {
    let c = unsafe { &mut *ctx };
    unsafe { c.set_fin(i, true) };
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

/// Addresses of the monomorphized helpers for a concrete `<V, A>`.
struct Helpers {
    cond_ineq: *const u8,
    cond_op_or_not: *const u8,
    cond_ne_if_value: *const u8,
    cond_op_or_eq_not_value: *const u8,
    cond_is_value: *const u8,
    cond_prefix: *const u8,
    act_define: *const u8,
    act_push: *const u8,
    act_descend: *const u8,
    act_next_i: *const u8,
    act_next_i_var: *const u8,
    act_end: *const u8,
}

impl Helpers {
    fn for_type<V, A>() -> Helpers
    where
        V: Clone + Send + Sync + Unpin + 'static,
        A: Allocator + 'static,
    {
        Helpers {
            cond_ineq: h_cond_ineq::<V, A> as *const u8,
            cond_op_or_not: h_cond_op_or_not::<V, A> as *const u8,
            cond_ne_if_value: h_cond_ne_if_value::<V, A> as *const u8,
            cond_op_or_eq_not_value: h_cond_op_or_eq_not_value::<V, A> as *const u8,
            cond_is_value: h_cond_is_value::<V, A> as *const u8,
            cond_prefix: h_cond_prefix::<V, A> as *const u8,
            act_define: h_act_define::<V, A> as *const u8,
            act_push: h_act_push::<V, A> as *const u8,
            act_descend: h_act_descend::<V, A> as *const u8,
            act_next_i: h_act_next_i::<V, A> as *const u8,
            act_next_i_var: h_act_next_i_var::<V, A> as *const u8,
            act_end: h_act_end as *const u8,
        }
    }
}

/// A compiled state machine. Owns the executable code and the side tables whose
/// addresses are referenced by it.
pub struct Jit {
    module: Option<JITModule>,
    func: *const u8,
    num_sources: usize,
    pub source_names: Vec<String>,
    _defines: Vec<Box<DefineDesc>>,
    _ds_lists: Vec<Box<[usize]>>,
}

impl Jit {
    pub fn num_sources(&self) -> usize {
        self.num_sources
    }

    /// Compile `graph` to native code, with helpers monomorphized for the
    /// concrete zipper value/allocator types `<V, A>` it will run on.
    pub fn compile<V, A>(graph: &Graph) -> Jit
    where
        V: Clone + Send + Sync + Unpin + 'static,
        A: Allocator + 'static,
    {
        let helpers = Helpers::for_type::<V, A>();

        let mut flags = settings::builder();
        flags.set("opt_level", "speed").unwrap();
        let isa_flags = settings::Flags::new(flags);
        let isa = cranelift_native::builder()
            .expect("host machine is not supported")
            .finish(isa_flags)
            .expect("failed to build isa");

        let mut jb = JITBuilder::with_isa(isa, default_libcall_names());
        register_symbols(&mut jb, &helpers);
        let mut module = JITModule::new(jb);

        let ptr = module.target_config().pointer_type();

        // Import each helper as an external function.
        let mut imports = Imports::declare(&mut module, ptr);

        let mut ctx = module.make_context();
        ctx.func.signature = Signature {
            params: vec![AbiParam::new(ptr)], // *mut JitCtx
            returns: vec![],
            call_conv: module.target_config().default_call_conv,
        };

        let mut fbctx = FunctionBuilderContext::new();
        let mut defines: Vec<Box<DefineDesc>> = Vec::new();
        let mut ds_lists: Vec<Box<[usize]>> = Vec::new();

        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            {
                let mut cg = Codegen {
                    b: &mut b,
                    module: &mut module,
                    imports: &mut imports,
                    ptr,
                    defines: &mut defines,
                    ds_lists: &mut ds_lists,
                };
                cg.emit(graph);
            }
            b.finalize();
        }

        let id = module
            .declare_function("synth_jit_main", Linkage::Export, &ctx.func.signature)
            .unwrap();
        module.define_function(id, &mut ctx).unwrap();
        module.clear_context(&mut ctx);
        module.finalize_definitions().unwrap();
        let func = module.get_finalized_function(id);

        Jit {
            module: Some(module),
            func,
            num_sources: graph.num_sources(),
            source_names: graph.source_names.clone(),
            _defines: defines,
            _ds_lists: ds_lists,
        }
    }

    /// Run the compiled machine. `zippers[i]` corresponds to `source_names[i]`.
    ///
    /// # Safety
    /// `<V, A>` must match the types passed to [`compile`](Jit::compile), and
    /// every zipper pointer must be valid and exclusively borrowed for the call.
    pub unsafe fn run<V, A>(
        &self,
        zippers: &[*mut ReadZipperUntracked<V, A>],
        sink: &mut Vec<Vec<u8>>,
    ) where
        V: Clone + Send + Sync,
        A: Allocator,
    {
        assert_eq!(zippers.len(), self.num_sources, "wrong number of zippers");
        let zptrs: Vec<*mut c_void> = zippers.iter().map(|&z| z as *mut c_void).collect();
        let mut finished = vec![1u8; self.num_sources];
        let mut ctx = JitCtx {
            zippers: zptrs.as_ptr(),
            finished: finished.as_mut_ptr(),
            m: -1,
            sink: sink as *mut _,
        };
        let f: extern "C" fn(*mut JitCtx) = unsafe { core::mem::transmute(self.func) };
        f(&mut ctx);
    }
}

impl Drop for Jit {
    fn drop(&mut self) {
        if let Some(module) = self.module.take() {
            // Safety: no compiled function is executing once `Jit` is dropped.
            unsafe { module.free_memory() };
        }
    }
}

fn register_symbols(jb: &mut JITBuilder, h: &Helpers) {
    jb.symbol("h_cond_ineq", h.cond_ineq as *const u8);
    jb.symbol("h_cond_op_or_not", h.cond_op_or_not as *const u8);
    jb.symbol("h_cond_ne_if_value", h.cond_ne_if_value as *const u8);
    jb.symbol("h_cond_op_or_eq_not_value", h.cond_op_or_eq_not_value as *const u8);
    jb.symbol("h_cond_is_value", h.cond_is_value as *const u8);
    jb.symbol("h_cond_prefix", h.cond_prefix as *const u8);
    jb.symbol("h_act_define", h.act_define as *const u8);
    jb.symbol("h_act_push", h.act_push as *const u8);
    jb.symbol("h_act_descend", h.act_descend as *const u8);
    jb.symbol("h_act_next_i", h.act_next_i as *const u8);
    jb.symbol("h_act_next_i_var", h.act_next_i_var as *const u8);
    jb.symbol("h_act_end", h.act_end as *const u8);
}

/// FuncIds of the imported helpers.
struct Imports {
    cond_ineq: cranelift_module::FuncId,
    cond_op_or_not: cranelift_module::FuncId,
    cond_ne_if_value: cranelift_module::FuncId,
    cond_op_or_eq_not_value: cranelift_module::FuncId,
    cond_is_value: cranelift_module::FuncId,
    cond_prefix: cranelift_module::FuncId,
    act_define: cranelift_module::FuncId,
    act_push: cranelift_module::FuncId,
    act_descend: cranelift_module::FuncId,
    act_next_i: cranelift_module::FuncId,
    act_next_i_var: cranelift_module::FuncId,
    act_end: cranelift_module::FuncId,
}

impl Imports {
    fn declare(module: &mut JITModule, ptr: types::Type) -> Imports {
        let i64t = types::I64;
        let i8t = types::I8;
        let cc = module.target_config().default_call_conv;
        let sig = |params: Vec<types::Type>, ret: Option<types::Type>| {
            let mut s = Signature::new(cc);
            for p in params {
                s.params.push(AbiParam::new(p));
            }
            if let Some(r) = ret {
                s.returns.push(AbiParam::new(r));
            }
            s
        };
        let imp = |module: &mut JITModule, name: &str, s: Signature| {
            module.declare_function(name, Linkage::Import, &s).unwrap()
        };
        Imports {
            cond_ineq: imp(module, "h_cond_ineq", sig(vec![ptr, i64t, i64t, i64t], Some(i8t))),
            cond_op_or_not: imp(module, "h_cond_op_or_not", sig(vec![ptr, i64t, i64t, i64t], Some(i8t))),
            cond_ne_if_value: imp(module, "h_cond_ne_if_value", sig(vec![ptr, i64t, i64t, i64t], Some(i8t))),
            cond_op_or_eq_not_value: imp(module, "h_cond_op_or_eq_not_value", sig(vec![ptr, i64t, i64t, i64t], Some(i8t))),
            cond_is_value: imp(module, "h_cond_is_value", sig(vec![ptr, i64t], Some(i8t))),
            cond_prefix: imp(module, "h_cond_prefix", sig(vec![ptr, i64t, i64t, i64t], Some(i8t))),
            act_define: imp(module, "h_act_define", sig(vec![ptr, ptr], None)),
            act_push: imp(module, "h_act_push", sig(vec![ptr, i64t], None)),
            act_descend: imp(module, "h_act_descend", sig(vec![ptr, i64t], None)),
            act_next_i: imp(module, "h_act_next_i", sig(vec![ptr, i64t, ptr, i64t], None)),
            act_next_i_var: imp(module, "h_act_next_i_var", sig(vec![ptr, i64t], None)),
            act_end: imp(module, "h_act_end", sig(vec![ptr, i64t], None)),
        }
    }
}

struct Codegen<'a, 'b> {
    b: &'a mut FunctionBuilder<'b>,
    module: &'a mut JITModule,
    imports: &'a mut Imports,
    ptr: types::Type,
    defines: &'a mut Vec<Box<DefineDesc>>,
    ds_lists: &'a mut Vec<Box<[usize]>>,
}

impl Codegen<'_, '_> {
    fn call(&mut self, id: cranelift_module::FuncId, args: &[Value]) -> Option<Value> {
        let fref = self.module.declare_func_in_func(id, self.b.func);
        let call = self.b.ins().call(fref, args);
        self.b.inst_results(call).first().copied()
    }

    fn iconst(&mut self, v: i64) -> Value {
        self.b.ins().iconst(types::I64, v)
    }

    /// Load `ctx.finished[i]` (i8, nonzero = finished).
    fn load_finished(&mut self, ctxv: Value, i: usize) -> Value {
        let fin_ptr = self
            .b
            .ins()
            .load(self.ptr, MemFlags::new(), ctxv, OFF_FINISHED);
        self.b
            .ins()
            .load(types::I8, MemFlags::new(), fin_ptr, i as i32)
    }

    fn load_m(&mut self, ctxv: Value) -> Value {
        self.b.ins().load(types::I64, MemFlags::new(), ctxv, OFF_M)
    }

    /// Emit a value that is nonzero iff the condition holds.
    fn emit_cond(&mut self, ctxv: Value, c: &Cond) -> Value {
        match *c {
            Cond::Finished(a) => self.load_finished(ctxv, a),
            Cond::NotFinished(a) => {
                let f = self.load_finished(ctxv, a);
                self.b.ins().icmp_imm(IntCC::Equal, f, 0)
            }
            Cond::VarNone => {
                let m = self.load_m(ctxv);
                self.b.ins().icmp_imm(IntCC::SignedLessThan, m, 0)
            }
            Cond::Ineq(op, a, b) => {
                let args = [ctxv, self.iconst(op.to_i64()), self.iconst(a as i64), self.iconst(b as i64)];
                self.call(self.imports.cond_ineq, &args).unwrap()
            }
            Cond::OpOrNot(op, a, b) => {
                let args = [ctxv, self.iconst(op.to_i64()), self.iconst(a as i64), self.iconst(b as i64)];
                self.call(self.imports.cond_op_or_not, &args).unwrap()
            }
            Cond::NeIfValue(op, a, b) => {
                let args = [ctxv, self.iconst(op.to_i64()), self.iconst(a as i64), self.iconst(b as i64)];
                self.call(self.imports.cond_ne_if_value, &args).unwrap()
            }
            Cond::OpOrEqNotValue(op, a, b) => {
                let args = [ctxv, self.iconst(op.to_i64()), self.iconst(a as i64), self.iconst(b as i64)];
                self.call(self.imports.cond_op_or_eq_not_value, &args).unwrap()
            }
            Cond::IsValue(a) => {
                let args = [ctxv, self.iconst(a as i64)];
                self.call(self.imports.cond_is_value, &args).unwrap()
            }
            Cond::NotValue(a) => {
                let args = [ctxv, self.iconst(a as i64)];
                let v = self.call(self.imports.cond_is_value, &args).unwrap();
                self.b.ins().icmp_imm(IntCC::Equal, v, 0)
            }
            Cond::PrefixOf(a, rhs) => {
                let (kind, idx) = ref_args(rhs);
                let args = [ctxv, self.iconst(a as i64), self.iconst(kind), self.iconst(idx)];
                self.call(self.imports.cond_prefix, &args).unwrap()
            }
            Cond::NotPrefixOf(a, rhs) => {
                let (kind, idx) = ref_args(rhs);
                let args = [ctxv, self.iconst(a as i64), self.iconst(kind), self.iconst(idx)];
                let v = self.call(self.imports.cond_prefix, &args).unwrap();
                self.b.ins().icmp_imm(IntCC::Equal, v, 0)
            }
        }
    }

    fn emit_actions(&mut self, ctxv: Value, t: &Transition) {
        if let Some(groups) = &t.define {
            let desc = Box::new(DefineDesc {
                groups: groups.clone(),
            });
            let addr = (&*desc as *const DefineDesc) as i64;
            self.defines.push(desc);
            let p = self.b.ins().iconst(self.ptr, addr);
            self.call(self.imports.act_define, &[ctxv, p]);
        }
        for &i in &t.push {
            let a = self.iconst(i as i64);
            self.call(self.imports.act_push, &[ctxv, a]);
        }
        for &i in &t.descend {
            let a = self.iconst(i as i64);
            self.call(self.imports.act_descend, &[ctxv, a]);
        }
        for (src, ds) in &t.next_i {
            let boxed: Box<[usize]> = ds.clone().into_boxed_slice();
            let addr = boxed.as_ptr() as i64;
            let len = boxed.len() as i64;
            self.ds_lists.push(boxed);
            let i = self.iconst(*src as i64);
            let p = self.b.ins().iconst(self.ptr, addr);
            let l = self.iconst(len);
            self.call(self.imports.act_next_i, &[ctxv, i, p, l]);
        }
        for &src in &t.next_i_var {
            let i = self.iconst(src as i64);
            self.call(self.imports.act_next_i_var, &[ctxv, i]);
        }
        for &i in &t.end {
            let a = self.iconst(i as i64);
            self.call(self.imports.act_end, &[ctxv, a]);
        }
    }

    fn emit(&mut self, graph: &Graph) {
        let entry = self.b.create_block();
        self.b.append_block_params_for_function_params(entry);
        self.b.switch_to_block(entry);
        let ctxv = self.b.block_params(entry)[0];

        let n = graph.states.len();
        let state_blocks: Vec<_> = (0..n).map(|_| self.b.create_block()).collect();
        let exit = self.b.create_block();

        self.b.ins().jump(state_blocks[graph.init], &[]);

        for (s, transitions) in graph.states.iter().enumerate() {
            self.b.switch_to_block(state_blocks[s]);
            if transitions.is_empty() {
                self.b.ins().jump(exit, &[]);
                continue;
            }
            // chain the transitions: each falls through to the next on failure.
            let trans_entry: Vec<_> = transitions.iter().map(|_| self.b.create_block()).collect();
            self.b.ins().jump(trans_entry[0], &[]);

            for (ti, t) in transitions.iter().enumerate() {
                let fail = if ti + 1 < transitions.len() {
                    trans_entry[ti + 1]
                } else {
                    exit
                };
                self.b.switch_to_block(trans_entry[ti]);

                // guard: active flags, then `when` conditions.
                for &i in &t.active {
                    let f = self.load_finished(ctxv, i); // nonzero = finished
                    let cont = self.b.create_block();
                    // active means NOT finished: take `cont` when finished == 0.
                    self.b.ins().brif(f, fail, &[], cont, &[]);
                    self.b.switch_to_block(cont);
                }
                for c in &t.when {
                    let v = self.emit_cond(ctxv, c);
                    let cont = self.b.create_block();
                    self.b.ins().brif(v, cont, &[], fail, &[]);
                    self.b.switch_to_block(cont);
                }

                // all passed: run actions, jump to target state.
                self.emit_actions(ctxv, t);
                self.b.ins().jump(state_blocks[t.to], &[]);
            }
        }

        self.b.switch_to_block(exit);
        self.b.ins().return_(&[]);
        self.b.seal_all_blocks();
    }
}

fn ref_args(r: Ref) -> (i64, i64) {
    match r {
        Ref::Src(j) => (0, j as i64),
        Ref::M => (1, 0),
    }
}
