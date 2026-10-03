"""Independent reference model of docs/SPEC_LombokHTML_v0.2.0.md.

Written from the SPEC (WHATWG HTML tokenizer section 13.2.5 plus the LombokHTML
tree, text, sanitizer, selector, meta and table rules). Shares no code with the
five ports. Used by vectors/build_vectors.py and vectors/check_conformance.py.
"""
import html.entities
import re

ENTITIES = html.entities.html5  # WHATWG named character references (with and without ';')
MAX_ENTITY = max(len(k) for k in ENTITIES)

REPLACEMENTS = {
    0x80: 0x20AC, 0x82: 0x201A, 0x83: 0x0192, 0x84: 0x201E, 0x85: 0x2026, 0x86: 0x2020, 0x87: 0x2021,
    0x88: 0x02C6, 0x89: 0x2030, 0x8A: 0x0160, 0x8B: 0x2039, 0x8C: 0x0152, 0x8E: 0x017D, 0x91: 0x2018,
    0x92: 0x2019, 0x93: 0x201C, 0x94: 0x201D, 0x95: 0x2022, 0x96: 0x2013, 0x97: 0x2014, 0x98: 0x02DC,
    0x99: 0x2122, 0x9A: 0x0161, 0x9B: 0x203A, 0x9C: 0x0153, 0x9E: 0x017E, 0x9F: 0x0178,
}

WS = "\t\n\f "
ALPHA = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
DIGITS = set("0123456789")
HEX = set("0123456789abcdefABCDEF")
ALNUM = ALPHA | DIGITS
ASCII_LOWER = {c: c + 32 for c in range(65, 91)}


def lower(s):
    """ASCII lowercase; every case-insensitive comparison in the SPEC is ASCII-only."""
    return s.translate(ASCII_LOWER)


def split_ws(s):
    """Split on ASCII whitespace (TAB, LF, FF, CR, SPACE), dropping empty pieces."""
    return [x for x in re.split("[\t\n\f\r ]+", s) if x]


def numeric_char(code):
    if code == 0 or code > 0x10FFFF or 0xD800 <= code <= 0xDFFF:
        return "�"
    if code in REPLACEMENTS:
        return chr(REPLACEMENTS[code])
    return chr(code)


