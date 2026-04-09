from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable
import re


# ============================================================
# AST
# ============================================================

@dataclass(frozen=True)
class Expr:
    def __and__(self, other):
        return And(self, other)

    def __or__(self, other):
        return Or(self, other)

    def __sub__(self, other):
        return Diff(self, other)


@dataclass(frozen=True)
class Var(Expr):
    name: str


@dataclass(frozen=True)
class And(Expr):
    left: Expr
    right: Expr


@dataclass(frozen=True)
class Or(Expr):
    left: Expr
    right: Expr


@dataclass(frozen=True)
class Diff(Expr):
    left: Expr
    right: Expr


# ============================================================
# Normal-form objects
# ============================================================

def _to_frozenset(xs: Iterable[str] | str) -> frozenset[str]:
    if isinstance(xs, str):
        return frozenset([xs])
    return frozenset(xs)


@dataclass(frozen=True, order=True)
class Clause:
    """
    Represents one clause:

        (⋂ P) \\ (⋃ N)

    where P and N are finite sets of variable names.
    """
    P: frozenset[str]
    N: frozenset[str]

    @staticmethod
    def make(P: Iterable[str] | str, N: Iterable[str] | str = ()) -> "Clause":
        return Clause(_to_frozenset(P), _to_frozenset(N))

    def is_empty(self) -> bool:
        # No universe constant in the language, so empty P is not allowed here.
        return (not self.P) or bool(self.P & self.N)

    def eval(self, env: dict[str, set]) -> set:
        if not self.P:
            return set()

        it = iter(self.P)
        result = set(env[next(it)])
        for p in it:
            result &= env[p]
        for n in self.N:
            result -= env[n]
        return result

    def __str__(self) -> str:
        p = " ∩ ".join(sorted(self.P))
        if not self.N:
            return f"({p})"
        n = " ∪ ".join(sorted(self.N))
        return f"(({p}) \\ ({n}))"


@dataclass(frozen=True)
class Formula:
    """
    Represents a union of clauses.
    """
    clauses: frozenset[Clause]

    @staticmethod
    def make(clauses: Iterable[Clause]) -> "Formula":
        return Formula(frozenset(clauses)).simplify()

    def eval(self, env: dict[str, set]) -> set:
        out = set()
        for c in self.clauses:
            out |= c.eval(env)
        return out

    def simplify(self) -> "Formula":
        """
        Simplify a union of clauses.

        Repeatedly:
        1. Drop empty clauses.
        2. Drop clauses redundant by subsumption:
               c ⊆ d  =>  c is redundant in a union
           where
               (P1 \\ N1) ⊆ (P2 \\ N2)  iff  P1 ⊇ P2 and N1 ⊇ N2
        3. Absorb negatives using pure positive clauses:
               (D) ∪ ((P) \\ (D ∪ N)) = (D) ∪ ((P) \\ N)
           more generally, if a clause d has d.N = ∅, then d.P may be removed
           from the negative side of every other clause.
        """
        clauses = set(self.clauses)
        changed = True

        while changed:
            changed = False

            # 1. Drop empty clauses
            new_clauses = {c for c in clauses if not c.is_empty()}
            if new_clauses != clauses:
                changed = True
                clauses = new_clauses

            # 2. Drop subsumed clauses
            reduced = set()
            for c in clauses:
                redundant = False
                for d in clauses:
                    if c == d:
                        continue
                    if clause_subset(c, d):  # c ⊆ d, so c is redundant in a union
                        redundant = True
                        break
                if not redundant:
                    reduced.add(c)

            if reduced != clauses:
                changed = True
                clauses = reduced

            # 3. Absorb negatives via pure positive clauses
            positive_clauses = [c for c in clauses if not c.N]

            absorbed = set()
            for c in clauses:
                newN = set(c.N)
                for d in positive_clauses:
                    if c != d:
                        newN -= d.P
                new_c = Clause.make(c.P, newN)
                absorbed.add(new_c)

            if absorbed != clauses:
                changed = True
                clauses = absorbed

        return Formula(frozenset(clauses))

    def __str__(self) -> str:
        if not self.clauses:
            return "∅"
        return " ∪ ".join(str(c) for c in sorted(self.clauses, key=_clause_sort_key))


def _clause_sort_key(c: Clause):
    return (tuple(sorted(c.P)), tuple(sorted(c.N)))


def clause_subset(c1: Clause, c2: Clause) -> bool:
    """
    True iff c1 ⊆ c2 as sets.

    (⋂P1 \\ ⋃N1) ⊆ (⋂P2 \\ ⋃N2)
    iff P1 ⊇ P2 and N1 ⊇ N2
    """
    return c1.P.issuperset(c2.P) and c1.N.issuperset(c2.N)


# ============================================================
# Core normalization
# ============================================================

def normalize(expr: Expr) -> Formula:
    """
    Convert any Expr built from Var, And, Or, Diff
    into difference-DNF:

        ⋃_r (⋂P_r \\ ⋃N_r)
    """
    if isinstance(expr, Var):
        return Formula.make([Clause.make([expr.name])])

    if isinstance(expr, Or):
        left = normalize(expr.left)
        right = normalize(expr.right)
        return Formula.make(left.clauses | right.clauses)

    if isinstance(expr, And):
        left = normalize(expr.left)
        right = normalize(expr.right)
        return and_formulas(left, right)

    if isinstance(expr, Diff):
        left = normalize(expr.left)
        right = normalize(expr.right)
        return diff_formulas(left, right)

    raise TypeError(f"Unknown Expr: {expr!r}")


def and_formulas(f1: Formula, f2: Formula) -> Formula:
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
    return Formula.make(out)


def diff_formulas(f1: Formula, f2: Formula) -> Formula:
    """
    Use:
        X \\ (Y ∪ Z) = (X \\ Y) \\ Z

    so subtract the RHS clauses one-by-one.
    """
    current = f1
    for rhs_clause in sorted(f2.clauses, key=_clause_sort_key):
        current = diff_formula_by_clause(current, rhs_clause)
    return current


def diff_formula_by_clause(f: Formula, rhs: Clause) -> Formula:
    """
    Subtract one clause from a formula:
        (⋃ c_i) \\ rhs = ⋃ (c_i \\ rhs)
    """
    out: list[Clause] = []
    for lhs in f.clauses:
        out.extend(diff_clause_by_clause(lhs, rhs))
    return Formula.make(out)


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

def rewrite_to_normal_form(s: str) -> Formula:
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

    print(nf)