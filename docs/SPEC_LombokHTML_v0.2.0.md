# LombokHTML — SPEC v0.2.0

This document is the normative cross-language contract. Every language port MUST produce byte-identical output for all specified inputs. Deviations from this specification are bugs.

| Atribut | Nilai |
|---|---|
| Versi SPEC | 0.2.0 (berlaku untuk paket `lombokhtml` 0.2.x di crates.io, npm, PyPI, Packagist, dan modul Go) |
| Acuan | WHATWG HTML Living Standard bagian 13.2.5 (tokenization) dan 13.5 (character references), tinjauan 2026-10-03; html5lib-tests commit `224991ec10db04f056a89eed8b0bd8695fd2950e`; CSS Selectors Level 4 (subset, §8); RFC 3986 (skema URL) |
| Data | `data/entities.json`: 2231 named character reference WHATWG (dengan dan tanpa `;`); setiap port menyematkan tabel yang dibangkitkan `scripts/gen_ports.py` (CI menolak tabel basi) |
| Vector | `vectors/lombokhtml-vectors-v1.json` — 228 kasus (sanitize 46, query 44, decode 30, parse 30, tokenize 25, safeUrl 16, extractText 12, tables 11, textContent 4, meta 4, escapeText 3, escapeAttr 3) — SHA-256 `67b29c0a671d264859cacf30b5637dcc84e4bea6f72e469de64a6ec7a6c2e03a` |
| Conformance | `conformance/html5lib-tokenizer.json` — 7028 kasus tokenizer dari html5lib-tests (MIT, `conformance/LICENSE-html5lib-tests`), diubah ke bentuk token §2.4; 4 tes yang memuat surrogate tunggal dilewati karena tidak dapat diwakili di Rust dan Go |
| Pemeriksa referensi | model Python independen `vectors/htmlmodel.py` (tidak berbagi kode dengan port); `vectors/check_conformance.py` menjalankannya terhadap 7028 kasus (0 gagal); ekspektasi tulis tangan di `vectors/build_vectors.py` divalidasi terhadap model |
| Port | Rust, TypeScript, Python, Go, PHP — kelimanya menjalankan seluruh vector dan seluruh kasus conformance |
| Tanggal tinjauan | 2026-10-03 |

Kata MUST, MUST NOT, SHOULD, MAY mengikuti RFC 2119.

## 0. Konvensi

1. Masukan adalah teks Unicode; posisi dan panjang dihitung dalam code point. Port yang menerima byte (Go, PHP) MUST mengganti setiap byte UTF-8 yang tidak sah dengan U+FFFD sebelum memproses. Surrogate tunggal di luar kontrak.
2. Sebelum tokenisasi, CR LF dan CR tunggal diganti LF.
3. Semua perbandingan tanpa membedakan huruf besar-kecil memakai ASCII saja (A-Z ke a-z). Spasi ASCII adalah TAB, LF, FF, CR, SPACE; di dalam tokenizer CR sudah tidak ada (butir 2).
4. Tidak ada operasi yang gagal karena HTML yang tidak sah; satu-satunya error adalah `BAD_SELECTOR` (§8).

## 1. Data

`data/entities.json` memuat pasangan nama (tanpa `&`, dengan atau tanpa `;` sesuai daftar WHATWG) dan teks penggantinya. Panjang nama terpanjang, termasuk `;`, adalah 32 karakter (`MAX_ENTITY`).

## 2. Tokenizer

### 2.1 Mesin state

Tokenizer MUST mengikuti WHATWG 13.2.5 untuk konten HTML, dengan pengecualian berikut:

1. Tidak ada foreign content: `<![CDATA[` di luar state CDATA masuk ke bogus comment.
2. Processing instruction tidak dikenali; `<?` masuk ke bogus comment.
3. Parse error tidak dilaporkan.
4. Atribut duplikat dibuang; yang pertama dipertahankan. Nama tag dan atribut di-lowercase ASCII.
5. Karakter yang berdampingan digabung menjadi satu token Character.
6. Character reference di atribut mengikuti aturan historis WHATWG: referensi bernama tanpa `;` yang diikuti `=` atau alfanumerik ASCII dibiarkan apa adanya.
7. Referensi numerik: 0, di atas U+10FFFF, dan surrogate menjadi U+FFFD; 0x80-0x9F memakai tabel pengganti Windows-1252 WHATWG; nilai di atas 0x110000 dijenuhkan, tidak meluap.