class Tokenizer:
    """WHATWG tokenizer. Tokens: ("StartTag", name, [[k, v], ...], selfclosing),
    ("EndTag", name), ("Character", data), ("Comment", data),
    ("DOCTYPE", name, public, system, correct)."""

    def __init__(self, text, state="data", last_start_tag=None, switch=False):
        text = text.replace("\r\n", "\n").replace("\r", "\n")
        self.s = text
        self.i = 0
        self.state = state
        self.last_start = last_start_tag
        self.switch = switch
        self.out = []
        self.ret = None
        self.tag = None
        self.attr = None
        self.temp = ""
        self.comment = None
        self.doctype = None
        self.code = 0

    # ---------------------------------------------------------------- helpers
    def emit_chars(self, s):
        if self.out and self.out[-1][0] == "Character":
            self.out[-1] = ("Character", self.out[-1][1] + s)
        else:
            self.out.append(("Character", s))

    def emit_tag(self):
        t = self.tag
        if t["kind"] == "start":
            attrs = []
            seen = set()
            for k, v in t["attrs"]:
                if k not in seen:
                    seen.add(k)
                    attrs.append([k, v])
            self.out.append(("StartTag", t["name"], attrs, t["self"]))
            self.last_start = t["name"]
            if self.switch:
                n = t["name"]
                if n in ("title", "textarea"):
                    self.state = "rcdata"
                elif n in ("style", "xmp", "iframe", "noembed", "noframes", "noscript"):
                    self.state = "rawtext"
                elif n == "script":
                    self.state = "script"
                elif n == "plaintext":
                    self.state = "plaintext"
        else:
            self.out.append(("EndTag", t["name"]))
        self.tag = None

    def appropriate(self):
        return self.tag is not None and self.tag["kind"] == "end" and self.tag["name"] == self.last_start

    def start_attr(self):
        self.attr = ["", ""]
        self.tag["attrs"].append(self.attr)

    def peek(self, n):
        return self.s[self.i:self.i + n]

    def in_attr(self):
        return self.ret in ("attr_dq", "attr_sq", "attr_unq")

    def flush_ref(self):
        if self.in_attr():
            self.attr[1] += self.temp
        else:
            self.emit_chars(self.temp)

    # ---------------------------------------------------------------- run
    def run(self):
        while True:
            c = self.s[self.i] if self.i < len(self.s) else None
            self.i += 1
            if getattr(self, "st_" + self.state)(c) is False:
                break
        return self.out

    def reconsume(self, state):
        self.i -= 1
        self.state = state

    # data-like states
    def st_data(self, c):
        if c == "&":
            self.ret = "data"
            self.state = "charref"
        elif c == "<":
            self.state = "tag_open"
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    def st_rcdata(self, c):
        if c == "&":
            self.ret = "rcdata"
            self.state = "charref"
        elif c == "<":
            self.state = "rcdata_lt"
        elif c == "\0":
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    def st_rawtext(self, c):
        if c == "<":
            self.state = "rawtext_lt"
        elif c == "\0":
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    def st_script(self, c):
        if c == "<":
            self.state = "script_lt"
        elif c == "\0":
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    def st_plaintext(self, c):
        if c == "\0":
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    # tags
    def st_tag_open(self, c):
        if c == "!":
            self.state = "markup_decl"
        elif c == "/":
            self.state = "end_tag_open"
        elif c is not None and c in ALPHA:
            self.tag = {"kind": "start", "name": "", "attrs": [], "self": False}
            self.reconsume("tag_name")
        elif c == "?":
            self.comment = ""
            self.reconsume("bogus_comment")
        elif c is None:
            self.emit_chars("<")
            return False
        else:
            self.emit_chars("<")
            self.reconsume("data")

    def st_end_tag_open(self, c):
        if c is not None and c in ALPHA:
            self.tag = {"kind": "end", "name": "", "attrs": [], "self": False}
            self.reconsume("tag_name")
        elif c == ">":
            self.state = "data"
        elif c is None:
            self.emit_chars("</")
            return False
        else:
            self.comment = ""
            self.reconsume("bogus_comment")

    def st_tag_name(self, c):
        if c is not None and c in WS:
            self.state = "before_attr_name"
        elif c == "/":
            self.state = "self_closing"
        elif c == ">":
            self.state = "data"
            self.emit_tag()
        elif c == "\0":
            self.tag["name"] += "�"
        elif c is None:
            return False
        else:
            self.tag["name"] += c.lower() if c in ALPHA else c

    # rcdata / rawtext / script end tags
    def _lt(self, c, base, open_state):
        if c == "/":
            self.temp = ""
            self.state = open_state
        else:
            self.emit_chars("<")
            self.reconsume(base)

    def _end_open(self, c, base, name_state):
        if c is not None and c in ALPHA:
            self.tag = {"kind": "end", "name": "", "attrs": [], "self": False}
            self.reconsume(name_state)
        else:
            self.emit_chars("</")
            self.reconsume(base)

    def _end_name(self, c, base):
        if c is not None and c in WS and self.appropriate():
            self.state = "before_attr_name"
            return
        if c == "/" and self.appropriate():
            self.state = "self_closing"
            return
        if c == ">" and self.appropriate():
            self.state = "data"
            self.emit_tag()
            return
        if c is not None and c in ALPHA:
            self.tag["name"] += c.lower()
            self.temp += c
            return
        self.emit_chars("</" + self.temp)
        self.tag = None
        self.reconsume(base)

    def st_rcdata_lt(self, c):
        self._lt(c, "rcdata", "rcdata_end_open")

    def st_rcdata_end_open(self, c):
        self._end_open(c, "rcdata", "rcdata_end_name")

    def st_rcdata_end_name(self, c):
        self._end_name(c, "rcdata")

    def st_rawtext_lt(self, c):
        self._lt(c, "rawtext", "rawtext_end_open")

    def st_rawtext_end_open(self, c):
        self._end_open(c, "rawtext", "rawtext_end_name")

    def st_rawtext_end_name(self, c):
        self._end_name(c, "rawtext")

    def st_script_lt(self, c):
        if c == "/":
            self.temp = ""
            self.state = "script_end_open"
        elif c == "!":
            self.state = "script_esc_start"
            self.emit_chars("<!")
        else:
            self.emit_chars("<")
            self.reconsume("script")

    def st_script_end_open(self, c):
        self._end_open(c, "script", "script_end_name")

    def st_script_end_name(self, c):
        self._end_name(c, "script")

    def st_script_esc_start(self, c):
        if c == "-":
            self.state = "script_esc_start_dash"
            self.emit_chars("-")
        else:
            self.reconsume("script")

    def st_script_esc_start_dash(self, c):
        if c == "-":
            self.state = "script_esc_dash_dash"
            self.emit_chars("-")
        else:
            self.reconsume("script")

    def st_script_esc(self, c):
        if c == "-":
            self.state = "script_esc_dash"
            self.emit_chars("-")
        elif c == "<":
            self.state = "script_esc_lt"
        elif c == "\0":
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    def st_script_esc_dash(self, c):
        if c == "-":
            self.state = "script_esc_dash_dash"
            self.emit_chars("-")
        elif c == "<":
            self.state = "script_esc_lt"
        elif c == "\0":
            self.state = "script_esc"
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.state = "script_esc"
            self.emit_chars(c)

    def st_script_esc_dash_dash(self, c):
        if c == "-":
            self.emit_chars("-")
        elif c == "<":
            self.state = "script_esc_lt"
        elif c == ">":
            self.state = "script"
            self.emit_chars(">")
        elif c == "\0":
            self.state = "script_esc"
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.state = "script_esc"
            self.emit_chars(c)

    def st_script_esc_lt(self, c):
        if c == "/":
            self.temp = ""
            self.state = "script_esc_end_open"
        elif c is not None and c in ALPHA:
            self.temp = ""
            self.emit_chars("<")
            self.reconsume("script_dbl_esc_start")
        else:
            self.emit_chars("<")
            self.reconsume("script_esc")

    def st_script_esc_end_open(self, c):
        self._end_open(c, "script_esc", "script_esc_end_name")

    def st_script_esc_end_name(self, c):
        self._end_name(c, "script_esc")

    def st_script_dbl_esc_start(self, c):
        if c is not None and (c in WS or c in "/>"):
            self.state = "script_dbl_esc" if self.temp == "script" else "script_esc"
            self.emit_chars(c)
        elif c is not None and c in ALPHA:
            self.temp += c.lower()
            self.emit_chars(c)
        else:
            self.reconsume("script_esc")

    def st_script_dbl_esc(self, c):
        if c == "-":
            self.state = "script_dbl_esc_dash"
            self.emit_chars("-")
        elif c == "<":
            self.state = "script_dbl_esc_lt"
            self.emit_chars("<")
        elif c == "\0":
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    def st_script_dbl_esc_dash(self, c):
        if c == "-":
            self.state = "script_dbl_esc_dash_dash"
            self.emit_chars("-")
        elif c == "<":
            self.state = "script_dbl_esc_lt"
            self.emit_chars("<")
        elif c == "\0":
            self.state = "script_dbl_esc"
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.state = "script_dbl_esc"
            self.emit_chars(c)

    def st_script_dbl_esc_dash_dash(self, c):
        if c == "-":
            self.emit_chars("-")
        elif c == "<":
            self.state = "script_dbl_esc_lt"
            self.emit_chars("<")
        elif c == ">":
            self.state = "script"
            self.emit_chars(">")
        elif c == "\0":
            self.state = "script_dbl_esc"
            self.emit_chars("�")
        elif c is None:
            return False
        else:
            self.state = "script_dbl_esc"
            self.emit_chars(c)

    def st_script_dbl_esc_lt(self, c):
        if c == "/":
            self.temp = ""
            self.state = "script_dbl_esc_end"
            self.emit_chars("/")
        else:
            self.reconsume("script_dbl_esc")

    def st_script_dbl_esc_end(self, c):
        if c is not None and (c in WS or c in "/>"):
            self.state = "script_esc" if self.temp == "script" else "script_dbl_esc"
            self.emit_chars(c)
        elif c is not None and c in ALPHA:
            self.temp += c.lower()
            self.emit_chars(c)
        else:
            self.reconsume("script_dbl_esc")

    # attributes
    def st_before_attr_name(self, c):
        if c is not None and c in WS:
            return
        if c is None or c in "/>":
            self.reconsume("after_attr_name")
        elif c == "=":
            self.start_attr()
            self.attr[0] = c
            self.state = "attr_name"
        else:
            self.start_attr()
            self.reconsume("attr_name")

    def st_attr_name(self, c):
        if c is None or c in WS or c in "/>":
            self.reconsume("after_attr_name")
        elif c == "=":
            self.state = "before_attr_value"
        elif c == "\0":
            self.attr[0] += "�"
        else:
            self.attr[0] += c.lower() if c in ALPHA else c

    def st_after_attr_name(self, c):
        if c is not None and c in WS:
            return
        if c == "/":
            self.state = "self_closing"
        elif c == "=":
            self.state = "before_attr_value"
        elif c == ">":
            self.state = "data"
            self.emit_tag()
        elif c is None:
            return False
        else:
            self.start_attr()
            self.reconsume("attr_name")

    def st_before_attr_value(self, c):
        if c is not None and c in WS:
            return
        if c == '"':
            self.state = "attr_dq"
        elif c == "'":
            self.state = "attr_sq"
        elif c == ">":
            self.state = "data"
            self.emit_tag()
        else:
            self.reconsume("attr_unq")

    def _attr_q(self, c, q, me):
        if c == q:
            self.state = "after_attr_value_q"
        elif c == "&":
            self.ret = me
            self.state = "charref"
        elif c == "\0":
            self.attr[1] += "�"
        elif c is None:
            return False
        else:
            self.attr[1] += c

    def st_attr_dq(self, c):
        return self._attr_q(c, '"', "attr_dq")

    def st_attr_sq(self, c):
        return self._attr_q(c, "'", "attr_sq")

    def st_attr_unq(self, c):
        if c is not None and c in WS:
            self.state = "before_attr_name"
        elif c == "&":
            self.ret = "attr_unq"
            self.state = "charref"
        elif c == ">":
            self.state = "data"
            self.emit_tag()
        elif c == "\0":
            self.attr[1] += "�"
        elif c is None:
            return False
        else:
            self.attr[1] += c

    def st_after_attr_value_q(self, c):
        if c is not None and c in WS:
            self.state = "before_attr_name"
        elif c == "/":
            self.state = "self_closing"
        elif c == ">":
            self.state = "data"
            self.emit_tag()
        elif c is None:
            return False
        else:
            self.reconsume("before_attr_name")

    def st_self_closing(self, c):
        if c == ">":
            self.tag["self"] = True
            self.state = "data"
            self.emit_tag()
        elif c is None:
            return False
        else:
            self.reconsume("before_attr_name")

    # comments
    def st_bogus_comment(self, c):
        if c == ">":
            self.state = "data"
            self.out.append(("Comment", self.comment))
        elif c is None:
            self.out.append(("Comment", self.comment))
            return False
        elif c == "\0":
            self.comment += "�"
        else:
            self.comment += c

    def st_markup_decl(self, c):
        self.i -= 1
        if self.peek(2) == "--":
            self.i += 2
            self.comment = ""
            self.state = "comment_start"
        elif lower(self.peek(7)) == "doctype":
            self.i += 7
            self.state = "doctype"
        elif self.peek(7) == "[CDATA[":
            # Not in foreign content: bogus comment.
            self.comment = ""
            self.state = "bogus_comment"
        else:
            self.comment = ""
            self.state = "bogus_comment"

    def st_comment_start(self, c):
        if c == "-":
            self.state = "comment_start_dash"
        elif c == ">":
            self.state = "data"
            self.out.append(("Comment", self.comment))
        else:
            self.reconsume("comment")

    def st_comment_start_dash(self, c):
        if c == "-":
            self.state = "comment_end"
        elif c == ">":
            self.state = "data"
            self.out.append(("Comment", self.comment))
        elif c is None:
            self.out.append(("Comment", self.comment))
            return False
        else:
            self.comment += "-"
            self.reconsume("comment")

    def st_comment(self, c):
        if c == "<":
            self.comment += c
            self.state = "comment_lt"
        elif c == "-":
            self.state = "comment_end_dash"
        elif c == "\0":
            self.comment += "�"
        elif c is None:
            self.out.append(("Comment", self.comment))
            return False
        else:
            self.comment += c

    def st_comment_lt(self, c):
        if c == "!":
            self.comment += c
            self.state = "comment_lt_bang"
        elif c == "<":
            self.comment += c
        else:
            self.reconsume("comment")

    def st_comment_lt_bang(self, c):
        if c == "-":
            self.state = "comment_lt_bang_dash"
        else:
            self.reconsume("comment")

    def st_comment_lt_bang_dash(self, c):
        if c == "-":
            self.state = "comment_lt_bang_dash_dash"
        else:
            self.reconsume("comment_end_dash")

    def st_comment_lt_bang_dash_dash(self, c):
        self.reconsume("comment_end")

    def st_comment_end_dash(self, c):
        if c == "-":
            self.state = "comment_end"
        elif c is None:
            self.out.append(("Comment", self.comment))
            return False
        else:
            self.comment += "-"
            self.reconsume("comment")

    def st_comment_end(self, c):
        if c == ">":
            self.state = "data"
            self.out.append(("Comment", self.comment))
        elif c == "!":
            self.state = "comment_end_bang"
        elif c == "-":
            self.comment += "-"
        elif c is None:
            self.out.append(("Comment", self.comment))
            return False
        else:
            self.comment += "--"
            self.reconsume("comment")

    def st_comment_end_bang(self, c):
        if c == "-":
            self.comment += "--!"
            self.state = "comment_end_dash"
        elif c == ">":
            self.state = "data"
            self.out.append(("Comment", self.comment))
        elif c is None:
            self.out.append(("Comment", self.comment))
            return False
        else:
            self.comment += "--!"
            self.reconsume("comment")

    # doctype
    def new_doctype(self):
        self.doctype = {"name": None, "public": None, "system": None, "quirks": False}

    def emit_doctype(self, quirks=False):
        if quirks:
            self.doctype["quirks"] = True
        d = self.doctype
        self.out.append(("DOCTYPE", d["name"], d["public"], d["system"], not d["quirks"]))

    def st_doctype(self, c):
        if c is not None and c in WS:
            self.state = "before_doctype_name"
        elif c == ">":
            self.reconsume("before_doctype_name")
        elif c is None:
            self.new_doctype()
            self.emit_doctype(True)
            return False
        else:
            self.reconsume("before_doctype_name")

    def st_before_doctype_name(self, c):
        if c is not None and c in WS:
            return
        self.new_doctype()
        if c == "\0":
            self.doctype["name"] = "�"
            self.state = "doctype_name"
        elif c == ">":
            self.state = "data"
            self.emit_doctype(True)
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.doctype["name"] = c.lower() if c in ALPHA else c
            self.state = "doctype_name"

    def st_doctype_name(self, c):
        if c is not None and c in WS:
            self.state = "after_doctype_name"
        elif c == ">":
            self.state = "data"
            self.emit_doctype()
        elif c == "\0":
            self.doctype["name"] += "�"
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.doctype["name"] += c.lower() if c in ALPHA else c

    def st_after_doctype_name(self, c):
        if c is not None and c in WS:
            return
        if c == ">":
            self.state = "data"
            self.emit_doctype()
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.i -= 1
            word = lower(self.peek(6))
            if word == "public":
                self.i += 6
                self.state = "after_doctype_public_kw"
            elif word == "system":
                self.i += 6
                self.state = "after_doctype_system_kw"
            else:
                self.i += 1
                self.doctype["quirks"] = True
                self.state = "bogus_doctype"

    def _after_kw(self, c, which, before_state):
        if c is not None and c in WS:
            self.state = before_state
        elif c in ('"', "'"):
            self.doctype[which] = ""
            self.state = "doctype_%s_%s" % (which, "dq" if c == '"' else "sq")
        elif c == ">":
            self.state = "data"
            self.emit_doctype(True)
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.doctype["quirks"] = True
            self.reconsume("bogus_doctype")

    def st_after_doctype_public_kw(self, c):
        return self._after_kw(c, "public", "before_doctype_public_id")

    def st_after_doctype_system_kw(self, c):
        return self._after_kw(c, "system", "before_doctype_system_id")

    def _before_id(self, c, which):
        if c is not None and c in WS:
            return
        if c in ('"', "'"):
            self.doctype[which] = ""
            self.state = "doctype_%s_%s" % (which, "dq" if c == '"' else "sq")
        elif c == ">":
            self.state = "data"
            self.emit_doctype(True)
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.doctype["quirks"] = True
            self.reconsume("bogus_doctype")

    def st_before_doctype_public_id(self, c):
        return self._before_id(c, "public")

    def st_before_doctype_system_id(self, c):
        return self._before_id(c, "system")

    def _id(self, c, which, q, after):
        if c == q:
            self.state = after
        elif c == "\0":
            self.doctype[which] += "�"
        elif c == ">":
            self.state = "data"
            self.emit_doctype(True)
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.doctype[which] += c

    def st_doctype_public_dq(self, c):
        return self._id(c, "public", '"', "after_doctype_public_id")

    def st_doctype_public_sq(self, c):
        return self._id(c, "public", "'", "after_doctype_public_id")

    def st_doctype_system_dq(self, c):
        return self._id(c, "system", '"', "after_doctype_system_id")

    def st_doctype_system_sq(self, c):
        return self._id(c, "system", "'", "after_doctype_system_id")

    def st_after_doctype_public_id(self, c):
        if c is not None and c in WS:
            self.state = "between_doctype_ids"
        elif c == ">":
            self.state = "data"
            self.emit_doctype()
        elif c in ('"', "'"):
            self.doctype["system"] = ""
            self.state = "doctype_system_dq" if c == '"' else "doctype_system_sq"
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.doctype["quirks"] = True
            self.reconsume("bogus_doctype")

    def st_between_doctype_ids(self, c):
        if c is not None and c in WS:
            return
        if c == ">":
            self.state = "data"
            self.emit_doctype()
        elif c in ('"', "'"):
            self.doctype["system"] = ""
            self.state = "doctype_system_dq" if c == '"' else "doctype_system_sq"
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.doctype["quirks"] = True
            self.reconsume("bogus_doctype")

    def st_after_doctype_system_id(self, c):
        if c is not None and c in WS:
            return
        if c == ">":
            self.state = "data"
            self.emit_doctype()
        elif c is None:
            self.emit_doctype(True)
            return False
        else:
            self.reconsume("bogus_doctype")

    def st_bogus_doctype(self, c):
        if c == ">":
            self.state = "data"
            self.emit_doctype()
        elif c is None:
            self.emit_doctype()
            return False

    # cdata (only as an initial state)
    def st_cdata(self, c):
        if c == "]":
            self.state = "cdata_bracket"
        elif c is None:
            return False
        else:
            self.emit_chars(c)

    def st_cdata_bracket(self, c):
        if c == "]":
            self.state = "cdata_end"
        else:
            self.emit_chars("]")
            self.reconsume("cdata")

    def st_cdata_end(self, c):
        if c == "]":
            self.emit_chars("]")
        elif c == ">":
            self.state = "data"
        else:
            self.emit_chars("]]")
            self.reconsume("cdata")

    # character references
    def st_charref(self, c):
        self.temp = "&"
        if c is not None and c in ALNUM:
            self.reconsume("named_ref")
        elif c == "#":
            self.temp += c
            self.state = "numeric_ref"
        else:
            self.flush_ref()
            self.reconsume(self.ret)

    def st_named_ref(self, c):
        self.i -= 1
        j = self.i
        while j < len(self.s) and self.s[j] in ALNUM and j - self.i < MAX_ENTITY:
            j += 1
        run = self.s[self.i:j]
        match = None
        if j < len(self.s) and self.s[j] == ";" and run + ";" in ENTITIES:
            match = run + ";"
        else:
            for k in range(len(run), 0, -1):
                if run[:k] in ENTITIES:
                    match = run[:k]
                    break
        if match is None:
            self.temp += run
            self.i = j
            self.flush_ref()
            self.state = "ambiguous_amp"
            return
        self.i += len(match)
        nxt = self.s[self.i] if self.i < len(self.s) else None
        if self.in_attr() and not match.endswith(";") and nxt is not None and (nxt == "=" or nxt in ALNUM):
            self.temp += match
            self.flush_ref()
            self.state = self.ret
            return
        self.temp = ENTITIES[match]
        self.flush_ref()
        self.state = self.ret

    def st_ambiguous_amp(self, c):
        if c is not None and c in ALNUM:
            if self.in_attr():
                self.attr[1] += c
            else:
                self.emit_chars(c)
        else:
            self.reconsume(self.ret)

    def st_numeric_ref(self, c):
        self.code = 0
        if c in ("x", "X"):
            self.temp += c
            self.state = "hex_ref_start"
        else:
            self.reconsume("dec_ref_start")

    def st_hex_ref_start(self, c):
        if c is not None and c in HEX:
            self.reconsume("hex_ref")
        else:
            self.flush_ref()
            self.reconsume(self.ret)

    def st_dec_ref_start(self, c):
        if c is not None and c in DIGITS:
            self.reconsume("dec_ref")
        else:
            self.flush_ref()
            self.reconsume(self.ret)

    def st_hex_ref(self, c):
        if c is not None and c in HEX:
            self.code = min(self.code * 16 + int(c, 16), 0x110000)
        elif c == ";":
            self.state = "numeric_ref_end"
        else:
            self.reconsume("numeric_ref_end")

    def st_dec_ref(self, c):
        if c is not None and c in DIGITS:
            self.code = min(self.code * 10 + int(c), 0x110000)
        elif c == ";":
            self.state = "numeric_ref_end"
        else:
            self.reconsume("numeric_ref_end")

    def st_numeric_ref_end(self, c):
        self.i -= 1
        self.temp = numeric_char(self.code)
        self.flush_ref()
        self.state = self.ret


