import unittest
import random
from collections import defaultdict
from contextlib import redirect_stdout
from io import StringIO

from src.set_generation import Formula, Clause
from src.synth import Sink, Source, Graph



class TestGraphGeneration(unittest.TestCase):

    # -------------------------
    # Pretty printing helpers
    # -------------------------

    def format_env(self, env: dict[str, set]) -> str:
        return ", ".join(
            f"{k}={{{', '.join(sorted(v))}}}"
            for k, v in sorted(env.items())
        )

    def repr_env(self, env: dict[str, set]) -> str:
        lines = ["{"]
        for name in sorted(env):
            elems = ", ".join(repr(x) for x in sorted(env[name]))
            lines.append(f'    "{name}": {{{elems}}},')
        lines.append("}")
        return "\n".join(lines)

    # -------------------------
    # Core reusable runner
    # -------------------------

    def run_formula_case(
        self,
        clauses,
        env,
        *,
        dependency_names=None,
        singleton_names=None,
        debug_py=False,
        debug_dot=False,
        seed=None,
        trial=None,
        case_name=None,
    ):
        dependency_names = dependency_names or {}
        singleton_names = singleton_names or ()

        wanted = set()
        for clause in clauses:
            wanted |= clause.eval(env)

        f = Formula(clauses)
        g = Graph()
        s0, s1, s2 = g.states("s0", "s1", "s2")
        g.init = s0

        names = sorted(env.keys())
        pulled = g.sources(*names)
        source_map = dict(zip(names, pulled))

        s0.to(s1, pull=tuple(pulled))

        dependencies = defaultdict(tuple)
        for name, deps in dependency_names.items():
            dependencies[name] = tuple(source_map[d] for d in deps)

        # tuple, not set, because Src is unhashable
        singletons = tuple(source_map[name] for name in singleton_names)

        # for clause in clauses:
        #     g = clause.make_graph(g, s1, dependencies, singletons)
        g = f.make_graph2(g, s1, s2, dependencies)

        if debug_dot:
            g.dot()

        g.py()

        exec_env = {n: Source(n, env[n]) for n in names}
        exec_env["r"] = Sink()

        s = StringIO()
        with redirect_stdout(s):
            g.py()

        generated_code = s.getvalue()

        if debug_py:
            print(generated_code)

        try:
            exec(generated_code, exec_env, exec_env)
        except IndexError:
            print("stopped by exhaustion")

        actual = set(exec_env["r"].data)

        self.assertSetEqual(
            wanted,
            actual,
            msg=(
                f"\ncase={case_name}"
                f"\nseed={seed}"
                f"\ntrial={trial}"
                f"\nclauses={clauses}"
                f"\nenv={self.repr_env(env)}"
                f"\nwanted={sorted(wanted)}"
                f"\nactual={sorted(actual)}"
            ),
        )

    # -------------------------
    # Random env helpers
    # -------------------------

    def random_env(self, variables, universe=None, rng=None):
        rng = rng or random
        universe = universe or tuple("123456789ABCDEF")

        return {
            v: set(rng.sample(universe, rng.randint(0, len(universe))))
            for v in variables
        }

    def run_random_envs(
        self,
        clauses,
        *,
        dependency_names=None,
        singleton_names=None,
        variables=None,
        trials=50,
        seed=0,
        case_name=None,
    ):
        if variables is None:
            variables = sorted(set().union(*(c.P | c.N for c in clauses)))

        rng = random.Random(seed)

        for i in range(trials):
            env = self.random_env(variables, rng=rng)

            with self.subTest(case=case_name, i=i, seed=seed, env=self.format_env(env)):
                self.run_formula_case(
                    clauses,
                    env,
                    dependency_names=dependency_names,
                    singleton_names=singleton_names,
                    seed=seed,
                    trial=i,
                    case_name=case_name,
                )

    # -------------------------
    # Run one case both ways
    # -------------------------

    def run_case_with_original_and_random(self, case):
        name = case["name"]
        clauses = case["clauses"]
        env = case["env"]
        dependency_names = case.get("dependency_names", {})
        singleton_names = case.get("singleton_names", ())
        variables = case.get("variables")
        trials = case.get("trials", 50)
        seed = case.get("seed", 0)

        with self.subTest(case=name, kind="original"):
            self.run_formula_case(
                clauses,
                env,
                dependency_names=dependency_names,
                singleton_names=singleton_names,
                case_name=name,
            )

        self.run_random_envs(
            clauses,
            dependency_names=dependency_names,
            singleton_names=singleton_names,
            variables=variables,
            trials=trials,
            seed=seed,
            case_name=name,
        )

    # -------------------------
    # All cases in one place
    # -------------------------

    CASES = [
        {
            "name": "clause",
            "clauses": [
                Clause.make({"a", "b", "c", "d"}, {"e", "f"}),
            ],
            "env": {
                "a": {"1", "3", "A", "B", "C"},
                "b": {"2", "3", "A", "B"},
                "c": {"1", "3", "4", "A", "B"},
                "d": {"1", "3", "A", "B"},
                "e": {"1", "A"},
                "f": {"3"},
            },
            "dependency_names": {
                "a": ("b", "c", "d", "e", "f"),
                "b": ("a", "c", "d", "e", "f"),
                "c": ("a", "b", "d", "e", "f"),
                "d": ("a", "b", "c", "e", "f"),
                "e": ("a", "b", "c", "d", "f"),
                "f": ("a", "b", "c", "d", "e")
            },
            "variables": ["a", "b", "c", "d", "e", "f"],
            "trials": 100,
            "seed": 10,
        },
        {
            "name": "two_results_non_overlapping",
            "clauses": [
                Clause.make({"a", "b"}, {"c"}),
                Clause.make({"d"}, {"e"}),
            ],
            "env": {
                "a": {"1", "3", "A", "B", "C"},
                "b": {"2", "3", "A", "B"},
                "c": {"1", "3", "4", "A", "B"},
                "d": {"1", "3", "A", "B"},
                "e": {"1", "A"},
            },
            "variables": ["a", "b", "c", "d", "e"],
            "trials": 100,
            "seed": 11,
        },
        {
            "name": "overlapping_intersections",
            "clauses": [
                Clause.make({"a", "c"}, {"d"}),
                Clause.make({"b", "c"}, {"e"}),
            ],
            "env": {
                "a": {"1", "3", "A", "B", "C"},
                "b": {"2", "3", "A", "B"},
                "c": {"1", "3", "4", "A", "B"},
                "d": {"1", "3", "B"},
                "e": {"1", "A"},
            },
            "dependency_names": {
                "c": ("a", "b"),
            },
            "variables": ["a", "b", "c", "d", "e"],
            "trials": 100,
            "seed": 12,
        },
        {
            "name": "first_formula",
            "clauses": [
                Clause.make({"a", "c"}, {"d"}),
                Clause.make({"b", "c"}, {"d"}),
            ],
            "env": {
                "a": {"1", "B", "C"},
                "b": {"2", "3", "A"},
                "c": {"3", "4", "A", "B"},
                "d": {"1", "A"},
            },
            "dependency_names": {
                "c": ("a", "b"),
            },
            "variables": ["a", "b", "c", "d"],
            "trials": 100,
            "seed": 13,
        },
        {
            "name": "random_original",
            "clauses": [
                Clause.make({"a", "b"}, {"c"}),
                Clause.make({"b"}, {"c"}),
            ],
            "env": {
                "a": {"1"},
                "b": {"3", "4", "F"},
                "c": {"2", "C"},
                "d": {"1", "2", "7", "B", "C"},
                "e": {"4"},
            },
            "dependency_names": {
                "a": ("b",),
                "b": ("a",),
                "c": ("a", "b"),
            },
            "singleton_names": ("b", ),
            "variables": ["a", "b", "c", "d", "e"],
            "trials": 100,
            "seed": 14,
        },
        {
            "name": "singleton",
            "clauses": [
                Clause.make({"a", "b"}, {"c"}),
                Clause.make({"b"}, {"d"}),
            ],
            "env": {
                "a": {"1", "4", "5", "6"},
                "b": {"2", "4", "5", "6", "7"},
                "c": {"3", "5"},
                "d": {"5", "6"},
            },
            "dependency_names": {
                "a": ("b",),
                "b": ("a",),
                "c": ("a", "b"),
                "d": ("b",),
            },
            "singleton_names": ("b",),
            "variables": ["a", "b", "c", "d"],
            "trials": 100,
            "seed": 15,
        },
    ]

    # -------------------------
    # One test per formula
    # -------------------------

    def test_clause(self):
        self.run_case_with_original_and_random(self.CASES[0])

    def test_two_results_non_overlapping(self):
        self.run_case_with_original_and_random(self.CASES[1])

    def test_overlapping_intersections(self):
        self.run_case_with_original_and_random(self.CASES[2])

    def test_first_formula(self):
        self.run_case_with_original_and_random(self.CASES[3])

    def test_random_original(self):
        self.run_case_with_original_and_random(self.CASES[4])

    def test_singleton(self):
        self.run_case_with_original_and_random(self.CASES[5])


