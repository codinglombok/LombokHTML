"""LombokHTML: WHATWG HTML tokenizer, a small tree builder, serialization, text
extraction, an allowlist sanitizer, a CSS selector subset, page metadata and
table extraction, with byte-identical results in Python, Rust, TypeScript, Go
and PHP (see docs/SPEC_LombokHTML_v0.2.0.md). No dependencies."""
from __future__ import annotations

import re
from typing import Dict, Iterable, Iterator, List, Optional, Tuple

from ._entities import ENTITIES
from ._tokenizer import ALNUM, MAX_ENTITY, STATES, Token, ascii_lower, numeric_char, tokenize, tokenize_state

__all__ = [
    "DEFAULT_SCHEMES", "MAX_CELLS", "MAX_DEPTH", "STATES", "Node", "PageMeta", "Policy", "Selector",
    "SelectorError", "Token", "decode_entities", "escape_attr", "escape_text", "html_to_text", "is_safe_url",
    "parse", "sanitize", "strip_tags", "tokenize", "tokenize_state",
]
__version__ = "0.2.0"

_ASCII_WS = re.compile("[\t\n\f\r ]+")

# ---------------------------------------------------------------- entities

_DIGITS = "0123456789"
_HEXDIGITS = "0123456789abcdefABCDEF"


def decode_entities(text: str) -> str:
    """Decodes character references as in text content (SPEC section 3.1)."""
    out: List[str] = []
    i, n = 0, len(text)
    while i < n:
        amp = text.find("&", i)
        if amp < 0:
            out.append(text[i:])
            break
        out.append(text[i:amp])
        i = amp
        j = i + 1
        if text.startswith("#", j):
            k = j + 1
            hexa = text[k:k + 1] in ("x", "X")
            if hexa:
                k += 1
            allowed = _HEXDIGITS if hexa else _DIGITS
            start = k
            code = 0
            while k < n and text[k] in allowed:
                code = min(code * (16 if hexa else 10) + int(text[k], 16), 0x110000)
                k += 1
            if k == start:
                out.append(text[i:k])
                i = k
                continue
            if text.startswith(";", k):
                k += 1
            out.append(numeric_char(code))
            i = k
            continue
        k = j
        while k < n and text[k] in ALNUM and k - j < MAX_ENTITY:
            k += 1
        run = text[j:k]
        match = None
        if text.startswith(";", k) and run + ";" in ENTITIES:
            match = run + ";"
        else:
            for m in range(len(run), 0, -1):
                if run[:m] in ENTITIES:
                    match = run[:m]
                    break
        if match is None:
            out.append("&" + run)
            i = k
        else:
            out.append(ENTITIES[match])
            i = j + len(match)
    return "".join(out)


_TEXT_ESC = {ord("&"): "&amp;", 0xA0: "&nbsp;", ord("<"): "&lt;", ord(">"): "&gt;"}
_ATTR_ESC = dict(_TEXT_ESC)
_ATTR_ESC[ord('"')] = "&quot;"


def escape_text(s: str) -> str:
    """Escapes text content: &, U+00A0, <, > (SPEC section 3.2)."""
    return s.translate(_TEXT_ESC)


def escape_attr(s: str) -> str:
    """Escapes an attribute value: &, U+00A0, ", <, > (SPEC section 3.2)."""
    return s.translate(_ATTR_ESC)


# ---------------------------------------------------------------- tree

def _set(s: str) -> frozenset:
    return frozenset(s.split())


VOID = _set("area base basefont bgsound br col embed frame hr img input keygen link meta param source track wbr")
_RAW_PARENTS = _set("style script xmp iframe noembed noframes plaintext noscript")
_SPECIAL = _set(
    "address applet area article aside base basefont bgsound blockquote body br button caption center col colgroup "
    "dd details dir div dl dt embed fieldset figcaption figure footer form frame frameset h1 h2 h3 h4 h5 h6 head "
    "header hgroup hr html iframe img input keygen li link listing main marquee menu meta nav noembed noframes "
    "noscript object ol p param plaintext pre script search section select source style summary table tbody td "
    "template textarea tfoot th thead title tr track ul wbr xmp")
