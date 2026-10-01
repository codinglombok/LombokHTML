// Helpers that reproduce Rust `str`/`char` semantics exactly, so every output is
// byte-identical with the Rust reference (ADR-015). JS strings are UTF-16; Rust
// operates on Unicode scalar values (code points) and UTF-8 bytes.

/** Unicode White_Space (what Rust's `char::is_whitespace` / `str::trim` use). */
export function isRustWhitespace(c: string): boolean {
  const n = c.codePointAt(0)!;
  return (
    (n >= 0x09 && n <= 0x0d) || n === 0x20 || n === 0x85 || n === 0xa0 || n === 0x1680 ||
    (n >= 0x2000 && n <= 0x200a) || n === 0x2028 || n === 0x2029 || n === 0x202f || n === 0x205f || n === 0x3000
  );
}
/** General category Cc (Rust `char::is_control`). */
export function isRustControl(c: string): boolean {
  const n = c.codePointAt(0)!;
  return n <= 0x1f || (n >= 0x7f && n <= 0x9f);
}
export const chars = (s: string): string[] => Array.from(s);

export function rustTrim(s: string): string {
  const cs = chars(s);
  let a = 0, b = cs.length;
  while (a < b && isRustWhitespace(cs[a]!)) a++;
  while (b > a && isRustWhitespace(cs[b - 1]!)) b--;
  return cs.slice(a, b).join("");
}
export const asciiLower = (s: string): string => s.replace(/[A-Z]/g, (c) => String.fromCharCode(c.charCodeAt(0) + 32));
export const asciiUpper = (s: string): string => s.replace(/[a-z]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 32));
export const eqIgnoreAsciiCase = (a: string, b: string): boolean => asciiLower(a) === asciiLower(b);
export const isAsciiAlpha = (s: string): boolean => /^[A-Za-z]+$/.test(s);
export const isAsciiAlnum = (s: string): boolean => /^[A-Za-z0-9]+$/.test(s);
export const utf8Len = (s: string): number => new TextEncoder().encode(s).length;
/** Byte-wise (== code point) ordering, like Rust `String: Ord` (JS default sorts UTF-16 units). */
export function cmpCodePoints(a: string, b: string): number {
  const x = chars(a), y = chars(b);
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const d = x[i]!.codePointAt(0)! - y[i]!.codePointAt(0)!;
    if (d !== 0) return d;
  }
  return x.length - y.length;
}
