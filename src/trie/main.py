from contextlib import redirect_stdout
from io import StringIO
from os.path import dirname, join

from src.expr import Var
from src.normalize import normalize
from src.trie.trie_generation import TrieExecution
from src.trie.trie_synth import Source, Sink
from src.trie.trie import bittrieset

if __name__ == '__main__':
    # make expression
    a, b, c, d = map(Var, "abcd")
    expr = ((a|b)&c) - d
    # expr = ((a&b)&c)
    # expr = (((b - c) | (d & c)) & (b - (b & d)))
    # expr = (a | b) & (c | (a & b))

    print("original expression: ", expr)

    # convert to normal form
    formula = normalize(expr)
    print("normalized expression: ", formula.show())

    # make state machine
    g = TrieExecution.create_state_machine(formula)

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

    # python, rust
    LANG = 'rust'
    if LANG == 'python':
        print("state machine:")
        print()
        # g.dot(expr.show())
        g.py()
        print()
        # get a result

        s = StringIO()
        with redirect_stdout(s):
            g.py()
        try:
            exec(s.getvalue())
        except IndexError:
            print("stopped by exhaustion")

        print("wanted", expr.eval(env))
        print("result", r.data)

    if LANG == 'rust':
        THIS_DIR = dirname(__file__)
        RS_DIR = join(THIS_DIR, '../../test_rs')

        s = StringIO()
        with redirect_stdout(s):
            g.rs()
        init = ""
        for name, value in env.items():
            ty = "Option<u32>"
            init += f'\tlet __src_{name}: PathMap<{ty}> = {value.rs()};\n'
            init += f'\tlet {name} = RefCell::new(__src_{name}.read_zipper());\n'
        init += f'\tlet mut r = Vec::new();\n'
        code = s.getvalue()
        code = f"""
use std::cell::RefCell;
mod shim;
#[allow(unused_imports)]
use shim::{{
    PathMap, argmin, argmax, descend_or_next,
    prefix_of, difference_level, next, path, is_val,
    Zipper, ZipperMoving, ReadZipperUntracked,
}};
#[allow(unused_parens)]
fn test() {{
{init}
{code}
// eprintln!("wanted: {{:?}}", {expr.eval(env)});
eprintln!("result: {{:?}}", r);
}}
fn main() {{ test(); }}
        """
        print(code)
        with open(join(RS_DIR, 'src/main.rs'), 'w') as f:
            f.write(code)

        cargo_cmd = [
            "cargo", "run",
            "--release",
            "--manifest-path", join(RS_DIR, 'Cargo.toml'),
        ]
        import subprocess
        result = subprocess.run(cargo_cmd, capture_output=True, text=True)
        print(result.stdout)
        print(result.stderr)