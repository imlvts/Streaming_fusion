//! Randomized validation: generate random set formulas + inputs, run them
//! through the runtime graph builder + raw-pointer interpreter, and compare to
//! a direct set-algebra oracle. Fully self-contained (no Python).

use std::collections::HashSet;

use pathmap::PathMap;
use synth_jit::formula::build_graph;

// Tiny deterministic LCG so the test is reproducible without a rand dependency.
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

fn eval(e: &E, env: &std::collections::HashMap<char, HashSet<String>>) -> HashSet<String> {
    match e {
        E::V(c) => env[c].clone(),
        E::And(l, r) => eval(l, env).intersection(&eval(r, env)).cloned().collect(),
        E::Or(l, r) => eval(l, env).union(&eval(r, env)).cloned().collect(),
        E::Diff(l, r) => eval(l, env).difference(&eval(r, env)).cloned().collect(),
    }
}

fn interp_result(formula: &str, env: &std::collections::HashMap<char, HashSet<String>>) -> HashSet<String> {
    let graph = build_graph(formula).expect("valid formula");
    let maps: Vec<PathMap<Option<u32>>> = graph
        .source_names
        .iter()
        .map(|name| {
            let ch = name.chars().next().unwrap();
            let empty = HashSet::new();
            let items = env.get(&ch).unwrap_or(&empty);
            PathMap::from_iter(items.iter().map(|s| (s.as_str(), None)))
        })
        .collect();
    let mut zippers: Vec<_> = maps.iter().map(|m| m.read_zipper()).collect();
    let ptrs: Vec<*mut _> = zippers.iter_mut().map(|z| z as *mut _).collect();
    let mut sink: Vec<Vec<u8>> = Vec::new();
    unsafe { graph.run(&ptrs, &mut sink) };
    sink.into_iter()
        .map(|v| String::from_utf8(v).unwrap())
        .collect()
}

#[test]
fn fuzz_against_set_oracle() {
    let universe: Vec<String> = (0..8u8).map(|n| format!("{n:03b}")).collect();
    let mut rng = Lcg(0x1234_5678_9abc_def0);

    let mut checked = 0;
    for _ in 0..400 {
        let depth = 2 + rng.below(3);
        let expr = gen_expr(&mut rng, depth);
        let formula = render(&expr);

        let mut vars = HashSet::new();
        used_vars(&expr, &mut vars);

        let mut env: std::collections::HashMap<char, HashSet<String>> =
            std::collections::HashMap::new();
        for &v in &vars {
            let k = 1 + rng.below(universe.len());
            let mut set = HashSet::new();
            // sample k distinct elements
            let mut pool: Vec<String> = universe.clone();
            for _ in 0..k {
                if pool.is_empty() {
                    break;
                }
                let idx = rng.below(pool.len());
                set.insert(pool.swap_remove(idx));
            }
            env.insert(v, set);
        }

        let expected = eval(&expr, &env);
        let got = interp_result(&formula, &env);
        assert_eq!(
            got, expected,
            "formula `{formula}` env {env:?}: got {got:?} expected {expected:?}"
        );
        checked += 1;
    }
    assert_eq!(checked, 400);
}
