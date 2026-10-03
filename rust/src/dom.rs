//! Tree construction subset, serialization and text extraction (SPEC sections 4 to 6).

use crate::entities::{escape_attr, escape_text};
use crate::tokenizer::{tokenize, Token};
use alloc::string::String;
use alloc::vec::Vec;

/// Index of a node inside its [`Document`].
pub type NodeId = usize;

/// What a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Element,
    Text,
    Comment,
}

/// One node. `name` and `attrs` are set for elements, `data` for text and comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub data: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

impl Node {
    fn new(kind: NodeKind, name: String, attrs: Vec<(String, String)>, data: String) -> Node {
        Node {
            kind,
            name,
            attrs,
            data,
            parent: None,
            children: Vec::new(),
        }
    }

    /// Value of the first attribute called `name`.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// True for an element called `name`.
    pub fn is(&self, name: &str) -> bool {
        self.kind == NodeKind::Element && self.name == name
    }
}

/// A parsed document. Node 0 is the document node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    nodes: Vec<Node>,
}

pub(crate) const VOID: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
    "keygen", "link", "meta", "param", "source", "track", "wbr",
];
pub(crate) const RAW_PARENTS: &[&str] = &[
    "style",
    "script",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "plaintext",
    "noscript",
];
const SPECIAL: &[&str] = &[
    "address",
    "applet",
    "area",
    "article",
    "aside",
    "base",
    "basefont",
    "bgsound",
    "blockquote",
    "body",
    "br",
    "button",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dir",
    "div",
    "dl",
    "dt",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "iframe",
    "img",
    "input",
    "keygen",
    "li",
    "link",
    "listing",
    "main",
    "marquee",
    "menu",
    "meta",
    "nav",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "ol",
    "p",
    "param",
    "plaintext",
    "pre",
    "script",
    "search",
    "section",
    "select",
    "source",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
    "wbr",
    "xmp",
];
const P_CLOSERS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "dd",
    "dt",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "plaintext",
    "pre",
    "search",
    "section",
    "summary",
    "table",
    "ul",
    "xmp",
];
pub(crate) const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];
const SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
];
const BUTTON_SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "button",
];
const LIST_SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "ol", "ul",
];
const TABLE_SCOPE: &[&str] = &["html", "table", "template"];
const TABLE_PARTS: &[&str] = &[
    "table", "caption", "tbody", "thead", "tfoot", "tr", "td", "th",
];
const HIDDEN: &[&str] = &["script", "style", "noscript", "template", "title"];
const BLOCKS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "caption",
    "details",
    "dialog",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "header",
    "hgroup",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "ul",
];
const SECTIONS: &[&str] = &["thead", "tbody", "tfoot"];

/// Maximum number of open elements; deeper start tags are ignored.
pub const MAX_DEPTH: usize = 256;

fn has(set: &[&str], name: &str) -> bool {
    set.contains(&name)
}

struct Builder {
    doc: Document,
    stack: Vec<NodeId>,
}

impl Builder {
    fn name(&self, id: NodeId) -> &str {
        &self.doc.nodes[id].name
    }

    fn current(&self) -> NodeId {
        self.stack.last().copied().unwrap_or(0)
    }

    fn append(&mut self, node: Node) -> NodeId {
        let parent = self.current();
        let id = self.doc.nodes.len();
        let mut node = node;
        node.parent = Some(parent);
        self.doc.nodes.push(node);
        self.doc.nodes[parent].children.push(id);
        id
    }

    fn add_text(&mut self, data: &str) {
        let parent = self.current();
        if let Some(&last) = self.doc.nodes[parent].children.last() {
            if self.doc.nodes[last].kind == NodeKind::Text {
                self.doc.nodes[last].data.push_str(data);
                return;
            }
        }
        self.append(Node::new(
            NodeKind::Text,
            String::new(),
            Vec::new(),
            data.into(),
        ));
    }

    fn in_scope(&self, names: &[&str], boundary: &[&str]) -> bool {
        for &id in self.stack.iter().rev() {
            let n = self.name(id);
            if has(names, n) {
                return true;
            }
            if has(boundary, n) {
                return false;
            }
        }
        false
    }

    fn pop_until(&mut self, names: &[&str]) {
        while let Some(id) = self.stack.pop() {
            if has(names, self.name(id)) {
                return;
            }
        }
    }

