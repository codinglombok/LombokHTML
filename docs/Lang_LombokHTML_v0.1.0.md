# LombokHTML — LANG (i18n) v0.1.0

Tingkat i18n, katalog ID pesan, dan cakupan bahasa untuk `LombokHTML` (ARCHITECTURE_UTAMA §7, ADR-009: "Library L1+ → hanya menyimpan ID pesan + teks en; terjemahan di-load via LombokLocale").

## 1. Tingkat i18n Repo Ini

**Tingkat E** menurut MASTERPLAN_UTAMA §13 (pesan error & dokumentasi saja). Saat ini tidak ada pesan error karena seluruh fungsi total (tidak mengembalikan error), sehingga tidak ada `locales/` dan tidak bergantung pada LombokLocale. Bila kelak ada pelaporan diagnostik, pesan itu wajib memakai katalog (ADR-009) dan dokumen ini diperbarui. Dokumentasi repo ini masih berbahasa Indonesia + README Inggris (belum Core-20).

## 2. Katalog ID Pesan

| `messageId` | Teks EN (default) | Digunakan di |
|---|---|---|
| — | — | (tidak ada) |

## 3. Cakupan Bahasa (Core-20 + Nusantara)

| Bahasa | Kode BCP-47 | Status katalog |
|---|---|---|
| Bahasa Indonesia | `id` | — (tidak ada pesan) |
| English | `en` | — (tidak ada pesan) |


## 4. Cara Menambah Bahasa Baru

1. Salin `locales/en/lombokhtml.json` ke `locales/<bcp47>/lombokhtml.json`.
2. Terjemahkan setiap value; jangan ubah `messageId` (key).
3. Jalankan test integrasi katalog (`cargo test catalog`) — memverifikasi setiap `messageId` di §2 punya entri di file baru.
4. Perbarui tabel §3 di dokumen ini pada PR yang sama.

## 5. Ketergantungan pada LombokLocale

Tidak bergantung pada LombokLocale (L0 murni).
