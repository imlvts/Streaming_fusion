//! Runtime (non-proc-macro) version of the trie state machine.
//!
//! Where `synth_macro` builds the graph at *compile* time and emits Rust that
//! the compiler turns into a dispatch loop, this crate builds the graph at
//! *run* time from a formula string and executes it. The execution model uses
//! **raw pointers** to the zippers and a parallel `finished` flag array instead
//! of `RefCell<ReadZipper>` / `Option<&RefCell<..>>` — the caller is
//! responsible for keeping the zippers valid and exclusively borrowed for the
//! duration of [`Graph::run`] (hence it is `unsafe`).
//!
//! [`run`](Graph::run) is the tree-walking interpreter. It is also the
//! correctness oracle for the Cranelift machine-code JIT in the [`jit`] module:
//! the JIT must produce identical output for the same graph and inputs.

pub mod formula;
pub mod jit;

use pathmap::zipper::{Zipper, ZipperMoving};

// ---------------------------------------------------------------------------
// Runtime IR
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Lt,
    Gt,
    Eq,
    Ne,
    Le,
    Ge,
}

impl Cmp {
    pub fn to_i64(self) -> i64 {
        match self {
            Cmp::Lt => 0,
            Cmp::Gt => 1,
            Cmp::Eq => 2,
            Cmp::Ne => 3,
            Cmp::Le => 4,
            Cmp::Ge => 5,
        }
    }
    pub fn from_i64(v: i64) -> Cmp {
        match v {
            0 => Cmp::Lt,
            1 => Cmp::Gt,
            2 => Cmp::Eq,
            3 => Cmp::Ne,
            4 => Cmp::Le,
            5 => Cmp::Ge,
            _ => unreachable!("bad Cmp encoding {v}"),
        }
    }
}

/// Operand for the right-hand side of a `prefix_of`: either a source or the
/// single defined var `m`.
#[derive(Clone, Copy, Debug)]
pub enum Ref {
    Src(usize),
    M,
}

/// A guard condition. Sources are referenced by index; `m` is the implicit
/// defined var. Mirrors the `Cond` subclasses in `trie_synth.py`.
#[derive(Debug)]
pub enum Cond {
    Ineq(Cmp, usize, usize),
    OpOrNot(Cmp, usize, usize),
    NeIfValue(Cmp, usize, usize),
    OpOrEqNotValue(Cmp, usize, usize),
    IsValue(usize),
    NotValue(usize),
    PrefixOf(usize, Ref),
    NotPrefixOf(usize, Ref),
    VarNone,
    Finished(usize),
    NotFinished(usize),
}

/// One transition out of a state. `to` is a resolved state id. Sinks are
/// implicit (there is a single sink), so `push` lists the sources to emit.
#[derive(Default, Debug)]
pub struct Transition {
    pub to: usize,
    pub active: Vec<usize>,
    pub when: Vec<Cond>,
    /// `m = ...`: a list of groups; one group -> argmax, many -> argmin of argmaxes.
    pub define: Option<Vec<Vec<usize>>>,
    pub push: Vec<usize>,
    pub descend: Vec<usize>,
    pub next_i: Vec<(usize, Vec<usize>)>,
    /// sources to advance towards `m` (`next_i_var`).
    pub next_i_var: Vec<usize>,
    pub end: Vec<usize>,
}

/// A complete, executable state machine.
pub struct Graph {
    /// source index -> variable name (sorted). The caller must supply zippers
    /// to [`run`](Graph::run) in this order.
    pub source_names: Vec<String>,
    pub init: usize,
    /// state id -> its outgoing transitions (priority order).
    pub states: Vec<Vec<Transition>>,
}

impl Graph {
    pub fn num_sources(&self) -> usize {
        self.source_names.len()
    }
}

// ---------------------------------------------------------------------------
// Raw-pointer zipper helpers (port of shim.rs, no RefCell)
// ---------------------------------------------------------------------------

/// Descend the first byte, or move to the next branch anywhere up the trie.
/// Returns `true` if the zipper is still positioned on something (active).
///
/// # Safety
/// `z` must point to a valid, exclusively-borrowed zipper.
pub(crate) unsafe fn descend_or_next<Z: ZipperMoving>(z: *mut Z) -> bool {
    let zr = unsafe { &mut *z };
    if !zr.descend_first_byte() {
        loop {
            if zr.to_next_sibling_byte() {
                break;
            }
            if !zr.ascend_byte() {
                return false;
            }
        }
    }
    true
}