    /// Returns true when a following newline is dropped.
    fn start(&mut self, name: String, attrs: Vec<(String, String)>) -> bool {
        let n = name.as_str();
        if n == "li" || n == "dd" || n == "dt" {
            let targets: &[&str] = if n == "li" { &["li"] } else { &["dd", "dt"] };
            for idx in (0..self.stack.len()).rev() {
                let cur = self.name(self.stack[idx]);
                if has(targets, cur) {
                    self.stack.truncate(idx);
                    break;
                }
                if has(SPECIAL, cur) && !matches!(cur, "address" | "div" | "p") {
                    break;
                }
            }
        }
        if has(P_CLOSERS, n) && self.in_scope(&["p"], BUTTON_SCOPE) {
            self.pop_until(&["p"]);
        }
        if has(HEADINGS, n)
            && self
                .stack
                .last()
                .is_some_and(|&t| has(HEADINGS, self.name(t)))
        {
            self.stack.pop();
        }
        if (n == "option" || n == "optgroup")
            && self.stack.last().is_some_and(|&t| self.name(t) == "option")
        {
            self.stack.pop();
        }
        if n == "a" && self.stack.iter().any(|&t| self.name(t) == "a") {
            self.pop_until(&["a"]);
        }
        if matches!(n, "td" | "th" | "tr" | "thead" | "tbody" | "tfoot")
            && self.in_scope(&["td", "th"], TABLE_SCOPE)
        {
            self.pop_until(&["td", "th"]);
        }
        if matches!(n, "tr" | "thead" | "tbody" | "tfoot") && self.in_scope(&["tr"], TABLE_SCOPE) {
            self.pop_until(&["tr"]);
        }
        if has(SECTIONS, n) && self.in_scope(SECTIONS, TABLE_SCOPE) {
            self.pop_until(SECTIONS);
        }
        if self.stack.len() >= MAX_DEPTH {
            return false;
        }
        let skip = matches!(n, "pre" | "listing" | "textarea");
        let void = has(VOID, n);
        let id = self.append(Node::new(NodeKind::Element, name, attrs, String::new()));
        if !void {
            self.stack.push(id);
        }
        skip
    }

    fn end(&mut self, name: &str) {
        if name == "br" {
            self.start("br".into(), Vec::new());
            return;
        }
        let (target, boundary): (&[&str], &[&str]) = if name == "p" {
            (&["p"], BUTTON_SCOPE)
        } else if has(HEADINGS, name) {
            (HEADINGS, SCOPE)
        } else if name == "li" {
            (&["li"], LIST_SCOPE)
        } else if name == "dd" || name == "dt" || has(SPECIAL, name) && !has(TABLE_PARTS, name) {
            (core::slice::from_ref(&name), SCOPE)
        } else if has(TABLE_PARTS, name) {
            (core::slice::from_ref(&name), TABLE_SCOPE)
        } else {
            for idx in (0..self.stack.len()).rev() {
                let cur = self.name(self.stack[idx]);
                if cur == name {
                    self.stack.truncate(idx);
                    return;
                }
                if has(SPECIAL, cur) {
                    return;
                }
            }
            return;
        };
        if self.in_scope(target, boundary) {
            self.pop_until(target);
        }
    }
}

/// Parses `html` into a [`Document`] (SPEC section 4). Never fails.
pub fn parse(html: &str) -> Document {
    let mut b = Builder {
        doc: Document {
            nodes: alloc::vec![Node::new(
                NodeKind::Document,
                String::new(),
                Vec::new(),
                String::new()
            )],
        },
        stack: Vec::new(),
    };
    let mut skip_newline = false;
    for tok in tokenize(html) {
        match tok {
            Token::Character(data) => {
                let d = if skip_newline {
                    data.strip_prefix('\n').unwrap_or(&data)
                } else {
                    &data
                };
                skip_newline = false;
                if !d.is_empty() {
                    b.add_text(d);
                }
            }
            Token::StartTag { name, attrs, .. } => skip_newline = b.start(name, attrs),
            Token::EndTag { name } => {
                skip_newline = false;
                b.end(&name);
            }
            Token::Comment(data) => {
                skip_newline = false;
                b.append(Node::new(
                    NodeKind::Comment,
                    String::new(),
                    Vec::new(),
                    data,
                ));
            }
            Token::Doctype { .. } => skip_newline = false,
        }
    }
    b.doc
}

impl Document {
    /// The document node.
    pub fn root(&self) -> NodeId {
        0
    }

    /// The node with index `id`. Panics when `id` is out of range.
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    /// Number of nodes, including the document node.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True when the document has no children.
    pub fn is_empty(&self) -> bool {
        self.nodes[0].children.is_empty()
    }

