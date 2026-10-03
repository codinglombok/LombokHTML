// WHATWG HTML tokenizer (SPEC section 2), for HTML content (no foreign content).
import { ENTITIES } from "./entities.js";

/** A token. Attribute names are lowercased; duplicate attributes keep the first. */
export type Token =
  | { type: "StartTag"; name: string; attrs: [string, string][]; selfClosing: boolean }
  | { type: "EndTag"; name: string }
  | { type: "Character"; data: string }
  | { type: "Comment"; data: string }
  | { type: "DOCTYPE"; name: string | null; publicId: string | null; systemId: string | null; correct: boolean };

/** Tokenizer state to start in (SPEC section 2.1). */
export type InitialState = "data" | "rcdata" | "rawtext" | "script" | "plaintext" | "cdata";

/** Longest named reference, in characters, including the trailing `;`. */
export const MAX_ENTITY = 32;

const REPLACEMENTS: Record<number, number> = {
  0x80: 0x20ac, 0x82: 0x201a, 0x83: 0x0192, 0x84: 0x201e, 0x85: 0x2026, 0x86: 0x2020, 0x87: 0x2021,
  0x88: 0x02c6, 0x89: 0x2030, 0x8a: 0x0160, 0x8b: 0x2039, 0x8c: 0x0152, 0x8e: 0x017d, 0x91: 0x2018,
  0x92: 0x2019, 0x93: 0x201c, 0x94: 0x201d, 0x95: 0x2022, 0x96: 0x2013, 0x97: 0x2014, 0x98: 0x02dc,
  0x99: 0x2122, 0x9a: 0x0161, 0x9b: 0x203a, 0x9c: 0x0153, 0x9e: 0x017e, 0x9f: 0x0178,
};

/** @internal Character for a numeric reference. */
export function numericChar(code: number): string {
  if (code === 0 || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff)) return "�";
  return String.fromCodePoint(REPLACEMENTS[code] ?? code);
}

/** @internal */
export function lookupNamed(name: string): string | undefined {
  return Object.prototype.hasOwnProperty.call(ENTITIES, name) ? ENTITIES[name] : undefined;
}

const isWs = (c: string | undefined): boolean => c === "\t" || c === "\n" || c === "\f" || c === " ";
const isAlpha = (c: string | undefined): boolean => c !== undefined && /^[A-Za-z]$/.test(c);
const isDigit = (c: string | undefined): boolean => c !== undefined && c >= "0" && c <= "9" && c.length === 1;
const isHex = (c: string | undefined): boolean => c !== undefined && /^[0-9A-Fa-f]$/.test(c);
/** @internal */
export const isAlnum = (c: string | undefined): boolean => isAlpha(c) || isDigit(c);
const lower = (c: string): string => c.replace(/[A-Z]/g, (m) => String.fromCharCode(m.charCodeAt(0) + 32));

interface Tag {
  end: boolean;
  name: string;
  attrs: [string, string][];
  selfClosing: boolean;
}

interface Doctype {
  name: string | null;
  publicId: string | null;
  systemId: string | null;
  quirks: boolean;
}

const R = "�";

class Tokenizer {
  private s: string[];
  private i = 0;
  private state: string;
  private ret = "data";
  private out: Token[] = [];
  private tag: Tag | null = null;
  private attr: [string, string] = ["", ""];
  private temp = "";
  private comment = "";
  private doctype: Doctype = { name: null, publicId: null, systemId: null, quirks: false };
  private code = 0;

  constructor(
    text: string,
    state: InitialState,
    private lastStart: string | null,
    private readonly switching: boolean,
  ) {
    this.s = Array.from(text.replace(/\r\n?/g, "\n"));
    this.state = state;
  }

  private emit(s: string): void {
    const last = this.out[this.out.length - 1];
    if (last !== undefined && last.type === "Character") last.data += s;
    else this.out.push({ type: "Character", data: s });
  }

  private emitTag(): void {
    const t = this.tag!;
    if (!t.end) {
      const seen = new Set<string>();
      const attrs: [string, string][] = [];
      for (const [k, v] of t.attrs) {
        if (!seen.has(k)) {
          seen.add(k);
          attrs.push([k, v]);
        }
      }
      this.out.push({ type: "StartTag", name: t.name, attrs, selfClosing: t.selfClosing });
      this.lastStart = t.name;
      if (this.switching) {
        const n = t.name;
        if (n === "title" || n === "textarea") this.state = "rcdata";
        else if (["style", "xmp", "iframe", "noembed", "noframes", "noscript"].includes(n)) this.state = "rawtext";
        else if (n === "script") this.state = "script";
        else if (n === "plaintext") this.state = "plaintext";
      }
    } else {
      this.out.push({ type: "EndTag", name: t.name });
    }
    this.tag = null;
  }

