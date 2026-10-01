//! CSS selector query: compound selectors (`tag`, `*`, `.class`, `#id`,
//! `[attr]`, `[attr=v]`, `[attr^=v]`, `[attr$=v]`, `[attr*=v]`, `[attr~=v]`),
//! the descendant (` `) and child (`>`) combinators, and comma-separated
//! selector lists. No sibling combinators or pseudo-classes (documented).
//!
//! Matching is relative to the queried subtree: ancestors above the query
//! root are not considered.

use crate::dom::{Node, NodeKind};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum AttrOp {
    Exists,
    Equals,
    Prefix,
    Suffix,
    Contains,
    Word,
}

/// One compound selector, e.g. `div.card#main[data-x=1]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Selector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, AttrOp, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combinator {
    Descendant,
    Child,
}

/// A complex selector: compounds joined by combinators (`a > b c`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Complex {
    /// `parts[0].0` is ignored (leading compound has no combinator).
    pub parts: Vec<(Combinator, Selector)>,
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Parse a single compound selector (no combinators).
pub fn parse_selector(sel: &str) -> Selector {
    let mut s = Selector::default();
    let chars: Vec<char> = sel.trim().chars().collect();
    let mut i = 0;
    let start = i;
    if chars.first() == Some(&'*') {
        i += 1;
    } else {
        while i < chars.len() && is_ident(chars[i]) {
            i += 1;
        }
        if i > start {
            s.tag = Some(
                chars[start..i]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase(),
            );
        }
    }
    while i < chars.len() {
        match chars[i] {
            '#' | '.' => {
                let kind = chars[i];
                i += 1;
                let st = i;
                while i < chars.len() && is_ident(chars[i]) {
                    i += 1;
                }
                let name: String = chars[st..i].iter().collect();
                if kind == '#' {
                    s.id = Some(name);
                } else {
                    s.classes.push(name);
                }
            }
            '[' => {
                i += 1;
                let st = i;
                while i < chars.len() && !matches!(chars[i], '=' | ']' | '^' | '$' | '*' | '~') {
                    i += 1;
                }
                let name = chars[st..i]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_ascii_lowercase();
                let mut op = AttrOp::Exists;
                match chars.get(i) {
                    Some('=') => {
                        op = AttrOp::Equals;
                        i += 1;
                    }
                    Some(c @ ('^' | '$' | '*' | '~')) if chars.get(i + 1) == Some(&'=') => {
                        op = match c {
                            '^' => AttrOp::Prefix,
                            '$' => AttrOp::Suffix,
                            '*' => AttrOp::Contains,
                            _ => AttrOp::Word,
                        };
                        i += 2;
                    }
                    _ => {}
                }
                let mut value = String::new();
                if op != AttrOp::Exists {
                    let quote = match chars.get(i) {
                        Some(q @ ('"' | '\'')) => {
                            i += 1;
                            Some(*q)
                        }
                        _ => None,
                    };
                    let vs = i;
                    while i < chars.len() && chars[i] != ']' && Some(chars[i]) != quote {
                        i += 1;
                    }
                    value = chars[vs..i].iter().collect();
                    if quote.is_some() && i < chars.len() {
                        i += 1;
                    }
                }
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
                i += 1;
                s.attrs.push((name, op, value));
            }
            _ => i += 1,
        }
    }
    s
}

/// Parse `a > b c, d` into a list of complex selectors.
pub fn parse_selector_list(sel: &str) -> Vec<Complex> {
    sel.split(',')
        .map(|part| {
            let mut complex = Complex::default();
            let mut comb = Combinator::Descendant;
            let mut buf = String::new();
            let flush = |buf: &mut String, comb: Combinator, complex: &mut Complex| {
                if !buf.is_empty() {
                    complex.parts.push((comb, parse_selector(buf)));
                    buf.clear();
                }
            };
            let mut pending_child = false;
            for tok in part.replace('>', " > ").split_whitespace() {
                if tok == ">" {
                    flush(&mut buf, comb, &mut complex);
                    pending_child = true;
                    continue;
                }
                buf.push_str(tok);
                let c = if pending_child {
                    Combinator::Child
                } else {
                    Combinator::Descendant
                };
                pending_child = false;
                comb = c;
                flush(&mut buf, comb, &mut complex);
            }
            complex
        })
        .filter(|c| !c.parts.is_empty())
        .collect()
}

fn attr_matches(node: &Node, name: &str, op: &AttrOp, expected: &str) -> bool {
    let Some(actual) = node.attr(name) else {
        return false;
    };
    match op {
        AttrOp::Exists => true,
        AttrOp::Equals => actual == expected,
        AttrOp::Prefix => !expected.is_empty() && actual.starts_with(expected),
        AttrOp::Suffix => !expected.is_empty() && actual.ends_with(expected),
        AttrOp::Contains => !expected.is_empty() && actual.contains(expected),
        AttrOp::Word => actual.split_whitespace().any(|w| w == expected),
    }
}

