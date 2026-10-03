// Package lombokhtml is a WHATWG HTML tokenizer with a small tree builder,
// serialization, text extraction, an allowlist sanitizer, a CSS selector
// subset, page metadata and table extraction. Results are byte-identical with
// the Rust, TypeScript, Python and PHP ports (docs/SPEC_LombokHTML_v0.2.0.md).
// No dependencies outside the standard library.
package lombokhtml

import (
	"strconv"
	"strings"
)

// ---------------------------------------------------------------- entities

// DecodeEntities decodes character references as in text content (SPEC section 3.1).
func DecodeEntities(text string) string {
	s := []rune(text)
	n := len(s)
	var out strings.Builder
	at := func(i int) rune {
		if i < n {
			return s[i]
		}
		return eof
	}
	for i := 0; i < n; {
		if s[i] != '&' {
			out.WriteRune(s[i])
			i++
			continue
		}
		j := i + 1
		if at(j) == '#' {
			k := j + 1
			hex := at(k) == 'x' || at(k) == 'X'
			if hex {
				k++
			}
			start, code := k, 0
			for k < n && ((hex && isHex(s[k])) || (!hex && isDigit(s[k]))) {
				base := 10
				if hex {
					base = 16
				}
				code = code*base + hexValue(s[k])
				if code > 0x110000 {
					code = 0x110000
				}
				k++
			}
			if k == start {
				out.WriteString(string(s[i:k]))
				i = k
				continue
			}
			if at(k) == ';' {
				k++
			}
			out.WriteString(numericChar(code))
			i = k
			continue
		}
		k := j
		for k < n && isAlnum(s[k]) && k-j < maxEntity {
			k++
		}
		run := string(s[j:k])
		match := ""
		if at(k) == ';' {
			if _, ok := entities[run+";"]; ok {
				match = run + ";"
			}
		}
		if match == "" {
			for m := len(run); m > 0; m-- {
				if _, ok := entities[run[:m]]; ok {
					match = run[:m]
					break
				}
			}
		}
		if match == "" {
			out.WriteString("&" + run)
			i = k
		} else {
			out.WriteString(entities[match])
			i = j + len(match)
		}
	}
	return out.String()
}

var textEscaper = strings.NewReplacer("&", "&amp;", " ", "&nbsp;", "<", "&lt;", ">", "&gt;")
var attrEscaper = strings.NewReplacer("&", "&amp;", " ", "&nbsp;", "\"", "&quot;", "<", "&lt;", ">", "&gt;")

// EscapeText escapes text content: &, U+00A0, <, > (SPEC section 3.2).
func EscapeText(s string) string { return textEscaper.Replace(s) }

// EscapeAttr escapes an attribute value: &, U+00A0, ", <, > (SPEC section 3.2).
func EscapeAttr(s string) string { return attrEscaper.Replace(s) }

// ---------------------------------------------------------------- tree

// NodeKind is the type of a Node.
type NodeKind int

// Node kinds.
const (
	DocumentNode NodeKind = iota
	ElementNode
	TextNode
	CommentNode
)

// Node is a tree node. Name and Attrs are set for elements, Data for text and comments.
type Node struct {
	Kind     NodeKind
	Name     string
	Attrs    []Attr
	Data     string
	Parent   *Node
	Children []*Node
}

func set(s string) map[string]bool {
	m := map[string]bool{}
	for _, w := range strings.Fields(s) {
		m[w] = true
	}
	return m
}

func union(a map[string]bool, extra ...string) map[string]bool {
	m := map[string]bool{}
	for k := range a {
		m[k] = true
	}
	for _, k := range extra {
		m[k] = true
	}
	return m
}