  private appropriate(): boolean {
    return this.tag !== null && this.tag.end && this.tag.name === this.lastStart;
  }

  private startAttr(): void {
    this.attr = ["", ""];
    this.tag!.attrs.push(this.attr);
  }

  private peek(n: number): string {
    return this.s.slice(this.i, this.i + n).join("");
  }

  private inAttr(): boolean {
    return this.ret === "attr_dq" || this.ret === "attr_sq" || this.ret === "attr_unq";
  }

  private flushRef(): void {
    if (this.inAttr()) this.attr[1] += this.temp;
    else this.emit(this.temp);
  }

  private reconsume(state: string): void {
    this.i -= 1;
    this.state = state;
  }

  private newTag(end: boolean): void {
    this.tag = { end, name: "", attrs: [], selfClosing: false };
  }

  private emitComment(): void {
    this.out.push({ type: "Comment", data: this.comment });
  }

  private emitDoctype(quirks = false): void {
    if (quirks) this.doctype.quirks = true;
    const d = this.doctype;
    this.out.push({ type: "DOCTYPE", name: d.name, publicId: d.publicId, systemId: d.systemId, correct: !d.quirks });
  }

  private newDoctype(): void {
    this.doctype = { name: null, publicId: null, systemId: null, quirks: false };
  }

  run(): Token[] {
    for (;;) {
      const c = this.i < this.s.length ? this.s[this.i] : undefined;
      this.i += 1;
      if (!this.step(c)) return this.out;
    }
  }

  private endName(c: string | undefined, base: string): void {
    if (isWs(c) && this.appropriate()) {
      this.state = "before_attr_name";
    } else if (c === "/" && this.appropriate()) {
      this.state = "self_closing";
    } else if (c === ">" && this.appropriate()) {
      this.state = "data";
      this.emitTag();
    } else if (isAlpha(c)) {
      this.tag!.name += lower(c!);
      this.temp += c;
    } else {
      this.emit("</" + this.temp);
      this.tag = null;
      this.reconsume(base);
    }
  }

  private endOpen(c: string | undefined, base: string, nameState: string): void {
    if (isAlpha(c)) {
      this.newTag(true);
      this.reconsume(nameState);
    } else {
      this.emit("</");
      this.reconsume(base);
    }
  }

  private lt(c: string | undefined, base: string, openState: string): void {
    if (c === "/") {
      this.temp = "";
      this.state = openState;
    } else {
      this.emit("<");
      this.reconsume(base);
    }
  }

  private quoteState(which: "public" | "system", c: string): string {
    return `doctype_${which}_${c === '"' ? "dq" : "sq"}`;
  }