State awal yang dapat dipilih: `data`, `rcdata`, `rawtext`, `script` (script data), `plaintext`, `cdata` (CDATA section). `lastStartTag` menentukan end tag yang "appropriate" di RCDATA, RAWTEXT dan script data.

### 2.2 Perpindahan state menurut elemen

Fungsi `tokenize(html)` dan parser (§4) mulai di state `data` dan, setelah memancarkan start tag, berpindah state:

| Start tag | State berikutnya |
|---|---|
| `title`, `textarea` | `rcdata` |
| `style`, `xmp`, `iframe`, `noembed`, `noframes`, `noscript` | `rawtext` |
| `script` | `script` |
| `plaintext` | `plaintext` |

`noscript` diperlakukan seperti browser dengan scripting aktif. Fungsi `tokenizeState(html, state, lastStartTag)` tidak melakukan perpindahan ini (dipakai oleh conformance).

### 2.3 Batas

Tokenizer SHOULD berjalan linear terhadap panjang masukan, termasuk teks, atribut, komentar dan nama tag yang sangat panjang.

### 2.4 Bentuk token (vector dan conformance)

| Token | Bentuk JSON |
|---|---|
| start tag | `["StartTag", name, [[attr, value], ...], selfClosing]` |
| end tag | `["EndTag", name]` |
| karakter | `["Character", data]` |
| komentar | `["Comment", data]` |
| DOCTYPE | `["DOCTYPE", name, publicId, systemId, correct]` — nilai yang hilang `null`; `correct` adalah kebalikan force-quirks |

## 3. Character reference dan escaping

### 3.1 decode_entities

Mendekode teks seperti isi teks biasa: `&#` diikuti digit desimal, atau `&#x`/`&#X` diikuti digit heksadesimal, menjadi karakter menurut §2.1 butir 7 (`;` penutup opsional); tanpa digit, teks dibiarkan. `&` diikuti alfanumerik ASCII mencoba nama terpanjang (maksimum 32 karakter) dengan `;` lebih dulu, lalu awalan terpanjang yang terdaftar; tanpa kecocokan, teks dibiarkan. Fungsi ini tidak memakai aturan atribut (§2.1 butir 6).

### 3.2 Escaping

| Fungsi | Diganti |
|---|---|
| `escape_text` | `&` ke `&amp;`, U+00A0 ke `&nbsp;`, `<` ke `&lt;`, `>` ke `&gt;` |
| `escape_attr` | seperti `escape_text`, ditambah `"` ke `&quot;` |

Urutan penggantian tidak memengaruhi hasil (tidak ada keluaran yang dipindai ulang).

## 4. Tree builder

Pohon terdiri dari node document, element (nama, atribut berurutan), text, dan comment. DOCTYPE diabaikan. Tidak ada elemen yang disisipkan secara implisit (`html`, `head`, `body`, `tbody` tidak dibuat bila tidak tertulis).

Istilah: *stack* adalah daftar elemen terbuka; *in scope(N, B)* benar bila, dari atas stack ke bawah, elemen bernama di N ditemukan sebelum elemen bernama di B; *pop until N* mengeluarkan elemen dari stack sampai dan termasuk elemen pertama yang bernama di N.

| Himpunan | Isi |
|---|---|
| VOID | area base basefont bgsound br col embed frame hr img input keygen link meta param source track wbr |
| SPECIAL | himpunan "special" WHATWG untuk HTML: address applet area article aside base basefont bgsound blockquote body br button caption center col colgroup dd details dir div dl dt embed fieldset figcaption figure footer form frame frameset h1-h6 head header hgroup hr html iframe img input keygen li link listing main marquee menu meta nav noembed noframes noscript object ol p param plaintext pre script search section select source style summary table tbody td template textarea tfoot th thead title tr track ul wbr xmp |
| P_CLOSERS | address article aside blockquote center details dialog dir div dl fieldset figcaption figure footer form h1-h6 header hgroup hr li dd dt listing main menu nav ol p plaintext pre search section summary table ul xmp |
| SCOPE | applet caption html table td th marquee object template |
| BUTTON_SCOPE / LIST_SCOPE | SCOPE + button / SCOPE + ol ul |
| TABLE_SCOPE | html table template |

