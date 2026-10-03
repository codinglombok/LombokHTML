# LombokHTML — Full Summary Project v0.2.0

| Item | Nilai |
|---|---|
| Deskripsi | Tokenizer HTML WHATWG (lulus html5lib-tests tokenizer), tree builder subset, serialisasi, ekstraksi teks, sanitizer allowlist, subset selector CSS, metadata dan tabel, dengan hasil sama di 5 bahasa; tanpa dependensi runtime |
| Cluster · tingkat | 03 Format, Parser & Serialisasi · L0 |
| Referensi | SPEC + model Python independen `vectors/htmlmodel.py` + conformance html5lib-tests (7028 kasus) |
| Port | Rust (`no_std` + `alloc`, `forbid(unsafe_code)`), TypeScript, Python, Go, PHP |
| Vector | 228 kasus · SHA-256 `67b29c0a...a6c2e03a` |
| Test | setiap port: runner vector (228) + runner conformance (7028) + tes API, termasuk tes regresi waktu linear · Rust 9 tes API · TS 238 · Python 238 · Go 7 tes API · PHP 31 cek API + 7304 cek vector/conformance |
| Coverage | Rust 99,1% baris · TS 99,8% baris / 98,5% cabang · Python 99% · Go 99,0% · PHP 99,0% baris |
| Registry | crates.io, npm, PyPI `lombokhtml`; Go `github.com/codinglombok/lombokhtml/go`; Packagist `codinglombok/lombokhtml` (semua belum terbit) |
| Lisensi | Apache-2.0; data conformance html5lib-tests MIT |

## 1. Tabel gap vs pembanding (jujur)

| Kemampuan | LombokHTML 0.2.0 | html5ever (Rust) | parse5 (JS) | html5lib / BeautifulSoup (Python) | x/net/html (Go) | DOMDocument / Dom\HTMLDocument (PHP) |
|---|---|---|---|---|---|---|
| Hasil identik lintas bahasa (SPEC + vector) | YA | TIDAK | TIDAK | TIDAK | TIDAK | TIDAK |
| Tokenizer WHATWG (html5lib-tests) | YA | YA | YA | YA | YA | PHP 8.4 (`Dom\HTMLDocument`, lexbor); `DOMDocument` tidak |
| Tree builder WHATWG penuh | TIDAK (subset) | YA | YA | YA | YA | PHP 8.4 YA |
| SVG/MathML (foreign content) | TIDAK | YA | YA | YA | YA | PHP 8.4 YA |
| Sanitizer bawaan | YA (allowlist, idempoten) | TIDAK (ammonia terpisah) | TIDAK (DOMPurify terpisah) | TIDAK (bleach terpisah, usang) | TIDAK (bluemonday terpisah) | TIDAK (HTML Purifier terpisah) |
| Selector CSS | subset | TIDAK (crate terpisah) | TIDAK | YA (soupsieve) | TIDAK (cascadia terpisah) | PHP 8.4 `querySelector` |
| Teks terstruktur, meta, tabel | YA | TIDAK | TIDAK | sebagian (get_text) | TIDAK | TIDAK |
| `no_std` / tanpa dependensi | YA | TIDAK | YA | TIDAK (BeautifulSoup + parser) | YA | ekstensi bawaan |

Posisi unik yang dibuktikan test: token, pohon, teks, HTML bersih, hasil selector, metadata dan tabel yang sama persis di lima bahasa untuk 228 kasus, dengan tokenizer yang lulus seluruh tes tokenizer html5lib pada commit yang dipin.

## 2. Batasan yang Diketahui

1. Tree builder adalah subset (SPEC §12): tanpa elemen implisit `html/head/body/tbody`, adoption agency, foster parenting, `template` content, SVG/MathML. Pohon dapat berbeda dari DOM browser untuk HTML yang rusak.
2. `noscript` selalu raw text (scripting aktif).
3. Selector adalah subset: tanpa `:is/:where/:has`, `*-of-type`, escape CSS, flag `i`, namespace.
4. Sanitizer belum diuji terhadap korpus mXSS di browser nyata dan belum diaudit; isi elemen raw text yang diizinkan selalu dibuang.
5. Tidak ada API streaming; seluruh dokumen dimuat ke memori.
6. Paket PHP berada di subdirektori; Packagist butuh repositori split.
7. Masukan dengan surrogate tunggal (TypeScript, Python) di luar kontrak.

## 3. Prinsip Universal (ringkas, untuk publik)

| Prinsip | Status | Bukti |
|---|---|---|
| U1 Mandiri | YA | README tanpa klaim kepemilikan |
| U2 Modern | YA | WHATWG HTML tinjauan 2026-10-03, html5lib-tests dipin |
| U3 Multi-platform | YA | CI ubuntu/windows/macos; Rust `no_std` + `alloc` |
| U4 Multi-bahasa | YA | 5 port, runner vector dan conformance di semua port |
| U5 Rentang skala | YA | perangkat kecil (`no_std`) sampai server; waktu linear diuji |
| U6 Lengkap & unik | SEBAGIAN | tabel gap di atas |
| U7 Aman & teruji | YA | SPEC §11; batas kedalaman/sel/span; sanitizer idempoten; coverage ≥ 99% di semua port |
| U8 Ekosistem tanpa kopling | YA | 0 dependensi wajib |
| U9 Internasional | YA | Unicode penuh, perbandingan ASCII-only; Lang_ |
| U10 Lisensi | YA | Apache-2.0; data pihak ketiga dengan lisensinya |
| U11 Siap registri | SEBAGIAN | crates/npm/PyPI/Go siap; Packagist butuh split repo |
| U12 Dokumentasi | YA | 10 dokumen publik + 2 internal |
| U13 Kerahasiaan & dokumen bersih | YA | `lombok-doctor.sh`: 0 emoji, `.gitignore` ADR-024 |

*Lisensi dokumen: Apache-2.0 · © codinglombok*
