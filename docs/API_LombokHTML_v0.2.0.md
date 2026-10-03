# LombokHTML — API v0.2.0

Perilaku normatif ada di SPEC. Dokumen ini memetakan konsep SPEC ke nama di setiap port.

## 1. Ringkasan lintas port

| Konsep (SPEC) | Rust | TypeScript | Python | Go | PHP |
|---|---|---|---|---|---|
| tokenize (§2.2) | `tokenize(html) -> Vec<Token>` | `tokenize(html): Token[]` | `tokenize(html) -> list[tuple]` | `Tokenize(html) []Token` | `Html::tokenize($html): array` |
| tokenize dari state (§2.1) | `tokenize_state(html, State::Rcdata, Some("title"))` | `tokenizeState(html, 'rcdata', 'title')` | `tokenize_state(html, "rcdata", "title")` | `TokenizeState(html, "rcdata", &last)` | `Html::tokenizeState($html, 'rcdata', 'title')` |
| decode (§3.1) | `decode_entities(s)` | `decodeEntities(s)` | `decode_entities(s)` | `DecodeEntities(s)` | `Html::decodeEntities($s)` |
| escape (§3.2) | `escape_text`, `escape_attr` | `escapeText`, `escapeAttr` | `escape_text`, `escape_attr` | `EscapeText`, `EscapeAttr` | `Html::escapeText`, `Html::escapeAttr` |
| parse (§4) | `parse(html) -> Document` | `parse(html): HtmlNode` | `parse(html) -> Node` | `Parse(html) *Node` | `Html::parse($html): Node` |
| serialisasi (§5) | `doc.serialize(id)` | `node.serialize()` | `node.serialize()` | `n.Serialize()` | `$node->serialize()` |
| text_content (§6.1) | `doc.text_content(id)`, `strip_tags(html)` | `node.textContent()`, `stripTags(html)` | `node.text_content()`, `strip_tags(html)` | `n.TextContent()`, `StripTags(html)` | `$node->textContent()`, `Html::stripTags($html)` |
| extract_text (§6.2) | `doc.extract_text(id)`, `extract_text(html)` | `node.extractText()`, `htmlToText(html)` | `node.extract_text()`, `html_to_text(html)` | `n.ExtractText()`, `HTMLToText(html)` | `$node->extractText()`, `Html::htmlToText($html)` |
| sanitize (§7) | `sanitize(html)`, `sanitize_with(html, &Policy)` | `sanitize(html, policy?)` | `sanitize(html, policy=None)` | `Sanitize(html, *Policy)` (nil = bawaan) | `Html::sanitize($html, ?Policy)` |
| policy | `Policy::new().allow_tag(t).allow_attr(t, a).allow_scheme(s)` | `new Policy().allowTag(t).allowAttr(t, a).allowScheme(s)` | `Policy().allow_tag(t)...` | `NewPolicy().AllowTag(t)...` | `(new Policy())->allowTag($t)...` |
| URL aman (§7.3) | `is_safe_url(u)`, `is_safe_url_with(u, &schemes)` | `isSafeUrl(u, schemes?)` | `is_safe_url(u, schemes=DEFAULT_SCHEMES)` | `IsSafeURL(u, schemes...)` | `Html::isSafeUrl($u, $schemes)` |
| query (§8) | `doc.query(sel) -> Result<Vec<NodeId>, SelectorError>` | `node.query(sel): HtmlNode[]` (throw) | `node.query(sel) -> list[Node]` (raise) | `n.Query(sel) ([]*Node, error)` | `$node->query($sel): array` (throw) |
| selector terkompilasi | `Selector::parse(s)?.matches(&doc, id)` | `Selector.parse(s).matches(el)` | `Selector(s).matches(el)` | `CompileSelector(s)` + `Matches(el)` | `Selector::parse($s)->matches($el)` |
| error selector | `SelectorError { offset }`, `code()` | `SelectorError` (`code`, `offset`) | `SelectorError` (`code`, `offset`; turunan `ValueError`) | `*SelectorError{Offset}`, `Code()` | `SelectorError` (`errorCode`, `offset`) |
| meta (§9) | `doc.meta() -> PageMeta` | `node.meta(): PageMeta` | `node.meta() -> PageMeta` (+ `to_dict()`) | `n.Meta() PageMeta` (pointer nil = tidak ada) | `$node->meta(): array` |
| tabel (§10) | `doc.tables() -> Vec<Table>` | `node.tables(): string[][][]` | `node.tables()` | `n.Tables() [][][]string` | `$node->tables(): array` |
| batas | `MAX_DEPTH`, `MAX_CELLS` | `MAX_DEPTH`, `MAX_CELLS` | `MAX_DEPTH`, `MAX_CELLS` | `MaxDepth`, `MaxCells` | `Html::MAX_DEPTH`, `Html::MAX_CELLS` |

## 2. Model pohon per port

| Port | Bentuk |
|---|---|
| Rust | arena: `Document` berisi `Node` yang diacu `NodeId` (indeks); `doc.root()`, `doc.node(id)`, `doc.elements()`, `doc.descendants(id)`; `Node { kind, name, attrs, data, parent, children }`, `attr(name)`, `is(name)` |
| TypeScript | objek `HtmlNode` (`kind`, `name`, `attrs`, `data`, `parent`, `children`), `attr`, `is`, `elements`, `elementSiblings` |
| Python | `Node` (`__slots__`), `attr`, `is_element`, `elements()` (generator), `element_siblings` |
| Go | `*Node` (`Kind`, `Name`, `Attrs []Attr`, `Data`, `Parent`, `Children`), `Attr(name) (string, bool)`, `Is`, `Elements` |
| PHP | `Node` (`kind`, `name`, `attrs`, `data`, `parent`, `children`), `attr`, `is`, `elements`, `elementSiblings` |

Token: Rust `enum Token`, TypeScript union `{ type: ... }`, Python tuple, Go `Token{Kind, ...}`, PHP array berbentuk JSON §2.4.

## 3. Kompatibilitas dengan 0.1.0

0.1.0 tidak terbit. Yang berubah di Rust: modul publik digabung ke akar crate; `query(&doc, sel)` menjadi `doc.query(sel)` dan mengembalikan `Result`; `extract_structured_text` menjadi `extract_text`; `extract_meta`/`extract_tables` menjadi `doc.meta()`/`doc.tables()`; `PageMeta.og_tags` menjadi `og`; `is_void_element`, `parse_selector`, `parse_selector_list` dihapus (diganti `Selector::parse`). Modul `compat` TypeScript dihapus.

*Lisensi dokumen: Apache-2.0 · © codinglombok*