_P_CLOSERS = _set(
    "address article aside blockquote center details dialog dir div dl fieldset figcaption figure footer form h1 h2 "
    "h3 h4 h5 h6 header hgroup hr li dd dt listing main menu nav ol p plaintext pre search section summary table ul "
    "xmp")
_HEADINGS = _set("h1 h2 h3 h4 h5 h6")
_SCOPE = _set("applet caption html table td th marquee object template")
_BUTTON_SCOPE = _SCOPE | {"button"}
_LIST_SCOPE = _SCOPE | {"ol", "ul"}
_TABLE_SCOPE = _set("html table template")
_TABLE_PARTS = _set("table caption tbody thead tfoot tr td th")
_SECTIONS = _set("thead tbody tfoot")
_HIDDEN = _set("script style noscript template title")
_BLOCKS = _set(
    "address article aside blockquote caption details dialog div dl fieldset figcaption figure footer form header "
    "hgroup main nav ol p pre section summary table ul")

MAX_DEPTH = 256
"""Maximum number of open elements; deeper start tags are ignored."""


class Node:
    """A tree node: kind is "document", "element", "text" or "comment"."""

    __slots__ = ("kind", "name", "attrs", "data", "children", "parent")

    def __init__(self, kind: str, name: str = "", attrs: Optional[List[Tuple[str, str]]] = None,
                 data: str = "") -> None:
        self.kind = kind
        self.name = name
        self.attrs: List[Tuple[str, str]] = attrs or []
        self.data = data
        self.children: List["Node"] = []
        self.parent: Optional["Node"] = None

    def __repr__(self) -> str:
        return "Node(%r, %r)" % (self.kind, self.name or self.data)

    def attr(self, name: str) -> Optional[str]:
        """Value of the first attribute called `name`."""
        for k, v in self.attrs:
            if k == name:
                return v
        return None

    def is_element(self, name: str) -> bool:
        return self.kind == "element" and self.name == name

    def elements(self) -> Iterator["Node"]:
        """Elements below this node in document order."""
        todo = list(reversed(self.children))
        while todo:
            n = todo.pop()
            if n.kind == "element":
                yield n
                todo.extend(reversed(n.children))

    def element_siblings(self) -> List["Node"]:
        if self.parent is None:
            return [self]
        return [c for c in self.parent.children if c.kind == "element"]

    def serialize(self) -> str:
        """Outer HTML of an element, inner HTML of the document (SPEC section 5)."""
        out: List[str] = []
        _write(self, out)
        return "".join(out)

    def text_content(self) -> str:
        """Text below this node, skipping script, style, noscript, template and title (SPEC section 6.1)."""
        out: List[str] = []
        todo = [self]
        while todo:
            n = todo.pop()
            if n.kind == "text":
                out.append(n.data)
            elif n.kind == "document" or (n.kind == "element" and n.name not in _HIDDEN):
                todo.extend(reversed(n.children))
        return "".join(out)

    def extract_text(self) -> str:
        """Structure-preserving plain text (SPEC section 6.2)."""
        return _extract_text(self)

    def query(self, selector: str) -> List["Node"]:
        """Elements matching `selector` in document order (SPEC section 8). Raises SelectorError."""
        sel = Selector(selector)
        ctx = _Ctx()
        return [e for e in self.elements() if sel._matches(e, ctx)]

    def meta(self) -> "PageMeta":
        """Page metadata (SPEC section 9)."""
        return _extract_meta(self)

    def tables(self) -> List[List[List[str]]]:
        """Every table as a grid of cell texts (SPEC section 10)."""
        return _extract_tables(self)


