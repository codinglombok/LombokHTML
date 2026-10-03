# LombokHTML — Development IDE v0.2.0

## 1. Lingkungan

| Alat | Versi |
|---|---|
| Rust | stable (MSRV 1.70, diperiksa CI); `cargo-llvm-cov` untuk coverage |
| Node.js | 20 LTS atau lebih baru (CI: 20, 22, 24) |
| Python | 3.9+ (CI: 3.9, 3.11, 3.13); `pytest`, `coverage`; model referensi dan generator memakai pustaka standar saja |
| Go | 1.22+ (CI: 1.22, 1.24) |
| PHP | 8.1+ (CI: 8.1, 8.3); ekstensi `pcov` untuk coverage; tanpa mbstring/intl |
| Editor | VS Code, RustRover/GoLand/PhpStorm/PyCharm, atau editor lain |
| Bash | untuk `scripts/lombok-doctor.sh` (Windows: Git Bash atau WSL) |

## 2. Perintah

| Direktori | Perintah | Fungsi |
|---|---|---|
| `rust/` | `cargo test` | tes API + runner vector + runner conformance |
| `rust/` | `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` | lint |
| `rust/` | `cargo llvm-cov --fail-under-lines 90` | coverage |
| `typescript/` | `npm ci` · `npm run lint` · `npm test` | tipe strict, test |
| `typescript/` | `npm run coverage` | test dengan ambang baris 90, cabang 90, fungsi 85 |
| `python/` | `python -m pytest` · `coverage run --branch --source=lombokhtml -m pytest && coverage report --fail-under=90` | test, coverage |
| `go/` | `go vet ./...` · `go test -cover ./...` | lint, test |
| `php/` | `php tests/run.php` · `php -d pcov.enabled=1 tests/run.php --coverage=90` | test, coverage |
| root | `python3 scripts/gen_ports.py` (`--check` di CI) | bangun ulang tabel entitas kelima port dari `data/entities.json` |
| root | `python3 vectors/build_vectors.py` | bangun ulang berkas vector dan `SHA256SUMS` dari model |
| root | `python3 vectors/check_conformance.py` | jalankan model terhadap 7028 kasus html5lib-tests |
| root | `python3 scripts/import_html5lib.py <html5lib-tests>` | impor ulang conformance dari checkout html5lib-tests pada commit yang dipin |
| root | `bash scripts/lombok-doctor.sh LombokHTML` | pemeriksaan standar Lombok v3.6 |

## 3. Alur mengubah perilaku

1. Ubah SPEC lebih dulu.
2. Ubah model `vectors/htmlmodel.py` (ditulis dari SPEC, tanpa kode bersama dengan port) dan tambah kasus di `vectors/build_vectors.py`; nilai harapan tulis tangan harus cocok dengan model.
3. Jalankan builder dan `check_conformance.py`; perbarui hash di SPEC.
4. Ubah kelima port sampai semua runner hijau.
5. Catat di `CHANGELOG.md`.

## 4. Arah pengembangan

- Tree builder WHATWG penuh (insertion mode, adoption agency, foster parenting, elemen implisit) sebagai mode terpisah, dengan conformance html5lib-tests tree-construction.
- Foreign content (SVG, MathML) di tokenizer dan tree builder.
- Selector tambahan: `:is()`, `:where()`, `:has()`, `:nth-of-type()`, escape CSS, flag atribut `i`.
- API streaming (tokenizer berbasis callback/iterator) untuk dokumen sangat besar.
- Pembaruan pin html5lib-tests setelah token processing instruction distabilkan.

*Lisensi dokumen: Apache-2.0 · © codinglombok*