  /** Returns false at end of input. */
  private step(c: string | undefined): boolean {
    switch (this.state) {
      case "data":
        if (c === "&") {
          this.ret = "data";
          this.state = "charref";
        } else if (c === "<") this.state = "tag_open";
        else if (c === undefined) return false;
        else this.emit(c);
        return true;
      case "rcdata":
        if (c === "&") {
          this.ret = "rcdata";
          this.state = "charref";
        } else if (c === "<") this.state = "rcdata_lt";
        else if (c === "\0") this.emit(R);
        else if (c === undefined) return false;
        else this.emit(c);
        return true;
      case "rawtext":
      case "script":
        if (c === "<") this.state = this.state === "rawtext" ? "rawtext_lt" : "script_lt";
        else if (c === "\0") this.emit(R);
        else if (c === undefined) return false;
        else this.emit(c);
        return true;
      case "plaintext":
        if (c === "\0") this.emit(R);
        else if (c === undefined) return false;
        else this.emit(c);
        return true;
      case "tag_open":
        if (c === "!") this.state = "markup_decl";
        else if (c === "/") this.state = "end_tag_open";
        else if (isAlpha(c)) {
          this.newTag(false);
          this.reconsume("tag_name");
        } else if (c === "?") {
          this.comment = "";
          this.reconsume("bogus_comment");
        } else if (c === undefined) {
          this.emit("<");
          return false;
        } else {
          this.emit("<");
          this.reconsume("data");
        }
        return true;
      case "end_tag_open":
        if (isAlpha(c)) {
          this.newTag(true);
          this.reconsume("tag_name");
        } else if (c === ">") this.state = "data";
        else if (c === undefined) {
          this.emit("</");
          return false;
        } else {
          this.comment = "";
          this.reconsume("bogus_comment");
        }
        return true;
      case "tag_name":
        if (isWs(c)) this.state = "before_attr_name";
        else if (c === "/") this.state = "self_closing";
        else if (c === ">") {
          this.state = "data";
          this.emitTag();
        } else if (c === "\0") this.tag!.name += R;
        else if (c === undefined) return false;
        else this.tag!.name += lower(c);
        return true;
      case "rcdata_lt":
        this.lt(c, "rcdata", "rcdata_end_open");
        return true;
      case "rcdata_end_open":
        this.endOpen(c, "rcdata", "rcdata_end_name");
        return true;
      case "rcdata_end_name":
        this.endName(c, "rcdata");
        return true;
      case "rawtext_lt":
        this.lt(c, "rawtext", "rawtext_end_open");
        return true;
      case "rawtext_end_open":
        this.endOpen(c, "rawtext", "rawtext_end_name");
        return true;
      case "rawtext_end_name":
        this.endName(c, "rawtext");
        return true;
      case "script_lt":
        if (c === "/") {
          this.temp = "";
          this.state = "script_end_open";
        } else if (c === "!") {
          this.state = "script_esc_start";
          this.emit("<!");
        } else {
          this.emit("<");
          this.reconsume("script");
        }
        return true;
      case "script_end_open":
        this.endOpen(c, "script", "script_end_name");
        return true;
      case "script_end_name":
        this.endName(c, "script");
        return true;
      case "script_esc_start":
      case "script_esc_start_dash":
        if (c === "-") {
          this.state = this.state === "script_esc_start" ? "script_esc_start_dash" : "script_esc_dash_dash";
          this.emit("-");
        } else this.reconsume("script");
        return true;
      case "script_esc":
      case "script_esc_dash":
      case "script_esc_dash_dash": {
        const st = this.state;
        if (c === "-") {
          if (st === "script_esc") this.state = "script_esc_dash";
          else this.state = "script_esc_dash_dash";
          this.emit("-");
        } else if (c === "<") this.state = "script_esc_lt";
        else if (c === ">" && st === "script_esc_dash_dash") {
          this.state = "script";
          this.emit(">");
        } else if (c === undefined) return false;
        else {
          this.state = "script_esc";
          this.emit(c === "\0" ? R : c);
        }
        return true;
      }
      case "script_esc_lt":
        if (c === "/") {
          this.temp = "";
          this.state = "script_esc_end_open";
        } else if (isAlpha(c)) {
          this.temp = "";
          this.emit("<");
          this.reconsume("script_dbl_esc_start");
        } else {
          this.emit("<");
          this.reconsume("script_esc");
        }
        return true;
      case "script_esc_end_open":
        this.endOpen(c, "script_esc", "script_esc_end_name");
        return true;
      case "script_esc_end_name":
        this.endName(c, "script_esc");
        return true;
      case "script_dbl_esc_start":
      case "script_dbl_esc_end": {
        const start = this.state === "script_dbl_esc_start";
        if (isWs(c) || c === "/" || c === ">") {
          const isScript = this.temp === "script";
          this.state = isScript === start ? "script_dbl_esc" : "script_esc";
          this.emit(c!);
        } else if (isAlpha(c)) {
          this.temp += lower(c!);
          this.emit(c!);
        } else this.reconsume(start ? "script_esc" : "script_dbl_esc");
        return true;
      }
      case "script_dbl_esc":
      case "script_dbl_esc_dash":
      case "script_dbl_esc_dash_dash": {
        const st = this.state;
        if (c === "-") {
          this.state = st === "script_dbl_esc" ? "script_dbl_esc_dash" : "script_dbl_esc_dash_dash";
          this.emit("-");
        } else if (c === "<") {
          this.state = "script_dbl_esc_lt";
          this.emit("<");
        } else if (c === ">" && st === "script_dbl_esc_dash_dash") {
          this.state = "script";
          this.emit(">");
        } else if (c === undefined) return false;
        else {
          this.state = "script_dbl_esc";
          this.emit(c === "\0" ? R : c);
        }
        return true;
      }
      case "script_dbl_esc_lt":
        if (c === "/") {
          this.temp = "";
          this.state = "script_dbl_esc_end";
          this.emit("/");
        } else this.reconsume("script_dbl_esc");
        return true;
      case "before_attr_name":
        if (isWs(c)) return true;
        if (c === undefined || c === "/" || c === ">") this.reconsume("after_attr_name");
        else if (c === "=") {
          this.startAttr();
          this.attr[0] = c;
          this.state = "attr_name";
        } else {
          this.startAttr();
          this.reconsume("attr_name");
        }
        return true;
      case "attr_name":
        if (c === undefined || isWs(c) || c === "/" || c === ">") this.reconsume("after_attr_name");
        else if (c === "=") this.state = "before_attr_value";
        else if (c === "\0") this.attr[0] += R;
        else this.attr[0] += lower(c);
        return true;
      case "after_attr_name":
        if (isWs(c)) return true;
        if (c === "/") this.state = "self_closing";
        else if (c === "=") this.state = "before_attr_value";
        else if (c === ">") {
          this.state = "data";
          this.emitTag();
        } else if (c === undefined) return false;
        else {
          this.startAttr();
          this.reconsume("attr_name");
        }
        return true;
      case "before_attr_value":
        if (isWs(c)) return true;
        if (c === '"') this.state = "attr_dq";
        else if (c === "'") this.state = "attr_sq";
        else if (c === ">") {
          this.state = "data";
          this.emitTag();
        } else this.reconsume("attr_unq");
        return true;
      case "attr_dq":
      case "attr_sq":
        if (c === (this.state === "attr_dq" ? '"' : "'")) this.state = "after_attr_value_q";
        else if (c === "&") {
          this.ret = this.state;
          this.state = "charref";
        } else if (c === "\0") this.attr[1] += R;
        else if (c === undefined) return false;
        else this.attr[1] += c;
        return true;
      case "attr_unq":
        if (isWs(c)) this.state = "before_attr_name";
        else if (c === "&") {
          this.ret = "attr_unq";
          this.state = "charref";
        } else if (c === ">") {
          this.state = "data";
          this.emitTag();
        } else if (c === "\0") this.attr[1] += R;
        else if (c === undefined) return false;
        else this.attr[1] += c;
        return true;
      case "after_attr_value_q":
        if (isWs(c)) this.state = "before_attr_name";
        else if (c === "/") this.state = "self_closing";
        else if (c === ">") {
          this.state = "data";
          this.emitTag();
        } else if (c === undefined) return false;
        else this.reconsume("before_attr_name");
        return true;
      case "self_closing":
        if (c === ">") {
          this.tag!.selfClosing = true;
          this.state = "data";
          this.emitTag();
        } else if (c === undefined) return false;
        else this.reconsume("before_attr_name");
        return true;
      case "bogus_comment":
        if (c === ">") {
          this.state = "data";
          this.emitComment();
        } else if (c === undefined) {
          this.emitComment();
          return false;
        } else this.comment += c === "\0" ? R : c;
        return true;
      case "markup_decl":
        this.i -= 1;
        this.comment = "";
        if (this.peek(2) === "--") {
          this.i += 2;
          this.state = "comment_start";
        } else if (lower(this.peek(7)) === "doctype") {
          this.i += 7;
          this.state = "doctype";
        } else this.state = "bogus_comment";
        return true;
      case "comment_start":
        if (c === "-") this.state = "comment_start_dash";
        else if (c === ">") {
          this.state = "data";
          this.emitComment();
        } else this.reconsume("comment");
        return true;
      case "comment_start_dash":
        if (c === "-") this.state = "comment_end";
        else if (c === ">") {
          this.state = "data";
          this.emitComment();
        } else if (c === undefined) {
          this.emitComment();
          return false;
        } else {
          this.comment += "-";
          this.reconsume("comment");
        }
        return true;
      case "comment":
        if (c === "<") {
          this.comment += c;
          this.state = "comment_lt";
        } else if (c === "-") this.state = "comment_end_dash";
        else if (c === "\0") this.comment += R;
        else if (c === undefined) {
          this.emitComment();
          return false;
        } else this.comment += c;
        return true;
      case "comment_lt":
        if (c === "!") {
          this.comment += c;
          this.state = "comment_lt_bang";
        } else if (c === "<") this.comment += c;
        else this.reconsume("comment");
        return true;
      case "comment_lt_bang":
        if (c === "-") this.state = "comment_lt_bang_dash";
        else this.reconsume("comment");
        return true;
      case "comment_lt_bang_dash":
        if (c === "-") this.state = "comment_lt_bang_dash_dash";
        else this.reconsume("comment_end_dash");
        return true;
      case "comment_lt_bang_dash_dash":
        this.reconsume("comment_end");
        return true;
      case "comment_end_dash":
        if (c === "-") this.state = "comment_end";
        else if (c === undefined) {
          this.emitComment();
          return false;
        } else {
          this.comment += "-";
          this.reconsume("comment");
        }
        return true;
      case "comment_end":
        if (c === ">") {
          this.state = "data";
          this.emitComment();
        } else if (c === "!") this.state = "comment_end_bang";
        else if (c === "-") this.comment += "-";
        else if (c === undefined) {
          this.emitComment();
          return false;
        } else {
          this.comment += "--";
          this.reconsume("comment");
        }
        return true;
      case "comment_end_bang":
        if (c === "-") {
          this.comment += "--!";
          this.state = "comment_end_dash";
        } else if (c === ">") {
          this.state = "data";
          this.emitComment();
        } else if (c === undefined) {
          this.emitComment();
          return false;
        } else {
          this.comment += "--!";
          this.reconsume("comment");
        }
        return true;
      case "doctype":
        if (isWs(c)) this.state = "before_doctype_name";
        else if (c === undefined) {
          this.newDoctype();
          this.emitDoctype(true);
          return false;
        } else this.reconsume("before_doctype_name");
        return true;
      case "before_doctype_name":
        if (isWs(c)) return true;
        this.newDoctype();
        if (c === ">") {
          this.state = "data";
          this.emitDoctype(true);
        } else if (c === undefined) {
          this.emitDoctype(true);
          return false;
        } else {
          this.doctype.name = c === "\0" ? R : lower(c);
          this.state = "doctype_name";
        }
        return true;
      case "doctype_name":
        if (isWs(c)) this.state = "after_doctype_name";
        else if (c === ">") {
          this.state = "data";
          this.emitDoctype();
        } else if (c === undefined) {
          this.emitDoctype(true);
          return false;
        } else this.doctype.name += c === "\0" ? R : lower(c);
        return true;
      case "after_doctype_name": {
        if (isWs(c)) return true;
        if (c === ">") {
          this.state = "data";
          this.emitDoctype();
          return true;
        }
        if (c === undefined) {
          this.emitDoctype(true);
          return false;
        }
        this.i -= 1;
        const word = lower(this.peek(6));
        if (word === "public" || word === "system") {
          this.i += 6;
          this.state = `after_doctype_${word}_kw`;
        } else {
          this.i += 1;
          this.doctype.quirks = true;
          this.state = "bogus_doctype";
        }
        return true;
      }
      case "after_doctype_public_kw":
      case "after_doctype_system_kw":
      case "before_doctype_public_id":
      case "before_doctype_system_id": {
        const which = this.state.includes("public") ? "public" : "system";
        const kw = this.state.startsWith("after");
        if (isWs(c)) {
          if (kw) this.state = `before_doctype_${which}_id`;
          return true;
        }
        if (c === '"' || c === "'") {
          if (which === "public") this.doctype.publicId = "";
          else this.doctype.systemId = "";
          this.state = this.quoteState(which, c);
        } else if (c === ">") {
          this.state = "data";
          this.emitDoctype(true);
        } else if (c === undefined) {
          this.emitDoctype(true);
          return false;
        } else {
          this.doctype.quirks = true;
          this.reconsume("bogus_doctype");
        }
        return true;
      }
      case "doctype_public_dq":
      case "doctype_public_sq":
      case "doctype_system_dq":
      case "doctype_system_sq": {
        const pub = this.state.includes("public");
        const q = this.state.endsWith("dq") ? '"' : "'";
        if (c === q) this.state = pub ? "after_doctype_public_id" : "after_doctype_system_id";
        else if (c === ">") {
          this.state = "data";
          this.emitDoctype(true);
        } else if (c === undefined) {
          this.emitDoctype(true);
          return false;
        } else {
          const ch = c === "\0" ? R : c;
          if (pub) this.doctype.publicId += ch;
          else this.doctype.systemId += ch;
        }
        return true;
      }
      case "after_doctype_public_id":
      case "between_doctype_ids": {
        const after = this.state === "after_doctype_public_id";
        if (isWs(c)) {
          if (after) this.state = "between_doctype_ids";
          return true;
        }
        if (c === ">") {
          this.state = "data";
          this.emitDoctype();
        } else if (c === '"' || c === "'") {
          this.doctype.systemId = "";
          this.state = this.quoteState("system", c);
        } else if (c === undefined) {
          this.emitDoctype(true);
          return false;
        } else {
          this.doctype.quirks = true;
          this.reconsume("bogus_doctype");
        }
        return true;
      }
      case "after_doctype_system_id":
        if (isWs(c)) return true;
        if (c === ">") {
          this.state = "data";
          this.emitDoctype();
        } else if (c === undefined) {
          this.emitDoctype(true);
          return false;
        } else this.reconsume("bogus_doctype");
        return true;
      case "bogus_doctype":
        if (c === ">") {
          this.state = "data";
          this.emitDoctype();
        } else if (c === undefined) {
          this.emitDoctype();
          return false;
        }
        return true;
      case "cdata":
        if (c === "]") this.state = "cdata_bracket";
        else if (c === undefined) return false;
        else this.emit(c);
        return true;
      case "cdata_bracket":
        if (c === "]") this.state = "cdata_end";
        else {
          this.emit("]");
          this.reconsume("cdata");
        }
        return true;
      case "cdata_end":
        if (c === "]") this.emit("]");
        else if (c === ">") this.state = "data";
        else {
          this.emit("]]");
          this.reconsume("cdata");
        }
        return true;
      case "charref":
        this.temp = "&";
        if (isAlnum(c)) this.reconsume("named_ref");
        else if (c === "#") {
          this.temp += c;
          this.state = "numeric_ref";
        } else {
          this.flushRef();
          this.reconsume(this.ret);
        }
        return true;
      case "named_ref": {
        this.i -= 1;
        const start = this.i;
        let j = start;
        while (j < this.s.length && isAlnum(this.s[j]) && j - start < MAX_ENTITY) j++;
        const run = this.s.slice(start, j).join("");
        let match: string | null = null;
        if (this.s[j] === ";" && lookupNamed(run + ";") !== undefined) match = run + ";";
        else {
          for (let k = run.length; k > 0; k--) {
            if (lookupNamed(run.slice(0, k)) !== undefined) {
              match = run.slice(0, k);
              break;
            }
          }
        }
        if (match === null) {
          this.temp += run;
          this.i = j;
          this.flushRef();
          this.state = "ambiguous_amp";
          return true;
        }
        this.i = start + match.length;
        const next = this.s[this.i];
        if (this.inAttr() && !match.endsWith(";") && (next === "=" || isAlnum(next))) this.temp += match;
        else this.temp = lookupNamed(match)!;
        this.flushRef();
        this.state = this.ret;
        return true;
      }
      case "ambiguous_amp":
        if (isAlnum(c)) {
          if (this.inAttr()) this.attr[1] += c;
          else this.emit(c!);
        } else this.reconsume(this.ret);
        return true;
      case "numeric_ref":
        this.code = 0;
        if (c === "x" || c === "X") {
          this.temp += c;
          this.state = "hex_ref_start";
        } else this.reconsume("dec_ref_start");
        return true;
      case "hex_ref_start":
      case "dec_ref_start": {
        const hex = this.state === "hex_ref_start";
        if (hex ? isHex(c) : isDigit(c)) this.reconsume(hex ? "hex_ref" : "dec_ref");
        else {
          this.flushRef();
          this.reconsume(this.ret);
        }
        return true;
      }
      case "hex_ref":
      case "dec_ref": {
        const hex = this.state === "hex_ref";
        if (hex ? isHex(c) : isDigit(c)) this.code = Math.min(this.code * (hex ? 16 : 10) + parseInt(c!, 16), 0x110000);
        else if (c === ";") this.state = "numeric_ref_end";
        else this.reconsume("numeric_ref_end");
        return true;
      }
      default: // numeric_ref_end
        this.i -= 1;
        this.temp = numericChar(this.code);
        this.flushRef();
        this.state = this.ret;
        return true;
    }
  }
}

/**
 * Tokenizes `html` as the parser does: start tags of `title`, `textarea`,
 * `style`, `xmp`, `iframe`, `noembed`, `noframes`, `noscript`, `script` and
 * `plaintext` switch the tokenizer state (SPEC section 2.2).
 */
export function tokenize(html: string): Token[] {
  return new Tokenizer(html, "data", null, true).run();
}

/**
 * Tokenizes `html` from `state` without element-driven switching, as the
 * html5lib tokenizer tests do.
 */
export function tokenizeState(html: string, state: InitialState, lastStartTag: string | null = null): Token[] {
  return new Tokenizer(html, state, lastStartTag, false).run();
}
