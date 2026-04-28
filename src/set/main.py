from contextlib import redirect_stdout
from io import StringIO

from src.expr import Var
from src.normalize import normalize
from src.set.set_generation import graph_generation
from src.set.synth import Source, Sink

if __name__ == '__main__':
    # make expression
    a, b, c, d = map(Var, "abcd")
    expr = ((a|b)&c) - d
    # expr = (((b - c) | (d & c)) & (b - (b & d)))
    # expr = (a | b) & (c | (a & b))

    print("original expression: ", expr)

    # convert to normal form
    formula = normalize(expr)
    print("normalized expression: ", formula.show())

    # make state machine
    g = graph_generation(formula)
    print("state machine:")
    print()
    g.dot(expr.show())
    print()



    # get a result

    env = {"a": {'1', 'B', 'C'},
           "b": {'2', '3', 'A'},
           "c": {'3', '4', 'A', 'B'},
           "d": {'1', 'A'}}

    a, b, c, d = map(lambda kv: Source(kv[0], kv[1]), env.items())
    r = Sink()
    # wanted: 3, B

    s = StringIO()
    with redirect_stdout(s):
        g.py()
    try:
        exec(s.getvalue())
    except IndexError:
        print("stopped by exhaustion")

    print("wanted", expr.eval(env))
    print("result", r.data)
