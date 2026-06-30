//! Procedural-macro port of the trie state-machine tooling in
//! `src/trie/*.py`.
//!
//! Two macros are provided:
//!
//! * [`synth!`] — port of `Graph.rs()` in `trie_synth.py`. Takes an explicit
//!   state-machine description (sources, sinks, init, transitions) and expands
//!   to the `'dispatch: loop` it used to print.
//!
//! * [`synth_formula!`] — port of the whole `expr` → `normalize` →
//!   `create_state_machine` pipeline. Takes a set formula such as
//!   `(a | b) & c - d`, builds the state machine from it at compile time, and
//!   expands to the same dispatch loop.
//!
//! Both expand to a block expression and require, at the call site: the helper
//! functions `path`, `is_val`, `descend_or_next`, `next`, `difference_level`,
//! `prefix_of`, `argmin`, `argmax`; each source as a `RefCell<ReadZipper>`
//! binding; and each sink as a `Vec` binding.

mod formula;

use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{bracketed, parenthesized, Ident, Result, Token};

// ---------------------------------------------------------------------------
// Intermediate representation (shared by both macros)
// ---------------------------------------------------------------------------

/// Comparison operator used by the inequality / "op or …" conditions.
#[derive(Clone, Copy)]
pub(crate) enum CmpOp {
    Lt,
    Gt,
    Eq,
    Ne,
    Le,
    Ge,
}

impl CmpOp {
    fn tokens(self) -> TokenStream2 {
        match self {
            CmpOp::Lt => quote!(<),
            CmpOp::Gt => quote!(>),
            CmpOp::Eq => quote!(==),
            CmpOp::Ne => quote!(!=),
            CmpOp::Le => quote!(<=),
            CmpOp::Ge => quote!(>=),
        }
    }
}

fn parse_cmp(input: ParseStream) -> Result<CmpOp> {
    // Multi-char operators must be checked before their single-char prefixes.
    if input.peek(Token![<=]) {
        input.parse::<Token![<=]>()?;
        Ok(CmpOp::Le)
    } else if input.peek(Token![>=]) {
        input.parse::<Token![>=]>()?;
        Ok(CmpOp::Ge)
    } else if input.peek(Token![==]) {
        input.parse::<Token![==]>()?;
        Ok(CmpOp::Eq)
    } else if input.peek(Token![!=]) {
        input.parse::<Token![!=]>()?;
        Ok(CmpOp::Ne)
    } else if input.peek(Token![<]) {
        input.parse::<Token![<]>()?;
        Ok(CmpOp::Lt)
    } else if input.peek(Token![>]) {
        input.parse::<Token![>]>()?;
        Ok(CmpOp::Gt)
    } else {
        Err(input.error("expected a comparison operator (`<`, `>`, `==`, `!=`, `<=`, `>=`)"))
    }
}

/// A guard condition on a transition. Mirrors the `Cond` subclasses in
/// `trie_synth.py`. `lhs`/`rhs` are either source idents or defined-var idents.
pub(crate) enum Cond {
    Ineq { op: CmpOp, lhs: Ident, rhs: Ident },
    OpOrNot { op: CmpOp, lhs: Ident, rhs: Ident },
    NeIfValue { op: CmpOp, lhs: Ident, rhs: Ident },
    OpOrEqNotValue { op: CmpOp, lhs: Ident, rhs: Ident },
    IsValue(Ident),
    NotValue(Ident),
    PrefixOf(Ident, Ident),
    NotPrefixOf(Ident, Ident),
    VarNone(Ident),
    Finished(Ident),
    NotFinished(Ident),
}

/// One transition, mirroring `Transition` / `Vtx.to(...)` in `trie_synth.py`.
/// State endpoints are stored by name.
#[derive(Default)]
pub(crate) struct Transition {
    pub from: String,
    pub to: String,
    pub when: Vec<Cond>,
    pub active: Vec<Ident>,
    pub push: Vec<(Ident, Ident)>,
    pub descend: Vec<Ident>,
    pub next_i: Vec<(Ident, Vec<Ident>)>,
    pub next_i_var: Vec<(Ident, Ident)>,
    pub define: Option<(Ident, Vec<Vec<Ident>>)>,
    pub end: Vec<Ident>,
}

impl Transition {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Transition {
            from: from.into(),
            to: to.into(),
            ..Default::default()
        }
    }
}

