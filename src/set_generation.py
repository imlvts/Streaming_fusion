from contextlib import redirect_stdout
from dataclasses import dataclass
from io import StringIO

from src.synth import Graph, Source, Sink


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

    def make_graph(self, g: Graph, s1, s2, dependency=None):
        # (a /\ b) /\ c
        # g = Graph()
        ps = g.sources(*self.P)
        ns = g.sources(*self.N)
        r, = g.sinks('r')

        # s0, s1, s2 = g.states('s0', 's1', 's2')
        # g.init = s0

        # s0.to(s1, pull=(*ps, *ns))  # s0
        for p in ps:
            for q in ps:
                s1.to(s1, p < q, pull=(p, ), unfinished=ps)

        s1.to(s2, *(ps[0] == q for q in ps), unfinished=ps)

        for n in ns:
            s2.to(s1, ps[0] == n, pull=ps, unfinished=(n,))
            s2.to(s2, ps[0] > n, pull=(n,), unfinished=(n,))

        s2.to(s1, *(ps[0] < n for n in ns), push = ((r, ps[0]), ), pull = ps, unfinished=ns)
        # TODO Should I not add a statement for if some are smaller, and some are finished?
        s2.to(s1, push = ((r, ps[0]), ), pull = ps, finished=ns)


        return g


class Formula:
    def __init__(self, clauses):
        self.clauses = frozenset(
            Clause.make(P, N) for (P, N) in clauses
        )

    def show(self) -> str:
        if not self.clauses:
            return "∅"
        return " ∪ ".join(
            sorted((c.show() for c in self.clauses))
        )

if __name__ == '__main__':
    f = Formula([
        ({"A", "B"}, {"C"}),
        ({"E"}, {"F"})])


    c1 = Clause.make({"a", "b"}, {"e"})
    c2 = Clause.make({"c", "d"}, {"e"})

    # dependencies = {"e": ("a", "b", "c", "d")}

    g = Graph()
    s0, s1, s2, s3 = g.states('s0', 's1', 's2', 's3')
    g.init = s0

    ps = g.sources("a", "b", "d")
    ns = g.sources("c", "e")
    s0.to(s1, pull=(*ps, *ns))  # s0

    g = c1.make_graph(g, s1, s2)
    g = c2.make_graph(g, s1, s3)
    g.py()

    # for t in c.make_graph().transitions:
    #     print((t.s_from.name, t.s_to.name, [(w.lhs.name, w.kind, w.rhs.name) for w in t.when], [p.name for r, p in t.push], [p.name for p in t.pull], [u.name for u in t.unfinished]))
    a_set = {'1', '3', 'A', 'B', 'C'}
    b_set = {'2', '3', 'A', 'B'}
    c_set = {'1', '3', '4', 'A', 'B'}
    d_set = {'1', '3', 'A', 'B'}
    e_set = {'1', 'A'}

    a = Source('a', {'1', '3', 'A', 'B', 'C'})
    b = Source('b', {'2', '3', 'A', 'B'})
    c = Source('c', {'1', '3', '4', 'A', 'B'})
    d = Source('d', {'1', '3', 'A', 'B'})
    e = Source('e', {'1', 'A'})
    f = Source('f', {'3'})
    r = Sink()
    # wanted: 3, B

    s = StringIO()
    with redirect_stdout(s):
        g.py()
    try:
        exec(s.getvalue())
    except IndexError:
        print("stopped by exhaustion")

    print("result", set(r.data))
    print("wanted", a_set.intersection(b_set).difference(c_set).union(d_set.difference(e_set)))