def tokenize(text, state="data", last_start_tag=None, switch=False):
    return Tokenizer(text, state, last_start_tag, switch).run()


def decode_entities(text):
    """Character references decoded as in text content (SPEC section 3)."""
    out = []
    i, n = 0, len(text)
    while i < n:
        ch = text[i]
        if ch != "&":
            out.append(ch)
            i += 1
            continue
        j = i + 1
        if j < n and text[j] == "#":
            k = j + 1
            hexa = k < n and text[k] in "xX"
            if hexa:
                k += 1
            digits = HEX if hexa else DIGITS
            start = k
            code = 0
            while k < n and text[k] in digits:
                code = min(code * (16 if hexa else 10) + int(text[k], 16), 0x110000)
                k += 1
            if k == start:
                out.append(text[i:k])
                i = k
                continue
            if k < n and text[k] == ";":
                k += 1
            out.append(numeric_char(code))
            i = k
            continue
        k = j
        while k < n and text[k] in ALNUM and k - j < MAX_ENTITY:
            k += 1
        run = text[j:k]
        match = None
        if k < n and text[k] == ";" and run + ";" in ENTITIES:
            match = run + ";"
        else:
            for m in range(len(run), 0, -1):
                if run[:m] in ENTITIES:
                    match = run[:m]
                    break
        if match is None:
            out.append("&" + run)
            i = k
            continue
        out.append(ENTITIES[match])
        i = j + len(match)
    return "".join(out)