fn compound_matches(node: &Node, sel: &Selector) -> bool {
    if node.kind != NodeKind::Element {
        return false;
    }
    if let Some(tag) = &sel.tag {
        if node.tag.as_deref() != Some(tag.as_str()) {
            return false;
        }
    }
    if let Some(id) = &sel.id {
        if node.attr("id") != Some(id.as_str()) {
            return false;
        }
    }
    if !sel.classes.is_empty() {
        let cls: Vec<&str> = node
            .attr("class")
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if !sel.classes.iter().all(|c| cls.contains(&c.as_str())) {
            return false;
        }
    }
    sel.attrs
        .iter()
        .all(|(n, op, v)| attr_matches(node, n, op, v))
}

/// Does `node` (with element `ancestors`, outermost first) match parts[..=idx]?
fn complex_matches(
    parts: &[(Combinator, Selector)],
    idx: usize,
    node: &Node,
    ancestors: &[&Node],
) -> bool {
    if !compound_matches(node, &parts[idx].1) {
        return false;
    }
    if idx == 0 {
        return true;
    }
    match parts[idx].0 {
        Combinator::Child => match ancestors.split_last() {
            Some((parent, rest)) => complex_matches(parts, idx - 1, parent, rest),
            None => false,
        },
        Combinator::Descendant => (0..ancestors.len())
            .rev()
            .any(|k| complex_matches(parts, idx - 1, ancestors[k], &ancestors[..k])),
    }
}

/// Query every descendant of `root` matching `selector` (list allowed),
/// in document order.
pub fn query<'a>(root: &'a Node, selector: &str) -> Vec<&'a Node> {
    let list = parse_selector_list(selector);
    let mut out = Vec::new();
    if list.is_empty() {
        return out;
    }
    let mut ancestors: Vec<&Node> = Vec::new();
    walk(root, &list, &mut ancestors, &mut out);
    out
}

fn walk<'a>(
    node: &'a Node,
    list: &[Complex],
    ancestors: &mut Vec<&'a Node>,
    out: &mut Vec<&'a Node>,
) {
    for c in &node.children {
        if c.kind == NodeKind::Element
            && list
                .iter()
                .any(|cx| complex_matches(&cx.parts, cx.parts.len() - 1, c, ancestors))
        {
            out.push(c);
        }
        if c.kind == NodeKind::Element {
            ancestors.push(c);
            walk(c, list, ancestors, out);
            ancestors.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse;

    #[test]
    fn tag_id_class_attr() {
        let doc = parse(r#"<div id="main" class="card x"><p class="card">a</p></div><p>b</p>"#);
        assert_eq!(query(&doc, "p").len(), 2);
        assert_eq!(query(&doc, "#main").len(), 1);
        assert_eq!(query(&doc, ".card").len(), 2);
        assert_eq!(query(&doc, "div.card#main").len(), 1);
        assert_eq!(query(&doc, "*").len(), 3);
    }

    #[test]
    fn attribute_operators() {
        let doc = parse(
            r#"<a href="https://x.com/a.pdf" rel="nofollow noopener">l</a><a href="/b">m</a>"#,
        );
        assert_eq!(query(&doc, "[href]").len(), 2);
        assert_eq!(query(&doc, "[href^=https]").len(), 1);
        assert_eq!(query(&doc, "[href$='.pdf']").len(), 1);
        assert_eq!(query(&doc, "[href*=x.com]").len(), 1);
        assert_eq!(query(&doc, "[rel~=noopener]").len(), 1);
        assert_eq!(query(&doc, "[rel~=nofollow]").len(), 1);
        assert_eq!(query(&doc, "[href='/b']").len(), 1);
    }

    #[test]
    fn descendant_and_child_combinators() {
        let doc = parse("<div><section><p>deep</p></section><p>direct</p></div><p>outside</p>");
        assert_eq!(query(&doc, "div p").len(), 2);
        assert_eq!(query(&doc, "div > p").len(), 1);
        assert_eq!(query(&doc, "div>p").len(), 1);
        assert_eq!(query(&doc, "div > section > p").len(), 1);
        assert_eq!(query(&doc, "section p").len(), 1);
    }

    #[test]
    fn selector_list_in_document_order() {
        let doc = parse("<h1>a</h1><p>b</p><h2>c</h2>");
        let r = query(&doc, "h2, h1");
        assert_eq!(r.len(), 2);
        assert!(r[0].is_element("h1"));
    }

    #[test]
    fn descendant_backtracking() {
        let doc = parse("<div class=a><div class=b><span>x</span></div></div>");
        assert_eq!(query(&doc, ".a span").len(), 1);
        assert_eq!(query(&doc, ".a > span").len(), 0);
        assert_eq!(query(&doc, ".a > .b > span").len(), 1);
    }

    #[test]
    fn garbage_selectors_do_not_panic() {
        let doc = parse("<p>x</p>");
        for s in [
            "", ">", ",,,", "[", "[a", "[a=", "#", ".", "a >", "> a", "[=]", "a[b^=]",
        ] {
            let _ = query(&doc, s);
        }
    }
}