class _Builder:
    def __init__(self) -> None:
        self.doc = Node("document")
        self.stack: List[Node] = []

    def current(self) -> Node:
        return self.stack[-1] if self.stack else self.doc

    def append(self, node: Node) -> None:
        p = self.current()
        node.parent = p
        p.children.append(node)

    def in_scope(self, names, boundary) -> bool:
        for node in reversed(self.stack):
            if node.name in names:
                return True
            if node.name in boundary:
                return False
        return False

    def pop_until(self, names) -> None:
        while self.stack:
            if self.stack.pop().name in names:
                return

    def top(self) -> str:
        return self.stack[-1].name if self.stack else ""

    def start(self, name: str, attrs: List[Tuple[str, str]]) -> bool:
        stack = self.stack
        if name in ("li", "dd", "dt"):
            targets = ("li",) if name == "li" else ("dd", "dt")
            for idx in range(len(stack) - 1, -1, -1):
                cur = stack[idx].name
                if cur in targets:
                    del stack[idx:]
                    break
                if cur in _SPECIAL and cur not in ("address", "div", "p"):
                    break
        if name in _P_CLOSERS and self.in_scope(("p",), _BUTTON_SCOPE):
            self.pop_until(("p",))
        if name in _HEADINGS and self.top() in _HEADINGS:
            stack.pop()
        if name in ("option", "optgroup") and self.top() == "option":
            stack.pop()
        if name == "a" and any(n.name == "a" for n in stack):
            self.pop_until(("a",))
        if name in ("td", "th", "tr", "thead", "tbody", "tfoot") and self.in_scope(("td", "th"), _TABLE_SCOPE):
            self.pop_until(("td", "th"))
        if name in ("tr", "thead", "tbody", "tfoot") and self.in_scope(("tr",), _TABLE_SCOPE):
            self.pop_until(("tr",))
        if name in _SECTIONS and self.in_scope(_SECTIONS, _TABLE_SCOPE):
            self.pop_until(_SECTIONS)
        if len(stack) >= MAX_DEPTH:
            return False
        el = Node("element", name, attrs)
        self.append(el)
        if name not in VOID:
            stack.append(el)
        return name in ("pre", "listing", "textarea")

    def end(self, name: str) -> None:
        if name == "br":
            self.start("br", [])
            return
        if name == "p":
            target, boundary = ("p",), _BUTTON_SCOPE
        elif name in _HEADINGS:
            target, boundary = _HEADINGS, _SCOPE
        elif name == "li":
            target, boundary = ("li",), _LIST_SCOPE
        elif name in _TABLE_PARTS:
            target, boundary = (name,), _TABLE_SCOPE
        elif name in ("dd", "dt") or name in _SPECIAL:
            target, boundary = (name,), _SCOPE
        else:
            for idx in range(len(self.stack) - 1, -1, -1):
                cur = self.stack[idx].name
                if cur == name:
                    del self.stack[idx:]
                    return
                if cur in _SPECIAL:
                    return
            return
        if self.in_scope(target, boundary):
            self.pop_until(target)


def parse(html: str) -> Node:
    """Parses `html` into a document node (SPEC section 4). Never raises."""
    b = _Builder()
    skip_newline = False
    for tok in tokenize(html):
        kind = tok[0]
        if kind == "Character":
            data = tok[1]
            if skip_newline and data.startswith("\n"):  # type: ignore[union-attr]
                data = data[1:]  # type: ignore[index]
            skip_newline = False
            if data:
                p = b.current()
                if p.children and p.children[-1].kind == "text":
                    p.children[-1].data += data  # type: ignore[operator]
                else:
                    b.append(Node("text", data=data))  # type: ignore[arg-type]
            continue
        skip_newline = False
        if kind == "StartTag":
            skip_newline = b.start(tok[1], list(tok[2]))  # type: ignore[arg-type]
        elif kind == "EndTag":
            b.end(tok[1])  # type: ignore[arg-type]
        elif kind == "Comment":
            b.append(Node("comment", data=tok[1]))  # type: ignore[arg-type]
    return b.doc


