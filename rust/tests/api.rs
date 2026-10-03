//! Public API checks beyond the shared vectors.
use lombokhtml::*;

#[test]
fn document_navigation() {
    let doc = parse("<div id=a><p>x</p><!--c--></div>");
    assert!(!doc.is_empty());
    assert_eq!(doc.len(), 5);
    let root = doc.node(doc.root());
    assert_eq!(root.kind, NodeKind::Document);
    let div = doc.elements()[0];
    assert!(doc.node(div).is("div"));
    assert_eq!(doc.node(div).attr("id"), Some("a"));
    assert_eq!(doc.node(div).attr("class"), None);
    assert_eq!(doc.descendants(div).len(), 1);
    assert_eq!(doc.serialize(div), "<div id=\"a\"><p>x</p><!--c--></div>");
    assert_eq!(doc.text_content(div), "x");
    assert_eq!(doc.extract_text(div), "x");
    assert!(parse("").is_empty());
}

#[test]
fn depth_limit() {
    let html = "<div>".repeat(MAX_DEPTH + 10);
    let doc = parse(&html);
    assert_eq!(doc.elements().len(), MAX_DEPTH);
}

#[test]
fn tokenizer_states() {
    let toks = tokenize_state("a</title>b", State::Rcdata, Some("title"));
    assert_eq!(
        toks,
        vec![
            Token::Character("a".into()),
            Token::EndTag {
                name: "title".into()
            },
            Token::Character("b".into())
        ]
    );
    assert_eq!(
        tokenize_state("<x>", State::Plaintext, None),
        vec![Token::Character("<x>".into())]
    );
    assert_eq!(
        tokenize_state("a]]>b", State::CdataSection, None),
        vec![Token::Character("ab".into())]
    );
    assert_eq!(
        tokenize_state("<b>", State::Rawtext, None),
        vec![Token::Character("<b>".into())]
    );
    assert_eq!(
        tokenize_state("<!--x--></script>", State::ScriptData, Some("script")),
        vec![
            Token::Character("<!--x-->".into()),
            Token::EndTag {
                name: "script".into()
            }
        ]
    );
    match &tokenize("<!DOCTYPE html>")[0] {
        Token::Doctype { name, correct, .. } => {
            assert_eq!(name.as_deref(), Some("html"));
            assert!(*correct);
        }
        t => panic!("{t:?}"),
    }
}

#[test]
fn policy_builder() {
    let p = Policy::new()
        .allow_tag("IFRAME")
        .allow_tag("iframe")
        .allow_attr("IFRAME", "SRC")
        .allow_attr("iframe", "src")
        .allow_scheme("FTP")
        .allow_scheme("ftp");
    assert_eq!(
        sanitize_with("<iframe src='ftp://x' srcdoc=y></iframe>", &p),
        "<iframe src=\"ftp://x\"></iframe>"
    );
    assert_eq!(sanitize("<script>x</script><b>y</b>"), "<b>y</b>");
    assert_eq!(Policy::default(), Policy::new());
    assert!(is_safe_url("https://e.com"));
    assert!(!is_safe_url(" java\tscript:alert(1)"));
    assert!(is_safe_url_with("ftp://x", &["ftp"]));
    assert_eq!(DEFAULT_SCHEMES.len(), 4);
}

#[test]
fn selectors() {
    let doc = parse("<ul><li class=a>1<li>2</ul>");
    let sel = Selector::parse("li.a").unwrap();
    let li = doc.elements()[1];
    assert!(sel.matches(&doc, li));
    assert!(!sel.matches(&doc, doc.root()));
    let err = Selector::parse("li >").unwrap_err();
    assert_eq!(err.code(), "BAD_SELECTOR");
    assert_eq!(err.offset, 4);
    assert_eq!(err.to_string(), "BAD_SELECTOR at character 4");
    let e: &dyn std::error::Error = &err;
    assert!(e.source().is_none());
    assert_eq!(doc.query("li:nth-last-child(-n+1)").unwrap().len(), 1);
    assert_eq!(doc.query("ul li ~ li").unwrap().len(), 1);
    for bad in [
        "[a=\"x]",
        "li:nth-child(1",
        ":not(li",
        "li:nth-child(2n+)",
        "li:nth-child(2nx)",
        "[a^]",
        "li:not(x",
    ] {
        assert!(doc.query(bad).is_err(), "{bad}");
    }
    for good in [
        "[class|=a]",
        "li:nth-child(-2n+3)",
        "li:nth-child(n)",
        "li:nth-child(-n- 0)",
        "* > li",
    ] {
        assert!(doc.query(good).is_ok(), "{good}");
    }
}

#[test]
fn meta_and_tables() {
    let doc = parse("<html lang=en><title> </title><link rel=canonical><link rel=canonical href=/c><meta content=x>");
    let m = doc.meta();
    assert_eq!(m.title, None);
    assert_eq!(m.lang.as_deref(), Some("en"));
    assert_eq!(m.canonical.as_deref(), Some("/c"));
    assert_eq!(m.description, None);
    assert_eq!(PageMeta::default().og.len(), 0);
    let t =
        parse("<table><tr><td colspan=+2 rowspan=2>a<td colspan=x>b<tr><td colspan=0>c</table>")
            .tables();
    assert_eq!(t, vec![vec![vec!["a", "a", "b"], vec!["a", "a", "c"]]]);
    let big = parse("<table><tr><td colspan=99999999999 rowspan=0>z</table>").tables();
    assert_eq!(big[0][0].len(), 1000);
    assert_eq!(MAX_CELLS, 1_000_000);
    let empty: Table = parse("<table><tr></table>").tables().remove(0);
    assert_eq!(empty, vec![Vec::<String>::new()]);
}

#[test]
fn many_cells_stop() {
    let row = "<tr><td colspan=1000 rowspan=65534>x";
    let html = format!("<table>{}</table>", row.repeat(2));
    let t = parse(&html).tables();
    assert_eq!(t[0][0].len(), 1000);
    let mut html = String::from("<table>");
    for _ in 0..1001 {
        html.push_str("<tr><td colspan=1000>x");
    }
    let t = parse(&html).tables();
    assert_eq!(t[0].len(), 1001);
    assert!(t[0][1000].is_empty());
}

#[test]
fn helpers() {
    assert_eq!(strip_tags("<p>a<script>b</script></p>"), "a");
    assert_eq!(extract_text("<pre>a\n  b</pre>"), "a\n  b");
    assert_eq!(
        decode_entities("&#x110000;&#0;&#xD800;&#128;"),
        "\u{fffd}\u{fffd}\u{fffd}\u{20ac}"
    );
    assert_eq!(escape_text("<&\u{a0}>"), "&lt;&amp;&nbsp;&gt;");
    assert_eq!(escape_attr("\"<&\u{a0}>"), "&quot;&lt;&amp;&nbsp;&gt;");
}

#[test]
fn linear_on_large_inputs() {
    let start = std::time::Instant::now();
    let doc = parse(&format!("<ul>{}</ul>", "<li>x".repeat(20000)));
    for sel in [
        "li:first-child",
        "li ~ li",
        "ul li + li",
        "li:nth-last-child(2)",
    ] {
        doc.query(sel).unwrap();
    }
    for html in [
        "a".repeat(200_000),
        format!("<a href=\"{}\">", "a".repeat(200_000)),
        format!("<!--{}", "a".repeat(200_000)),
        "1<".repeat(100_000),
    ] {
        tokenize(&html);
    }
    assert!(start.elapsed().as_secs() < 10, "{:?}", start.elapsed());
}
