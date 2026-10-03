#!/usr/bin/env python3
"""Builds vectors/lombokhtml-vectors-v1.json from the reference model (vectors/htmlmodel.py).

Hand-written expectations are checked against the model and the build fails on
any mismatch. Every sanitizer case is also checked for idempotence
(sanitize(sanitize(x)) == sanitize(x)). The tokenizer itself is additionally
checked against html5lib-tests by vectors/check_conformance.py.

    python3 vectors/build_vectors.py   # writes the JSON and vectors/SHA256SUMS
"""
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import htmlmodel as m  # noqa: E402

CASES = []
COUNT = {}


def tokens_json(toks):
    out = []
    for t in toks:
        if t[0] == "StartTag":
            out.append(["StartTag", t[1], [list(a) for a in t[2]], t[3]])
        else:
            out.append(list(t))
    return out


def run(kind, i):
    try:
        if kind == "decode":
            return {"ok": m.decode_entities(i["text"])}
        if kind == "escapeText":
            return {"ok": m.escape_text(i["text"])}
        if kind == "escapeAttr":
            return {"ok": m.escape_attr(i["text"])}
        if kind == "tokenize":
            return {"ok": tokens_json(m.tokenize(i["html"], switch=True))}
        if kind == "parse":
            return {"ok": m.serialize(m.parse(i["html"]))}
        if kind == "textContent":
            return {"ok": m.text_content(m.parse(i["html"]))}
        if kind == "extractText":
            return {"ok": m.extract_text(m.parse(i["html"]))}
        if kind == "sanitize":
            out = m.sanitize(i["html"], i.get("policy"))
            if m.sanitize(out, i.get("policy")) != out:
                raise SystemExit("sanitizer not idempotent for %r: %r" % (i["html"], out))
            return {"ok": out}
        if kind == "safeUrl":
            return {"ok": m.is_safe_url(i["url"], i.get("schemes", m.DEFAULT_SCHEMES))}
        if kind == "query":
            return {"ok": [m.serialize(e) for e in m.query(m.parse(i["html"]), i["selector"])]}
        if kind == "meta":
            return {"ok": m.extract_meta(m.parse(i["html"]))}
        if kind == "tables":
            return {"ok": m.extract_tables(m.parse(i["html"]))}
    except m.SelectorError:
        return {"error": "BAD_SELECTOR"}
    raise ValueError(kind)


def case(kind, inp, want=None, note=""):
    got = run(kind, inp)
    if want is not None:
        w = want if isinstance(want, dict) and set(want) <= {"ok", "error"} and want else {"ok": want}
        if w != got:
            raise SystemExit("model disagrees with expectation in %s %r:\n got  %r\n want %r" % (kind, inp, got, w))
    COUNT[kind] = COUNT.get(kind, 0) + 1
    CASES.append({"id": "%s-%03d" % (kind, COUNT[kind]), "kind": kind, "note": note, "input": inp, "expected": got})


# ---------------------------------------------------------------- entities and escaping
for text, want in [("&amp;", "&"), ("&lt;&gt;&quot;&apos;", "<>\"'"), ("&copy 2026", "\u00a9 2026"),
                   ("&notit;", "\u00acit;"), ("&notin;", "\u2209"), ("&#65;&#x42;&#X43;", "ABC"), ("&#65", "A"),
                   ("&#x;", "&#x;"), ("&#;", "&#;"), ("&#0;", "\ufffd"), ("&#x110000;", "\ufffd"),
                   ("&#xD800;", "\ufffd"), ("&#128;", "\u20ac"), ("&#x9F;", "\u0178"), ("&bogus;", "&bogus;"),
                   ("&", "&"), ("&&amp;", "&&"), ("a & b", "a & b"), ("&NotNestedGreaterGreater;", "\u2aa2\u0338"),
                   ("&Afr;", "\U0001d504"), ("&ampx", "&x"), ("&AMP;", "&"), ("&nbsp;", "\xa0"),
                   ("&#99999999999999;", "\ufffd"), ("&CounterClockwiseContourIntegral;", "\u2233"), ("", ""),
                   ("no refs", "no refs"), ("&lt", "<"), ("&LT", "<"), ("&ltx", "<x")]:
    case("decode", {"text": text}, want)
for text, want in [("a<b>&c", "a&lt;b&gt;&amp;c"), ("\xa0", "&nbsp;"), ('"q"', '"q"')]:
    case("escapeText", {"text": text}, want)