var (
	voidElements = set("area base basefont bgsound br col embed frame hr img input keygen link meta param source track wbr")
	rawParents   = set("style script xmp iframe noembed noframes plaintext noscript")
	special      = set("address applet area article aside base basefont bgsound blockquote body br button caption center col colgroup dd " +
		"details dir div dl dt embed fieldset figcaption figure footer form frame frameset h1 h2 h3 h4 h5 h6 head header " +
		"hgroup hr html iframe img input keygen li link listing main marquee menu meta nav noembed noframes noscript " +
		"object ol p param plaintext pre script search section select source style summary table tbody td template " +
		"textarea tfoot th thead title tr track ul wbr xmp")
	pClosers = set("address article aside blockquote center details dialog dir div dl fieldset figcaption figure footer form h1 h2 " +
		"h3 h4 h5 h6 header hgroup hr li dd dt listing main menu nav ol p plaintext pre search section summary table ul xmp")
	headings    = set("h1 h2 h3 h4 h5 h6")
	scope       = set("applet caption html table td th marquee object template")
	buttonScope = union(scope, "button")
	listScope   = union(scope, "ol", "ul")
	tableScope  = set("html table template")
	tableParts  = set("table caption tbody thead tfoot tr td th")
	sections    = set("thead tbody tfoot")
	hidden      = set("script style noscript template title")
	setP        = set("p")
	setA        = set("a")
	setLi       = set("li")
	setDdDt     = set("dd dt")
	setCell     = set("td th")
	setTr       = set("tr")
	cellStart   = set("td th tr thead tbody tfoot")
	rowStart    = set("tr thead tbody tfoot")
	blocks      = set("address article aside blockquote caption details dialog div dl fieldset figcaption figure footer form header " +
		"hgroup main nav ol p pre section summary table ul")
)

// MaxDepth is the maximum number of open elements; deeper start tags are ignored.
const MaxDepth = 256

// Attr returns the value of the first attribute called name.
func (n *Node) Attr(name string) (string, bool) {
	for _, a := range n.Attrs {
		if a.Name == name {
			return a.Value, true
		}
	}
	return "", false
}

func (n *Node) attrOr(name string) string {
	v, _ := n.Attr(name)
	return v
}

// Is reports whether n is an element called name.
func (n *Node) Is(name string) bool { return n.Kind == ElementNode && n.Name == name }

// Elements returns the elements below n in document order.
func (n *Node) Elements() []*Node {
	var out []*Node
	todo := make([]*Node, 0, len(n.Children))
	for i := len(n.Children) - 1; i >= 0; i-- {
		todo = append(todo, n.Children[i])
	}
	for len(todo) > 0 {
		c := todo[len(todo)-1]
		todo = todo[:len(todo)-1]
		if c.Kind == ElementNode {
			out = append(out, c)
			for i := len(c.Children) - 1; i >= 0; i-- {
				todo = append(todo, c.Children[i])
			}
		}
	}
	return out
}

func (n *Node) elementSiblings() []*Node {
	if n.Parent == nil {
		return []*Node{n}
	}
	var out []*Node
	for _, c := range n.Parent.Children {
		if c.Kind == ElementNode {
			out = append(out, c)
		}
	}
	return out
}

type builder struct {
	doc   *Node
	stack []*Node
}

func (b *builder) current() *Node {
	if len(b.stack) > 0 {
		return b.stack[len(b.stack)-1]
	}
	return b.doc
}

func (b *builder) appendNode(node *Node) {
	p := b.current()
	node.Parent = p
	p.Children = append(p.Children, node)
}

func (b *builder) inScope(names, boundary map[string]bool) bool {
	for i := len(b.stack) - 1; i >= 0; i-- {
		if names[b.stack[i].Name] {
			return true
		}
		if boundary[b.stack[i].Name] {
			return false
		}
	}
	return false
}

func (b *builder) popUntil(names map[string]bool) {
	for len(b.stack) > 0 {
		top := b.stack[len(b.stack)-1]
		b.stack = b.stack[:len(b.stack)-1]
		if names[top.Name] {
			return
		}
	}
}

func (b *builder) top() string {
	if len(b.stack) == 0 {
		return ""
	}
	return b.stack[len(b.stack)-1].Name
}

