# LombokHTML — MAP v0.1.0

Peta relasi `LombokHTML` di dalam Lombok Ecosystem — untuk peta lengkap 88 repo lihat MAP_UTAMA_v3.3.md.

## 1. Posisi Dependensi

```
LombokHTML (L0)
   (tidak bergantung pada apa pun)
        ▲
        │ (rencana)
LombokTemplates · LombokPDF · LombokChunker
```

## 2. Contoh Dependen di Ekosistem (arah dependensi; bukan kepemilikan)

Daftar ini mencatat siapa yang *dapat* memakai `LombokHTML` (aplikasi → library). Library tetap mandiri dan tidak diklaim sebagai bagian dari salah satunya.

| Repo | Tingkat | Status |
|---|---|---|
| LombokTemplates | L1 | ⚪ rencana |
| LombokPDF | A | ⚪ rencana |
| LombokChunker | L1 | ⚪ rencana |

## 3. `LombokHTML` Bergantung pada

Tidak ada.

## 4. Jalur Kontrak Normatif

`LombokHTML` → `SPEC_LombokHTML_v0.1.0.md` → `vectors/lombokhtml-vectors-v1.json` → dijalankan oleh **LombokTest** di setiap port bahasa (lihat [SPEC](SPEC_LombokHTML_v0.1.0.md)).

## 5. Peta Folder Lintas Cluster

| Cluster | Path di monorepo katalog |
|---|---|
| 03 · Format, Parser & Serialisasi (03.03) | `ecosystem/03_Format-Parser-Serialisasi/03_LombokHTML/LombokHTML/` |
