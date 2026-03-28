# ** Code by Adam Vandervorst, graph by me **

from contextlib import redirect_stdout
from io import StringIO
from typing import Self, Literal

class Graph:
    def __init__(self): self.srcs = []; self.snks = []; self.vtcs = []; self.transitions = []; self.init = None
    def sources(self, *names): new = Src.named(self, *names); self.srcs += new; return new
    def sinks(self, *names): new = Snk.named(self, *names); self.snks += new; return new
    def states(self, *names): new = Vtx.named(self, *names); self.vtcs += new; return new
    def py(self):
        for src in self.srcs:
            print(f"tmp_{src.name} = None")
        print(f"state = '{self.init.name}'")
        print("for i in range(100):")
        print('\tprint("state", state)')
        for state in self.vtcs:
            print(f"\tif state == '{state.name}':")
            for t in self.transitions:
                if t.s_from != state: continue
                if t.unfinished: print(f"\t\tif {' and '.join(f'tmp_{c.name}' for c in t.unfinished)}:")
                else: print(f"\t\tif True:")
                if t.when: print(f"\t\t\tif {' and '.join(f'tmp_{c.lhs.name} {c.kind} tmp_{c.rhs.name}' for c in t.when)}:")
                else: print(f"\t\t\tif True:")
                for dst, src in t.push: print(f"\t\t\t\t{dst.name}.push(tmp_{src.name})")
                for src in t.pull: print(f"\t\t\t\ttmp_{src.name} = {src.name}.pull()")
                print(f"\t\t\t\tstate = '{t.s_to.name}'")
                print(f"\t\t\t\tcontinue")
            print("\t\t\tprint(state, 'not continued!')")
            print("\t\t\tbreak")
    def dot(self):
        ...
class Node:
    def __init__(self, graph, name): self.graph = graph; self.name = name
    @classmethod
    def named(cls, g, *args) -> list[Self]: return [cls(g, a) for a in args]
class Src(Node):
    def __lt__(self, other: 'Src'): return Cond('<', self, other)
    def __gt__(self, other: 'Src'): return Cond('>', self, other)
    def __eq__(self, other: 'Src'): return Cond('==', self, other)
class Snk(Node): pass
class Cond:
    def __init__(self, kind: Literal["=="] | Literal["<"] | Literal[">"], lhs: Src, rhs: Src):
        self.kind = kind; self.lhs = lhs; self.rhs = rhs
class Transition:
    def __init__(self, s_from, s_to, when, push, pull, unfinished):
        self.s_from = s_from; self.s_to = s_to; self.when = when; self.push = push; self.pull = pull; self.unfinished = unfinished
class Vtx(Node):
    def to(self, other, *when, push=(), pull=(), unfinished=()):
        self.graph.transitions.append(Transition(self, other, when, push, pull, unfinished))

def ctx():

    # (a\/b)/\c \ d

    g = Graph()
    a, b, c, d = g.sources('a', 'b', 'c', 'd')
    r, = g.sinks('r')
    s0, s1, s11, s15, s19 = g.states('s0', 's1', 's11', 's15', 's19')
    g.init = s0

    s0.to(s1, pull=(a, b, c, d)) # s0

    s1.to(s1, a < c, b < c, pull=(a, b)) # s2
    s1.to(s1, a < c, b == c, pull=(a,)) # s3
    s1.to(s1, a < c, b > c, pull=(a,)) # s4
    s1.to(s1, a == c, b < c, pull=(b,)) # s5
    s1.to(s1, a > c, b < c, pull=(b,)) # s8
    s1.to(s1, a > c, b > c, pull=(c,)) # s10
    s1.to(s11, a == c, b == c, pull=()) # s6
    s1.to(s15, a == c, b > c, pull=()) # s7
    s1.to(s19, a > c, b == c, pull=()) # s9

    s11.to(s11, a > d, b > d, pull=(d,)) # s14
    s11.to(s1, a == d, b == d, pull=(a, b, c)) # s13
    s11.to(s1, a < d, b < d, push=((r, a),), pull=(a, b)) # s12

    s15.to(s15, a > d, pull=(d,)) # s18
    s15.to(s1, a == d, pull=(a,)) # s17
    s15.to(s1, a < d, push=((r, a),), pull=(a,)) # s16

    s19.to(s19, b > d, pull=(d,)) # s22
    s19.to(s1, b == d, pull=(b,)) # s21
    s19.to(s1, b < d, push=((r, b),), pull=(b,)) # s20

    return g

def intersection_graph():

    # (a /\ b) /\ c

    g = Graph()
    a, b, c = g.sources('a', 'b', 'c')
    r, = g.sinks('r')
    s0, s1 = g.states('s0', 's1')
    g.init = s0

    s0.to(s1, pull=(a, b, c)) # s0

    s1.to(s1, a < b, pull=(a, ), unfinished=(a, b, c))
    s1.to(s1, a < c, pull=(a, ), unfinished=(a, b, c))
    s1.to(s1, b < a, pull=(b, ), unfinished=(a, b, c))
    s1.to(s1, b < c, pull=(b, ), unfinished=(a, b, c))
    s1.to(s1, c < a, pull=(c, ), unfinished=(a, b, c))
    s1.to(s1, c < b, pull=(c, ), unfinished=(a, b, c))
    s1.to(s1, a == b, b ==c, push=((r, a), ), pull=(a, b, c), unfinished=(a, b, c))

    return g

class Source:
    def __init__(self, n, s):
        self.name = n
        self.data = [*sorted(s)]
        self.index = 0
    def pull(self):
        ret = self.data[self.index]
        print(f"pulled {ret} from {self.name}")
        self.index += 1
        return ret
class Sink:
    def __init__(self):
        self.data = []
    def push(self, inp):
        print("pushed", inp)
        self.data.append(inp)

if __name__ == '__main__':
    # g = ctx()
    g = intersection_graph()
    g.py()

    # a = Source('a', {'1', 'B', 'C'})
    a = Source('a', {'1', 'B', 'C', 'A', '3'})
    b = Source('b', {'2', '3', 'A'})
    c = Source('c', {'3', '4', 'A', 'B'})
    d = Source('d', {'1', 'A'})
    r = Sink()
    #wanted: 3, B

    s = StringIO()
    with redirect_stdout(s): g.py()
    try: exec(s.getvalue())
    except IndexError: print("stopped by exhaustion")

    print("result", r.data)