def _write(n: Node, out: List[str]) -> None:
    if n.kind == "text":
        p = n.parent
        raw = p is not None and p.kind == "element" and p.name in _RAW_PARENTS
        out.append(n.data if raw else escape_text(n.data))
    elif n.kind == "comment":
        out.append("<!--" + n.data + "-->")
    elif n.kind == "element":
        out.append("<" + n.name)
        for k, v in n.attrs:
            out.append(" " + k + '="' + escape_attr(v) + '"')
        out.append(">")
        if n.name in VOID:
            return
        for c in n.children:
            _write(c, out)
        out.append("</" + n.name + ">")
    else:
        for c in n.children:
            _write(c, out)


# ---------------------------------------------------------------- text

def _extract_text(node: Node) -> str:
    out: List[str] = []

    def walk(n: Node, pre: bool) -> None:
        if n.kind == "text":
            if pre:
                out.append(n.data)
                return
            collapsed = _ASCII_WS.sub(" ", n.data)
            if collapsed.startswith(" ") and (not out or out[-1][-1:] in ("", " ", "\n", "\t")):
                collapsed = collapsed[1:]
            if collapsed:
                out.append(collapsed)
            return
        if n.kind == "comment":
            return
        if n.kind == "document":
            for c in n.children:
                walk(c, pre)
            return
        name = n.name
        if name in _HIDDEN:
            return
        if name in _HEADINGS:
            out.append("\n\n" + "#" * int(name[1]) + " ")
            for c in n.children:
                walk(c, pre)
            out.append("\n\n")
            return
        if name == "li":
            out.append("\n- ")
        elif name in ("dd", "dt", "tr"):
            out.append("\n")
        elif name == "br":
            out.append("\n")
            return
        elif name == "hr":
            out.append("\n\n---\n\n")
            return
        block = name in _BLOCKS
        if block:
            out.append("\n\n")
        inner = pre or name in ("pre", "listing", "textarea")
        for c in n.children:
            walk(c, inner)
        if name in ("td", "th"):
            out.append("\t")
        if block:
            out.append("\n\n")

    walk(node, False)
    text = "\n".join(line.rstrip(" \t") for line in "".join(out).split("\n"))
    return re.sub("\n{3,}", "\n\n", text).strip("\n")


def html_to_text(html: str) -> str:
    """Parses `html` and returns its structure-preserving plain text."""
    return parse(html).extract_text()


def strip_tags(html: str) -> str:
    """Parses `html` and returns its text content without markup."""
    return parse(html).text_content()


# ---------------------------------------------------------------- sanitizer

_DEFAULT_ALLOWED = _set(
    "a abbr b blockquote br caption cite code dd del dfn div dl dt em figcaption figure h1 h2 h3 h4 h5 h6 hr i img "
    "ins kbd li mark ol p pre q s samp small span strong sub sup table tbody td tfoot th thead time tr u ul")
_DEFAULT_DROP = _set(
    "applet audio base button canvas embed frame frameset head iframe link math meta noembed noframes noscript "
    "object option plaintext script select style svg template textarea title video xmp")
_DEFAULT_ATTRS = frozenset(tuple(x.split(":")) for x in (
    "*:dir *:lang *:title a:href blockquote:cite img:alt img:height img:src img:width ol:start q:cite td:colspan "
    "td:rowspan th:colspan th:rowspan th:scope time:datetime").split())
_URL_ATTRS = _set("cite href src")
DEFAULT_SCHEMES: Tuple[str, ...] = ("http", "https", "mailto", "tel")
"""URL schemes allowed by default."""


