//! LombokHTML — zero-dependency HTML ingestion core (L0).
//!
//! Implements the scope from `09_LombokHTML.md`: an error-tolerant
//! HTML5-subset tokenizer/parser, DOM tree, tag stripping and
//! structure-preserving text extraction, a sanitizer, basic CSS selector
//! query, meta-tag extraction, and table extraction.
//!
//! `no_std + alloc` by default. No dependency on any other Lombok
//! library — L0 per ARCHITECTURE_UTAMA_v3.3 §2. (No i18n surface here:
//! unlike LombokValidator, this library has no user-facing error
//! messages to route through LombokLocale — its outputs are text/DOM
//! data, not messages.)

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod dom;
pub mod entities;
pub mod meta;
pub mod sanitize;
pub mod selector;
pub mod table;
pub mod text;
pub mod tokenizer;

pub use dom::{parse, Node, NodeKind};
pub use entities::{decode_entities, escape_attr, escape_text};
pub use meta::{extract_meta, PageMeta};
pub use sanitize::{is_safe_url, sanitize, sanitize_with, Policy};
pub use selector::{
    parse_selector, parse_selector_list, query, AttrOp, Combinator, Complex, Selector,
};
pub use table::{extract_tables, Row, Table};
pub use text::{extract_structured_text, strip_tags};
pub use tokenizer::{is_void_element, tokenize, Token};

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn end_to_end_ingestion_pipeline() {
        let html = r#"
            <html>
              <head>
                <title>Report</title>
                <meta name="description" content="Quarterly numbers.">
                <meta property="og:title" content="Q3 Report">
                <script>track()</script>
              </head>
              <body>
                <h1>Q3 Results</h1>
                <p onclick="steal()">Revenue is up.</p>
                <table><tr><th>Region</th><th>Sales</th></tr><tr><td>West</td><td>120</td></tr></table>
              </body>
            </html>
        "#;

        let doc = parse(html);

        let meta = extract_meta(&doc);
        assert_eq!(meta.title.as_deref(), Some("Report"));
        assert_eq!(meta.description.as_deref(), Some("Quarterly numbers."));
        assert!(meta.og_tags.contains(&("title".into(), "Q3 Report".into())));

        let tables = extract_tables(&doc);
        assert_eq!(tables[0][1], alloc::vec!["West", "120"]);

        let clean_html = sanitize(html);
        assert!(!clean_html.contains("<script"));
        assert!(!clean_html.contains("track()"));
        assert!(!clean_html.contains("onclick"));

        let text = extract_structured_text(html);
        assert!(text.contains("# Q3 Results"));
        assert!(text.contains("Revenue is up."));

        let flat = strip_tags(html);
        assert!(!flat.contains('<'));
        assert!(flat.contains("Revenue is up."));

        let headers = query(&doc, "h1");
        assert_eq!(headers.len(), 1);
    }
}
