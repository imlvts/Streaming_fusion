from dataclasses import dataclass
from typing import DefaultDict

from test.fuzzer import Clause


@dataclass(frozen=True)
class Clause:
    P: frozenset[str]
    N: frozenset[str]

    @staticmethod
    def make(p, n=()):
        return Clause(frozenset(p), frozenset(n))

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
        for p in self.P:
            result &= env[p]
        for n in self.N:
            result -= env[n]
        return result

class Formula:
    def __init__(self, clauses):
        self.clauses: list[Clause] = clauses

    def eval(self, env: dict[str, set]) -> set:
        result = self.clauses[0].eval(env)
        for c in self.clauses:
            result |= c.eval(env)
        return result

    def vars(self):
        v = set()
        for c in self.clauses:
            v |= c.P
            v |= c.N
        return v

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

    def show(self) -> str:
        if not self.clauses:
            return "∅"
        return " ∪ ".join(
            sorted((c.show() for c in self.clauses))
        )


if __name__ == '__main__':
    c1 = Clause.make({"a", "b", "e"}, {"c", "d"})
    c2 = Clause.make({"c", "b"}, {"a"})

    f = Formula([c1, c2])

    dp, dn = f.groups()
    print(dp)
    print(dn)