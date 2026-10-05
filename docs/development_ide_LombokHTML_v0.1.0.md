# LombokHTML — DEVELOPMENT IDE / ARAH PENGEMBANGAN v0.1.0

"Ide" di sini = arah pengembangan (development direction), bukan editor kode.

## 1. Roadmap

| Versi | Isi | Status |
|---|---|---|
| 0.1.0 | Inti Rust + port TypeScript + vektor 3.099 kasus; sanitizer allowlist; entitas; combinator; rowspan | ✅ (belum dipublikasikan) |
| 0.2.0 | Uji terhadap browser nyata (mXSS); fuzz berbasis cakupan; ~2.000 entitas bernama; port Python | ⚪ |
| 0.3.0 | Port Go/PHP; algoritma pohon HTML5 lebih lengkap; sibling combinator | ⚪ |
| 0.4.0 | WASM/C-ABI; streaming | ⚪ |

## 2. Yang Sengaja Belum Dikerjakan (Deferred Scope)

Algoritma pohon HTML5 penuh, streaming parser, pseudo-class selector, deteksi encoding, entitas bernama lengkap, audit keamanan pihak ketiga.

## 3. Prinsip Desain untuk Kontributor Baru

Jangan pernah panic pada masukan apa pun (fuzz-friendly); setiap perilaku baru = vector baru; keamanan: default menolak, bukan mengizinkan.

## 4. Cara Berkontribusi

1. Baca [SPEC_LombokHTML_v0.1.0.md](SPEC_LombokHTML_v0.1.0.md) — perilaku yang diubah harus tetap sesuai kontrak, atau kontrak diajukan perubahan lewat RFC (lihat ARCHITECTURE_UTAMA §2, aturan L0 "perubahan breaking butuh RFC").
2. Tambahkan/ubah test vector di `vectors/lombokhtml-vectors-v1.json` **sebelum** mengubah implementasi (test-first, selaras ADR-002/ADR-015).
3. Jalankan `./scripts/lombok-doctor-docs.sh LombokHTML` — PR yang mengubah API publik wajib memperbarui `API_LombokHTML_v0.1.0.md` di PR yang sama.
4. `cargo test && cargo clippy -- -D warnings` harus hijau sebelum membuka PR.

## 5. Pertanyaan Terbuka (Open Questions)

- Daftar tag/atribut default sanitizer sudah tepat untuk kasus umum (artikel/komentar)? Apakah `class`/`id` perlu opsi bawaan?
- Perlu mode streaming untuk dokumen besar?
- Siapa/bagaimana melakukan pengujian mXSS terhadap browser nyata?
