from dataclasses import dataclass
from typing import DefaultDict, Iterable

from src.trie.trie import bittriemap, BitTrieMap


def _to_frozenset(xs: Iterable[str] | str) -> frozenset[str]:
    if isinstance(xs, str):
        return frozenset([xs])
    return frozenset(xs)

@dataclass(frozen=True)
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
        P = _to_frozenset(P)
        N = _to_frozenset(N)
        # if P & N:
        #     raise ValueError(f"inconsistent clause: {P=} {N=}")
        return Clause(P, N)

    def is_empty(self) -> bool:
        return not self.P or bool(self.P & self.N)

    def show(self) -> str:
        p = " ∩ ".join(sorted(self.P)) if self.P else "∅"
        n = " ∪ ".join(sorted(self.N)) if self.N else "∅"
        if self.N:
            return f"(({p}) \\ ({n}))"
        return f"({p})"

    def eval(self, env: dict[str, set]) -> set:
        if not self.P:
            return set()
        it = iter(self.P)
        result = env[next(it)]
        if not isinstance(result, BitTrieMap):
            result = set(result)
        for p in self.P:
            result &= env[p]
        for n in self.N:
            result -= env[n]
        return result

def _clause_subset(c1: Clause, c2: Clause) -> bool:
    """
    True iff c1 ⊆ c2 as sets.

    (⋂P1 \\ ⋃N1) ⊆ (⋂P2 \\ ⋃N2)
    iff P1 ⊇ P2 and N1 ⊇ N2
    """
    return c1.P.issuperset(c2.P) and c1.N.issuperset(c2.N)

class DNF:
    def __init__(self, clauses):
        self.clauses = clauses

    @staticmethod
    def make(clauses: Iterable[Clause]) -> "DNF":
        return DNF(frozenset(clauses)).simplify()

    def vars(self):
        v = set()
        for c in self.clauses:
            v |= c.P
            v |= c.N
        return v

    def eval(self, env: dict[str, set]) -> set:
        result = self.clauses[0].eval(env)
        for c in self.clauses:
            result |= c.eval(env)
        return result

    def show(self) -> str:
        if not self.clauses:
            return "∅"
        return " ∪ ".join(
            sorted((c.show() for c in self.clauses))
        )

    def simplify(self) -> "DNF":
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
                    if _clause_subset(c, d):  # c ⊆ d, so c is redundant in a union
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

        return DNF(frozenset(clauses))

    def groups(self):
        dp = DefaultDict()
        dn = DefaultDict()

        for v in self.vars():
            pos = {c.P.difference(v) for c in self.clauses if v in c.P}
            dp[v] = {s for s in pos if s}
            neg = {c.P.difference(v) for c in self.clauses if v in c.N}
            dn[v] = {s for s in neg if s}
        return dp, dn

    def singletons(self):
        return {v for v in self.vars() if any(len(c.P) == 1 and v in c.P for c in self.clauses)}