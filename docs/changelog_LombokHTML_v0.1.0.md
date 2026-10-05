# LombokHTML — CHANGELOG

Format mengikuti [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) dan [SemVer](https://semver.org/). Versi ini dirilis lewat `release-please` (ADR-011) — commit `feat:`/`fix:` men-trigger bump otomatis; jangan edit versi manual di sini.

*Entri terbaru selalu di bagian paling depan (PRINSIP_UNIVERSAL U12).*

## [0.1.0] — 2026-09-28

### Added
- Port TypeScript (lulus 3.099 vektor) + uji mutasi
- **Sanitizer allowlist** (`Policy`), dekode/escape entitas, combinator selector, `rowspan`
- Batas kedalaman DOM, pseudo-fuzz, verifikasi keluaran sanitizer dengan parser independen

### Changed
- `sanitize` kini allowlist (bukan denylist); `class/id/style` dibuang secara bawaan

### Fixed
- Tokenizer mengakhiri tag pada `>` di dalam atribut berkutip
- `colspan` tak terbatas (DoS alokasi); baris tabel bersarang bocor ke tabel luar
- Serializer tidak meng-escape `"` (atribut) dan `<` (teks)
- `&#+65;` di-decode (perilaku `from_str_radix`), kini ditolak sesuai spesifikasi

### Security
- Sanitizer allowlist; pemeriksaan URL gaya-browser; batas kedalaman & ukuran tabel

---

*Untuk riwayat lengkap tiap versi, lihat [GitHub Releases](https://github.com/codinglombok/LombokHTML/releases).*
