//! End-to-end check: the REAL create_state_machine graph (same one behind
//! src/main__example_output.rs) driven through the `synth!` macro, with the
//! input data from that example. Expected result: 010, 101.
use std::cell::RefCell;

#[allow(unused_imports)]
use test_rs::shim::{
    argmax, argmin, descend_or_next, difference_level, is_val, next, path, prefix_of, PathMap,
    ReadZipperUntracked, Zipper, ZipperMoving,
};
use synth_macro::synth;

#[allow(unused_parens)]
fn main() {
    let __src_a: PathMap<Option<u32>> = PathMap::from_iter([("001", None), ("100", None), ("101", None), ("110", None)]);
    let a = RefCell::new(__src_a.read_zipper());
    let __src_b: PathMap<Option<u32>> = PathMap::from_iter([("001", None), ("010", None), ("100", None), ("101", None)]);
    let b = RefCell::new(__src_b.read_zipper());
    let __src_c: PathMap<Option<u32>> = PathMap::from_iter([("010", None), ("011", None), ("100", None), ("101", None)]);
    let c = RefCell::new(__src_c.read_zipper());
    let __src_d: PathMap<Option<u32>> = PathMap::from_iter([("000", None), ("100", None)]);
    let d = RefCell::new(__src_d.read_zipper());
    let mut r = Vec::new();

    synth! {
        sources: a, b, c, d;
        sinks: r;
        init: s0;

        s0 -> s1: descend = [a, c, b, d];
        s1 -> sc0: is_value(b), is_value(c), b == b, b == c, op_or_not(a >= b), ne_if_value(d != b), active = [b, c];
        sc0 -> sb: op_or_eq_not_value(d > b), push = [(r, b)];
        sc0 -> n0: d < b, active = [d];
        n0 -> sc0: prefix_of(d, b), active = [d], descend = [d];
        n0 -> sc0: not_prefix_of(d, b), active = [d], next_i = [d -> (b)];
        sc0 -> s1: is_value(d), d == b, active = [d];
        s1 -> sc1: is_value(a), is_value(c), a == a, a == c, op_or_not(b >= a), ne_if_value(d != a), active = [a, c];
        sc1 -> sa: op_or_eq_not_value(d > a), push = [(r, a)];
        sc1 -> n1: d < a, active = [d];
        n1 -> sc1: prefix_of(d, a), active = [d], descend = [d];
        n1 -> sc1: not_prefix_of(d, a), active = [d], next_i = [d -> (a)];
        sc1 -> s1: is_value(d), d == a, active = [d];
        s1 -> s2;
        s2 -> sb: op_or_not(a >= b), op_or_not(c >= b), active = [b];
        sb -> n2: b == a, active = [a];
        n2 -> sb: b == a, prefix_of(a, c), active = [c], descend = [a];
        n2 -> sb: b == a, not_prefix_of(a, c), active = [c], next_i = [a -> (c)];
        n2 -> sb: finished(c), end = [a];
        sb -> n3: b == c, active = [c], define = m = argmin(a, b);
        n3 -> sb: var_none(m), descend = [c];
        n3 -> sb: prefix_of(c, m), descend = [c];
        n3 -> sb: not_prefix_of(c, m), next_i_var = [c -> m];
        sb -> n4: define = m = c;
        n4 -> s1: var_none(m), descend = [b];
        n4 -> s1: prefix_of(b, m), descend = [b];
        n4 -> s1: not_prefix_of(b, m), next_i_var = [b -> m];
        s2 -> sa: op_or_not(b >= a), op_or_not(c >= a), active = [a];
        sa -> n5: a == b, active = [b];
        n5 -> sa: a == b, prefix_of(b, c), active = [c], descend = [b];
        n5 -> sa: a == b, not_prefix_of(b, c), active = [c], next_i = [b -> (c)];
        n5 -> sa: finished(c), end = [b];
        sa -> n6: a == c, active = [c], define = m = argmin(a, b);
        n6 -> sa: var_none(m), descend = [c];
        n6 -> sa: prefix_of(c, m), descend = [c];
        n6 -> sa: not_prefix_of(c, m), next_i_var = [c -> m];
        sa -> n7: define = m = c;
        n7 -> s1: var_none(m), descend = [a];
        n7 -> s1: prefix_of(a, m), descend = [a];
        n7 -> s1: not_prefix_of(a, m), next_i_var = [a -> m];
        s2 -> sc: op_or_not(b >= c), op_or_not(a >= c), active = [c];
        sc -> n8: c == b, active = [b];
        n8 -> sc: c == b, prefix_of(b, c), active = [c], descend = [b];
        n8 -> sc: c == b, not_prefix_of(b, c), active = [c], next_i = [b -> (c)];
        n8 -> sc: finished(c), end = [b];
        sc -> n9: c == a, active = [a];
        n9 -> sc: c == a, prefix_of(a, c), active = [c], descend = [a];
        n9 -> sc: c == a, not_prefix_of(a, c), active = [c], next_i = [a -> (c)];
        n9 -> sc: finished(c), end = [a];
        sc -> n10: define = m = argmin(a, b);
        n10 -> s1: var_none(m), descend = [c];
        n10 -> s1: prefix_of(c, m), descend = [c];
        n10 -> s1: not_prefix_of(c, m), next_i_var = [c -> m];
    }

    println!("result:");
    for v in r {
        test_rs::shim::print_path(&v);
    }
}