/// Ascend to `level`, then advance to the next sibling branch. Returns `true`
/// if still active.
///
/// # Safety
/// `z` must point to a valid, exclusively-borrowed zipper.
pub(crate) unsafe fn next_at<Z: ZipperMoving>(z: *mut Z, level: usize) -> bool {
    let zr = unsafe { &mut *z };
    let plen = zr.path().len();
    debug_assert!(level < plen, "next level {level} out of range for path len {plen}");
    let to_ascend = plen - level - 1;
    zr.ascend(to_ascend);
    loop {
        if zr.to_next_sibling_byte() {
            break;
        }
        if !zr.ascend_byte() {
            return false;
        }
    }
    true
}

pub(crate) fn cmp_ok(c: Cmp, a: &[u8], b: &[u8]) -> bool {
    match c {
        Cmp::Lt => a < b,
        Cmp::Gt => a > b,
        Cmp::Eq => a == b,
        Cmp::Ne => a != b,
        Cmp::Le => a <= b,
        Cmp::Ge => a >= b,
    }
}

/// Length of the common prefix of `a` and `b` (port of `difference_level` /
/// `find_prefix_overlap`).
pub(crate) fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

// ---------------------------------------------------------------------------
// Interpreter
// ---------------------------------------------------------------------------

/// Receives the paths matched by [`Graph::run`].
///
/// Implemented for `Vec<Vec<u8>>` (collect owned copies) and for any
/// `FnMut(&[u8])` (stream without allocating). The `&[u8]` argument borrows
/// the emitting zipper's path buffer and is only valid for the duration of
/// the call.
pub trait Sink {
    fn push(&mut self, path: &[u8]);
}

impl Sink for Vec<Vec<u8>> {
    fn push(&mut self, path: &[u8]) {
        self.push(path.to_vec());
    }
}

impl<F: FnMut(&[u8])> Sink for F {
    fn push(&mut self, path: &[u8]) {
        self(path)
    }
}

/// Per-call execution state, kept in registers/stack rather than per-source
/// `RefCell`s. Generic over the zipper type: any `Zipper + ZipperMoving`
/// works as a source, e.g. `ReadZipperUntracked` (PathMap) or `ACTZipper`
/// (ArenaCompactTree).
struct Exec<'z, Z> {
    z: &'z [*mut Z],
    finished: Vec<bool>,
    m: Option<usize>,
}

