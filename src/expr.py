from __future__ import annotations

from abc import abstractmethod
from dataclasses import dataclass

@dataclass(frozen=True)
class Expr:
    def __and__(self, other):
        return And(self, other)

    def __or__(self, other):
        return Or(self, other)

    def __sub__(self, other):
        return Diff(self, other)

    def eval(self, env):
        if isinstance(self, Var):
            return set(env[self.name])
        if isinstance(self, And):
            return self.left.eval(env) & self.right.eval(env)
        if isinstance(self, Or):
            return self.left.eval(env) | self.right.eval(env)
        if isinstance(self, Diff):
            return self.left.eval(env) - self.right.eval(env)
        raise TypeError(f"Unknown expr: {self!r}")

    @abstractmethod
    def show(self):
        pass


@dataclass(frozen=True)
class Var(Expr):
    name: str

    def show(self):
        return self.name


@dataclass(frozen=True)
class And(Expr):
    left: Expr
    right: Expr

    def show(self):
        return f"({self.left.show()} ∩ {self.right.show()})"


@dataclass(frozen=True)
class Or(Expr):
    left: Expr
    right: Expr

    def show(self):
        return f"({self.left.show()} ∪ {self.right.show()})"


@dataclass(frozen=True)
class Diff(Expr):
    left: Expr
    right: Expr

    def show(self):
        return f"({self.left.show()} \\ {self.right.show()})"


if __name__ == '__main__':
    a, b, c = map(Var, "abc")
    expr = (a|b) - c
    print(expr.show())