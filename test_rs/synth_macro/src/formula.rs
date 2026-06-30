//! Port of the formula → state-machine pipeline:
//!   * `src/expr.py`      — the `Expr` AST (`Var`, `And`, `Or`, `Diff`)
//!   * `src/normalize.py` — `normalize`: Expr → difference-DNF
//!   * `src/clause.py`    — `Clause`/`DNF` with `simplify`, `dependencies`, `singletons`
//!   * `src/trie/trie_generation.py` — `TrieExecution.create_state_machine`
//!
//! Set operations that Python ran over `set`/`frozenset` (whose iteration order
//! is arbitrary) are run here over `BTreeSet`, giving a deterministic graph. The
//! algorithm is order-independent for the computed result.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, Ident, Result, Token};

use crate::{CmpOp, Cond, Graph, Transition};

// ---------------------------------------------------------------------------
// Expr (src/expr.py) + parser (custom precedence, NOT Rust's)
// ---------------------------------------------------------------------------

/// `&` binds tightest, then `-`, then `|` (matching the Python parser, which
/// differs from Rust's native operator precedence — hence the hand parser).
enum Expr {
    Var(String),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Diff(Box<Expr>, Box<Expr>),
}

pub(crate) struct FormulaInput {
    expr: Expr,
    sink: Ident,
    /// var name -> the user-supplied source ident (first occurrence).
    vars: BTreeMap<String, Ident>,
}

fn parse_or(input: ParseStream, vars: &mut BTreeMap<String, Ident>) -> Result<Expr> {
    let mut e = parse_diff(input, vars)?;
    while input.peek(Token![|]) {
        input.parse::<Token![|]>()?;
        let rhs = parse_diff(input, vars)?;
        e = Expr::Or(Box::new(e), Box::new(rhs));
    }
    Ok(e)
}

fn parse_diff(input: ParseStream, vars: &mut BTreeMap<String, Ident>) -> Result<Expr> {
    let mut e = parse_and(input, vars)?;
    while input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        let rhs = parse_and(input, vars)?;
        e = Expr::Diff(Box::new(e), Box::new(rhs));
    }
    Ok(e)
}

fn parse_and(input: ParseStream, vars: &mut BTreeMap<String, Ident>) -> Result<Expr> {
    let mut e = parse_atom(input, vars)?;
    while input.peek(Token![&]) {
        input.parse::<Token![&]>()?;
        let rhs = parse_atom(input, vars)?;
        e = Expr::And(Box::new(e), Box::new(rhs));
    }
    Ok(e)
}

fn parse_atom(input: ParseStream, vars: &mut BTreeMap<String, Ident>) -> Result<Expr> {
    if input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        return parse_or(&content, vars);
    }
    let id: Ident = input.parse()?;
    let name = id.to_string();
    vars.entry(name.clone()).or_insert(id);
    Ok(Expr::Var(name))
}

impl Parse for FormulaInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut expr: Option<Expr> = None;
        let mut sink: Option<Ident> = None;
        let mut vars: BTreeMap<String, Ident> = BTreeMap::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "formula" => expr = Some(parse_or(input, &mut vars)?),
                "sink" => sink = Some(input.parse()?),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown key `{other}` (expected `formula` or `sink`)"),
                    ))
                }
            }
            input.parse::<Token![;]>()?;
        }

        let expr = expr.ok_or_else(|| input.error("missing `formula: <expr>;`"))?;
        let sink = sink.ok_or_else(|| input.error("missing `sink: <ident>;`"))?;
        Ok(FormulaInput { expr, sink, vars })
    }
}

// ---------------------------------------------------------------------------
// Clause / DNF (src/clause.py) + normalize (src/normalize.py)
// ---------------------------------------------------------------------------

type VarSet = BTreeSet<String>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Clause {
    p: VarSet,
    n: VarSet,
}

impl Clause {
    fn make(p: VarSet, n: VarSet) -> Clause {
        Clause { p, n }
    }
    /// `not self.P or bool(self.P & self.N)`
    fn is_empty(&self) -> bool {
        self.p.is_empty() || !self.p.is_disjoint(&self.n)
    }
}

fn one(name: &str) -> VarSet {
    let mut s = BTreeSet::new();
    s.insert(name.to_string());
    s
}

/// `c1 ⊆ c2` iff `P1 ⊇ P2 and N1 ⊇ N2`.
fn clause_subset(c1: &Clause, c2: &Clause) -> bool {
    c1.p.is_superset(&c2.p) && c1.n.is_superset(&c2.n)
}

struct Dnf {
    clauses: BTreeSet<Clause>,
}

