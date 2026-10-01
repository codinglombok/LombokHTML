//! HTML character-reference decoding and escaping.
//!
//! Decodes numeric (`&#38;`, `&#x26;`) and a curated set of named references.
//! Unknown named references are left untouched (fail-safe, never panics).
//! Invalid code points (0, surrogates, > U+10FFFF) become U+FFFD.

use alloc::string::String;

const NAMED: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{00A0}'),
    ("copy", '\u{00A9}'),
    ("reg", '\u{00AE}'),
    ("trade", '\u{2122}'),
    ("hellip", '\u{2026}'),
    ("mdash", '\u{2014}'),
    ("ndash", '\u{2013}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("ldquo", '\u{201C}'),
    ("rdquo", '\u{201D}'),
    ("laquo", '\u{00AB}'),
    ("raquo", '\u{00BB}'),
    ("bull", '\u{2022}'),
    ("middot", '\u{00B7}'),
    ("euro", '\u{20AC}'),
    ("pound", '\u{00A3}'),
    ("yen", '\u{00A5}'),
    ("cent", '\u{00A2}'),
    ("times", '\u{00D7}'),
    ("divide", '\u{00F7}'),
    ("deg", '\u{00B0}'),
    ("plusmn", '\u{00B1}'),
    ("sect", '\u{00A7}'),
    ("para", '\u{00B6}'),
];

fn numeric(body: &str) -> Option<char> {
    let (digits, radix) = match body.strip_prefix(['x', 'X']) {
        Some(h) => (h, 16),
        None => (body, 10),
    };
    // Strictly digits of the radix: `from_str_radix` alone would also accept a
    // leading '+', which browsers and the HTML spec do not.
    if digits.is_empty() || digits.len() > 8 || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let n = u32::from_str_radix(digits, radix).ok()?;
    Some(
        char::from_u32(n)
            .filter(|c| *c != '\0')
            .unwrap_or('\u{FFFD}'),
    )
}

/// Decode character references in `s`.
pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return String::from(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        // A reference is at most ~32 chars: look for ';' within a short window.
        let window_end = after
            .char_indices()
            .take(34)
            .find(|(_, c)| *c == ';')
            .map(|(i, _)| i);
        let decoded = window_end.and_then(|end| {
            let body = &after[..end];
            let ch = if let Some(num) = body.strip_prefix('#') {
                numeric(num)
            } else {
                NAMED.iter().find(|(n, _)| *n == body).map(|(_, c)| *c)
            };
            ch.map(|c| (c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Escape text content for serialization.
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape an attribute value for double-quoted serialization.
pub fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_named_and_numeric() {
        assert_eq!(
            decode_entities("a &amp; b &lt;c&gt; &quot;d&quot;"),
            "a & b <c> \"d\""
        );
        assert_eq!(decode_entities("&#38;&#x26;&#X41;"), "&&A");
        assert_eq!(decode_entities("caf&eacute;"), "caf&eacute;"); // unknown: untouched
    }

    #[test]
    fn invalid_code_points_become_replacement() {
        assert_eq!(decode_entities("&#0;"), "\u{FFFD}");
        assert_eq!(decode_entities("&#xD800;"), "\u{FFFD}");
        assert_eq!(decode_entities("&#x110000;"), "\u{FFFD}");
    }

    #[test]
    fn sign_prefixed_numbers_are_not_references() {
        assert_eq!(decode_entities("&#+65;"), "&#+65;");
        assert_eq!(decode_entities("&#x+41;"), "&#x+41;");
        assert_eq!(decode_entities("&#-65;"), "&#-65;");
    }

    #[test]
    fn malformed_is_left_alone() {
        assert_eq!(decode_entities("AT&T"), "AT&T");
        assert_eq!(decode_entities("&"), "&");
        assert_eq!(decode_entities("&#;"), "&#;");
        assert_eq!(decode_entities("&amp"), "&amp");
    }

    #[test]
    fn escape_roundtrip() {
        let s = "<a href=\"x\">&</a>";
        assert_eq!(decode_entities(&escape_attr(s)), s);
        assert_eq!(decode_entities(&escape_text(s)), s);
    }
}
