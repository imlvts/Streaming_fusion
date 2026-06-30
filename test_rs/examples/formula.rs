//! End-to-end check of `synth_formula!`: the whole pipeline
//! (formula -> normalize -> create_state_machine -> dispatch loop) runs at
//! compile time. Same formula `((a | b) & c) - d` and input data as
//! src/main__example_output.rs. Expected result: 010, 101.

use std::cell::RefCell;
#[allow(unused_imports)]
use test_rs::shim::{
    argmax, argmin, descend_or_next, difference_level, is_val, next, path, prefix_of, PathMap,
    ReadZipperUntracked, Zipper, ZipperMoving,
};
use synth_macro::synth_formula;

#[allow(unused_parens)]
fn main() {
    let __src_a: PathMap<Option<u32>> =
        PathMap::from_iter([("001", None), ("100", None), ("101", None), ("110", None)]);
    let a = RefCell::new(__src_a.read_zipper());
    let __src_b: PathMap<Option<u32>> =
        PathMap::from_iter([("001", None), ("010", None), ("100", None), ("101", None)]);
    let b = RefCell::new(__src_b.read_zipper());
    let __src_c: PathMap<Option<u32>> =
        PathMap::from_iter([("010", None), ("011", None), ("100", None), ("101", None)]);
    let c = RefCell::new(__src_c.read_zipper());
    let __src_d: PathMap<Option<u32>> = PathMap::from_iter([("000", None), ("100", None)]);
    let d = RefCell::new(__src_d.read_zipper());
    let mut r = Vec::new();

    synth_formula! {
        sink: r;
        formula: (a | b) & c - d;
    }

    println!("result:");
    for v in r {
        test_rs::shim::print_path(&v);
    }
}
