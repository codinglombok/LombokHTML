//! WHATWG HTML tokenizer (SPEC section 2), for HTML content (no foreign content).

use crate::entities::{lookup_named, numeric_char, MAX_ENTITY};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// A token. Attribute names are lowercased; duplicate attributes keep the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    StartTag {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    /// Adjacent characters are merged into one token.
    Character(String),
    Comment(String),
    Doctype {
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
        /// False when the tokenizer set the force-quirks flag.
        correct: bool,
    },
}

/// Tokenizer state to start in (SPEC section 2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Data,
    Rcdata,
    Rawtext,
    ScriptData,
    Plaintext,
    CdataSection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum S {
    Data,
    Rcdata,
    Rawtext,
    Script,
    Plaintext,
    TagOpen,
    EndTagOpen,
    TagName,
    RcdataLt,
    RcdataEndOpen,
    RcdataEndName,
    RawtextLt,
    RawtextEndOpen,
    RawtextEndName,
    ScriptLt,
    ScriptEndOpen,
    ScriptEndName,
    ScriptEscStart,
    ScriptEscStartDash,
    ScriptEsc,
    ScriptEscDash,
    ScriptEscDashDash,
    ScriptEscLt,
    ScriptEscEndOpen,
    ScriptEscEndName,
    ScriptDblEscStart,
    ScriptDblEsc,
    ScriptDblEscDash,
    ScriptDblEscDashDash,
    ScriptDblEscLt,
    ScriptDblEscEnd,
    BeforeAttrName,
    AttrName,
    AfterAttrName,
    BeforeAttrValue,
    AttrDq,
    AttrSq,
    AttrUnq,
    AfterAttrValueQ,
    SelfClosing,
    BogusComment,
    MarkupDecl,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLt,
    CommentLtBang,
    CommentLtBangDash,
    CommentLtBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterPublicKw,
    BeforePublicId,
    PublicDq,
    PublicSq,
    AfterPublicId,
    BetweenIds,
    AfterSystemKw,
    BeforeSystemId,
    SystemDq,
    SystemSq,
    AfterSystemId,
    BogusDoctype,
    Cdata,
    CdataBracket,
    CdataEnd,
    CharRef,
    NamedRef,
    AmbiguousAmp,
    NumericRef,
    HexRefStart,
    DecRefStart,
    HexRef,
    DecRef,
    NumericRefEnd,
}

struct Tag {
    end: bool,
    name: String,
    attrs: Vec<(String, String)>,
    self_closing: bool,
}

struct Doctype {
    name: Option<String>,
    public_id: Option<String>,
    system_id: Option<String>,
    quirks: bool,
}

pub(crate) struct Tokenizer {
    s: Vec<char>,
    i: usize,
    state: S,
    ret: S,
    last_start: Option<String>,
    switch: bool,
    out: Vec<Token>,
    tag: Option<Tag>,
    temp: String,
    comment: String,
    doctype: Option<Doctype>,
    code: u32,
}

fn is_ws(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{c}' | ' ')
}

impl Tokenizer {
    pub(crate) fn new(text: &str, state: State, last_start: Option<&str>, switch: bool) -> Self {
        let mut s = Vec::with_capacity(text.len());
        let mut it = text.chars().peekable();
        while let Some(c) = it.next() {
            if c == '\r' {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                s.push('\n');
            } else {
                s.push(c);
            }
        }
        let st = match state {
            State::Data => S::Data,
            State::Rcdata => S::Rcdata,
            State::Rawtext => S::Rawtext,
            State::ScriptData => S::Script,
            State::Plaintext => S::Plaintext,
            State::CdataSection => S::Cdata,
        };
        Tokenizer {
            s,
            i: 0,
            state: st,
            ret: S::Data,
            last_start: last_start.map(ToString::to_string),
            switch,
            out: Vec::new(),
            tag: None,
            temp: String::new(),
            comment: String::new(),
            doctype: None,
            code: 0,
        }
    }

    fn emit_str(&mut self, s: &str) {
        if let Some(Token::Character(last)) = self.out.last_mut() {
            last.push_str(s);
        } else {
            self.out.push(Token::Character(s.to_string()));
        }
    }