func (b *builder) start(name string, attrs []Attr) bool {
	if name == "li" || name == "dd" || name == "dt" {
		targets := setDdDt
		if name == "li" {
			targets = setLi
		}
		for i := len(b.stack) - 1; i >= 0; i-- {
			cur := b.stack[i].Name
			if targets[cur] {
				b.stack = b.stack[:i]
				break
			}
			if special[cur] && cur != "address" && cur != "div" && cur != "p" {
				break
			}
		}
	}
	if pClosers[name] && b.inScope(setP, buttonScope) {
		b.popUntil(setP)
	}
	if headings[name] && headings[b.top()] {
		b.stack = b.stack[:len(b.stack)-1]
	}
	if (name == "option" || name == "optgroup") && b.top() == "option" {
		b.stack = b.stack[:len(b.stack)-1]
	}
	if name == "a" {
		for _, n := range b.stack {
			if n.Name == "a" {
				b.popUntil(setA)
				break
			}
		}
	}
	if cellStart[name] && b.inScope(setCell, tableScope) {
		b.popUntil(setCell)
	}
	if rowStart[name] && b.inScope(setTr, tableScope) {
		b.popUntil(setTr)
	}
	if sections[name] && b.inScope(sections, tableScope) {
		b.popUntil(sections)
	}
	if len(b.stack) >= MaxDepth {
		return false
	}
	el := &Node{Kind: ElementNode, Name: name, Attrs: attrs}
	b.appendNode(el)
	if !voidElements[name] {
		b.stack = append(b.stack, el)
	}
	return name == "pre" || name == "listing" || name == "textarea"
}

func (b *builder) end(name string) {
	var target, boundary map[string]bool
	switch {
	case name == "br":
		b.start("br", nil)
		return
	case name == "p":
		target, boundary = setP, buttonScope
	case headings[name]:
		target, boundary = headings, scope
	case name == "li":
		target, boundary = setLi, listScope
	case tableParts[name]:
		target, boundary = set(name), tableScope
	case name == "dd" || name == "dt" || special[name]:
		target, boundary = set(name), scope
	default:
		for i := len(b.stack) - 1; i >= 0; i-- {
			cur := b.stack[i].Name
			if cur == name {
				b.stack = b.stack[:i]
				return
			}
			if special[cur] {
				return
			}
		}
		return
	}
	if b.inScope(target, boundary) {
		b.popUntil(target)
	}
}

// Parse parses html into a document node (SPEC section 4). It never fails.
func Parse(html string) *Node {
	b := &builder{doc: &Node{Kind: DocumentNode}}
	skipNewline := false
	for _, tok := range Tokenize(html) {
		if tok.Kind == Character {
			data := tok.Data
			if skipNewline {
				data = strings.TrimPrefix(data, "\n")
			}
			skipNewline = false
			if data != "" {
				p := b.current()
				if k := len(p.Children); k > 0 && p.Children[k-1].Kind == TextNode {
					p.Children[k-1].Data += data
				} else {
					b.appendNode(&Node{Kind: TextNode, Data: data})
				}
			}
			continue
		}
		skipNewline = false
		switch tok.Kind {
		case StartTag:
			skipNewline = b.start(tok.Name, tok.Attrs)
		case EndTag:
			b.end(tok.Name)
		case Comment:
			b.appendNode(&Node{Kind: CommentNode, Data: tok.Data})
		}
	}
	return b.doc
}

// Serialize returns the outer HTML of an element, the inner HTML of the document (SPEC section 5).
func (n *Node) Serialize() string {
	var sb strings.Builder
	writeNode(n, &sb)
	return sb.String()
}

func writeNode(n *Node, sb *strings.Builder) {
	switch n.Kind {
	case TextNode:
		if n.Parent != nil && n.Parent.Kind == ElementNode && rawParents[n.Parent.Name] {
			sb.WriteString(n.Data)
		} else {
			sb.WriteString(EscapeText(n.Data))
		}
	case CommentNode:
		sb.WriteString("<!--" + n.Data + "-->")
	case ElementNode:
		sb.WriteString("<" + n.Name)
		for _, a := range n.Attrs {
			sb.WriteString(" " + a.Name + "=\"" + EscapeAttr(a.Value) + "\"")
		}
		sb.WriteString(">")
		if voidElements[n.Name] {
			return
		}
		for _, c := range n.Children {
			writeNode(c, sb)
		}
		sb.WriteString("</" + n.Name + ">")
	default:
		for _, c := range n.Children {
			writeNode(c, sb)
		}
	}
}

