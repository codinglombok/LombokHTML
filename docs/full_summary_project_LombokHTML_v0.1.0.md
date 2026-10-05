# LombokHTML — FULL SUMMARY PROJECT v0.1.0

Ringkasan satu-halaman untuk pembaca baru (reviewer, calon kontributor, aplikasi yang mengevaluasi dependensi) — tidak perlu membaca 11 dokumen lain untuk memahami apa, mengapa, dan status proyek ini.

## Apa Ini?

Ingesti dokumen HTML zero-dependency: tokenizer & pembangun DOM toleran-galat (subset HTML5), dekode entitas, penghapus tag, ekstraksi teks berstruktur, **sanitizer allowlist (aman secara bawaan)**, query selector CSS (combinator turunan/anak, daftar), ekstraksi meta (title/description/og:*) dan tabel (colspan + rowspan) → data terstruktur.

## Mengapa Dibuat (bukan pakai library pihak ketiga)?

`html5ever`/`scraper` besar dan menarik banyak dependensi; DOMPurify hanya JS. LombokHTML kecil, `no_std + alloc`, dan menjamin keluaran identik lintas port.

## Fitur Utama

*Library ini mandiri dan universal (PRINSIP_UNIVERSAL U1); tidak menjadi bagian dari aplikasi/framework mana pun.*

- Tokenizer toleran-galat (atribut quoted/unquoted/boolean, void, raw-text, komentar, doctype; `>` dalam atribut berkutip)
- Dekode entitas; DOM dengan pemulihan nesting salah; kedalaman dibatasi
- `strip_tags`, `extract_structured_text`
- **Sanitizer allowlist** dengan `Policy` yang dapat disesuaikan, pemeriksaan URL gaya-browser, serialisasi ber-escape dan idempoten
- Selector dengan combinator (` `, `>`), daftar (`,`), operator atribut (`= ^= $= *= ~=`)
- Meta title/description/og:*
- Tabel → `Vec<Vec<String>>` dengan colspan + rowspan, nested table benar

## Status Saat Ini

| Aspek | Nilai |
|---|---|
| Status | 🔵 BUILDING — inti Rust + port TypeScript lulus vektor bersama; belum dipublikasikan ke GitHub/registry |
| Versi | 0.1.0 |
| Bahasa referensi | Rust |
| Port tersedia | TypeScript ✅ (lulus seluruh vektor); Python, Go, PHP — folder stub, belum ada kode |
| Test | 60 test Rust (unit + robustness 20.000 iterasi & masukan patologis + 3.099 vektor); port TypeScript lulus 3.099 vektor yang sama + uji ketahanan; keluaran sanitizer 533 kasus diverifikasi parser Python independen; `no_std` lulus; `clippy -D warnings` bersih; uji mutasi TS (skema `data:`, jepitan colspan, dll.) tertangkap |
| Dependensi wajib | — (L0: tanpa dependensi Lombok wajib) |
| Tingkat (tier) | L0 |

## Contoh Pemakai (ilustrasi, bukan kepemilikan)

- Siapa pun yang membaca, membersihkan, atau mengekstrak HTML; tidak ada pemakai yang memiliki library ini.
- Contoh dependen di ekosistem Lombok dicatat hanya di `map_` (arah dependensi).

## Batasan yang Diketahui (Known Limitations)

- Bukan algoritma HTML5 penuh (tanpa elemen implisit, tanpa auto-close `p/li/td`; `<table>` tanpa `tbody` tidak dinormalisasi; hasil pada markup salah-bentuk berbeda dari browser).
- Entitas: hanya numerik + 30 nama umum; referensi tanpa `;` (`&amp`) dan rentang 128–159 Windows-1252 tidak ditangani.
- Sanitizer: tanpa pengujian browser nyata dan tanpa audit (lihat catatan keamanan); CSS/SVG/MathML dibuang, bukan disanitasi; `class`/`id`/`style` dibuang secara bawaan.
- Selector: tanpa sibling combinator (`+`, `~`) dan pseudo-class; token dipisah spasi (nilai atribut berspasi tidak didukung); pencocokan relatif terhadap subpohon yang di-query.
- `rowspan=0` diperlakukan sebagai 1; sel yang bertumpuk pada grid tidak beraturan mengikuti aturan sederhana (bukan algoritma tabel HTML penuh).
- Masukan `&str`/UTF-8: tidak ada deteksi encoding.
- Port Python/Go/PHP belum ada; C-ABI/WASM belum dibuat.

## Untuk Info Lebih Lanjut

- Cara pakai → [Guide How to Use](guide_how_to_use_LombokHTML_v0.1.0.md)
- Cara instal/deploy → [How to Dist](how_to_dist_LombokHTML_v0.1.0.md)
- API lengkap → [API](API_LombokHTML_v0.1.0.md)
- Kontrak lintas bahasa → [SPEC](SPEC_LombokHTML_v0.1.0.md)

## Gap vs Pembanding (U6)

| Pembanding | Kelebihan pembanding | Posisi LombokHTML (jujur) |
|---|---|---|
| html5ever / parse5 | Algoritma pohon HTML5 penuh, semua entitas, sesuai browser | Belum setara; LombokHTML unggul pada ukuran kecil, `no_std + alloc`, tanpa dependensi |
| DOMPurify / ammonia | Sanitizer allowlist teruji lama, diuji terhadap browser | Kini allowlist dan diuji terhadap korpus XSS + parser independen, **tetapi belum diuji terhadap browser nyata dan belum diaudit** |
| Cheerio / scraper | Selector lengkap (sibling, pseudo-class) | Selector dasar + combinator turunan/anak |
| Keunikan yang diklaim | — | Perilaku byte-identik lintas bahasa (Rust + TS terbukti); ekstraksi teks berstruktur + tabel (rowspan) + meta dalam satu inti kecil |
