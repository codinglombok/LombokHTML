# LombokHTML

HTML ingestion: tolerant parser and DOM, entity decoding, text extraction, an allowlist sanitizer, CSS selectors, meta and table extraction.

A standalone, general-purpose library of the **Lombok Ecosystem** — Tier **L0**. No mandatory dependency on any other Lombok library (L0).

> **Universal by design.** Usable by anyone — from small embedded devices to premium industrial software — without any application or framework. It is not part of, and not owned by, any app or server (e.g. RAG stacks); apps are merely example users. See [`docs/masterplan_LombokHTML_v0.1.0.md`](docs/masterplan_LombokHTML_v0.1.0.md) §2 for the U1–U12 evidence table (honest ✅/🟡/⚪ status).

## Status

🔵 **v0.1.0 — not yet published** to GitHub/registries. Rust reference + **TypeScript port** both pass the same shared test vectors (`vectors/lombokhtml-vectors-v1.json`, byte-identical behaviour per ADR-015). Python/Go/PHP ports are stubs. **Security note:** the sanitizer is allowlist-based and checked against an XSS corpus and an independent parser, but it has not been tested against real browsers (mXSS) or audited — add a Content-Security-Policy for highly untrusted input.

## Features

- **Error-tolerant tokenizer & DOM** (subset HTML5): quoted/unquoted/boolean attributes (`>` inside quotes is safe), void & raw-text elements, comments, doctype; nesting-error recovery; depth capped at 256
- **Entities**: numeric + common named references decoded; output always escaped
- **Text**: `strip_tags`; `extract_structured_text` (headings `#`, lists `- `, table cells tab-separated)
- **Allowlist sanitizer (secure by default)**: unknown tags unwrapped, dangerous tags dropped with content, attributes/URLs allowlisted (browser-style scheme check), customizable `Policy`, idempotent output
- **Selectors**: tag, `*`, `.class`, `#id`, `[attr]`, `= ^= $= *= ~=`, descendant ` ` and child `>` combinators, `,` lists
- **Meta** (`title`, `description`, `og:*`) and **tables** (`colspan` + `rowspan`, size-capped, nested tables correct)
- `no_std + alloc` core, zero dependencies

## Quick Start

### Rust (reference)

```toml
[dependencies]
lombokhtml = { git = "https://github.com/codinglombok/LombokHTML", package = "lombokhtml" }   # not on crates.io yet
```

```rust
use lombokhtml::*;
let html = "<h1>Title</h1><p onclick='x()'>Body <a href='javascript:alert(1)'>link</a></p>";
sanitize(html);                       // "<h1>Title</h1><p>Body <a>link</a></p>"
strip_tags(html);                     // "TitleBody link"
let doc = parse(html);
query(&doc, "h1, p > a").len();       // 2
extract_tables(&parse("<table><tr><td rowspan=2>A</td><td>B</td></tr><tr><td>C</td></tr></table>"))[0]; // [["A","B"],["A","C"]]
```

### TypeScript

```bash
cd typescript && npm install && npm test     # builds, then runs every shared vector
```

Zero runtime dependencies, ESM, Node ≥ 18. See `docs/API_LombokHTML_v0.1.0.md` for the camelCase API.

## Testing

```bash
cd rust && cargo test --release                       # unit + robustness (pseudo-fuzz) + shared vectors
cd rust && cargo build --no-default-features          # no_std + alloc proof
cd typescript && npm test                             # same vectors, TypeScript port
LOMBOK_REGEN=1 cargo test --release --test vectors    # regenerate expected outputs from the Rust reference (review the diff!)
./scripts/lombok-doctor-docs.sh LombokHTML                # 12 docs, versions, vector hash, license, no ownership claims
```

Vector inputs are authored in `vectors/gen_inputs.py` (deterministic); expected outputs come from the Rust reference and are reviewed by hand and, where possible, by independent checks. Changing any vector requires updating its SHA-256 in `docs/SPEC_LombokHTML_v0.1.0.md` (CI enforces it).

## Documentation (12 standard documents)

All in [`docs/`](docs/): masterplan · architecture · changelog · map · structure_repo · full_summary_project · guide_how_to_use · how_to_dist · development_ide · **API** · **Lang** · **SPEC** (normative contract). Distribution to GitHub & registries: [`how_to_dist`](docs/how_to_dist_LombokHTML_v0.1.0.md).

## License

Apache 2.0 — see [LICENSE](LICENSE). (MASTERPLAN §10.1 suggests `Apache-2.0 OR MIT`; pending owner decision.)
