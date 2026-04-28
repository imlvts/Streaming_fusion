def argmin(srcs):
    return min((e for e in srcs if e is not None), key=lambda e: e.path(), default=None)

def argmax(srcs):
    return max((e for e in srcs if e is not None), key=lambda e: e.path(), default=None)
