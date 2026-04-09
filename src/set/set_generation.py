from collections import defaultdict
from dataclasses import dataclass

from src.set.synth import Graph, Source, Sink, OpOrNot


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
        result = set(env[next(it)])
        for p in self.P:
            result &= env[p]
        for n in self.N:
            result -= env[n]
        return result



    def make_graph(self, g: Graph, s_main, dependency=None, singletons=(), singleton_nodes=None):
        # FIXME bugged!!!!
        if dependency is None:
            dependency = defaultdict(str)
        if dependency is None:
            dependency = defaultdict(str)
        # (a /\ b) /\ c
        # g = Graph()
        ps = g.sources(*self.P)
        ns = g.sources(*self.N)
        r, = g.sinks('r')

        p_names = [p.name for p in ps]

        # s0, s1, s2 = g.states('s0', 's1', 's2')
        # g.init = s0

        # s0.to(s1, pull=(*ps, *ns))  # s0
        for p in ps:
            for q in ps:
                if p.name not in [s.name for s in singletons]:
                    if p.name != q.name:
                        s_main.to(s_main, p < q, *(OpOrNot('>', d, p) for d in dependency[p.name]), pull=(p, ), active=ps)

        for p in ps:
            s_main.to(s_main, *(ps[0] == q for q in ps), *(OpOrNot('>', d, p) for d in dependency[p.name] if d.name not in p_names), *(OpOrNot('>', n, p) for n in ns), push=((r, p),), pull=(p,), active=ps)

        for p in ps:
            for n in ns:
                # if p.name in singletons:
                #     s_main.to(singleton_nodes[p.name], *(ps[0] == q for q in ps), p == n, *(OpOrNot('>', d, p) for d in dependency[p.name] if d.name not in p_names), active=(*ps, n))
                # else:
                s_main.to(s_main, *(ps[0] == q for q in ps), p == n, *(OpOrNot('>', d, p) for d in dependency[p.name] if d.name not in p_names), pull=(p, ), active=(*ps, n))

        for n in ns:
            s_main.to(s_main, *(ps[0] == q for q in ps), *(ps[0] != n2 for n2 in ns), ps[0] > n, *(OpOrNot('>', d, n) for d in dependency[n.name]), pull=(n,), active=(*ps, n))


        """
        for (p, n) in zip(s_news, product(ps, ns)):
            s_main.to(s_main, *(ps[0] == q for q in ps), *(p <= d for d in dependency[p.name]), active=ps)

            for n in ns:
                s_new_.to(s_main, ps[0] == n, pull=(p, ), active=(n,))
                s_new_.to(s_new_, ps[0] > n, pull=(n,), active=(n,))  # TODO dependency???

            s_new_.to(s_main, *(ps[0] < n for n in ns), push = ((r, ps[0]), ), pull = (p, ), active=ns)
            # TODO Should I not add a statement for if some are smaller, and some are finished?
            s_new_.to(s_main, push = ((r, ps[0]), ), pull = (p, ), finished=ns)
        """

        return g


class Formula:
    def __init__(self, clauses):
        self.clauses = clauses

    def naive(self, env):
        srcs = {k: Source(k, v) for (k, v) in env.items()}
        r = Sink()
        values = {k: s.pull() for (k, s) in srcs.items()}

        for i in range(100):
            matched = False

            for c in self.clauses:
                ps = list(c.P)
                if all(values[p] is not None for p in ps):
                    if (
                            all(values[ps[0]] == values[p] for p in ps)
                            and all(values[n] > values[ps[0]] for n in c.N if values[n] is not None)
                    ):
                        r.push(values[ps[0]])
                        p = ps[0]
                        values[p] = srcs[p].pull()
                        matched = True
                        break

            if matched:
                continue

            if all(v is None for v in values.values()):
                break

            m = min(
                (k for k, v in values.items() if v is not None),
                key=values.get
            )
            values[m] = srcs[m].pull()

        return r

    def show(self) -> str:
        if not self.clauses:
            return "∅"
        return " ∪ ".join(
            sorted((c.show() for c in self.clauses))
        )

if __name__ == '__main__':
    c1 = Clause.make({"a", "b"}, {"c"})
    c2 = Clause.make({"b"}, {"d"})

    f = Formula((c1, c2))
    #
    #
    # g = Graph()
    # s0, s1, s2 = g.states('s0', 's1', 's2')
    # g.init = s0
    #
    # # s2 = g.states('s2')
    #
    #
    #
    #
    # a, b, c, d = g.sources("a", "b", "c", "d")
    # s0.to(s1, pull=(a, b, c, d))  # s0
    #
    # dependency_names = {
    #     "a": ("b",),
    #     "b": ("a", "d"),
    #     "c": ("a", "b"),
    #     "d": ("b", )
    # }
    #
    # dependencies = defaultdict(tuple[str], {"a": (b,),
    #     "b": (a, d),
    #     "c": (a, b),
    #     "d": (b, )})
    # singletons = (b, )
    # # g = c1.make_graph(g, s1, dependencies, singletons)
    # # g = c2.make_graph(g, s1, dependencies, singletons)
    #
    # g = f.make_graph2(g, s1, s2, dependencies)
    # g.py()
    #
    # # for t in c.make_graph().transitions:
    # #     print((t.s_from.name, t.s_to.name, [(w.lhs.name, w.kind, w.rhs.name) for w in t.when], [p.name for r, p in t.push], [p.name for p in t.pull], [u.name for u in t.active]))
    env = {
        "a": {'6', '8', 'C'},
        "b": {'2', '3', '4', '5', '6', '7', '9', 'B', 'C', 'D', 'F'},
        "c": {'2', '3', '4', '5', '6', '7', '8', 'A', 'B', 'C', 'D', 'E', 'F'},
        "d": {'8', 'E'},
    }
    #
    # a = Source('a', env["a"])
    # b = Source('b', env["b"])
    # c = Source('c', env["c"])
    # d = Source('d', env["d"])
    # # e = Source('e', e_set)
    # # f = Source('f', {'3'})
    # r = Sink()
    # # wanted: 3, B
    #
    #
    # s = StringIO()
    # with redirect_stdout(s):
    #     g.py()
    # try:
    #     exec(s.getvalue())
    # except IndexError:
    #     print("stopped by exhaustion")

    # print("result", set(r.data))
    print(f.naive(env).data)
    print("wanted", c1.eval(env) | c2.eval(env))