# ---------------------------------------------------------------- tree

VOID = {"area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input", "keygen",
        "link", "meta", "param", "source", "track", "wbr"}
RAW_PARENTS = {"style", "script", "xmp", "iframe", "noembed", "noframes", "plaintext", "noscript"}
SPECIAL = {"address", "applet", "area", "article", "aside", "base", "basefont", "bgsound", "blockquote", "body", "br",
           "button", "caption", "center", "col", "colgroup", "dd", "details", "dir", "div", "dl", "dt", "embed",
           "fieldset", "figcaption", "figure", "footer", "form", "frame", "frameset", "h1", "h2", "h3", "h4", "h5",
           "h6", "head", "header", "hgroup", "hr", "html", "iframe", "img", "input", "keygen", "li", "link", "listing",
           "main", "marquee", "menu", "meta", "nav", "noembed", "noframes", "noscript", "object", "ol", "p", "param",
           "plaintext", "pre", "script", "search", "section", "select", "source", "style", "summary", "table",
           "tbody", "td", "template", "textarea", "tfoot", "th", "thead", "title", "tr", "track", "ul", "wbr", "xmp"}
P_CLOSERS = {"address", "article", "aside", "blockquote", "center", "details", "dialog", "dir", "div", "dl",
             "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header",
             "hgroup", "hr", "li", "dd", "dt", "listing", "main", "menu", "nav", "ol", "p", "plaintext", "pre",
             "search", "section", "summary", "table", "ul", "xmp"}
