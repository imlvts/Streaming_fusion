//! Demo: take a set formula as a *runtime* string, build the state machine,
//! JIT-compile it with Cranelift, and run it over some tries. Also runs the
//! interpreter on the same input to show they agree.

use pathmap::PathMap;
use synth_jit::formula::build_graph;
use synth_jit::jit::Jit;

fn main() {
    // A formula only known at run time — exactly the case a proc macro can't handle.
    let formula = "(a | b) & c - d";

    let env: &[(&str, &[&str])] = &[
        ("a", &["001", "100", "101", "110"]),
        ("b", &["001", "010", "100", "101"]),
        ("c", &["010", "011", "100", "101"]),
        ("d", &["000", "100"]),
    ];

    let graph = build_graph(formula).expect("valid formula");
    println!("formula: {formula}");
    println!("sources: {:?}", graph.source_names);
    println!("states:  {}", graph.states.len());

    let maps: Vec<PathMap<Option<u32>>> = graph
        .source_names
        .iter()
        .map(|name| {
            let items = env.iter().find(|(n, _)| n == name).map(|(_, v)| *v).unwrap_or(&[]);
            PathMap::from_iter(items.iter().map(|s| (*s, None)))
        })
        .collect();

    // ---- interpreter ----
    let mut interp_out = {
        let mut zippers: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
        let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();
        let mut sink = Vec::new();
        unsafe { graph.run(&ptrs, &mut sink) };
        dedup_strings(sink)
    };

    // ---- JIT ----
    let jit = Jit::compile::<Option<u32>, ()>(&graph);
    let mut jit_out = {
        let mut zippers: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
        let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();
        let mut sink = Vec::new();
        unsafe { jit.run(&ptrs, &mut sink) };
        dedup_strings(sink)
    };

    interp_out.sort();
    jit_out.sort();

    println!("interpreter: {interp_out:?}");
    println!("jit:         {jit_out:?}");
    assert_eq!(interp_out, jit_out, "JIT and interpreter disagree");
    println!("agree ✓");
}

fn dedup_strings(v: Vec<Vec<u8>>) -> Vec<String> {
    let mut s: Vec<String> = v.into_iter().map(|x| String::from_utf8(x).unwrap()).collect();
    s.sort();
    s.dedup();
    s
}
