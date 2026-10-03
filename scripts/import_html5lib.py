#!/usr/bin/env python3
"""Converts html5lib-tests tokenizer tests into conformance/html5lib-tokenizer.json.

    python3 scripts/import_html5lib.py /path/to/html5lib-tests

Pinned to html5lib-tests commit 224991ec10db04f056a89eed8b0bd8695fd2950e (the
commit before processing-instruction tokens were added on 2026-09-29). Tests whose
input or output contains a lone surrogate are skipped: such strings cannot be
represented in Rust or Go. Start tags are rewritten to the LombokHTML token form
["StartTag", name, [[attr, value], ...], selfClosing] so every port compares
directly. html5lib-tests is MIT licensed (conformance/LICENSE-html5lib-tests).
"""
import glob
import json
import re
import sys
from pathlib import Path

COMMIT = "224991ec10db04f056a89eed8b0bd8695fd2950e"
STATES = {"Data state": "data", "PLAINTEXT state": "plaintext", "RCDATA state": "rcdata",
          "RAWTEXT state": "rawtext", "Script data state": "script", "CDATA section state": "cdata"}


def unesc(s):
    return re.sub(r"\\u([0-9A-Fa-f]{4})", lambda m: chr(int(m.group(1), 16)), s)


def deep(v):
    if isinstance(v, str):
        return unesc(v)
    if isinstance(v, list):
        return [deep(x) for x in v]
    if isinstance(v, dict):
        return {unesc(k): deep(x) for k, x in v.items()}
    return v


def lombok_form(out):
    res = []
    for t in out:
        if t[0] == "StartTag":
            res.append(["StartTag", t[1], [[k, v] for k, v in t[2].items()], len(t) > 3 and bool(t[3])])
        else:
            res.append(t)
    return res


def main():
    root = Path(sys.argv[1])
    cases = []
    skipped = 0
    for f in sorted(glob.glob(str(root / "tokenizer" / "*.test"))):
        for t in json.load(open(f, encoding="utf-8")).get("tests", []):
            inp, out = t["input"], t["output"]
            if t.get("doubleEscaped"):
                inp, out = unesc(inp), deep(out)
            if any(0xD800 <= ord(c) <= 0xDFFF for c in inp + json.dumps(out, ensure_ascii=False)):
                skipped += 1
                continue
            for st in t.get("initialStates", ["Data state"]):
                c = {"file": Path(f).name, "description": t["description"], "state": STATES[st], "input": inp,
                     "output": lombok_form(out)}
                if "lastStartTag" in t:
                    c["lastStartTag"] = t["lastStartTag"]
                cases.append(c)
    doc = {"source": "https://github.com/html5lib/html5lib-tests", "commit": COMMIT, "license": "MIT",
           "skipped_surrogate_tests": skipped, "cases": cases}
    dest = Path(__file__).resolve().parent.parent / "conformance" / "html5lib-tokenizer.json"
    dest.write_text(json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    print(len(cases), "cases,", skipped, "skipped")


if __name__ == "__main__":
    main()