for text, want in [('a"b', "a&quot;b"), ("<&>", "&lt;&amp;&gt;"), ("\xa0'", "&nbsp;'")]:
    case("escapeAttr", {"text": text}, want)

# ---------------------------------------------------------------- tokenizer (with element-driven state switching)
for html, want in [
        ("<p class=a>Hi</p>", [["StartTag", "p", [["class", "a"]], False], ["Character", "Hi"], ["EndTag", "p"]]),
        ("<A HREF='x' title=\"t\" checked>", [["StartTag", "a", [["href", "x"], ["title", "t"], ["checked", ""]], False]]),
        ("<br/>", [["StartTag", "br", [], True]]),
        ("<img src=a src=b>", [["StartTag", "img", [["src", "a"]], False]]),
        ("<!-- c -->", [["Comment", " c "]]),
        ("<!DOCTYPE html>", [["DOCTYPE", "html", None, None, True]]),
        ("<script>if (a<b) {}</script>x", [["StartTag", "script", [], False], ["Character", "if (a<b) {}"],
                                           ["EndTag", "script"], ["Character", "x"]]),
        ("<title><b>&amp;</b></title>", [["StartTag", "title", [], False], ["Character", "<b>&</b>"], ["EndTag", "title"]]),
        ("<style></STYLE >", [["StartTag", "style", [], False], ["EndTag", "style"]]),
        ("<textarea></textarea>", None), ("<xmp><b></xmp>", None), ("<plaintext></plaintext>", None),
        ("<noscript><b></noscript>", None), ("<script><!--<script></script>--></script>", None),
        ("a < b", [["Character", "a < b"]]), ("</>", []), ("</ p>", [["Comment", " p"]]), ("<?php x ?>", None),
        ("<a href=\"&amp;x&ampy\">", [["StartTag", "a", [["href", "&x&ampy"]], False]]),
        ("<a b='&notin'>", None), ("<p\x00>", None), ("a\r\nb\rc", [["Character", "a\nb\nc"]]),
        ("<![CDATA[x]]>", [["Comment", "[CDATA[x]]"]]), ("<div a=1/>", None), ("<!---->", [["Comment", ""]])]:
    case("tokenize", {"html": html}, want)

# ---------------------------------------------------------------- tree construction (serialized)
for html, want in [
        ("<p>a<p>b", "<p>a</p><p>b</p>"),
        ("<ul><li>a<li>b</ul>", "<ul><li>a</li><li>b</li></ul>"),
        ("<dl><dt>a<dd>b<dt>c</dl>", "<dl><dt>a</dt><dd>b</dd><dt>c</dt></dl>"),
        ("<p>a<div>b</div>", "<p>a</p><div>b</div>"),
        ("<h1>a<h2>b", "<h1>a</h1><h2>b</h2>"),
        ("<table><tr><td>1<td>2<tr><td>3</table>",
         "<table><tr><td>1</td><td>2</td></tr><tr><td>3</td></tr></table>"),
        ("<a href=x>1<a href=y>2</a>", '<a href="x">1</a><a href="y">2</a>'),
        ("<b>x</div>y</b>", "<b>xy</b>"),
        ("<b><i>x</b>y</i>", "<b><i>x</i></b>y"),
        ("</p>x", "x"),
        ("<br></br>", "<br><br>"),
        ("<pre>\nx\n</pre>", "<pre>x\n</pre>"),
        ("<textarea>\n\nx</textarea>", "<textarea>\nx</textarea>"),
        ("<select><option>a<option>b</select>", "<select><option>a</option><option>b</option></select>"),
        ("<img src=x><input><hr>", '<img src="x"><input><hr>'),
        ("<p>1 &lt; 2 &amp; 3&nbsp;4</p>", "<p>1 &lt; 2 &amp; 3&nbsp;4</p>"),
        ("<script>a<b</script>", "<script>a<b</script>"),
        ("<div title='a\"<>&'>x</div>", '<div title="a&quot;&lt;&gt;&amp;">x</div>'),
        ("<!DOCTYPE html><html><head><title>T</title></head><body>x</body></html>",
         "<html><head><title>T</title></head><body>x</body></html>"),
        ("<!-- keep -->", "<!-- keep -->"),
        ("<span>a<span>b", "<span>a<span>b</span></span>"),
        ("<ul><li><ul><li>x</ul><li>y</ul>", "<ul><li><ul><li>x</li></ul></li><li>y</li></ul>"),
        ("<div><p>a</span>b</div>", "<div><p>ab</p></div>"),
        ("<p><button><p>x</button>y", "<p><button><p>x</p></button>y</p>"),
        ("x<p>", "x<p></p>"), ("", ""), ("plain", "plain"),
        ("<table><tbody><tr><td>a</table>", "<table><tbody><tr><td>a</td></tr></tbody></table>"),
        ("<option>a<optgroup>b", "<option>a</option><optgroup>b</optgroup>")]:
    case("parse", {"html": html}, want)