**Start tag** `name`, berurutan:

1. `li`: dari atas stack, bila ditemukan `li` maka stack dipotong sampai elemen itu (termasuk); berhenti bila bertemu elemen SPECIAL selain address, div, p. `dd`/`dt` sama dengan target `dd` atau `dt`.
2. `name` di P_CLOSERS dan in scope(p, BUTTON_SCOPE): pop until p.
3. `name` heading dan elemen teratas heading: pop satu.
4. `option`/`optgroup` dan elemen teratas `option`: pop satu.
5. `a` dan ada `a` di stack: pop until a.
6. `td th tr thead tbody tfoot` dan in scope(td/th, TABLE_SCOPE): pop until td/th.
7. `tr thead tbody tfoot` dan in scope(tr, TABLE_SCOPE): pop until tr.
8. `thead tbody tfoot` dan in scope(thead/tbody/tfoot, TABLE_SCOPE): pop until salah satunya.
9. Bila stack berisi 256 elemen (`MAX_DEPTH`), start tag diabaikan (beserta flag newline di butir 10).
10. Elemen ditambahkan ke elemen teratas (atau document); bila bukan VOID, didorong ke stack. Setelah `pre`, `listing`, `textarea`, satu LF di awal token Character berikutnya dibuang.

**End tag** `name`:

| name | Aturan |
|---|---|
| `br` | diperlakukan sebagai start tag `br` |
| `p` | bila in scope(p, BUTTON_SCOPE): pop until p; selain itu diabaikan |
| `h1`-`h6` | bila heading apa pun in scope(SCOPE): pop until heading |
| `li` | in scope(li, LIST_SCOPE): pop until li |
| `table caption tbody thead tfoot tr td th` | in scope(name, TABLE_SCOPE): pop until name |
| `dd`, `dt`, nama SPECIAL lain | in scope(name, SCOPE): pop until name |
| lainnya | dari atas stack: elemen bernama sama dipotong (termasuk elemen di atasnya); berhenti tanpa perubahan bila lebih dulu bertemu elemen SPECIAL |

**Teks** ditambahkan ke elemen teratas, digabung dengan node teks sebelumnya bila node anak terakhir adalah teks. **Komentar** ditambahkan sebagai node comment.

## 5. Serialisasi

`serialize(node)`: element menjadi `<name` + untuk setiap atribut ` k="escape_attr(v)"` + `>`; elemen VOID berhenti di sini; selain itu anak-anak lalu `</name>`. Text di bawah `style script xmp iframe noembed noframes plaintext noscript` ditulis apa adanya, selain itu `escape_text`. Comment menjadi `<!--data-->`. Document menulis anak-anaknya.

## 6. Teks

### 6.1 text_content

Gabungan semua node text di bawah node, melewati subpohon `script style noscript template title`.

### 6.2 extract_text

Penelusuran berurutan dengan penanda `pre` (benar di dalam `pre`, `listing`, `textarea`):

| Node | Keluaran |
|---|---|
| text, `pre` benar | data apa adanya |
| text, lainnya | setiap deretan spasi ASCII menjadi satu spasi; spasi awal dibuang bila keluaran kosong atau berakhir dengan spasi, LF, atau TAB |
| `script style noscript template title`, comment | tidak ada |
| `h1`-`h6` | `\n\n` + `#` sebanyak level + spasi, anak-anak, `\n\n` |
| `li` | `\n- ` lalu anak-anak |
| `dd dt tr` | `\n` lalu anak-anak |
| `br` / `hr` | `\n` / `\n\n---\n\n` |
| `td th` | anak-anak lalu TAB |
| blok: address article aside blockquote caption details dialog div dl fieldset figcaption figure footer form header hgroup main nav ol p pre section summary table ul | `\n\n`, anak-anak, `\n\n` |

