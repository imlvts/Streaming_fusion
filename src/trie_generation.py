from src.clause import Clause, Formula
from src.trie import bittrieset
from src.trie_synth import Source, Sink


class TrieExecution:
    @staticmethod
    def naive(formula, env):
        srcs = {k: Source(k, v) for (k, v) in env.items()}
        r = Sink()
        values = {k: s.descend_or_next() for (k, s) in srcs.items()}

        for i in range(500):
            matched = False

            for c in formula.clauses:
                ps = list(c.P)
                if all(values[p] is not None for p in ps):
                    print("got here")
                    print(all(values[ps[0]].path() == values[p].path() for p in ps))
                    print(all(values[p].is_value() for p in ps))
                    print(all(values[n].path() > values[ps[0]].path() or (values[n].path() == values[ps[0]].path() and not values[n].is_value()) for n in c.N if values[n] is not None))
                    if (
                            all(values[ps[0]].path() == values[p].path() for p in ps)
                            and all(values[p].is_value() for p in ps)
                            and all(values[n].path() > values[ps[0]].path() or (values[n].path() == values[ps[0]].path() and not values[n].is_value()) for n in c.N if values[n] is not None)
                    ):
                        r.push(values[ps[0]].path())
                        p = ps[0]
                        values[p] = srcs[p].descend_or_next()   #TODO see how much we can skip!
                        matched = True
                        break

            if matched:
                continue

            if all(v is None for v in values.values()):
                break

            # compute minimum path value
            min_path = min(
                (v.path() for v in values.values() if v is not None),
            )

            # get all keys with that minimum
            mins = [
                k for k, v in values.items()
                if v is not None and v.path() == min_path
            ]

            # update all of them
            for k in mins:
                values[k] = srcs[k].descend_or_next()   #TODO see how much we can skip!

        return r


if __name__ == '__main__':
    c1 = Clause.make({"a", "b"}, {"c"})
    c2 = Clause.make({"b"}, {"d"})
    f = Formula([c1, c2])

    x, y, z, w, _1, _2, _3 = ('000', '001', '010', '011', '100', '101', '110')

    # a = Source('a', bittrieset(x, _2, _3))
    a = Source('a', bittrieset(y, _1, _2, _3))
    b = Source('b', bittrieset(y, z, _1, _2))
    c = Source('c', bittrieset(z, w, _1, _2))
    d = Source('d', bittrieset(x, _1))

    env = {
        "a": bittrieset(y, _1, _2, _3),
        "b": bittrieset(y, z, _1, _2),
        "c": bittrieset(z, w, _1, _2),
        "d": bittrieset(x, _1),
    }

    wanted = f.eval(env)

    r = TrieExecution.naive(f, env)
    print(set(r.data))
    print(wanted)

