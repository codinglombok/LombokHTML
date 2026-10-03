"""WHATWG HTML tokenizer (SPEC section 2), for HTML content (no foreign content)."""
from __future__ import annotations

from typing import List, Optional, Tuple, Union

from ._entities import ENTITIES

MAX_ENTITY = 32
_R = "�"
_WS = frozenset("\t\n\f ")
_ALPHA = frozenset("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
_DIGIT = frozenset("0123456789")
_HEX = frozenset("0123456789abcdefABCDEF")
ALNUM = _ALPHA | _DIGIT
_LOWER = {c: c + 32 for c in range(65, 91)}
_RAWTEXT = frozenset(("style", "xmp", "iframe", "noembed", "noframes", "noscript"))
_REPLACEMENTS = {
    0x80: 0x20AC, 0x82: 0x201A, 0x83: 0x0192, 0x84: 0x201E, 0x85: 0x2026, 0x86: 0x2020, 0x87: 0x2021,
    0x88: 0x02C6, 0x89: 0x2030, 0x8A: 0x0160, 0x8B: 0x2039, 0x8C: 0x0152, 0x8E: 0x017D, 0x91: 0x2018,
    0x92: 0x2019, 0x93: 0x201C, 0x94: 0x201D, 0x95: 0x2022, 0x96: 0x2013, 0x97: 0x2014, 0x98: 0x02DC,
    0x99: 0x2122, 0x9A: 0x0161, 0x9B: 0x203A, 0x9C: 0x0153, 0x9E: 0x017E, 0x9F: 0x0178,
}

# Token tuples: ("StartTag", name, [(k, v), ...], self_closing), ("EndTag", name),
# ("Character", data), ("Comment", data), ("DOCTYPE", name, public_id, system_id, correct).
Token = Tuple[Union[str, bool, None, List[Tuple[str, str]]], ...]


def ascii_lower(s: str) -> str:
    return s.translate(_LOWER)


def numeric_char(code: int) -> str:
    if code == 0 or code > 0x10FFFF or 0xD800 <= code <= 0xDFFF:
        return _R
    return chr(_REPLACEMENTS.get(code, code))


class _Tokenizer:
    def __init__(self, text: str, state: str, last_start: Optional[str], switching: bool) -> None:
        self.s = text.replace("\r\n", "\n").replace("\r", "\n")
        self.i = 0
        self.state = state
        self.ret = "data"
        self.last_start = last_start
        self.switching = switching
        self.out: List[list] = []
        self.tag: Optional[list] = None  # [is_end, name, attrs, self_closing]
        self.attr: list = [[], []]
        self.temp: List[str] = []
        self.comment: List[str] = []
        self.chars: List[str] = []  # pending Character data, flushed before any other token
        self.doctype: list = [None, None, None, False]  # name, public, system, quirks
        self.code = 0

    # helpers
    def emit(self, s: str) -> None:
        self.chars.append(s)

    def flush_chars(self) -> None:
        if self.chars:
            self.out.append(["Character", "".join(self.chars)])
            self.chars = []

    def push(self, tok: list) -> None:
        self.flush_chars()
        self.out.append(tok)

    def emit_tag(self) -> None:
        end, parts, attrs, sc = self.tag  # type: ignore[misc]
        name = "".join(parts)
        if end:
            self.push(["EndTag", name])
        else:
            seen = set()
            kept = []
            for kp, vp in attrs:
                k, v = "".join(kp), "".join(vp)
                if k not in seen:
                    seen.add(k)
                    kept.append((k, v))
            self.push(["StartTag", name, kept, sc])
            self.last_start = name
            if self.switching:
                if name in ("title", "textarea"):
                    self.state = "rcdata"
                elif name in _RAWTEXT:
                    self.state = "rawtext"
                elif name == "script":
                    self.state = "script"
                elif name == "plaintext":
                    self.state = "plaintext"
        self.tag = None

    def appropriate(self) -> bool:
        return self.tag is not None and self.tag[0] and "".join(self.tag[1]) == self.last_start

    def start_attr(self) -> None:
        self.attr = [[], []]
        self.tag[2].append(self.attr)  # type: ignore[index]

    def in_attr(self) -> bool:
        return self.ret in ("attr_dq", "attr_sq", "attr_unq")

    def flush_ref(self) -> None:
        if self.in_attr():
            self.attr[1].extend(self.temp)
        else:
            self.chars.extend(self.temp)

    def reconsume(self, state: str) -> None:
        self.i -= 1
        self.state = state

    def emit_comment(self) -> None:
        self.push(["Comment", "".join(self.comment)])

    def emit_doctype(self, quirks: bool = False) -> None:
        if quirks:
            self.doctype[3] = True
        d = [None if x is None else "".join(x) for x in self.doctype[:3]]
        self.push(["DOCTYPE", d[0], d[1], d[2], not self.doctype[3]])

    def lt(self, c, base, open_state):
        if c == "/":
            self.temp = []
            self.state = open_state
        else:
            self.emit("<")
            self.reconsume(base)

    def end_open(self, c, base, name_state):
        if c in _ALPHA:
            self.tag = [True, [], [], False]
            self.reconsume(name_state)
        else:
            self.emit("</")
            self.reconsume(base)

    def end_name(self, c, base):
        if c in _WS and self.appropriate():
            self.state = "before_attr_name"
        elif c == "/" and self.appropriate():
            self.state = "self_closing"
        elif c == ">" and self.appropriate():
            self.state = "data"
            self.emit_tag()
        elif c in _ALPHA:
            self.tag[1].append(ascii_lower(c))  # type: ignore[index]
            self.temp.append(c)
        else:
            self.emit("</")
            self.chars.extend(self.temp)
            self.tag = None
            self.reconsume(base)

    def run(self) -> List[list]:
        s = self.s
        n = len(s)
        while True:
            c = s[self.i] if self.i < n else ""
            self.i += 1
            if not self.step(c):
                self.flush_chars()
                return self.out

    def step(self, c: str) -> bool:  # noqa: C901 - one branch per WHATWG state
        st = self.state
        eof = c == ""
        if st == "data":
            if c == "&":
                self.ret = "data"
                self.state = "charref"
            elif c == "<":
                self.state = "tag_open"
            elif eof:
                return False
            else:
                self.emit(c)
        elif st == "rcdata":
            if c == "&":
                self.ret = "rcdata"
                self.state = "charref"
            elif c == "<":
                self.state = "rcdata_lt"
            elif eof:
                return False
            else:
                self.emit(_R if c == "\0" else c)
        elif st in ("rawtext", "script"):
            if c == "<":
                self.state = st + "_lt"
            elif eof:
                return False
            else:
                self.emit(_R if c == "\0" else c)
        elif st == "plaintext":
            if eof:
                return False
            self.emit(_R if c == "\0" else c)
        elif st == "tag_open":
            if c == "!":
                self.state = "markup_decl"
            elif c == "/":
                self.state = "end_tag_open"
            elif c in _ALPHA:
                self.tag = [False, [], [], False]
                self.reconsume("tag_name")
            elif c == "?":
                self.comment = []
                self.reconsume("bogus_comment")
            elif eof:
                self.emit("<")
                return False
            else:
                self.emit("<")
                self.reconsume("data")
        elif st == "end_tag_open":
            if c in _ALPHA:
                self.tag = [True, [], [], False]
                self.reconsume("tag_name")
            elif c == ">":
                self.state = "data"
            elif eof:
                self.emit("</")
                return False
            else:
                self.comment = []
                self.reconsume("bogus_comment")
        elif st == "tag_name":
            if c in _WS:
                self.state = "before_attr_name"
            elif c == "/":
                self.state = "self_closing"
            elif c == ">":
                self.state = "data"
                self.emit_tag()
            elif eof:
                return False
            else:
                self.tag[1].append(_R if c == "\0" else ascii_lower(c))  # type: ignore[index]
        elif st == "rcdata_lt":
            self.lt(c, "rcdata", "rcdata_end_open")
        elif st == "rcdata_end_open":
            self.end_open(c, "rcdata", "rcdata_end_name")
        elif st == "rcdata_end_name":
            self.end_name(c, "rcdata")
        elif st == "rawtext_lt":
            self.lt(c, "rawtext", "rawtext_end_open")
        elif st == "rawtext_end_open":
            self.end_open(c, "rawtext", "rawtext_end_name")
        elif st == "rawtext_end_name":
            self.end_name(c, "rawtext")
        elif st == "script_lt":
            if c == "/":
                self.temp = []
                self.state = "script_end_open"
            elif c == "!":
                self.state = "script_esc_start"
                self.emit("<!")
            else:
                self.emit("<")
                self.reconsume("script")
        elif st == "script_end_open":
            self.end_open(c, "script", "script_end_name")
        elif st == "script_end_name":
            self.end_name(c, "script")
        elif st in ("script_esc_start", "script_esc_start_dash"):
            if c == "-":
                self.state = "script_esc_start_dash" if st == "script_esc_start" else "script_esc_dash_dash"
                self.emit("-")
            else:
                self.reconsume("script")
        elif st in ("script_esc", "script_esc_dash", "script_esc_dash_dash"):
            if c == "-":
                self.state = "script_esc_dash" if st == "script_esc" else "script_esc_dash_dash"
                self.emit("-")
            elif c == "<":
                self.state = "script_esc_lt"
            elif c == ">" and st == "script_esc_dash_dash":
                self.state = "script"
                self.emit(">")
            elif eof:
                return False
            else:
                self.state = "script_esc"
                self.emit(_R if c == "\0" else c)
        elif st == "script_esc_lt":
            if c == "/":
                self.temp = []
                self.state = "script_esc_end_open"
            elif c in _ALPHA:
                self.temp = []
                self.emit("<")
                self.reconsume("script_dbl_esc_start")
            else:
                self.emit("<")
                self.reconsume("script_esc")
        elif st == "script_esc_end_open":
            self.end_open(c, "script_esc", "script_esc_end_name")
        elif st == "script_esc_end_name":
            self.end_name(c, "script_esc")
        elif st in ("script_dbl_esc_start", "script_dbl_esc_end"):
            start = st == "script_dbl_esc_start"
            if c in _WS or c == "/" or c == ">":
                self.state = "script_dbl_esc" if ("".join(self.temp) == "script") == start else "script_esc"
                self.emit(c)
            elif c in _ALPHA:
                self.temp.append(ascii_lower(c))
                self.emit(c)
            else:
                self.reconsume("script_esc" if start else "script_dbl_esc")
        elif st in ("script_dbl_esc", "script_dbl_esc_dash", "script_dbl_esc_dash_dash"):
            if c == "-":
                self.state = "script_dbl_esc_dash" if st == "script_dbl_esc" else "script_dbl_esc_dash_dash"
                self.emit("-")
            elif c == "<":
                self.state = "script_dbl_esc_lt"
                self.emit("<")
            elif c == ">" and st == "script_dbl_esc_dash_dash":
                self.state = "script"
                self.emit(">")
            elif eof:
                return False
            else:
                self.state = "script_dbl_esc"
                self.emit(_R if c == "\0" else c)
        elif st == "script_dbl_esc_lt":
            if c == "/":
                self.temp = []
                self.state = "script_dbl_esc_end"
                self.emit("/")
            else:
                self.reconsume("script_dbl_esc")
        elif st == "before_attr_name":
            if c in _WS:
                pass
            elif eof or c == "/" or c == ">":
                self.reconsume("after_attr_name")
            else:
                self.start_attr()
                if c == "=":
                    self.attr[0].append(c)
                    self.state = "attr_name"
                else:
                    self.reconsume("attr_name")
        elif st == "attr_name":
            if eof or c in _WS or c == "/" or c == ">":
                self.reconsume("after_attr_name")
            elif c == "=":
                self.state = "before_attr_value"
            else:
                self.attr[0].append(_R if c == "\0" else ascii_lower(c))
        elif st == "after_attr_name":
            if c in _WS:
                pass
            elif c == "/":
                self.state = "self_closing"
            elif c == "=":
                self.state = "before_attr_value"
            elif c == ">":
                self.state = "data"
                self.emit_tag()
            elif eof:
                return False
            else:
                self.start_attr()
                self.reconsume("attr_name")
        elif st == "before_attr_value":
            if c in _WS:
                pass
            elif c == '"':
                self.state = "attr_dq"
            elif c == "'":
                self.state = "attr_sq"
            elif c == ">":
                self.state = "data"
                self.emit_tag()
            else:
                self.reconsume("attr_unq")
        elif st in ("attr_dq", "attr_sq"):
            if c == ('"' if st == "attr_dq" else "'"):
                self.state = "after_attr_value_q"
            elif c == "&":
                self.ret = st
                self.state = "charref"
            elif eof:
                return False
            else:
                self.attr[1].append(_R if c == "\0" else c)
        elif st == "attr_unq":
            if c in _WS:
                self.state = "before_attr_name"
            elif c == "&":
                self.ret = "attr_unq"
                self.state = "charref"
            elif c == ">":
                self.state = "data"
                self.emit_tag()
            elif eof:
                return False
            else:
                self.attr[1].append(_R if c == "\0" else c)
        elif st == "after_attr_value_q":
            if c in _WS:
                self.state = "before_attr_name"
            elif c == "/":
                self.state = "self_closing"
            elif c == ">":
                self.state = "data"
                self.emit_tag()
            elif eof:
                return False
            else:
                self.reconsume("before_attr_name")
        elif st == "self_closing":
            if c == ">":
                self.tag[3] = True  # type: ignore[index]
                self.state = "data"
                self.emit_tag()
            elif eof:
                return False
            else:
                self.reconsume("before_attr_name")
        elif st == "bogus_comment":
            if c == ">":
                self.state = "data"
                self.emit_comment()
            elif eof:
                self.emit_comment()
                return False
            else:
                self.comment.append(_R if c == "\0" else c)
        elif st == "markup_decl":
            self.i -= 1
            self.comment = []
            if self.s.startswith("--", self.i):
                self.i += 2
                self.state = "comment_start"
            elif ascii_lower(self.s[self.i:self.i + 7]) == "doctype":
                self.i += 7
                self.state = "doctype"
            else:
                self.state = "bogus_comment"
        elif st == "comment_start":
            if c == "-":
                self.state = "comment_start_dash"
            elif c == ">":
                self.state = "data"
                self.emit_comment()
            else:
                self.reconsume("comment")
        elif st == "comment_start_dash":
            if c == "-":
                self.state = "comment_end"
            elif c == ">":
                self.state = "data"
                self.emit_comment()
            elif eof:
                self.emit_comment()
                return False
            else:
                self.comment.append("-")
                self.reconsume("comment")
        elif st == "comment":
            if c == "<":
                self.comment.append(c)
                self.state = "comment_lt"
            elif c == "-":
                self.state = "comment_end_dash"
            elif eof:
                self.emit_comment()
                return False
            else:
                self.comment.append(_R if c == "\0" else c)
        elif st == "comment_lt":
            if c == "!":
                self.comment.append(c)
                self.state = "comment_lt_bang"
            elif c == "<":
                self.comment.append(c)
            else:
                self.reconsume("comment")
        elif st == "comment_lt_bang":
            if c == "-":
                self.state = "comment_lt_bang_dash"
            else:
                self.reconsume("comment")
        elif st == "comment_lt_bang_dash":
            if c == "-":
                self.state = "comment_lt_bang_dash_dash"
            else:
                self.reconsume("comment_end_dash")
        elif st == "comment_lt_bang_dash_dash":
            self.reconsume("comment_end")
        elif st == "comment_end_dash":
            if c == "-":
                self.state = "comment_end"
            elif eof:
                self.emit_comment()
                return False
            else:
                self.comment.append("-")
                self.reconsume("comment")
        elif st == "comment_end":
            if c == ">":
                self.state = "data"
                self.emit_comment()
            elif c == "!":
                self.state = "comment_end_bang"
            elif c == "-":
                self.comment.append("-")
            elif eof:
                self.emit_comment()
                return False
            else:
                self.comment.append("--")
                self.reconsume("comment")
        elif st == "comment_end_bang":
            if c == "-":
                self.comment.append("--!")
                self.state = "comment_end_dash"
            elif c == ">":
                self.state = "data"
                self.emit_comment()
            elif eof:
                self.emit_comment()
                return False
            else:
                self.comment.append("--!")
                self.reconsume("comment")
        elif st == "doctype":
            if c in _WS:
                self.state = "before_doctype_name"
            elif eof:
                self.doctype = [None, None, None, False]
                self.emit_doctype(True)
                return False
            else:
                self.reconsume("before_doctype_name")
        elif st == "before_doctype_name":
            if c in _WS:
                return True
            self.doctype = [None, None, None, False]
            if c == ">":
                self.state = "data"
                self.emit_doctype(True)
            elif eof:
                self.emit_doctype(True)
                return False
            else:
                self.doctype[0] = [_R if c == "\0" else ascii_lower(c)]
                self.state = "doctype_name"
        elif st == "doctype_name":
            if c in _WS:
                self.state = "after_doctype_name"
            elif c == ">":
                self.state = "data"
                self.emit_doctype()
            elif eof:
                self.emit_doctype(True)
                return False
            else:
                self.doctype[0].append(_R if c == "\0" else ascii_lower(c))
        elif st == "after_doctype_name":
            if c in _WS:
                return True
            if c == ">":
                self.state = "data"
                self.emit_doctype()
            elif eof:
                self.emit_doctype(True)
                return False
            else:
                word = ascii_lower(self.s[self.i - 1:self.i + 5])
                if word in ("public", "system"):
                    self.i += 5
                    self.state = "after_doctype_%s_kw" % word
                else:
                    self.doctype[3] = True
                    self.state = "bogus_doctype"
        elif st in ("after_doctype_public_kw", "after_doctype_system_kw",
                    "before_doctype_public_id", "before_doctype_system_id"):
            idx = 1 if "public" in st else 2
            which = "public" if idx == 1 else "system"
            if c in _WS:
                if st.startswith("after"):
                    self.state = "before_doctype_%s_id" % which
            elif c in ('"', "'"):
                self.doctype[idx] = []
                self.state = "doctype_%s_%s" % (which, "dq" if c == '"' else "sq")
            elif c == ">":
                self.state = "data"
                self.emit_doctype(True)
            elif eof:
                self.emit_doctype(True)
                return False
            else:
                self.doctype[3] = True
                self.reconsume("bogus_doctype")
        elif st in ("doctype_public_dq", "doctype_public_sq", "doctype_system_dq", "doctype_system_sq"):
            idx = 1 if "public" in st else 2
            if c == ('"' if st.endswith("dq") else "'"):
                self.state = "after_doctype_public_id" if idx == 1 else "after_doctype_system_id"
            elif c == ">":
                self.state = "data"
                self.emit_doctype(True)
            elif eof:
                self.emit_doctype(True)
                return False
            else:
                self.doctype[idx].append(_R if c == "\0" else c)
        elif st in ("after_doctype_public_id", "between_doctype_ids"):
            if c in _WS:
                self.state = "between_doctype_ids"
            elif c == ">":
                self.state = "data"
                self.emit_doctype()
            elif c in ('"', "'"):
                self.doctype[2] = []
                self.state = "doctype_system_dq" if c == '"' else "doctype_system_sq"
            elif eof:
                self.emit_doctype(True)
                return False
            else:
                self.doctype[3] = True
                self.reconsume("bogus_doctype")
        elif st == "after_doctype_system_id":
            if c in _WS:
                pass
            elif c == ">":
                self.state = "data"
                self.emit_doctype()
            elif eof:
                self.emit_doctype(True)
                return False
            else:
                self.reconsume("bogus_doctype")
        elif st == "bogus_doctype":
            if c == ">":
                self.state = "data"
                self.emit_doctype()
            elif eof:
                self.emit_doctype()
                return False
        elif st == "cdata":
            if c == "]":
                self.state = "cdata_bracket"
            elif eof:
                return False
            else:
                self.emit(c)
        elif st == "cdata_bracket":
            if c == "]":
                self.state = "cdata_end"
            else:
                self.emit("]")
                self.reconsume("cdata")
        elif st == "cdata_end":
            if c == "]":
                self.emit("]")
            elif c == ">":
                self.state = "data"
            else:
                self.emit("]]")
                self.reconsume("cdata")
        elif st == "charref":
            self.temp = ["&"]
            if c in ALNUM:
                self.reconsume("named_ref")
            elif c == "#":
                self.temp.append(c)
                self.state = "numeric_ref"
            else:
                self.flush_ref()
                self.reconsume(self.ret)
        elif st == "named_ref":
            self.named_ref()
        elif st == "ambiguous_amp":
            if c in ALNUM:
                if self.in_attr():
                    self.attr[1].append(c)
                else:
                    self.emit(c)
            else:
                self.reconsume(self.ret)
        elif st == "numeric_ref":
            self.code = 0
            if c in ("x", "X"):
                self.temp.append(c)
                self.state = "hex_ref_start"
            else:
                self.reconsume("dec_ref_start")
        elif st in ("hex_ref_start", "dec_ref_start"):
            hexa = st == "hex_ref_start"
            if c in (_HEX if hexa else _DIGIT):
                self.reconsume("hex_ref" if hexa else "dec_ref")
            else:
                self.flush_ref()
                self.reconsume(self.ret)
        elif st in ("hex_ref", "dec_ref"):
            hexa = st == "hex_ref"
            if c in (_HEX if hexa else _DIGIT):
                self.code = min(self.code * (16 if hexa else 10) + int(c, 16), 0x110000)
            elif c == ";":
                self.state = "numeric_ref_end"
            else:
                self.reconsume("numeric_ref_end")
        else:  # numeric_ref_end
            self.i -= 1
            self.temp = [numeric_char(self.code)]
            self.flush_ref()
            self.state = self.ret
        return True

    def named_ref(self) -> None:
        self.i -= 1
        start = j = self.i
        s = self.s
        while j < len(s) and s[j] in ALNUM and j - start < MAX_ENTITY:
            j += 1
        run = s[start:j]
        match = None
        if s.startswith(";", j) and run + ";" in ENTITIES:
            match = run + ";"
        else:
            for k in range(len(run), 0, -1):
                if run[:k] in ENTITIES:
                    match = run[:k]
                    break
        if match is None:
            self.temp.append(run)
            self.i = j
            self.flush_ref()
            self.state = "ambiguous_amp"
            return
        self.i = start + len(match)
        nxt = s[self.i:self.i + 1]
        if self.in_attr() and not match.endswith(";") and nxt != "" and (nxt == "=" or nxt in ALNUM):
            self.temp.append(match)
        else:
            self.temp = [ENTITIES[match]]
        self.flush_ref()
        self.state = self.ret


def to_tokens(raw: List[list]) -> List[Token]:
    return [tuple(t) for t in raw]


STATES = ("data", "rcdata", "rawtext", "script", "plaintext", "cdata")


def tokenize(html: str) -> List[Token]:
    """Tokenizes `html` as the parser does, switching state after title,
    textarea, style, xmp, iframe, noembed, noframes, noscript, script and
    plaintext start tags (SPEC section 2.2)."""
    return to_tokens(_Tokenizer(html, "data", None, True).run())


def tokenize_state(html: str, state: str = "data", last_start_tag: Optional[str] = None) -> List[Token]:
    """Tokenizes `html` from `state` (one of STATES) without element-driven switching."""
    if state not in STATES:
        raise ValueError("unknown tokenizer state %r" % (state,))
    return to_tokens(_Tokenizer(html, state, last_start_tag, False).run())
