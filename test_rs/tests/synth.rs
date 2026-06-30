//! Regression tests for the `synth!` and `synth_formula!` macros.
//!
//! Each test builds the source tries, runs the generated dispatch loop, and
//! checks the produced set (deduplicated) against the expected set semantics.

#![allow(unused_parens)]

use std::cell::RefCell;

use synth_macro::{synth, synth_formula};
#[allow(unused_imports)]
use test_rs::shim::{
    argmax, argmin, descend_or_next, difference_level, is_val, next, path, prefix_of, PathMap,
    ReadZipperUntracked, Zipper, ZipperMoving,
};

fn pm(items: &[&str]) -> PathMap<Option<u32>> {
    PathMap::from_iter(items.iter().map(|s| (*s, None)))
}

fn sorted_unique(v: Vec<Vec<u8>>) -> Vec<String> {
    let mut s: Vec<String> = v
        .into_iter()
        .map(|x| String::from_utf8(x).unwrap())
        .collect();
    s.sort();
    s.dedup();
    s
}

fn want(items: &[&str]) -> Vec<String> {
    let mut s: Vec<String> = items.iter().map(|x| x.to_string()).collect();
    s.sort();
    s.dedup();
    s
}

#[test]
fn formula_union_intersect_diff() {
    // ((a | b) & c) - d   — the canonical example.
    let ma = pm(&["001", "100", "101", "110"]);
    let mb = pm(&["001", "010", "100", "101"]);
    let mc = pm(&["010", "011", "100", "101"]);
    let md = pm(&["000", "100"]);
    let a = RefCell::new(ma.read_zipper());
    let b = RefCell::new(mb.read_zipper());
    let c = RefCell::new(mc.read_zipper());
    let d = RefCell::new(md.read_zipper());
    let mut r = Vec::new();

    synth_formula! { sink: r; formula: (a | b) & c - d; }

    assert_eq!(sorted_unique(r), want(&["010", "101"]));
}

#[test]
fn formula_intersection() {
    // a & b & c
    let ma = pm(&["000", "010", "101", "110"]);
    let mb = pm(&["010", "011", "101", "111"]);
    let mc = pm(&["001", "010", "101"]);
    let a = RefCell::new(ma.read_zipper());
    let b = RefCell::new(mb.read_zipper());
    let c = RefCell::new(mc.read_zipper());
    let mut r = Vec::new();

    synth_formula! { sink: r; formula: a & b & c; }

    assert_eq!(sorted_unique(r), want(&["010", "101"]));
}

#[test]
fn formula_singleton_union() {
    // a | (b & c)   — exercises the `singleton` path for `a`.
    let ma = pm(&["000", "101"]);
    let mb = pm(&["010", "011", "100", "101"]);
    let mc = pm(&["010", "100", "101", "110"]);
    let a = RefCell::new(ma.read_zipper());
    let b = RefCell::new(mb.read_zipper());
    let c = RefCell::new(mc.read_zipper());
    let mut r = Vec::new();

    synth_formula! { sink: r; formula: a | (b & c); }

    assert_eq!(sorted_unique(r), want(&["000", "010", "100", "101"]));
}

#[test]
fn formula_diff_then_union() {
    // (a - b) | (c & d)
    let ma = pm(&["001", "010", "100", "111"]);
    let mb = pm(&["010", "111"]);
    let mc = pm(&["000", "100", "101"]);
    let md = pm(&["100", "101", "110"]);
    let a = RefCell::new(ma.read_zipper());
    let b = RefCell::new(mb.read_zipper());
    let c = RefCell::new(mc.read_zipper());
    let d = RefCell::new(md.read_zipper());
    let mut r = Vec::new();

    synth_formula! { sink: r; formula: (a - b) | (c & d); }

    assert_eq!(sorted_unique(r), want(&["001", "100", "101"]));
}

#[test]
fn formula_nested_difference() {
    // a - (b - c)
    let ma = pm(&["000", "001", "010", "100", "101"]);
    let mb = pm(&["001", "010", "100"]);
    let mc = pm(&["010", "100"]);
    let a = RefCell::new(ma.read_zipper());
    let b = RefCell::new(mb.read_zipper());
    let c = RefCell::new(mc.read_zipper());
    let mut r = Vec::new();

    synth_formula! { sink: r; formula: a - (b - c); }

    assert_eq!(sorted_unique(r), want(&["000", "010", "100", "101"]));
}

#[test]
fn synth_explicit_graph_enumerates_source() {
    // A hand-written `synth!` graph that walks one source and pushes every
    // value path — exercises the explicit-DSL macro end to end.
    let ma = pm(&["01", "10", "111"]);
    let a = RefCell::new(ma.read_zipper());
    let mut r = Vec::new();

    synth! {
        sources: a;
        sinks: r;
        init: s0;

        s0 -> s1: descend = [a];
        s1 -> s1: not_finished(a), is_value(a), push = [(r, a)], descend = [a];
        s1 -> s1: not_finished(a), not_value(a), descend = [a];
    }

    assert_eq!(sorted_unique(r), want(&["01", "10", "111"]));
}
