//! Benchmark: the same formula `(a | b) & c - d` evaluated four ways over the
//! same tries:
//!
//!   * `macro`        — `synth_formula!` (compile-time codegen, RefCell zippers)
//!   * `interpreter`  — `synth_jit::Graph::run` (runtime graph, raw pointers)
//!   * `jit`          — `synth_jit::jit::Jit`   (Cranelift native code)
//!   * `materialize`  — naive: build the full intermediate tries with PathMap's
//!                      `join`/`meet`/`subtract` set algebra, one op at a time
//!
//! The first three stream the formula in a single fused traversal that never
//! allocates an intermediate result set; `materialize` is the textbook approach
//! that computes `a | b`, then `& c`, then `- d`, allocating a whole trie at
//! each step. It's the baseline the fused passes are compared against (note that
//! PathMap's set algebra shares subtries, so the naive version is no pushover).
//!
//! Run with: `cargo run --release --example bench`

use std::cell::RefCell;
use std::hint::black_box;
use std::time::{Duration, Instant};

use pathmap::PathMap;
#[allow(unused_imports)]
use test_rs::shim::{
    argmax, argmin, descend_or_next, difference_level, is_val, next, path, prefix_of,
    ReadZipperUntracked, Zipper, ZipperMoving,
};

use synth_jit::formula::build_graph;
use synth_jit::jit::Jit;
use synth_jit::Graph;
use synth_macro::synth_formula;

type Map = PathMap<Option<u32>>;

/// Map used by the `materialize` baseline. Its value type is `()` because
/// `PathMap::subtract` requires `V: DistributiveLattice`, which `Option<u32>`
/// does not implement (but `()` does). The path *set* is identical to `Map`'s,
/// which is all the materializing set algebra cares about.
type UnitMap = PathMap<()>;

