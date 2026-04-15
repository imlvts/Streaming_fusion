# ============================================================
# Normal-form objects
# ============================================================
from typing import Iterable

import re

from src.clause import DNF, Clause
from src.expr import Var, Expr, And, Diff, Or


# ============================================================
# Normal-form objects
# ============================================================

def _to_frozenset(xs: Iterable[str] | str) -> frozenset[str]:
    if isinstance(xs, str):
        return frozenset([xs])
    return frozenset(xs)


def _clause_sort_key(c: Clause):
    return (tuple(sorted(c.P)), tuple(sorted(c.N)))


# ============================================================
# Core normalization
# ============================================================

def normalize(expr: Expr) -> DNF:
    """
    Convert any Expr built from Var, And, Or, Diff
    into difference-DNF:

        ⋃_r (⋂P_r \\ ⋃N_r)
    """
    if isinstance(expr, Var):
        return DNF.make([Clause.make([expr.name])])

    if isinstance(expr, Or):
        left = normalize(expr.left)
        right = normalize(expr.right)
        return DNF.make(left.clauses | right.clauses)

    if isinstance(expr, And):
        left = normalize(expr.left)
        right = normalize(expr.right)
        return and_formulas(left, right)

    if isinstance(expr, Diff):
        left = normalize(expr.left)
        right = normalize(expr.right)
        return diff_formulas(left, right)

    raise TypeError(f"Unknown Expr: {expr!r}")


def and_formulas(f1: DNF, f2: DNF) -> DNF:
    """
    Distribute intersection over unions:
        (⋃ c_i) ∩ (⋃ d_j) = ⋃ (c_i ∩ d_j)

    and
        (⋂P1 \\ ⋃N1) ∩ (⋂P2 \\ ⋃N2)
        = (⋂(P1∪P2)) \\ (⋃(N1∪N2))
    """
    out: list[Clause] = []
    for c1 in f1.clauses:
        for c2 in f2.clauses:
            c = Clause.make(c1.P | c2.P, c1.N | c2.N)
            if not c.is_empty():
                out.append(c)
    return DNF.make(out)


def diff_formulas(f1: DNF, f2: DNF) -> DNF:
    """
    Use:
        X \\ (Y ∪ Z) = (X \\ Y) \\ Z

    so subtract the RHS clauses one-by-one.
    """
    current = f1
    for rhs_clause in sorted(f2.clauses, key=_clause_sort_key):
        current = diff_formula_by_clause(current, rhs_clause)
    return current


def diff_formula_by_clause(f: DNF, rhs: Clause) -> DNF:
    """
    Subtract one clause from a formula:
        (⋃ c_i) \\ rhs = ⋃ (c_i \\ rhs)
    """
    out: list[Clause] = []
    for lhs in f.clauses:
        out.extend(diff_clause_by_clause(lhs, rhs))
    return DNF.make(out)


def diff_clause_by_clause(lhs: Clause, rhs: Clause) -> list[Clause]:
    r"""
    Compute:

        lhs \ rhs

    where
        lhs = (⋂P \\ ⋃N)
        rhs = (⋂Q \\ ⋃M)

    Key identity:
        not( (⋂Q) \ (⋃M) )
        = (⋁_{m∈M} m)  ∨  (⋁_{q∈Q} not q)

    Therefore:
        lhs \ rhs
        =
        ⋃_{m∈M}  (lhs ∩ m)
        ∪
        ⋃_{q∈Q}  (lhs \ q)

    In clause form:
        - "lhs ∩ m" means add m to positives
        - "lhs \ q" means add q to negatives
    """
    out: list[Clause] = []

    # Case 1: satisfy one of rhs's negative witnesses positively
    for m in rhs.N:
        c = Clause.make(lhs.P | {m}, lhs.N)
        if not c.is_empty():
            out.append(c)

    # Case 2: fail one of rhs's positive requirements
    for q in rhs.P:
        c = Clause.make(lhs.P, lhs.N | {q})
        if not c.is_empty():
            out.append(c)

    return out


# ============================================================
# Parser
# ============================================================

TOKEN_RE = re.compile(
    r"""
    \s*
    (
        /\            |   # conjunction
        \\/           |   # disjunction
        \\            |   # difference
        \(|\)         |   # parentheses
        [A-Za-z_][A-Za-z0-9_]*   # identifiers
    )
    """,
    re.VERBOSE,
)


def tokenize(s: str) -> list[str]:
    pos = 0
    tokens: list[str] = []
    while pos < len(s):
        m = TOKEN_RE.match(s, pos)
        if not m:
            raise SyntaxError(f"Unexpected input near: {s[pos:pos+20]!r}")
        tokens.append(m.group(1))
        pos = m.end()
    return tokens


class Parser:
    """
    Grammar:

        expr   := diff ( | diff )*
        diff   := and_expr ( - and_expr )*
        and_expr := atom ( & atom )*
        atom   := IDENT | '(' expr ')'

    Precedence:
        &   highest
        -    middle
        |   lowest

    Difference is parsed left-associatively:
        a - b - c   =  (a - b) - c
    """
    def __init__(self, tokens: list[str]):
        self.tokens = tokens
        self.i = 0

    def peek(self) -> str | None:
        if self.i >= len(self.tokens):
            return None
        return self.tokens[self.i]

    def pop(self, expected: str | None = None) -> str:
        tok = self.peek()
        if tok is None:
            raise SyntaxError("Unexpected end of input")
        if expected is not None and tok != expected:
            raise SyntaxError(f"Expected {expected!r}, got {tok!r}")
        self.i += 1
        return tok

    def parse(self) -> Expr:
        expr = self.parse_or()
        if self.peek() is not None:
            raise SyntaxError(f"Unexpected token: {self.peek()!r}")
        return expr

    def parse_or(self) -> Expr:
        expr = self.parse_diff()
        while self.peek() == r"\/":
            self.pop(r"\/")
            expr = Or(expr, self.parse_diff())
        return expr

    def parse_diff(self) -> Expr:
        expr = self.parse_and()
        while self.peek() == "\\":
            self.pop("\\")
            expr = Diff(expr, self.parse_and())
        return expr

    def parse_and(self) -> Expr:
        expr = self.parse_atom()
        while self.peek() == "/\\":
            self.pop("/\\")
            expr = And(expr, self.parse_atom())
        return expr

    def parse_atom(self) -> Expr:
        tok = self.peek()
        if tok is None:
            raise SyntaxError("Unexpected end of input")

        if tok == "(":
            self.pop("(")
            expr = self.parse_or()
            self.pop(")")
            return expr

        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", tok):
            self.pop()
            return Var(tok)

        raise SyntaxError(f"Unexpected token: {tok!r}")


def parse_expr(s: str) -> Expr:
    return Parser(tokenize(s)).parse()


# ============================================================
# Convenience API
# ============================================================

def rewrite_to_normal_form(s: str) -> DNF:
    expr = parse_expr(s)
    return normalize(expr)

def var(name: str) -> Var:
    return Var(name)



if __name__ == '__main__':
    a = var("a")
    b = var("b")
    c = var("c")
    d = var("d")
    expr = (a | (b - (c | a)))
    nf = normalize(expr)

    print(nf.show())