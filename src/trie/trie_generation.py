from src.clause import Clause, Formula
from src.trie.trie import bittrieset
from src.trie.trie_synth import *


class TrieExecution:
    @staticmethod
    def naive(formula, env):
        print(formula.groups())
        srcs = {k: Source(k, v) for (k, v) in env.items() if k in formula.vars()}
        r = Sink()
        values = {k: s.descend_or_next() for (k, s) in srcs.items()}

        for i in range(500):
            print("ITER", i)
            matched = False

            # check whether there are paths to add to the results
            for c in formula.clauses:
                ps = list(c.P)
                if all(values[p] is not None for p in ps):
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
            print("next iter")

            print("all mins", mins)
            for k in mins:
                if k in formula.singletons():
                    values[k] = srcs[k].descend_or_next()
                    continue


                print("for minimum", k)
                pgs, ngs = formula.groups()

                pos_candidates = [
                    max(
                        (p for p in pg if values[p] is not None),
                        key=lambda p: values[p].path(),
                        default=None
                    )
                    for pg in pgs[k]
                    if pg and any(values[p] is not None for p in pg)
                ]
                # print("pos candidates: ", [{p: values[p].path() for p in pg} for pg in pgs])

                temp_pos = min(pos_candidates, key=lambda p: values[p].path()) if pos_candidates else None

                neg_candidates = [
                    max(
                        (n for n in ng if values[n] is not None),
                        key=lambda n: values[n].path(),
                        default=None
                    )
                    for ng in ngs[k]
                    if ng and any(values[n] is not None for n in ng)
                ]
                temp_neg = min(neg_candidates, key=lambda p: values[p].path()) if neg_candidates else None

                candidates = [x for x in (temp_pos, temp_neg) if x is not None]
                temp = min(candidates, key=lambda c: values[c].path()) if candidates else None
                print("approach", temp, values[temp].path() if temp else '/')

                if temp is None:
                    values[k] = srcs[k].descend_or_next() # TODO what to do?
                    continue

                if values[temp].path() == values[k].path():
                    values[k] = srcs[k].descend_or_next()
                else:
                    if srcs[k].prefix_of(srcs[temp]):
                        values[k] = srcs[k].descend_or_next()

                    else:
                        i = srcs[k].difference_level(srcs[temp])
                        values[k] = srcs[k].next(i)

                # print("min", k, temp)

        return r

    @staticmethod
    def create_graph(formula):
        g = Graph()
        s0, s1 = g.states('s0', 's1')


    """
    @staticmethod
    def create_graph(formula):
        g = Graph()
        srcs = {v : g.sources(v)[0] for v in formula.vars()}
        s0 = g.states('s0')
        states = g.states('s1', 's2', 's3', 's4', 's5', 's6', 's7', 's8', 's9', 's10', 's11', 's12',"s15")

        g.init = s0

        s0.to(states[0], descend=(srcs.values()))

        r, = g.sinks('r')

        for e, c in enumerate(formula.clauses):
            ps = list(c.P)
            states[e].to(states[e + 1], *(NotFinished(p) for p in c.P), *(ps[0] == p for p in ps), *(IsValue(p) for p in ps))

        if (
                all(values[ps[0]].path() == values[p].path() for p in ps)
                and all(values[p].is_value() for p in ps)
                and all(values[n].path() > values[ps[0]].path() or (
                values[n].path() == values[ps[0]].path() and not values[n].is_value()) for n in c.N if
                        values[n] is not None)
        )
    """


if __name__ == '__main__':
    """
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
    """

    clauses = [Clause(P=frozenset({'a', 'b'}), N=frozenset({'d'})), Clause(P=frozenset({'d', 'c'}), N=frozenset({'a'}))]
    env = {
        "a": {('0001', None), ('00010', None), ('000111', None), ('011', None), ('0111', None), ('101', None),
              ('101010', None)},
        "b": {('000111', None), ('101010', None)},
        "c": {('0100', None), ('100000', None), ('101010', None), ('1011', None), ('1110', None)},
        "d": {('101010', None)},
    }


    f = Formula(clauses)

    TrieExecution.create_graph(f)

    """
    env = {k: bittrieset(*[e[0] for e in v]) for k, v in env.items()}

    wanted = f.eval(env)

    r = TrieExecution.naive(f, env)

    print(f.show())
    print("found", set(r.data))
    print("wanted", wanted)
    """