    fn emit_char(&mut self, c: char) {
        if let Some(Token::Character(last)) = self.out.last_mut() {
            last.push(c);
        } else {
            self.out.push(Token::Character(c.to_string()));
        }
    }

    fn new_tag(&mut self, end: bool) {
        self.tag = Some(Tag {
            end,
            name: String::new(),
            attrs: Vec::new(),
            self_closing: false,
        });
    }

    fn tag(&mut self) -> &mut Tag {
        self.tag.as_mut().expect("tag")
    }

    fn emit_tag(&mut self) {
        let t = self.tag.take().expect("tag");
        if t.end {
            self.out.push(Token::EndTag { name: t.name });
            return;
        }
        let mut attrs: Vec<(String, String)> = Vec::new();
        for (k, v) in t.attrs {
            if !attrs.iter().any(|(a, _)| *a == k) {
                attrs.push((k, v));
            }
        }
        if self.switch {
            self.state = match t.name.as_str() {
                "title" | "textarea" => S::Rcdata,
                "style" | "xmp" | "iframe" | "noembed" | "noframes" | "noscript" => S::Rawtext,
                "script" => S::Script,
                "plaintext" => S::Plaintext,
                _ => self.state,
            };
        }
        self.last_start = Some(t.name.clone());
        self.out.push(Token::StartTag {
            name: t.name,
            attrs,
            self_closing: t.self_closing,
        });
    }

    fn appropriate(&self) -> bool {
        match (&self.tag, &self.last_start) {
            (Some(t), Some(l)) => t.end && t.name == *l,
            _ => false,
        }
    }

    fn start_attr(&mut self) {
        self.tag().attrs.push((String::new(), String::new()));
    }

    fn attr_name(&mut self) -> &mut String {
        &mut self.tag().attrs.last_mut().expect("attr").0
    }

    fn attr_value(&mut self) -> &mut String {
        &mut self.tag().attrs.last_mut().expect("attr").1
    }

    fn in_attr(&self) -> bool {
        matches!(self.ret, S::AttrDq | S::AttrSq | S::AttrUnq)
    }

    fn flush_ref(&mut self) {
        let t = core::mem::take(&mut self.temp);
        if self.in_attr() {
            self.attr_value().push_str(&t);
        } else {
            self.emit_str(&t);
        }
    }

    fn reconsume(&mut self, st: S) {
        self.i -= 1;
        self.state = st;
    }

    fn emit_comment(&mut self) {
        let c = core::mem::take(&mut self.comment);
        self.out.push(Token::Comment(c));
    }

    fn new_doctype(&mut self) {
        self.doctype = Some(Doctype {
            name: None,
            public_id: None,
            system_id: None,
            quirks: false,
        });
    }

    fn doctype(&mut self) -> &mut Doctype {
        self.doctype.as_mut().expect("doctype")
    }

    fn emit_doctype(&mut self, quirks: bool) {
        let d = self.doctype.take().expect("doctype");
        self.out.push(Token::Doctype {
            name: d.name,
            public_id: d.public_id,
            system_id: d.system_id,
            correct: !(d.quirks || quirks),
        });
    }

    fn lower(c: char) -> char {
        c.to_ascii_lowercase()
    }

    fn peek_eq(&self, word: &str, ignore_case: bool) -> bool {
        let w: Vec<char> = word.chars().collect();
        if self.i + w.len() > self.s.len() {
            return false;
        }
        self.s[self.i..self.i + w.len()]
            .iter()
            .zip(w.iter())
            .all(|(a, b)| {
                if ignore_case {
                    a.eq_ignore_ascii_case(b)
                } else {
                    a == b
                }
            })
    }

    pub(crate) fn run(mut self) -> Vec<Token> {
        loop {
            let c = self.s.get(self.i).copied();
            self.i += 1;
            if !self.step(c) {
                break;
            }
        }
        self.out
    }

