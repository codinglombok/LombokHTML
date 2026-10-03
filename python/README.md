# LombokHTML — python port

WHATWG HTML tokenizer (passes the html5lib-tests tokenizer suite), a small tree builder, serialization, text extraction, an allowlist sanitizer, a CSS selector subset, page metadata and table extraction, with the same results as the other four ports. No runtime dependencies.

```bash
pip install lombokhtml
```

API mapping for this port: [docs/API_LombokHTML_v0.2.0.md](https://github.com/codinglombok/LombokHTML/blob/main/docs/API_LombokHTML_v0.2.0.md). Behaviour: [docs/SPEC_LombokHTML_v0.2.0.md](https://github.com/codinglombok/LombokHTML/blob/main/docs/SPEC_LombokHTML_v0.2.0.md).

Tests (run from this directory; they include the shared vectors in `../vectors` and the conformance file in `../conformance`):

```bash
python -m pytest
```

License: Apache-2.0. Part of the [Lombok Ecosystem](https://github.com/codinglombok).