deep = "<div>" * 300 + "x" + "</div>" * 300
case("parse", {"html": deep}, "<div>" * 256 + "x" + "</div>" * 256, "depth capped at 256")

# ---------------------------------------------------------------- text
for html, want in [("<p>Hello <b>world</b></p><script>x()</script><style>a{}</style>", "Hello world"),
                   ("<title>T</title>a<noscript>n</noscript><template>t</template>", "a"),
                   ("a<!-- c -->b", "ab"), ("&lt;tag&gt;", "<tag>")]:
    case("textContent", {"html": html}, want)
for html, want in [
        ("<h1>Title</h1><p>One  two\nthree</p><ul><li>a</li><li>b</li></ul>", "# Title\n\nOne two three\n\n- a\n- b"),
        ("<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>", "A\tB\n1\t2"),
        ("a<br>b", "a\nb"), ("<p>a</p><hr><p>b</p>", "a\n\n---\n\nb"),
        ("<pre>  x\n    y</pre>", "  x\n    y"), ("<div><div><p>deep</p></div></div>", "deep"),
        ("<h3>x</h3>", "### x"), ("<dl><dt>t</dt><dd>d</dd></dl>", "t\nd"),
        ("  lead  <span> mid </span> trail  ", "lead mid trail"), ("<p></p><p></p>", ""),
        ("<script>x</script>", ""), ("<ol><li>one<li>two</ol>", "- one\n- two")]:
    case("extractText", {"html": html}, want)

# ---------------------------------------------------------------- sanitizer
for html, want in [
        ("<p onclick='x()'>Hi</p>", "<p>Hi</p>"),
        ("<script>alert(1)</script>ok", "ok"),
        ("<a href='javascript:alert(1)'>x</a>", "<a>x</a>"),
        ("<a href=' JaVaScRiPt:alert(1)'>x</a>", "<a>x</a>"),
        ("<a href='java\tscript:alert(1)'>x</a>", "<a>x</a>"),
        ("<a href='jav&#x09;ascript:alert(1)'>x</a>", "<a>x</a>"),
        ("<a href='&#106;avascript:alert(1)'>x</a>", "<a>x</a>"),
        ("<a href='vbscript:x'>x</a>", "<a>x</a>"),
        ("<a href='data:text/html,<script>1</script>'>x</a>", "<a>x</a>"),
        ("<a href='https://e.com/?q=1#f'>x</a>", '<a href="https://e.com/?q=1#f">x</a>'),
        ("<a href='/rel'>x</a>", '<a href="/rel">x</a>'),
        ("<a href='page?x=a:b'>x</a>", '<a href="page?x=a:b">x</a>'),
        ("<a href='mailto:a@b.co'>x</a>", '<a href="mailto:a@b.co">x</a>'),
        ("<img src=x onerror=alert(1)>", '<img src="x">'),
        ("<img src='javascript:x'>", "<img>"),
        ("<iframe src=x>inner</iframe>after", "after"),
        ("<svg><script>1</script></svg>t", "t"),
        ("<math><mi>x</mi></math>t", "t"),
        ("<style>*{}</style><p style='color:red'>c</p>", "<p>c</p>"),
        ("<custom-el>keep text</custom-el>", "keep text"),
        ("<section><article>x</article></section>", "x"),
        ("<p title='a&quot;b' class=c id=d>t</p>", '<p title="a&quot;b">t</p>'),
        ("<!-- secret -->x", "x"),
        ("<noscript><p title=\"</noscript><img src=x onerror=alert(1)>\"></noscript>", '<img src="x">"&gt;'),
        ("<textarea><script>1</script></textarea>x", "x"),
        ("<xmp><script>1</script></xmp>x", "x"),
        ("<plaintext><script>1</script>", ""),
        ("<template><script>1</script></template>x", "x"),
        ("<table><td colspan=2 onclick=x>a</table>", '<table><td colspan="2">a</td></table>'),
        ("<p>1 &lt; 2</p>", "<p>1 &lt; 2</p>"),
        ("<a href=\"http://x\" title='<script>'>x</a>", '<a href="http://x" title="&lt;script&gt;">x</a>'),
        ("<div lang=en dir=rtl>x</div>", '<div lang="en" dir="rtl">x</div>'),
        ("<form><input value=x><button>b</button></form>t", "t"),
        ("<base href=evil><link rel=stylesheet href=x><meta http-equiv=refresh>t", "t"),
        ("<object data=x>o</object><embed src=x>t", "t"),
        ("<a href=\"\x01javascript:1\">x</a>", "<a>x</a>"),
        ("<img src=\"https://e.com/a.png\" alt=\"A\" width=10>", '<img src="https://e.com/a.png" alt="A" width="10">'),
        ("<ul><li>a<li>b</ul>", "<ul><li>a</li><li>b</li></ul>"),
        ("<q cite='javascript:1'>q</q>", "<q>q</q>"),
        ("<blockquote cite='https://x'>q</blockquote>", '<blockquote cite="https://x">q</blockquote>'),
        ("x<br>y", "x<br>y")]:
    case("sanitize", {"html": html}, want)
