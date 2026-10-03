//! Character references and escaping (SPEC section 3).

use crate::entities_data::ENTITIES;
use alloc::string::String;

/// Longest named reference, in characters, including the trailing `;`.
pub(crate) const MAX_ENTITY: usize = 32;

/// Replacement text for a named reference (`"amp;"`, `"amp"`), if WHATWG defines it.
pub(crate) fn lookup_named(name: &str) -> Option<&'static str> {
    ENTITIES
        .binary_search_by(|(k, _)| k.cmp(&name))
        .ok()
        .map(|i| ENTITIES[i].1)
}

/// Character for a numeric reference, with the WHATWG replacements for
/// 0x80..0x9F and U+FFFD for zero, surrogates and values above U+10FFFF.
pub(crate) fn numeric_char(code: u32) -> char {
    let mapped = match code {
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        0 => 0xFFFD,
        c => c,
    };
    char::from_u32(mapped).unwrap_or('\u{FFFD}')
}

/// Decodes character references as in text content (SPEC section 3.1).
pub fn decode_entities(text: &str) -> String {
    let s: alloc::vec::Vec<char> = text.chars().collect();
    let n = s.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        if s[i] != '&' {
            out.push(s[i]);
            i += 1;
            continue;
        }
        let j = i + 1;
        if j < n && s[j] == '#' {
            let mut k = j + 1;
            let hex = k < n && (s[k] == 'x' || s[k] == 'X');
            if hex {
                k += 1;
            }
            let radix = if hex { 16 } else { 10 };
            let start = k;
            let mut code: u32 = 0;
            while k < n {
                match s[k].to_digit(radix) {
                    Some(d) if s[k].is_ascii() => {
                        code = (code * radix + d).min(0x110000);
                        k += 1;
                    }
                    _ => break,
                }
            }
            if k == start {
                out.extend(&s[i..k]);
                i = k;
                continue;
            }
            if k < n && s[k] == ';' {
                k += 1;
            }
            out.push(numeric_char(code));
            i = k;
            continue;
        }
        let mut k = j;
        while k < n && s[k].is_ascii_alphanumeric() && k - j < MAX_ENTITY {
            k += 1;
        }
        let run: String = s[j..k].iter().collect();
        let mut matched: Option<(usize, &'static str)> = None;
        if k < n && s[k] == ';' {
            let mut key = run.clone();
            key.push(';');
            if let Some(v) = lookup_named(&key) {
                matched = Some((run.len() + 1, v));
            }
        }
        if matched.is_none() {
            let mut m = run.len();
            while m > 0 {
                if let Some(v) = lookup_named(&run[..m]) {
                    matched = Some((m, v));
                    break;
                }
                m -= 1;
            }
        }
        match matched {
            None => {
                out.push('&');
                out.push_str(&run);
                i = k;
            }
            Some((len, v)) => {
                out.push_str(v);
                i = j + len;
            }
        }
    }
    out
}

/// Escapes text content: `&`, U+00A0, `<`, `>` (SPEC section 3.2).
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes an attribute value: `&`, U+00A0, `"`, `<`, `>` (SPEC section 3.2).
pub fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}