HEADINGS = {"h1", "h2", "h3", "h4", "h5", "h6"}
SCOPE = {"applet", "caption", "html", "table", "td", "th", "marquee", "object", "template"}
BUTTON_SCOPE = SCOPE | {"button"}
LIST_SCOPE = SCOPE | {"ol", "ul"}
TABLE_SCOPE = {"html", "table", "template"}
MAX_DEPTH = 256


class Node:
    __slots__ = ("kind", "name", "attrs", "data", "children", "parent")

    def __init__(self, kind, name=None, attrs=None, data=None):
        self.kind = kind  # document, element, text, comment
        self.name = name
        self.attrs = attrs or []
        self.data = data
        self.children = []
        self.parent = None

    def attr(self, name):
        for k, v in self.attrs:
            if k == name:
                return v
        return None

    def elements(self):
        for c in self.children:
            if c.kind == "element":
                yield c
                yield from c.elements()


def in_scope(stack, names, boundary):
    for node in reversed(stack):
        if node.name in names:
            return True
        if node.name in boundary:
            return False
    return False


def pop_until(stack, names):
    while stack:
        node = stack.pop()
        if node.name in names:
            return


def parse(text):
    doc = Node("document")
    stack = []
    skip_newline = False

    def current():
        return stack[-1] if stack else doc

    def append(node):
        parent = current()
        node.parent = parent
        parent.children.append(node)

    def add_text(data):
        parent = current()
        if parent.children and parent.children[-1].kind == "text":
            parent.children[-1].data += data
        else:
            append(Node("text", data=data))

    def start(name, attrs):
        if name in ("li", "dd", "dt"):
            targets = {"li"} if name == "li" else {"dd", "dt"}
            for node in reversed(stack):
                if node.name in targets:
                    pop_until(stack, {node.name})
                    break
                if node.name in SPECIAL and node.name not in ("address", "div", "p"):
                    break
        if name in P_CLOSERS and in_scope(stack, {"p"}, BUTTON_SCOPE):
            pop_until(stack, {"p"})
        if name in HEADINGS and stack and stack[-1].name in HEADINGS:
            stack.pop()
        if name in ("option", "optgroup") and stack and stack[-1].name == "option":
            stack.pop()
        if name == "a" and any(n.name == "a" for n in stack):
            pop_until(stack, {"a"})
        if name in ("td", "th", "tr", "thead", "tbody", "tfoot") and in_scope(stack, {"td", "th"}, TABLE_SCOPE):
            pop_until(stack, {"td", "th"})
        if name in ("tr", "thead", "tbody", "tfoot") and in_scope(stack, {"tr"}, TABLE_SCOPE):
            pop_until(stack, {"tr"})
        if name in ("thead", "tbody", "tfoot") and in_scope(stack, {"thead", "tbody", "tfoot"}, TABLE_SCOPE):
            pop_until(stack, {"thead", "tbody", "tfoot"})
        if len(stack) >= MAX_DEPTH:
            return False
        el = Node("element", name, attrs)
        append(el)
        if name not in VOID:
            stack.append(el)
        return name in ("pre", "listing", "textarea")

    def end(name):
        if name == "br":
            start("br", [])
            return
        if name == "p":
            if in_scope(stack, {"p"}, BUTTON_SCOPE):
                pop_until(stack, {"p"})
            return
        if name in HEADINGS:
            if in_scope(stack, HEADINGS, SCOPE):
                pop_until(stack, HEADINGS)
            return
        if name == "li":
            if in_scope(stack, {"li"}, LIST_SCOPE):
                pop_until(stack, {"li"})
            return
        if name in ("dd", "dt"):
            if in_scope(stack, {name}, SCOPE):
                pop_until(stack, {name})
            return
        if name in ("table", "caption", "tbody", "thead", "tfoot", "tr", "td", "th"):
            if in_scope(stack, {name}, TABLE_SCOPE):
                pop_until(stack, {name})
            return
        if name in SPECIAL:
            if in_scope(stack, {name}, SCOPE):
                pop_until(stack, {name})
            return
        for idx in range(len(stack) - 1, -1, -1):
            node = stack[idx]
            if node.name == name:
                del stack[idx:]
                return
            if node.name in SPECIAL:
                return

    for tok in tokenize(text, switch=True):
        kind = tok[0]
        if kind == "Character":
            data = tok[1]
            if skip_newline and data.startswith("\n"):
                data = data[1:]
            skip_newline = False
            if data:
                add_text(data)
            continue
        skip_newline = False
        if kind == "StartTag":
            skip_newline = start(tok[1], [list(a) for a in tok[2]])
        elif kind == "EndTag":
            end(tok[1])
        elif kind == "Comment":
            append(Node("comment", data=tok[1]))
    return doc


