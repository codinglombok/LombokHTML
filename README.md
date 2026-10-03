# LombokHTML

> A WHATWG HTML tokenizer that passes the html5lib-tests tokenizer suite, a small tree builder, serialization, text extraction, an allowlist sanitizer, a CSS selector subset, page metadata and table extraction. The same input gives the same output in Rust, TypeScript, Python, Go and PHP. No runtime dependencies.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/codinglombok/LombokHTML/actions/workflows/ci.yml/badge.svg)](https://github.com/codinglombok/LombokHTML/actions/workflows/ci.yml)
[![Vectors](https://img.shields.io/badge/shared%20vectors-228%20x%205%20ports-success)](vectors/)
[![html5lib-tests](https://img.shields.io/badge/html5lib--tests%20tokenizer-7028%20cases-success)](conformance/)
[![Lombok Ecosystem](https://img.shields.io/badge/Lombok-Ecosystem-2e7d5b?logo=github)](https://github.com/codinglombok)

Part of the [Lombok Ecosystem](https://github.com/codinglombok).

## Mengapa library ini? (Why this library?)

- **One behaviour in five languages.** html5ever, parse5, html5lib, golang.org/x/net/html and DOMDocument each build slightly different trees and text. LombokHTML has one [SPEC](docs/SPEC_LombokHTML_v0.2.0.md) and 228 shared cases, so a scraper, an indexer and a sanitizer give the same result in the backend and the frontend.
- **A real tokenizer.** Every port runs the 7028 tokenizer cases of html5lib-tests (character references, script data escapes, comments, DOCTYPE), not a regex approximation.
- **Safe by default.** The sanitizer is an allowlist, drops `script`/`style`/`iframe` with their content, checks URL schemes like a browser does, and its output is a fixed point (sanitizing twice changes nothing).
- **Bounded.** Depth 256, at most one million table cells, linear time on long text, attributes, comments and large sibling lists (checked by tests in every port).
- **Small and embeddable.** No dependencies; the Rust crate is `no_std` + `alloc` with `forbid(unsafe_code)`.

## Installation

| Language | Package | Status |
|---|---|---|
| Rust | `lombokhtml` (crates.io) | not yet published |
| TypeScript / JavaScript | `lombokhtml` (npm) | not yet published |
| Python | `lombokhtml` (PyPI) | not yet published |
| Go | `github.com/codinglombok/lombokhtml/go` | tag `go/v0.2.0` on release |
| PHP | `codinglombok/lombokhtml` (Packagist) | needs a split repository first |

## Quick start

```ts
import { parse, sanitize, htmlToText, stripTags } from 'lombokhtml';

const html = '<h1>Title</h1><p onclick="x()">Body <a href="javascript:alert(1)">link</a> &amp; more</p>';
sanitize(html);      // '<h1>Title</h1><p>Body <a>link</a> &amp; more</p>'
htmlToText(html);    // '# Title\n\nBody link & more'
stripTags(html);     // 'TitleBody link & more'
parse(html).query('h1, p > a').map((e) => e.serialize());
// ['<h1>Title</h1>', '<a href="javascript:alert(1)">link</a>']
```

```rust
use lombokhtml::{parse, sanitize_with, Policy};

let doc = parse("<table><tr><td rowspan=2>A<td>B<tr><td>C</table>");
assert_eq!(doc.tables(), vec![vec![vec!["A", "B"], vec!["A", "C"]]]);
let clean = sanitize_with("<iframe src='https://x'></iframe>", &Policy::new().allow_tag("iframe").allow_attr("iframe", "src"));
```

```python
import lombokhtml as h

h.parse('<title>Q3</title><meta property="og:title" content="Report">').meta().to_dict()
# {'title': 'Q3', 'description': None, 'canonical': None, 'lang': None, 'og': [['title', 'Report']]}
h.tokenize('<a href=x>&copy;</a>')
# [('StartTag', 'a', [('href', 'x')], False), ('Character', '©'), ('EndTag', 'a')]
```

```go
doc := lombokhtml.Parse(html)
links, err := doc.Query(`a[href^="https"]`) // err is *SelectorError for an invalid selector
```

```php
use LombokHTML\Html;

Html::sanitize('<img src=x onerror=alert(1)>');   // '<img src="x">'
```

## What it does not do

The tree builder is a subset of the WHATWG algorithm: no implied `html`/`head`/`body`/`tbody`, no adoption agency or foster parenting, no SVG/MathML, `noscript` is always raw text. It is meant for extraction, sanitizing and querying, not for reproducing a browser DOM. See [docs/full_summary_project_LombokHTML_v0.2.0.md](docs/full_summary_project_LombokHTML_v0.2.0.md) and SPEC section 12.

## Development

```bash
python3 scripts/gen_ports.py --check && python3 vectors/build_vectors.py && python3 vectors/check_conformance.py
cd rust && cargo test && cargo clippy --all-targets -- -D warnings
cd typescript && npm ci && npm run coverage
cd python && python -m pytest
cd go && go test ./...
cd php && php tests/run.php
bash scripts/lombok-doctor.sh LombokHTML
```

## License

Apache-2.0 ([LICENSE](LICENSE)). The html5lib-tests conformance data in `conformance/` is MIT ([conformance/LICENSE-html5lib-tests](conformance/LICENSE-html5lib-tests)).
