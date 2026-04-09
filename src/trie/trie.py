# ** By Adam Vandervorst **

from __future__ import annotations

from dataclasses import dataclass
from types import NoneType
from typing import Callable, Generic, Iterable, Iterator, Optional, TypeVar, Literal

T = TypeVar("T")
S = TypeVar("S")
R = TypeVar("R")


def _validate_bitstring(key: str) -> None:
    if any(ch not in "01" for ch in key):
        raise ValueError(f"invalid bitstring key: {key!r}")


def _dot_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


@dataclass(frozen=True)
class _Node(Generic[T]):
    has_value: bool = False
    value: Optional[T] = None
    left: Optional["_Node[T]"] = None
    right: Optional["_Node[T]"] = None

    def is_empty(self) -> bool:
        return not self.has_value and self.left is None and self.right is None


def _mk_node(
    *,
    has_value: bool,
    value: Optional[T],
    left: Optional[_Node[T]],
    right: Optional[_Node[T]],
) -> Optional[_Node[T]]:
    node = _Node(has_value=has_value, value=value, left=left, right=right)
    return None if node.is_empty() else node


class BitTrieMap(Generic[T]):
    __slots__ = ("_root",)

    def __init__(self, root: Optional[_Node[T]] = None) -> None:
        self._root = root

    def ref(self, path=""):
        return TrieRef(self, path)

    # ---------- constructors ----------

    @staticmethod
    def empty() -> "BitTrieMap[T]":
        return BitTrieMap()

    @staticmethod
    def single(key: str, value: T) -> "BitTrieMap[T]":
        _validate_bitstring(key)
        return BitTrieMap.empty().updated(key, value)

    @staticmethod
    def from_items(items: Iterable[tuple[str, T]]) -> "BitTrieMap[T]":
        m: BitTrieMap[T] = BitTrieMap.empty()
        for k, v in items:
            m = m.updated(k, v)
        return m

    # ---------- basic queries ----------

    def is_empty(self) -> bool:
        return self._root is None

    def non_empty(self) -> bool:
        return self._root is not None

    def known_size(self) -> int:
        return 0 if self._root is None else -1

    def _size(self) -> int:
        def rec(node: Optional[_Node[T]]) -> int:
            if node is None:
                return 0
            return (1 if node.has_value else 0) + rec(node.left) + rec(node.right)

        return rec(self._root)

    def _contains(self, key: str) -> bool:
        _validate_bitstring(key)
        node = self.get_node(key)
        return node is not None and node.has_value

    def get_node(self, key: str) -> Optional[_Node[T]]:
        _validate_bitstring(key)
        node = self._root
        for ch in key:
            if node is None:
                return None
            node = node.left if ch == "0" else node.right
        return node

    def path_exists(self, key: str) -> Optional[_Node[T]]:
        _validate_bitstring(key)
        return self.get_node(key) is not None

    def get(self, key: str) -> Optional[T]:
        _validate_bitstring(key)
        node = self.get_node(key)
        if node is None or not node.has_value:
            return None
        return node.value

    def get_or_else(self, key: str, default: S) -> T | S:
        v = self.get(key)
        return default if v is None else v

    def __getitem__(self, key: str) -> T:
        v = self.get(key)
        if v is None:
            raise KeyError(key)
        return v

    # ---------- updates ----------

    def updated(self, key: str, value: S) -> "BitTrieMap[S]":
        _validate_bitstring(key)

        def rec(node: Optional[_Node[object]], i: int) -> _Node[object]:
            if node is None:
                node = _Node()

            if i == len(key):
                return _Node(True, value, node.left, node.right)

            if key[i] == "0":
                new_left = rec(node.left, i + 1)
                return _Node(node.has_value, node.value, new_left, node.right)
            else:
                new_right = rec(node.right, i + 1)
                return _Node(node.has_value, node.value, node.left, new_right)

        return BitTrieMap(rec(self._root, 0))  # type: ignore[arg-type]

    def updated_with(self, key: str, remap: Callable[[Optional[T]], Optional[S]]) -> "BitTrieMap[S]":
        current = self.get(key)
        newv = remap(current)
        if current is None:
            return self if newv is None else self.updated(key, newv)  # type: ignore[return-value]
        return self.removed(key) if newv is None else self.updated(key, newv)

    def updated_with_default(self, key: str, value: S, f: Callable[[T], S]) -> "BitTrieMap[S]":
        current = self.get(key)
        return self.updated(key, value if current is None else f(current))

    def removed(self, key: str) -> "BitTrieMap[T]":
        _validate_bitstring(key)

        def rec(node: Optional[_Node[T]], i: int) -> Optional[_Node[T]]:
            if node is None:
                return None

            if i == len(key):
                return _mk_node(
                    has_value=False,
                    value=None,
                    left=node.left,
                    right=node.right,
                )

            if key[i] == "0":
                new_left = rec(node.left, i + 1)
                return _mk_node(
                    has_value=node.has_value,
                    value=node.value,
                    left=new_left,
                    right=node.right,
                )

            new_right = rec(node.right, i + 1)
            return _mk_node(
                has_value=node.has_value,
                value=node.value,
                left=node.left,
                right=new_right,
            )

        return BitTrieMap(rec(self._root, 0))

    # ---------- iteration ----------

    def iterator(self) -> Iterator[tuple[str, T]]:
        def rec(node: Optional[_Node[T]], prefix: str) -> Iterator[tuple[str, T]]:
            if node is None:
                return
            if node.has_value:
                yield (prefix, node.value)  # type: ignore[misc]
            yield from rec(node.left, prefix + "0")
            yield from rec(node.right, prefix + "1")

        yield from rec(self._root, "")

    def keys_iterator(self) -> Iterator[str]:
        for k, _ in self.iterator():
            yield k

    def values_iterator(self) -> Iterator[T]:
        for _, v in self.iterator():
            yield v

    def foreach(self, f: Callable[[tuple[str, T]], object]) -> None:
        for item in self.iterator():
            f(item)

    def foreach_entry(self, f: Callable[[str, T], object]) -> None:
        for k, v in self.iterator():
            f(k, v)

    def foreach_key(self, f: Callable[[str], object]) -> None:
        for k in self.keys_iterator():
            f(k)

    def foreach_value(self, f: Callable[[T], object]) -> None:
        for v in self.values_iterator():
            f(v)

    # ---------- transforms / filters ----------

    def filter(self, f: Callable[[str, T], bool]) -> "BitTrieMap[T]":
        def rec(node: Optional[_Node[T]], prefix: str) -> Optional[_Node[T]]:
            if node is None:
                return None

            keep_here = False
            value_here: Optional[T] = None
            if node.has_value and f(prefix, node.value):  # type: ignore[arg-type]
                keep_here = True
                value_here = node.value

            new_left = rec(node.left, prefix + "0")
            new_right = rec(node.right, prefix + "1")

            if (
                keep_here == node.has_value
                and value_here is node.value
                and new_left is node.left
                and new_right is node.right
            ):
                return node

            return _mk_node(
                has_value=keep_here,
                value=value_here,
                left=new_left,
                right=new_right,
            )

        return BitTrieMap(rec(self._root, ""))

    def map_values_now(self, f: Callable[[T], S]) -> "BitTrieMap[S]":
        def rec(node: Optional[_Node[T]]) -> Optional[_Node[S]]:
            if node is None:
                return None

            new_left = rec(node.left)
            new_right = rec(node.right)

            if node.has_value:
                new_value = f(node.value)  # type: ignore[arg-type]
                return _mk_node(
                    has_value=True,
                    value=new_value,
                    left=new_left,
                    right=new_right,
                )

            return _mk_node(
                has_value=False,
                value=None,
                left=new_left,
                right=new_right,
            )

        return BitTrieMap(rec(self._root))

    def transform(self, f: Callable[[str, T], S]) -> "BitTrieMap[S]":
        def rec(node: Optional[_Node[T]], prefix: str) -> Optional[_Node[S]]:
            if node is None:
                return None

            new_left = rec(node.left, prefix + "0")
            new_right = rec(node.right, prefix + "1")

            if node.has_value:
                new_value = f(prefix, node.value)  # type: ignore[arg-type]
                return _mk_node(
                    has_value=True,
                    value=new_value,
                    left=new_left,
                    right=new_right,
                )

            return _mk_node(
                has_value=False,
                value=None,
                left=new_left,
                right=new_right,
            )

        return BitTrieMap(rec(self._root, ""))

    def modify_or_remove(self, f: Callable[[str, T], Optional[S]]) -> "BitTrieMap[S]":
        def rec(node: Optional[_Node[T]], prefix: str) -> Optional[_Node[S]]:
            if node is None:
                return None

            new_left = rec(node.left, prefix + "0")
            new_right = rec(node.right, prefix + "1")

            if node.has_value:
                out = f(prefix, node.value)  # type: ignore[arg-type]
                if out is None:
                    return _mk_node(
                        has_value=False,
                        value=None,
                        left=new_left,
                        right=new_right,
                    )
                return _mk_node(
                    has_value=True,
                    value=out,
                    left=new_left,
                    right=new_right,
                )

            return _mk_node(
                has_value=False,
                value=None,
                left=new_left,
                right=new_right,
            )

        return BitTrieMap(rec(self._root, ""))

    # ---------- binary trie ops ----------

    def union_with(self, that: "BitTrieMap[S]", f: Callable[[str, S, S], S]) -> "BitTrieMap[S]":
        def rec(
            n1: Optional[_Node[T]],
            n2: Optional[_Node[S]],
            prefix: str,
        ) -> Optional[_Node[S]]:
            if n1 is None:
                return n2
            if n2 is None:
                return n1  # type: ignore[return-value]

            left = rec(n1.left, n2.left, prefix + "0")
            right = rec(n1.right, n2.right, prefix + "1")

            if n1.has_value and n2.has_value:
                value = f(prefix, n1.value, n2.value)  # type: ignore[arg-type]
                return _mk_node(has_value=True, value=value, left=left, right=right)

            if n1.has_value:
                return _mk_node(
                    has_value=True,
                    value=n1.value,  # type: ignore[arg-type]
                    left=left,
                    right=right,
                )

            if n2.has_value:
                return _mk_node(
                    has_value=True,
                    value=n2.value,
                    left=left,
                    right=right,
                )

            return _mk_node(has_value=False, value=None, left=left, right=right)

        return BitTrieMap(rec(self._root, that._root, ""))

    def union(self, that: "BitTrieMap[S]") -> "BitTrieMap[S]":
        return self.union_with(that, lambda _k, x, _y: x)

    def intersection_with(self, that: "BitTrieMap[S]", f: Callable[[str, T, S], R]) -> "BitTrieMap[R]":
        def rec(
            n1: Optional[_Node[T]],
            n2: Optional[_Node[S]],
            prefix: str,
        ) -> Optional[_Node[R]]:
            if n1 is None or n2 is None:
                return None

            left = rec(n1.left, n2.left, prefix + "0")
            right = rec(n1.right, n2.right, prefix + "1")

            if n1.has_value and n2.has_value:
                value = f(prefix, n1.value, n2.value)  # type: ignore[arg-type]
                return _mk_node(has_value=True, value=value, left=left, right=right)

            return _mk_node(has_value=False, value=None, left=left, right=right)

        return BitTrieMap(rec(self._root, that._root, ""))

    def intersection(self, that: "BitTrieMap[S]") -> "BitTrieMap[T]":
        return self.intersection_with(that, lambda _k, v, _v2: v)

    def subtract_with(self, p: Callable[[S, S], Optional[S]], m2: "BitTrieMap[S]") -> "BitTrieMap[S]":
        def rec(
            n1: Optional[_Node[T]],
            n2: Optional[_Node[S]],
        ) -> Optional[_Node[S]]:
            if n1 is None:
                return None

            left = rec(n1.left, None if n2 is None else n2.left)
            right = rec(n1.right, None if n2 is None else n2.right)

            if n2 is None or not n2.has_value:
                if n1.has_value:
                    return _mk_node(
                        has_value=True,
                        value=n1.value,  # type: ignore[arg-type]
                        left=left,
                        right=right,
                    )
                return _mk_node(has_value=False, value=None, left=left, right=right)

            if not n1.has_value:
                return _mk_node(has_value=False, value=None, left=left, right=right)

            out = p(n1.value, n2.value)  # type: ignore[arg-type]
            if out is None:
                return _mk_node(has_value=False, value=None, left=left, right=right)
            return _mk_node(has_value=True, value=out, left=left, right=right)

        return BitTrieMap(rec(self._root, m2._root))

    def subtract(self, that: "BitTrieMap[object]") -> "BitTrieMap[T]":
        def rec(
            n1: Optional[_Node[T]],
            n2: Optional[_Node[object]],
        ) -> Optional[_Node[T]]:
            if n1 is None:
                return None

            left = rec(n1.left, None if n2 is None else n2.left)
            right = rec(n1.right, None if n2 is None else n2.right)

            keep_here = n1.has_value and not (n2 is not None and n2.has_value)
            value_here = n1.value if keep_here else None

            return _mk_node(
                has_value=keep_here,
                value=value_here,
                left=left,
                right=right,
            )

        return BitTrieMap(rec(self._root, that._root))

    # ---------- boundary keys ----------

    def first_key(self) -> str:
        def rec(node: Optional[_Node[T]], prefix: str) -> str:
            if node is None:
                raise ValueError("empty BitTrieMap")
            if node.has_value:
                return prefix
            if node.left is not None:
                return rec(node.left, prefix + "0")
            if node.right is not None:
                return rec(node.right, prefix + "1")
            raise ValueError("empty BitTrieMap")

        return rec(self._root, "")

    def last_key(self) -> str:
        def rec(node: Optional[_Node[T]], prefix: str) -> str:
            if node is None:
                raise ValueError("empty BitTrieMap")
            if node.right is not None:
                return rec(node.right, prefix + "1")
            if node.left is not None:
                return rec(node.left, prefix + "0")
            if node.has_value:
                return prefix
            raise ValueError("empty BitTrieMap")

        return rec(self._root, "")

    # ---------- dot export ----------

    def to_logical_dot(self, *, show_values: bool = True) -> str:
        lines: list[str] = [
            "digraph BitTrieMapLogical {",
            "  rankdir=TB;",
        ]

        if self._root is None:
            lines.append('  n0 [shape=box, label="∅"];')
            lines.append("}")
            return "\n".join(lines)

        ids: dict[int, str] = {}
        counter = 0

        def node_id(node: _Node[T]) -> str:
            nonlocal counter
            oid = id(node)
            if oid not in ids:
                ids[oid] = f"n{counter}"
                counter += 1
            return ids[oid]

        stack: list[tuple[_Node[T], int, str]] = [(self._root, 0, "")]
        while stack:
            node, depth, path = stack.pop()
            nid = node_id(node)

            if depth == 0:
                if node.has_value:
                    if show_values:
                        value_repr = _dot_escape(repr(node.value))
                        lines.append(f'  {nid} [shape=box, label="ε\\nvalue={value_repr}"];')
                    else:
                        lines.append(f'  {nid} [shape=box, label="ε"];')
                else:
                    lines.append(f'  {nid} [shape=circle, label="root"];')
            else:
                if node.has_value:
                    key_repr = _dot_escape(path)
                    if show_values:
                        value_repr = _dot_escape(repr(node.value))
                        lines.append(f'  {nid} [shape=box, label="{key_repr}\\nvalue={value_repr}"];')
                    else:
                        lines.append(f'  {nid} [shape=box, label="{key_repr}"];')
                else:
                    lines.append(f'  {nid} [shape=point];')

            if node.left is not None:
                left_id = node_id(node.left)
                lines.append(f'  {nid} -> {left_id} [label="0", color=red];')
                stack.append((node.left, depth + 1, path + "0"))

            if node.right is not None:
                right_id = node_id(node.right)
                lines.append(f'  {nid} -> {right_id} [label="1", color=green];')
                stack.append((node.right, depth + 1, path + "1"))

        lines.append("}")
        return "\n".join(lines)

    # ---------- python niceties ----------

    def __and__(self, other): return self.intersection(other)
    def __or__(self, other): return self.union(other)
    def __sub__(self, other): return self.subtract(other)
    def __iter__(self) -> Iterator[tuple[str, T]]: return self.iterator()
    def __len__(self) -> int: return self._size()

    def __contains__(self, key: object) -> bool:
        return isinstance(key, str) and all(ch in "01" for ch in key) and self._contains(key)

    def __repr__(self) -> str:
        items = ", ".join(f"{k!r}: {v!r}" for k, v in self.iterator())
        return f"BitTrieMap({{{items}}})"


