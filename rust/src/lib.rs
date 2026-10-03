//! LombokHTML: WHATWG HTML tokenizer, a small tree builder, serialization,
//! text extraction, an allowlist sanitizer, a CSS selector subset, page
//! metadata and table extraction, with byte-identical results in Rust,
//! TypeScript, Python, Go and PHP (see `docs/SPEC_LombokHTML_v0.2.0.md`).
//!
//! `no_std` + `alloc`, no dependencies, no `unsafe`.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod dom;
mod entities;
#[rustfmt::skip]
mod entities_data;
mod meta;
mod sanitize;
mod selector;
mod table;
mod tokenizer;

pub use dom::{parse, Document, Node, NodeId, NodeKind, MAX_DEPTH};
pub use entities::{decode_entities, escape_attr, escape_text};
pub use meta::PageMeta;
pub use sanitize::{
    is_safe_url, is_safe_url_with, sanitize, sanitize_with, Policy, DEFAULT_SCHEMES,
};
pub use selector::{Selector, SelectorError};
pub use table::{Table, MAX_CELLS};
pub use tokenizer::{tokenize, tokenize_state, State, Token};

use alloc::string::String;

/// Parses `html` and returns its structure-preserving plain text.
pub fn extract_text(html: &str) -> String {
    let doc = parse(html);
    doc.extract_text(doc.root())
}

/// Parses `html` and returns its text content without markup.
pub fn strip_tags(html: &str) -> String {
    let doc = parse(html);
    doc.text_content(doc.root())
}
