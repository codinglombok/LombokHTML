//! Page metadata (SPEC section 9).

use crate::dom::{Document, NodeKind};
use alloc::string::String;
use alloc::vec::Vec;

/// Metadata found in a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageMeta {
    /// Text of the first `title` element, whitespace collapsed; `None` when absent or blank.
    pub title: Option<String>,
    /// `content` of the first `<meta name="description">`.
    pub description: Option<String>,
    /// `href` of the first `<link rel="canonical">`.
    pub canonical: Option<String>,
    /// `lang` of the first `html` element that has one.
    pub lang: Option<String>,
    /// `(property without "og:", content)` for every `<meta property="og:...">`, in order.
    pub og: Vec<(String, String)>,
}

pub(crate) fn is_ws(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{c}' | '\r' | ' ')
}

/// Collapses runs of ASCII whitespace to one space and trims spaces.
pub(crate) fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split(is_ws).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

impl Document {
    /// Extracts [`PageMeta`] (SPEC section 9).
    pub fn meta(&self) -> PageMeta {
        let mut out = PageMeta::default();
        let mut seen_title = false;
        for id in self.elements() {
            let el = self.node(id);
            match el.name.as_str() {
                "title" if !seen_title => {
                    seen_title = true;
                    let mut raw = String::new();
                    for &c in &el.children {
                        let n = self.node(c);
                        if n.kind == NodeKind::Text {
                            raw.push_str(&n.data);
                        }
                    }
                    let t = collapse(&raw);
                    out.title = if t.is_empty() { None } else { Some(t) };
                }
                "html" => {
                    if out.lang.is_none() {
                        out.lang = el.attr("lang").map(String::from);
                    }
                }
                "meta" => {
                    let Some(content) = el.attr("content") else {
                        continue;
                    };
                    let name = el.attr("name").unwrap_or("").to_ascii_lowercase();
                    if name == "description" && out.description.is_none() {
                        out.description = Some(content.into());
                    }
                    let prop = el.attr("property").unwrap_or("");
                    if prop.len() >= 3 && prop.as_bytes()[..3].eq_ignore_ascii_case(b"og:") {
                        out.og.push((prop[3..].into(), content.into()));
                    }
                }
                "link" if out.canonical.is_none() => {
                    let rel = el.attr("rel").unwrap_or("").to_ascii_lowercase();
                    if rel.split(is_ws).any(|r| r == "canonical") {
                        out.canonical = el.attr("href").map(String::from);
                    }
                }
                _ => {}
            }
        }
        out
    }
}
