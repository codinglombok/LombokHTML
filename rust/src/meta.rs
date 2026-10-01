//! Meta tag extraction (scope item 7): `<title>`, `<meta name="description">`,
//! and `og:*` Open Graph tags.

use crate::dom::Node;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    /// `(property, content)` pairs for every `og:*` meta tag, in document order.
    pub og_tags: Vec<(String, String)>,
}

pub fn extract_meta(doc: &Node) -> PageMeta {
    let mut meta = PageMeta::default();

    if let Some(title_node) = doc.find_first("title") {
        let mut text = String::new();
        for c in &title_node.children {
            if let Some(t) = &c.text {
                text.push_str(t);
            }
        }
        if !text.is_empty() {
            meta.title = Some(text);
        }
    }

    for m in doc.find_all("meta") {
        if let Some(name) = m.attr("name") {
            if name.eq_ignore_ascii_case("description") {
                if let Some(content) = m.attr("content") {
                    meta.description = Some(content.into());
                }
            }
        }
        if let Some(prop) = m.attr("property") {
            if let Some(rest) = prop.strip_prefix("og:") {
                if let Some(content) = m.attr("content") {
                    meta.og_tags.push((rest.into(), content.into()));
                }
            }
        }
    }

    meta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse;

    #[test]
    fn extracts_title() {
        let doc = parse("<html><head><title>My Page</title></head></html>");
        assert_eq!(extract_meta(&doc).title.as_deref(), Some("My Page"));
    }

    #[test]
    fn extracts_description() {
        let doc = parse(r#"<meta name="description" content="A great page.">"#);
        assert_eq!(
            extract_meta(&doc).description.as_deref(),
            Some("A great page.")
        );
    }

    #[test]
    fn extracts_og_tags() {
        let html = r#"
            <meta property="og:title" content="OG Title">
            <meta property="og:image" content="https://x.com/img.png">
        "#;
        let doc = parse(html);
        let meta = extract_meta(&doc);
        assert!(meta.og_tags.contains(&("title".into(), "OG Title".into())));
        assert!(meta
            .og_tags
            .contains(&("image".into(), "https://x.com/img.png".into())));
    }

    #[test]
    fn missing_tags_are_none() {
        let doc = parse("<p>no meta here</p>");
        let meta = extract_meta(&doc);
        assert!(meta.title.is_none());
        assert!(meta.description.is_none());
        assert!(meta.og_tags.is_empty());
    }
}
