// Cross-language conformance: every case in ../vectors/lombokhtml-vectors-v1.json.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import * as H from "../src/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const doc = JSON.parse(readFileSync(join(here, "../../../vectors/lombokhtml-vectors-v1.json"), "utf8")) as {
  cases: Array<{ fn: string; in: any; out: unknown }>;
};

function dump(n: H.Node): unknown {
  switch (n.kind) {
    case "text": return n.text ?? "";
    case "comment": return { "!": n.text ?? "" };
    case "element": return { t: n.tag, a: n.attrs.map(([k, v]) => [k, v]), c: n.children.map(dump) };
    case "document": return { d: n.children.map(dump) };
  }
}
function evalCase(fn: string, i: any): unknown {
  switch (fn) {
    case "parse": return dump(H.parse(i));
    case "strip_tags": return H.stripTags(i);
    case "extract_structured_text": return H.extractStructuredText(i);
    case "sanitize": return H.sanitize(i);
    case "decode_entities": return H.decodeEntities(i);
    case "is_safe_url": return H.isSafeUrl(i, H.defaultPolicy().allowedSchemes);
    case "extract_tables": return H.extractTables(H.parse(i));
    case "extract_meta": { const m = H.extractMeta(H.parse(i)); return { title: m.title, description: m.description, og: m.ogTags }; }
    case "query": return H.query(H.parse(i.html), i.selector).map((n) => ({ t: n.tag, a: n.attrs.map(([k, v]) => [k, v]) }));
    default: throw new Error("unknown fn " + fn);
  }
}

test(`conformance: ${doc.cases.length} vectors`, () => {
  const failures: string[] = [];
  doc.cases.forEach((c, n) => {
    let got: unknown;
    try { got = evalCase(c.fn, c.in); } catch (e) { got = "THROW " + (e as Error).message; }
    try { assert.deepStrictEqual(got, c.out); }
    catch { failures.push(`case ${n} ${c.fn} in=${JSON.stringify(c.in).slice(0, 100)}\n   expected=${JSON.stringify(c.out).slice(0, 160)}\n   got     =${JSON.stringify(got).slice(0, 160)}`); }
  });
  assert.equal(failures.length, 0, `${failures.length} failures:\n` + failures.slice(0, 5).join("\n"));
});

test("robustness: pathological inputs terminate without throwing", () => {
  const deep = "<div>".repeat(200_000);
  H.parse(deep); H.stripTags(deep); H.sanitize(deep); H.query(H.parse(deep), "div div");
  H.sanitize(`<p ${"a=1 ".repeat(50_000)}>x</p>`);
  H.parse("<a href=\"".repeat(20_000));
  H.stripTags("<".repeat(100_000));
});
