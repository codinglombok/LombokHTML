//! Dependency-free pseudo-fuzz: deterministic random HTML-ish soup must never
//! panic any public API, and sanitizer output must satisfy its invariants.

use lombokhtml::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[(self.next() % xs.len() as u64) as usize]
    }
}

const PIECES: &[&str] = &[
    "<",
    ">",
    "</",
    "/>",
    "<!--",
    "-->",
    "<!DOCTYPE html>",
    "<p>",
    "</p>",
    "<div>",
    "</div>",
    "<b>",
    "</b>",
    "<i>",
    "<a href=\"",
    "javascript:alert(1)",
    "\"",
    "'",
    "=",
    " ",
    "\n",
    "\t",
    "<script>",
    "</script>",
    "<style>",
    "</style>",
    "<table>",
    "<tr>",
    "<td colspan=99999999999>",
    "<td rowspan=3>",
    "</td>",
    "</tr>",
    "</table>",
    "onclick=",
    "onerror=x ",
    "<img src=x ",
    "&amp;",
    "&#0;",
    "&#x110000;",
    "&lt;",
    "&",
    "&#",
    "é",
    "日本",
    "\u{0}",
    "\u{FFFF}",
    "<svg>",
    "</svg>",
    "<iframe src=",
    "<li>",
    "<ul>",
    "</ul>",
    "<h1>",
    "</h1>",
    "text",
    "<br>",
    "<title>",
    "<meta property=\"og:x\" content=\"y\">",
    "class=",
    "id=",
    "[",
    "]",
    ",",
    "#",
    ".",
    "*",
];

fn soup(rng: &mut Rng) -> String {
    let n = (rng.next() % 40) as usize;
    (0..n).map(|_| rng.pick(PIECES)).collect()
}

#[test]
fn no_panics_and_sanitizer_invariants() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..20_000 {
        let html = soup(&mut rng);
        let doc = parse(&html);
        let _ = strip_tags(&html);
        let _ = extract_structured_text(&html);
        let _ = extract_tables(&doc);
        let _ = extract_meta(&doc);
        for sel in ["p", "div > p", "a[href^=j], .x #y", "*", "[", ">>", ""] {
            let _ = query(&doc, sel);
        }
        let clean = sanitize(&html);
        let lower = clean.to_ascii_lowercase();
        assert!(
            !lower.contains("<script"),
            "script survived: {html:?} -> {clean:?}"
        );
        assert!(!lower.contains("<iframe") && !lower.contains("<svg") && !lower.contains("<style"));
        // Structural check: every emitted tag/attribute must be on the allowlist.
        let p = Policy::default();
        for tok in tokenize(&clean) {
            if let Token::StartTag { name, attrs, .. } = tok {
                assert!(
                    p.allowed_tags.contains(&name),
                    "tag {name} leaked: {html:?} -> {clean:?}"
                );
                for (k, val) in attrs {
                    if p.url_attrs.contains(&k) {
                        assert!(
                            is_safe_url(&val, &p.allowed_schemes),
                            "unsafe url {val:?}: {html:?} -> {clean:?}"
                        );
                    }
                    assert!(
                        p.global_attrs.contains(&k)
                            || p.tag_attrs.iter().any(|(t, a)| *t == name && *a == k),
                        "attr {k} leaked on {name}: {html:?} -> {clean:?}"
                    );
                }
            }
        }
        assert_eq!(sanitize(&clean), clean, "not idempotent: {html:?}");
    }
}

#[test]
fn pathological_inputs() {
    // Deep nesting must not overflow the stack.
    let deep = "<div>".repeat(200_000);
    let doc = parse(&deep);
    let _ = strip_tags(&deep);
    let _ = sanitize(&deep);
    let _ = query(&doc, "div div");
    // Very long attribute / text / many attributes.
    let long = format!("<p {}>x</p>", "a=1 ".repeat(100_000));
    let _ = sanitize(&long);
    let unclosed = "<a href=\"".repeat(50_000);
    let _ = parse(&unclosed);
    // Quadratic-ish comment/tag soup stays fast enough to finish.
    let soup = "<!--".repeat(50_000);
    let _ = parse(&soup);
    let lt = "<".repeat(100_000);
    let _ = strip_tags(&lt);
}