# ---------------------------------------------------------------- serialization

def escape_text(s):
    return s.replace("&", "&amp;").replace("\xa0", "&nbsp;").replace("<", "&lt;").replace(">", "&gt;")


def escape_attr(s):
    return (s.replace("&", "&amp;").replace("\xa0", "&nbsp;").replace('"', "&quot;")
            .replace("<", "&lt;").replace(">", "&gt;"))


def serialize_node(node, out):
    if node.kind == "text":
        raw = node.parent is not None and node.parent.kind == "element" and node.parent.name in RAW_PARENTS
        out.append(node.data if raw else escape_text(node.data))
    elif node.kind == "comment":
        out.append("<!--%s-->" % node.data)
    elif node.kind == "element":
        out.append("<" + node.name)
        for k, v in node.attrs:
            out.append(' %s="%s"' % (k, escape_attr(v)))
        out.append(">")
        if node.name in VOID:
            return
        for c in node.children:
            serialize_node(c, out)
        out.append("</%s>" % node.name)
    else:
        for c in node.children:
            serialize_node(c, out)


def serialize(node):
    out = []
    serialize_node(node, out)
    return "".join(out)


# ---------------------------------------------------------------- text

HIDDEN = {"script", "style", "noscript", "template", "title"}


def text_content(node):
    out = []

    def walk(n):
        if n.kind == "text":
            out.append(n.data)
        elif n.kind in ("element", "document"):
            if n.kind == "element" and n.name in HIDDEN:
                return
            for c in n.children:
                walk(c)
    walk(node)
    return "".join(out)


BLOCKS = {"address", "article", "aside", "blockquote", "caption", "details", "dialog", "div", "dl", "fieldset",
          "figcaption", "figure", "footer", "form", "header", "hgroup", "main", "nav", "ol", "p", "pre", "section",
          "summary", "table", "ul"}
ASCII_WS = "\t\n\f\r "


def extract_text(node):
    out = []

    def last():
        return out[-1][-1] if out and out[-1] else ""

    def emit_text(data, pre):
        if pre:
            out.append(data)
            return
        collapsed = re.sub("[\t\n\f\r ]+", " ", data)
        if collapsed.startswith(" ") and (not out or last() in ("", " ", "\n", "\t")):
            collapsed = collapsed[1:]
        if collapsed:
            out.append(collapsed)

    def walk(n, pre):
        if n.kind == "text":
            emit_text(n.data, pre)
            return
        if n.kind == "comment":
            return
        if n.kind == "document":
            for c in n.children:
                walk(c, pre)
            return
        name = n.name
        if name in HIDDEN:
            return
        if name in HEADINGS:
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
        block = name in BLOCKS
        if block:
            out.append("\n\n")
        for c in n.children:
            walk(c, pre or name in ("pre", "listing", "textarea"))
        if name in ("td", "th"):
            out.append("\t")
        if block:
            out.append("\n\n")

    walk(node, False)
    text = "".join(out)
    lines = [ln.rstrip(" \t") for ln in text.split("\n")]
    text = "\n".join(lines)
    text = re.sub("\n{3,}", "\n\n", text)
    return text.strip("\n")


# ---------------------------------------------------------------- sanitizer

DEFAULT_ALLOWED = ["a", "abbr", "b", "blockquote", "br", "caption", "cite", "code", "dd", "del", "dfn", "div", "dl",
                   "dt", "em", "figcaption", "figure", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "i", "img", "ins",
                   "kbd", "li", "mark", "ol", "p", "pre", "q", "s", "samp", "small", "span", "strong", "sub", "sup",
                   "table", "tbody", "td", "tfoot", "th", "thead", "time", "tr", "u", "ul"]
DEFAULT_DROP = ["applet", "audio", "base", "button", "canvas", "embed", "frame", "frameset", "head", "iframe", "link",
                "math", "meta", "noembed", "noframes", "noscript", "object", "option", "plaintext", "script",
                "select", "style", "svg", "template", "textarea", "title", "video", "xmp"]
