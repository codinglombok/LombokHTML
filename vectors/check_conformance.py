#!/usr/bin/env python3
"""Runs the reference model against conformance/html5lib-tokenizer.json."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import htmlmodel  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent


def as_json(tokens):
    return [["StartTag", t[1], [list(a) for a in t[2]], t[3]] if t[0] == "StartTag" else list(t) for t in tokens]


def main():
    doc = json.loads((ROOT / "conformance" / "html5lib-tokenizer.json").read_text(encoding="utf-8"))
    bad = 0
    for c in doc["cases"]:
        got = as_json(htmlmodel.tokenize(c["input"], c["state"], c.get("lastStartTag")))
        if got != c["output"]:
            bad += 1
            if bad <= 10:
                print("FAIL", c["file"], c["description"])
    print("model: %d conformance cases, %d failures" % (len(doc["cases"]), bad))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
