# LombokHTML — API REFERENCE v0.1.0

Seluruh API publik, per bahasa, dengan kode error dan status stabilitas (ARCHITECTURE_UTAMA §5, baris "Dokumen" — normatif sejak MASTERPLAN_UTAMA v3.3 §1.1/§5).

Legenda stabilitas: 🟢 **Stable** (breaking change butuh major bump + RFC) · 🟡 **Beta** (boleh berubah, diumumkan di changelog) · 🔴 **Experimental** (bisa hilang tanpa notice).

## Rust (referensi)

| Simbol | Tanda tangan | Stabilitas |
|---|---|---|
| `tokenize` | `(&str) -> Vec<Token>` | 🟡 |
| `parse` | `(&str) -> Node` | 🟢 |
| `Node` | `kind, tag, attrs, text, children; attr(), is_element(), find_first(), find_all(), walk()` | 🟢 |
| `decode_entities` / `escape_text` / `escape_attr` | `(&str) -> String` | 🟢 |
| `strip_tags` | `(&str) -> String` | 🟢 |
| `extract_structured_text` | `(&str) -> String` | 🟡 |
| `sanitize` / `sanitize_with` | `(&str) -> String` / `(&str, &Policy) -> String` | 🟡 |
| `Policy` | `default(); allow_tag(); allow_attr(); allow_scheme()` | 🟡 |
| `is_safe_url` | `(&str, &[String] schemes) -> bool` | 🟡 |
| `parse_selector` / `parse_selector_list` / `query` | `(&str) -> Selector` / `-> Vec<Complex>` / `(&Node, &str) -> Vec<&Node>` | 🟡 |
| `extract_meta` | `(&Node) -> PageMeta {title, description, og_tags}` | 🟢 |
| `extract_tables` | `(&Node) -> Vec<Vec<Vec<String>>>` | 🟢 |
| `is_void_element` | `(&str) -> bool` | 🟢 |

### Kode Error

Tidak ada — semua fungsi total (tidak mengembalikan error; masukan cacat ditangani secara toleran).

## Port Lain

**TypeScript** (`typescript/src/index.ts`): padanan camelCase (`parse`, `stripTags`, `extractStructuredText`, `sanitize`/`sanitizeWith`/`defaultPolicy`, `isSafeUrl`, `decodeEntities`, `query`, `parseSelectorList`, `extractMeta`, `extractTables`, kelas `Node`). Lulus seluruh vektor.

Python/Go/PHP: belum ada.

## Kompatibilitas Lintas Bahasa

Setiap fungsi di atas **harus** menghasilkan output byte-identik lintas port untuk input yang sama, dibuktikan oleh `vectors/lombokhtml-vectors-v1.json` (lihat [SPEC](SPEC_LombokHTML_v0.1.0.md)). Perbedaan hasil antar-port adalah bug, bukan variasi yang sah (ADR-015).
