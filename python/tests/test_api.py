"""Public API checks beyond the shared vectors."""
import pytest

import lombokhtml as lh


def test_document_navigation():
    doc = lh.parse("<div id=a><p>x</p><!--c--></div>")
    div = next(doc.elements())
    assert div.is_element("div")
    assert div.attr("id") == "a" and div.attr("class") is None
    assert div.serialize() == '<div id="a"><p>x</p><!--c--></div>'
    assert div.text_content() == "x" and div.extract_text() == "x"
    assert repr(div) == "Node('element', 'div')"
    assert lh.Node("element", "b").element_siblings()[0].name == "b"
    assert lh.parse("").children == []


def test_depth_limit():
    assert len(list(lh.parse("<div>" * (lh.MAX_DEPTH + 10)).elements())) == lh.MAX_DEPTH


def test_tokenizer_states():
    assert lh.tokenize_state("a</title>b", "rcdata", "title") == [
        ("Character", "a"), ("EndTag", "title"), ("Character", "b")]
    assert lh.tokenize_state("<x>", "plaintext") == [("Character", "<x>")]
    assert lh.tokenize_state("a]]>b", "cdata") == [("Character", "ab")]
    assert lh.tokenize("<!DOCTYPE html>") == [("DOCTYPE", "html", None, None, True)]
    with pytest.raises(ValueError):
        lh.tokenize_state("x", "nope")
    assert "script" in lh.STATES


def test_policy_builder():
    p = lh.Policy().allow_tag("IFRAME").allow_attr("IFRAME", "SRC").allow_scheme("FTP")
    assert lh.sanitize("<iframe src='ftp://x' srcdoc=y></iframe>", p) == '<iframe src="ftp://x"></iframe>'
    assert lh.sanitize("<script>x</script><b>y</b>") == "<b>y</b>"
    assert lh.is_safe_url("https://e.com")
    assert not lh.is_safe_url(" java\tscript:alert(1)")
    assert lh.is_safe_url("ftp://x", ["ftp"])
    assert len(lh.DEFAULT_SCHEMES) == 4


def test_selectors():
    doc = lh.parse("<ul><li class=a>1<li>2</ul>")
    sel = lh.Selector("li.a")
    assert sel.matches(list(doc.elements())[1])
    assert not sel.matches(doc)
    with pytest.raises(lh.SelectorError) as ei:
        lh.Selector("li >")
    assert ei.value.code == "BAD_SELECTOR" and ei.value.offset == 4
    assert str(ei.value) == "BAD_SELECTOR at character 4"
    assert len(doc.query("li:nth-last-child(-n+1)")) == 1
    assert len(doc.query("ul li ~ li")) == 1
    for bad in ['[a="x]', "li:nth-child(1", ":not(li", "li:nth-child(2n+)", "li:nth-child(2nx)", "[a^]", "li:not(x"]:
        with pytest.raises(lh.SelectorError):
            doc.query(bad)
    for good in ["[class|=a]", "li:nth-child(-2n+3)", "li:nth-child(n)", "li:nth-child(-n- 0)", "* > li"]:
        doc.query(good)


def test_meta_and_tables():
    m = lh.parse("<html lang=en><title> </title><link rel=canonical><link rel=canonical href=/c><meta content=x>").meta()
    assert m.to_dict() == {"title": None, "description": None, "canonical": "/c", "lang": "en", "og": []}
    t = lh.parse("<table><tr><td colspan=+2 rowspan=2>a<td colspan=x>b<tr><td colspan=0>c</table>").tables()
    assert t == [[["a", "a", "b"], ["a", "a", "c"]]]
    assert len(lh.parse("<table><tr><td colspan=99999999999 rowspan=0>z</table>").tables()[0][0]) == 1000
    assert lh.parse("<table><tr></table>").tables() == [[[]]]
    big = lh.parse("<table>" + "<tr><td colspan=1000>x" * 1001).tables()
    assert len(big[0]) == 1001 and big[0][1000] == []
    assert lh.MAX_CELLS == 1000000


def test_helpers():
    assert lh.decode_entities("&#x110000;&#0;&#xD800;&#128;&#;&bogus;") == "���€&#;&bogus;"
    assert lh.escape_attr('"<&\xa0>') == "&quot;&lt;&amp;&nbsp;&gt;"


def test_linear_on_large_inputs():
    import time
    start = time.time()
    doc = lh.parse("<ul>" + "<li>x" * 20000 + "</ul>")
    for sel in ["li:first-child", "li ~ li", "ul li + li", "li:nth-last-child(2)"]:
        doc.query(sel)
    for html in ["a" * 200000, '<a href="' + "a" * 200000 + '">', "<!--" + "a" * 200000, "1<" * 100000]:
        lh.tokenize(html)
    lh.html_to_text("<b>x</b> " * 20000)
    assert time.time() - start < 10