// ---------------------------------------------------------------- text

// TextContent returns the text below n, skipping script, style, noscript,
// template and title (SPEC section 6.1).
func (n *Node) TextContent() string {
	var sb strings.Builder
	var walk func(*Node)
	walk = func(x *Node) {
		switch {
		case x.Kind == TextNode:
			sb.WriteString(x.Data)
		case x.Kind == DocumentNode || (x.Kind == ElementNode && !hidden[x.Name]):
			for _, c := range x.Children {
				walk(c)
			}
		}
	}
	walk(n)
	return sb.String()
}

func isASCIIWs(c rune) bool { return c == '\t' || c == '\n' || c == '\f' || c == '\r' || c == ' ' }

func collapseRuns(s string) string {
	var sb strings.Builder
	inWs := false
	for _, c := range s {
		if isASCIIWs(c) {
			if !inWs {
				sb.WriteByte(' ')
			}
			inWs = true
		} else {
			sb.WriteRune(c)
			inWs = false
		}
	}
	return sb.String()
}

// ExtractText returns structure-preserving plain text (SPEC section 6.2).
func (n *Node) ExtractText() string {
	var sb strings.Builder
	var walk func(*Node, bool)
	walk = func(x *Node, pre bool) {
		switch x.Kind {
		case TextNode:
			if pre {
				sb.WriteString(x.Data)
				return
			}
			collapsed := collapseRuns(x.Data)
			cur := sb.String()
			if strings.HasPrefix(collapsed, " ") && (cur == "" || strings.ContainsAny(cur[len(cur)-1:], " \n\t")) {
				collapsed = collapsed[1:]
			}
			sb.WriteString(collapsed)
			return
		case CommentNode:
			return
		case DocumentNode:
			for _, c := range x.Children {
				walk(c, pre)
			}
			return
		}
		name := x.Name
		if hidden[name] {
			return
		}
		if headings[name] {
			sb.WriteString("\n\n" + strings.Repeat("#", int(name[1]-'0')) + " ")
			for _, c := range x.Children {
				walk(c, pre)
			}
			sb.WriteString("\n\n")
			return
		}
		switch name {
		case "li":
			sb.WriteString("\n- ")
		case "dd", "dt", "tr":
			sb.WriteString("\n")
		case "br":
			sb.WriteString("\n")
			return
		case "hr":
			sb.WriteString("\n\n---\n\n")
			return
		}
		block := blocks[name]
		if block {
			sb.WriteString("\n\n")
		}
		inner := pre || name == "pre" || name == "listing" || name == "textarea"
		for _, c := range x.Children {
			walk(c, inner)
		}
		if name == "td" || name == "th" {
			sb.WriteString("\t")
		}
		if block {
			sb.WriteString("\n\n")
		}
	}
	walk(n, false)
	lines := strings.Split(sb.String(), "\n")
	for i, l := range lines {
		lines[i] = strings.TrimRight(l, " \t")
	}
	text := strings.Join(lines, "\n")
	var out strings.Builder
	newlines := 0
	for _, c := range text {
		if c == '\n' {
			newlines++
			if newlines > 2 {
				continue
			}
		} else {
			newlines = 0
		}
		out.WriteRune(c)
	}
	return strings.Trim(out.String(), "\n")
}

// HTMLToText parses html and returns its structure-preserving plain text.
func HTMLToText(html string) string { return Parse(html).ExtractText() }

// StripTags parses html and returns its text content without markup.
func StripTags(html string) string { return Parse(html).TextContent() }

// ---------------------------------------------------------------- sanitizer

// DefaultSchemes are the URL schemes allowed by default.
var DefaultSchemes = []string{"http", "https", "mailto", "tel"}

const defaultAttrs = "*:dir *:lang *:title a:href blockquote:cite img:alt img:height img:src img:width ol:start q:cite " +
	"td:colspan td:rowspan th:colspan th:rowspan th:scope time:datetime"

var urlAttrs = set("cite href src")

// Policy is a sanitizer policy: the defaults plus extra tags, attributes and schemes.
type Policy struct {
	tags, drop, attrs, schemes map[string]bool
}