case("sanitize", {"html": "<iframe src='https://x'>i</iframe><a href='ftp://x' class=c>f</a>",
                  "policy": {"tags": ["iframe"], "attrs": [["iframe", "src"], ["*", "class"]], "schemes": ["ftp"]}},
     '<iframe src="https://x"></iframe><a href="ftp://x" class="c">f</a>', "custom policy")
case("sanitize", {"html": "<iframe>a&amp;b<p>x</iframe>y", "policy": {"tags": ["iframe"]}}, "<iframe></iframe>y",
     "kept raw text element is written without content (idempotent)")
case("sanitize", {"html": "<style>p>a{color:red}</style><p>t</p>", "policy": {"tags": ["STYLE"]}}, "<style></style><p>t</p>",
     "kept style keeps no content")
case("sanitize", {"html": "<b>a</b><plaintext><i>b</i>", "policy": {"tags": ["plaintext"]}}, "<b>a</b>",
     "plaintext is never kept")
case("sanitize", {"html": "<a href='ftp://x'>f</a>"}, "<a>f</a>", "ftp not allowed by default")

for url, want in [("https://x", True), ("HTTP://X", True), ("/a", True), ("a/b:c", True), ("?q=a:b", True),
                  ("#f", True), ("", True), ("javascript:1", False), (" java\nscript:1", False), ("data:x", False),
                  ("blob:x", False), ("mailto:a", True), ("tel:1", True), ("x:y", False), ("\x7fjavascript:1", False)]:
    case("safeUrl", {"url": url}, want)
case("safeUrl", {"url": "ftp://x", "schemes": ["ftp"]}, True)

# ---------------------------------------------------------------- selectors
PAGE = ('<div id="main" class="box wide"><h1>T</h1><p class="lead">one</p><p>two</p>'
        '<a href="https://e.com/a.pdf" data-x="a b" lang="en-US">l</a><span></span></div>'
        '<ul><li>1</li><li>2</li><li>3</li><li>4</li></ul>')
for sel, want in [("h1", ["<h1>T</h1>"]), ("p", ['<p class="lead">one</p>', "<p>two</p>"]),
                  (".lead", ['<p class="lead">one</p>']), ("#main > h1", ["<h1>T</h1>"]),
                  ("div p", ['<p class="lead">one</p>', "<p>two</p>"]), ("h1 + p", ['<p class="lead">one</p>']),
                  ("h1 ~ p", ['<p class="lead">one</p>', "<p>two</p>"]), ("[href$='.pdf']", None),
                  ("a[href^=https]", None), ("a[href*='e.com']", None), ("[data-x~=b]", None), ("[lang|=en]", None),
                  ("[data-x='a']", []), ("span:empty", ["<span></span>"]), ("li:first-child", ["<li>1</li>"]),
                  ("li:last-child", ["<li>4</li>"]), ("li:nth-child(2n+1)", ["<li>1</li>", "<li>3</li>"]),
                  ("li:nth-child(even)", ["<li>2</li>", "<li>4</li>"]), ("li:nth-child(-n+2)", ["<li>1</li>", "<li>2</li>"]),
                  ("li:nth-last-child(1)", ["<li>4</li>"]), ("li:not(:first-child):not(:last-child)", ["<li>2</li>", "<li>3</li>"]),
                  ("h1, li:nth-child(3)", ["<h1>T</h1>", "<li>3</li>"]), (".box.wide > span", ["<span></span>"]),
                  ("DIV > H1", ["<h1>T</h1>"]), ("ul > *", None), ("p:only-child", []), ("table", []),
                  ("", E := {"error": "BAD_SELECTOR"}), ("p >", E), ("p,", E), ("[href", E), (":hover", E),
                  ("p!", E), ("li:nth-child(x)", E), ("a[href=]", E)]:
    case("query", {"html": PAGE, "selector": sel}, want)
