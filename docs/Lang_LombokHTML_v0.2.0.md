# LombokHTML — Bahasa & i18n v0.2.0

| Item | Nilai |
|---|---|
| Teks pengguna | tidak ada; library tidak menghasilkan pesan untuk pengguna akhir |
| Kode error | `BAD_SELECTOR` (sama di semua port), dengan offset karakter |
| Pesan error | bahasa Inggris, satu baris (`BAD_SELECTOR at character N`), untuk pengembang |
| Katalog `locales/` | tidak diperlukan |

## 1. Prinsip

1. Keluaran adalah data (token, pohon, teks, HTML), bukan pesan; tidak ada yang perlu diterjemahkan.
2. Pemakai yang ingin menampilkan error selector kepada pengguna akhir memetakan `BAD_SELECTOR` ke katalog mereka sendiri (mis. lewat LombokLocale).
3. Teks dalam bahasa apa pun diproses sebagai Unicode; perbandingan tanpa beda huruf memakai ASCII saja (SPEC §0), sehingga hasil tidak bergantung pada locale sistem.

## 2. Rencana

Tidak ada pekerjaan i18n yang direncanakan untuk 0.x.

*Lisensi dokumen: Apache-2.0 · © codinglombok*