class Policy:
    """Sanitizer policy: the defaults plus extra tags, attributes and schemes."""

    def __init__(self) -> None:
        self.tags = set(_DEFAULT_ALLOWED)
        self.drop = set(_DEFAULT_DROP)
        self.attrs = set(_DEFAULT_ATTRS)
        self.schemes = set(DEFAULT_SCHEMES)

    def allow_tag(self, tag: str) -> "Policy":
        """Keeps `tag` (ASCII case-insensitive), also when it is on the drop list."""
        t = ascii_lower(tag)
        if t == "plaintext":  # cannot be closed again, so never kept
            return self
        self.tags.add(t)
        self.drop.discard(t)
        return self

    def allow_attr(self, tag: str, attr: str) -> "Policy":
        """Keeps attribute `attr` on `tag`; tag "*" means every kept element."""
        self.attrs.add((ascii_lower(tag), ascii_lower(attr)))
        return self

    def allow_scheme(self, scheme: str) -> "Policy":
        """Accepts URLs with `scheme` in href, src and cite."""
        self.schemes.add(ascii_lower(scheme))
        return self


_URL_STRIP = {c: None for c in list(range(0x21)) + [0x7F]}


def is_safe_url(url: str, schemes: Iterable[str] = DEFAULT_SCHEMES) -> bool:
    """True when `url` has no scheme or one of `schemes` (lowercase), after removing
    C0 controls, space and DEL (SPEC section 7.3)."""
    cleaned = ascii_lower(url.translate(_URL_STRIP))
    m = re.search("[/?#:]", cleaned)
    if m is None or m.group(0) != ":":
        return True
    return cleaned[:m.start()] in set(schemes)


def sanitize(html: str, policy: Optional[Policy] = None) -> str:
    """Sanitizes `html` (SPEC section 7). The result is a fixed point."""
    p = policy or Policy()
    out: List[str] = []

    def walk(n: Node) -> None:
        if n.kind == "text":
            out.append(escape_text(n.data))
            return
        if n.kind == "comment":
            return
        name = n.name
        if n.kind == "element":
            if name in p.drop:
                return
            if name in p.tags:
                out.append("<" + name)
                for k, v in n.attrs:
                    if (name, k) not in p.attrs and ("*", k) not in p.attrs:
                        continue
                    if k in _URL_ATTRS and not is_safe_url(v, p.schemes):
                        continue
                    out.append(" " + k + '="' + escape_attr(v) + '"')
                out.append(">")
                if name in VOID:
                    return
                if name not in _RAW_PARENTS:  # raw text content would not survive a second pass
                    for c in n.children:
                        walk(c)
                out.append("</" + name + ">")
                return
        for c in n.children:
            walk(c)

    walk(parse(html))
    return "".join(out)


# ---------------------------------------------------------------- selectors

class SelectorError(ValueError):
    """A selector that does not follow the SPEC grammar; `offset` counts characters."""

    code = "BAD_SELECTOR"

    def __init__(self, offset: int) -> None:
        super().__init__("BAD_SELECTOR at character %d" % offset)
        self.offset = offset


_IDENT = re.compile(r"(?:[A-Za-z0-9_\-]|[^\x00-\x7f])+")
_SEL_WS = re.compile(r"[ \t\n\r\f]*")
_NTH = re.compile(r"([+-]?[0-9]{0,9})n(?:[\t\n\f\r ]*([+-])[\t\n\f\r ]*([0-9]{1,9}))?")
_INT = re.compile(r"[+-]?[0-9]{1,9}")


def _parse_nth(text: str) -> Optional[Tuple[int, int]]:
    t = ascii_lower(text.strip("\t\n\f\r "))
    if t == "odd":
        return (2, 1)
    if t == "even":
        return (2, 0)
    if _INT.fullmatch(t):
        return (0, int(t))
    m = _NTH.fullmatch(t)
    if m is None:
        return None
    sa = m.group(1)
    a = 1 if sa in ("", "+") else -1 if sa == "-" else int(sa)
    b = int(m.group(3)) * (-1 if m.group(2) == "-" else 1) if m.group(3) else 0
    return (a, b)


