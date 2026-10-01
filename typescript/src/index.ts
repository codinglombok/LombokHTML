// LombokHTML — TypeScript port (zero dependencies). Behaviour is defined by
// SPEC_LombokHTML and verified against ../vectors/lombokhtml-vectors-v1.json.
// Input is a JS string (UTF-16); all scanning is done on Unicode code points so
// results are identical to the Rust reference.

import { asciiLower, chars, isRustWhitespace, rustTrim } from "./compat.js";

// ───────────────────────────── entities ─────────────────────────────

const NAMED: Record<string, string> = {
  amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0", copy: "\u00a9", reg: "\u00ae", trade: "\u2122",
  hellip: "\u2026", mdash: "\u2014", ndash: "\u2013", lsquo: "\u2018", rsquo: "\u2019", ldquo: "\u201c", rdquo: "\u201d",
  laquo: "\u00ab", raquo: "\u00bb", bull: "\u2022", middot: "\u00b7", euro: "\u20ac", pound: "\u00a3", yen: "\u00a5",
  cent: "\u00a2", times: "\u00d7", divide: "\u00f7", deg: "\u00b0", plusmn: "\u00b1", sect: "\u00a7", para: "\u00b6",
};
function numeric(body: string): string | null {
  let digits = body, radix = 10;
  if (body.startsWith("x") || body.startsWith("X")) { digits = body.slice(1); radix = 16; }
  const re = radix === 16 ? /^[0-9a-fA-F]+$/ : /^[0-9]+$/;
  if (digits === "" || digits.length > 8 || !re.test(digits)) return null;
  const n = parseInt(digits, radix);
  if (n === 0 || n > 0x10ffff || (n >= 0xd800 && n <= 0xdfff)) return "\ufffd";
  return String.fromCodePoint(n);
}
export function decodeEntities(s: string): string {
  if (!s.includes("&")) return s;
  let out = "", rest = s;
  for (;;) {
    const pos = rest.indexOf("&");
    if (pos === -1) break;
    out += rest.slice(0, pos);
    const after = rest.slice(pos + 1);
    // ';' must occur within the first 34 code points after '&'
    let end = -1, count = 0;
    for (let i = 0; i < after.length && count < 34; ) {
      const cp = after.codePointAt(i)!;
      if (cp === 0x3b) { end = i; break; }
      i += cp > 0xffff ? 2 : 1; count++;
    }
    let decoded: string | null = null;
    if (end !== -1) {
      const body = after.slice(0, end);
      decoded = body.startsWith("#") ? numeric(body.slice(1)) : Object.prototype.hasOwnProperty.call(NAMED, body) ? NAMED[body]! : null;
    }
    if (decoded !== null) { out += decoded; rest = after.slice(end + 1); }
    else { out += "&"; rest = after; }
  }
  return out + rest;
}
export const escapeText = (s: string): string => s.replace(/[&<>]/g, (c) => (c === "&" ? "&amp;" : c === "<" ? "&lt;" : "&gt;"));
export const escapeAttr = (s: string): string => s.replace(/[&<>"]/g, (c) => (c === "&" ? "&amp;" : c === "<" ? "&lt;" : c === ">" ? "&gt;" : "&quot;"));

// ───────────────────────────── tokenizer ─────────────────────────────

export type Token =
  | { type: "start"; name: string; attrs: Array<[string, string]>; selfClosing: boolean }
  | { type: "end"; name: string } | { type: "text"; text: string } | { type: "comment"; text: string } | { type: "doctype"; text: string };

export const VOID_ELEMENTS = ["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"];
export const isVoidElement = (t: string): boolean => VOID_ELEMENTS.includes(t);
const RAW_TEXT = ["script", "style"];

const startsWith = (cs: string[], at: number, pat: string[]): boolean => pat.every((p, k) => cs[at + k] === p);
const isAsciiAlphaChar = (c: string | undefined): boolean => c !== undefined && /^[A-Za-z]$/.test(c);

function findTagEnd(cs: string[], from: number): number {
  let i = from, quote: string | null = null;
  while (i < cs.length) {
    const c = cs[i]!;
    if (quote !== null) { if (c === quote) quote = null; }
    else {
      if (c === ">") return i;
      if (c === "=") {
        let j = i + 1;
        while (j < cs.length && isRustWhitespace(cs[j]!)) j++;
        const q = cs[j];
        if (q === '"' || q === "'") { quote = q; i = j; }
      }
    }
    i++;
  }
  return cs.length;
}

function parseTagContents(raw: string): { name: string; attrs: Array<[string, string]>; selfClosing: boolean } {
  const cs = chars(raw);
  let i = 0;
  while (i < cs.length && isRustWhitespace(cs[i]!)) i++;
  const ns = i;
  while (i < cs.length && (/^[A-Za-z0-9]$/.test(cs[i]!) || cs[i] === "-" || cs[i] === ":")) i++;
  const name = asciiLower(cs.slice(ns, i).join(""));
  const attrs: Array<[string, string]> = [];
  let selfClosing = false;
  for (;;) {
    while (i < cs.length && isRustWhitespace(cs[i]!)) i++;
    if (i >= cs.length) break;
    if (cs[i] === "/") { selfClosing = true; i++; continue; }
    const as = i;
    while (i < cs.length && !isRustWhitespace(cs[i]!) && cs[i] !== "=" && cs[i] !== "/") i++;
    if (i === as) { i++; continue; }
    const attrName = asciiLower(cs.slice(as, i).join(""));
    while (i < cs.length && isRustWhitespace(cs[i]!)) i++;
    let value = "";
    if (cs[i] === "=") {
      i++;
      while (i < cs.length && isRustWhitespace(cs[i]!)) i++;
      const q = cs[i];
      if (q === '"' || q === "'") {
        i++;
        const vs = i;
        while (i < cs.length && cs[i] !== q) i++;
        value = cs.slice(vs, i).join("");
        if (i < cs.length) i++;
      } else if (q !== undefined) {
        const vs = i;
        while (i < cs.length && !isRustWhitespace(cs[i]!) && cs[i] !== "/") i++;
        value = cs.slice(vs, i).join("");
      }
    }
    attrs.push([attrName, decodeEntities(value)]);
  }
  return { name, attrs, selfClosing };
}

export function tokenize(html: string): Token[] {
  const cs = chars(html), tokens: Token[] = [];
  let i = 0, buf = "";
  const flush = () => { if (buf !== "") { tokens.push({ type: "text", text: decodeEntities(buf) }); buf = ""; } };
  const C_START = ["<", "!", "-", "-"], C_END = ["-", "-", ">"];
  while (i < cs.length) {
    if (cs[i] === "<") {
      if (startsWith(cs, i, C_START)) {
        flush();
        const start = i + 4;
        let end = start;
        while (end < cs.length && !startsWith(cs, end, C_END)) end++;
        tokens.push({ type: "comment", text: cs.slice(start, end).join("") });
        i = end < cs.length ? end + 3 : cs.length;
        continue;
      }
      if (asciiLower(cs.slice(i, i + 9).join("")) === "<!doctype") {
        flush();
        let end = i;
        while (end < cs.length && cs[end] !== ">") end++;
        tokens.push({ type: "doctype", text: cs.slice(i, Math.min(end, cs.length)).join("") });
        i = end < cs.length ? end + 1 : cs.length;
        continue;
      }
      if (cs[i + 1] === "/") {
        flush();
        let end = i + 2;
        while (end < cs.length && cs[end] !== ">") end++;
        const name = asciiLower(rustTrim(cs.slice(i + 2, Math.min(end, cs.length)).join("")));
        if (name !== "") tokens.push({ type: "end", name });
        i = end < cs.length ? end + 1 : cs.length;
        continue;
      }
      if (isAsciiAlphaChar(cs[i + 1])) {
        flush();
        const ts = i + 1, end = findTagEnd(cs, ts);
        const { name, attrs, selfClosing } = parseTagContents(cs.slice(ts, Math.min(end, cs.length)).join(""));
        tokens.push({ type: "start", name, attrs, selfClosing });
        i = end < cs.length ? end + 1 : cs.length;
        if (RAW_TEXT.includes(name) && !selfClosing) {
          const closePat = chars("</" + name);
          let j = i, found = -1;
          while (j < cs.length) { if (startsWith(cs, j, closePat)) { found = j; break; } j++; }
          const contentEnd = found === -1 ? cs.length : found;
          const raw = cs.slice(i, contentEnd).join("");
          if (raw !== "") tokens.push({ type: "text", text: raw });
          if (found !== -1) {
            let k = found;
            while (k < cs.length && cs[k] !== ">") k++;
            tokens.push({ type: "end", name });
            i = k < cs.length ? k + 1 : cs.length;
          } else i = cs.length;
        }
        continue;
      }
      buf += "<"; i++;
      continue;
    }
    buf += cs[i]; i++;
  }
  flush();
  return tokens;
}

// ───────────────────────────── DOM ─────────────────────────────

export type NodeKind = "document" | "element" | "text" | "comment";
export class Node {
  constructor(public kind: NodeKind, public tag: string | null = null, public attrs: Array<[string, string]> = [], public text: string | null = null, public children: Node[] = []) {}
  attr(name: string): string | undefined { return this.attrs.find(([k]) => k === name)?.[1]; }
  isElement(tag: string): boolean { return this.kind === "element" && this.tag === tag; }
  findFirst(tag: string): Node | undefined {
    if (this.isElement(tag)) return this;
    for (const c of this.children) { const f = c.findFirst(tag); if (f) return f; }
    return undefined;
  }
  findAll(tag: string): Node[] {
    const out: Node[] = [];
    for (const c of this.children) { if (c.isElement(tag)) out.push(c); out.push(...c.findAll(tag)); }
    return out;
  }
}
export const MAX_DEPTH = 256;

export function parse(html: string): Node {
  const root = new Node("document"), stack: Node[] = [];
  const push = (n: Node) => (stack.length ? stack[stack.length - 1]! : root).children.push(n);
  for (const t of tokenize(html)) {
    switch (t.type) {
      case "doctype": break;
      case "comment": push(new Node("comment", null, [], t.text)); break;
      case "text": push(new Node("text", null, [], t.text)); break;
      case "start": {
        const n = new Node("element", t.name, t.attrs);
        if (t.selfClosing || isVoidElement(t.name) || stack.length >= MAX_DEPTH) push(n); else stack.push(n);
        break;
      }
      case "end": {
        let pos = -1;
        for (let k = stack.length - 1; k >= 0; k--) if (stack[k]!.tag === t.name) { pos = k; break; }
        if (pos !== -1) while (stack.length > pos) push(stack.pop()!);
        break;
      }
    }
  }
  while (stack.length) push(stack.pop()!);
  return root;
}

// ───────────────────────────── text extraction ─────────────────────────────

const invisible = (tag: string): boolean => tag === "script" || tag === "style" || tag === "noscript" || tag === "template";
function collectText(n: Node, out: string[]): void {
  if (n.tag !== null && invisible(n.tag)) return;
  if (n.kind === "text") { out.push(n.text ?? ""); return; }
  for (const c of n.children) collectText(c, out);
}
export function stripTags(html: string): string { const out: string[] = []; collectText(parse(html), out); return out.join(""); }

const HEADERS: Record<string, string> = { h1: "# ", h2: "## ", h3: "### ", h4: "#### ", h5: "##### ", h6: "###### " };
function walkStructured(n: Node, out: string[]): void {
  if (n.tag !== null) {
    if (invisible(n.tag)) return;
    if (Object.prototype.hasOwnProperty.call(HEADERS, n.tag)) {
      out.push("\n", HEADERS[n.tag]!); for (const c of n.children) walkStructured(c, out); out.push("\n"); return;
    }
    switch (n.tag) {
      case "li": out.push("\n- "); for (const c of n.children) walkStructured(c, out); return;
      case "br": out.push("\n"); return;
      case "p": case "div": case "tr": case "ul": case "ol": case "table":
        out.push("\n"); for (const c of n.children) walkStructured(c, out); out.push("\n"); return;
      case "td": case "th": for (const c of n.children) walkStructured(c, out); out.push("\t"); return;
    }
  }
  if (n.kind === "text") out.push(n.text ?? "");
  else for (const c of n.children) walkStructured(c, out);
}
export function extractStructuredText(html: string): string {
  const out: string[] = [];
  walkStructured(parse(html), out);
  let collapsed = "", run = 0;
  for (const c of out.join("")) {
    if (c === "\n") { run++; if (run <= 2) collapsed += c; } else { run = 0; collapsed += c; }
  }
  return rustTrim(collapsed);
}

// ───────────────────────────── sanitizer ─────────────────────────────

export interface Policy {
  allowedTags: string[]; dropWithContent: string[]; globalAttrs: string[]; tagAttrs: Array<[string, string]>; urlAttrs: string[]; allowedSchemes: string[];
}
export function defaultPolicy(): Policy {
  const w = (s: string) => s.split(" ");
  return {
    allowedTags: w("a abbr b blockquote br caption cite code dd del dfn div dl dt em figcaption figure h1 h2 h3 h4 h5 h6 hr i img ins kbd li mark ol p pre q s samp small span strong sub sup table tbody td tfoot th thead time tr u ul"),
    dropWithContent: w("script style iframe frame frameset object embed applet svg math template noscript head title select option textarea button audio video canvas base link meta"),
    globalAttrs: w("title lang dir"),
    tagAttrs: [["a", "href"], ["img", "src"], ["img", "alt"], ["img", "width"], ["img", "height"], ["td", "colspan"], ["td", "rowspan"], ["th", "colspan"], ["th", "rowspan"], ["th", "scope"], ["ol", "start"], ["blockquote", "cite"], ["q", "cite"], ["time", "datetime"]],
    urlAttrs: w("href src cite"),
    allowedSchemes: w("http https mailto tel"),
  };
}
/** True if `value` is a relative URL or uses an allowed scheme (browser-style whitespace/control stripping). */
export function isSafeUrl(value: string, allowedSchemes: string[]): boolean {
  const cleaned = asciiLower(chars(value).filter((c) => !(c.codePointAt(0)! <= 0x20 || c === "\u007f")).join(""));
  const i = cleaned.search(/[:/?#]/);
  if (i !== -1 && cleaned[i] === ":") return allowedSchemes.includes(cleaned.slice(0, i));
  return true;
}
function clean(n: Node, p: Policy, out: string[]): void {
  switch (n.kind) {
    case "text": out.push(escapeText(n.text ?? "")); return;
    case "comment": return;
    case "document": for (const c of n.children) clean(c, p, out); return;
    case "element": {
      const tag = n.tag ?? "";
      if (p.dropWithContent.includes(tag)) return;
      if (!p.allowedTags.includes(tag)) { for (const c of n.children) clean(c, p, out); return; }
      out.push("<", tag);
      const seen: string[] = [];
      for (const [k, v] of n.attrs) {
        if (seen.includes(k)) continue;
        if (!(p.globalAttrs.includes(k) || p.tagAttrs.some(([t, a]) => t === tag && a === k))) continue;
        if (p.urlAttrs.includes(k) && !isSafeUrl(v, p.allowedSchemes)) continue;
        seen.push(k);
        out.push(" ", k, '="', escapeAttr(v), '"');
      }
      if (isVoidElement(tag)) out.push(" />");
      else { out.push(">"); for (const c of n.children) clean(c, p, out); out.push("</", tag, ">"); }
    }
  }
}
export function sanitizeWith(html: string, policy: Policy): string { const out: string[] = []; clean(parse(html), policy, out); return out.join(""); }
export const sanitize = (html: string): string => sanitizeWith(html, defaultPolicy());

// ───────────────────────────── selectors ─────────────────────────────

export type AttrOp = "exists" | "equals" | "prefix" | "suffix" | "contains" | "word";
export interface Selector { tag?: string; id?: string; classes: string[]; attrs: Array<[string, AttrOp, string]> }
export type Combinator = "descendant" | "child";
export interface Complex { parts: Array<[Combinator, Selector]> }
const isIdent = (c: string): boolean => /^[A-Za-z0-9_-]$/.test(c);

export function parseSelector(sel: string): Selector {
  const s: Selector = { classes: [], attrs: [] };
  const cs = chars(rustTrim(sel));
  let i = 0;
  if (cs[0] === "*") i = 1;
  else {
    while (i < cs.length && isIdent(cs[i]!)) i++;
    if (i > 0) s.tag = asciiLower(cs.slice(0, i).join(""));
  }
  while (i < cs.length) {
    const c = cs[i]!;
    if (c === "#" || c === ".") {
      i++;
      const st = i;
      while (i < cs.length && isIdent(cs[i]!)) i++;
      const name = cs.slice(st, i).join("");
      if (c === "#") s.id = name; else s.classes.push(name);
    } else if (c === "[") {
      i++;
      const st = i;
      while (i < cs.length && !["=", "]", "^", "$", "*", "~"].includes(cs[i]!)) i++;
      const name = asciiLower(rustTrim(cs.slice(st, i).join("")));
      let op: AttrOp = "exists";
      const cur = cs[i];
      if (cur === "=") { op = "equals"; i++; }
      else if ((cur === "^" || cur === "$" || cur === "*" || cur === "~") && cs[i + 1] === "=") {
        op = cur === "^" ? "prefix" : cur === "$" ? "suffix" : cur === "*" ? "contains" : "word";
        i += 2;
      }
      let value = "";
      if (op !== "exists") {
        let quote: string | undefined;
        if (cs[i] === '"' || cs[i] === "'") { quote = cs[i]; i++; }
        const vs = i;
        while (i < cs.length && cs[i] !== "]" && cs[i] !== quote) i++;
        value = cs.slice(vs, i).join("");
        if (quote !== undefined && i < cs.length) i++;
      }
      while (i < cs.length && cs[i] !== "]") i++;
      i++;
      s.attrs.push([name, op, value]);
    } else i++;
  }
  return s;
}
const splitWhitespace = (s: string): string[] => {
  const out: string[] = []; let cur = "";
  for (const c of chars(s)) { if (isRustWhitespace(c)) { if (cur) { out.push(cur); cur = ""; } } else cur += c; }
  if (cur) out.push(cur);
  return out;
};
export function parseSelectorList(sel: string): Complex[] {
  return sel.split(",").map((part) => {
    const cx: Complex = { parts: [] };
    let pendingChild = false;
    for (const tok of splitWhitespace(part.split(">").join(" > "))) {
      if (tok === ">") { pendingChild = true; continue; }
      cx.parts.push([pendingChild ? "child" : "descendant", parseSelector(tok)]);
      pendingChild = false;
    }
    return cx;
  }).filter((c) => c.parts.length > 0);
}
function attrMatches(n: Node, name: string, op: AttrOp, expected: string): boolean {
  const actual = n.attr(name);
  if (actual === undefined) return false;
  switch (op) {
    case "exists": return true;
    case "equals": return actual === expected;
    case "prefix": return expected !== "" && actual.startsWith(expected);
    case "suffix": return expected !== "" && actual.endsWith(expected);
    case "contains": return expected !== "" && actual.includes(expected);
    case "word": return splitWhitespace(actual).includes(expected);
  }
}
function compoundMatches(n: Node, s: Selector): boolean {
  if (n.kind !== "element") return false;
  if (s.tag !== undefined && n.tag !== s.tag) return false;
  if (s.id !== undefined && n.attr("id") !== s.id) return false;
  if (s.classes.length) { const cls = splitWhitespace(n.attr("class") ?? ""); if (!s.classes.every((c) => cls.includes(c))) return false; }
  return s.attrs.every(([nm, op, v]) => attrMatches(n, nm, op, v));
}
function complexMatches(parts: Array<[Combinator, Selector]>, idx: number, n: Node, anc: Node[]): boolean {
  if (!compoundMatches(n, parts[idx]![1])) return false;
  if (idx === 0) return true;
  if (parts[idx]![0] === "child") return anc.length > 0 && complexMatches(parts, idx - 1, anc[anc.length - 1]!, anc.slice(0, -1));
  for (let k = anc.length - 1; k >= 0; k--) if (complexMatches(parts, idx - 1, anc[k]!, anc.slice(0, k))) return true;
  return false;
}
export function query(root: Node, selector: string): Node[] {
  const list = parseSelectorList(selector), out: Node[] = [];
  if (!list.length) return out;
  const anc: Node[] = [];
  const walk = (n: Node) => {
    for (const c of n.children) {
      if (c.kind === "element" && list.some((cx) => complexMatches(cx.parts, cx.parts.length - 1, c, anc))) out.push(c);
      if (c.kind === "element") { anc.push(c); walk(c); anc.pop(); }
    }
  };
  walk(root);
  return out;
}

// ───────────────────────────── meta ─────────────────────────────

export interface PageMeta { title: string | null; description: string | null; ogTags: Array<[string, string]> }
export function extractMeta(doc: Node): PageMeta {
  const m: PageMeta = { title: null, description: null, ogTags: [] };
  const t = doc.findFirst("title");
  if (t) { let text = ""; for (const c of t.children) if (c.text !== null) text += c.text; if (text !== "") m.title = text; }
  for (const el of doc.findAll("meta")) {
    const name = el.attr("name");
    if (name !== undefined && asciiLower(name) === "description") { const c = el.attr("content"); if (c !== undefined) m.description = c; }
    const prop = el.attr("property");
    if (prop !== undefined && prop.startsWith("og:")) { const c = el.attr("content"); if (c !== undefined) m.ogTags.push([prop.slice(3), c]); }
  }
  return m;
}

// ───────────────────────────── tables ─────────────────────────────

export type Row = string[];
export type Table = Row[];
const MAX_COLSPAN = 1000, MAX_ROWSPAN = 65534, MAX_CELLS = 1_000_000;
function cellText(cell: Node): string {
  const parts: string[] = [];
  const collect = (n: Node) => { if (n.text !== null) parts.push(n.text, " "); for (const c of n.children) collect(c); };
  for (const c of cell.children) collect(c);
  return splitWhitespace(parts.join("")).join(" ");
}
function span(cell: Node, attr: string, max: number): number {
  const v = cell.attr(attr);
  if (v === undefined) return 1;
  const t = rustTrim(v);
  if (!/^\+?[0-9]+$/.test(t)) return 1;             // Rust usize::parse accepts a leading '+'
  const n = BigInt(t);                              // usize::parse fails above u64::MAX -> default 1
  if (n > 18446744073709551615n || n === 0n) return 1;
  return n > BigInt(max) ? max : Number(n);
}
function collectRows(n: Node, rows: Node[]): void {
  for (const c of n.children) {
    if (c.isElement("table")) continue;
    if (c.isElement("tr")) rows.push(c); else collectRows(c, rows);
  }
}
export function extractTables(doc: Node): Table[] { return doc.findAll("table").map(extractOne); }
function extractOne(table: Node): Table {
  const trs: Node[] = []; collectRows(table, trs);
  const grid: Table = [];
  const pending: Array<[number, string] | null> = [];
  let total = 0;
  for (const tr of trs) {
    const row: Row = []; let col = 0;
    const fill = () => {
      for (;;) {
        const p = pending[col];
        if (!p) break;
        row.push(p[1]); p[0]--;
        if (p[0] === 0) pending[col] = null;
        col++;
      }
    };
    for (const cell of tr.children.filter((c) => c.isElement("td") || c.isElement("th"))) {
      fill();
      const text = cellText(cell), cs = span(cell, "colspan", MAX_COLSPAN), rs = span(cell, "rowspan", MAX_ROWSPAN);
      for (let k = 0; k < cs; k++) {
        if (++total > MAX_CELLS) return grid;
        row.push(text);
        if (rs > 1) { while (pending.length <= col) pending.push(null); pending[col] = [rs - 1, text]; }
        col++;
      }
    }
    fill();
    while (col < pending.length && pending[col]) fill();
    grid.push(row);
  }
  return grid;
}
