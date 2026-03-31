import unittest
from collections import defaultdict
from contextlib import redirect_stdout
from io import StringIO

from src.set_generation import Formula, Clause
from src.synth import Sink, Source, Graph



class TestGraphGeneration(unittest.TestCase):
    def test_clause(self):
        c = Clause.make({"a", "b", "c", "d"}, {"e", "f"})

        g = Graph()
        s0, s1, s2, s3, s4 = g.states('s0', 's1', 's2', 's3', 's4')
        g.init = s0

        ps = g.sources("a", "b", "c", "d")
        ns = g.sources("f", "e")
        s0.to(s1, pull=(*ps, *ns))  # s0

        g = c.make_graph(g, s1, (s2, s3))

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
        s0, s1, s2, s3, s4 = g.states('s0', 's1', 's2', 's3', 's4')
        g.init = s0

        ps = g.sources("a", "b", "d")
        ns = g.sources("c", "e")
        s0.to(s1, pull=(*ps, *ns))  # s0

        dependency = defaultdict(str)

        g = c1.make_graph(g, s1, (s2, s3), dependency)
        g = c2.make_graph(g, s1, (s4, ), dependency)
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


    def test_overlapping_intersections(self):
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

        s = StringIO()
        with redirect_stdout(s):
            g.py()
        try:
            exec(s.getvalue())
        except IndexError:
            print("stopped by exhaustion")

        wanted = a_set.intersection(c_set).difference(d_set).union(b_set.intersection(c_set).difference(e_set))
        self.assertSetEqual(wanted, set(r.data))


    def test_first_formula(self):
        c1 = Clause.make({"a", "c"}, {"d"})
        c2 = Clause.make({"b", "c"}, {"d"})

        g = Graph()
        s0, s1, s2, s3, s4, s5 = g.states('s0', 's1', 's2', 's3', 's4', 's5')
        g.init = s0

        a, b, c, d = g.sources("a", "b", "c", "d")
        s0.to(s1, pull=(a, b, c, d))  # s0

        dependencies = defaultdict(tuple[str], {"c": (a, b)})
        g = c1.make_graph(g, s1, [s2, s3], dependencies)
        g = c2.make_graph(g, s1, [s4, s5], dependencies)
        g.py()

        # for t in c.make_graph().transitions:
        #     print((t.s_from.name, t.s_to.name, [(w.lhs.name, w.kind, w.rhs.name) for w in t.when], [p.name for r, p in t.push], [p.name for p in t.pull], [u.name for u in t.unfinished]))
        a_set = {'1', 'B', 'C'}
        b_set = {'2', '3', 'A'}
        c_set = {'3', '4', 'A', 'B'}
        d_set = {'1', 'A'}


        a = Source('a', {'1', 'B', 'C'})
        b = Source('b', {'2', '3', 'A'})
        c = Source('c', {'3', '4', 'A', 'B'})
        d = Source('d', {'1', 'A'})
        r = Sink()

        s = StringIO()
        with redirect_stdout(s):
            g.py()
        try:
            exec(s.getvalue())
        except IndexError:
            print("stopped by exhaustion")

        wanted = a_set.intersection(c_set).difference(d_set).union(b_set.intersection(c_set).difference(d_set))
        self.assertSetEqual(wanted, set(r.data))