class _SelParser:
    def __init__(self, s: str) -> None:
        self.s = s
        self.i = 0

    def fail(self):
        raise SelectorError(self.i)

    def ws(self) -> bool:
        end = _SEL_WS.match(self.s, self.i).end()  # type: ignore[union-attr]
        moved = end > self.i
        self.i = end
        return moved

    def ident(self) -> str:
        m = _IDENT.match(self.s, self.i)
        if m is None:
            self.fail()
        self.i = m.end()  # type: ignore[union-attr]
        return m.group(0)  # type: ignore[union-attr]

    def at(self, k: int = 0) -> str:
        return self.s[self.i + k:self.i + k + 1]

    def parse_list(self) -> list:
        out = []
        while True:
            self.ws()
            out.append(self.complex())
            self.ws()
            if self.i >= len(self.s):
                return out
            if self.at() != ",":
                self.fail()
            self.i += 1

    def complex(self) -> list:
        parts = [("", self.compound())]
        while True:
            save = self.i
            had_ws = self.ws()
            c = self.at()
            if c in ("", ","):
                self.i = save
                return parts
            if c in (">", "+", "~"):
                self.i += 1
                self.ws()
                parts.append((c, self.compound()))
            elif had_ws:
                parts.append((" ", self.compound()))
            else:
                self.fail()

    def compound(self) -> tuple:
        tag = None
        simple: list = []
        if self.at() == "*":
            self.i += 1
            tag = "*"
        elif _IDENT.match(self.s, self.i):
            tag = ascii_lower(self.ident())
        while True:
            c = self.at()
            if c in ("#", "."):
                self.i += 1
                simple.append(("id" if c == "#" else "class", self.ident()))
            elif c == "[":
                self.i += 1
                self.ws()
                name = ascii_lower(self.ident())
                self.ws()
                op = None
                val = ""
                if self.at() == "=":
                    op = "="
                    self.i += 1
                elif self.at() in ("~", "|", "^", "$", "*") and self.at(1) == "=":
                    op = self.at() + "="
                    self.i += 2
                if op is not None:
                    self.ws()
                    q = self.at()
                    if q in ("'", '"'):
                        end = self.s.find(q, self.i + 1)
                        if end < 0:
                            self.fail()
                        val = self.s[self.i + 1:end]
                        self.i = end + 1
                    else:
                        val = self.ident()
                    self.ws()
                if self.at() != "]":
                    self.fail()
                self.i += 1
                simple.append(("attr", name, op, val))
            elif c == ":":
                self.i += 1
                name = ascii_lower(self.ident())
                if name in ("first-child", "last-child", "only-child", "empty"):
                    simple.append((name,))
                elif name in ("nth-child", "nth-last-child", "not"):
                    if self.at() != "(":
                        self.fail()
                    self.i += 1
                    self.ws()
                    if name == "not":
                        item: tuple = ("not", self.compound())
                        self.ws()
                    else:
                        end = self.s.find(")", self.i)
                        if end < 0:
                            self.fail()
                        ab = _parse_nth(self.s[self.i:end])
                        if ab is None:
                            self.fail()
                        self.i = end
                        item = (name, ab[0], ab[1])  # type: ignore[index]
                    if self.at() != ")":
                        self.fail()
                    self.i += 1
                    simple.append(item)
                else:
                    self.fail()
            else:
                break
        if tag is None and not simple:
            self.fail()
        return (tag, simple)


def _nth_ok(a: int, b: int, pos: int) -> bool:
    if a == 0:
        return pos == b
    if a > 0:
        return pos >= b and (pos - b) % a == 0
    return pos <= b and (b - pos) % (-a) == 0


def _split_ws(s: str) -> List[str]:
    return [x for x in _ASCII_WS.split(s) if x]


