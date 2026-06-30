//! Tests for the runtime formula→graph builder + raw-pointer interpreter.
//! These mirror the `synth_macro` regression cases and assert the same sets.

use pathmap::PathMap;
use synth_jit::formula::build_graph;

fn run_formula(formula: &str, env: &[(&str, &[&str])]) -> Vec<String> {
    let graph = build_graph(formula).expect("valid formula");

    // The graph fixes the source order; build the maps and zippers in that order.
    let maps: Vec<PathMap<Option<u32>>> = graph
        .source_names
        .iter()
        .map(|name| {
            let items = env
                .iter()
                .find(|(n, _)| n == name)
                .unwrap_or_else(|| panic!("no data for source {name}"))
                .1;
            PathMap::from_iter(items.iter().map(|s| (*s, None)))
        })
        .collect();

    // Zippers must outlive the run and be exclusively borrowed (raw pointers).
    let mut zippers: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
    let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();

    let mut sink: Vec<Vec<u8>> = Vec::new();
    unsafe { graph.run(&ptrs, &mut sink) };

    let mut out: Vec<String> = sink
        .into_iter()
        .map(|v| String::from_utf8(v).unwrap())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn want(items: &[&str]) -> Vec<String> {
    let mut s: Vec<String> = items.iter().map(|x| x.to_string()).collect();
    s.sort();
    s.dedup();
    s
}

#[test]
fn union_intersect_diff() {
    let got = run_formula(
        "(a | b) & c - d",
        &[
            ("a", &["001", "100", "101", "110"]),
            ("b", &["001", "010", "100", "101"]),
            ("c", &["010", "011", "100", "101"]),
            ("d", &["000", "100"]),
        ],
    );
    assert_eq!(got, want(&["010", "101"]));
}

#[test]
fn intersection() {
    let got = run_formula(
        "a & b & c",
        &[
            ("a", &["000", "010", "101", "110"]),
            ("b", &["010", "011", "101", "111"]),
            ("c", &["001", "010", "101"]),
        ],
    );
    assert_eq!(got, want(&["010", "101"]));
}

#[test]
fn singleton_union() {
    let got = run_formula(
        "a | (b & c)",
        &[
            ("a", &["000", "101"]),
            ("b", &["010", "011", "100", "101"]),
            ("c", &["010", "100", "101", "110"]),
        ],
    );
    assert_eq!(got, want(&["000", "010", "100", "101"]));
}

#[test]
fn diff_then_union() {
    let got = run_formula(
        "(a - b) | (c & d)",
        &[
            ("a", &["001", "010", "100", "111"]),
            ("b", &["010", "111"]),
            ("c", &["000", "100", "101"]),
            ("d", &["100", "101", "110"]),
        ],
    );
    assert_eq!(got, want(&["001", "100", "101"]));
}

#[test]
fn nested_difference() {
    let got = run_formula(
        "a - (b - c)",
        &[
            ("a", &["000", "001", "010", "100", "101"]),
            ("b", &["001", "010", "100"]),
            ("c", &["010", "100"]),
        ],
    );
    assert_eq!(got, want(&["000", "010", "100", "101"]));
}
