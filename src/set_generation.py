from collections import defaultdict
from contextlib import redirect_stdout
from dataclasses import dataclass
from io import StringIO

from src.synth import Graph, Source, Sink, OpOrNot


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

    def make_graph(self, g: Graph, s_main, s_news, dependency=None):
        if dependency is None:
            dependency = defaultdict(str)
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
                s_main.to(s_main, p < q, *(OpOrNot('>', d, p) for d in dependency[p.name]), pull=(p, ), unfinished=ps)

        for s_new_, p in zip(s_news, ps):
            s_main.to(s_new_, *(ps[0] == q for q in ps), *(p <= d for d in dependency[p.name]), unfinished=ps)

            for n in ns:
                s_new_.to(s_main, ps[0] == n, pull=(p, ), unfinished=(n,))
                s_new_.to(s_new_, ps[0] > n, pull=(n,), unfinished=(n,))  # TODO dependency???

            s_new_.to(s_main, *(ps[0] < n for n in ns), push = ((r, ps[0]), ), pull = (p, ), unfinished=ns)
            # TODO Should I not add a statement for if some are smaller, and some are finished?
            s_new_.to(s_main, push = ((r, ps[0]), ), pull = (p, ), finished=ns)


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


    c1 = Clause.make({"a", "c"}, {"d"})
    c2 = Clause.make({"b", "c"}, {"e"})


    g = Graph()
    s0, s1, s2, s3, s4, s5 = g.states('s0', 's1', 's2', 's3', 's4', 's5')
    g.init = s0

    a, b, c, d, e = g.sources("a", "b", "c", "d", "e")
    s0.to(s1, pull=(a, b, c, d, e))  # s0

    dependencies = defaultdict(tuple[str], {"c": (a, b)})
    g = c1.make_graph(g, s1, [s2, s3], dependencies)
    g = c2.make_graph(g, s1, [s4, s5], dependencies)
    g.py()

    # for t in c.make_graph().transitions:
    #     print((t.s_from.name, t.s_to.name, [(w.lhs.name, w.kind, w.rhs.name) for w in t.when], [p.name for r, p in t.push], [p.name for p in t.pull], [u.name for u in t.unfinished]))
    a_set = {'1', '3', 'A', 'B', 'C'}
    b_set = {'2', '3', 'A', 'B'}
    c_set = {'1', '3', '4', 'A', 'B'}
    d_set = {'1', '3', 'B'}
    e_set = {'1', 'A'}

    a = Source('a', a_set)
    b = Source('b', b_set)
    c = Source('c', c_set)
    d = Source('d', d_set)
    e = Source('e', e_set)
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
    print("wanted", a_set.intersection(c_set).difference(d_set).union(b_set.intersection(c_set).difference(e_set)))