    /// Elements below `id` in document order.
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut todo: Vec<NodeId> = self.nodes[id].children.iter().rev().copied().collect();
        while let Some(n) = todo.pop() {
            if self.nodes[n].kind == NodeKind::Element {
                out.push(n);
                todo.extend(self.nodes[n].children.iter().rev().copied());
            }
        }
        out
    }

    /// All elements in document order.
    pub fn elements(&self) -> Vec<NodeId> {
        self.descendants(0)
    }

    /// Element children of the parent of `id`, `id` included.
    pub(crate) fn element_siblings(&self, id: NodeId) -> Vec<NodeId> {
        match self.nodes[id].parent {
            Some(p) => self.nodes[p]
                .children
                .iter()
                .copied()
                .filter(|&c| self.nodes[c].kind == NodeKind::Element)
                .collect(),
            None => alloc::vec![id],
        }
    }

    /// HTML serialization of `id` (SPEC section 5): the outer HTML of an
    /// element, the inner HTML of the document.
    pub fn serialize(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.write(id, &mut out);
        out
    }

    fn write(&self, id: NodeId, out: &mut String) {
        let n = &self.nodes[id];
        match n.kind {
            NodeKind::Text => {
                let raw = n.parent.is_some_and(|p| {
                    let pn = &self.nodes[p];
                    pn.kind == NodeKind::Element && has(RAW_PARENTS, &pn.name)
                });
                if raw {
                    out.push_str(&n.data);
                } else {
                    out.push_str(&escape_text(&n.data));
                }
            }
            NodeKind::Comment => {
                out.push_str("<!--");
                out.push_str(&n.data);
                out.push_str("-->");
            }
            NodeKind::Element => {
                out.push('<');
                out.push_str(&n.name);
                for (k, v) in &n.attrs {
                    out.push(' ');
                    out.push_str(k);
                    out.push_str("=\"");
                    out.push_str(&escape_attr(v));
                    out.push('"');
                }
                out.push('>');
                if has(VOID, &n.name) {
                    return;
                }
                for &c in &n.children {
                    self.write(c, out);
                }
                out.push_str("</");
                out.push_str(&n.name);
                out.push('>');
            }
            NodeKind::Document => {
                for &c in &n.children {
                    self.write(c, out);
                }
            }
        }
    }

    /// Concatenated text below `id`, skipping script, style, noscript,
    /// template and title (SPEC section 6.1).
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.text_into(id, &mut out);
        out
    }

    fn text_into(&self, id: NodeId, out: &mut String) {
        let n = &self.nodes[id];
        match n.kind {
            NodeKind::Text => out.push_str(&n.data),
            NodeKind::Comment => {}
            NodeKind::Element if has(HIDDEN, &n.name) => {}
            _ => {
                for &c in &n.children {
                    self.text_into(c, out);
                }
            }
        }
    }

    /// Structure-preserving plain text of `id` (SPEC section 6.2).
    pub fn extract_text(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.extract_into(id, false, &mut out);
        let mut lines = String::with_capacity(out.len());
        for (i, line) in out.split('\n').enumerate() {
            if i > 0 {
                lines.push('\n');
            }
            lines.push_str(line.trim_end_matches([' ', '\t']));
        }
        let mut squeezed = String::with_capacity(lines.len());
        let mut newlines = 0;
        for c in lines.chars() {
            if c == '\n' {
                newlines += 1;
                if newlines <= 2 {
                    squeezed.push(c);
                }
            } else {
                newlines = 0;
                squeezed.push(c);
            }
        }
        squeezed.trim_matches('\n').into()
    }

    fn extract_into(&self, id: NodeId, pre: bool, out: &mut String) {
        let n = &self.nodes[id];
        match n.kind {
            NodeKind::Text => {
                if pre {
                    out.push_str(&n.data);
                    return;
                }
                let mut collapsed = String::with_capacity(n.data.len());
                let mut in_ws = false;
                for c in n.data.chars() {
                    if matches!(c, '\t' | '\n' | '\u{c}' | '\r' | ' ') {
                        if !in_ws {
                            collapsed.push(' ');
                        }
                        in_ws = true;
                    } else {
                        collapsed.push(c);
                        in_ws = false;
                    }
                }
                let at_break = matches!(out.chars().last(), None | Some(' ' | '\n' | '\t'));
                let text = if at_break {
                    collapsed.strip_prefix(' ').unwrap_or(&collapsed)
                } else {
                    &collapsed
                };
                out.push_str(text);
            }
            NodeKind::Comment => {}
            NodeKind::Document => {
                for &c in &n.children {
                    self.extract_into(c, pre, out);
                }
            }
            NodeKind::Element => {
                let name = n.name.as_str();
                if has(HIDDEN, name) {
                    return;
                }
                if has(HEADINGS, name) {
                    out.push_str("\n\n");
                    let level = (name.as_bytes()[1] - b'0') as usize;
                    for _ in 0..level {
                        out.push('#');
                    }
                    out.push(' ');
                    for &c in &n.children {
                        self.extract_into(c, pre, out);
                    }
                    out.push_str("\n\n");
                    return;
                }
                match name {
                    "li" => out.push_str("\n- "),
                    "dd" | "dt" | "tr" => out.push('\n'),
                    "br" => {
                        out.push('\n');
                        return;
                    }
                    "hr" => {
                        out.push_str("\n\n---\n\n");
                        return;
                    }
                    _ => {}
                }
                let block = has(BLOCKS, name);
                if block {
                    out.push_str("\n\n");
                }
                let inner_pre = pre || matches!(name, "pre" | "listing" | "textarea");
                for &c in &n.children {
                    self.extract_into(c, inner_pre, out);
                }
                if name == "td" || name == "th" {
                    out.push('\t');
                }
                if block {
                    out.push_str("\n\n");
                }
            }
        }
    }
}
