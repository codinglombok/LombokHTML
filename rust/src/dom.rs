//! Builds a DOM-like tree from the token stream. Error-tolerant like a
//! browser parser (scope item 1/5): a stray end tag closes back up the
//! stack to the nearest matching ancestor if one exists, and is dropped
//! silently otherwise; an unclosed tag at EOF is auto-closed.

use crate::tokenizer::{is_void_element, tokenize, Token};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Document,
    Element,
    Text,
    Comment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    /// Lowercase tag name, only set for `Element`.
    pub tag: Option<String>,
    pub attrs: Vec<(String, String)>,
    /// Raw text, only set for `Text`/`Comment`.
    pub text: Option<String>,
    pub children: Vec<Node>,
}

impl Node {
    fn element(tag: String, attrs: Vec<(String, String)>) -> Self {
        Node {
            kind: NodeKind::Element,
            tag: Some(tag),
            attrs,
            text: None,
            children: Vec::new(),
        }
    }

    fn text(t: String) -> Self {
        Node {
            kind: NodeKind::Text,
            tag: None,
            attrs: Vec::new(),
            text: Some(t),
            children: Vec::new(),
        }
    }

    fn comment(t: String) -> Self {
        Node {
            kind: NodeKind::Comment,
            tag: None,
            attrs: Vec::new(),
            text: Some(t),
            children: Vec::new(),
        }
    }

    pub fn document() -> Self {
        Node {
            kind: NodeKind::Document,
            tag: None,
            attrs: Vec::new(),
            text: None,
            children: Vec::new(),
        }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn is_element(&self, tag: &str) -> bool {
        self.kind == NodeKind::Element && self.tag.as_deref() == Some(tag)
    }

    /// Depth-first, pre-order iteration over this node and all descendants.
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a Node)) {
        f(self);
        for c in &self.children {
            c.walk(f);
        }
    }

    /// First descendant (or self) matching `tag`, depth-first.
    pub fn find_first(&self, tag: &str) -> Option<&Node> {
        if self.is_element(tag) {
            return Some(self);
        }
        for c in &self.children {
            if let Some(found) = c.find_first(tag) {
                return Some(found);
            }
        }
        None
    }

    /// All descendants (not self) matching `tag`, depth-first.
    pub fn find_all(&self, tag: &str) -> Vec<&Node> {
        let mut out = Vec::new();
        for c in &self.children {
            if c.is_element(tag) {
                out.push(c);
            }
            out.extend(c.find_all(tag));
        }
        out
    }
}

/// Maximum element nesting kept in the tree; deeper start tags are attached as
/// leaves so recursive walkers can never overflow the stack (U7).
pub const MAX_DEPTH: usize = 256;

/// Parse an HTML document into a [`Node::document()`] tree.
pub fn parse(html: &str) -> Node {
    let tokens = tokenize(html);
    let mut root = Node::document();
    // Stack of indices-into-parent-children paths isn't cheap to splice in
    // a plain Vec<Node> tree, so we build with an explicit stack of owned
    // "open element" nodes and fold children in as we close tags.
    let mut stack: Vec<Node> = Vec::new();

    let push_child = |stack: &mut Vec<Node>, root: &mut Node, child: Node| {
        if let Some(top) = stack.last_mut() {
            top.children.push(child);
        } else {
            root.children.push(child);
        }
    };

    for tok in tokens {
        match tok {
            Token::Doctype(_) => { /* not represented in the tree */ }
            Token::Comment(c) => push_child(&mut stack, &mut root, Node::comment(c)),
            Token::Text(t) => push_child(&mut stack, &mut root, Node::text(t)),
            Token::StartTag {
                name,
                attrs,
                self_closing,
            } => {
                let node = Node::element(name.clone(), attrs);
                if self_closing || is_void_element(&name) || stack.len() >= MAX_DEPTH {
                    push_child(&mut stack, &mut root, node);
                } else {
                    stack.push(node);
                }
            }
            Token::EndTag { name } => {
                // Find the nearest matching open element on the stack.
                if let Some(pos) = stack
                    .iter()
                    .rposition(|n| n.tag.as_deref() == Some(name.as_str()))
                {
                    // Close everything above `pos` too (mismatched nesting):
                    // fold each closed node into its new parent in order.
                    while stack.len() > pos {
                        let closed = stack.pop().unwrap();
                        push_child(&mut stack, &mut root, closed);
                    }
                }
                // else: stray end tag with no open match — dropped.
            }
        }
    }
    // EOF with unclosed tags: fold whatever remains, innermost first.
    while let Some(closed) = stack.pop() {
        push_child(&mut stack, &mut root, closed);
    }

    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_simple_tree() {
        let doc = parse("<div><p>Hello</p></div>");
        let div = &doc.children[0];
        assert!(div.is_element("div"));
        let p = &div.children[0];
        assert!(p.is_element("p"));
        assert_eq!(p.children[0].text.as_deref(), Some("Hello"));
    }

    #[test]
    fn void_elements_have_no_children_and_stay_at_level() {
        let doc = parse("<div><img src=\"a.png\"><p>x</p></div>");
        let div = &doc.children[0];
        assert_eq!(div.children.len(), 2);
        assert!(div.children[0].is_element("img"));
        assert!(div.children[0].children.is_empty());
    }

    #[test]
    fn unclosed_tag_at_eof_is_auto_closed() {
        let doc = parse("<div><p>unterminated");
        assert!(doc.find_first("div").is_some());
        assert!(doc.find_first("p").is_some());
        let p = doc.find_first("p").unwrap();
        assert_eq!(p.children[0].text.as_deref(), Some("unterminated"));
    }

    #[test]
    fn mismatched_nesting_recovers() {
        // <b><i>text</b></i> -- browsers close <i> when </b> arrives.
        let doc = parse("<b><i>text</b></i>");
        let b = doc.find_first("b").unwrap();
        assert!(b.find_first("i").is_some());
    }

    #[test]
    fn stray_end_tag_is_dropped() {
        let doc = parse("<p>hello</span>world</p>");
        let p = doc.find_first("p").unwrap();
        // Both text runs should end up inside <p>, the stray </span> ignored.
        let texts: Vec<&str> = p
            .children
            .iter()
            .filter_map(|c| c.text.as_deref())
            .collect();
        assert_eq!(texts, alloc::vec!["hello", "world"]);
    }

    #[test]
    fn find_all_collects_every_match() {
        let doc = parse("<ul><li>a</li><li>b</li><li>c</li></ul>");
        assert_eq!(doc.find_first("ul").unwrap().find_all("li").len(), 3);
    }
}
