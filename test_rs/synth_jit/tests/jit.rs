//! Tests for the Cranelift JIT: it must produce the same results as the
//! interpreter (and the set-algebra oracle).

use std::collections::{HashMap, HashSet};

use pathmap::PathMap;
use synth_jit::formula::build_graph;
use synth_jit::jit::Jit;

/// Run a formula through the JIT and return the deduplicated result set.
fn jit_run(formula: &str, env: &[(&str, &[&str])]) -> Vec<String> {
    let graph = build_graph(formula).expect("valid formula");
    let jit = Jit::compile::<Option<u32>, ()>(&graph);

    let maps: Vec<PathMap<Option<u32>>> = graph
        .source_names
        .iter()
        .map(|name| {
            let items = env
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| *v)
                .unwrap_or(&[]);
            PathMap::from_iter(items.iter().map(|s| (*s, None)))
        })
        .collect();
    let mut zippers: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
    let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();

    let mut sink: Vec<Vec<u8>> = Vec::new();
    unsafe { jit.run(&ptrs, &mut sink) };

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
fn jit_union_intersect_diff() {
    let got = jit_run(
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
fn jit_intersection() {
    let got = jit_run(
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
fn jit_singleton_union() {
    let got = jit_run(
        "a | (b & c)",
        &[
            ("a", &["000", "101"]),
            ("b", &["010", "011", "100", "101"]),
            ("c", &["010", "100", "101", "110"]),
        ],
    );
    assert_eq!(got, want(&["000", "010", "100", "101"]));
}

// ---- randomized: JIT vs set-algebra oracle ----

struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() as usize) % n
    }
}

#[derive(Clone)]
enum E {
    V(char),
    And(Box<E>, Box<E>),
    Or(Box<E>, Box<E>),
    Diff(Box<E>, Box<E>),
}

const VARS: &[char] = &['a', 'b', 'c', 'd'];

fn gen_expr(rng: &mut Lcg, depth: usize) -> E {
    if depth == 0 || (depth < 3 && rng.below(10) < 4) {
        return E::V(VARS[rng.below(VARS.len())]);
    }
    let l = Box::new(gen_expr(rng, depth - 1));
    let r = Box::new(gen_expr(rng, depth - 1));
    match rng.below(3) {
        0 => E::And(l, r),
        1 => E::Or(l, r),
        _ => E::Diff(l, r),
    }
}

fn render(e: &E) -> String {
    match e {
        E::V(c) => c.to_string(),
        E::And(l, r) => format!("({} & {})", render(l), render(r)),
        E::Or(l, r) => format!("({} | {})", render(l), render(r)),
        E::Diff(l, r) => format!("({} - {})", render(l), render(r)),
    }
}

fn used_vars(e: &E, out: &mut HashSet<char>) {
    match e {
        E::V(c) => {
            out.insert(*c);
        }
        E::And(l, r) | E::Or(l, r) | E::Diff(l, r) => {
            used_vars(l, out);
            used_vars(r, out);
        }
    }
}

fn eval(e: &E, env: &HashMap<char, HashSet<String>>) -> HashSet<String> {
    match e {
        E::V(c) => env[c].clone(),
        E::And(l, r) => eval(l, env).intersection(&eval(r, env)).cloned().collect(),
        E::Or(l, r) => eval(l, env).union(&eval(r, env)).cloned().collect(),
        E::Diff(l, r) => eval(l, env).difference(&eval(r, env)).cloned().collect(),
    }
}

#[test]
fn jit_fuzz_against_oracle() {
    let universe: Vec<String> = (0..8u8).map(|n| format!("{n:03b}")).collect();
    let mut rng = Lcg(0xcafe_f00d_1234_5678);

    for _ in 0..300 {
        let depth = 2 + rng.below(3);
        let expr = gen_expr(&mut rng, depth);
        let formula = render(&expr);

        let mut vars = HashSet::new();
        used_vars(&expr, &mut vars);

        let mut env: HashMap<char, HashSet<String>> = HashMap::new();
        for &v in &vars {
            let k = 1 + rng.below(universe.len());
            let mut pool = universe.clone();
            let mut set = HashSet::new();
            for _ in 0..k {
                if pool.is_empty() {
                    break;
                }
                set.insert(pool.swap_remove(rng.below(pool.len())));
            }
            env.insert(v, set);
        }

        let expected: Vec<String> = {
            let mut v: Vec<String> = eval(&expr, &env).into_iter().collect();
            v.sort();
            v
        };

        let env_slices: Vec<(String, Vec<String>)> = env
            .iter()
            .map(|(k, set)| (k.to_string(), set.iter().cloned().collect()))
            .collect();
        let env_refs: Vec<(&str, Vec<&str>)> = env_slices
            .iter()
            .map(|(k, v)| (k.as_str(), v.iter().map(|s| s.as_str()).collect()))
            .collect();
        let slices: Vec<(&str, &[&str])> =
            env_refs.iter().map(|(k, v)| (*k, v.as_slice())).collect();

        let got = jit_run(&formula, &slices);
        assert_eq!(
            got, expected,
            "formula `{formula}` env {env:?}: got {got:?} expected {expected:?}"
        );
    }
}
