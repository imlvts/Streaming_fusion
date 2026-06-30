//! Runtime port of the formula → state-machine pipeline (the same algorithm as
//! `synth_macro::formula`, but operating on `&str` input and `usize` source
//! indices instead of `syn` tokens). Produces a [`Graph`].

use std::collections::{BTreeMap, BTreeSet};

use crate::{Cmp, Cond, Graph, Ref, Transition};

// ---------------------------------------------------------------------------
// Expr + tokenizer + parser (port of src/normalize.py)
// ---------------------------------------------------------------------------

enum Expr {
    Var(String),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Diff(Box<Expr>, Box<Expr>),
}

#[derive(PartialEq)]
enum Tok {
    Ident(String),
    And,
    Or,
    Diff,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(&ch) = it.peek() {
        match ch {
            c if c.is_whitespace() => {
                it.next();
            }
            '&' => {
                it.next();
                toks.push(Tok::And);
            }
            '|' => {
                it.next();
                toks.push(Tok::Or);
            }
            '-' => {
                it.next();
                toks.push(Tok::Diff);
            }
            '(' => {
                it.next();
                toks.push(Tok::LParen);
            }
            ')' => {
                it.next();
                toks.push(Tok::RParen);
            }
            c if c.is_ascii_alphanumeric() || c == '_' => {
                let mut name = String::new();
                while let Some(&c) = it.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        name.push(c);
                        it.next();
                    } else {
                        break;
                    }
                }
                toks.push(Tok::Ident(name));
            }
            other => return Err(format!("unexpected character {other:?}")),
        }
    }
    Ok(toks)
}