// NewPolicy returns the default policy.
func NewPolicy() *Policy {
	return &Policy{
		tags: set("a abbr b blockquote br caption cite code dd del dfn div dl dt em figcaption figure h1 h2 h3 h4 h5 h6 hr i img " +
			"ins kbd li mark ol p pre q s samp small span strong sub sup table tbody td tfoot th thead time tr u ul"),
		drop: set("applet audio base button canvas embed frame frameset head iframe link math meta noembed noframes noscript " +
			"object option plaintext script select style svg template textarea title video xmp"),
		attrs:   set(defaultAttrs),
		schemes: set(strings.Join(DefaultSchemes, " ")),
	}
}

// AllowTag keeps tag (ASCII case-insensitive), also when it is on the drop list.
func (p *Policy) AllowTag(tag string) *Policy {
	t := asciiLower(tag)
	if t == "plaintext" { // cannot be closed again, so never kept
		return p
	}
	p.tags[t] = true
	delete(p.drop, t)
	return p
}

// AllowAttr keeps attribute attr on tag; tag "*" means every kept element.
func (p *Policy) AllowAttr(tag, attr string) *Policy {
	p.attrs[asciiLower(tag)+":"+asciiLower(attr)] = true
	return p
}

// AllowScheme accepts URLs with scheme in href, src and cite.
func (p *Policy) AllowScheme(scheme string) *Policy {
	p.schemes[asciiLower(scheme)] = true
	return p
}

// IsSafeURL reports whether url has no scheme or one of schemes (lowercase),
// after removing C0 controls, space and DEL (SPEC section 7.3). With no
// schemes given, DefaultSchemes apply.
func IsSafeURL(url string, schemes ...string) bool {
	if len(schemes) == 0 {
		schemes = DefaultSchemes
	}
	return safeURL(url, set(strings.Join(schemes, " ")))
}

func safeURL(url string, schemes map[string]bool) bool {
	var sb strings.Builder
	for _, c := range url {
		if c > 0x20 && c != 0x7f {
			sb.WriteRune(c)
		}
	}
	cleaned := asciiLower(sb.String())
	i := strings.IndexAny(cleaned, "/?#:")
	if i < 0 || cleaned[i] != ':' {
		return true
	}
	return schemes[cleaned[:i]]
}

// Sanitize sanitizes html with policy, or the default policy when nil (SPEC
// section 7). The result is a fixed point.
func Sanitize(html string, policy *Policy) string {
	p := policy
	if p == nil {
		p = NewPolicy()
	}
	var sb strings.Builder
	var walk func(*Node)
	walk = func(n *Node) {
		switch n.Kind {
		case TextNode:
			sb.WriteString(EscapeText(n.Data))
			return
		case CommentNode:
			return
		case ElementNode:
			if p.drop[n.Name] {
				return
			}
			if p.tags[n.Name] {
				sb.WriteString("<" + n.Name)
				for _, a := range n.Attrs {
					if !p.attrs[n.Name+":"+a.Name] && !p.attrs["*:"+a.Name] {
						continue
					}
					if urlAttrs[a.Name] && !safeURL(a.Value, p.schemes) {
						continue
					}
					sb.WriteString(" " + a.Name + "=\"" + EscapeAttr(a.Value) + "\"")
				}
				sb.WriteString(">")
				if voidElements[n.Name] {
					return
				}
				// Raw text content would not survive a second pass, so it is dropped.
				if !rawParents[n.Name] {
					for _, c := range n.Children {
						walk(c)
					}
				}
				sb.WriteString("</" + n.Name + ">")
				return
			}
		}
		for _, c := range n.Children {
			walk(c)
		}
	}
	walk(Parse(html))
	return sb.String()
}

// ---------------------------------------------------------------- meta and tables

// PageMeta is the metadata found in a document (SPEC section 9). Nil means absent.
type PageMeta struct {
	Title, Description, Canonical, Lang *string
	// OG holds (property without "og:", content) pairs in document order.
	OG [][2]string
}

func collapse(s string) string { return strings.Trim(collapseRuns(s), " ") }

func splitASCIIWs(s string) []string { return strings.FieldsFunc(s, isASCIIWs) }

