# LombokHTML — STRUCTURE REPO v0.1.0

## 1. Struktur Folder

```
LombokHTML/
├── rust/            # src/{lib,tokenizer,entities,dom,text,sanitize,selector,meta,table}.rs · tests/{vectors,robustness}.rs
├── typescript/      # src/{index,compat}.ts · test/vectors.test.ts (lulus vektor bersama)
├── python/ go/ php/ # stub README
├── vectors/         # lombokhtml-vectors-v1.json (3.099 kasus) + gen_inputs.py
├── docs/  scripts/  .github/workflows/{ci,release}.yml
├── README.md  LICENSE  .gitignore
```

## 2. Konvensi Penamaan (mengikuti MASTERPLAN_UTAMA §4)

| Platform | Nama di repo ini |
|---|---|
| GitHub repo | `codinglombok/LombokHTML` |
| Package registry (npm/PyPI/crates.io) | `lombokhtml` |
| Packagist | `codinglombok/lombokhtml` |
| Go module | `github.com/codinglombok/lombokhtml/go` (tag `go/vX.Y.Z`) |
| Namespace/Class | `LombokHTML` (Rust: `lombokhtml::`) |

## 3. File Wajib di Root

| File | Ada? |
|---|---|
| `README.md` | ✅ |
| `LICENSE` (dual-license: `LICENSE-APACHE` + `LICENSE-MIT`) | ✅ |
| `.gitignore` | ✅ |
| `CHANGELOG.md` → lihat `changelog_LombokHTML_v0.1.0.md` | ✅ |
| `.github/workflows/ci.yml` | ✅ |
| `.github/workflows/release.yml` | ✅ |
| 12 dokumen standar (`docs/`) | ✅ — lihat daftar di [Masterplan](masterplan_LombokHTML_v0.1.0.md) |

## 4. Struktur per Bahasa Port

- `rust/` — implementasi referensi; `LOMBOK_REGEN=1 cargo test --test vectors` menulis ulang ekspektasi vektor dari referensi.
- `typescript/` — port native tanpa dependensi (referensi profil web/doc); `npm test` menjalankan seluruh vektor + uji ketahanan.
- `python/ go/ php/` — stub.

## 5. Catatan

Tidak ada `locales/`: library ini tidak menghasilkan pesan yang perlu diterjemahkan (lihat Lang_).
