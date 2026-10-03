// LombokHTML: WHATWG HTML tokenizer, a small tree builder, serialization, text
// extraction, an allowlist sanitizer, a CSS selector subset, page metadata and
// table extraction, with byte-identical results in TypeScript, Rust, Python, Go
// and PHP (see docs/SPEC_LombokHTML_v0.2.0.md). No dependencies.
import { isAlnum, lookupNamed, MAX_ENTITY, numericChar, tokenize } from "./tokenizer.js";

export { tokenize, tokenizeState } from "./tokenizer.js";
export type { InitialState, Token } from "./tokenizer.js";

// ---------------------------------------------------------------- entities

/** Decodes character references as in text content (SPEC section 3.1). */
export function decodeEntities(text: string): string {
  const s = Array.from(text);
  const n = s.length;
  let out = "";
  let i = 0;
  while (i < n) {
    if (s[i] !== "&") {
      out += s[i++];
      continue;
    }
    const j = i + 1;
    if (s[j] === "#") {
      let k = j + 1;
      const hex = s[k] === "x" || s[k] === "X";
      if (hex) k++;
      const re = hex ? /^[0-9A-Fa-f]$/ : /^[0-9]$/;
      const start = k;
      let code = 0;
      while (k < n && re.test(s[k])) {
        code = Math.min(code * (hex ? 16 : 10) + parseInt(s[k], 16), 0x110000);
        k++;
      }
      if (k === start) {
        out += s.slice(i, k).join("");
        i = k;
        continue;
      }
      if (s[k] === ";") k++;
      out += numericChar(code);
      i = k;
      continue;
    }
    let k = j;
    while (k < n && isAlnum(s[k]) && k - j < MAX_ENTITY) k++;
    const run = s.slice(j, k).join("");
    let match: string | null = null;
    if (s[k] === ";" && lookupNamed(run + ";") !== undefined) match = run + ";";
    else {
      for (let m = run.length; m > 0; m--) {
        if (lookupNamed(run.slice(0, m)) !== undefined) {
          match = run.slice(0, m);
          break;
        }
      }
    }
    if (match === null) {
      out += "&" + run;
      i = k;
    } else {
      out += lookupNamed(match)!;
      i = j + match.length;
    }
  }
  return out;
}