impl Dnf {
    /// `DNF(frozenset(clauses)).simplify()`
    fn make(clauses: impl IntoIterator<Item = Clause>) -> Dnf {
        Dnf {
            clauses: simplify(clauses.into_iter().collect()),
        }
    }

    fn vars(&self) -> VarSet {
        let mut v = BTreeSet::new();
        for c in &self.clauses {
            v.extend(c.p.iter().cloned());
            v.extend(c.n.iter().cloned());
        }
        v
    }

    /// For each var, the set of `P \ {v}` (over clauses where v is positive) and
    /// `P` (over clauses where v is negative), dropping empties.
    fn dependencies(&self) -> BTreeMap<String, BTreeSet<VarSet>> {
        let mut dep = BTreeMap::new();
        for v in self.vars() {
            let mut set: BTreeSet<VarSet> = BTreeSet::new();
            for c in &self.clauses {
                if c.p.contains(&v) || c.n.contains(&v) {
                    let s: VarSet = c.p.iter().filter(|x| **x != v).cloned().collect();
                    if !s.is_empty() {
                        set.insert(s);
                    }
                }
            }
            dep.insert(v, set);
        }
        dep
    }

    fn singletons(&self) -> VarSet {
        self.vars()
            .into_iter()
            .filter(|v| {
                self.clauses
                    .iter()
                    .any(|c| c.p.len() == 1 && c.p.contains(v))
            })
            .collect()
    }
}

fn simplify(mut clauses: BTreeSet<Clause>) -> BTreeSet<Clause> {
    loop {
        let mut changed = false;

        // 1. drop empty clauses
        let new: BTreeSet<Clause> = clauses.iter().filter(|c| !c.is_empty()).cloned().collect();
        if new != clauses {
            changed = true;
            clauses = new;
        }

        // 2. drop subsumed clauses (c redundant if c ⊆ d for some other d)
        let reduced: BTreeSet<Clause> = clauses
            .iter()
            .filter(|c| !clauses.iter().any(|d| d != *c && clause_subset(c, d)))
            .cloned()
            .collect();
        if reduced != clauses {
            changed = true;
            clauses = reduced;
        }

        // 3. absorb negatives using singleton-positive clauses
        let singleton_positives: VarSet = clauses
            .iter()
            .filter(|c| c.n.is_empty() && c.p.len() == 1)
            .map(|c| c.p.iter().next().unwrap().clone())
            .collect();
        let absorbed: BTreeSet<Clause> = clauses
            .iter()
            .map(|c| {
                let newn: VarSet = c.n.difference(&singleton_positives).cloned().collect();
                Clause::make(c.p.clone(), newn)
            })
            .collect();
        if absorbed != clauses {
            changed = true;
            clauses = absorbed;
        }

        if !changed {
            break;
        }
    }
    clauses
}

fn union(a: &VarSet, b: &VarSet) -> VarSet {
    a.union(b).cloned().collect()
}

fn normalize(expr: &Expr) -> Dnf {
    match expr {
        Expr::Var(name) => Dnf::make([Clause::make(one(name), BTreeSet::new())]),
        Expr::Or(l, r) => {
            let lf = normalize(l);
            let rf = normalize(r);
            Dnf::make(lf.clauses.into_iter().chain(rf.clauses))
        }
        Expr::And(l, r) => and_formulas(&normalize(l), &normalize(r)),
        Expr::Diff(l, r) => diff_formulas(normalize(l), &normalize(r)),
    }
}

fn and_formulas(f1: &Dnf, f2: &Dnf) -> Dnf {
    let mut out = Vec::new();
    for c1 in &f1.clauses {
        for c2 in &f2.clauses {
            let c = Clause::make(union(&c1.p, &c2.p), union(&c1.n, &c2.n));
            if !c.is_empty() {
                out.push(c);
            }
        }
    }
    Dnf::make(out)
}

fn diff_formulas(f1: Dnf, f2: &Dnf) -> Dnf {
    let mut current = f1;
    // f2.clauses is a BTreeSet, already in `_clause_sort_key` order.
    for rhs in &f2.clauses {
        current = diff_formula_by_clause(&current, rhs);
    }
    current
}

fn diff_formula_by_clause(f: &Dnf, rhs: &Clause) -> Dnf {
    let mut out = Vec::new();
    for lhs in &f.clauses {
        out.extend(diff_clause_by_clause(lhs, rhs));
    }
    Dnf::make(out)
}

