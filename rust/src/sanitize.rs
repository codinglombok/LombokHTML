//! Allowlist HTML sanitizer (secure by default).
//!
//! Model: parse → walk → emit *only* what the [`Policy`] explicitly allows,
//! then re-serialize with strict escaping. Anything unknown is dropped:
//! - tags in `drop_with_content` (script, style, iframe, object, embed, svg,
//!   math, template, noscript, head, form controls …) vanish **with** their
//!   children;
//! - other non-allowed tags are *unwrapped* (children kept, tag removed);
//! - attributes not allowed for that tag are dropped (no `on*`, `style`,
//!   `class`, `id` by default);
//! - URL attributes must be relative or use an allowed scheme (default
//!   `http`, `https`, `mailto`, `tel`); control characters/whitespace inside
//!   the scheme are ignored the way browsers do (`java\tscript:` is caught);
//! - comments are dropped; text and attribute values are escaped.
//!
//! Limits: this targets HTML *fragments* rendered in an HTML context. It does
//! not sanitize CSS, SVG or MathML (those elements are removed), and no
//! sanitizer is a substitute for a Content-Security-Policy.

use crate::dom::{Node, NodeKind};
use crate::entities::{escape_attr, escape_text};
use crate::tokenizer::is_void_element;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct Policy {
    pub allowed_tags: Vec<String>,
    pub drop_with_content: Vec<String>,
    /// Attributes allowed on every allowed tag.
    pub global_attrs: Vec<String>,
    /// `(tag, attr)` pairs allowed only on that tag.
    pub tag_attrs: Vec<(String, String)>,
    /// Attributes whose value is a URL and must pass the scheme check.
    pub url_attrs: Vec<String>,
    pub allowed_schemes: Vec<String>,
}

fn v(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            allowed_tags: v(&[
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
            ]),
            drop_with_content: v(&[
                "script", "style", "iframe", "frame", "frameset", "object", "embed", "applet",
                "svg", "math", "template", "noscript", "head", "title", "select", "option",
                "textarea", "button", "audio", "video", "canvas", "base", "link", "meta",
            ]),
            global_attrs: v(&["title", "lang", "dir"]),
            tag_attrs: alloc::vec![
                ("a".into(), "href".into()),
                ("img".into(), "src".into()),
                ("img".into(), "alt".into()),
                ("img".into(), "width".into()),
                ("img".into(), "height".into()),
                ("td".into(), "colspan".into()),
                ("td".into(), "rowspan".into()),
                ("th".into(), "colspan".into()),
                ("th".into(), "rowspan".into()),
                ("th".into(), "scope".into()),
                ("ol".into(), "start".into()),
                ("blockquote".into(), "cite".into()),
                ("q".into(), "cite".into()),
                ("time".into(), "datetime".into()),
            ],
            url_attrs: v(&["href", "src", "cite"]),
            allowed_schemes: v(&["http", "https", "mailto", "tel"]),
        }
    }
}

impl Policy {
    pub fn allow_tag(mut self, tag: &str) -> Self {
        self.allowed_tags.push(tag.to_ascii_lowercase());
        self.drop_with_content.retain(|t| t != tag);
        self
    }
    pub fn allow_attr(mut self, tag: &str, attr: &str) -> Self {
        self.tag_attrs
            .push((tag.to_ascii_lowercase(), attr.to_ascii_lowercase()));
        self
    }
    pub fn allow_scheme(mut self, scheme: &str) -> Self {
        self.allowed_schemes.push(scheme.to_ascii_lowercase());
        self
    }

    fn tag_ok(&self, t: &str) -> bool {
        self.allowed_tags.iter().any(|a| a == t)
    }
    fn drops(&self, t: &str) -> bool {
        self.drop_with_content.iter().any(|a| a == t)
    }
    fn attr_ok(&self, tag: &str, attr: &str) -> bool {
        self.global_attrs.iter().any(|a| a == attr)
            || self.tag_attrs.iter().any(|(t, a)| t == tag && a == attr)
    }
}

/// True if `value` is a relative URL or uses an allowed scheme.
pub fn is_safe_url(value: &str, allowed_schemes: &[String]) -> bool {
    // Browsers ignore ASCII whitespace/control characters anywhere in the scheme.
    let cleaned: String = value
        .chars()
        .filter(|c| !(*c <= ' ' || *c == '\u{7f}'))
        .collect::<String>()
        .to_ascii_lowercase();
    match cleaned.find([':', '/', '?', '#']) {
        Some(i) if cleaned.as_bytes()[i] == b':' => {
            let scheme = &cleaned[..i];
            allowed_schemes.iter().any(|s| s == scheme)
        }
        _ => true, // relative reference (no scheme before first / ? #)
    }
}

