from contextlib import redirect_stdout
from io import StringIO

from src.expr import Var
from src.normalize import normalize
from src.trie.trie_generation import TrieExecution
from src.trie.trie_synth import Source, Sink
from src.trie.trie import bittrieset

if __name__ == '__main__':
    # make expression
    a, b, c, d = map(Var, "abcd")
    # expr = ((a|b)&c) - d
    expr = ((a&b)&c)
    # expr = (((b - c) | (d & c)) & (b - (b & d)))
    # expr = (a | b) & (c | (a & b))

    print("original expression: ", expr)

    # convert to normal form
    formula = normalize(expr)
    print("normalized expression: ", formula.show())

    # make state machine
    g = TrieExecution.create_state_machine(formula)
    print("state machine:")
    print()
    g.dot(expr.show())
    print()



    # get a result

    x, y, z, w, _1, _2, _3 = ('000', '001', '010', '011', '100', '101', '110')

    env = {
        "a": bittrieset(y, _1, _2, _3),
        "b": bittrieset(y, z, _1, _2),
        "c": bittrieset(z, w, _1, _2),
        "d": bittrieset(x, _1),
    }

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