fn diff_clause_by_clause(lhs: &Clause, rhs: &Clause) -> Vec<Clause> {
    let mut out = Vec::new();
    // Case 1: satisfy one of rhs's negative witnesses positively.
    for m in &rhs.n {
        let c = Clause::make(union(&lhs.p, &one(m)), lhs.n.clone());
        if !c.is_empty() {
            out.push(c);
        }
    }
    // Case 2: fail one of rhs's positive requirements.
    for q in &rhs.p {
        let c = Clause::make(lhs.p.clone(), union(&lhs.n, &one(q)));
        if !c.is_empty() {
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// create_state_machine (src/trie/trie_generation.py)
// ---------------------------------------------------------------------------

pub(crate) fn build_graph(input: &FormulaInput) -> Graph {
    let dnf = normalize(&input.expr);

    let src = |v: &str| -> Ident {
        input
            .vars
            .get(v)
            .cloned()
            .unwrap_or_else(|| Ident::new(v, Span::call_site()))
    };
    let m = || Ident::new("m", Span::call_site());
    let r = input.sink.clone();

    let var_state = |v: &str| format!("s_{v}");
    let clause_state = |e: usize| format!("sc{e}");

    let vars = dnf.vars();
    let clauses: Vec<&Clause> = dnf.clauses.iter().collect();
    let dependencies = dnf.dependencies();
    let singletons = dnf.singletons();
    let pos_vars: VarSet = dnf
        .clauses
        .iter()
        .flat_map(|c| c.p.iter().cloned())
        .collect();

    let idents_of = |groups: &BTreeSet<VarSet>| -> Vec<Vec<Ident>> {
        groups
            .iter()
            .map(|g| g.iter().map(|p| src(p)).collect())
            .collect()
    };

    let mut ts: Vec<Transition> = Vec::new();
    let mut stateidx = 0usize;

    // s0 -> s1: descend every source.
    let mut t = Transition::new("s0", "s1");
    t.descend = vars.iter().map(|v| src(v)).collect();
    ts.push(t);

    for (e, clause) in clauses.iter().enumerate() {
        let ps: Vec<String> = clause.p.iter().cloned().collect();
        let p0 = ps[0].clone();
        let ps_set: VarSet = clause.p.clone();

        // s1 -> sc{e}: all positives are equal minima and not equal to a negative.
        let mut t = Transition::new("s1", clause_state(e));
        for p in &ps {
            t.when.push(Cond::IsValue(src(p)));
        }
        for q in &ps {
            t.when.push(Cond::Ineq { op: CmpOp::Eq, lhs: src(&p0), rhs: src(q) });
        }
        for v in pos_vars.difference(&ps_set) {
            t.when.push(Cond::OpOrNot { op: CmpOp::Ge, lhs: src(v), rhs: src(&p0) });
        }
        for n in &clause.n {
            t.when.push(Cond::NeIfValue { op: CmpOp::Ne, lhs: src(n), rhs: src(&p0) });
        }
        t.active = ps.iter().map(|p| src(p)).collect();
        ts.push(t);

        // sc{e} -> s_{p0}: all negatives are strictly bigger -> push.
        let mut t = Transition::new(clause_state(e), var_state(&p0));
        for n in &clause.n {
            t.when.push(Cond::OpOrEqNotValue { op: CmpOp::Gt, lhs: src(n), rhs: src(&p0) });
        }
        t.push = vec![(r.clone(), src(&p0))];
        ts.push(t);

        for n in &clause.n {
            let ns = format!("n{stateidx}");
            stateidx += 1;

            // a negative smaller than the positives -> advance it.
            let mut t = Transition::new(clause_state(e), ns.clone());
            t.when.push(Cond::Ineq { op: CmpOp::Lt, lhs: src(n), rhs: src(&p0) });
            t.active = vec![src(n)];
            ts.push(t);

            let mut t = Transition::new(ns.clone(), clause_state(e));
            t.when.push(Cond::PrefixOf(src(n), src(&p0)));
            t.active = vec![src(n)];
            t.descend = vec![src(n)];
            ts.push(t);

            let mut t = Transition::new(ns.clone(), clause_state(e));
            t.when.push(Cond::NotPrefixOf(src(n), src(&p0)));
            t.active = vec![src(n)];
            t.next_i = vec![(src(n), vec![src(&p0)])];
            ts.push(t);

            // a negative equal to the positives -> no push.
            let mut t = Transition::new(clause_state(e), "s1");
            t.when.push(Cond::IsValue(src(n)));
            t.when.push(Cond::Ineq { op: CmpOp::Eq, lhs: src(n), rhs: src(&p0) });
            t.active = vec![src(n)];
            ts.push(t);
        }
    }

    // s1 -> s2 (unconditional fallback).
    ts.push(Transition::new("s1", "s2"));

    for v in &pos_vars {
        // s2 -> s_{v}: v is one of the minimum elements.
        let mut t = Transition::new("s2", var_state(v));
        for v2 in pos_vars.iter().filter(|x| *x != v) {
            t.when.push(Cond::OpOrNot { op: CmpOp::Ge, lhs: src(v2), rhs: src(v) });
        }
        t.active = vec![src(v)];
        ts.push(t);

        for v2 in pos_vars.iter().filter(|x| *x != v) {
            let depv2 = &dependencies[v2];

            if singletons.contains(v2) || depv2.is_empty() {
                // pull the other minimum directly.
                let mut t = Transition::new(var_state(v), var_state(v));
                t.when.push(Cond::Ineq { op: CmpOp::Eq, lhs: src(v), rhs: src(v2) });
                t.active = vec![src(v2)];
                t.descend = vec![src(v2)];
                ts.push(t);
            } else if depv2.len() == 1 && depv2.iter().next().unwrap().len() == 1 {
                let target_name = depv2.iter().next().unwrap().iter().next().unwrap();
                let target = src(target_name);
                let ns = format!("n{stateidx}");
                stateidx += 1;

                let mut t = Transition::new(var_state(v), ns.clone());
                t.when.push(Cond::Ineq { op: CmpOp::Eq, lhs: src(v), rhs: src(v2) });
                t.active = vec![src(v2)];
                ts.push(t);

                let mut t = Transition::new(ns.clone(), var_state(v));
                t.when.push(Cond::Ineq { op: CmpOp::Eq, lhs: src(v), rhs: src(v2) });
                t.when.push(Cond::PrefixOf(src(v2), target.clone()));
                t.active = vec![target.clone()];
                t.descend = vec![src(v2)];
                ts.push(t);

                let mut t = Transition::new(ns.clone(), var_state(v));
                t.when.push(Cond::Ineq { op: CmpOp::Eq, lhs: src(v), rhs: src(v2) });
                t.when.push(Cond::NotPrefixOf(src(v2), target.clone()));
                t.active = vec![target.clone()];
                t.next_i = vec![(src(v2), vec![target.clone()])];
                ts.push(t);

                let mut t = Transition::new(ns.clone(), var_state(v));
                t.when.push(Cond::Finished(target));
                t.end = vec![src(v2)];
                ts.push(t);
            } else {
                let ns = format!("n{stateidx}");
                stateidx += 1;

                let mut t = Transition::new(var_state(v), ns.clone());
                t.when.push(Cond::Ineq { op: CmpOp::Eq, lhs: src(v), rhs: src(v2) });
                t.active = vec![src(v2)];
                t.define = Some((m(), idents_of(depv2)));
                ts.push(t);

                let mut t = Transition::new(ns.clone(), var_state(v));
                t.when.push(Cond::VarNone(m()));
                t.descend = vec![src(v2)];
                ts.push(t);

                let mut t = Transition::new(ns.clone(), var_state(v));
                t.when.push(Cond::PrefixOf(src(v2), m()));
                t.descend = vec![src(v2)];
                ts.push(t);

                let mut t = Transition::new(ns.clone(), var_state(v));
                t.when.push(Cond::NotPrefixOf(src(v2), m()));
                t.next_i_var = vec![(src(v2), m())];
                ts.push(t);
            }
        }

        // else branch: this var is the (only) minimum.
        if singletons.contains(v) {
            let mut t = Transition::new(var_state(v), "s1");
            t.descend = vec![src(v)];
            ts.push(t);
        } else {
            let ns = format!("n{stateidx}");
            stateidx += 1;

            let mut t = Transition::new(var_state(v), ns.clone());
            t.define = Some((m(), idents_of(&dependencies[v])));
            ts.push(t);

            let mut t = Transition::new(ns.clone(), "s1");
            t.when.push(Cond::VarNone(m()));
            t.descend = vec![src(v)];
            ts.push(t);

            let mut t = Transition::new(ns.clone(), "s1");
            t.when.push(Cond::PrefixOf(src(v), m()));
            t.descend = vec![src(v)];
            ts.push(t);

            let mut t = Transition::new(ns.clone(), "s1");
            t.when.push(Cond::NotPrefixOf(src(v), m()));
            t.next_i_var = vec![(src(v), m())];
            ts.push(t);
        }
    }

    Graph {
        sources: vars.iter().map(|v| src(v)).collect(),
        sinks: vec![r],
        init: "s0".to_string(),
        transitions: ts,
    }
}