/// A complete state machine ready for code generation.
pub(crate) struct Graph {
    pub sources: Vec<Ident>,
    #[allow(dead_code)]
    pub sinks: Vec<Ident>,
    pub init: String,
    pub transitions: Vec<Transition>,
}

// ---------------------------------------------------------------------------
// Code generation (port of `Graph.rs()`)
// ---------------------------------------------------------------------------

struct Codegen {
    sources: HashSet<String>,
}

impl Codegen {
    fn tmp(name: &Ident) -> Ident {
        format_ident!("tmp_{}", name)
    }

    /// Mirror of `make_ref` in `trie_synth.py`: sources are wrapped as
    /// `&Some(&name)`, defined vars are passed as `&name`.
    fn make_ref(&self, name: &Ident) -> TokenStream2 {
        if self.sources.contains(&name.to_string()) {
            quote!(&Some(&#name))
        } else {
            quote!(&#name)
        }
    }

    /// `&tmp_rhs` for sources, `&rhs` for defined vars (the `is_var` flag).
    fn prefix_rhs(&self, rhs: &Ident) -> TokenStream2 {
        if self.sources.contains(&rhs.to_string()) {
            let t = Self::tmp(rhs);
            quote!(&#t)
        } else {
            quote!(&#rhs)
        }
    }

    fn cond_tokens(&self, c: &Cond) -> TokenStream2 {
        match c {
            Cond::Ineq { op, lhs, rhs } => {
                let (l, r, o) = (self.make_ref(lhs), self.make_ref(rhs), op.tokens());
                quote!(path(#l) #o path(#r))
            }
            Cond::OpOrNot { op, lhs, rhs } => {
                let tl = Self::tmp(lhs);
                let (l, r, o) = (self.make_ref(lhs), self.make_ref(rhs), op.tokens());
                quote!((#tl.is_none() || path(#l) #o path(#r)))
            }
            Cond::NeIfValue { op, lhs, rhs } => {
                let tl = Self::tmp(lhs);
                let (l, r, o) = (self.make_ref(lhs), self.make_ref(rhs), op.tokens());
                quote!((#tl.is_none() || (path(#l) #o path(#r) || !is_val(#l))))
            }
            Cond::OpOrEqNotValue { op, lhs, rhs } => {
                let tl = Self::tmp(lhs);
                let (l, r, o) = (self.make_ref(lhs), self.make_ref(rhs), op.tokens());
                quote!((#tl.is_none() || path(#l) #o path(#r) || (path(#l) == path(#r) && !is_val(#l))))
            }
            Cond::IsValue(a) => {
                let r = self.make_ref(a);
                quote!(is_val(#r))
            }
            Cond::NotValue(a) => {
                let r = self.make_ref(a);
                quote!(!is_val(#r))
            }
            Cond::PrefixOf(a, b) => {
                let ta = Self::tmp(a);
                let rb = self.prefix_rhs(b);
                quote!(prefix_of(&#ta, #rb))
            }
            Cond::NotPrefixOf(a, b) => {
                let ta = Self::tmp(a);
                let rb = self.prefix_rhs(b);
                quote!(!prefix_of(&#ta, #rb))
            }
            Cond::VarNone(v) => quote!(#v.is_none()),
            Cond::Finished(a) => {
                let ta = Self::tmp(a);
                quote!(#ta.is_none())
            }
            Cond::NotFinished(a) => {
                let ta = Self::tmp(a);
                quote!(#ta.is_some())
            }
        }
    }

    fn define_expr(&self, values: &[Vec<Ident>]) -> TokenStream2 {
        if values.len() == 1 {
            // single group -> `EXPR.clone()`
            let g = &values[0];
            if g.len() == 1 {
                let t = Self::tmp(&g[0]);
                quote!(#t.clone())
            } else {
                let refs = g.iter().map(|e| {
                    let t = Self::tmp(e);
                    quote!(&#t)
                });
                quote!(argmax(&[#(#refs),*]).clone())
            }
        } else {
            // multiple groups -> `argmin(&[ group, ... ])`. The Python
            // generator emitted a bare `argmax(...)` here, but every array
            // element must be a `&Option<..>`; `create_state_machine` only
            // ever produced single-element groups so the mismatch never
            // surfaced. We emit `&argmax(...)` so nested groups also compile.
            let groups = values.iter().map(|g| {
                if g.len() == 1 {
                    let t = Self::tmp(&g[0]);
                    quote!(&#t)
                } else {
                    let refs = g.iter().map(|e| {
                        let t = Self::tmp(e);
                        quote!(&#t)
                    });
                    quote!(&argmax(&[#(#refs),*]))
                }
            });
            quote!(argmin(&[#(#groups),*]))
        }
    }

    /// The body of one `if <cond> { ... }` block for a transition.
    fn transition_body(&self, t: &Transition, to_id: usize) -> TokenStream2 {
        let mut body = TokenStream2::new();

        if let Some((var, values)) = &t.define {
            let expr = self.define_expr(values);
            body.extend(quote!(#var = #expr;));
        }
        for (sink, src) in &t.push {
            let ts = Self::tmp(src);
            body.extend(quote!(#sink.push(path(&#ts).to_vec());));
        }
        for src in &t.descend {
            let ts = Self::tmp(src);
            body.extend(quote!(#ts = descend_or_next(&#src);));
        }
        for (src, ds) in &t.next_i {
            let ts = Self::tmp(src);
            if ds.len() > 1 {
                let lvls = ds.iter().map(|rhs| {
                    let (s, r) = (self.make_ref(src), self.make_ref(rhs));
                    quote!(difference_level(#s, #r))
                });
                body.extend(quote!(let diff_level = [#(#lvls),*].into_iter().max().unwrap();));
            } else {
                let (s, r) = (self.make_ref(src), self.make_ref(&ds[0]));
                body.extend(quote!(let diff_level = difference_level(#s, #r);));
            }
            body.extend(quote!(#ts = next(&#src, diff_level);));
        }
        for (src, var) in &t.next_i_var {
            let ts = Self::tmp(src);
            let (s, r) = (self.make_ref(src), self.make_ref(var));
            body.extend(quote!(let diff_level = difference_level(#s, #r);));
            body.extend(quote!(#ts = next(&#src, diff_level);));
        }
        for src in &t.end {
            let ts = Self::tmp(src);
            body.extend(quote!(#ts = None;));
        }

        let to_lit = Literal::usize_unsuffixed(to_id);
        body.extend(quote!(state = #to_lit; continue 'dispatch;));
        body
    }
}

/// Generate the dispatch loop for a graph. Shared by both macros.
fn codegen(graph: &Graph) -> TokenStream2 {
    // Assign each state a numeric id in order of first appearance, with the
    // initial state always 0.
    let mut order: Vec<String> = vec![graph.init.clone()];
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(graph.init.clone());
    for t in &graph.transitions {
        for s in [&t.from, &t.to] {
            if seen.insert(s.clone()) {
                order.push(s.clone());
            }
        }
    }
    let id_of = |name: &str| -> usize {
        order
            .iter()
            .position(|s| s == name)
            .expect("state id lookup")
    };

    let cg = Codegen {
        sources: graph.sources.iter().map(|s| s.to_string()).collect(),
    };

    // Collect defined-var names in first-appearance order.
    let mut defined_vars: Vec<Ident> = Vec::new();
    let mut defined_seen: HashSet<String> = HashSet::new();
    for t in &graph.transitions {
        if let Some((var, _)) = &t.define {
            if defined_seen.insert(var.to_string()) {
                defined_vars.push(var.clone());
            }
        }
    }

    // Build one match arm per state.
    let arms = order.iter().map(|state| {
        let state_lit = Literal::usize_unsuffixed(id_of(state));
        let ifs = graph
            .transitions
            .iter()
            .filter(|t| &t.from == state)
            .map(|t| {
                // condition list: explicit `active` first, then `when`.
                let mut conds: Vec<TokenStream2> = t
                    .active
                    .iter()
                    .map(|c| {
                        let tc = Codegen::tmp(c);
                        quote!(#tc.is_some())
                    })
                    .collect();
                conds.extend(t.when.iter().map(|c| cg.cond_tokens(c)));
                let cond_expr = if conds.is_empty() {
                    quote!(true)
                } else {
                    let mut it = conds.into_iter();
                    let mut acc = it.next().unwrap();
                    for c in it {
                        acc = quote!(#acc && #c);
                    }
                    acc
                };
                let body = cg.transition_body(t, id_of(&t.to));
                quote!(if #cond_expr { #body })
            });
        quote!(#state_lit => { #(#ifs)* break 'dispatch; })
    });

    let tmp_decls = graph.sources.iter().map(|s| {
        let t = Codegen::tmp(s);
        quote!(let mut #t = None;)
    });
    let init_lit = Literal::usize_unsuffixed(id_of(&graph.init));

    quote! {
        {
            #(let mut #defined_vars = None;)*
            #(#tmp_decls)*
            let mut state = #init_lit;
            'dispatch: loop {
                match state {
                    #(#arms,)*
                    unk_state => unreachable!("invalid state {}", unk_state),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// `synth!` — explicit graph DSL
// ---------------------------------------------------------------------------

/// `name = [ a, b, c ]`
fn parse_ident_list(input: ParseStream) -> Result<Vec<Ident>> {
    let content;
    bracketed!(content in input);
    let mut v = Vec::new();
    while !content.is_empty() {
        v.push(content.parse()?);
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(v)
}

/// `push = [ (sink, src), ... ]`
fn parse_pairs(input: ParseStream) -> Result<Vec<(Ident, Ident)>> {
    let content;
    bracketed!(content in input);
    let mut v = Vec::new();
    while !content.is_empty() {
        let inner;
        parenthesized!(inner in content);
        let a: Ident = inner.parse()?;
        inner.parse::<Token![,]>()?;
        let b: Ident = inner.parse()?;
        v.push((a, b));
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(v)
}

/// `next_i = [ a -> (c), c -> (a, b) ]`
fn parse_next_i(input: ParseStream) -> Result<Vec<(Ident, Vec<Ident>)>> {
    let content;
    bracketed!(content in input);
    let mut v = Vec::new();
    while !content.is_empty() {
        let src: Ident = content.parse()?;
        content.parse::<Token![->]>()?;
        let inner;
        parenthesized!(inner in content);
        let mut ds = Vec::new();
        while !inner.is_empty() {
            ds.push(inner.parse()?);
            if inner.peek(Token![,]) {
                inner.parse::<Token![,]>()?;
            }
        }
        v.push((src, ds));
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(v)
}

/// `next_i_var = [ c -> m ]`
fn parse_next_i_var(input: ParseStream) -> Result<Vec<(Ident, Ident)>> {
    let content;
    bracketed!(content in input);
    let mut v = Vec::new();
    while !content.is_empty() {
        let src: Ident = content.parse()?;
        content.parse::<Token![->]>()?;
        let var: Ident = content.parse()?;
        v.push((src, var));
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(v)
}

/// Parse one `argmax(...)`/single-ident group inside `define`.
fn parse_group(input: ParseStream) -> Result<Vec<Ident>> {
    let head: Ident = input.parse()?;
    if head == "argmax" && input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        let mut ids = Vec::new();
        while !content.is_empty() {
            ids.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(ids)
    } else {
        Ok(vec![head])
    }
}

/// `define = m = argmin(argmax(a, b), c)` / `m = argmax(a, b)` / `m = a`.
/// The leading `define =` has already been consumed.
fn parse_define(input: ParseStream) -> Result<(Ident, Vec<Vec<Ident>>)> {
    let var: Ident = input.parse()?;
    input.parse::<Token![=]>()?;
    let head: Ident = input.parse()?;
    let values = if head == "argmin" && input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        let mut groups = Vec::new();
        while !content.is_empty() {
            groups.push(parse_group(&content)?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        groups
    } else if head == "argmax" && input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        let mut ids = Vec::new();
        while !content.is_empty() {
            ids.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        vec![ids]
    } else {
        vec![vec![head]]
    };
    Ok((var, values))
}

/// Parse a function-form condition `name(...)`. `name` is already parsed.
fn parse_fn_cond(name: Ident, input: ParseStream) -> Result<Cond> {
    let content;
    parenthesized!(content in input);
    let s = name.to_string();
    let cond = match s.as_str() {
        "finished" => Cond::Finished(content.parse()?),
        "not_finished" => Cond::NotFinished(content.parse()?),
        "is_value" => Cond::IsValue(content.parse()?),
        "not_value" => Cond::NotValue(content.parse()?),
        "var_none" => Cond::VarNone(content.parse()?),
        "prefix_of" | "not_prefix_of" => {
            let a: Ident = content.parse()?;
            content.parse::<Token![,]>()?;
            let b: Ident = content.parse()?;
            if s == "prefix_of" {
                Cond::PrefixOf(a, b)
            } else {
                Cond::NotPrefixOf(a, b)
            }
        }
        "op_or_not" | "ne_if_value" | "op_or_eq_not_value" => {
            let a: Ident = content.parse()?;
            let op = parse_cmp(&content)?;
            let b: Ident = content.parse()?;
            match s.as_str() {
                "op_or_not" => Cond::OpOrNot { op, lhs: a, rhs: b },
                "ne_if_value" => Cond::NeIfValue { op, lhs: a, rhs: b },
                _ => Cond::OpOrEqNotValue { op, lhs: a, rhs: b },
            }
        }
        _ => return Err(syn::Error::new(name.span(), format!("unknown condition `{s}`"))),
    };
    Ok(cond)
}

/// Parse one comma-separated argument of a transition: either an action
/// (`kw = [...]`) or a positional `when` condition.
fn parse_arg(t: &mut Transition, input: ParseStream) -> Result<()> {
    let name: Ident = input.parse()?;
    if input.peek(syn::token::Paren) {
        // function-form condition: not_finished(a), prefix_of(a, b), ...
        t.when.push(parse_fn_cond(name, input)?);
    } else if input.peek(Token![=]) && !input.peek(Token![==]) {
        // action keyword: `kw = ...`
        input.parse::<Token![=]>()?;
        match name.to_string().as_str() {
            "descend" => t.descend = parse_ident_list(input)?,
            "active" => t.active = parse_ident_list(input)?,
            "end" => t.end = parse_ident_list(input)?,
            "push" => t.push = parse_pairs(input)?,
            "next_i" => t.next_i = parse_next_i(input)?,
            "next_i_var" => t.next_i_var = parse_next_i_var(input)?,
            "define" => t.define = Some(parse_define(input)?),
            other => {
                return Err(syn::Error::new(
                    name.span(),
                    format!("unknown transition action `{other}`"),
                ))
            }
        }
    } else {
        // inequality condition: a < b
        let op = parse_cmp(input)?;
        let rhs: Ident = input.parse()?;
        t.when.push(Cond::Ineq { op, lhs: name, rhs });
    }
    Ok(())
}

impl Parse for Graph {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut sources = Vec::new();
        let mut sinks = Vec::new();
        let mut init: Option<String> = None;
        let mut transitions = Vec::new();

        while !input.is_empty() {
            let head: Ident = input.parse()?;
            match head.to_string().as_str() {
                "sources" => {
                    input.parse::<Token![:]>()?;
                    while !input.peek(Token![;]) {
                        sources.push(input.parse()?);
                        if input.peek(Token![,]) {
                            input.parse::<Token![,]>()?;
                        }
                    }
                    input.parse::<Token![;]>()?;
                }
                "sinks" => {
                    input.parse::<Token![:]>()?;
                    while !input.peek(Token![;]) {
                        sinks.push(input.parse()?);
                        if input.peek(Token![,]) {
                            input.parse::<Token![,]>()?;
                        }
                    }
                    input.parse::<Token![;]>()?;
                }
                "init" => {
                    input.parse::<Token![:]>()?;
                    let id: Ident = input.parse()?;
                    init = Some(id.to_string());
                    input.parse::<Token![;]>()?;
                }
                _ => {
                    // a transition: FROM -> TO [: args] ;   (head is FROM)
                    input.parse::<Token![->]>()?;
                    let to: Ident = input.parse()?;
                    let mut t = Transition::new(head.to_string(), to.to_string());
                    if input.peek(Token![:]) {
                        input.parse::<Token![:]>()?;
                        while !input.peek(Token![;]) {
                            parse_arg(&mut t, input)?;
                            if input.peek(Token![,]) {
                                input.parse::<Token![,]>()?;
                            }
                        }
                    }
                    input.parse::<Token![;]>()?;
                    transitions.push(t);
                }
            }
        }

        let init = init.ok_or_else(|| input.error("missing `init: <state>;` declaration"))?;
        Ok(Graph {
            sources,
            sinks,
            init,
            transitions,
        })
    }
}

#[proc_macro]
pub fn synth(input: TokenStream) -> TokenStream {
    let graph = syn::parse_macro_input!(input as Graph);
    codegen(&graph).into()
}

// ---------------------------------------------------------------------------
// `synth_formula!` — build the graph from a set formula
// ---------------------------------------------------------------------------

#[proc_macro]
pub fn synth_formula(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as formula::FormulaInput);
    let graph = formula::build_graph(&parsed);
    codegen(&graph).into()
}
