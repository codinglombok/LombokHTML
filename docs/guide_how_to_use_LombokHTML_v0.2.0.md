# LombokHTML — Guide How to Use v0.2.0

## 1. Pemasangan

| Bahasa | Perintah |
|---|---|
| Rust | `cargo add lombokhtml` (atau `--no-default-features` untuk `no_std` + `alloc`) |
| TypeScript | `npm install lombokhtml` (ESM, Node 20+) |
| Python | `pip install lombokhtml` (3.9+) |
| Go | `go get github.com/codinglombok/lombokhtml/go` |
| PHP | `composer require codinglombok/lombokhtml` (8.1+, tanpa ekstensi) |

## 2. Mengambil teks

```python
import lombokhtml as h

h.strip_tags("<p>Hi <b>there</b><script>x()</script></p>")      # "Hi there"
h.html_to_text("<h2>Menu</h2><ul><li>Nasi<li>Sate</ul><p>Total</p>")
# "## Menu\n\n- Nasi\n- Sate\n\nTotal"
```

`strip_tags` menggabungkan teks apa adanya; `html_to_text` mempertahankan struktur (heading `#`, daftar `- `, sel tabel dipisah TAB, `pre` utuh) dan cocok untuk indeks pencarian atau masukan model bahasa.

## 3. Membersihkan HTML dari pengguna

```ts
import { sanitize, Policy } from 'lombokhtml';

sanitize('<a href="javascript:alert(1)" onclick="x()">klik</a>');   // '<a>klik</a>'
const p = new Policy().allowTag('iframe').allowAttr('iframe', 'src').allowScheme('ftp');
sanitize('<iframe src="https://video.example/1"></iframe>', p);       // '<iframe src="https://video.example/1"></iframe>'
```

Tag di luar allowlist dibuka (isinya dipertahankan), tag berbahaya dibuang beserta isinya, atribut dan skema URL disaring. Hasilnya stabil bila disanitasi ulang. Untuk HTML yang benar-benar tak tepercaya tetap pasang Content-Security-Policy (SPEC §11).

## 4. Mencari elemen

```go
doc := lombokhtml.Parse(page)
items, err := doc.Query("ul.menu > li:not(.hidden) a[href^='https']")
if err != nil { /* *lombokhtml.SelectorError, Code() == "BAD_SELECTOR" */ }
for _, a := range items { href, _ := a.Attr("href"); fmt.Println(href, a.TextContent()) }
```

## 5. Metadata dan tabel

```php
use LombokHTML\Html;

$doc = Html::parse($page);
$doc->meta();    // ['title' => ..., 'description' => ..., 'canonical' => ..., 'lang' => ..., 'og' => [['title', '...'], ...]]
$doc->tables();  // [[['Wilayah', 'Penjualan'], ['Barat', '120']]] dengan colspan/rowspan diratakan
```

## 6. Token

```rust
use lombokhtml::{tokenize, Token};

for tok in tokenize("<a href=x>&copy;</a>") {
    if let Token::StartTag { name, attrs, .. } = tok { println!("{name} {attrs:?}"); }
}
```

## 7. Skenario netral

| Kebutuhan | Fungsi |
|---|---|
| Teks bersih untuk indeks pencarian | `html_to_text` |
| Komentar pengguna di forum | `sanitize` dengan policy bawaan |
| Pratinjau tautan | `meta()` |
| Mengimpor tabel dari halaman web | `tables()` |
| Pengujian HTML hasil template | `parse` + `query` + `serialize` |
| Perangkat kecil tanpa alokator besar | crate Rust `no_std` + `alloc` |

## 8. Pemecahan masalah

| Gejala | Penyebab | Solusi |
|---|---|---|
| `BAD_SELECTOR` | selector di luar subset SPEC §8 (mis. `:hover`, `::before`, escape CSS) | sederhanakan selector |
| elemen tidak muncul di `tbody` | tidak ada `tbody` implisit | tulis selector tanpa `tbody`, atau `table tr` |
| isi `iframe` hilang setelah sanitize | elemen raw text yang diizinkan ditulis tanpa isi (SPEC §7.2) | perilaku yang disengaja |
| teks setelah 256 elemen bersarang hilang strukturnya | batas kedalaman | perilaku yang disengaja (SPEC §11) |

*Lisensi dokumen: Apache-2.0 · © codinglombok*
