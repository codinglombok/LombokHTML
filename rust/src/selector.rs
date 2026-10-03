//! CSS selector subset (SPEC section 8).

use crate::dom::{Document, NodeId, NodeKind};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// A selector that does not follow the SPEC grammar. `offset` counts
/// characters from the start of the selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectorError {
    pub offset: usize,
}

impl SelectorError {
    /// The error code shared by every port.
    pub fn code(&self) -> &'static str {
        "BAD_SELECTOR"
    }
}

impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BAD_SELECTOR at character {}", self.offset)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SelectorError {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Simple {
    Id(String),
    Class(String),
    Attr(String, Option<(String, String)>),
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    NthChild(i64, i64),
    NthLastChild(i64, i64),
    Not(Box<Compound>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Compound {
    tag: Option<String>,
    simple: Vec<Simple>,
}

type Complex = Vec<(char, Compound)>;

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || !c.is_ascii()
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

struct Parser {
    s: Vec<char>,
    i: usize,
}

type R<T> = Result<T, SelectorError>;

impl Parser {
    fn fail<T>(&self) -> R<T> {
        Err(SelectorError { offset: self.i })
    }

    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn ws(&mut self) -> bool {
        let start = self.i;
        while self.peek().is_some_and(is_ws) {
            self.i += 1;
        }
        self.i > start
    }

    fn ident(&mut self) -> R<String> {
        let start = self.i;
        while self.peek().is_some_and(is_ident) {
            self.i += 1;
        }
        if self.i == start {
            return self.fail();
        }
        Ok(self.s[start..self.i].iter().collect())
    }

    fn list(&mut self) -> R<Vec<Complex>> {
        let mut out = Vec::new();
        loop {
            self.ws();
            out.push(self.complex()?);
            self.ws();
            match self.peek() {
                None => return Ok(out),
                Some(',') => self.i += 1,
                Some(_) => return self.fail(),
            }
        }
    }

    fn complex(&mut self) -> R<Complex> {
        let mut parts = alloc::vec![('\0', self.compound()?)];
        loop {
            let save = self.i;
            let had_ws = self.ws();
            match self.peek() {
                None | Some(',') => {
                    self.i = save;
                    return Ok(parts);
                }
                Some(c @ ('>' | '+' | '~')) => {
                    self.i += 1;
                    self.ws();
                    parts.push((c, self.compound()?));
                }
                Some(_) if had_ws => parts.push((' ', self.compound()?)),
                Some(_) => return self.fail(),
            }
        }
    }

    fn compound(&mut self) -> R<Compound> {
        let mut comp = Compound {
            tag: None,
            simple: Vec::new(),
        };
        match self.peek() {
            Some('*') => {
                self.i += 1;
                comp.tag = Some("*".into());
            }
            Some(c) if is_ident(c) => comp.tag = Some(self.ident()?.to_ascii_lowercase()),
            _ => {}
        }
        while let Some(c) = self.peek() {
            match c {
                '#' => {
                    self.i += 1;
                    comp.simple.push(Simple::Id(self.ident()?));
                }
                '.' => {
                    self.i += 1;
                    comp.simple.push(Simple::Class(self.ident()?));
                }
                '[' => {
                    self.i += 1;
                    self.ws();
                    let name = self.ident()?.to_ascii_lowercase();
                    self.ws();
                    let op = match (self.peek(), self.s.get(self.i + 1).copied()) {
                        (Some('='), _) => {
                            self.i += 1;
                            Some("=")
                        }
                        (Some(a @ ('~' | '|' | '^' | '$' | '*')), Some('=')) => {
                            self.i += 2;
                            Some(match a {
                                '~' => "~=",
                                '|' => "|=",
                                '^' => "^=",
                                '$' => "$=",
                                _ => "*=",
                            })
                        }
                        _ => None,
                    };
                    let mut test = None;
                    if let Some(op) = op {
                        self.ws();
                        let val = match self.peek() {
                            Some(q @ ('\'' | '"')) => {
                                let end = match self.s[self.i + 1..].iter().position(|&x| x == q) {
                                    Some(e) => self.i + 1 + e,
                                    None => return self.fail(),
                                };
                                let v: String = self.s[self.i + 1..end].iter().collect();
                                self.i = end + 1;
                                v
                            }
                            _ => self.ident()?,
                        };
                        self.ws();
                        test = Some((op.into(), val));
                    }
                    if self.peek() != Some(']') {
                        return self.fail();
                    }
                    self.i += 1;
                    comp.simple.push(Simple::Attr(name, test));
                }
                ':' => {
                    self.i += 1;
                    let name = self.ident()?.to_ascii_lowercase();
                    let simple = match name.as_str() {
                        "first-child" => Simple::FirstChild,
                        "last-child" => Simple::LastChild,
                        "only-child" => Simple::OnlyChild,
                        "empty" => Simple::Empty,
                        "nth-child" | "nth-last-child" | "not" => {
                            if self.peek() != Some('(') {
                                return self.fail();
                            }
                            self.i += 1;
                            self.ws();
                            let s = if name == "not" {
                                let inner = self.compound()?;
                                self.ws();
                                Simple::Not(Box::new(inner))
                            } else {
                                let end = match self.s[self.i..].iter().position(|&x| x == ')') {
                                    Some(e) => self.i + e,
                                    None => return self.fail(),
                                };
                                let arg: String = self.s[self.i..end].iter().collect();
                                let (a, b) = match parse_nth(&arg) {
                                    Some(ab) => ab,
                                    None => return self.fail(),
                                };
                                self.i = end;
                                if name == "nth-child" {
                                    Simple::NthChild(a, b)
                                } else {
                                    Simple::NthLastChild(a, b)
                                }
                            };
                            if self.peek() != Some(')') {
                                return self.fail();
                            }
                            self.i += 1;
                            s
                        }
                        _ => return self.fail(),
                    };
                    comp.simple.push(simple);
                }
                _ => break,
            }
        }
        if comp.tag.is_none() && comp.simple.is_empty() {
            return self.fail();
        }
        Ok(comp)
    }
}

fn digits(s: &str, min: usize) -> Option<i64> {
    if s.len() < min || s.len() > 9 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if s.is_empty() {
        return Some(-1);
    }
    s.parse().ok()
}

/// Parses an `An+B` argument; numbers have at most nine digits.
fn parse_nth(text: &str) -> Option<(i64, i64)> {
    let t = text.trim_matches(is_ws).to_ascii_lowercase();
    match t.as_str() {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    let (sign, rest) = match t.as_bytes().first() {
        Some(b'+') => (1, &t[1..]),
        Some(b'-') => (-1, &t[1..]),
        _ => (1, t.as_str()),
    };
    let Some(npos) = rest.find('n') else {
        return digits(rest, 1).map(|b| (0, sign * b));
    };
    let a = match digits(&rest[..npos], 0)? {
        -1 => sign,
        v => sign * v,
    };
    let tail = rest[npos + 1..].trim_start_matches(is_ws);
    if tail.is_empty() {
        return Some((a, 0));
    }
    let bsign = match tail.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let b = digits(tail[1..].trim_start_matches(is_ws), 1)?;
    Some((a, bsign * b))
}

fn nth_ok(a: i64, b: i64, pos: i64) -> bool {
    if a == 0 {
        pos == b
    } else if a > 0 {
        pos >= b && (pos - b) % a == 0
    } else {
        pos <= b && (b - pos) % (-a) == 0
    }
}

fn split_ws(s: &str) -> impl Iterator<Item = &str> {
    s.split(is_ws).filter(|x| !x.is_empty())
}

fn match_compound(doc: &Document, id: NodeId, comp: &Compound, ctx: &mut Ctx) -> bool {
    let el = doc.node(id);
    if let Some(tag) = &comp.tag {
        if tag != "*" && el.name != *tag {
            return false;
        }
    }
    for s in &comp.simple {
        let ok = match s {
            Simple::Id(v) => el.attr("id") == Some(v.as_str()),
            Simple::Class(v) => split_ws(el.attr("class").unwrap_or("")).any(|c| c == v),
            Simple::Attr(name, test) => match (el.attr(name), test) {
                (None, _) => false,
                (Some(_), None) => true,
                (Some(v), Some((op, val))) => match op.as_str() {
                    "=" => v == val,
                    "~=" => split_ws(v).any(|c| c == val),
                    "|=" => {
                        v == val
                            || v.strip_prefix(val.as_str())
                                .is_some_and(|r| r.starts_with('-'))
                    }
                    "^=" => !val.is_empty() && v.starts_with(val.as_str()),
                    "$=" => !val.is_empty() && v.ends_with(val.as_str()),
                    _ => !val.is_empty() && v.contains(val.as_str()),
                },
            },
            Simple::Empty => !el
                .children
                .iter()
                .any(|&c| matches!(doc.node(c).kind, NodeKind::Element | NodeKind::Text)),
            Simple::Not(inner) => !match_compound(doc, id, inner, ctx),
            _ => {
                let (idx, len) = ctx.position(doc, id);
                let (pos, len) = (idx as i64 + 1, len as i64);
                match s {
                    Simple::FirstChild => pos == 1,
                    Simple::LastChild => pos == len,
                    Simple::OnlyChild => len == 1,
                    Simple::NthChild(a, b) => nth_ok(*a, *b, pos),
                    Simple::NthLastChild(a, b) => nth_ok(*a, *b, len - pos + 1),
                    _ => true,
                }
            }
        };
        if !ok {
            return false;
        }
    }
    true
}

fn is_element(doc: &Document, id: NodeId) -> bool {
    doc.node(id).kind == NodeKind::Element
}

/// Per-query caches: element-sibling positions per parent, match results per
/// (step, node) and the first sibling matching a step (for `~`). They keep
/// matching linear in the number of siblings and in the tree depth; results
/// are unchanged.
#[derive(Default)]
struct Ctx {
    siblings: BTreeMap<NodeId, Vec<NodeId>>,
    position: BTreeMap<NodeId, (usize, usize)>,
    memo: BTreeMap<(usize, NodeId), bool>,
    first: BTreeMap<(usize, NodeId), usize>,
}

impl Ctx {
    fn siblings(&mut self, doc: &Document, id: NodeId) -> (Option<NodeId>, &[NodeId]) {
        let Some(parent) = doc.node(id).parent else {
            return (None, &[]);
        };
        if !self.siblings.contains_key(&parent) {
            let sib = doc.element_siblings(id);
            let len = sib.len();
            for (i, &x) in sib.iter().enumerate() {
                self.position.insert(x, (i, len));
            }
            self.siblings.insert(parent, sib);
        }
        (Some(parent), &self.siblings[&parent])
    }

    /// Zero-based index among element siblings and their count.
    fn position(&mut self, doc: &Document, id: NodeId) -> (usize, usize) {
        if self.siblings(doc, id).0.is_none() {
            return (0, 1);
        }
        self.position[&id]
    }
}

struct Matcher<'a> {
    doc: &'a Document,
    parts: &'a Complex,
    /// Offset of this complex selector's steps in the memo keys.
    base: usize,
}

impl Matcher<'_> {
    fn matches(&self, id: NodeId, k: usize, ctx: &mut Ctx) -> bool {
        let key = (self.base + k, id);
        if let Some(&hit) = ctx.memo.get(&key) {
            return hit;
        }
        let hit = self.step(id, k, ctx);
        ctx.memo.insert(key, hit);
        hit
    }

    fn step(&self, id: NodeId, k: usize, ctx: &mut Ctx) -> bool {
        let doc = self.doc;
        let (comb, comp) = &self.parts[k];
        if !match_compound(doc, id, comp, ctx) {
            return false;
        }
        if k == 0 {
            return true;
        }
        match comb {
            '>' => doc
                .node(id)
                .parent
                .is_some_and(|p| is_element(doc, p) && self.matches(p, k - 1, ctx)),
            ' ' => {
                let mut p = doc.node(id).parent;
                while let Some(pid) = p {
                    if !is_element(doc, pid) {
                        break;
                    }
                    if self.matches(pid, k - 1, ctx) {
                        return true;
                    }
                    p = doc.node(pid).parent;
                }
                false
            }
            _ => {
                let (idx, _) = ctx.position(doc, id);
                let Some(parent) = doc.node(id).parent else {
                    return false;
                };
                if *comb == '+' {
                    let prev = if idx > 0 {
                        Some(ctx.siblings(doc, id).1[idx - 1])
                    } else {
                        None
                    };
                    return prev.is_some_and(|x| self.matches(x, k - 1, ctx));
                }
                let fkey = (self.base + k, parent);
                let first = match ctx.first.get(&fkey) {
                    Some(&f) => f,
                    None => {
                        let sib = ctx.siblings(doc, id).1.to_vec();
                        let f = sib
                            .iter()
                            .position(|&x| self.matches(x, k - 1, ctx))
                            .unwrap_or(sib.len());
                        ctx.first.insert(fkey, f);
                        f
                    }
                };
                first < idx
            }
        }
    }
}