/// Deterministic distinct 3-byte keys (LCG, no rand dependency). 3 bytes ->
/// up to 16M distinct keys, so inputs can get large.
fn gen_keys(seed: u64, count: usize) -> Vec<[u8; 3]> {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
    let mut set = std::collections::BTreeSet::new();
    while set.len() < count {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let v = (s >> 24) as u32 & 0x00FF_FFFF;
        set.insert([(v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }
    set.into_iter().collect()
}

fn make_map(keys: &[[u8; 3]]) -> Map {
    PathMap::from_iter(keys.iter().map(|k| (k.as_slice(), None)))
}

fn make_unit_map(keys: &[[u8; 3]]) -> UnitMap {
    PathMap::from_iter(keys.iter().map(|k| (k.as_slice(), ())))
}

/// Naive materializing evaluation of `(a | b) & c - d`: build each intermediate
/// trie explicitly with PathMap's set algebra, then read out the resulting
/// paths. Allocates a fresh trie per operator, unlike the fused passes.
fn run_materialize(maps: &[&UnitMap; 4]) -> Vec<Vec<u8>> {
    let [a, b, c, d] = *maps;
    let union = a.join(b); // a | b
    let inter = union.meet(c); // (a | b) & c
    let result = inter.subtract(d); // (a | b) & c - d
    result.iter().map(|(path, _)| path).collect()
}

fn run_interp(graph: &Graph, maps: &[&Map; 4]) -> Vec<Vec<u8>> {
    let mut zs: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
    let ptrs: Vec<*mut _> = zs.iter_mut().map(|z| z as *mut _).collect();
    let mut r = Vec::new();
    unsafe { graph.run(&ptrs, &mut r) };
    r
}

fn run_jit(jit: &Jit, maps: &[&Map; 4]) -> Vec<Vec<u8>> {
    let mut zs: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
    let ptrs: Vec<*mut _> = zs.iter_mut().map(|z| z as *mut _).collect();
    let mut r = Vec::new();
    unsafe { jit.run(&ptrs, &mut r) };
    r
}

/// Adaptive timer: warm up ~0.2s, then take the fastest single run over ~1s
/// (and at least 5 runs). Returns seconds per run.
#[allow(unused_assignments)]
fn bench<F: FnMut() -> usize>(name: &str, mut f: F) -> f64 {
    let warm = Instant::now();
    while warm.elapsed() < Duration::from_millis(200) {
        black_box(f());
    }
    let mut best = f64::MAX;
    let mut n = 0;
    let mut runs = 0u32;
    let overall = Instant::now();
    loop {
        let t = Instant::now();
        n = black_box(f());
        best = best.min(t.elapsed().as_secs_f64());
        runs += 1;
        if (overall.elapsed() >= Duration::from_secs(1) && runs >= 5) || runs >= 2000 {
            break;
        }
    }
    let rps = 1.0 / best;
    let rps_str = if rps >= 100.0 {
        format!("{rps:.0}")
    } else {
        format!("{rps:.1}")
    };
    println!(
        "  {name:<12} {:>10.3} ms/run   {rps_str:>10} runs/s   (result set: {n})",
        best * 1e3,
    );
    best
}

fn sorted_unique(v: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut s = v.to_vec();
    s.sort();
    s.dedup();
    s
}

fn run_all(n: usize, graph: &Graph, jit: &Jit) {
    let ka = gen_keys(1, n);
    let kb = gen_keys(2, n);
    let kc = gen_keys(3, n);
    let kd = gen_keys(4, n);
    let ma = make_map(&ka);
    let mb = make_map(&kb);
    let mc = make_map(&kc);
    let md = make_map(&kd);
    let maps = [&ma, &mb, &mc, &md];

    // Separate `()`-valued tries for the materializing baseline (same key sets).
    let ua = make_unit_map(&ka);
    let ub = make_unit_map(&kb);
    let uc = make_unit_map(&kc);
    let ud = make_unit_map(&kd);
    let unit_maps = [&ua, &ub, &uc, &ud];

    // correctness: all three must agree.
    let macro_once = {
        let a = RefCell::new(ma.read_zipper());
        let b = RefCell::new(mb.read_zipper());
        let c = RefCell::new(mc.read_zipper());
        let d = RefCell::new(md.read_zipper());
        let mut r = Vec::new();
        synth_formula! { sink: r; formula: (a | b) & c - d; }
        r
    };
    let interp_once = run_interp(graph, &maps);
    let jit_once = run_jit(jit, &maps);
    let mat_once = run_materialize(&unit_maps);
    assert_eq!(sorted_unique(&macro_once), sorted_unique(&interp_once));
    assert_eq!(sorted_unique(&interp_once), sorted_unique(&jit_once));
    assert_eq!(sorted_unique(&jit_once), sorted_unique(&mat_once));

    println!(
        "--- {} keys x 4 sources, {} matched paths ---",
        fmt_count(n),
        sorted_unique(&jit_once).len()
    );
    let m = bench("macro", || {
        let a = RefCell::new(ma.read_zipper());
        let b = RefCell::new(mb.read_zipper());
        let c = RefCell::new(mc.read_zipper());
        let d = RefCell::new(md.read_zipper());
        let mut r = Vec::new();
        synth_formula! { sink: r; formula: (a | b) & c - d; }
        r.len()
    });
    let i = bench("interpreter", || run_interp(graph, &maps).len());
    let j = bench("jit", || run_jit(jit, &maps).len());
    let mt = bench("materialize", || run_materialize(&unit_maps).len());
    println!(
        "  speedup vs macro:  interpreter {:.2}x   jit {:.2}x   |   jit vs interpreter {:.2}x\n  fused vs materialize:  macro {:.2}x   interpreter {:.2}x   jit {:.2}x\n",
        m / i,
        m / j,
        i / j,
        mt / m,
        mt / i,
        mt / j,
    );
}

fn fmt_count(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

fn main() {
    let formula = "(a | b) & c - d";

    let graph = build_graph(formula).unwrap();
    let jit = Jit::compile::<Option<u32>, ()>(&graph);

    println!("formula:  {formula}   ({} states)", graph.states.len());
    println!("(macro = compile-time codegen w/ RefCell zippers; interpreter & jit = raw pointers)\n");

    // Construction cost (per formula, independent of input size). The macro has
    // no runtime construction — its dispatch loop is generated by the compiler.
    println!("--- construction (per formula) ---");
    bench("build_graph", || build_graph(formula).unwrap().states.len());
    bench("jit_compile", || {
        Jit::compile::<Option<u32>, ()>(&graph).num_sources()
    });
    println!();

    // Sizes are configurable via args, e.g. `cargo run --release --example bench -- 50000 1000000`.
    let sizes: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let sizes = if sizes.is_empty() {
        vec![50_000, 500_000, 2_000_000]
    } else {
        sizes
    };

    for n in sizes {
        run_all(n, &graph, &jit);
    }
}