impl<'z, Z> Exec<'z, Z>
where
    Z: Zipper + ZipperMoving,
{
    /// Borrow the path of source `i`. Reads the live zipper regardless of the
    /// `finished` flag (matching `make_ref` -> `&Some(&src)` in the codegen).
    ///
    /// # Safety: `z[i]` is valid per the `run` contract.
    fn path(&self, i: usize) -> &[u8] {
        unsafe { (*self.z[i]).path() }
    }

    fn is_val(&self, i: usize) -> bool {
        unsafe { (*self.z[i]).is_val() }
    }

    /// `prefix_of(lhs, rhs)`: `rhs.path().starts_with(lhs.path())`, false if
    /// either operand is absent.
    fn prefix(&self, lhs: usize, rhs: Ref) -> bool {
        if self.finished[lhs] {
            return false;
        }
        let rj = match rhs {
            Ref::Src(j) => {
                if self.finished[j] {
                    return false;
                }
                j
            }
            Ref::M => match self.m {
                Some(j) => j,
                None => return false,
            },
        };
        self.path(rj).starts_with(self.path(lhs))
    }

    fn eval_cond(&self, c: &Cond) -> bool {
        match *c {
            Cond::Ineq(op, a, b) => cmp_ok(op, self.path(a), self.path(b)),
            Cond::OpOrNot(op, a, b) => self.finished[a] || cmp_ok(op, self.path(a), self.path(b)),
            Cond::NeIfValue(op, a, b) => {
                self.finished[a] || (cmp_ok(op, self.path(a), self.path(b)) || !self.is_val(a))
            }
            Cond::OpOrEqNotValue(op, a, b) => {
                self.finished[a]
                    || cmp_ok(op, self.path(a), self.path(b))
                    || (self.path(a) == self.path(b) && !self.is_val(a))
            }
            Cond::IsValue(a) => self.is_val(a),
            Cond::NotValue(a) => !self.is_val(a),
            Cond::PrefixOf(a, rhs) => self.prefix(a, rhs),
            Cond::NotPrefixOf(a, rhs) => !self.prefix(a, rhs),
            Cond::VarNone => self.m.is_none(),
            Cond::Finished(a) => self.finished[a],
            Cond::NotFinished(a) => !self.finished[a],
        }
    }

    fn matches(&self, t: &Transition) -> bool {
        t.active.iter().all(|&i| !self.finished[i]) && t.when.iter().all(|c| self.eval_cond(c))
    }

    /// argmax over a group: the active source with the lexicographically
    /// greatest path (last one on ties, matching `max_by_key`).
    fn group_winner(&self, g: &[usize]) -> Option<usize> {
        g.iter()
            .copied()
            .filter(|&i| !self.finished[i])
            .max_by(|&x, &y| self.path(x).cmp(self.path(y)))
    }

    fn eval_define(&self, values: &[Vec<usize>]) -> Option<usize> {
        if values.len() == 1 {
            self.group_winner(&values[0])
        } else {
            values
                .iter()
                .filter_map(|g| self.group_winner(g))
                .min_by(|&x, &y| self.path(x).cmp(self.path(y)))
        }
    }

    /// Returns `true` if the transition advanced the machine (moved or
    /// finished a zipper) — used by the non-termination guard in `run`.
    fn exec<S: Sink + ?Sized>(&mut self, t: &Transition, sink: &mut S) -> bool {
        if let Some(values) = &t.define {
            self.m = self.eval_define(values);
        }
        for &i in &t.push {
            sink.push(self.path(i));
        }
        for &i in &t.descend {
            self.finished[i] = !unsafe { descend_or_next(self.z[i]) };
        }
        for (src, ds) in &t.next_i {
            let level = if ds.len() > 1 {
                ds.iter()
                    .map(|&d| common_prefix(self.path(*src), self.path(d)))
                    .max()
                    .unwrap()
            } else {
                common_prefix(self.path(*src), self.path(ds[0]))
            };
            self.finished[*src] = !unsafe { next_at(self.z[*src], level) };
        }
        for &src in &t.next_i_var {
            let mj = self.m.expect("next_i_var with m = None");
            let level = common_prefix(self.path(src), self.path(mj));
            self.finished[src] = !unsafe { next_at(self.z[src], level) };
        }
        for &i in &t.end {
            self.finished[i] = true;
        }
        !t.descend.is_empty()
            || !t.next_i.is_empty()
            || !t.next_i_var.is_empty()
            || !t.end.is_empty()
    }
}

impl Graph {
    /// Run the state machine. `zippers[i]` must correspond to `source_names[i]`
    /// and stay valid and exclusively borrowed for the call. Pushes matched
    /// paths into `sink` — a `Vec<Vec<u8>>` to collect them, or any
    /// `FnMut(&[u8])` to stream them without allocating (see [`Sink`]).
    ///
    /// Sources are any `Zipper + ZipperMoving`, e.g. PathMap's
    /// `ReadZipperUntracked` or ArenaCompactTree's `ACTZipper`.
    ///
    /// # Safety
    /// Every pointer in `zippers` must be valid, aligned, and not aliased
    /// elsewhere while this runs.
    pub unsafe fn run<Z, S>(&self, zippers: &[*mut Z], sink: &mut S)
    where
        Z: Zipper + ZipperMoving,
        S: Sink + ?Sized,
    {
        assert_eq!(zippers.len(), self.num_sources(), "wrong number of zippers");
        let mut exec = Exec {
            z: zippers,
            finished: vec![true; self.num_sources()],
            m: None,
        };

        let mut state = self.init;
        // Guard against a buggy non-terminating graph: total transition count
        // scales with input size, so instead count consecutive transitions
        // that don't advance any zipper. Real work always advances; a stuck
        // graph cycles through no-op transitions.
        let mut stalled: u64 = 0;
        loop {
            let mut moved = false;
            for t in &self.states[state] {
                if exec.matches(t) {
                    let progressed = exec.exec(t, sink);
                    state = t.to;
                    moved = true;
                    if progressed {
                        stalled = 0;
                    } else {
                        stalled += 1;
                        assert!(stalled < 1_000_000, "state machine stopped making progress");
                    }
                    break;
                }
            }
            if !moved {
                break;
            }
        }
    }
}
