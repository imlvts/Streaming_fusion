from dataclasses import dataclass


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

    def show(self) -> str:
        if not self.clauses:
            return "∅"
        return " ∪ ".join(
            sorted((c.show() for c in self.clauses))
        )