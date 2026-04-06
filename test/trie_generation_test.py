import random
import unittest
from abc import ABC, abstractmethod

from src.clause import Clause, Formula
from src.trie import bittrieset, BitTrieMap
from src.trie_generation import TrieExecution

CASES = [
    {
        "name": "clause",
        "clauses": [
            Clause.make({"a", "b", "c", "d"}, {"e", "f"}),
        ],
        "env": {
            "a": {"0", "001", "011", "1011", "11000"},
            "b": {"001", "011", "111", "11000"},
            "c": {"0", "001", "011", "100", "11000"},
            "d": {"0", "001", "011", "11000"},
            "e": {"0", "001"},
            "f": {"011"},
        },
        "dependency_names": {
            "a": ("b", "c", "d", "e", "f"),
            "b": ("a", "c", "d", "e", "f"),
            "c": ("a", "b", "d", "e", "f"),
            "d": ("a", "b", "c", "e", "f"),
            "e": ("a", "b", "c", "d", "f"),
            "f": ("a", "b", "c", "d", "e"),
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
            "a": {"0", "011", "1011", "11000", "11111"},
            "b": {"001", "011", "1011", "11000"},
            "c": {"0", "011", "100", "1011", "11000"},
            "d": {"0", "011", "1011", "11000"},
            "e": {"0", "1011"},
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
            "a": {"0", "011", "1011", "11000", "11111"},
            "b": {"001", "011", "1011", "11000"},
            "c": {"0", "011", "100", "1011", "11000"},
            "d": {"0", "011", "11000"},
            "e": {"0", "1011"},
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
            "a": {"0", "1011", "11111"},
            "b": {"001", "011", "11000"},
            "c": {"011", "100", "1011", "11000"},
            "d": {"0", "1011"},
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
            "a": {"0"},
            "b": {"011", "100", "11111"},
            "c": {"001", "1011"},
            "d": {"0", "001", "111", "11000", "1011"},
            "e": {"100"},
        },
        "dependency_names": {
            "a": ("b",),
            "b": ("a",),
            "c": ("a", "b"),
        },
        "singleton_names": ("b",),
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
            "a": {"0", "100", "101", "110"},
            "b": {"001", "100", "101", "110", "111"},
            "c": {"011", "101"},
            "d": {"101", "110"},
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


class FormulaTestBase(unittest.TestCase, ABC):
    maxDiff = None
    CASES = CASES

    def format_env(self, env):
        return ", ".join(
            f"{k}={{{', '.join(sorted(v))}}}"
            for k, v in sorted(env.items())
        )

    def repr_env(self, env):
        lines = ["{"]
        for name in sorted(env):
            elems = ", ".join(repr(x) for x in sorted(env[name]))
            lines.append(f'    "{name}": {{{elems}}},')
        lines.append("}")
        return "\n".join(lines)


    def assert_formula_result(self, *, wanted, actual, clauses, env, **meta):
        self.assertSetEqual(
            wanted,
            actual,
            msg=(
                f"\ncase={meta.get('case_name')}"
                f"\nseed={meta.get('seed')}"
                f"\ntrial={meta.get('trial')}"
                f"\nclauses={clauses}"
                f"\nenv={self.repr_env(env)}"
                f"\nwanted={sorted(wanted)}"
                f"\nactual={sorted(actual)}"
            ),
        )

    # -------- random path generation --------

    def random_path(self, rng):
        return "".join(rng.choice("01") for _ in range(rng.randint(1, 6)))

    def random_pool(self, rng, size=50):
        pool = set()
        while len(pool) < size:
            pool.add(self.random_path(rng))
        return list(pool)

    def random_env_for_clauses(self, clauses, variables, rng):
        pool = self.random_pool(rng)
        env = {v: set() for v in variables}

        # inject overlap per clause
        for clause in clauses:
            shared = rng.sample(pool, rng.randint(1, 3))
            for v in clause.P:
                env[v].update(shared)

        # add noise
        for v in variables:
            env[v].update(rng.sample(pool, rng.randint(0, 5)))

        # convert plain sets -> bittrieset
        return {v: bittrieset(*paths) for v, paths in env.items()}
    # -------- runners --------

    @abstractmethod
    def run_formula_case(self, clauses, env, **kwargs):
        pass

    def run_random_envs(self, clauses, *, variables, trials, seed, case_name, **kwargs):
        rng = random.Random(seed)

        for i in range(trials):
            env = self.random_env_for_clauses(clauses, variables, rng)

            with self.subTest(case=case_name, i=i):
                self.run_formula_case(
                    clauses,
                    env,
                    trial=i,
                    seed=seed,
                    case_name=case_name,
                )

    def run_case_with_original_and_random(self, case):
        name = case["name"]

        # original
        # with self.subTest(case=name, kind="original"):
        #     self.run_formula_case(
        #         case["clauses"],
        #         case["env"],
        #         case_name=name,
        #     )

        # random
        self.run_random_envs(
            case["clauses"],
            variables=case.get("variables"),
            trials=case.get("trials", 50),
            seed=case.get("seed", 0),
            case_name=name,
        )


class TestNaiveGeneration(FormulaTestBase):
    def run_formula_case(self, clauses, env, **meta):
        formula = Formula(clauses)
        wanted = set(formula.eval(env).keys_iterator())
        print(wanted)
        actual = set(TrieExecution.naive(Formula(clauses), env).data)

        self.assert_formula_result(
            wanted=wanted,
            actual=actual,
            clauses=clauses,
            env=env,
            **meta,
        )

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