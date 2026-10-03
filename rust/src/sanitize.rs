//! Allowlist sanitizer (SPEC section 7).

use crate::dom::{parse, Document, NodeId, NodeKind, RAW_PARENTS, VOID};
use crate::entities::{escape_attr, escape_text};
use alloc::string::String;
use alloc::vec::Vec;

const DEFAULT_ALLOWED: &[&str] = &[
    "a",
    "abbr",
    "b",
    "blockquote",
    "br",
    "caption",
    "cite",
    "code",
    "dd",
    "del",
    "dfn",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "i",
    "img",
    "ins",
    "kbd",
    "li",
    "mark",
    "ol",
    "p",
    "pre",
    "q",
    "s",
    "samp",
    "small",
    "span",
    "strong",
    "sub",
    "sup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "u",
    "ul",
];
const DEFAULT_DROP: &[&str] = &[
    "applet",
    "audio",
    "base",
    "button",
    "canvas",
    "embed",
    "frame",
    "frameset",
    "head",
    "iframe",
    "link",
    "math",
    "meta",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "option",
    "plaintext",
    "script",
    "select",
    "style",
    "svg",
    "template",
    "textarea",
    "title",
    "video",
    "xmp",
];
const DEFAULT_ATTRS: &[(&str, &str)] = &[
    ("*", "dir"),
    ("*", "lang"),
    ("*", "title"),
    ("a", "href"),
    ("blockquote", "cite"),
    ("img", "alt"),
    ("img", "height"),
    ("img", "src"),
    ("img", "width"),
    ("ol", "start"),
    ("q", "cite"),
    ("td", "colspan"),
    ("td", "rowspan"),
    ("th", "colspan"),
    ("th", "rowspan"),
    ("th", "scope"),
    ("time", "datetime"),
];
const URL_ATTRS: &[&str] = &["cite", "href", "src"];
/// URL schemes allowed by default.
pub const DEFAULT_SCHEMES: &[&str] = &["http", "https", "mailto", "tel"];

/// Sanitizer policy: the defaults plus extra tags, attributes and schemes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    tags: Vec<String>,
    drop: Vec<String>,
    attrs: Vec<(String, String)>,
    schemes: Vec<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            tags: DEFAULT_ALLOWED.iter().map(|s| (*s).into()).collect(),
            drop: DEFAULT_DROP.iter().map(|s| (*s).into()).collect(),
            attrs: DEFAULT_ATTRS
                .iter()
                .map(|(t, a)| ((*t).into(), (*a).into()))
                .collect(),
            schemes: DEFAULT_SCHEMES.iter().map(|s| (*s).into()).collect(),
        }
    }
}

impl Policy {
    /// The default policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps `tag` (ASCII case-insensitive), also when it is on the drop list.
    /// `plaintext` is never kept: it cannot be closed again.
    pub fn allow_tag(mut self, tag: &str) -> Self {
        let t = tag.to_ascii_lowercase();
        if t == "plaintext" {
            return self;
        }
        self.drop.retain(|d| *d != t);
        if !self.tags.contains(&t) {
            self.tags.push(t);
        }
        self
    }

    /// Keeps attribute `attr` on `tag`; `tag` `"*"` means every kept element.
    pub fn allow_attr(mut self, tag: &str, attr: &str) -> Self {
        let pair = (tag.to_ascii_lowercase(), attr.to_ascii_lowercase());
        if !self.attrs.contains(&pair) {
            self.attrs.push(pair);
        }
        self
    }

    /// Accepts URLs with `scheme` in `href`, `src` and `cite`.
    pub fn allow_scheme(mut self, scheme: &str) -> Self {
        let s = scheme.to_ascii_lowercase();
        if !self.schemes.contains(&s) {
            self.schemes.push(s);
        }
        self
    }

    fn attr_ok(&self, tag: &str, attr: &str) -> bool {
        self.attrs
            .iter()
            .any(|(t, a)| a == attr && (t == tag || t == "*"))
    }
}

/// True when `url` has no scheme or one of `schemes` (lowercase), after
/// removing C0 controls, space and DEL (SPEC section 7.3).
pub fn is_safe_url_with(url: &str, schemes: &[&str]) -> bool {
    let cleaned: String = url
        .chars()
        .filter(|&c| !(c <= ' ' || c == '\u{7f}'))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    for (i, c) in cleaned.char_indices() {
        match c {
            '/' | '?' | '#' => return true,
            ':' => return schemes.contains(&&cleaned[..i]),
            _ => {}
        }
    }
    true
}

/// [`is_safe_url_with`] for the default schemes.
pub fn is_safe_url(url: &str) -> bool {
    is_safe_url_with(url, DEFAULT_SCHEMES)
}

/// Sanitizes `html` with the default policy.
pub fn sanitize(html: &str) -> String {
    sanitize_with(html, &Policy::default())
}

/// Sanitizes `html` with `policy`. The result is a fixed point: sanitizing it again returns it unchanged.
pub fn sanitize_with(html: &str, policy: &Policy) -> String {
    let doc = parse(html);
    let schemes: Vec<&str> = policy.schemes.iter().map(String::as_str).collect();
    let mut out = String::new();
    walk(&doc, doc.root(), policy, &schemes, &mut out);
    out
}

fn walk(doc: &Document, id: NodeId, p: &Policy, schemes: &[&str], out: &mut String) {
    let n = doc.node(id);
    match n.kind {
        NodeKind::Text => out.push_str(&escape_text(&n.data)),
        NodeKind::Comment => {}
        NodeKind::Document => {
            for &c in &n.children {
                walk(doc, c, p, schemes, out);
            }
        }
        NodeKind::Element => {
            let name = n.name.as_str();
            if p.drop.iter().any(|d| d == name) {
                return;
            }
            if !p.tags.iter().any(|t| t == name) {
                for &c in &n.children {
                    walk(doc, c, p, schemes, out);
                }
                return;
            }
            out.push('<');
            out.push_str(name);
            for (k, v) in &n.attrs {
                if !p.attr_ok(name, k) {
                    continue;
                }
                if URL_ATTRS.contains(&k.as_str()) && !is_safe_url_with(v, schemes) {
                    continue;
                }
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                out.push_str(&escape_attr(v));
                out.push('"');
            }
            out.push('>');
            if VOID.contains(&name) {
                return;
            }
            // Raw text content would not survive a second pass, so it is dropped.
            if !RAW_PARENTS.contains(&name) {
                for &c in &n.children {
                    walk(doc, c, p, schemes, out);
                }
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
    }
}
