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
        print("for i in range(500):")
        print('\tprint("state", state)')
        # print('\tprint("temps", tmp_a, tmp_b, tmp_c, tmp_d)')
        for state in self.vtcs:
            print(f"\tif state == '{state.name}':")
            for t in self.transitions:
                if t.s_from != state: continue
                if t.active: print(f"\t\tif {' and '.join(f'tmp_{c.name}' for c in t.active)}:")
                elif t.finished: print(f"\t\tif {' and '.join(f'not tmp_{c.name}' for c in t.finished)}:")
                else: print(f"\t\tif True:")
                if t.when:
                    temp = []
                    for c in t.when:
                        if isinstance(c, Inequality): temp.append(f'tmp_{c.lhs.name} {c.kind} tmp_{c.rhs.name}')
                        if isinstance(c, OpOrNot): temp.append(f'(tmp_{c.lhs.name} is None or tmp_{c.lhs.name} {c.kind} tmp_{c.rhs.name})')
                    print(f"\t\t\tif {' and '.join(temp)}:")
                else: print(f"\t\t\tif True:")
                for dst, src in t.push: print(f"\t\t\t\t{dst.name}.push(tmp_{src.name})")
                # print(p for p in t.pull)
                # for src in t.pull: print(f"yeeeeeeeeeehya")
                for src in t.pull: print(f"\t\t\t\ttmp_{src.name} = {src.name}.pull()")
                print(f"\t\t\t\tstate = '{t.s_to.name}'")
                print(f"\t\t\t\tcontinue")
            print("\t\tprint(state, 'not continued!')")
            print("\t\tbreak")
    def dot(self, title=None):
        if title:
            title = title.replace("\\", "\\\\")
            print(f"title [shape=\"rect\", label=\"{title}\"]")
        label_index = 0
        for state in self.vtcs:
            print(state.name, ";")
        for t in self.transitions:
            cond = []
            todo = []
            for src in t.active: cond.append(f"active {src.name}")
            for src in t.finished: cond.append(f"finished {src.name}")
            if t.when:
                for c in t.when:
                    if isinstance(c, Inequality): cond.append(f'{c.lhs.name} {c.kind} {c.rhs.name}')
                    if isinstance(c, OpOrNot): cond.append(f'({c.lhs.name} is None or {c.lhs.name} {c.kind} {c.rhs.name})')

            for dst, src in t.push: todo.append(f"{dst.name}!{src.name}")
            for src in t.pull: todo.append(f"{src.name}.pull()")
            if len(cond) == 0:
                cond = ["else"]
            temp = [f"{' \\n '.join(cond)}", f"{' \\n '.join(todo)}"]
            label = f"{{{' | '.join(temp)}}}"
            label = label.replace('>', '\\>')
            label = label.replace('<', '\\<')
            print(f"l{label_index} [shape=\"record\", label=\"{label}\"];")
            print(f"{t.s_from.name} -> l{label_index} [arrowhead=\"none\"];")
            print(f"l{label_index} -> {t.s_to.name};")
            label_index += 1

class Node:
    def __init__(self, graph, name): self.graph = graph; self.name = name
    @classmethod
    def named(cls, g, *args) -> list[Self]: return [cls(g, a) for a in args]
class Src(Node):
    def __lt__(self, other: 'Src'): return Inequality('<', self, other)
    def __gt__(self, other: 'Src'): return Inequality('>', self, other)
    def __eq__(self, other: 'Src'): return Inequality('==', self, other)
    def __ne__(self, other: 'Src'): return Inequality('!=', self, other)
    def __ge__(self, other: 'Src'): return Inequality('>=', self, other)
    def __le__(self, other: 'Src'): return Inequality('<=', self, other)
class Snk(Node): pass
class Cond: pass
class Inequality(Cond):
    def __init__(self, kind: Literal["=="] | Literal["<"] | Literal[">"] | Literal[">="] | Literal["<="] | Literal["!="], lhs: Src, rhs: Src):
        self.kind = kind; self.lhs = lhs; self.rhs = rhs
class OpOrNot(Cond):
    def __init__(self, kind: Literal["=="] | Literal["<"] | Literal[">"] | Literal[">="] | Literal["<="] | Literal["!="], lhs: Src, rhs: Src):
        self.kind = kind; self.lhs = lhs; self.rhs = rhs
class Transition:
    def __init__(self, s_from, s_to, when, push, pull, active, finished):
        self.s_from = s_from; self.s_to = s_to; self.when = when; self.push = push; self.pull = pull; self.active = active; self.finished = finished;
class Vtx(Node):
    def to(self, other, *when, push=(), pull=(), active=(), finished=()):
        self.graph.transitions.append(Transition(self, other, when, push, pull, active, finished))

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

    s1.to(s1, a < b, pull=(a, ), active=(a, b, c))
    s1.to(s1, a < c, pull=(a, ), active=(a, b, c))
    s1.to(s1, b < a, pull=(b, ), active=(a, b, c))
    s1.to(s1, b < c, pull=(b, ), active=(a, b, c))
    s1.to(s1, c < a, pull=(c, ), active=(a, b, c))
    s1.to(s1, c < b, pull=(c, ), active=(a, b, c))
    s1.to(s1, a == b, b ==c, push=((r, a), ), pull=(a, b, c), active=(a, b, c))

    return g

def ctx2():

    # (a/\c \ d)\/(b/\c \ e)

    g = Graph()
    a, b, c, d = g.sources('a', 'b', 'c', 'd')
    r, = g.sinks('r')
    s0, s1, s2, s3 = g.states('s0', 's1', 's2', 's3')
    g.init = s0

    s0.to(s1, pull=(a, b, c, d)) # s0

    s1.to(s1, a < c, pull=(a, ), active=(a, c))
    s1.to(s1, b < c, pull=(b, ), active=(b, c))
    s1.to(s2, a == c, active=(a, c))
    s1.to(s3, b == c, active=(b, c))
    s1.to(s1, OpOrNot(">", a, c), OpOrNot(">", b, c), pull=(c,), active=(c, ))  # s10

    s2.to(s2, a > d, pull=(d,), active=(a, d)) # s14
    s2.to(s1, a == d, pull=(a, ), active=(a, d))
    s2.to(s1, OpOrNot(">", d, a), push=((r, a),), pull=(a, )) # s12

    s3.to(s3, b > d, pull=(d,), active=(b, d)) # s14
    s3.to(s1, b == d, pull=(b, ), active=(b, d))
    s3.to(s1, OpOrNot(">", d, b), push=((r, b),), pull=(b, )) # s12

    return g


class Source:
    def __init__(self, n, s):
        self.name = n
        self.data = [*sorted(s)]
        self.index = 0
    def pull(self):
        if self.index >= len(self.data):
            print(f"pulled None from {self.name}")
            return None
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
    g = ctx2()
    # g = intersection_graph()
    g.py()

    a = Source('a', {'1', 'B', 'C'})
    # a = Source('a', {'1', 'B', 'C', 'A', '3'})
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
    g.dot()