/// A parsed selector list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    list: Vec<Complex>,
}

impl Selector {
    /// Parses `selector` (SPEC section 8.1).
    pub fn parse(selector: &str) -> Result<Selector, SelectorError> {
        let mut p = Parser {
            s: selector.chars().collect(),
            i: 0,
        };
        Ok(Selector { list: p.list()? })
    }

    /// True when element `id` matches.
    pub fn matches(&self, doc: &Document, id: NodeId) -> bool {
        self.matches_with(doc, id, &mut Ctx::default())
    }

    fn matches_with(&self, doc: &Document, id: NodeId, ctx: &mut Ctx) -> bool {
        if !is_element(doc, id) {
            return false;
        }
        let mut base = 0;
        for parts in &self.list {
            let m = Matcher { doc, parts, base };
            if m.matches(id, parts.len() - 1, ctx) {
                return true;
            }
            base += parts.len();
        }
        false
    }
}

impl Document {
    /// Elements matching `selector`, in document order (SPEC section 8.2).
    pub fn query(&self, selector: &str) -> Result<Vec<NodeId>, SelectorError> {
        let sel = Selector::parse(selector)?;
        let mut ctx = Ctx::default();
        Ok(self
            .elements()
            .into_iter()
            .filter(|&id| sel.matches_with(self, id, &mut ctx))
            .collect())
    }
}
