import unittest
from contextlib import redirect_stdout
from io import StringIO

from src.set_generation import Formula, Clause
from src.synth import Sink, Source, Graph


class TestGraphGeneration(unittest.TestCase):
    def test_clause(self):
        c = Clause.make({"a", "b", "c", "d"}, {"e", "f"})

        g = c.make_graph()

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

        self.assertEqual(['B'], r.data)

    def test_two_results_non_overlapping(self):
        c1 = Clause.make({"a", "b"}, {"c"})
        c2 = Clause.make({"d"}, {"e"})

        g = Graph()
        s0, s1, s2, s3 = g.states('s0', 's1', 's2', 's3')
        g.init = s0

        ps = g.sources("a", "b", "d")
        ns = g.sources("c", "e")
        s0.to(s1, pull=(*ps, *ns))  # s0

        g = c1.make_graph(g, s1, s2)
        g = c2.make_graph(g, s1, s3)
        # g.py()

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

        self.assertSetEqual(a_set.intersection(b_set).difference(c_set).union(d_set.difference(e_set)), set(r.data))