def bittriemap(*items: tuple[str, T]) -> BitTrieMap[T]:
    return BitTrieMap.from_items(items)

def bittrieset(*keys: str) -> BitTrieMap[NoneType]:
    return BitTrieMap.from_items((k, None) for k in keys)

@dataclass(frozen=True)
class TrieRef:
    trie: BitTrieMap[type]
    path: str

    def descend_bit(self, bit: Literal['0'] | Literal['1']) -> Optional[TrieRef]:
        if bit == '0' and self.trie.path_exists(self.path + '0'):
            return TrieRef(self.trie, self.path + '0')
        elif bit == '1' and self.trie.path_exists(self.path + '1'):
            return TrieRef(self.trie, self.path + '1')

    def next_sibling(self) -> Optional[TrieRef]:
        if self.path and self.path[-1] == '0' and self.trie.path_exists(self.path[:-1] + '1'):
            return TrieRef(self.trie, self.path[:-1] + '1')

    def ascend_bit(self) -> Optional[TrieRef]:
        if self.path:
            return TrieRef(self.trie, self.path[:-1])

    def descend_first(self) -> Optional[TrieRef]:
        if self.trie.path_exists(self.path + '0'):
            return TrieRef(self.trie, self.path + '0')
        if self.trie.path_exists(self.path + '1'):
            return TrieRef(self.trie, self.path + '1')

    def is_value(self):
        return self.trie.get_node(self.path).has_value


if __name__ == '__main__':
    x, y, z, w, _1, _2, _3 = ('000', '001', '010', '011', '100', '101', '110')
    a = bittrieset(x, _2, _3)
    b = bittrieset(y, z, _1)
    c = bittrieset(z, w, _1, _2)
    d = bittrieset(x, _1)
    r = ((a | b) & c) - d
    print({k for k, v in locals().items() if v in r})
    print(c.to_logical_dot(show_values=False))

    assert a.ref().descend_bit('1').descend_bit('1') is not None
    assert a.ref().descend_bit('1').descend_bit('1').descend_bit('1') is None
    assert a.ref().descend_bit('1').descend_bit('1').ascend_bit() == a.ref().descend_bit('1')
    assert a.ref().descend_bit('1').descend_bit('0').next_sibling() == a.ref().descend_bit('1').descend_bit('1')
    assert a.ref().descend_first().descend_first().descend_first().path == a.first_key()
    assert not a.ref().descend_bit('1').descend_bit('1').is_value()
    assert a.ref().descend_bit('1').descend_bit('1').descend_bit('0').is_value()