/** Escapes text content: `&`, U+00A0, `<`, `>` (SPEC section 3.2). */
export function escapeText(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/ /g, "&nbsp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Escapes an attribute value: `&`, U+00A0, `"`, `<`, `>` (SPEC section 3.2). */
export function escapeAttr(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/ /g, "&nbsp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

// ---------------------------------------------------------------- tree

export type NodeKind = "document" | "element" | "text" | "comment";

/** A tree node. `name` and `attrs` are set for elements, `data` for text and comments. */
export class HtmlNode {
  parent: HtmlNode | null = null;
  readonly children: HtmlNode[] = [];

  constructor(
    readonly kind: NodeKind,
    readonly name: string = "",
    readonly attrs: [string, string][] = [],
    public data: string = "",
  ) {}

  /** Value of the first attribute called `name`. */
  attr(name: string): string | null {
    for (const [k, v] of this.attrs) if (k === name) return v;
    return null;
  }

  /** True for an element called `name`. */
  is(name: string): boolean {
    return this.kind === "element" && this.name === name;
  }

  /** Elements below this node in document order. */
  elements(): HtmlNode[] {
    const out: HtmlNode[] = [];
    const todo = [...this.children].reverse();
    while (todo.length > 0) {
      const n = todo.pop()!;
      if (n.kind === "element") {
        out.push(n);
        for (let i = n.children.length - 1; i >= 0; i--) todo.push(n.children[i]);
      }
    }
    return out;
  }

  /** Element children of the parent, this node included. */
  elementSiblings(): HtmlNode[] {
    return this.parent === null ? [this] : this.parent.children.filter((c) => c.kind === "element");
  }

  /** HTML serialization: outer HTML of an element, inner HTML of the document (SPEC section 5). */
  serialize(): string {
    const out: string[] = [];
    writeNode(this, out);
    return out.join("");
  }

  /** Text below this node, skipping script, style, noscript, template and title (SPEC section 6.1). */
  textContent(): string {
    const out: string[] = [];
    const walk = (n: HtmlNode): void => {
      if (n.kind === "text") out.push(n.data);
      else if (n.kind === "document" || (n.kind === "element" && !HIDDEN.has(n.name))) n.children.forEach(walk);
    };
    walk(this);
    return out.join("");
  }

  /** Structure-preserving plain text (SPEC section 6.2). */
  extractText(): string {
    return extractText(this);
  }

  /** Elements below this node matching `selector`, in document order (SPEC section 8). Throws {@link SelectorError}. */
  query(selector: string): HtmlNode[] {
    const sel = Selector.parse(selector);
    const ctx = new Ctx();
    return this.elements().filter((e) => sel.matchesIn(e, ctx));
  }

  /** Page metadata (SPEC section 9). */
  meta(): PageMeta {
    return extractMeta(this);
  }

  /** Every table as a grid of cell texts (SPEC section 10). */
  tables(): string[][][] {
    return extractTables(this);
  }
}

const set = (s: string): Set<string> => new Set(s.split(" "));
const VOID = set("area base basefont bgsound br col embed frame hr img input keygen link meta param source track wbr");
const RAW_PARENTS = set("style script xmp iframe noembed noframes plaintext noscript");
const SPECIAL = set(
  "address applet area article aside base basefont bgsound blockquote body br button caption center col colgroup dd " +
    "details dir div dl dt embed fieldset figcaption figure footer form frame frameset h1 h2 h3 h4 h5 h6 head header " +
    "hgroup hr html iframe img input keygen li link listing main marquee menu meta nav noembed noframes noscript " +
    "object ol p param plaintext pre script search section select source style summary table tbody td template " +
    "textarea tfoot th thead title tr track ul wbr xmp",
);
const P_CLOSERS = set(
  "address article aside blockquote center details dialog dir div dl fieldset figcaption figure footer form h1 h2 " +
    "h3 h4 h5 h6 header hgroup hr li dd dt listing main menu nav ol p plaintext pre search section summary table ul xmp",
);
const HEADINGS = set("h1 h2 h3 h4 h5 h6");
const SCOPE = set("applet caption html table td th marquee object template");
const BUTTON_SCOPE = new Set([...SCOPE, "button"]);
const LIST_SCOPE = new Set([...SCOPE, "ol", "ul"]);
const TABLE_SCOPE = set("html table template");
const TABLE_PARTS = set("table caption tbody thead tfoot tr td th");
const SECTIONS = set("thead tbody tfoot");
const HIDDEN = set("script style noscript template title");
const BLOCKS = set(
  "address article aside blockquote caption details dialog div dl fieldset figcaption figure footer form header " +
    "hgroup main nav ol p pre section summary table ul",
);

/** Maximum number of open elements; deeper start tags are ignored. */
export const MAX_DEPTH = 256;

/** Parses `html` into a document node (SPEC section 4). Never throws. */
export function parse(html: string): HtmlNode {
  const doc = new HtmlNode("document");
  const stack: HtmlNode[] = [];
  const current = (): HtmlNode => (stack.length > 0 ? stack[stack.length - 1] : doc);
  const append = (node: HtmlNode): void => {
    const p = current();
    node.parent = p;
    p.children.push(node);
  };
  const inScope = (names: Set<string>, boundary: Set<string>): boolean => {
    for (let i = stack.length - 1; i >= 0; i--) {
      if (names.has(stack[i].name)) return true;
      if (boundary.has(stack[i].name)) return false;
    }
    return false;
  };
  const popUntil = (names: Set<string>): void => {
    while (stack.length > 0) if (names.has(stack.pop()!.name)) return;
  };
  const top = (): string | null => (stack.length > 0 ? stack[stack.length - 1].name : null);

  const start = (name: string, attrs: [string, string][]): boolean => {
    if (name === "li" || name === "dd" || name === "dt") {
      const targets = name === "li" ? set("li") : set("dd dt");
      for (let i = stack.length - 1; i >= 0; i--) {
        const cur = stack[i].name;
        if (targets.has(cur)) {
          stack.length = i;
          break;
        }
        if (SPECIAL.has(cur) && cur !== "address" && cur !== "div" && cur !== "p") break;
      }
    }
    if (P_CLOSERS.has(name) && inScope(set("p"), BUTTON_SCOPE)) popUntil(set("p"));
    if (HEADINGS.has(name) && HEADINGS.has(top() ?? "")) stack.pop();
    if ((name === "option" || name === "optgroup") && top() === "option") stack.pop();
    if (name === "a" && stack.some((n) => n.name === "a")) popUntil(set("a"));
    if (["td", "th", "tr", "thead", "tbody", "tfoot"].includes(name) && inScope(set("td th"), TABLE_SCOPE))
      popUntil(set("td th"));
    if (["tr", "thead", "tbody", "tfoot"].includes(name) && inScope(set("tr"), TABLE_SCOPE)) popUntil(set("tr"));
    if (SECTIONS.has(name) && inScope(SECTIONS, TABLE_SCOPE)) popUntil(SECTIONS);
    if (stack.length >= MAX_DEPTH) return false;
    const el = new HtmlNode("element", name, attrs);
    append(el);
    if (!VOID.has(name)) stack.push(el);
    return name === "pre" || name === "listing" || name === "textarea";
  };

  const end = (name: string): void => {
    if (name === "br") {
      start("br", []);
      return;
    }
    let target: Set<string>;
    let boundary: Set<string>;
    if (name === "p") [target, boundary] = [set("p"), BUTTON_SCOPE];
    else if (HEADINGS.has(name)) [target, boundary] = [HEADINGS, SCOPE];
    else if (name === "li") [target, boundary] = [set("li"), LIST_SCOPE];
    else if (TABLE_PARTS.has(name)) [target, boundary] = [set(name), TABLE_SCOPE];
    else if (name === "dd" || name === "dt" || SPECIAL.has(name)) [target, boundary] = [set(name), SCOPE];
    else {
      for (let i = stack.length - 1; i >= 0; i--) {
        if (stack[i].name === name) {
          stack.length = i;
          return;
        }
        if (SPECIAL.has(stack[i].name)) return;
      }
      return;
    }
    if (inScope(target, boundary)) popUntil(target);
  };

  let skipNewline = false;
  for (const tok of tokenize(html)) {
    if (tok.type === "Character") {
      let data = tok.data;
      if (skipNewline && data.startsWith("\n")) data = data.slice(1);
      skipNewline = false;
      if (data !== "") {
        const p = current();
        const last = p.children[p.children.length - 1];
        if (last !== undefined && last.kind === "text") last.data += data;
        else append(new HtmlNode("text", "", [], data));
      }
      continue;
    }
    skipNewline = false;
    if (tok.type === "StartTag") skipNewline = start(tok.name, tok.attrs);
    else if (tok.type === "EndTag") end(tok.name);
    else if (tok.type === "Comment") append(new HtmlNode("comment", "", [], tok.data));
  }
  return doc;
}

function writeNode(n: HtmlNode, out: string[]): void {
  if (n.kind === "text") {
    const raw = n.parent !== null && n.parent.kind === "element" && RAW_PARENTS.has(n.parent.name);
    out.push(raw ? n.data : escapeText(n.data));
  } else if (n.kind === "comment") {
    out.push(`<!--${n.data}-->`);
  } else if (n.kind === "element") {
    out.push("<" + n.name);
    for (const [k, v] of n.attrs) out.push(` ${k}="${escapeAttr(v)}"`);
    out.push(">");
    if (VOID.has(n.name)) return;
    for (const c of n.children) writeNode(c, out);
    out.push(`</${n.name}>`);
  } else {
    for (const c of n.children) writeNode(c, out);
  }
}

// ---------------------------------------------------------------- text

function extractText(node: HtmlNode): string {
  const parts: string[] = [];
  let lastCh = "";
  const push = (t: string): void => {
    if (t !== "") {
      parts.push(t);
      lastCh = t[t.length - 1];
    }
  };
  const walk = (n: HtmlNode, pre: boolean): void => {
    if (n.kind === "text") {
      if (pre) {
        push(n.data);
        return;
      }
      let collapsed = n.data.replace(/[\t\n\f\r ]+/g, " ");
      if (collapsed.startsWith(" ") && (lastCh === "" || lastCh === " " || lastCh === "\n" || lastCh === "\t"))
        collapsed = collapsed.slice(1);
      push(collapsed);
      return;
    }
    if (n.kind === "comment") return;
    if (n.kind === "document") {
      for (const c of n.children) walk(c, pre);
      return;
    }
    const name = n.name;
    if (HIDDEN.has(name)) return;
    if (HEADINGS.has(name)) {
      push("\n\n" + "#".repeat(Number(name[1])) + " ");
      for (const c of n.children) walk(c, pre);
      push("\n\n");
      return;
    }
    if (name === "li") push("\n- ");
    else if (name === "dd" || name === "dt" || name === "tr") push("\n");
    else if (name === "br") {
      push("\n");
      return;
    } else if (name === "hr") {
      push("\n\n---\n\n");
      return;
    }
    const block = BLOCKS.has(name);
    if (block) push("\n\n");
    const innerPre = pre || name === "pre" || name === "listing" || name === "textarea";
    for (const c of n.children) walk(c, innerPre);
    if (name === "td" || name === "th") push("\t");
    if (block) push("\n\n");
  };
  walk(node, false);
  const text = parts
    .join("")
    .split("\n")
    .map((l) => l.replace(/[ \t]+$/, ""))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n");
  return text.replace(/^\n+/, "").replace(/\n+$/, "");
}

/** Parses `html` and returns its structure-preserving plain text. */
export function htmlToText(html: string): string {
  return parse(html).extractText();
}

/** Parses `html` and returns its text content without markup. */
export function stripTags(html: string): string {
  return parse(html).textContent();
}

// ---------------------------------------------------------------- sanitizer

const DEFAULT_ALLOWED =
  "a abbr b blockquote br caption cite code dd del dfn div dl dt em figcaption figure h1 h2 h3 h4 h5 h6 hr i img " +
  "ins kbd li mark ol p pre q s samp small span strong sub sup table tbody td tfoot th thead time tr u ul";
const DEFAULT_DROP =
  "applet audio base button canvas embed frame frameset head iframe link math meta noembed noframes noscript " +
  "object option plaintext script select style svg template textarea title video xmp";
const DEFAULT_ATTRS =
  "*:dir *:lang *:title a:href blockquote:cite img:alt img:height img:src img:width ol:start q:cite td:colspan " +
  "td:rowspan th:colspan th:rowspan th:scope time:datetime";
const URL_ATTRS = set("cite href src");
/** URL schemes allowed by default. */
export const DEFAULT_SCHEMES: readonly string[] = ["http", "https", "mailto", "tel"];

const asciiLower = (s: string): string => s.replace(/[A-Z]/g, (m) => String.fromCharCode(m.charCodeAt(0) + 32));

/** Sanitizer policy: the defaults plus extra tags, attributes and schemes. */
export class Policy {
  private readonly tags = set(DEFAULT_ALLOWED);
  private readonly drop = set(DEFAULT_DROP);
  private readonly attrs = set(DEFAULT_ATTRS);
  /** @internal */
  readonly schemes = new Set(DEFAULT_SCHEMES);

  /** Keeps `tag` (ASCII case-insensitive), also when it is on the drop list. */
  allowTag(tag: string): this {
    const t = asciiLower(tag);
    if (t === "plaintext") return this; // cannot be closed again, so never kept
    this.tags.add(t);
    this.drop.delete(t);
    return this;
  }

  /** Keeps attribute `attr` on `tag`; tag `"*"` means every kept element. */
  allowAttr(tag: string, attr: string): this {
    this.attrs.add(asciiLower(tag) + ":" + asciiLower(attr));
    return this;
  }

  /** Accepts URLs with `scheme` in `href`, `src` and `cite`. */
  allowScheme(scheme: string): this {
    this.schemes.add(asciiLower(scheme));
    return this;
  }

  /** @internal */
  keep(tag: string): "drop" | "unwrap" | "keep" {
    return this.drop.has(tag) ? "drop" : this.tags.has(tag) ? "keep" : "unwrap";
  }

  /** @internal */
  attrOk(tag: string, attr: string): boolean {
    return this.attrs.has(tag + ":" + attr) || this.attrs.has("*:" + attr);
  }
}

/**
 * True when `url` has no scheme or one of `schemes` (lowercase), after removing
 * C0 controls, space and DEL (SPEC section 7.3).
 */
export function isSafeUrl(url: string, schemes: Iterable<string> = DEFAULT_SCHEMES): boolean {
  const allowed = new Set(schemes);
  let cleaned = "";
  for (const ch of url) {
    const cp = ch.codePointAt(0)!;
    if (cp > 0x20 && cp !== 0x7f) cleaned += ch;
  }
  cleaned = asciiLower(cleaned);
  for (let i = 0; i < cleaned.length; i++) {
    const ch = cleaned[i];
    if (ch === "/" || ch === "?" || ch === "#") return true;
    if (ch === ":") return allowed.has(cleaned.slice(0, i));
  }
  return true;
}

/** Sanitizes `html` with `policy` (SPEC section 7). The result is a fixed point. */
export function sanitize(html: string, policy: Policy = new Policy()): string {
  const out: string[] = [];
  const walk = (n: HtmlNode): void => {
    if (n.kind === "text") {
      out.push(escapeText(n.data));
      return;
    }
    if (n.kind === "comment") return;
    if (n.kind === "document") {
      n.children.forEach(walk);
      return;
    }
    const action = policy.keep(n.name);
    if (action === "drop") return;
    if (action === "unwrap") {
      n.children.forEach(walk);
      return;
    }
    out.push("<" + n.name);
    for (const [k, v] of n.attrs) {
      if (!policy.attrOk(n.name, k)) continue;
      if (URL_ATTRS.has(k) && !isSafeUrl(v, policy.schemes)) continue;
      out.push(` ${k}="${escapeAttr(v)}"`);
    }
    out.push(">");
    if (VOID.has(n.name)) return;
    // Raw text content would not survive a second pass, so it is dropped.
    if (!RAW_PARENTS.has(n.name)) n.children.forEach(walk);
    out.push(`</${n.name}>`);
  };
  walk(parse(html));
  return out.join("");
}

// ---------------------------------------------------------------- selectors

/** A selector that does not follow the SPEC grammar; `offset` counts code points. */
export class SelectorError extends Error {
  readonly code = "BAD_SELECTOR";

  constructor(readonly offset: number) {
    super(`BAD_SELECTOR at character ${offset}`);
    this.name = "SelectorError";
  }
}

type Simple =
  | { k: "id" | "class"; v: string }
  | { k: "attr"; name: string; op: string | null; v: string }
  | { k: "first-child" | "last-child" | "only-child" | "empty" }
  | { k: "nth-child" | "nth-last-child"; a: number; b: number }
  | { k: "not"; inner: Compound };
interface Compound {
  tag: string | null;
  simple: Simple[];
}
type Complex = [string, Compound][];

const isWsSel = (c: string | undefined): boolean =>
  c === " " || c === "\t" || c === "\n" || c === "\r" || c === "\f";
const isIdent = (c: string | undefined): boolean =>
  c !== undefined && (/^[A-Za-z0-9_-]$/.test(c) || c.codePointAt(0)! > 0x7f);

class SelParser {
  private readonly s: string[];
  private i = 0;

  constructor(text: string) {
    this.s = Array.from(text);
  }

  private fail(): never {
    throw new SelectorError(this.i);
  }

  private ws(): boolean {
    const start = this.i;
    while (isWsSel(this.s[this.i])) this.i++;
    return this.i > start;
  }

  private ident(): string {
    const start = this.i;
    while (isIdent(this.s[this.i])) this.i++;
    if (this.i === start) this.fail();
    return this.s.slice(start, this.i).join("");
  }

  list(): Complex[] {
    const out: Complex[] = [];
    for (;;) {
      this.ws();
      out.push(this.complex());
      this.ws();
      if (this.i >= this.s.length) return out;
      if (this.s[this.i] !== ",") this.fail();
      this.i++;
    }
  }

  private complex(): Complex {
    const parts: Complex = [["", this.compound()]];
    for (;;) {
      const save = this.i;
      const hadWs = this.ws();
      const c = this.s[this.i];
      if (c === undefined || c === ",") {
        this.i = save;
        return parts;
      }
      if (c === ">" || c === "+" || c === "~") {
        this.i++;
        this.ws();
        parts.push([c, this.compound()]);
      } else if (hadWs) parts.push([" ", this.compound()]);
      else this.fail();
    }
  }

  private compound(): Compound {
    const comp: Compound = { tag: null, simple: [] };
    if (this.s[this.i] === "*") {
      this.i++;
      comp.tag = "*";
    } else if (isIdent(this.s[this.i])) comp.tag = asciiLower(this.ident());
    for (;;) {
      const c = this.s[this.i];
      if (c === "#" || c === ".") {
        this.i++;
        comp.simple.push({ k: c === "#" ? "id" : "class", v: this.ident() });
      } else if (c === "[") {
        this.i++;
        this.ws();
        const name = asciiLower(this.ident());
        this.ws();
        let op: string | null = null;
        let v = "";
        const two = this.s.slice(this.i, this.i + 2).join("");
        if (this.s[this.i] === "=") {
          op = "=";
          this.i++;
        } else if (["~=", "|=", "^=", "$=", "*="].includes(two)) {
          op = two;
          this.i += 2;
        }
        if (op !== null) {
          this.ws();
          const q = this.s[this.i];
          if (q === "'" || q === '"') {
            const end = this.s.indexOf(q, this.i + 1);
            if (end < 0) this.fail();
            v = this.s.slice(this.i + 1, end).join("");
            this.i = end + 1;
          } else v = this.ident();
          this.ws();
        }
        if (this.s[this.i] !== "]") this.fail();
        this.i++;
        comp.simple.push({ k: "attr", name, op, v });
      } else if (c === ":") {
        this.i++;
        const name = asciiLower(this.ident());
        if (name === "first-child" || name === "last-child" || name === "only-child" || name === "empty") {
          comp.simple.push({ k: name });
        } else if (name === "nth-child" || name === "nth-last-child" || name === "not") {
          if (this.s[this.i] !== "(") this.fail();
          this.i++;
          this.ws();
          let simple: Simple;
          if (name === "not") {
            simple = { k: "not", inner: this.compound() };
            this.ws();
          } else {
            const end = this.s.indexOf(")", this.i);
            if (end < 0) this.fail();
            const ab = parseNth(this.s.slice(this.i, end).join(""));
            if (ab === null) this.fail();
            this.i = end;
            simple = { k: name, a: ab[0], b: ab[1] };
          }
          if (this.s[this.i] !== ")") this.fail();
          this.i++;
          comp.simple.push(simple);
        } else this.fail();
      } else break;
    }
    if (comp.tag === null && comp.simple.length === 0) this.fail();
    return comp;
  }
}

function parseNth(text: string): [number, number] | null {
  const t = asciiLower(text.replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, ""));
  if (t === "odd") return [2, 1];
  if (t === "even") return [2, 0];
  if (/^[+-]?[0-9]{1,9}$/.test(t)) return [0, parseInt(t, 10)];
  const m = /^([+-]?[0-9]{0,9})n(?:[\t\n\f\r ]*([+-])[\t\n\f\r ]*([0-9]{1,9}))?$/.exec(t);
  if (m === null) return null;
  const a = m[1] === "" || m[1] === "+" ? 1 : m[1] === "-" ? -1 : parseInt(m[1], 10);
  const b = m[3] === undefined ? 0 : parseInt(m[3], 10) * (m[2] === "-" ? -1 : 1);
  return [a, b];
}

function nthOk(a: number, b: number, pos: number): boolean {
  if (a === 0) return pos === b;
  if (a > 0) return pos >= b && (pos - b) % a === 0;
  return pos <= b && (b - pos) % -a === 0;
}

const splitWs = (s: string): string[] => s.split(/[\t\n\f\r ]+/).filter((x) => x !== "");

/**
 * Per-query caches: element-sibling positions per parent, match results per
 * (step, node) and the first sibling matching a step (for "~"). They keep
 * matching linear in the number of siblings and in the tree depth; results are
 * unchanged.
 */
class Ctx {
  private readonly sibs = new Map<HtmlNode, HtmlNode[]>();
  private readonly pos = new Map<HtmlNode, number>();
  readonly memo = new Map<string, Map<HtmlNode, boolean>>();
  readonly first = new Map<string, Map<HtmlNode, number>>();

  siblings(el: HtmlNode): [HtmlNode[], number] {
    const p = el.parent;
    if (p === null) return [[el], 0];
    let sib = this.sibs.get(p);
    if (sib === undefined) {
      sib = el.elementSiblings();
      sib.forEach((x, i) => this.pos.set(x, i));
      this.sibs.set(p, sib);
    }
    return [sib, this.pos.get(el)!];
  }
}

function table<K, V>(m: Map<string, Map<K, V>>, key: string): Map<K, V> {
  let t = m.get(key);
  if (t === undefined) {
    t = new Map();
    m.set(key, t);
  }
  return t;
}

function matchCompound(el: HtmlNode, comp: Compound, ctx: Ctx): boolean {
  if (comp.tag !== null && comp.tag !== "*" && el.name !== comp.tag) return false;
  for (const s of comp.simple) {
    switch (s.k) {
      case "id":
        if (el.attr("id") !== s.v) return false;
        break;
      case "class":
        if (!splitWs(el.attr("class") ?? "").includes(s.v)) return false;
        break;
      case "attr": {
        const v = el.attr(s.name);
        if (v === null) return false;
        const val = s.v;
        if (s.op === "=" && v !== val) return false;
        if (s.op === "~=" && !splitWs(v).includes(val)) return false;
        if (s.op === "|=" && !(v === val || v.startsWith(val + "-"))) return false;
        if (s.op === "^=" && !(val !== "" && v.startsWith(val))) return false;
        if (s.op === "$=" && !(val !== "" && v.endsWith(val))) return false;
        if (s.op === "*=" && !(val !== "" && v.includes(val))) return false;
        break;
      }
      case "empty":
        if (el.children.some((c) => c.kind === "element" || c.kind === "text")) return false;
        break;
      case "not":
        if (matchCompound(el, s.inner, ctx)) return false;
        break;
      default: {
        const [sib, idx] = ctx.siblings(el);
        const pos = idx + 1;
        if (s.k === "first-child" && pos !== 1) return false;
        if (s.k === "last-child" && pos !== sib.length) return false;
        if (s.k === "only-child" && sib.length !== 1) return false;
        if (s.k === "nth-child" && !nthOk(s.a, s.b, pos)) return false;
        if (s.k === "nth-last-child" && !nthOk(s.a, s.b, sib.length - pos + 1)) return false;
      }
    }
  }
  return true;
}

function matchComplex(el: HtmlNode, parts: Complex, k: number, ctx: Ctx, which: number): boolean {
  const memo = table(ctx.memo, `${which}:${k}`);
  let hit = memo.get(el);
  if (hit === undefined) {
    hit = matchStep(el, parts, k, ctx, which);
    memo.set(el, hit);
  }
  return hit;
}

function matchStep(el: HtmlNode, parts: Complex, k: number, ctx: Ctx, which: number): boolean {
  const [comb, comp] = parts[k];
  if (!matchCompound(el, comp, ctx)) return false;
  if (k === 0) return true;
  if (comb === ">") {
    const p = el.parent;
    return p !== null && p.kind === "element" && matchComplex(p, parts, k - 1, ctx, which);
  }
  if (comb === " ") {
    for (let p = el.parent; p !== null && p.kind === "element"; p = p.parent) {
      if (matchComplex(p, parts, k - 1, ctx, which)) return true;
    }
    return false;
  }
  const [sib, idx] = ctx.siblings(el);
  if (comb === "+") return idx > 0 && matchComplex(sib[idx - 1], parts, k - 1, ctx, which);
  // "~": some earlier sibling matches step k-1; remember the first such sibling per parent.
  const firsts = table(ctx.first, `${which}:${k}`);
  const p = el.parent!;
  let first = firsts.get(p);
  if (first === undefined) {
    first = sib.findIndex((x) => matchComplex(x, parts, k - 1, ctx, which));
    if (first < 0) first = sib.length;
    firsts.set(p, first);
  }
  return first < idx;
}

/** A parsed selector list (SPEC section 8). */
export class Selector {
  private constructor(private readonly list: Complex[]) {}

  /** Parses `selector`; throws {@link SelectorError}. */
  static parse(selector: string): Selector {
    return new Selector(new SelParser(selector).list());
  }

  /** True when `el` is an element that matches. */
  matches(el: HtmlNode): boolean {
    return this.matchesIn(el, new Ctx());
  }

  /** @internal */
  matchesIn(el: HtmlNode, ctx: Ctx): boolean {
    return el.kind === "element" && this.list.some((parts, i) => matchComplex(el, parts, parts.length - 1, ctx, i));
  }
}

// ---------------------------------------------------------------- meta and tables

/** Metadata found in a document (SPEC section 9). */
export interface PageMeta {
  title: string | null;
  description: string | null;
  canonical: string | null;
  lang: string | null;
  og: [string, string][];
}

const collapse = (s: string): string => s.replace(/[\t\n\f\r ]+/g, " ").replace(/^ +| +$/g, "");

function extractMeta(doc: HtmlNode): PageMeta {
  const out: PageMeta = { title: null, description: null, canonical: null, lang: null, og: [] };
  let seenTitle = false;
  for (const el of doc.elements()) {
    if (el.name === "title" && !seenTitle) {
      seenTitle = true;
      const t = collapse(
        el.children
          .filter((c) => c.kind === "text")
          .map((c) => c.data)
          .join(""),
      );
      out.title = t === "" ? null : t;
    } else if (el.name === "html") {
      if (out.lang === null) out.lang = el.attr("lang");
    } else if (el.name === "meta") {
      const content = el.attr("content");
      if (content === null) continue;
      if (asciiLower(el.attr("name") ?? "") === "description" && out.description === null) out.description = content;
      const prop = el.attr("property") ?? "";
      if (asciiLower(prop).startsWith("og:")) out.og.push([prop.slice(3), content]);
    } else if (el.name === "link" && out.canonical === null) {
      if (splitWs(asciiLower(el.attr("rel") ?? "")).includes("canonical")) out.canonical = el.attr("href");
    }
  }
  return out;
}

function parseSpan(v: string | null, dflt: number, maximum: number): number {
  if (v === null) return dflt;
  const m = /^[\t\n\f\r ]*\+?([0-9]+)/.exec(v);
  if (m === null) return dflt;
  const n = m[1].length <= 9 ? parseInt(m[1], 10) : maximum;
  if (n === 0) return dflt;
  return Math.min(n, maximum);
}

/** Most cells produced for one table; extraction stops there. */
export const MAX_CELLS = 1_000_000;

function extractTables(doc: HtmlNode): string[][][] {
  const tables: string[][][] = [];
  for (const table of doc.elements()) {
    if (table.name !== "table") continue;
    const rows: HtmlNode[] = [];
    const collect = (n: HtmlNode): void => {
      for (const c of n.children) {
        if (c.kind !== "element" || c.name === "table") continue;
        if (c.name === "tr") rows.push(c);
        else collect(c);
      }
    };
    collect(table);
    const grid: string[][] = [];
    let pending = new Map<number, [number, string]>();
    let total = 0;
    let stop = false;
    for (const tr of rows) {
      const row = new Map<number, string>();
      const next = new Map<number, [number, string]>();
      for (const [col, [left, text]] of pending) {
        row.set(col, text);
        if (left > 1) next.set(col, [left - 1, text]);
      }
      let col = 0;
      for (const cell of tr.children) {
        if (!(cell.is("td") || cell.is("th"))) continue;
        const text = collapse(cell.textContent());
        const cs = parseSpan(cell.attr("colspan"), 1, 1000);
        const rs = parseSpan(cell.attr("rowspan"), 1, 65534);
        for (let x = 0; x < cs; x++) {
          while (row.has(col)) col++;
          total++;
          if (total > MAX_CELLS) {
            stop = true;
            break;
          }
          row.set(col, text);
          if (rs > 1) next.set(col, [rs - 1, text]);
          col++;
        }
        if (stop) break;
      }
      pending = next;
      let width = 0;
      for (const c of row.keys()) width = Math.max(width, c + 1);
      const line: string[] = [];
      for (let c = 0; c < width; c++) line.push(row.get(c) ?? "");
      grid.push(line);
      if (stop) break;
    }
    tables.push(grid);
  }
  return tables;
}
