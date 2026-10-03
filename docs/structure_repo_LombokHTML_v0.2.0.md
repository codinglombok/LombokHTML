# LombokHTML — Structure Repo v0.2.0

```
LombokHTML/
├── README.md · CHANGELOG.md · LICENSE (Apache-2.0)
├── .github/workflows/  ci.yml (5 port + standards) · release.yml (terbit pada tag v*)
├── docs/               10 dokumen publik; masterplan_ dan architecture_ adalah dokumen internal (ADR-024), tidak di-commit
├── data/               entities.json (2231 named character reference WHATWG)
├── conformance/        html5lib-tokenizer.json (7028 kasus) · LICENSE-html5lib-tests (MIT)
├── scripts/            gen_ports.py · import_html5lib.py · lombok-doctor.sh
├── vectors/            lombokhtml-vectors-v1.json · SHA256SUMS · htmlmodel.py (model referensi) · build_vectors.py · check_conformance.py
├── rust/               Cargo.toml · src/{lib,tokenizer,entities,entities_data,dom,sanitize,selector,meta,table}.rs · tests/{vectors,api}.rs
├── typescript/         package.json · src/{index,tokenizer,entities}.ts · tests/{api,vectors}.test.ts · scripts/coverage.mjs
├── python/             pyproject.toml · lombokhtml/{__init__,_tokenizer,_entities}.py · tests/{test_api,test_vectors}.py
├── go/                 go.mod · html.go · tokenizer.go · selector.go · entities.go · {api,vectors}_test.go
└── php/                composer.json · src/{Html,Node,Tokenizer,Policy,Selector,SelectorError,Entities}.php · tests/{run,api,vectors,bootstrap}.php
```

Aturan: `entities_data.rs`, `entities.ts`, `_entities.py`, `entities.go`, `Entities.php` dibangkitkan dan tidak diedit tangan (`python3 scripts/gen_ports.py --check` di CI); `conformance/html5lib-tokenizer.json` dibangkitkan `scripts/import_html5lib.py` dari commit html5lib-tests yang dipin; perubahan perilaku mengubah SPEC, model dan vector lebih dulu, lalu kelima port; hasil build tidak di-commit.

*Lisensi dokumen: Apache-2.0 · © codinglombok*