// Meta extracts PageMeta (SPEC section 9).
func (n *Node) Meta() PageMeta {
	out := PageMeta{OG: [][2]string{}}
	seenTitle := false
	for _, el := range n.Elements() {
		switch {
		case el.Name == "title" && !seenTitle:
			seenTitle = true
			var raw strings.Builder
			for _, c := range el.Children {
				if c.Kind == TextNode {
					raw.WriteString(c.Data)
				}
			}
			if t := collapse(raw.String()); t != "" {
				out.Title = &t
			}
		case el.Name == "html":
			if v, ok := el.Attr("lang"); ok && out.Lang == nil {
				out.Lang = &v
			}
		case el.Name == "meta":
			content, ok := el.Attr("content")
			if !ok {
				continue
			}
			if asciiLower(el.attrOr("name")) == "description" && out.Description == nil {
				c := content
				out.Description = &c
			}
			prop := el.attrOr("property")
			if len(prop) >= 3 && asciiLower(prop[:3]) == "og:" {
				out.OG = append(out.OG, [2]string{prop[3:], content})
			}
		case el.Name == "link" && out.Canonical == nil:
			for _, r := range splitASCIIWs(asciiLower(el.attrOr("rel"))) {
				if r == "canonical" {
					if v, ok := el.Attr("href"); ok {
						out.Canonical = &v
					}
					break
				}
			}
		}
	}
	return out
}

func parseSpan(v string, ok bool, def, maximum int) int {
	if !ok {
		return def
	}
	t := strings.TrimLeftFunc(v, isASCIIWs)
	t = strings.TrimPrefix(t, "+")
	end := 0
	for end < len(t) && t[end] >= '0' && t[end] <= '9' {
		end++
	}
	if end == 0 {
		return def
	}
	n := maximum
	if end <= 9 {
		n, _ = strconv.Atoi(t[:end])
	}
	if n == 0 {
		return def
	}
	if n > maximum {
		return maximum
	}
	return n
}

// MaxCells is the most cells produced for one table; extraction stops there.
const MaxCells = 1000000

func collectRows(n *Node, rows *[]*Node) {
	for _, c := range n.Children {
		if c.Kind != ElementNode || c.Name == "table" {
			continue
		}
		if c.Name == "tr" {
			*rows = append(*rows, c)
		} else {
			collectRows(c, rows)
		}
	}
}

type spanCell struct {
	left int
	text string
}

// Tables returns every table as a grid of cell texts (SPEC section 10).
func (n *Node) Tables() [][][]string {
	tables := [][][]string{}
	for _, table := range n.Elements() {
		if table.Name != "table" {
			continue
		}
		var rows []*Node
		collectRows(table, &rows)
		grid := [][]string{}
		pending := map[int]spanCell{}
		total := 0
		stop := false
		for _, tr := range rows {
			row := map[int]string{}
			next := map[int]spanCell{}
			for col, sc := range pending {
				row[col] = sc.text
				if sc.left > 1 {
					next[col] = spanCell{sc.left - 1, sc.text}
				}
			}
			col := 0
			for _, cell := range tr.Children {
				if !cell.Is("td") && !cell.Is("th") {
					continue
				}
				text := collapse(cell.TextContent())
				cv, cok := cell.Attr("colspan")
				rv, rok := cell.Attr("rowspan")
				cs := parseSpan(cv, cok, 1, 1000)
				rs := parseSpan(rv, rok, 1, 65534)
				for x := 0; x < cs; x++ {
					for {
						if _, taken := row[col]; !taken {
							break
						}
						col++
					}
					total++
					if total > MaxCells {
						stop = true
						break
					}
					row[col] = text
					if rs > 1 {
						next[col] = spanCell{rs - 1, text}
					}
					col++
				}
				if stop {
					break
				}
			}
			pending = next
			width := 0
			for c := range row {
				if c+1 > width {
					width = c + 1
				}
			}
			line := make([]string, width)
			for c := range line {
				line[c] = row[c]
			}
			grid = append(grid, line)
			if stop {
				break
			}
		}
		tables = append(tables, grid)
	}
	return tables
}