Pasca-proses: setiap baris dibuang spasi dan TAB di ujung kanannya; tiga LF atau lebih berturut-turut menjadi dua; LF di awal dan akhir dibuang.

## 7. Sanitizer

### 7.1 Policy bawaan

| Daftar | Isi |
|---|---|
| tag dipertahankan | a abbr b blockquote br caption cite code dd del dfn div dl dt em figcaption figure h1-h6 hr i img ins kbd li mark ol p pre q s samp small span strong sub sup table tbody td tfoot th thead time tr u ul |
| tag dibuang beserta isi | applet audio base button canvas embed frame frameset head iframe link math meta noembed noframes noscript object option plaintext script select style svg template textarea title video xmp |
| atribut (tag:atribut, `*` = semua tag yang dipertahankan) | *:dir *:lang *:title a:href blockquote:cite img:alt img:height img:src img:width ol:start q:cite td:colspan td:rowspan th:colspan th:rowspan th:scope time:datetime |
| skema URL | http https mailto tel |

Policy dapat ditambah: `allow_tag` (juga mengeluarkannya dari daftar buang; `plaintext` tidak pernah dipertahankan karena tidak dapat ditutup lagi), `allow_attr(tag, attr)`, `allow_scheme`; semua di-lowercase ASCII.

### 7.2 Algoritme

HTML di-parse (§4), lalu ditulis ulang: text di-`escape_text` (termasuk di dalam elemen raw text), comment dibuang, elemen di daftar buang dibuang beserta isi, elemen lain yang tidak dipertahankan diganti dengan isinya, elemen yang dipertahankan ditulis dengan atribut yang diizinkan saja (urutan asli), dan atribut `href`, `src`, `cite` hanya bila URL aman (§7.3). Elemen raw text yang dipertahankan lewat policy (`style script xmp iframe noembed noframes noscript`) ditulis tanpa isi, karena isinya terbaca mentah pada parse berikutnya. Keluaran MUST idempoten: `sanitize(sanitize(x)) == sanitize(x)` (diperiksa untuk setiap kasus vector).

### 7.3 URL aman

Hapus semua karakter U+0000-U+0020 dan U+007F, lowercase ASCII, lalu cari karakter pertama dari `/ ? # :`. Bila tidak ada atau bukan `:`, URL relatif dan aman. Bila `:`, aman hanya bila teks sebelumnya ada di daftar skema.

## 8. Selector

### 8.1 Tata bahasa

```
list      = ws complex ws ( "," ws complex ws )*
complex   = compound ( ( ws? [>+~] ws? | ws ) compound )*
compound  = ( "*" | ident )? ( "#" ident | "." ident | attr | pseudo )*      ; tidak boleh kosong
attr      = "[" ws ident ws ( op ws ( ident | '"' .*? '"' | "'" .*? "'" ) ws )? "]"
op        = "=" | "~=" | "|=" | "^=" | "$=" | "*="
pseudo    = ":" ( "first-child" | "last-child" | "only-child" | "empty"
                | "nth-child(" ws nth ")" | "nth-last-child(" ws nth ")" | "not(" ws compound ws ")" )
ident     = ( [A-Za-z0-9_-] | karakter non-ASCII )+
ws        = [ \t\n\r\f]*
nth       = odd | even | [+-]?[0-9]{1,9} | [+-]?[0-9]{0,9} "n" ( ws [+-] ws [0-9]{1,9} )?   ; setelah trim spasi, lowercase ASCII
```

Nama tag, nama atribut dan nama pseudo di-lowercase ASCII; nilai id, class dan atribut peka huruf. Selector lain (termasuk escape CSS, namespace, pseudo lain) MUST ditolak dengan error `BAD_SELECTOR` dan offset karakter tempat parser berhenti.

### 8.2 Pencocokan