class _Ctx:
    """Per-query caches: sibling lists and positions, match results per (node, step),
    and the first sibling matching a step (for "~"). They keep matching linear in
    the number of siblings and in the tree depth; results are unchanged."""

    __slots__ = ("sibs", "memo", "first")

    def __init__(self) -> None:
        self.sibs: Dict[int, Tuple[List[Node], Dict[int, int]]] = {}
        self.memo: Dict[Tuple[int, int, int], bool] = {}
        self.first: Dict[Tuple[int, int, int], int] = {}

    def siblings(self, el: Node) -> Tuple[List[Node], int]:
        parent = el.parent
        if parent is None:
            return [el], 0
        entry = self.sibs.get(id(parent))
        if entry is None:
            sib = [c for c in parent.children if c.kind == "element"]
            entry = (sib, {id(x): i for i, x in enumerate(sib)})
            self.sibs[id(parent)] = entry
        return entry[0], entry[1][id(el)]


def _match_compound(el: Node, comp: tuple, ctx: _Ctx) -> bool:
    tag, simple = comp
    if tag is not None and tag != "*" and el.name != tag:
        return False
    for s in simple:
        k = s[0]
        if k == "id":
            ok = el.attr("id") == s[1]
        elif k == "class":
            ok = s[1] in _split_ws(el.attr("class") or "")
        elif k == "attr":
            _, name, op, val = s
            v = el.attr(name)
            if v is None:
                ok = False
            elif op is None:
                ok = True
            elif op == "=":
                ok = v == val
            elif op == "~=":
                ok = val in _split_ws(v)
            elif op == "|=":
                ok = v == val or v.startswith(val + "-")
            elif op == "^=":
                ok = val != "" and v.startswith(val)
            elif op == "$=":
                ok = val != "" and v.endswith(val)
            else:
                ok = val != "" and val in v
        elif k == "empty":
            ok = not any(c.kind in ("element", "text") for c in el.children)
        elif k == "not":
            ok = not _match_compound(el, s[1], ctx)
        else:
            sib, idx = ctx.siblings(el)
            pos = idx + 1
            if k == "first-child":
                ok = pos == 1
            elif k == "last-child":
                ok = pos == len(sib)
            elif k == "only-child":
                ok = len(sib) == 1
            elif k == "nth-child":
                ok = _nth_ok(s[1], s[2], pos)
            else:
                ok = _nth_ok(s[1], s[2], len(sib) - pos + 1)
        if not ok:
            return False
    return True


def _match_complex(el: Node, parts: list, k: int, ctx: _Ctx, which: int) -> bool:
    key = (id(el), which, k)
    hit = ctx.memo.get(key)
    if hit is None:
        hit = _match_step(el, parts, k, ctx, which)
        ctx.memo[key] = hit
    return hit


def _match_step(el: Node, parts: list, k: int, ctx: _Ctx, which: int) -> bool:
    comb, comp = parts[k]
    if not _match_compound(el, comp, ctx):
        return False
    if k == 0:
        return True
    if comb == ">":
        p = el.parent
        return p is not None and p.kind == "element" and _match_complex(p, parts, k - 1, ctx, which)
    if comb == " ":
        p = el.parent
        while p is not None and p.kind == "element":
            if _match_complex(p, parts, k - 1, ctx, which):
                return True
            p = p.parent
        return False
    sib, idx = ctx.siblings(el)
    if comb == "+":
        return idx > 0 and _match_complex(sib[idx - 1], parts, k - 1, ctx, which)
    # "~": some earlier sibling matches step k-1; remember the first such sibling per parent.
    fkey = (id(el.parent), which, k)
    first = ctx.first.get(fkey)
    if first is None:
        first = len(sib)
        for i, x in enumerate(sib):
            if _match_complex(x, parts, k - 1, ctx, which):
                first = i
                break
        ctx.first[fkey] = first
    return first < idx