    #[allow(clippy::cognitive_complexity)]
    fn step(&mut self, c: Option<char>) -> bool {
        use S::*;
        match self.state {
            Data => match c {
                Some('&') => {
                    self.ret = Data;
                    self.state = CharRef;
                }
                Some('<') => self.state = TagOpen,
                None => return false,
                Some(ch) => self.emit_char(ch),
            },
            Rcdata => match c {
                Some('&') => {
                    self.ret = Rcdata;
                    self.state = CharRef;
                }
                Some('<') => self.state = RcdataLt,
                Some('\0') => self.emit_char('\u{fffd}'),
                None => return false,
                Some(ch) => self.emit_char(ch),
            },
            Rawtext | Script | Plaintext => {
                let lt = match self.state {
                    Rawtext => Some(RawtextLt),
                    Script => Some(ScriptLt),
                    _ => None,
                };
                match c {
                    Some('<') if lt.is_some() => self.state = lt.expect("lt"),
                    Some('\0') => self.emit_char('\u{fffd}'),
                    None => return false,
                    Some(ch) => self.emit_char(ch),
                }
            }
            TagOpen => match c {
                Some('!') => self.state = MarkupDecl,
                Some('/') => self.state = EndTagOpen,
                Some(ch) if ch.is_ascii_alphabetic() => {
                    self.new_tag(false);
                    self.reconsume(TagName);
                }
                Some('?') => {
                    self.comment.clear();
                    self.reconsume(BogusComment);
                }
                None => {
                    self.emit_char('<');
                    return false;
                }
                Some(_) => {
                    self.emit_char('<');
                    self.reconsume(Data);
                }
            },
            EndTagOpen => match c {
                Some(ch) if ch.is_ascii_alphabetic() => {
                    self.new_tag(true);
                    self.reconsume(TagName);
                }
                Some('>') => self.state = Data,
                None => {
                    self.emit_str("</");
                    return false;
                }
                Some(_) => {
                    self.comment.clear();
                    self.reconsume(BogusComment);
                }
            },
            TagName => match c {
                Some(ch) if is_ws(ch) => self.state = BeforeAttrName,
                Some('/') => self.state = SelfClosing,
                Some('>') => {
                    self.state = Data;
                    self.emit_tag();
                }
                Some('\0') => self.tag().name.push('\u{fffd}'),
                None => return false,
                Some(ch) => self.tag().name.push(Self::lower(ch)),
            },
            RcdataLt => self.lt(c, Rcdata, RcdataEndOpen),
            RcdataEndOpen => self.end_open(c, Rcdata, RcdataEndName),
            RcdataEndName => self.end_name(c, Rcdata),
            RawtextLt => self.lt(c, Rawtext, RawtextEndOpen),
            RawtextEndOpen => self.end_open(c, Rawtext, RawtextEndName),
            RawtextEndName => self.end_name(c, Rawtext),
            ScriptLt => match c {
                Some('/') => {
                    self.temp.clear();
                    self.state = ScriptEndOpen;
                }
                Some('!') => {
                    self.state = ScriptEscStart;
                    self.emit_str("<!");
                }
                _ => {
                    self.emit_char('<');
                    self.reconsume(Script);
                }
            },
            ScriptEndOpen => self.end_open(c, Script, ScriptEndName),
            ScriptEndName => self.end_name(c, Script),
            ScriptEscStart => match c {
                Some('-') => {
                    self.state = ScriptEscStartDash;
                    self.emit_char('-');
                }
                _ => self.reconsume(Script),
            },
            ScriptEscStartDash => match c {
                Some('-') => {
                    self.state = ScriptEscDashDash;
                    self.emit_char('-');
                }
                _ => self.reconsume(Script),
            },
            ScriptEsc => match c {
                Some('-') => {
                    self.state = ScriptEscDash;
                    self.emit_char('-');
                }
                Some('<') => self.state = ScriptEscLt,
                Some('\0') => self.emit_char('\u{fffd}'),
                None => return false,
                Some(ch) => self.emit_char(ch),
            },
            ScriptEscDash => match c {
                Some('-') => {
                    self.state = ScriptEscDashDash;
                    self.emit_char('-');
                }
                Some('<') => self.state = ScriptEscLt,
                Some('\0') => {
                    self.state = ScriptEsc;
                    self.emit_char('\u{fffd}');
                }
                None => return false,
                Some(ch) => {
                    self.state = ScriptEsc;
                    self.emit_char(ch);
                }
            },
            ScriptEscDashDash => match c {
                Some('-') => self.emit_char('-'),
                Some('<') => self.state = ScriptEscLt,
                Some('>') => {
                    self.state = Script;
                    self.emit_char('>');
                }
                Some('\0') => {
                    self.state = ScriptEsc;
                    self.emit_char('\u{fffd}');
                }
                None => return false,
                Some(ch) => {
                    self.state = ScriptEsc;
                    self.emit_char(ch);
                }
            },
            ScriptEscLt => match c {
                Some('/') => {
                    self.temp.clear();
                    self.state = ScriptEscEndOpen;
                }
                Some(ch) if ch.is_ascii_alphabetic() => {
                    self.temp.clear();
                    self.emit_char('<');
                    self.reconsume(ScriptDblEscStart);
                }
                _ => {
                    self.emit_char('<');
                    self.reconsume(ScriptEsc);
                }
            },
            ScriptEscEndOpen => self.end_open(c, ScriptEsc, ScriptEscEndName),
            ScriptEscEndName => self.end_name(c, ScriptEsc),
            ScriptDblEscStart | ScriptDblEscEnd => {
                let start = self.state == ScriptDblEscStart;
                match c {
                    Some(ch) if is_ws(ch) || ch == '/' || ch == '>' => {
                        let is_script = self.temp == "script";
                        self.state = match (start, is_script) {
                            (true, true) | (false, false) => ScriptDblEsc,
                            _ => ScriptEsc,
                        };
                        self.emit_char(ch);
                    }
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        self.temp.push(Self::lower(ch));
                        self.emit_char(ch);
                    }
                    _ => self.reconsume(if start { ScriptEsc } else { ScriptDblEsc }),
                }
            }
            ScriptDblEsc => match c {
                Some('-') => {
                    self.state = ScriptDblEscDash;
                    self.emit_char('-');
                }
                Some('<') => {
                    self.state = ScriptDblEscLt;
                    self.emit_char('<');
                }
                Some('\0') => self.emit_char('\u{fffd}'),
                None => return false,
                Some(ch) => self.emit_char(ch),
            },
            ScriptDblEscDash => match c {
                Some('-') => {
                    self.state = ScriptDblEscDashDash;
                    self.emit_char('-');
                }
                Some('<') => {
                    self.state = ScriptDblEscLt;
                    self.emit_char('<');
                }
                Some('\0') => {
                    self.state = ScriptDblEsc;
                    self.emit_char('\u{fffd}');
                }
                None => return false,
                Some(ch) => {
                    self.state = ScriptDblEsc;
                    self.emit_char(ch);
                }
            },
            ScriptDblEscDashDash => match c {
                Some('-') => self.emit_char('-'),
                Some('<') => {
                    self.state = ScriptDblEscLt;
                    self.emit_char('<');
                }
                Some('>') => {
                    self.state = Script;
                    self.emit_char('>');
                }
                Some('\0') => {
                    self.state = ScriptDblEsc;
                    self.emit_char('\u{fffd}');
                }
                None => return false,
                Some(ch) => {
                    self.state = ScriptDblEsc;
                    self.emit_char(ch);
                }
            },
            ScriptDblEscLt => match c {
                Some('/') => {
                    self.temp.clear();
                    self.state = ScriptDblEscEnd;
                    self.emit_char('/');
                }
                _ => self.reconsume(ScriptDblEsc),
            },
            BeforeAttrName => match c {
                Some(ch) if is_ws(ch) => {}
                None | Some('/') | Some('>') => self.reconsume(AfterAttrName),
                Some('=') => {
                    self.start_attr();
                    self.attr_name().push('=');
                    self.state = AttrName;
                }
                Some(_) => {
                    self.start_attr();
                    self.reconsume(AttrName);
                }
            },
            AttrName => match c {
                None | Some('/') | Some('>') => self.reconsume(AfterAttrName),
                Some(ch) if is_ws(ch) => self.reconsume(AfterAttrName),
                Some('=') => self.state = BeforeAttrValue,
                Some('\0') => self.attr_name().push('\u{fffd}'),
                Some(ch) => self.attr_name().push(Self::lower(ch)),
            },
            AfterAttrName => match c {
                Some(ch) if is_ws(ch) => {}
                Some('/') => self.state = SelfClosing,
                Some('=') => self.state = BeforeAttrValue,
                Some('>') => {
                    self.state = Data;
                    self.emit_tag();
                }
                None => return false,
                Some(_) => {
                    self.start_attr();
                    self.reconsume(AttrName);
                }
            },
            BeforeAttrValue => match c {
                Some(ch) if is_ws(ch) => {}
                Some('"') => self.state = AttrDq,
                Some('\'') => self.state = AttrSq,
                Some('>') => {
                    self.state = Data;
                    self.emit_tag();
                }
                _ => self.reconsume(AttrUnq),
            },
            AttrDq | AttrSq => {
                let q = if self.state == AttrDq { '"' } else { '\'' };
                match c {
                    Some(ch) if ch == q => self.state = AfterAttrValueQ,
                    Some('&') => {
                        self.ret = self.state;
                        self.state = CharRef;
                    }
                    Some('\0') => self.attr_value().push('\u{fffd}'),
                    None => return false,
                    Some(ch) => self.attr_value().push(ch),
                }
            }
            AttrUnq => match c {
                Some(ch) if is_ws(ch) => self.state = BeforeAttrName,
                Some('&') => {
                    self.ret = AttrUnq;
                    self.state = CharRef;
                }
                Some('>') => {
                    self.state = Data;
                    self.emit_tag();
                }
                Some('\0') => self.attr_value().push('\u{fffd}'),
                None => return false,
                Some(ch) => self.attr_value().push(ch),
            },
            AfterAttrValueQ => match c {
                Some(ch) if is_ws(ch) => self.state = BeforeAttrName,
                Some('/') => self.state = SelfClosing,
                Some('>') => {
                    self.state = Data;
                    self.emit_tag();
                }
                None => return false,
                Some(_) => self.reconsume(BeforeAttrName),
            },
            SelfClosing => match c {
                Some('>') => {
                    self.tag().self_closing = true;
                    self.state = Data;
                    self.emit_tag();
                }
                None => return false,
                Some(_) => self.reconsume(BeforeAttrName),
            },
            BogusComment => match c {
                Some('>') => {
                    self.state = Data;
                    self.emit_comment();
                }
                None => {
                    self.emit_comment();
                    return false;
                }
                Some('\0') => self.comment.push('\u{fffd}'),
                Some(ch) => self.comment.push(ch),
            },
            MarkupDecl => {
                self.i -= 1;
                self.comment.clear();
                if self.peek_eq("--", false) {
                    self.i += 2;
                    self.state = CommentStart;
                } else if self.peek_eq("doctype", true) {
                    self.i += 7;
                    self.state = Doctype;
                } else {
                    self.state = BogusComment;
                }
            }
            CommentStart => match c {
                Some('-') => self.state = CommentStartDash,
                Some('>') => {
                    self.state = Data;
                    self.emit_comment();
                }
                _ => self.reconsume(Comment),
            },
            CommentStartDash => match c {
                Some('-') => self.state = CommentEnd,
                Some('>') => {
                    self.state = Data;
                    self.emit_comment();
                }
                None => {
                    self.emit_comment();
                    return false;
                }
                Some(_) => {
                    self.comment.push('-');
                    self.reconsume(Comment);
                }
            },
            Comment => match c {
                Some('<') => {
                    self.comment.push('<');
                    self.state = CommentLt;
                }
                Some('-') => self.state = CommentEndDash,
                Some('\0') => self.comment.push('\u{fffd}'),
                None => {
                    self.emit_comment();
                    return false;
                }
                Some(ch) => self.comment.push(ch),
            },
            CommentLt => match c {
                Some('!') => {
                    self.comment.push('!');
                    self.state = CommentLtBang;
                }
                Some('<') => self.comment.push('<'),
                _ => self.reconsume(Comment),
            },
            CommentLtBang => match c {
                Some('-') => self.state = CommentLtBangDash,
                _ => self.reconsume(Comment),
            },
            CommentLtBangDash => match c {
                Some('-') => self.state = CommentLtBangDashDash,
                _ => self.reconsume(CommentEndDash),
            },
            CommentLtBangDashDash => self.reconsume(CommentEnd),
            CommentEndDash => match c {
                Some('-') => self.state = CommentEnd,
                None => {
                    self.emit_comment();
                    return false;
                }
                Some(_) => {
                    self.comment.push('-');
                    self.reconsume(Comment);
                }
            },
            CommentEnd => match c {
                Some('>') => {
                    self.state = Data;
                    self.emit_comment();
                }
                Some('!') => self.state = CommentEndBang,
                Some('-') => self.comment.push('-'),
                None => {
                    self.emit_comment();
                    return false;
                }
                Some(_) => {
                    self.comment.push_str("--");
                    self.reconsume(Comment);
                }
            },
            CommentEndBang => match c {
                Some('-') => {
                    self.comment.push_str("--!");
                    self.state = CommentEndDash;
                }
                Some('>') => {
                    self.state = Data;
                    self.emit_comment();
                }
                None => {
                    self.emit_comment();
                    return false;
                }
                Some(_) => {
                    self.comment.push_str("--!");
                    self.reconsume(Comment);
                }
            },
            Doctype => match c {
                Some(ch) if is_ws(ch) => self.state = BeforeDoctypeName,
                None => {
                    self.new_doctype();
                    self.emit_doctype(true);
                    return false;
                }
                Some(_) => self.reconsume(BeforeDoctypeName),
            },
            BeforeDoctypeName => match c {
                Some(ch) if is_ws(ch) => {}
                Some('>') => {
                    self.new_doctype();
                    self.state = Data;
                    self.emit_doctype(true);
                }
                None => {
                    self.new_doctype();
                    self.emit_doctype(true);
                    return false;
                }
                Some(ch) => {
                    self.new_doctype();
                    let first = if ch == '\0' {
                        '\u{fffd}'
                    } else {
                        Self::lower(ch)
                    };
                    self.doctype().name = Some(first.to_string());
                    self.state = DoctypeName;
                }
            },
            DoctypeName => match c {
                Some(ch) if is_ws(ch) => self.state = AfterDoctypeName,
                Some('>') => {
                    self.state = Data;
                    self.emit_doctype(false);
                }
                None => {
                    self.emit_doctype(true);
                    return false;
                }
                Some(ch) => {
                    let ch = if ch == '\0' {
                        '\u{fffd}'
                    } else {
                        Self::lower(ch)
                    };
                    self.doctype().name.as_mut().expect("name").push(ch);
                }
            },
            AfterDoctypeName => match c {
                Some(ch) if is_ws(ch) => {}
                Some('>') => {
                    self.state = Data;
                    self.emit_doctype(false);
                }
                None => {
                    self.emit_doctype(true);
                    return false;
                }
                Some(_) => {
                    self.i -= 1;
                    if self.peek_eq("public", true) {
                        self.i += 6;
                        self.state = AfterPublicKw;
                    } else if self.peek_eq("system", true) {
                        self.i += 6;
                        self.state = AfterSystemKw;
                    } else {
                        self.i += 1;
                        self.doctype().quirks = true;
                        self.state = BogusDoctype;
                    }
                }
            },
            AfterPublicKw | AfterSystemKw | BeforePublicId | BeforeSystemId => {
                let public = matches!(self.state, AfterPublicKw | BeforePublicId);
                let after_kw = matches!(self.state, AfterPublicKw | AfterSystemKw);
                match c {
                    Some(ch) if is_ws(ch) => {
                        if after_kw {
                            self.state = if public {
                                BeforePublicId
                            } else {
                                BeforeSystemId
                            };
                        }
                    }
                    Some(q @ ('"' | '\'')) => {
                        if public {
                            self.doctype().public_id = Some(String::new());
                            self.state = if q == '"' { PublicDq } else { PublicSq };
                        } else {
                            self.doctype().system_id = Some(String::new());
                            self.state = if q == '"' { SystemDq } else { SystemSq };
                        }
                    }
                    Some('>') => {
                        self.state = Data;
                        self.emit_doctype(true);
                    }
                    None => {
                        self.emit_doctype(true);
                        return false;
                    }
                    Some(_) => {
                        self.doctype().quirks = true;
                        self.reconsume(BogusDoctype);
                    }
                }
            }
            PublicDq | PublicSq | SystemDq | SystemSq => {
                let public = matches!(self.state, PublicDq | PublicSq);
                let q = if matches!(self.state, PublicDq | SystemDq) {
                    '"'
                } else {
                    '\''
                };
                match c {
                    Some(ch) if ch == q => {
                        self.state = if public { AfterPublicId } else { AfterSystemId }
                    }
                    Some('>') => {
                        self.state = Data;
                        self.emit_doctype(true);
                    }
                    None => {
                        self.emit_doctype(true);
                        return false;
                    }
                    Some(ch) => {
                        let ch = if ch == '\0' { '\u{fffd}' } else { ch };
                        let d = self.doctype();
                        let field = if public {
                            &mut d.public_id
                        } else {
                            &mut d.system_id
                        };
                        field.as_mut().expect("id").push(ch);
                    }
                }
            }
            AfterPublicId | BetweenIds => match c {
                Some(ch) if is_ws(ch) => {
                    if self.state == AfterPublicId {
                        self.state = BetweenIds;
                    }
                }
                Some('>') => {
                    self.state = Data;
                    self.emit_doctype(false);
                }
                Some(q @ ('"' | '\'')) => {
                    self.doctype().system_id = Some(String::new());
                    self.state = if q == '"' { SystemDq } else { SystemSq };
                }
                None => {
                    self.emit_doctype(true);
                    return false;
                }
                Some(_) => {
                    self.doctype().quirks = true;
                    self.reconsume(BogusDoctype);
                }
            },
            AfterSystemId => match c {
                Some(ch) if is_ws(ch) => {}
                Some('>') => {
                    self.state = Data;
                    self.emit_doctype(false);
                }
                None => {
                    self.emit_doctype(true);
                    return false;
                }
                Some(_) => self.reconsume(BogusDoctype),
            },
            BogusDoctype => match c {
                Some('>') => {
                    self.state = Data;
                    self.emit_doctype(false);
                }
                None => {
                    self.emit_doctype(false);
                    return false;
                }
                Some(_) => {}
            },
            Cdata => match c {
                Some(']') => self.state = CdataBracket,
                None => return false,
                Some(ch) => self.emit_char(ch),
            },
            CdataBracket => match c {
                Some(']') => self.state = CdataEnd,
                _ => {
                    self.emit_char(']');
                    self.reconsume(Cdata);
                }
            },
            CdataEnd => match c {
                Some(']') => self.emit_char(']'),
                Some('>') => self.state = Data,
                _ => {
                    self.emit_str("]]");
                    self.reconsume(Cdata);
                }
            },
            CharRef => {
                self.temp.clear();
                self.temp.push('&');
                match c {
                    Some(ch) if ch.is_ascii_alphanumeric() => self.reconsume(NamedRef),
                    Some('#') => {
                        self.temp.push('#');
                        self.state = NumericRef;
                    }
                    _ => {
                        self.flush_ref();
                        let r = self.ret;
                        self.reconsume(r);
                    }
                }
            }
            NamedRef => {
                self.i -= 1;
                let start = self.i;
                let mut j = start;
                while j < self.s.len()
                    && self.s[j].is_ascii_alphanumeric()
                    && j - start < MAX_ENTITY
                {
                    j += 1;
                }
                let run: String = self.s[start..j].iter().collect();
                let semi = self.s.get(j) == Some(&';');
                let mut matched: Option<(usize, &'static str, bool)> = None;
                if semi {
                    let mut key = run.clone();
                    key.push(';');
                    if let Some(v) = lookup_named(&key) {
                        matched = Some((run.len() + 1, v, true));
                    }
                }
                if matched.is_none() {
                    for k in (1..=run.len()).rev() {
                        if let Some(v) = lookup_named(&run[..k]) {
                            matched = Some((k, v, false));
                            break;
                        }
                    }
                }
                match matched {
                    None => {
                        self.temp.push_str(&run);
                        self.i = j;
                        self.flush_ref();
                        self.state = AmbiguousAmp;
                    }
                    Some((len, value, with_semi)) => {
                        self.i = start + len;
                        let next = self.s.get(self.i).copied();
                        if self.in_attr()
                            && !with_semi
                            && next.is_some_and(|n| n == '=' || n.is_ascii_alphanumeric())
                        {
                            self.temp.push_str(&run[..len]);
                        } else {
                            self.temp.clear();
                            self.temp.push_str(value);
                        }
                        self.flush_ref();
                        self.state = self.ret;
                    }
                }
            }
            AmbiguousAmp => match c {
                Some(ch) if ch.is_ascii_alphanumeric() => {
                    if self.in_attr() {
                        self.attr_value().push(ch);
                    } else {
                        self.emit_char(ch);
                    }
                }
                _ => {
                    let r = self.ret;
                    self.reconsume(r);
                }
            },
            NumericRef => {
                self.code = 0;
                match c {
                    Some(ch @ ('x' | 'X')) => {
                        self.temp.push(ch);
                        self.state = HexRefStart;
                    }
                    _ => self.reconsume(DecRefStart),
                }
            }
            HexRefStart | DecRefStart => {
                let hex = self.state == HexRefStart;
                let ok = c.is_some_and(|ch| {
                    if hex {
                        ch.is_ascii_hexdigit()
                    } else {
                        ch.is_ascii_digit()
                    }
                });
                if ok {
                    self.reconsume(if hex { HexRef } else { DecRef });
                } else {
                    self.flush_ref();
                    let r = self.ret;
                    self.reconsume(r);
                }
            }
            HexRef | DecRef => {
                let radix = if self.state == HexRef { 16 } else { 10 };
                match c.and_then(|ch| ch.to_digit(radix)) {
                    Some(d) => {
                        self.code =
                            (self.code.saturating_mul(radix).saturating_add(d)).min(0x11_0000)
                    }
                    None if c == Some(';') => self.state = NumericRefEnd,
                    None => self.reconsume(NumericRefEnd),
                }
            }
            NumericRefEnd => {
                self.i -= 1;
                self.temp.clear();
                self.temp.push(numeric_char(self.code));
                self.flush_ref();
                self.state = self.ret;
            }
        }
        true
    }

