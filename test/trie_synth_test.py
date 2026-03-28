from src.trie_synth import *
import unittest
import random


def random_bitstring(min_len=1, max_len=6):
    length = random.randint(min_len, max_len)
    return ''.join(random.choice('01') for _ in range(length))

def random_bitstring_set(min_size=1, max_size=10):
    length = random.randint(min_size, max_size)
    return {random_bitstring() for _ in range(length)}


class TestTrieOps(unittest.TestCase):
    def t(self, a, b, c, d):
        g = ctx()
        # g.py()

        r = Sink()
        r_wanted = ((a.data | b.data) & c.data) - d.data
        print()
        print(r_wanted)


        s = StringIO()
        with redirect_stdout(s):
            g.py()
        try:
            exec(s.getvalue())
        except IndexError:
            print("stopped by exhaustion")

        # print("result", r.data)
        self.assertEqual(set(r_wanted.keys_iterator()), set(r.data))
        print(r_wanted)
        print(r.data)


    def test_initial_example(self):
        x, y, z, w, _1, _2, _3 = ('000', '001', '010', '011', '100', '101', '110')

        a = Source('a', bittrieset(x, _2, _3))
        b = Source('b', bittrieset(y, z, _1))
        c = Source('c', bittrieset(z, w, _1, _2))
        d = Source('d', bittrieset(x, _1))

        self.t(a, b, c, d)

    def test_nested(self):
        a = Source('a', bittrieset("1", "11", "111"))
        b = Source('b', bittrieset("0", "00", "000"))
        c = Source('c', bittrieset("0", "11", "111"))
        d = Source('d', bittrieset("11", "111"))

        self.t(a, b, c, d)

    def test_random_bitsets(self):
        for _ in range(1000):  # run many random cases
            a_set = random_bitstring_set()
            b_set = random_bitstring_set()
            c_set = random_bitstring_set()
            d_set = random_bitstring_set()

            print(a_set, b_set, c_set, d_set)

            a = Source('a', bittrieset(*a_set))
            b = Source('b', bittrieset(*b_set))
            c = Source('c', bittrieset(*c_set))
            d = Source('d', bittrieset(*d_set))

            self.t(a, b, c, d)  # your implementation

            # expected = (a_set | b_set) & c_set - d_set

            # assert result == expected

class TestIntersection(unittest.TestCase):
    def t(self, a, b, c):
        g = intersection_graph()
        # g.py()

        r = Sink()
        r_wanted = ((a.data & b.data) & c.data)
        print()
        print(r_wanted)


        s = StringIO()
        with redirect_stdout(s):
            g.py()
        try:
            exec(s.getvalue())
        except IndexError:
            print("stopped by exhaustion")

        # print("result", r.data)
        self.assertEqual(set(r_wanted.keys_iterator()), set(r.data))
        print(r_wanted)
        print(r.data)


    def test_initial_example(self):
        x, y, z, w, _1, _2, _3 = ('000', '001', '010', '011', '100', '101', '110')

        a = Source('a', bittrieset(y, _1, _2, _3))
        b = Source('b', bittrieset(y, z, _1))
        c = Source('c', bittrieset(z, w, _1, _2))

        self.t(a, b, c)

    def test_nested(self):
        a = Source('a', bittrieset("1", "11", "111"))
        b = Source('b', bittrieset("0", "11", "000"))
        c = Source('c', bittrieset("0", "11", "111"))

        self.t(a, b, c)

    def test_random_bitsets(self):
        for _ in range(1000):  # run many random cases
            all_elems = random_bitstring_set(10, 11)
            k = 6
            if len(all_elems) < k:
                k = len(all_elems)
            a_set = random.sample(list(all_elems), k)
            b_set = random.sample(list(all_elems), k)
            c_set = random.sample(list(all_elems), k)

            print(a_set, b_set, c_set)

            a = Source('a', bittrieset(*a_set))
            b = Source('b', bittrieset(*b_set))
            c = Source('c', bittrieset(*c_set))

            self.t(a, b, c)  # your implementation

            # expected = (a_set | b_set) & c_set - d_set

            # assert result == expected


if __name__ == "__main__":
    unittest.main()