# LombokHTML — Map v0.2.0

## 1. Posisi di ekosistem

```
Cluster 03 Format, Parser & Serialisasi · tingkat L0 (tanpa dependensi Lombok wajib)

L0  LombokHTML
     dependensi wajib    : (tidak ada)
     dependensi opsional : (tidak ada)
     data tertanam       : 2231 named character reference WHATWG
     dependensi dev      : serde_json (runner vector Rust), typescript + @types/node (TS), pytest + coverage (Python, CI)
```

## 2. Contoh pemakai di ekosistem

Library ini mandiri dan dapat dipakai siapa pun. Library dan aplikasi Lombok yang dapat memakainya (arah dependensi selalu pemakai ke library):

| Pemakai | Pemakaian |
|---|---|
| LombokMarkDown | sanitasi HTML hasil render dan HTML mentah di dalam Markdown |
| LombokDocFlow | ekstraksi teks dan tabel dari dokumen HTML |
| LombokMiner | `extract_text`, `meta`, `tables` untuk pengindeksan halaman |
| LombokCSS | pemilih elemen dengan subset selector yang sama |

## 3. Peta fitur x port

| Fitur | Rust | TypeScript | Python | Go | PHP |
|---|---|---|---|---|---|
| tokenizer WHATWG (SPEC §2) | YA | YA | YA | YA | YA |
| conformance html5lib-tests (7028) | YA | YA | YA | YA | YA |
| entitas dan escaping (§3) | YA | YA | YA | YA | YA |
| tree builder dan serialisasi (§4-5) | YA | YA | YA | YA | YA |
| text_content, extract_text (§6) | YA | YA | YA | YA | YA |
| sanitizer (§7) | YA | YA | YA | YA | YA |
| selector (§8) | YA | YA | YA | YA | YA |
| meta, tabel (§9-10) | YA | YA | YA | YA | YA |
| `no_std` | YA | - | - | - | - |
| vector | 228/228 | 228/228 | 228/228 | 228/228 | 228/228 |

*Lisensi dokumen: Apache-2.0 · © codinglombok*
