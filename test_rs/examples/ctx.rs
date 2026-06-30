use std::cell::RefCell;
#[allow(unused_imports)]
use test_rs::shim::{
    argmax, argmin, descend_or_next, difference_level, is_val, next, path, prefix_of, PathMap,
    ReadZipperUntracked, Zipper, ZipperMoving,
};
use synth_macro::synth;

#[allow(unused_parens)]
fn test() {
    let __src_a: PathMap<Option<u32>> =
        PathMap::from_iter([("000", None), ("101", None), ("110", None)]);
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

    // ctx() from src/trie/trie_synth.py: ((a | b) & c) - d
    synth! {
        sources: a, b, c, d;
        sinks: r;
        init: s0;

        s0 -> s1: descend = [a, b, c, d];

        s1 -> s2:  not_finished(a), not_finished(c), a < c;
        s1 -> s3:  not_finished(b), not_finished(c), b < c;
        s1 -> s4:  not_finished(a), not_finished(c), a == c;
        s1 -> s5:  not_finished(b), not_finished(c), b == c;
        s1 -> s6:  not_finished(a), not_finished(b), not_finished(c), a > c, b > c;
        s1 -> s11: finished(a), not_finished(b), not_finished(c), b > c;
        s1 -> s12: finished(b), not_finished(a), not_finished(c), a > c;

        s2 -> s1: prefix_of(a, c), descend = [a];
        s2 -> s1: not_prefix_of(a, c), next_i = [a -> (c)];

        s3 -> s1: prefix_of(b, c), descend = [b];
        s3 -> s1: not_prefix_of(b, c), next_i = [b -> (c)];

        s4 -> s7: is_value(a), is_value(c);
        s4 -> s1: not_value(a), descend = [a];
        s4 -> s1: not_value(c), descend = [a];

        s5 -> s9: is_value(b), is_value(c);
        s5 -> s1: not_value(b), descend = [b];
        s5 -> s1: not_value(c), descend = [b];

        s6 -> s1: prefix_of(c, a), descend = [c];
        s6 -> s1: prefix_of(c, b), descend = [c];
        s6 -> s1: not_prefix_of(c, a), not_prefix_of(c, b), next_i = [c -> (a, b)];

        s7 -> s8: not_finished(d), a > d;
        s7 -> s1: not_finished(d), a < d, push = [(r, a)], descend = [a];
        s7 -> s1: not_finished(d), a == d, is_value(d), descend = [a];
        s7 -> s1: not_finished(d), a == d, not_value(d), push = [(r, a)], descend = [a];
        s7 -> s1: finished(d), push = [(r, a)], descend = [a];

        s8 -> s7: not_finished(d), prefix_of(d, a), descend = [d];
        s8 -> s7: not_finished(d), not_prefix_of(d, a), next_i = [d -> (a)];

        s9 -> s10: not_finished(d), b > d;
        s9 -> s1:  not_finished(d), b < d, push = [(r, b)], descend = [b];
        s9 -> s1:  not_finished(d), b == d, is_value(d), descend = [b];
        s9 -> s1:  not_finished(d), b == d, not_value(d), push = [(r, b)], descend = [b];
        s9 -> s1:  finished(d), push = [(r, b)], descend = [b];

        s10 -> s9: not_finished(d), prefix_of(d, b), descend = [d];
        s10 -> s9: not_finished(d), not_prefix_of(d, b), next_i = [d -> (b)];

        s11 -> s1: prefix_of(c, b), descend = [c];
        s11 -> s1: not_prefix_of(c, b), next_i = [c -> (b)];

        s12 -> s1: prefix_of(c, a), descend = [c];
        s12 -> s1: not_prefix_of(c, a), next_i = [c -> (a)];
    }

    println!("result:");
    for v in r {
        test_rs::shim::print_path(&v);
    }
}

fn main() {
    test();
}
