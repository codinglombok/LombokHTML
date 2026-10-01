//! Cross-language conformance runner (ADR-002/015). `LOMBOK_REGEN=1` rewrites
//! expected outputs from THIS reference implementation — review the diff.

use lombokhtml::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vectors/lombokhtml-vectors-v1.json")
}

/// Language-neutral tree dump: text -> string, comment -> {"!": text},
/// element -> {"t": tag, "a": [[k, v], ...], "c": [children]}, document -> {"d": [children]}.
fn dump(n: &Node) -> Value {
    match n.kind {
        NodeKind::Text => json!(n.text.clone().unwrap_or_default()),
        NodeKind::Comment => json!({"!": n.text.clone().unwrap_or_default()}),
        NodeKind::Element => json!({
            "t": n.tag,
            "a": n.attrs.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
            "c": n.children.iter().map(dump).collect::<Vec<_>>(),
        }),
        NodeKind::Document => json!({"d": n.children.iter().map(dump).collect::<Vec<_>>()}),
    }
}

fn eval(c: &Value) -> Value {
    let i = &c["in"];
    match c["fn"].as_str().unwrap() {
        "parse" => json!({"out": dump(&parse(i.as_str().unwrap()))}),
        "strip_tags" => json!({"out": strip_tags(i.as_str().unwrap())}),
        "extract_structured_text" => json!({"out": extract_structured_text(i.as_str().unwrap())}),
        "sanitize" => json!({"out": sanitize(i.as_str().unwrap())}),
        "decode_entities" => json!({"out": decode_entities(i.as_str().unwrap())}),
        "is_safe_url" => {
            json!({"out": is_safe_url(i.as_str().unwrap(), &Policy::default().allowed_schemes)})
        }
        "extract_tables" => json!({"out": extract_tables(&parse(i.as_str().unwrap()))}),
        "extract_meta" => {
            let m = extract_meta(&parse(i.as_str().unwrap()));
            json!({"out": {"title": m.title, "description": m.description,
                "og": m.og_tags.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>()}})
        }
        "query" => {
            let doc = parse(i["html"].as_str().unwrap());
            let found = query(&doc, i["selector"].as_str().unwrap());
            json!({"out": found.iter().map(|n| json!({"t": n.tag,
                "a": n.attrs.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>()})).collect::<Vec<_>>()})
        }
        other => panic!("unknown fn {other}"),
    }
}

#[test]
fn conformance() {
    let doc: Value = serde_json::from_str(&fs::read_to_string(path()).unwrap()).unwrap();
    let regen = std::env::var("LOMBOK_REGEN").is_ok();
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    for (n, c) in doc["cases"].as_array().unwrap().iter().enumerate() {
        let got = eval(c);
        let mut merged = json!({"fn": c["fn"], "in": c["in"]});
        merged["out"] = got["out"].clone();
        if !regen && c.get("out") != got.get("out") {
            failures.push(format!(
                "case {n} {} in={} expected={} got={}",
                c["fn"], c["in"], c["out"], got["out"]
            ));
        }
        lines.push(serde_json::to_string(&merged).unwrap());
    }
    if regen {
        fs::write(
            path(),
            format!(
                "{{\"suite\":\"lombokhtml\",\"version\":1,\"cases\":[\n{}\n]}}\n",
                lines.join(",\n")
            ),
        )
        .unwrap();
        return;
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
