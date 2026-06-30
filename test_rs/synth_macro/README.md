# `synth_macro` — trie state-machine proc macros

A procedural-macro port of the trie tooling in `src/trie/*.py`. Two macros:

* [`synth!`](#synth--explicit-graph) — port of `Graph.rs()` in
  `trie_synth.py`. Takes an explicit state-machine description and expands, at
  compile time, to the `'dispatch: loop` that `rs()` used to print as text.
* [`synth_formula!`](#synth_formula--from-a-set-formula) — port of the whole
  `expr` → `normalize` → `create_state_machine` pipeline. Takes a set formula
  such as `(a | b) & c - d`, builds the state machine at compile time, and
  expands to the same dispatch loop.

Runnable demos: `../examples/ctx.rs`, `../examples/real_graph.rs`,
`../examples/formula.rs`, `../examples/feature_check.rs`.

## `synth_formula!` — from a set formula

```rust
synth_formula! {
    sink: r;
    formula: (a | b) & c - d;
}
```

The formula is parsed with the Python operator precedence (`&` tightest, then
`-`, then `|` — **not** Rust's native precedence), normalized to
difference-DNF, and compiled to a state machine via the `create_state_machine`
algorithm. Sources are taken from the formula's variables; each must be in
scope as a `RefCell<ReadZipper>` binding, plus the `sink` `Vec`. Operators:
`&` = intersection, `|` = union, `-` = difference.

## `synth!` — explicit graph

A procedural-macro port of the Rust code generator in
`src/trie/trie_synth.py` (the `Graph.rs()` method). Instead of emitting Rust
*text* from Python, `synth!` takes the state-machine description directly and
expands, at compile time, to the same `'dispatch: loop` that `rs()` used to
print.

## Usage

The macro expands to a block expression. The following must be in scope at the
call site (the example crate brings them in from `shim`):

- the helper functions `path`, `is_val`, `descend_or_next`, `next`,
  `difference_level`, `prefix_of`, `argmin`, `argmax`;
- each **source** as a `RefCell<ReadZipper>` binding (e.g. `let a = RefCell::new(map.read_zipper());`);
- each **sink** as a `Vec` binding (e.g. `let mut r = Vec::new();`).

```rust
synth! {
    sources: a, b, c, d;
    sinks: r;
    init: s0;

    s0 -> s1: descend = [a, b, c, d];
    s1 -> s2: not_finished(a), not_finished(c), a < c;
    s2 -> s1: prefix_of(a, c), descend = [a];
    s2 -> s1: not_prefix_of(a, c), next_i = [a -> (c)];
    s7 -> s1: not_finished(d), a < d, push = [(r, a)], descend = [a];
}
```

See `../examples/ctx.rs` (runnable end-to-end), `../examples/real_graph.rs`
(the real `create_state_machine` graph), and `../examples/feature_check.rs`
(every codegen path).

## Grammar

```
synth! {
    sources: <ident>, ... ;
    sinks:   <ident>, ... ;
    init:    <state> ;

    <state> -> <state> [ : <arg>, <arg>, ... ] ;
    ...
}
```

Each `<arg>` is either a **condition** (positional `when`) or an **action**
(`kw = ...`). A transition with no args is just `from -> to;`.

### Conditions (map to `trie_synth.py` `Cond` subclasses)

| DSL | Python `Cond` |
| --- | --- |
| `a < b`, `a > b`, `a == b`, `a != b` | `Inequality` |
| `finished(a)` / `not_finished(a)` | `Finished` / `NotFinished` |
| `is_value(a)` / `not_value(a)` | `IsValue` / `NotValue` |
| `prefix_of(a, b)` / `not_prefix_of(a, b)` | `PrefixOf` / `NotPrefixOf` |
| `var_none(m)` | `VarNone` |
| `op_or_not(a < b)` | `OpOrNot` |
| `ne_if_value(a < b)` | `NEIfValue` |
| `op_or_eq_not_value(a < b)` | `OpOrEqNotValue` |

For `prefix_of`/`not_prefix_of`, an identifier declared as a `source` is
referenced as `tmp_<name>`; any other identifier is a defined var (the Python
`is_var=True` case) and is referenced directly.

### Actions (map to `Vtx.to(...)` keyword args)

| DSL | Python kwarg |
| --- | --- |
| `descend = [a, b]` | `descend` |
| `active = [a, c]` | `active` |
| `end = [a]` | `end` |
| `push = [(r, a), ...]` | `push` (sink, src) |
| `next_i = [a -> (c), c -> (a, b)]` | `next_i` |
| `next_i_var = [c -> m]` | `next_i_var` |
| `define = m = argmin(argmax(a, b), c)` | `define_to_approach` |

`define` forms: `m = a` (single), `m = argmax(a, b)` (one group → `argmax`),
`m = argmin(g, ...)` where each group `g` is `id` or `argmax(id, ...)`.

## Intentional deviation from the Python output

In the multi-group `argmin` case, the Python generator emitted a bare
`argmax(...)` element. Every element of the `argmin` slice must be a
`&Option<..>`, so a bare `argmax(...)` (which returns `Option<..>`) does not
type-check. `create_state_machine` only ever produced single-element groups, so
the original never hit this; the macro emits `&argmax(...)` so nested groups
compile. Output is byte-for-byte identical for every form the Python generator
actually produced.