fn clean(node: &Node, policy: &Policy, out: &mut String) {
    match node.kind {
        NodeKind::Text => {
            if let Some(t) = &node.text {
                out.push_str(&escape_text(t));
            }
        }
        NodeKind::Comment => {}
        NodeKind::Document => {
            for c in &node.children {
                clean(c, policy, out);
            }
        }
        NodeKind::Element => {
            let tag = node.tag.as_deref().unwrap_or("");
            if policy.drops(tag) {
                return;
            }
            if !policy.tag_ok(tag) {
                // Unwrap: keep children, drop the tag itself.
                for c in &node.children {
                    clean(c, policy, out);
                }
                return;
            }
            out.push('<');
            out.push_str(tag);
            let mut seen: Vec<&str> = Vec::new();
            for (k, val) in &node.attrs {
                if seen.contains(&k.as_str()) || !policy.attr_ok(tag, k) {
                    continue;
                }
                if policy.url_attrs.iter().any(|u| u == k)
                    && !is_safe_url(val, &policy.allowed_schemes)
                {
                    continue;
                }
                seen.push(k.as_str());
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                out.push_str(&escape_attr(val));
                out.push('"');
            }
            if is_void_element(tag) {
                out.push_str(" />");
            } else {
                out.push('>');
                for c in &node.children {
                    clean(c, policy, out);
                }
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
        }
    }
}

/// Sanitize with an explicit policy.
pub fn sanitize_with(html: &str, policy: &Policy) -> String {
    let doc = crate::dom::parse(html);
    let mut out = String::new();
    clean(&doc, policy, &mut out);
    out
}

/// Sanitize with the default (strict) allowlist policy.
pub fn sanitize(html: &str) -> String {
    sanitize_with(html, &Policy::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_script_style_and_content() {
        let c = sanitize("<div><script>alert(1)</script><style>b{}</style><p>safe</p></div>");
        assert_eq!(c, "<div><p>safe</p></div>");
    }

    #[test]
    fn strips_event_handlers_and_style() {
        let c = sanitize(r#"<p onclick="x()" style="color:red" class="a" id="b" title="t">hi</p>"#);
        assert_eq!(c, r#"<p title="t">hi</p>"#);
    }

    #[test]
    fn blocks_dangerous_urls_including_obfuscated() {
        for bad in [
            "javascript:alert(1)",
            "  JaVaScRiPt:alert(1)",
            "java\tscript:alert(1)",
            "java\nscript:alert(1)",
            "vbscript:x",
            "data:text/html,<script>1</script>",
            "&#106;avascript:alert(1)", // decoded by tokenizer before the check
        ] {
            let c = sanitize(&alloc::format!("<a href=\"{}\">x</a>", bad));
            assert_eq!(c, "<a>x</a>", "leaked: {}", bad);
        }
    }

    #[test]
    fn keeps_safe_and_relative_urls() {
        assert!(sanitize(r#"<a href="https://e.com/a?b=1#c">x</a>"#).contains("https://e.com"));
        assert!(sanitize(r#"<a href="/rel/path">x</a>"#).contains("/rel/path"));
        assert!(sanitize(r#"<a href="mailto:a@b.co">x</a>"#).contains("mailto:"));
        assert!(sanitize(r#"<a href="page.html?x=a:b">x</a>"#).contains("page.html"));
    }

    #[test]
    fn unwraps_unknown_tags_keeps_text() {
        assert_eq!(sanitize("<section><custom-x>hi</custom-x></section>"), "hi");
    }

    #[test]
    fn drops_svg_math_iframe_form_controls_with_content() {
        for t in [
            "svg", "math", "iframe", "object", "template", "noscript", "textarea",
        ] {
            let c = sanitize(&alloc::format!("a<{0}>evil</{0}>b", t));
            assert_eq!(c, "ab", "tag {}", t);
        }
    }

    #[test]
    fn void_embed_is_removed() {
        assert_eq!(sanitize("a<embed src=x>b"), "ab");
    }

    #[test]
    fn escapes_text_and_attributes() {
        let c = sanitize(r#"<p title='a" onmouseover="x'>&lt;script&gt; 5 < 10</p>"#);
        assert!(!c.contains("<script"));
        assert!(c.contains("&lt;script&gt;"));
        assert!(c.contains("&quot;"));
        assert!(!c.contains("\" onmouseover"));
    }

    #[test]
    fn img_src_scheme_checked_and_dedup_attrs() {
        assert_eq!(
            sanitize(r#"<img src="javascript:x" alt="a" alt="b">"#),
            r#"<img alt="a" />"#
        );
    }

    #[test]
    fn drops_comments_and_head() {
        assert_eq!(
            sanitize("<!-- c --><html><head><title>t</title></head><body><b>x</b></body></html>"),
            "<b>x</b>"
        );
    }

    #[test]
    fn custom_policy_can_allow_more() {
        let p = Policy::default().allow_attr("p", "class");
        assert_eq!(
            sanitize_with(r#"<p class="k">x</p>"#, &p),
            r#"<p class="k">x</p>"#
        );
    }

    #[test]
    fn idempotent() {
        let inputs = [
            "<p>hi <b>there</b></p>",
            "<a href=\"x\">a<script>1</script></a>",
            "<p title='q\"'>&amp;&lt;</p>",
            "<div><div><p>5 < 10 & 20 > 5</p></div></div>",
        ];
        for i in inputs {
            let once = sanitize(i);
            assert_eq!(sanitize(&once), once, "not idempotent for {}", i);
        }
    }
}
