//! Tag stripping and structure-preserving text extraction (scope items 2
//! and 3). `strip_tags` gives clean flat text; `extract_structured_text`
//! keeps enough structure (headers, list items, paragraph/table breaks)
//! for downstream chunking.

use crate::dom::{Node, NodeKind};
use crate::tokenizer::RAW_TEXT_ELEMENTS;
use alloc::string::String;

/// Elements that contribute no visible text at all.
fn is_invisible(tag: &str) -> bool {
    RAW_TEXT_ELEMENTS.contains(&tag) || tag == "noscript" || tag == "template"
}

/// Concatenate every visible text node's content, with no structural
/// markers — closest equivalent to a browser's `textContent`, minus
/// script/style content.
pub fn strip_tags(html: &str) -> String {
    let doc = crate::dom::parse(html);
    let mut out = String::new();
    collect_text(&doc, &mut out);
    out
}

fn collect_text(node: &Node, out: &mut String) {
    if let Some(tag) = &node.tag {
        if is_invisible(tag) {
            return;
        }
    }
    match node.kind {
        NodeKind::Text => {
            if let Some(t) = &node.text {
                out.push_str(t);
            }
        }
        _ => {
            for c in &node.children {
                collect_text(c, out);
            }
        }
    }
}

/// Extract text while preserving enough structure for chunking:
/// - `h1`..`h6` are prefixed with `#`..`######` (Markdown-style) and
///   surrounded by blank lines
/// - `li` items are prefixed with `- `
/// - `p`, `div`, `tr`, `br` force a line break
/// - table cells (`td`/`th`) are tab-separated within a row
pub fn extract_structured_text(html: &str) -> String {
    let doc = crate::dom::parse(html);
    let mut out = String::new();
    walk_structured(&doc, &mut out);
    // Collapse runs of 3+ newlines down to 2 (one blank line), and trim.
    let mut collapsed = String::with_capacity(out.len());
    let mut newline_run = 0;
    for c in out.chars() {
        if c == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                collapsed.push(c);
            }
        } else {
            newline_run = 0;
            collapsed.push(c);
        }
    }
    collapsed.trim().into()
}

const HEADER_TAGS: &[(&str, &str)] = &[
    ("h1", "# "),
    ("h2", "## "),
    ("h3", "### "),
    ("h4", "#### "),
    ("h5", "##### "),
    ("h6", "###### "),
];

fn walk_structured(node: &Node, out: &mut String) {
    if let Some(tag) = &node.tag {
        if is_invisible(tag) {
            return;
        }
        if let Some((_, prefix)) = HEADER_TAGS.iter().find(|(t, _)| t == tag) {
            out.push('\n');
            out.push_str(prefix);
            for c in &node.children {
                walk_structured(c, out);
            }
            out.push('\n');
            return;
        }
        match tag.as_str() {
            "li" => {
                out.push_str("\n- ");
                for c in &node.children {
                    walk_structured(c, out);
                }
                return;
            }
            "br" => {
                out.push('\n');
                return;
            }
            "p" | "div" | "tr" | "ul" | "ol" | "table" => {
                out.push('\n');
                for c in &node.children {
                    walk_structured(c, out);
                }
                out.push('\n');
                return;
            }
            "td" | "th" => {
                for c in &node.children {
                    walk_structured(c, out);
                }
                out.push('\t');
                return;
            }
            _ => {}
        }
    }
    match node.kind {
        NodeKind::Text => {
            if let Some(t) = &node.text {
                out.push_str(t);
            }
        }
        _ => {
            for c in &node.children {
                walk_structured(c, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_tags_drops_markup_and_scripts() {
        let html = "<div><p>Hello <b>world</b></p><script>evil()</script></div>";
        assert_eq!(strip_tags(html), "Hello world");
    }

    #[test]
    fn structured_headers() {
        let html = "<h1>Title</h1><p>Body text</p>";
        let text = extract_structured_text(html);
        assert!(text.starts_with("# Title"));
        assert!(text.contains("Body text"));
    }

    #[test]
    fn structured_lists() {
        let html = "<ul><li>One</li><li>Two</li></ul>";
        let text = extract_structured_text(html);
        assert!(text.contains("- One"));
        assert!(text.contains("- Two"));
    }

    #[test]
    fn structured_table_tabs_cells() {
        let html = "<table><tr><td>A</td><td>B</td></tr></table>";
        let text = extract_structured_text(html);
        assert!(text.contains("A\tB"));
    }
}