DEFAULT_ATTRS = [["*", "dir"], ["*", "lang"], ["*", "title"], ["a", "href"], ["blockquote", "cite"],
                 ["img", "alt"], ["img", "height"], ["img", "src"], ["img", "width"], ["ol", "start"],
                 ["q", "cite"], ["td", "colspan"], ["td", "rowspan"], ["th", "colspan"], ["th", "rowspan"],
                 ["th", "scope"], ["time", "datetime"]]
URL_ATTRS = ["cite", "href", "src"]
DEFAULT_SCHEMES = ["http", "https", "mailto", "tel"]


def make_policy(extra=None):
    p = {"tags": set(DEFAULT_ALLOWED), "drop": set(DEFAULT_DROP), "attrs": {tuple(a) for a in DEFAULT_ATTRS},
         "schemes": set(DEFAULT_SCHEMES)}
    extra = extra or {}
    for t in extra.get("tags", []):
        t = lower(t)
        if t == "plaintext":  # can never be closed again, so never kept
            continue
        p["tags"].add(t)
        p["drop"].discard(t)
    for t, a in extra.get("attrs", []):
        p["attrs"].add((lower(t), lower(a)))
    for sc in extra.get("schemes", []):
        p["schemes"].add(lower(sc))
    return p


def is_safe_url(value, schemes=DEFAULT_SCHEMES):
    cleaned = "".join(ch for ch in value if not (ord(ch) <= 0x20 or ord(ch) == 0x7F))
    cleaned = cleaned.translate({c: c + 32 for c in range(65, 91)})
    for i, ch in enumerate(cleaned):
        if ch in "/?#":
            return True
        if ch == ":":
            return cleaned[:i] in schemes
    return True


def sanitize(text, extra=None):
    p = make_policy(extra)
    out = []

    def walk(n):
        if n.kind == "text":
            out.append(escape_text(n.data))
            return
        if n.kind == "comment":
            return
        if n.kind == "document":
            for c in n.children:
                walk(c)
            return
        name = n.name
        if name in p["drop"]:
            return
        if name not in p["tags"]:
            for c in n.children:
                walk(c)
            return
        out.append("<" + name)
        for k, v in n.attrs:
            if (name, k) not in p["attrs"] and ("*", k) not in p["attrs"]:
                continue
            if k in URL_ATTRS and not is_safe_url(v, p["schemes"]):
                continue
            out.append(' %s="%s"' % (k, escape_attr(v)))
        out.append(">")
        if name in VOID:
            return
        if name not in RAW_PARENTS:  # raw text content would not survive a second pass
            for c in n.children:
                walk(c)
        out.append("</%s>" % name)

    walk(parse(text))
    return "".join(out)


# ---------------------------------------------------------------- selectors

class SelectorError(Exception):
    pass


IDENT_RE = re.compile(r"(?:[A-Za-z0-9_\-]|[^\x00-\x7f])+")


class SelParser:
    def __init__(self, s):
        self.s = s
        self.i = 0

    def fail(self):
        raise SelectorError(self.i)

    def ws(self):
        start = self.i
        while self.i < len(self.s) and self.s[self.i] in " \t\n\r\f":
            self.i += 1
        return self.i > start

    def ident(self):
        m = IDENT_RE.match(self.s, self.i)
        if not m:
            self.fail()
        self.i = m.end()
        return m.group(0)

    def parse_list(self):
        out = []
        while True:
            self.ws()
            out.append(self.complex())
            self.ws()
            if self.i >= len(self.s):
                return out
            if self.s[self.i] != ",":
                self.fail()
            self.i += 1

    def complex(self):
        parts = [(None, self.compound())]
        while True:
            save = self.i
            had_ws = self.ws()
            if self.i >= len(self.s) or self.s[self.i] == ",":
                self.i = save
                return parts
            c = self.s[self.i]
            if c in ">+~":
                self.i += 1
                self.ws()
                parts.append((c, self.compound()))
            elif had_ws:
                parts.append((" ", self.compound()))
            else:
                self.fail()

    def compound(self):
        comp = {"tag": None, "simple": []}
        if self.i < len(self.s) and self.s[self.i] == "*":
            self.i += 1
            comp["tag"] = "*"
        elif self.i < len(self.s) and IDENT_RE.match(self.s, self.i):
            comp["tag"] = lower(self.ident())
        while self.i < len(self.s):
            c = self.s[self.i]
            if c == "#":
                self.i += 1
                comp["simple"].append(("id", self.ident()))
            elif c == ".":
                self.i += 1
                comp["simple"].append(("class", self.ident()))
            elif c == "[":
                self.i += 1
                self.ws()
                name = lower(self.ident())
                self.ws()
                op = None
                val = None
                if self.s[self.i:self.i + 1] == "=":
                    op = "="
                    self.i += 1
                elif self.s[self.i:self.i + 2] in ("~=", "|=", "^=", "$=", "*="):
                    op = self.s[self.i:self.i + 2]
                    self.i += 2
                if op:
                    self.ws()
                    q = self.s[self.i:self.i + 1]
                    if q in ("'", '"'):
                        end = self.s.find(q, self.i + 1)
                        if end < 0:
                            self.fail()
                        val = self.s[self.i + 1:end]
                        self.i = end + 1
                    else:
                        val = self.ident()
                    self.ws()
                if self.s[self.i:self.i + 1] != "]":
                    self.fail()
                self.i += 1
                comp["simple"].append(("attr", name, op, val))
            elif c == ":":
                self.i += 1
                name = lower(self.ident())
                if name in ("first-child", "last-child", "only-child", "empty"):
                    comp["simple"].append(("pseudo", name))
                elif name in ("nth-child", "nth-last-child", "not"):
                    if self.s[self.i:self.i + 1] != "(":
                        self.fail()
                    self.i += 1
                    self.ws()
                    if name == "not":
                        inner = self.compound()
                        self.ws()
                        arg = inner
                    else:
                        end = self.s.find(")", self.i)
                        if end < 0:
                            self.fail()
                        arg = parse_nth(self.s[self.i:end])
                        if arg is None:
                            self.fail()
                        self.i = end
                    if self.s[self.i:self.i + 1] != ")":
                        self.fail()
                    self.i += 1
                    comp["simple"].append(("pseudo", name, arg))
                else:
                    self.fail()
            else:
                break
        if comp["tag"] is None and not comp["simple"]:
            self.fail()
        return comp


