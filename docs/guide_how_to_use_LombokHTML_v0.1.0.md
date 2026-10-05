# LombokHTML — GUIDE: HOW TO USE v0.1.0

## Instalasi

```toml
# Rust — saat ini via git (belum di crates.io)
[dependencies]
lombokhtml = { git = "https://github.com/codinglombok/LombokHTML", package = "lombokhtml" }
```

```bash
# TypeScript — saat ini dari sumber (belum di npm)
cd typescript && npm install && npm run build
```

Setelah rilis pertama: `cargo add lombokhtml` · `npm i lombokhtml`. Python/Go/PHP: belum ada port.

## Konsep Dasar

- **Token** → **Node** → operasi (teks, sanitasi, query, meta, tabel).
- **Toleran-galat**: masukan cacat tidak pernah menyebabkan error/panic; hasilnya sebaik mungkin.

## Contoh Penggunaan

```rust
use lombokhtml::*;
let html = "<h1>Judul</h1><p onclick='x()'>Isi <a href='javascript:alert(1)'>klik</a></p>";
let doc = parse(html);
query(&doc, "body p a, h1").len();  // selector dengan combinator + daftar
strip_tags(html);                    // "JudulIsi klik"
extract_structured_text(html);       // "# Judul\n\nIsi klik"
sanitize(html);                      // "<h1>Judul</h1><p>Isi <a>klik</a></p>"
extract_meta(&doc).title;            // None
```

## Pola Pemakaian Umum (Recipes)

- **Ingesti dokumen untuk pencarian/arsip/LLM**: `extract_structured_text` → pecah per heading.
- **Tabel harga**: `extract_tables(&parse(html))` lalu petakan baris pertama sebagai header.
- **Metadata halaman**: `extract_meta`.

## Kesalahan Umum (Common Pitfalls)

- `sanitize` mengeluarkan **fragmen HTML** untuk konteks HTML; bukan untuk disisipkan ke atribut/skrip/CSS. Tambahkan CSP untuk konten sangat tidak tepercaya.
- Default membuang `class`, `id`, `style`; izinkan lewat `Policy::allow_attr` bila perlu (risiko DOM clobbering/CSS injection).
- Hasil pada markup salah-bentuk mengikuti SPEC ini, bukan algoritma browser.
- `query(&doc, ...)` tidak mengembalikan `doc` itu sendiri.

## Lihat Juga

- [API Reference lengkap](API_LombokHTML_v0.1.0.md)
- [Full Summary](full_summary_project_LombokHTML_v0.1.0.md)