`query(doc, selector)` mengembalikan elemen yang cocok dengan salah satu complex selector, dalam urutan dokumen tanpa duplikat. Kombinator: spasi (leluhur elemen mana pun), `>` (induk elemen), `+` (saudara elemen tepat sebelumnya), `~` (saudara elemen sebelumnya mana pun). `#x` membandingkan atribut `id`; `.x` mencari token di `class` yang dipisah spasi ASCII; `~=` sama untuk nilai atribut; `|=` sama atau berawalan `nilai-`; `^=`, `$=`, `*=` tidak pernah cocok dengan nilai kosong. Posisi dihitung di antara saudara elemen (1-based); `An+B` cocok bila posisi = A*k + B untuk bilangan bulat k >= 0. `:empty` cocok bila tidak ada anak element atau text. `:not(compound)` cocok bila compound tidak cocok.

Pencocokan SHOULD linear terhadap jumlah elemen kali jumlah langkah selector (cache per query untuk posisi saudara dan hasil per langkah); hasil tidak bergantung pada cache.

## 9. Meta

Dalam urutan dokumen: `title` = teks langsung `title` pertama, spasi diciutkan dan di-trim, `null` bila kosong; `lang` = atribut `lang` elemen `html` pertama yang memilikinya; `description` = `content` `meta` pertama yang `name`-nya (lowercase ASCII) `description`; `canonical` = `href` `link` pertama yang `rel`-nya (token lowercase ASCII) memuat `canonical` dan memiliki `href`; `og` = untuk setiap `meta` dengan `content` dan `property` berawalan `og:` (ASCII tanpa beda huruf), pasangan [property tanpa 3 karakter pertama, content].

## 10. Tabel

Untuk setiap `table` dalam urutan dokumen: baris adalah `tr` di bawahnya yang tidak berada di dalam `table` bersarang. Setiap sel (`td`/`th` anak langsung `tr`) bernilai `text_content` dengan spasi diciutkan dan di-trim. `colspan` (maksimum 1000) dan `rowspan` (maksimum 65534) dibaca sebagai bilangan bulat di awal nilai (spasi ASCII dan `+` boleh di depan); tidak ada, tidak sah, atau 0 menjadi 1; lebih dari 9 digit menjadi maksimum. Sel ditempatkan pada kolom kosong pertama, menempati `colspan` kolom dan `rowspan` baris. Lebar setiap baris adalah kolom terisi terbesar + 1; celah diisi `""`. Bila jumlah sel yang ditempatkan dalam satu tabel melewati 1.000.000 (`MAX_CELLS`), penempatan berhenti; baris yang sedang diproses tetap dikeluarkan.

## 11. Keamanan (normatif)

1. Tidak ada operasi yang mengeksekusi, mengambil sumber daya, atau membaca berkas.
2. Kedalaman pohon dibatasi 256; sel tabel 1.000.000; span 1000 x 65534. Rekursi dalam port dibatasi kedalaman pohon.
3. Sanitizer bersifat allowlist dan idempoten. Sanitizer TIDAK menjamin kesetaraan dengan parser browser untuk semua masukan (tree builder adalah subset, §12); untuk masukan tak tepercaya tetap pasang Content-Security-Policy.
4. Port MUST NOT memakai kode `unsafe` (Rust `forbid(unsafe_code)`) dan MUST NOT memakai ekstensi native di luar pustaka standar.

## 12. Perbedaan dengan browser yang diketahui

1. Tree builder adalah subset: tidak ada insertion mode, adoption agency, foster parenting, elemen implisit `html/head/body/tbody`, `template` content, foreign content (SVG/MathML), atau penanganan form pointer.
2. `noscript` selalu diperlakukan sebagai raw text (scripting aktif).
3. Tokenizer lulus 7028 kasus html5lib-tests pada commit yang dipin; tes processing instruction yang ditambahkan setelah commit itu tidak berlaku.

## 13. Perubahan dari 0.1.0

0.1.0 memakai tokenizer subset buatan sendiri dan ekspektasi vector yang dibangkitkan port Rust. 0.2.0: tokenizer WHATWG lengkap untuk konten HTML, 2231 entitas bernama, tree builder dengan aturan WHATWG di atas, serialisasi WHATWG, sanitizer dengan daftar buang dan atribut per tag, selector dengan `+ ~ |=` dan pseudo-class, meta `lang`/`canonical`, tabel dengan batas span, serta vector dari model independen dan conformance html5lib-tests.

*Lisensi dokumen: Apache-2.0 · © codinglombok*