NTH_RE = re.compile(r"^([+-]?[0-9]{0,9})n(?:[\t\n\f\r ]*([+-])[\t\n\f\r ]*([0-9]{1,9}))?$")


def parse_nth(text):
    t = lower(text.strip("\t\n\f\r "))
    if t == "odd":
        return (2, 1)
    if t == "even":
        return (2, 0)
    if re.fullmatch(r"[+-]?[0-9]{1,9}", t):
        return (0, int(t))
    m = NTH_RE.match(t)
    if not m:
        return None
    a = m.group(1)
    a = 1 if a in ("", "+") else -1 if a == "-" else int(a)
    b = int(m.group(3)) * (-1 if m.group(2) == "-" else 1) if m.group(3) else 0
    return (a, b)


def nth_ok(a, b, pos):
    """True when pos = a*k + b for some integer k >= 0."""
    if a == 0:
        return pos == b
    if a > 0:
        return pos >= b and (pos - b) % a == 0
    return pos <= b and (b - pos) % (-a) == 0


def element_siblings(el):
    return [c for c in el.parent.children if c.kind == "element"] if el.parent else [el]


def match_compound(el, comp):
    if comp["tag"] not in (None, "*") and el.name != comp["tag"]:
        return False
    for s in comp["simple"]:
        kind = s[0]
        if kind == "id":
            if el.attr("id") != s[1]:
                return False
        elif kind == "class":
            if s[1] not in split_ws(el.attr("class") or ""):
                return False
        elif kind == "attr":
            _, name, op, val = s
            v = el.attr(name)
            if v is None:
                return False
            if op == "=" and v != val:
                return False
            if op == "~=" and val not in split_ws(v):
                return False
            if op == "|=" and not (v == val or v.startswith(val + "-")):
                return False
            if op == "^=" and not (val and v.startswith(val)):
                return False
            if op == "$=" and not (val and v.endswith(val)):
                return False
            if op == "*=" and not (val and val in v):
                return False
        else:
            name = s[1]
            sib = element_siblings(el)
            pos = sib.index(el) + 1
            if name == "first-child" and pos != 1:
                return False
            if name == "last-child" and pos != len(sib):
                return False
            if name == "only-child" and len(sib) != 1:
                return False
            if name == "empty" and any(c.kind in ("element", "text") for c in el.children):
                return False
            if name == "nth-child" and not nth_ok(s[2][0], s[2][1], pos):
                return False
            if name == "nth-last-child" and not nth_ok(s[2][0], s[2][1], len(sib) - pos + 1):
                return False
            if name == "not" and match_compound(el, s[2]):
                return False
    return True


def match_complex(el, parts, k):
    comb, comp = parts[k]
    if not match_compound(el, comp):
        return False
    if k == 0:
        return True
    if comb == ">":
        p = el.parent
        return p is not None and p.kind == "element" and match_complex(p, parts, k - 1)
    if comb == " ":
        p = el.parent
        while p is not None and p.kind == "element":
            if match_complex(p, parts, k - 1):
                return True
            p = p.parent
        return False
    sib = element_siblings(el)
    idx = sib.index(el)
    if comb == "+":
        return idx > 0 and match_complex(sib[idx - 1], parts, k - 1)
    return any(match_complex(x, parts, k - 1) for x in sib[:idx])


def query(doc, selector):
    lst = SelParser(selector).parse_list()
    return [el for el in doc.elements() if any(match_complex(el, parts, len(parts) - 1) for parts in lst)]


# ---------------------------------------------------------------- meta and tables

def collapse(s):
    return re.sub("[\\t\\n\\f\\r ]+", " ", s).strip(" ")


def extract_meta(doc):
    out = {"title": None, "description": None, "canonical": None, "lang": None, "og": []}
    seen_title = False
    for el in doc.elements():
        if el.name == "title" and not seen_title:
            seen_title = True
            t = collapse("".join(c.data for c in el.children if c.kind == "text"))
            out["title"] = t or None
        elif el.name == "html" and out["lang"] is None and el.attr("lang") is not None:
            out["lang"] = el.attr("lang")
        elif el.name == "meta":
            name = lower(el.attr("name") or "")
            prop = el.attr("property") or ""
            content = el.attr("content")
            if content is None:
                continue
            if name == "description" and out["description"] is None:
                out["description"] = content
            if lower(prop).startswith("og:"):
                out["og"].append([prop[3:], content])
        elif el.name == "link" and out["canonical"] is None:
            rels = split_ws(lower(el.attr("rel") or ""))
            if "canonical" in rels and el.attr("href") is not None:
                out["canonical"] = el.attr("href")
    return out


def parse_span(v, default, maximum):
    if v is None:
        return default
    m = re.match("[\\t\\n\\f\\r ]*\\+?([0-9]+)", v)
    if not m:
        return default
    n = int(m.group(1)) if len(m.group(1)) <= 9 else maximum
    if n == 0:
        return default
    return min(n, maximum)


MAX_CELLS = 1000000


def extract_tables(doc):
    tables = []
    for table in (el for el in doc.elements() if el.name == "table"):
        rows = []

        def collect(n):
            for c in n.children:
                if c.kind != "element" or c.name == "table":
                    continue
                if c.name == "tr":
                    rows.append(c)
                else:
                    collect(c)
        collect(table)
        grid = []
        pending = {}
        total = 0
        stop = False
        for tr in rows:
            row = {}
            for col, (left, text) in pending.items():
                row[col] = text
            new_pending = {col: (left - 1, text) for col, (left, text) in pending.items() if left > 1}
            col = 0
            for cell in (c for c in tr.children if c.kind == "element" and c.name in ("td", "th")):
                text = collapse(text_content(cell))
                cs = parse_span(cell.attr("colspan"), 1, 1000)
                rs = parse_span(cell.attr("rowspan"), 1, 65534)
                for _ in range(cs):
                    while col in row:
                        col += 1
                    total += 1
                    if total > MAX_CELLS:
                        stop = True
                        break
                    row[col] = text
                    if rs > 1:
                        new_pending[col] = (rs - 1, text)
                    col += 1
                if stop:
                    break
            pending = new_pending
            width = max(row) + 1 if row else 0
            grid.append([row.get(c, "") for c in range(width)])
            if stop:
                break
        tables.append(grid)
    return tables
