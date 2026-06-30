//! Compile + expansion check for the `synth!` codegen paths that the `ctx()`
//! demo in `src/main.rs` does not exercise: `active`, `define` (all three
//! forms), `op_or_not`, `ne_if_value`, `op_or_eq_not_value`, `next_i_var`,
//! `var_none`, and `end`.

use std::cell::RefCell;

#[allow(unused_imports)]
use test_rs::shim::{
    argmax, argmin, descend_or_next, difference_level, is_val, next, path, prefix_of, PathMap,
    ReadZipperUntracked, Zipper, ZipperMoving,
};
use synth_macro::synth;

#[allow(unused_parens)]
fn main() {
    let __src_a: PathMap<Option<u32>> = PathMap::from_iter([("000", None)]);
    let a = RefCell::new(__src_a.read_zipper());
    let __src_b: PathMap<Option<u32>> = PathMap::from_iter([("000", None)]);
    let b = RefCell::new(__src_b.read_zipper());
    let __src_c: PathMap<Option<u32>> = PathMap::from_iter([("000", None)]);
    let c = RefCell::new(__src_c.read_zipper());
    let __src_d: PathMap<Option<u32>> = PathMap::from_iter([("000", None)]);
    let d = RefCell::new(__src_d.read_zipper());
    #[allow(unused_mut)]
    let mut r: Vec<Vec<u8>> = Vec::new();

    synth! {
        sources: a, b, c, d;
        sinks: r;
        init: s0;

        s0 -> s1: active = [a, c], descend = [a, b, c, d], define = m = argmin(argmax(a, b), c);
        s1 -> s2: define = m = argmax(a, b);
        s2 -> s3: define = m = a;
        s3 -> s4: op_or_not(a < b), ne_if_value(c >= d), op_or_eq_not_value(a != c);
        s4 -> s5: var_none(m), prefix_of(a, m);
        s5 -> s6: next_i_var = [c -> m], end = [a];
    }

    let _ = &r;
}
