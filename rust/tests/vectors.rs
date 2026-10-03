//! Runs every case of vectors/lombokhtml-vectors-v1.json.
use lombokhtml::*;
use serde_json::{json, Value};

fn token_json(t: &Token) -> Value {
    match t {
        Token::StartTag {
            name,
            attrs,
            self_closing,
        } => {
            let a: Vec<Value> = attrs.iter().map(|(k, v)| json!([k, v])).collect();
            json!(["StartTag", name, a, self_closing])
        }
        Token::EndTag { name } => json!(["EndTag", name]),
        Token::Character(d) => json!(["Character", d]),
        Token::Comment(d) => json!(["Comment", d]),
        Token::Doctype {
            name,
            public_id,
            system_id,
            correct,
        } => {
            json!(["DOCTYPE", name, public_id, system_id, correct])
        }
    }
}

fn policy(v: &Value) -> Policy {
    let mut p = Policy::new();
    let strs = |k: &str| -> Vec<String> {
        v.get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect())
            .unwrap_or_default()
    };
    for t in strs("tags") {
        p = p.allow_tag(&t);
    }
    if let Some(attrs) = v.get("attrs").and_then(Value::as_array) {
        for pair in attrs {
            p = p.allow_attr(pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
        }
    }
    for s in strs("schemes") {
        p = p.allow_scheme(&s);
    }
    p
}

fn run(kind: &str, i: &Value) -> Value {
    let s = |k: &str| i[k].as_str().unwrap();
    let ok = |v: Value| json!({ "ok": v });
    match kind {
        "decode" => ok(json!(decode_entities(s("text")))),
        "escapeText" => ok(json!(escape_text(s("text")))),
        "escapeAttr" => ok(json!(escape_attr(s("text")))),
        "tokenize" => ok(Value::Array(
            tokenize(s("html")).iter().map(token_json).collect(),
        )),
        "parse" => {
            let d = parse(s("html"));
            ok(json!(d.serialize(d.root())))
        }
        "textContent" => ok(json!(strip_tags(s("html")))),
        "extractText" => ok(json!(extract_text(s("html")))),
        "sanitize" => {
            let p = i.get("policy").map(policy).unwrap_or_default();
            let out = sanitize_with(s("html"), &p);
            assert_eq!(sanitize_with(&out, &p), out, "sanitizer not idempotent");
            ok(json!(out))
        }
        "safeUrl" => match i.get("schemes") {
            Some(sc) => {
                let v: Vec<&str> = sc
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_str().unwrap())
                    .collect();
                ok(json!(is_safe_url_with(s("url"), &v)))
            }
            None => ok(json!(is_safe_url(s("url")))),
        },
        "query" => {
            let d = parse(s("html"));
            match d.query(s("selector")) {
                Ok(ids) => ok(json!(ids
                    .iter()
                    .map(|&id| d.serialize(id))
                    .collect::<Vec<_>>())),
                Err(e) => json!({ "error": e.code() }),
            }
        }
        "meta" => {
            let m = parse(s("html")).meta();
            let og: Vec<Value> = m.og.iter().map(|(k, v)| json!([k, v])).collect();
            ok(
                json!({"title": m.title, "description": m.description, "canonical": m.canonical,
                      "lang": m.lang, "og": og}),
            )
        }
        "tables" => ok(json!(parse(s("html")).tables())),
        other => panic!("unknown kind {other}"),
    }
}

#[test]
fn vectors() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vectors/lombokhtml-vectors-v1.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 100);
    let mut failed = Vec::new();
    for c in cases {
        let got = run(c["kind"].as_str().unwrap(), &c["input"]);
        if got != c["expected"] {
            failed.push(format!("{}: got {} want {}", c["id"], got, c["expected"]));
        }
    }
    assert!(
        failed.is_empty(),
        "{} failures:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn conformance() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../conformance/html5lib-tokenizer.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    let mut failed = Vec::new();
    let cases = doc["cases"].as_array().unwrap();
    for c in cases {
        let state = match c["state"].as_str().unwrap() {
            "data" => State::Data,
            "rcdata" => State::Rcdata,
            "rawtext" => State::Rawtext,
            "script" => State::ScriptData,
            "plaintext" => State::Plaintext,
            "cdata" => State::CdataSection,
            other => panic!("state {other}"),
        };
        let toks = tokenize_state(
            c["input"].as_str().unwrap(),
            state,
            c["lastStartTag"].as_str(),
        );
        let got = Value::Array(toks.iter().map(token_json).collect());
        if got != c["output"] {
            failed.push(format!(
                "{} {}: got {} want {}",
                c["file"], c["description"], got, c["output"]
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "{} of {} failures:\n{}",
        failed.len(),
        cases.len(),
        failed[..failed.len().min(20)].join("\n")
    );
}