class TestNaiveGeneration(unittest.TestCase):

    # -------------------------
    # Pretty printing helpers
    # -------------------------

    def format_env(self, env: dict[str, set]) -> str:
        return ", ".join(
            f"{k}={{{', '.join(sorted(v))}}}"
            for k, v in sorted(env.items())
        )

    def repr_env(self, env: dict[str, set]) -> str:
        lines = ["{"]
        for name in sorted(env):
            elems = ", ".join(repr(x) for x in sorted(env[name]))
            lines.append(f'    "{name}": {{{elems}}},')
        lines.append("}")
        return "\n".join(lines)

    # -------------------------
    # Core reusable runner
    # -------------------------

    def run_formula_case(
        self,
        clauses,
        env,
        *,
        dependency_names=None,
        singleton_names=None,
        debug_py=False,
        debug_dot=False,
        seed=None,
        trial=None,
        case_name=None,
    ):
        dependency_names = dependency_names or {}
        singleton_names = singleton_names or ()

        wanted = set()
        for clause in clauses:
            wanted |= clause.eval(env)

        f = Formula(clauses)

        actual = set(f.naive(env).data)
        print()

        self.assertSetEqual(
            wanted,
            actual,
            msg=(
                f"\ncase={case_name}"
                f"\nseed={seed}"
                f"\ntrial={trial}"
                f"\nclauses={clauses}"
                f"\nenv={self.repr_env(env)}"
                f"\nwanted={sorted(wanted)}"
                f"\nactual={sorted(actual)}"
            ),
        )

    # -------------------------
    # Random env helpers
    # -------------------------

    def random_env(self, variables, universe=None, rng=None):
        rng = rng or random
        universe = universe or tuple("123456789ABCDEF")

        return {
            v: set(rng.sample(universe, rng.randint(0, len(universe))))
            for v in variables
        }

    def run_random_envs(
        self,
        clauses,
        *,
        dependency_names=None,
        singleton_names=None,
        variables=None,
        trials=50,
        seed=0,
        case_name=None,
    ):
        if variables is None:
            variables = sorted(set().union(*(c.P | c.N for c in clauses)))

        rng = random.Random(seed)

        for i in range(trials):
            env = self.random_env(variables, rng=rng)

            with self.subTest(case=case_name, i=i, seed=seed, env=self.format_env(env)):
                self.run_formula_case(
                    clauses,
                    env,
                    dependency_names=dependency_names,
                    singleton_names=singleton_names,
                    seed=seed,
                    trial=i,
                    case_name=case_name,
                )

    # -------------------------
    # Run one case both ways
    # -------------------------

    def run_case_with_original_and_random(self, case):
        name = case["name"]
        clauses = case["clauses"]
        env = case["env"]
        dependency_names = case.get("dependency_names", {})
        singleton_names = case.get("singleton_names", ())
        variables = case.get("variables")
        trials = case.get("trials", 50)
        seed = case.get("seed", 0)

        with self.subTest(case=name, kind="original"):
            self.run_formula_case(
                clauses,
                env,
                dependency_names=dependency_names,
                singleton_names=singleton_names,
                case_name=name,
            )

        self.run_random_envs(
            clauses,
            dependency_names=dependency_names,
            singleton_names=singleton_names,
            variables=variables,
            trials=trials,
            seed=seed,
            case_name=name,
        )

    # -------------------------
    # All cases in one place
    # -------------------------

    CASES = [
        {
            "name": "clause",
            "clauses": [
                Clause.make({"a", "b", "c", "d"}, {"e", "f"}),
            ],
            "env": {
                "a": {"1", "3", "A", "B", "C"},
                "b": {"2", "3", "A", "B"},
                "c": {"1", "3", "4", "A", "B"},
                "d": {"1", "3", "A", "B"},
                "e": {"1", "A"},
                "f": {"3"},
            },
            "dependency_names": {
                "a": ("b", "c", "d", "e", "f"),
                "b": ("a", "c", "d", "e", "f"),
                "c": ("a", "b", "d", "e", "f"),
                "d": ("a", "b", "c", "e", "f"),
                "e": ("a", "b", "c", "d", "f"),
                "f": ("a", "b", "c", "d", "e")
            },
            "variables": ["a", "b", "c", "d", "e", "f"],
            "trials": 100,
            "seed": 10,
        },
        {
            "name": "two_results_non_overlapping",
            "clauses": [
                Clause.make({"a", "b"}, {"c"}),
                Clause.make({"d"}, {"e"}),
            ],
            "env": {
                "a": {"1", "3", "A", "B", "C"},
                "b": {"2", "3", "A", "B"},
                "c": {"1", "3", "4", "A", "B"},
                "d": {"1", "3", "A", "B"},
                "e": {"1", "A"},
            },
            "variables": ["a", "b", "c", "d", "e"],
            "trials": 100,
            "seed": 11,
        },
        {
            "name": "overlapping_intersections",
            "clauses": [
                Clause.make({"a", "c"}, {"d"}),
                Clause.make({"b", "c"}, {"e"}),
            ],
            "env": {
                "a": {"1", "3", "A", "B", "C"},
                "b": {"2", "3", "A", "B"},
                "c": {"1", "3", "4", "A", "B"},
                "d": {"1", "3", "B"},
                "e": {"1", "A"},
            },
            "dependency_names": {
                "c": ("a", "b"),
            },
            "variables": ["a", "b", "c", "d", "e"],
            "trials": 100,
            "seed": 12,
        },
        {
            "name": "first_formula",
            "clauses": [
                Clause.make({"a", "c"}, {"d"}),
                Clause.make({"b", "c"}, {"d"}),
            ],
            "env": {
                "a": {"1", "B", "C"},
                "b": {"2", "3", "A"},
                "c": {"3", "4", "A", "B"},
                "d": {"1", "A"},
            },
            "dependency_names": {
                "c": ("a", "b"),
            },
            "variables": ["a", "b", "c", "d"],
            "trials": 100,
            "seed": 13,
        },
        {
            "name": "random_original",
            "clauses": [
                Clause.make({"a", "b"}, {"c"}),
                Clause.make({"b"}, {"c"}),
            ],
            "env": {
                "a": {"1"},
                "b": {"3", "4", "F"},
                "c": {"2", "C"},
                "d": {"1", "2", "7", "B", "C"},
                "e": {"4"},
            },
            "dependency_names": {
                "a": ("b",),
                "b": ("a",),
                "c": ("a", "b"),
            },
            "singleton_names": ("b", ),
            "variables": ["a", "b", "c", "d", "e"],
            "trials": 100,
            "seed": 14,
        },
        {
            "name": "singleton",
            "clauses": [
                Clause.make({"a", "b"}, {"c"}),
                Clause.make({"b"}, {"d"}),
            ],
            "env": {
                "a": {"1", "4", "5", "6"},
                "b": {"2", "4", "5", "6", "7"},
                "c": {"3", "5"},
                "d": {"5", "6"},
            },
            "dependency_names": {
                "a": ("b",),
                "b": ("a",),
                "c": ("a", "b"),
                "d": ("b",),
            },
            "singleton_names": ("b",),
            "variables": ["a", "b", "c", "d"],
            "trials": 100,
            "seed": 15,
        },
    ]

    # -------------------------
    # One test per formula
    # -------------------------

    def test_clause(self):
        self.run_case_with_original_and_random(self.CASES[0])

    def test_two_results_non_overlapping(self):
        self.run_case_with_original_and_random(self.CASES[1])

    def test_overlapping_intersections(self):
        self.run_case_with_original_and_random(self.CASES[2])

    def test_first_formula(self):
        self.run_case_with_original_and_random(self.CASES[3])

    def test_random_original(self):
        self.run_case_with_original_and_random(self.CASES[4])

    def test_singleton(self):
        self.run_case_with_original_and_random(self.CASES[5])



if __name__ == "__main__":
    unittest.main()