class Selector:
    """A parsed selector list (SPEC section 8). Raises SelectorError."""

    def __init__(self, selector: str) -> None:
        self._list = _SelParser(selector).parse_list()

    def matches(self, el: Node) -> bool:
        return self._matches(el, _Ctx())

    def _matches(self, el: Node, ctx: _Ctx) -> bool:
        return el.kind == "element" and any(
            _match_complex(el, parts, len(parts) - 1, ctx, i) for i, parts in enumerate(self._list))


# ---------------------------------------------------------------- meta and tables

class PageMeta:
    """Metadata found in a document (SPEC section 9)."""

    __slots__ = ("title", "description", "canonical", "lang", "og")

    def __init__(self) -> None:
        self.title: Optional[str] = None
        self.description: Optional[str] = None
        self.canonical: Optional[str] = None
        self.lang: Optional[str] = None
        self.og: List[Tuple[str, str]] = []

    def to_dict(self) -> Dict[str, object]:
        return {"title": self.title, "description": self.description, "canonical": self.canonical,
                "lang": self.lang, "og": [list(x) for x in self.og]}


def _collapse(s: str) -> str:
    return _ASCII_WS.sub(" ", s).strip(" ")


def _extract_meta(doc: Node) -> PageMeta:
    out = PageMeta()
    seen_title = False
    for el in doc.elements():
        name = el.name
        if name == "title" and not seen_title:
            seen_title = True
            out.title = _collapse("".join(c.data for c in el.children if c.kind == "text")) or None
        elif name == "html":
            if out.lang is None:
                out.lang = el.attr("lang")
        elif name == "meta":
            content = el.attr("content")
            if content is None:
                continue
            if ascii_lower(el.attr("name") or "") == "description" and out.description is None:
                out.description = content
            prop = el.attr("property") or ""
            if ascii_lower(prop[:3]) == "og:":
                out.og.append((prop[3:], content))
        elif name == "link" and out.canonical is None:
            if "canonical" in _split_ws(ascii_lower(el.attr("rel") or "")):
                out.canonical = el.attr("href")
    return out


_SPAN = re.compile("[\t\n\f\r ]*\\+?([0-9]+)")


def _parse_span(v: Optional[str], default: int, maximum: int) -> int:
    if v is None:
        return default
    m = _SPAN.match(v)
    if m is None:
        return default
    digits = m.group(1)
    n = int(digits) if len(digits) <= 9 else maximum
    return default if n == 0 else min(n, maximum)


MAX_CELLS = 1000000
"""Most cells produced for one table; extraction stops there."""


def _rows(n: Node, rows: List[Node]) -> None:
    for c in n.children:
        if c.kind != "element" or c.name == "table":
            continue
        if c.name == "tr":
            rows.append(c)
        else:
            _rows(c, rows)


def _extract_tables(doc: Node) -> List[List[List[str]]]:
    tables = []
    for table in doc.elements():
        if table.name != "table":
            continue
        rows: List[Node] = []
        _rows(table, rows)
        grid: List[List[str]] = []
        pending: Dict[int, Tuple[int, str]] = {}
        total = 0
        stop = False
        for tr in rows:
            row = {col: text for col, (_, text) in pending.items()}
            nxt = {col: (left - 1, text) for col, (left, text) in pending.items() if left > 1}
            col = 0
            for cell in tr.children:
                if cell.kind != "element" or cell.name not in ("td", "th"):
                    continue
                text = _collapse(cell.text_content())
                cs = _parse_span(cell.attr("colspan"), 1, 1000)
                rs = _parse_span(cell.attr("rowspan"), 1, 65534)
                for _ in range(cs):
                    while col in row:
                        col += 1
                    total += 1
                    if total > MAX_CELLS:
                        stop = True
                        break
                    row[col] = text
                    if rs > 1:
                        nxt[col] = (rs - 1, text)
                    col += 1
                if stop:
                    break
            pending = nxt
            width = max(row) + 1 if row else 0
            grid.append([row.get(c, "") for c in range(width)])
            if stop:
                break
        tables.append(grid)
    return tables