    fn lt(&mut self, c: Option<char>, base: S, open: S) {
        if c == Some('/') {
            self.temp.clear();
            self.state = open;
        } else {
            self.emit_char('<');
            self.reconsume(base);
        }
    }

    fn end_open(&mut self, c: Option<char>, base: S, name: S) {
        if c.is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.new_tag(true);
            self.reconsume(name);
        } else {
            self.emit_str("</");
            self.reconsume(base);
        }
    }

    fn end_name(&mut self, c: Option<char>, base: S) {
        match c {
            Some(ch) if is_ws(ch) && self.appropriate() => self.state = S::BeforeAttrName,
            Some('/') if self.appropriate() => self.state = S::SelfClosing,
            Some('>') if self.appropriate() => {
                self.state = S::Data;
                self.emit_tag();
            }
            Some(ch) if ch.is_ascii_alphabetic() => {
                self.tag().name.push(Self::lower(ch));
                self.temp.push(ch);
            }
            _ => {
                let mut s = String::from("</");
                s.push_str(&self.temp);
                self.emit_str(&s);
                self.tag = None;
                self.reconsume(base);
            }
        }
    }
}

/// Tokenizes `html` as the parser does: start tags of `title`, `textarea`,
/// `style`, `xmp`, `iframe`, `noembed`, `noframes`, `noscript`, `script` and
/// `plaintext` switch the tokenizer state (SPEC section 2.2).
pub fn tokenize(html: &str) -> Vec<Token> {
    Tokenizer::new(html, State::Data, None, true).run()
}

/// Tokenizes `html` from `state` without element-driven switching, as the
/// html5lib tokenizer tests do; `last_start_tag` decides which end tag closes
/// RCDATA, RAWTEXT and script data.
pub fn tokenize_state(html: &str, state: State, last_start_tag: Option<&str>) -> Vec<Token> {
    Tokenizer::new(html, state, last_start_tag, false).run()
}