/// Precedence (matching the Python parser): `&` tightest, then `-`, then `|`.
struct Parser {
    toks: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_diff()?;
        while self.peek() == Some(&Tok::Or) {
            self.i += 1;
            let r = self.parse_diff()?;
            e = Expr::Or(Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_diff(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_and()?;
        while self.peek() == Some(&Tok::Diff) {
            self.i += 1;
            let r = self.parse_and()?;
            e = Expr::Diff(Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_atom()?;
        while self.peek() == Some(&Tok::And) {
            self.i += 1;
            let r = self.parse_atom()?;
            e = Expr::And(Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Tok::LParen) => {
                self.i += 1;
                let e = self.parse_or()?;
                if self.peek() != Some(&Tok::RParen) {
                    return Err("expected `)`".into());
                }
                self.i += 1;
                Ok(e)
            }
            Some(Tok::Ident(_)) => {
                let name = match &self.toks[self.i] {
                    Tok::Ident(n) => n.clone(),
                    _ => unreachable!(),
                };
                self.i += 1;
                Ok(Expr::Var(name))
            }
            _ => Err("expected identifier or `(`".into()),
        }
    }
}

fn parse_expr(s: &str) -> Result<Expr, String> {
    let mut p = Parser {
        toks: tokenize(s)?,
        i: 0,
    };
    let e = p.parse_or()?;
    if p.i != p.toks.len() {
        return Err("trailing tokens after expression".into());
    }
    Ok(e)
}

// ---------------------------------------------------------------------------
// Clause / DNF / normalize (port of src/clause.py + src/normalize.py)
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
    fn is_empty(&self) -> bool {
        self.p.is_empty() || !self.p.is_disjoint(&self.n)
    }
}

fn one(name: &str) -> VarSet {
    let mut s = BTreeSet::new();
    s.insert(name.to_string());
    s
}

fn union(a: &VarSet, b: &VarSet) -> VarSet {
    a.union(b).cloned().collect()
}

fn clause_subset(c1: &Clause, c2: &Clause) -> bool {
    c1.p.is_superset(&c2.p) && c1.n.is_superset(&c2.n)
}

struct Dnf {
    clauses: BTreeSet<Clause>,
}

impl Dnf {
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
            .filter(|v| self.clauses.iter().any(|c| c.p.len() == 1 && c.p.contains(v)))
            .collect()
    }
}

fn simplify(mut clauses: BTreeSet<Clause>) -> BTreeSet<Clause> {
    loop {
        let mut changed = false;

        let new: BTreeSet<Clause> = clauses.iter().filter(|c| !c.is_empty()).cloned().collect();
        if new != clauses {
            changed = true;
            clauses = new;
        }

        let reduced: BTreeSet<Clause> = clauses
            .iter()
            .filter(|c| !clauses.iter().any(|d| d != *c && clause_subset(c, d)))
            .cloned()
            .collect();
        if reduced != clauses {
            changed = true;
            clauses = reduced;
        }

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
    for m in &rhs.n {
        let c = Clause::make(union(&lhs.p, &one(m)), lhs.n.clone());
        if !c.is_empty() {
            out.push(c);
        }
    }
    for q in &rhs.p {
        let c = Clause::make(lhs.p.clone(), union(&lhs.n, &one(q)));
        if !c.is_empty() {
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// create_state_machine (port of src/trie/trie_generation.py)
// ---------------------------------------------------------------------------

/// Builder transition with state endpoints kept by name; resolved to ids at the
/// end.
struct Tb {
    from: String,
    to: String,
    t: Transition,
}

/// Build an executable [`Graph`] from a set formula string (operators `&`, `|`,
/// `-`). Returns an error on a malformed formula.
pub fn build_graph(formula: &str) -> Result<Graph, String> {
    let dnf = normalize(&parse_expr(formula)?);

    let vars: Vec<String> = dnf.vars().into_iter().collect(); // sorted
    let idx: BTreeMap<String, usize> = vars
        .iter()
        .enumerate()
        .map(|(i, v)| (v.clone(), i))
        .collect();
    let sid = |v: &str| idx[v];

    let clauses: Vec<&Clause> = dnf.clauses.iter().collect();
    let dependencies = dnf.dependencies();
    let singletons = dnf.singletons();
    let pos_vars: VarSet = dnf.clauses.iter().flat_map(|c| c.p.iter().cloned()).collect();

    let groups_idx = |groups: &BTreeSet<VarSet>| -> Vec<Vec<usize>> {
        groups
            .iter()
            .map(|g| g.iter().map(|p| sid(p)).collect())
            .collect()
    };

    let var_state = |v: &str| format!("s_{v}");
    let clause_state = |e: usize| format!("sc{e}");

    let mut tbs: Vec<Tb> = Vec::new();
    let mut stateidx = 0usize;
    let mut add = |from: String, to: String, t: Transition| tbs.push(Tb { from, to, t });

    // s0 -> s1: descend every source.
    add(
        "s0".into(),
        "s1".into(),
        Transition {
            descend: vars.iter().map(|v| sid(v)).collect(),
            ..Default::default()
        },
    );

    for (e, clause) in clauses.iter().enumerate() {
        let ps: Vec<String> = clause.p.iter().cloned().collect();
        let p0 = ps[0].clone();

        // s1 -> sc{e}
        let mut t = Transition::default();
        for p in &ps {
            t.when.push(Cond::IsValue(sid(p)));
        }
        for q in &ps {
            t.when.push(Cond::Ineq(Cmp::Eq, sid(&p0), sid(q)));
        }
        for v in pos_vars.difference(&clause.p) {
            t.when.push(Cond::OpOrNot(Cmp::Ge, sid(v), sid(&p0)));
        }
        for n in &clause.n {
            t.when.push(Cond::NeIfValue(Cmp::Ne, sid(n), sid(&p0)));
        }
        t.active = ps.iter().map(|p| sid(p)).collect();
        add("s1".into(), clause_state(e), t);

        // sc{e} -> s_{p0}: push
        let mut t = Transition::default();
        for n in &clause.n {
            t.when.push(Cond::OpOrEqNotValue(Cmp::Gt, sid(n), sid(&p0)));
        }
        t.push = vec![sid(&p0)];
        add(clause_state(e), var_state(&p0), t);

        for n in &clause.n {
            let ns = format!("n{stateidx}");
            stateidx += 1;

            let mut t = Transition::default();
            t.when.push(Cond::Ineq(Cmp::Lt, sid(n), sid(&p0)));
            t.active = vec![sid(n)];
            add(clause_state(e), ns.clone(), t);

            let mut t = Transition::default();
            t.when.push(Cond::PrefixOf(sid(n), Ref::Src(sid(&p0))));
            t.active = vec![sid(n)];
            t.descend = vec![sid(n)];
            add(ns.clone(), clause_state(e), t);

            let mut t = Transition::default();
            t.when.push(Cond::NotPrefixOf(sid(n), Ref::Src(sid(&p0))));
            t.active = vec![sid(n)];
            t.next_i = vec![(sid(n), vec![sid(&p0)])];
            add(ns.clone(), clause_state(e), t);

            let mut t = Transition::default();
            t.when.push(Cond::IsValue(sid(n)));
            t.when.push(Cond::Ineq(Cmp::Eq, sid(n), sid(&p0)));
            t.active = vec![sid(n)];
            add(clause_state(e), "s1".into(), t);
        }
    }

    // s1 -> s2 (unconditional fallback).
    add("s1".into(), "s2".into(), Transition::default());

    for v in &pos_vars {
        // s2 -> s_{v}
        let mut t = Transition::default();
        for v2 in pos_vars.iter().filter(|x| *x != v) {
            t.when.push(Cond::OpOrNot(Cmp::Ge, sid(v2), sid(v)));
        }
        t.active = vec![sid(v)];
        add("s2".into(), var_state(v), t);

        for v2 in pos_vars.iter().filter(|x| *x != v) {
            let depv2 = &dependencies[v2];

            if singletons.contains(v2) || depv2.is_empty() {
                let mut t = Transition::default();
                t.when.push(Cond::Ineq(Cmp::Eq, sid(v), sid(v2)));
                t.active = vec![sid(v2)];
                t.descend = vec![sid(v2)];
                add(var_state(v), var_state(v), t);
            } else if depv2.len() == 1 && depv2.iter().next().unwrap().len() == 1 {
                let target = sid(depv2.iter().next().unwrap().iter().next().unwrap());
                let ns = format!("n{stateidx}");
                stateidx += 1;

                let mut t = Transition::default();
                t.when.push(Cond::Ineq(Cmp::Eq, sid(v), sid(v2)));
                t.active = vec![sid(v2)];
                add(var_state(v), ns.clone(), t);

                let mut t = Transition::default();
                t.when.push(Cond::Ineq(Cmp::Eq, sid(v), sid(v2)));
                t.when.push(Cond::PrefixOf(sid(v2), Ref::Src(target)));
                t.active = vec![target];
                t.descend = vec![sid(v2)];
                add(ns.clone(), var_state(v), t);

                let mut t = Transition::default();
                t.when.push(Cond::Ineq(Cmp::Eq, sid(v), sid(v2)));
                t.when.push(Cond::NotPrefixOf(sid(v2), Ref::Src(target)));
                t.active = vec![target];
                t.next_i = vec![(sid(v2), vec![target])];
                add(ns.clone(), var_state(v), t);

                let mut t = Transition::default();
                t.when.push(Cond::Finished(target));
                t.end = vec![sid(v2)];
                add(ns.clone(), var_state(v), t);
            } else {
                let ns = format!("n{stateidx}");
                stateidx += 1;

                let mut t = Transition::default();
                t.when.push(Cond::Ineq(Cmp::Eq, sid(v), sid(v2)));
                t.active = vec![sid(v2)];
                t.define = Some(groups_idx(depv2));
                add(var_state(v), ns.clone(), t);

                let mut t = Transition::default();
                t.when.push(Cond::VarNone);
                t.descend = vec![sid(v2)];
                add(ns.clone(), var_state(v), t);

                let mut t = Transition::default();
                t.when.push(Cond::PrefixOf(sid(v2), Ref::M));
                t.descend = vec![sid(v2)];
                add(ns.clone(), var_state(v), t);

                let mut t = Transition::default();
                t.when.push(Cond::NotPrefixOf(sid(v2), Ref::M));
                t.next_i_var = vec![sid(v2)];
                add(ns.clone(), var_state(v), t);
            }
        }

        // else branch
        if singletons.contains(v) {
            let mut t = Transition::default();
            t.descend = vec![sid(v)];
            add(var_state(v), "s1".into(), t);
        } else {
            let ns = format!("n{stateidx}");
            stateidx += 1;

            let mut t = Transition::default();
            t.define = Some(groups_idx(&dependencies[v]));
            add(var_state(v), ns.clone(), t);

            let mut t = Transition::default();
            t.when.push(Cond::VarNone);
            t.descend = vec![sid(v)];
            add(ns.clone(), "s1".into(), t);

            let mut t = Transition::default();
            t.when.push(Cond::PrefixOf(sid(v), Ref::M));
            t.descend = vec![sid(v)];
            add(ns.clone(), "s1".into(), t);

            let mut t = Transition::default();
            t.when.push(Cond::NotPrefixOf(sid(v), Ref::M));
            t.next_i_var = vec![sid(v)];
            add(ns.clone(), "s1".into(), t);
        }
    }

    Ok(resolve(tbs, vars))
}

/// Resolve state names to ids (init = "s0" first, then first appearance) and
/// bucket transitions by source state.
fn resolve(tbs: Vec<Tb>, source_names: Vec<String>) -> Graph {
    let mut order: Vec<String> = vec!["s0".to_string()];
    let mut id_of: BTreeMap<String, usize> = BTreeMap::new();
    id_of.insert("s0".to_string(), 0);
    let intern = |name: &str, order: &mut Vec<String>, id_of: &mut BTreeMap<String, usize>| {
        if !id_of.contains_key(name) {
            id_of.insert(name.to_string(), order.len());
            order.push(name.to_string());
        }
    };
    for tb in &tbs {
        intern(&tb.from, &mut order, &mut id_of);
        intern(&tb.to, &mut order, &mut id_of);
    }

    let mut states: Vec<Vec<Transition>> = (0..order.len()).map(|_| Vec::new()).collect();
    for mut tb in tbs {
        tb.t.to = id_of[&tb.to];
        states[id_of[&tb.from]].push(tb.t);
    }

    Graph {
        source_names,
        init: 0,
        states,
    }
}