UNI = '<p class="a\u00a0b">x</p><p class="a b">y</p><ul><li>1</li><li>2</li></ul>'
for sel, want in [(".a", ['<p class="a b">y</p>']), ("[class~=b]", ['<p class="a b">y</p>']),
                  ("li:nth-child( 2 )", ["<li>2</li>"]), ("li:nth-child(+n - 1)", ["<li>1</li>", "<li>2</li>"]),
                  ("li:nth-child(999999999)", []), ("li:nth-child(1234567890)", {"error": "BAD_SELECTOR"}),
                  ("li:nth-child(\u0661)", {"error": "BAD_SELECTOR"}), ("LI:FIRST-CHILD", ["<li>1</li>"]),
                  ("\u0130", [])]:
    case("query", {"html": UNI, "selector": sel}, want, "ASCII-only case folding, whitespace and digits")

# ---------------------------------------------------------------- meta
case("meta", {"html": '<html lang="id"><head><title>  Laporan\n Kuartal </title>'
                      '<meta name="Description" content="Ringkasan">'
                      '<meta property="og:title" content="Q3"><meta property="OG:image" content="/i.png">'
                      '<link rel="alternate canonical" href="https://e.com/q3"></head></html>'},
     {"title": "Laporan Kuartal", "description": "Ringkasan", "canonical": "https://e.com/q3", "lang": "id",
      "og": [["title", "Q3"], ["image", "/i.png"]]})
case("meta", {"html": "<p>no head</p>"}, {"title": None, "description": None, "canonical": None, "lang": None, "og": []})
case("meta", {"html": "<title></title><title>Second</title><meta name=description><meta name=description content=x>"},
     {"title": None, "description": "x", "canonical": None, "lang": None, "og": []})
case("meta", {"html": "<title>a &amp; b</title>"}, {"title": "a & b", "description": None, "canonical": None, "lang": None, "og": []})

# ---------------------------------------------------------------- tables
for html, want in [
        ("<table><tr><th>A<th>B<tr><td>1<td>2</table>", [[["A", "B"], ["1", "2"]]]),
        ("<table><tr><td rowspan=2>A<td>B<tr><td>C</table>", [[["A", "B"], ["A", "C"]]]),
        ("<table><tr><td colspan=2>A<td>B</table>", [[["A", "A", "B"]]]),
        ("<table><tr><td>a<table><tr><td>inner</table></td></tr></table>", [[["ainner"]], [["inner"]]]),
        ("<table><tr><td colspan=0>a<td rowspan=x>b</table>", [[["a", "b"]]]),
        ("<table><tr><td colspan=5000>a</table>", [[["a"] * 1000]]),
        ("<table><tr><td rowspan=3>A<td>B<tr><td>C<tr></table>", [[["A", "B"], ["A", "C"], ["A"]]]),
        ("<table><tr><td>a</td></tr><tr></tr></table>", [[["a"], []]]),
        ("<table><caption>c</caption><thead><tr><th> H </th></tr></thead><tbody><tr><td>x<br>y</td></tr></tbody></table>",
         [[["H"], ["xy"]]]),
        ("<table><tr><td> +3 </td><td colspan=' 2'>z</td></tr></table>", [[["+3", "z", "z"]]]),
        ("<p>none</p>", [])]:
    case("tables", {"html": html}, want)

doc = {"name": "lombokhtml-vectors", "version": 1, "spec": "docs/SPEC_LombokHTML_v0.2.0.md", "cases": CASES}
out = HERE / "lombokhtml-vectors-v1.json"
out.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
digest = hashlib.sha256(out.read_bytes()).hexdigest()
(HERE / "SHA256SUMS").write_text("%s  vectors/lombokhtml-vectors-v1.json\n" % digest, encoding="utf-8")
print(len(CASES), "cases", COUNT, digest)
