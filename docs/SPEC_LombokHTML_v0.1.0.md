# LombokHTML — SPEC v0.1.0

> This document is the normative cross-language contract. Every language port MUST produce byte-identical output for all specified inputs. Deviations from this specification are bugs.

(Kalimat di atas wajib verbatim per MASTERPLAN_UTAMA_v3.3 §5 — jangan diterjemahkan atau diparafrase.)

## 1. Vektor Uji (Test Vectors)

| Atribut | Nilai |
|---|---|
| Berkas | `vectors/lombokhtml-vectors-v1.json` |
| SHA-256 | `2d83fa1a8ab658075bdc9292aba270baa6e431f6541a74065ff827693f115ca8` |
| Diverifikasi CI oleh | `lombok-ci.yml` job `docs-and-vectors` (`sha256sum -c`) |

Setiap perubahan pada `vectors/lombokhtml-vectors-v1.json` **wajib** memperbarui SHA-256 di atas pada PR yang sama, atau CI gagal (ADR-015).

## 2. Kontrak per Fungsi

- **`tokenize`**: nama tag & atribut huruf kecil (ASCII); atribut boolean bernilai kosong; nilai atribut dan teks didekode (kecuali raw-text); `script`/`style` isinya satu token teks mentah sampai `</nama` (peka huruf); `>` di dalam nilai atribut berkutip tidak menutup tag; `<` yang tidak membentuk tag/komentar/doctype adalah teks; komentar `<!-- -->`; doctype tidak masuk pohon; end tag kosong diabaikan.
- **`decode_entities`**: `&#N;`/`&#xH;` (hanya digit radix; ≤8 digit; 0, surrogate, >U+10FFFF → U+FFFD) dan 30 nama umum; `;` harus muncul dalam 34 titik kode; selainnya dibiarkan (`&`).
- **`parse`**: void/self-closing tanpa anak; end tag menutup ke leluhur terbuka terdekat (yang di antaranya ikut ditutup); end tag tanpa pasangan dibuang; tag terbuka di akhir ditutup otomatis; kedalaman ≥256 → start tag menjadi daun.
- **`strip_tags`**: gabungan teks kecuali di `script/style/noscript/template`.
- **`extract_structured_text`**: `h1..h6` → `#`..`######`; `li` → `- `; `p/div/tr/ul/ol/table/br` → baris baru; `td/th` diakhiri tab; 3+ baris kosong dipadatkan jadi 2; hasil di-trim.
- **`sanitize`**: lihat catatan keamanan; tag di `drop_with_content` dibuang beserta isinya; tag lain di luar `allowed_tags` dibuka-bungkus; atribut hanya global (`title lang dir`) atau per-tag (`a[href] img[src alt width height] td/th[colspan rowspan] th[scope] ol[start] blockquote/q[cite] time[datetime]`); atribut URL lolos jika relatif atau skema ∈ {http, https, mailto, tel} setelah membuang spasi/kontrol dan lowercase; atribut ganda: yang pertama lolos yang dipakai; void → ` />`; teks di-escape `& < >`, atribut `& < > "`; komentar dibuang.
- **`query`**: daftar dipisah `,`; token dipisah spasi Unicode; `>` = anak, spasi = turunan; operator atribut `= ^= $= *= ~=` (`^= $= *=` dengan nilai kosong tak pernah cocok); `class` per token; hasil dalam urutan dokumen; pencocokan relatif terhadap subpohon.
- **`extract_meta`**: title = teks (termasuk komentar) anak `<title>` pertama, `null` bila kosong; description dari `meta[name=description]` (tak peka huruf) — **yang terakhir menang**; `og` dari `meta[property^=og:]` (peka huruf) tanpa prefiks, urutan dokumen.
- **`extract_tables`**: baris dari `tr` turunan tabel tanpa masuk tabel bersarang; sel = teks turunan dengan spasi dipadatkan; `colspan`≤1000 mengulang teks, `rowspan`≤65534 membawa teks ke baris berikut; nilai tak valid/0 → 1; total sel ≤1.000.000.

## 3. Batasan yang Didefinisikan secara Eksplisit (Explicitly Defined Edge Cases)

- Masukan kosong → dokumen tanpa anak.
- `<p>text<un` (tag tak tertutup di akhir) → tidak panic.
- `<a title="x>y">` → satu tag dengan `title="x>y"`.
- `<a href="&#106;avascript:…">` → `href` dibuang oleh sanitizer.
- `<td colspan=4000000000>` → 1000 sel.
- `<div>` × 200.000 → tidak meluap.

## 4. Non-Goals (di luar kontrak normatif ini)

Kesesuaian WHATWG penuh, eksekusi JavaScript, CSS layout, jaminan keamanan sanitizer untuk HTML tak tepercaya (sampai allowlist).

## 5. Riwayat Perubahan Kontrak

| Versi SPEC | Perubahan | Alasan |
|---|---|---|
| 0.1.0 | Kontrak awal | Rilis pertama |
