//! An error-tolerant HTML tokenizer covering the subset real-world HTML
//! ingestion needs: start/end tags with attributes, comments, doctype,
//! and text runs. Malformed markup (unquoted attributes, missing closing
//! `>`, stray `<`) degrades gracefully rather than erroring, matching
//! browser-parser behavior (scope item 1).

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    StartTag {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Text(String),
    Comment(String),
    Doctype(String),
}

/// Elements with no content and no end tag (HTML5 "void elements").
pub const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

pub fn is_void_element(tag: &str) -> bool {
    VOID_ELEMENTS.contains(&tag)
}

/// Elements whose content is raw text (not further tokenized as markup).
pub const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

pub fn tokenize(html: &str) -> Vec<Token> {
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0usize;
    let mut tokens = Vec::new();
    let mut text_buf = String::new();

    macro_rules! flush_text {
        () => {
            if !text_buf.is_empty() {
                tokens.push(Token::Text(crate::entities::decode_entities(
                    &core::mem::take(&mut text_buf),
                )));
            }
        };
    }

    while i < chars.len() {
        if chars[i] == '<' {
            // Comment
            if chars[i..].starts_with(&['<', '!', '-', '-']) {
                flush_text!();
                let start = i + 4;
                let mut end = start;
                while end < chars.len() && !chars[end..].starts_with(&['-', '-', '>']) {
                    end += 1;
                }
                let content: String = chars[start..end].iter().collect();
                tokens.push(Token::Comment(content));
                i = if end < chars.len() {
                    end + 3
                } else {
                    chars.len()
                };
                continue;
            }
            // Doctype
            if chars[i..].to_ascii_lowercase_prefix(9) == "<!doctype" {
                flush_text!();
                let start = i;
                let mut end = start;
                while end < chars.len() && chars[end] != '>' {
                    end += 1;
                }
                let content: String = chars[start..(end.min(chars.len()))].iter().collect();
                tokens.push(Token::Doctype(content));
                i = if end < chars.len() {
                    end + 1
                } else {
                    chars.len()
                };
                continue;
            }
            // End tag
            if chars.get(i + 1) == Some(&'/') {
                flush_text!();
                let mut end = i + 2;
                while end < chars.len() && chars[end] != '>' {
                    end += 1;
                }
                let name: String = chars[i + 2..end.min(chars.len())]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_ascii_lowercase();
                if !name.is_empty() {
                    tokens.push(Token::EndTag { name: name.clone() });
                }
                i = if end < chars.len() {
                    end + 1
                } else {
                    chars.len()
                };

                // Raw-text elements: nothing to skip here (handled after
                // the matching start tag below), but if content ever
                // slips through unbalanced we simply continue.
                continue;
            }
            // Start tag (must begin with an ASCII letter after '<')
            if chars.get(i + 1).map(|c| c.is_ascii_alphabetic()) == Some(true) {
                flush_text!();
                let tag_start = i + 1;
                let end = find_tag_end(&chars, tag_start);
                let raw: String = chars[tag_start..end.min(chars.len())].iter().collect();
                let (name, attrs, self_closing) = parse_tag_contents(&raw);
                let is_raw_text = RAW_TEXT_ELEMENTS.contains(&name.as_str());
                tokens.push(Token::StartTag {
                    name: name.clone(),
                    attrs,
                    self_closing,
                });
                i = if end < chars.len() {
                    end + 1
                } else {
                    chars.len()
                };

                if is_raw_text && !self_closing {
                    // Consume everything up to (and including) the
                    // matching </name>, verbatim, as a single Text token.
                    let close_pat: Vec<char> = alloc::format!("</{}", name).chars().collect();
                    let mut j = i;
                    let mut found_close = None;
                    while j < chars.len() {
                        if chars[j..].starts_with(close_pat.as_slice()) {
                            found_close = Some(j);
                            break;
                        }
                        j += 1;
                    }
                    let content_end = found_close.unwrap_or(chars.len());
                    let raw_content: String = chars[i..content_end].iter().collect();
                    if !raw_content.is_empty() {
                        tokens.push(Token::Text(raw_content));
                    }
                    if let Some(close_start) = found_close {
                        let mut k = close_start;
                        while k < chars.len() && chars[k] != '>' {
                            k += 1;
                        }
                        tokens.push(Token::EndTag { name });
                        i = if k < chars.len() { k + 1 } else { chars.len() };
                    } else {
                        i = chars.len();
                    }
                }
                continue;
            }
            // Stray '<' not starting a recognizable construct: literal text.
            text_buf.push('<');
            i += 1;
            continue;
        }
        text_buf.push(chars[i]);
        i += 1;
    }
    flush_text!();
    tokens
}

