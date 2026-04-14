from __future__ import annotations

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


@dataclass(frozen=True)
class Var(Expr):
    name: str

    def __repr__(self):
        return self.name


@dataclass(frozen=True)
class And(Expr):
    left: Expr
    right: Expr

    def __repr__(self):
        return f"({self.left} & {self.right})"


@dataclass(frozen=True)
class Or(Expr):
    left: Expr
    right: Expr

    def __repr__(self):
        return f"({self.left} | {self.right})"


@dataclass(frozen=True)
class Diff(Expr):
    left: Expr
    right: Expr

    def __repr__(self):
        return f"({self.left} - {self.right})"
