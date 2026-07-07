//! Demo: run the trie state machine with **ArenaCompactTree (ACT)** zippers
//! as sources instead of PathMap read zippers.
//!
//! Each source is streamed to an on-disk `.act` file with `ACTOutputStream`,
//! opened back via mmap, and its `ACTZipper` is handed to [`Graph::run`] —
//! which is generic over any `Zipper + ZipperMoving`. Results are
//! cross-checked against the same formula evaluated over PathMap sources.

use pathmap::PathMap;
use pathmap::arena_compact::{ACTMmap, ACTOutputStream};
use synth_jit::formula::build_graph;

fn lookup<'e>(env: &'e [(&str, &[&str])], name: &str) -> &'e [&'e str] {
    env.iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("no data for source {name}"))
        .1
}

/// Evaluate `formula` with every source backed by an mmap'd ACT file.
fn run_formula_act(formula: &str, env: &[(&str, &[&str])]) -> Vec<String> {
    let graph = build_graph(formula).expect("valid formula");
    let dir = tempfile::tempdir().expect("tempdir");

    // Stream each source's paths (sorted, as ACTOutputStream requires) to
    // its own .act file, then mmap it back.
    let trees: Vec<ACTMmap> = graph
        .source_names
        .iter()
        .map(|name| {
            let mut items = lookup(env, name).to_vec();
            items.sort();
            let mut act = ACTOutputStream::new(dir.path().join(format!("{name}.act")))
                .expect("create act file");
            for item in items {
                act.push(item).expect("push path");
            }
            act.finish().expect("finish act file")
        })
        .collect();

    let mut zippers: Vec<_> = trees.iter().map(|t| t.read_zipper()).collect();
    let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();

    // Stream results through a closure sink; no per-path Vec allocation.
    let mut out: Vec<String> = Vec::new();
    let mut sink = |path: &[u8]| out.push(String::from_utf8(path.to_vec()).unwrap());
    unsafe { graph.run(&ptrs, &mut sink) };
    out
}

/// Reference evaluation over PathMap sources.
fn run_formula_pathmap(formula: &str, env: &[(&str, &[&str])]) -> Vec<String> {
    let graph = build_graph(formula).expect("valid formula");
    let maps: Vec<PathMap<Option<u32>>> = graph
        .source_names
        .iter()
        .map(|name| PathMap::from_iter(lookup(env, name).iter().map(|s| (*s, None))))
        .collect();
    let mut zippers: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
    let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();

    let mut sink: Vec<Vec<u8>> = Vec::new();
    unsafe { graph.run(&ptrs, &mut sink) };

    sink.into_iter()
        .map(|v| String::from_utf8(v).unwrap())
        .collect()
}

/// The machine emits paths in ascending order with no duplicates, so the
/// expected output is asserted exactly — no sorting or dedup on either side.
fn check(formula: &str, env: &[(&str, &[&str])], expect: &[&str]) {
    let want: Vec<String> = expect.iter().map(|s| s.to_string()).collect();
    assert_eq!(run_formula_act(formula, env), want, "ACT sources: {formula}");
    assert_eq!(
        run_formula_pathmap(formula, env),
        want,
        "PathMap sources: {formula}"
    );
}

#[test]
fn act_union_intersect_diff() {
    check(
        "(a | b) & c - d",
        &[
            ("a", &["001", "100", "101", "110"]),
            ("b", &["001", "010", "100", "101"]),
            ("c", &["010", "011", "100", "101"]),
            ("d", &["000", "100"]),
        ],
        &["010", "101"],
    );
}

#[test]
fn act_intersection() {
    check(
        "a & b & c",
        &[
            ("a", &["000", "010", "101", "110"]),
            ("b", &["010", "011", "101", "111"]),
            ("c", &["001", "010", "101"]),
        ],
        &["010", "101"],
    );
}

#[test]
fn act_diff_then_union() {
    check(
        "(a - b) | (c & d)",
        &[
            ("a", &["001", "010", "100", "111"]),
            ("b", &["010", "111"]),
            ("c", &["000", "100", "101"]),
            ("d", &["100", "101", "110"]),
        ],
        &["001", "100", "101"],
    );
}

/// Variable-length keys exercise the ACT line-node (path compression) reads.
#[test]
fn act_variable_length_keys() {
    check(
        "a & b",
        &[
            ("a", &["romane", "romanus", "romulus", "rubens", "ruber"]),
            ("b", &["romane", "romulus", "ruber", "rubicon"]),
        ],
        &["romane", "romulus", "ruber"],
    );
}