/// Index of the `>` that closes a start tag beginning at `from`, ignoring any
/// `>` inside a quoted attribute value (`<a title="a>b">`) exactly like a
/// browser does. Returns `chars.len()` if the tag never closes.
fn find_tag_end(chars: &[char], from: usize) -> usize {
    let mut i = from;
    let mut quote: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '>' {
                    return i;
                }
                if c == '=' {
                    let mut j = i + 1;
                    while j < chars.len() && chars[j].is_whitespace() {
                        j += 1;
                    }
                    if let Some(&q) = chars.get(j) {
                        if q == '"' || q == '\'' {
                            quote = Some(q);
                            i = j;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    chars.len()
}

/// Parse `tag attr="val" attr2='val2' attr3 checked/` (without the
/// enclosing `<`/`>`) into (name, attrs, self_closing).
fn parse_tag_contents(raw: &str) -> (String, Vec<(String, String)>, bool) {
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    let name_start = i;
    while i < chars.len()
        && (chars[i].is_ascii_alphanumeric() || chars[i] == '-' || chars[i] == ':')
    {
        i += 1;
    }
    let name: String = chars[name_start..i]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();

    let mut attrs = Vec::new();
    let mut self_closing = false;

    loop {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        if chars[i] == '/' {
            self_closing = true;
            i += 1;
            continue;
        }
        let attr_name_start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' && chars[i] != '/' {
            i += 1;
        }
        if i == attr_name_start {
            i += 1;
            continue;
        }
        let attr_name: String = chars[attr_name_start..i]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();

        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let mut attr_value = String::new();
        if chars.get(i) == Some(&'=') {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if let Some(&quote) = chars.get(i) {
                if quote == '"' || quote == '\'' {
                    i += 1;
                    let val_start = i;
                    while i < chars.len() && chars[i] != quote {
                        i += 1;
                    }
                    attr_value = chars[val_start..i].iter().collect();
                    if i < chars.len() {
                        i += 1;
                    }
                } else {
                    let val_start = i;
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '/' {
                        i += 1;
                    }
                    attr_value = chars[val_start..i].iter().collect();
                }
            }
        }
        attrs.push((attr_name, crate::entities::decode_entities(&attr_value)));
    }

    (name, attrs, self_closing)
}

/// Small helper trait used above to peek a lowercase prefix without
/// allocating for every comparison.
trait AsciiLowerPrefix {
    fn to_ascii_lowercase_prefix(&self, n: usize) -> String;
}
impl AsciiLowerPrefix for [char] {
    fn to_ascii_lowercase_prefix(&self, n: usize) -> String {
        self.iter().take(n).collect::<String>().to_ascii_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_simple_element() {
        let toks = tokenize("<p>Hello</p>");
        assert_eq!(
            toks,
            alloc::vec![
                Token::StartTag {
                    name: "p".into(),
                    attrs: alloc::vec![],
                    self_closing: false
                },
                Token::Text("Hello".into()),
                Token::EndTag { name: "p".into() },
            ]
        );
    }

    #[test]
    fn tokenizes_attributes() {
        let toks = tokenize(r#"<a href="https://x.com" target='_blank' checked>Go</a>"#);
        match &toks[0] {
            Token::StartTag { name, attrs, .. } => {
                assert_eq!(name, "a");
                assert_eq!(attrs[0], ("href".to_string(), "https://x.com".to_string()));
                assert_eq!(attrs[1], ("target".to_string(), "_blank".to_string()));
                assert_eq!(attrs[2], ("checked".to_string(), "".to_string()));
            }
            _ => panic!("expected start tag"),
        }
    }

    #[test]
    fn void_elements_have_no_end_tag() {
        let toks = tokenize("<br><img src=\"a.png\">");
        assert_eq!(toks.len(), 2);
        assert!(matches!(toks[0], Token::StartTag { .. }));
        assert!(matches!(toks[1], Token::StartTag { .. }));
    }

    #[test]
    fn script_content_is_raw_text() {
        let toks = tokenize("<script>if (1 < 2) { alert('<b>') }</script>after");
        assert_eq!(
            toks,
            alloc::vec![
                Token::StartTag {
                    name: "script".into(),
                    attrs: alloc::vec![],
                    self_closing: false
                },
                Token::Text("if (1 < 2) { alert('<b>') }".into()),
                Token::EndTag {
                    name: "script".into()
                },
                Token::Text("after".into()),
            ]
        );
    }

    #[test]
    fn comment_is_captured() {
        let toks = tokenize("<!-- note --><p>x</p>");
        assert_eq!(toks[0], Token::Comment(" note ".into()));
    }

    #[test]
    fn doctype_is_captured() {
        let toks = tokenize("<!DOCTYPE html><html></html>");
        assert!(matches!(toks[0], Token::Doctype(_)));
    }

    #[test]
    fn malformed_unclosed_tag_degrades_gracefully() {
        // No closing '>' before EOF: swallowed as a tag with an empty/garbage
        // name rather than panicking.
        let toks = tokenize("<p>text<un");
        assert!(!toks.is_empty());
    }

    #[test]
    fn gt_inside_quoted_attribute_does_not_end_tag() {
        let toks = tokenize(r#"<a title="x>y" href='a>b'>t</a>"#);
        match &toks[0] {
            Token::StartTag { attrs, .. } => {
                assert_eq!(attrs[0].1, "x>y");
                assert_eq!(attrs[1].1, "a>b");
            }
            _ => panic!(),
        }
        assert_eq!(toks[1], Token::Text("t".into()));
    }

    #[test]
    fn entities_decoded_in_text_and_attrs_but_not_raw_text() {
        let toks = tokenize("<p title=\"a&amp;b\">x &lt; y</p><script>a &lt; b</script>");
        match &toks[0] {
            Token::StartTag { attrs, .. } => assert_eq!(attrs[0].1, "a&b"),
            _ => panic!(),
        }
        assert_eq!(toks[1], Token::Text("x < y".into()));
        assert_eq!(toks[4], Token::Text("a &lt; b".into()));
    }

    #[test]
    fn stray_lt_in_text_is_literal() {
        let toks = tokenize("5 < 10 and 20 > 10");
        assert_eq!(toks, alloc::vec![Token::Text("5 < 10 and 20 > 10".into())]);
    }
}
