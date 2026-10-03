"""Runs every case of vectors/lombokhtml-vectors-v1.json and the html5lib-tests tokenizer conformance file."""
import json
from pathlib import Path

import pytest

import lombokhtml as lh

ROOT = Path(__file__).resolve().parents[2]
DOC = json.loads((ROOT / "vectors" / "lombokhtml-vectors-v1.json").read_text(encoding="utf-8"))
CONF = json.loads((ROOT / "conformance" / "html5lib-tokenizer.json").read_text(encoding="utf-8"))


def _tok(t):
    if t[0] == "StartTag":
        return ["StartTag", t[1], [list(a) for a in t[2]], t[3]]
    return list(t)


def _policy(p):
    out = lh.Policy()
    for t in p.get("tags", []):
        out.allow_tag(t)
    for t, a in p.get("attrs", []):
        out.allow_attr(t, a)
    for s in p.get("schemes", []):
        out.allow_scheme(s)
    return out


def _run(kind, i):
    if kind == "decode":
        return lh.decode_entities(i["text"])
    if kind == "escapeText":
        return lh.escape_text(i["text"])
    if kind == "escapeAttr":
        return lh.escape_attr(i["text"])
    if kind == "tokenize":
        return [_tok(t) for t in lh.tokenize(i["html"])]
    if kind == "parse":
        return lh.parse(i["html"]).serialize()
    if kind == "textContent":
        return lh.strip_tags(i["html"])
    if kind == "extractText":
        return lh.html_to_text(i["html"])
    if kind == "sanitize":
        p = _policy(i.get("policy", {}))
        out = lh.sanitize(i["html"], p)
        assert lh.sanitize(out, p) == out
        return out
    if kind == "safeUrl":
        return lh.is_safe_url(i["url"], i["schemes"]) if "schemes" in i else lh.is_safe_url(i["url"])
    if kind == "query":
        return [e.serialize() for e in lh.parse(i["html"]).query(i["selector"])]
    if kind == "meta":
        return lh.parse(i["html"]).meta().to_dict()
    if kind == "tables":
        return lh.parse(i["html"]).tables()
    raise AssertionError(kind)


def test_vector_count():
    assert len(DOC["cases"]) >= 100


@pytest.mark.parametrize("case", DOC["cases"], ids=[c["id"] for c in DOC["cases"]])
def test_vector(case):
    try:
        got = {"ok": _run(case["kind"], case["input"])}
    except lh.SelectorError as e:
        got = {"error": e.code}
    assert got == case["expected"]


def test_conformance():
    failed = []
    for c in CONF["cases"]:
        got = [_tok(t) for t in lh.tokenize_state(c["input"], c["state"], c.get("lastStartTag"))]
        if got != c["output"]:
            failed.append("%s %s" % (c["file"], c["description"]))
    assert failed[:20] == []
    assert len(CONF["cases"]) > 7000
