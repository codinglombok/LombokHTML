package lombokhtml

import (
	"strings"
	"unicode/utf8"
)

// TokenKind names a token type as written in the SPEC.
type TokenKind string

// Token kinds.
const (
	StartTag  TokenKind = "StartTag"
	EndTag    TokenKind = "EndTag"
	Character TokenKind = "Character"
	Comment   TokenKind = "Comment"
	Doctype   TokenKind = "DOCTYPE"
)

// Attr is one attribute; names are lowercased.
type Attr struct {
	Name, Value string
}

// Token is one tokenizer output. Name is set for tags and DOCTYPE (nil Name
// pointer below means a missing DOCTYPE name), Data for characters and comments.
type Token struct {
	Kind        TokenKind
	Name        string
	Attrs       []Attr
	SelfClosing bool
	Data        string
	// DOCTYPE fields; nil means missing.
	DoctypeName, PublicID, SystemID *string
	// Correct is false when the tokenizer set the DOCTYPE force-quirks flag.
	Correct bool
}

// States accepted by TokenizeState (SPEC section 2.1).
var States = []string{"data", "rcdata", "rawtext", "script", "plaintext", "cdata"}

const maxEntity = 32
const eof rune = -1
const repl = "�"

var replacements = map[rune]rune{
	0x80: 0x20AC, 0x82: 0x201A, 0x83: 0x0192, 0x84: 0x201E, 0x85: 0x2026, 0x86: 0x2020, 0x87: 0x2021,
	0x88: 0x02C6, 0x89: 0x2030, 0x8A: 0x0160, 0x8B: 0x2039, 0x8C: 0x0152, 0x8E: 0x017D, 0x91: 0x2018,
	0x92: 0x2019, 0x93: 0x201C, 0x94: 0x201D, 0x95: 0x2022, 0x96: 0x2013, 0x97: 0x2014, 0x98: 0x02DC,
	0x99: 0x2122, 0x9A: 0x0161, 0x9B: 0x203A, 0x9C: 0x0153, 0x9E: 0x017E, 0x9F: 0x0178,
}

func numericChar(code int) string {
	if code == 0 || code > 0x10FFFF || (code >= 0xD800 && code <= 0xDFFF) {
		return repl
	}
	if r, ok := replacements[rune(code)]; ok {
		return string(r)
	}
	return string(rune(code))
}

func isWs(c rune) bool    { return c == '\t' || c == '\n' || c == '\f' || c == ' ' }
func isAlpha(c rune) bool { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') }
func isDigit(c rune) bool { return c >= '0' && c <= '9' }
func isHex(c rune) bool   { return isDigit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F') }
func isAlnum(c rune) bool { return isAlpha(c) || isDigit(c) }

func lowerRune(c rune) rune {
	if c >= 'A' && c <= 'Z' {
		return c + 32
	}
	return c
}

func asciiLower(s string) string {
	b := []byte(s)
	for i, c := range b {
		if c >= 'A' && c <= 'Z' {
			b[i] = c + 32
		}
	}
	return string(b)
}

func utf8Append(b []byte, c rune) []byte {
	return utf8.AppendRune(b, c)
}

func lowerOrRepl(c rune) string {
	if c == 0 {
		return repl
	}
	return string(lowerRune(c))
}

func orRepl(c rune) string {
	if c == 0 {
		return repl
	}
	return string(c)
}

type tag struct {
	end         bool
	name        []byte
	attrs       []*attrBuf
	selfClosing bool
}

// attrBuf collects an attribute; byte slices keep appends amortized O(1).
type attrBuf struct {
	name, value []byte
}

type tokenizer struct {
	s         []rune
	i         int
	state     string
	ret       string
	lastStart string
	hasLast   bool
	switching bool
	out       []Token
	chars     []byte // pending Character data, flushed before any other token
	tag       *tag
	attr      *attrBuf
	temp      []byte
	comment   []byte
	dName     []byte
	dPublic   []byte
	dSystem   []byte
	hasName   bool
	hasPublic bool
	hasSystem bool
	quirks    bool
	code      int
}

func newTokenizer(text, state string, lastStart *string, switching bool) *tokenizer {
	text = strings.ReplaceAll(text, "\r\n", "\n")
	text = strings.ReplaceAll(text, "\r", "\n")
	t := &tokenizer{s: []rune(text), state: state, ret: "data", switching: switching}
	if lastStart != nil {
		t.lastStart, t.hasLast = *lastStart, true
	}
	return t
}

func (t *tokenizer) emit(s string) {
	t.chars = append(t.chars, s...)
}

// push appends a non-character token after flushing pending character data.
func (t *tokenizer) push(tok Token) {
	t.flushChars()
	t.out = append(t.out, tok)
}

func (t *tokenizer) flushChars() {
	if len(t.chars) > 0 {
		t.out = append(t.out, Token{Kind: Character, Data: string(t.chars)})
		t.chars = t.chars[:0]
	}
}

func (t *tokenizer) emitTag() {
	tg := t.tag
	name := string(tg.name)
	if tg.end {
		t.push(Token{Kind: EndTag, Name: name})
	} else {
		seen := map[string]bool{}
		attrs := []Attr{}
		for _, a := range tg.attrs {
			k := string(a.name)
			if !seen[k] {
				seen[k] = true
				attrs = append(attrs, Attr{Name: k, Value: string(a.value)})
			}
		}
		t.push(Token{Kind: StartTag, Name: name, Attrs: attrs, SelfClosing: tg.selfClosing})
		t.lastStart, t.hasLast = name, true
		if t.switching {
			switch name {
			case "title", "textarea":
				t.state = "rcdata"
			case "style", "xmp", "iframe", "noembed", "noframes", "noscript":
				t.state = "rawtext"
			case "script":
				t.state = "script"
			case "plaintext":
				t.state = "plaintext"
			}
		}
	}
	t.tag = nil
}

func (t *tokenizer) appropriate() bool {
	return t.tag != nil && t.tag.end && t.hasLast && string(t.tag.name) == t.lastStart
}

func (t *tokenizer) startAttr() {
	t.attr = &attrBuf{}
	t.tag.attrs = append(t.tag.attrs, t.attr)
}

func (t *tokenizer) peek(n int) string {
	end := t.i + n
	if end > len(t.s) {
		end = len(t.s)
	}
	return string(t.s[t.i:end])
}

func (t *tokenizer) at(i int) rune {
	if i < len(t.s) {
		return t.s[i]
	}
	return eof
}

func (t *tokenizer) inAttr() bool {
	return t.ret == "attr_dq" || t.ret == "attr_sq" || t.ret == "attr_unq"
}

func (t *tokenizer) flushRef() {
	if t.inAttr() {
		t.attr.value = append(t.attr.value, t.temp...)
	} else {
		t.chars = append(t.chars, t.temp...)
	}
}

func (t *tokenizer) reconsume(state string) {
	t.i--
	t.state = state
}

func (t *tokenizer) emitComment() {
	t.push(Token{Kind: Comment, Data: string(t.comment)})
}

func (t *tokenizer) newDoctype() {
	t.dName, t.dPublic, t.dSystem = t.dName[:0], t.dPublic[:0], t.dSystem[:0]
	t.hasName, t.hasPublic, t.hasSystem, t.quirks = false, false, false, false
}

func optional(has bool, b []byte) *string {
	if !has {
		return nil
	}
	s := string(b)
	return &s
}

func (t *tokenizer) emitDoctype(quirks bool) {
	if quirks {
		t.quirks = true
	}
	t.push(Token{Kind: Doctype, DoctypeName: optional(t.hasName, t.dName), PublicID: optional(t.hasPublic, t.dPublic),
		SystemID: optional(t.hasSystem, t.dSystem), Correct: !t.quirks})
}

func (t *tokenizer) run() []Token {
	for {
		c := t.at(t.i)
		t.i++
		if !t.step(c) {
			t.flushChars()
			return t.out
		}
	}
}

func (t *tokenizer) lt(c rune, base, openState string) {
	if c == '/' {
		t.temp = t.temp[:0]
		t.state = openState
	} else {
		t.emit("<")
		t.reconsume(base)
	}
}

func (t *tokenizer) endOpen(c rune, base, nameState string) {
	if isAlpha(c) {
		t.tag = &tag{end: true}
		t.reconsume(nameState)
	} else {
		t.emit("</")
		t.reconsume(base)
	}
}

func (t *tokenizer) endName(c rune, base string) {
	switch {
	case isWs(c) && t.appropriate():
		t.state = "before_attr_name"
	case c == '/' && t.appropriate():
		t.state = "self_closing"
	case c == '>' && t.appropriate():
		t.state = "data"
		t.emitTag()
	case isAlpha(c):
		t.tag.name = utf8Append(t.tag.name, lowerRune(c))
		t.temp = append(t.temp, string(c)...)
	default:
		t.emit("</")
		t.chars = append(t.chars, t.temp...)
		t.tag = nil
		t.reconsume(base)
	}
}

func (t *tokenizer) setDoctypeID(public bool) {
	if public {
		t.dPublic, t.hasPublic = t.dPublic[:0], true
	} else {
		t.dSystem, t.hasSystem = t.dSystem[:0], true
	}
}

func quoteState(which string, c rune) string {
	if c == '"' {
		return "doctype_" + which + "_dq"
	}
	return "doctype_" + which + "_sq"
}

// step consumes c and returns false at end of input.
func (t *tokenizer) step(c rune) bool {
	st := t.state
	switch st {
	case "data":
		switch {
		case c == '&':
			t.ret, t.state = "data", "charref"
		case c == '<':
			t.state = "tag_open"
		case c == eof:
			return false
		default:
			t.emit(string(c))
		}
	case "rcdata":
		switch {
		case c == '&':
			t.ret, t.state = "rcdata", "charref"
		case c == '<':
			t.state = "rcdata_lt"
		case c == eof:
			return false
		default:
			t.emit(orRepl(c))
		}
	case "rawtext", "script":
		switch {
		case c == '<':
			t.state = st + "_lt"
		case c == eof:
			return false
		default:
			t.emit(orRepl(c))
		}
	case "plaintext":
		if c == eof {
			return false
		}
		t.emit(orRepl(c))
	case "tag_open":
		switch {
		case c == '!':
			t.state = "markup_decl"
		case c == '/':
			t.state = "end_tag_open"
		case isAlpha(c):
			t.tag = &tag{}
			t.reconsume("tag_name")
		case c == '?':
			t.comment = t.comment[:0]
			t.reconsume("bogus_comment")
		case c == eof:
			t.emit("<")
			return false
		default:
			t.emit("<")
			t.reconsume("data")
		}
	case "end_tag_open":
		switch {
		case isAlpha(c):
			t.tag = &tag{end: true}
			t.reconsume("tag_name")
		case c == '>':
			t.state = "data"
		case c == eof:
			t.emit("</")
			return false
		default:
			t.comment = t.comment[:0]
			t.reconsume("bogus_comment")
		}
	case "tag_name":
		switch {
		case isWs(c):
			t.state = "before_attr_name"
		case c == '/':
			t.state = "self_closing"
		case c == '>':
			t.state = "data"
			t.emitTag()
		case c == eof:
			return false
		default:
			t.tag.name = append(t.tag.name, lowerOrRepl(c)...)
		}
	case "rcdata_lt":
		t.lt(c, "rcdata", "rcdata_end_open")
	case "rcdata_end_open":
		t.endOpen(c, "rcdata", "rcdata_end_name")
	case "rcdata_end_name":
		t.endName(c, "rcdata")
	case "rawtext_lt":
		t.lt(c, "rawtext", "rawtext_end_open")
	case "rawtext_end_open":
		t.endOpen(c, "rawtext", "rawtext_end_name")
	case "rawtext_end_name":
		t.endName(c, "rawtext")
	case "script_lt":
		switch c {
		case '/':
			t.temp = t.temp[:0]
			t.state = "script_end_open"
		case '!':
			t.state = "script_esc_start"
			t.emit("<!")
		default:
			t.emit("<")
			t.reconsume("script")
		}
	case "script_end_open":
		t.endOpen(c, "script", "script_end_name")
	case "script_end_name":
		t.endName(c, "script")
	case "script_esc_start", "script_esc_start_dash":
		if c == '-' {
			if st == "script_esc_start" {
				t.state = "script_esc_start_dash"
			} else {
				t.state = "script_esc_dash_dash"
			}
			t.emit("-")
		} else {
			t.reconsume("script")
		}
	case "script_esc", "script_esc_dash", "script_esc_dash_dash":
		switch {
		case c == '-':
			if st == "script_esc" {
				t.state = "script_esc_dash"
			} else {
				t.state = "script_esc_dash_dash"
			}
			t.emit("-")
		case c == '<':
			t.state = "script_esc_lt"
		case c == '>' && st == "script_esc_dash_dash":
			t.state = "script"
			t.emit(">")
		case c == eof:
			return false
		default:
			t.state = "script_esc"
			t.emit(orRepl(c))
		}
	case "script_esc_lt":
		switch {
		case c == '/':
			t.temp = t.temp[:0]
			t.state = "script_esc_end_open"
		case isAlpha(c):
			t.temp = t.temp[:0]
			t.emit("<")
			t.reconsume("script_dbl_esc_start")
		default:
			t.emit("<")
			t.reconsume("script_esc")
		}
	case "script_esc_end_open":
		t.endOpen(c, "script_esc", "script_esc_end_name")
	case "script_esc_end_name":
		t.endName(c, "script_esc")
	case "script_dbl_esc_start", "script_dbl_esc_end":
		start := st == "script_dbl_esc_start"
		switch {
		case isWs(c) || c == '/' || c == '>':
			if (string(t.temp) == "script") == start {
				t.state = "script_dbl_esc"
			} else {
				t.state = "script_esc"
			}
			t.emit(string(c))
		case isAlpha(c):
			t.temp = utf8Append(t.temp, lowerRune(c))
			t.emit(string(c))
		case start:
			t.reconsume("script_esc")
		default:
			t.reconsume("script_dbl_esc")
		}
	case "script_dbl_esc", "script_dbl_esc_dash", "script_dbl_esc_dash_dash":
		switch {
		case c == '-':
			if st == "script_dbl_esc" {
				t.state = "script_dbl_esc_dash"
			} else {
				t.state = "script_dbl_esc_dash_dash"
			}
			t.emit("-")
		case c == '<':
			t.state = "script_dbl_esc_lt"
			t.emit("<")
		case c == '>' && st == "script_dbl_esc_dash_dash":
			t.state = "script"
			t.emit(">")
		case c == eof:
			return false
		default:
			t.state = "script_dbl_esc"
			t.emit(orRepl(c))
		}
	case "script_dbl_esc_lt":
		if c == '/' {
			t.temp = t.temp[:0]
			t.state = "script_dbl_esc_end"
			t.emit("/")
		} else {
			t.reconsume("script_dbl_esc")
		}
	case "before_attr_name":
		switch {
		case isWs(c):
		case c == eof || c == '/' || c == '>':
			t.reconsume("after_attr_name")
		case c == '=':
			t.startAttr()
			t.attr.name = append(t.attr.name, '=')
			t.state = "attr_name"
		default:
			t.startAttr()
			t.reconsume("attr_name")
		}
	case "attr_name":
		switch {
		case c == eof || isWs(c) || c == '/' || c == '>':
			t.reconsume("after_attr_name")
		case c == '=':
			t.state = "before_attr_value"
		default:
			t.attr.name = append(t.attr.name, lowerOrRepl(c)...)
		}
	case "after_attr_name":
		switch {
		case isWs(c):
		case c == '/':
			t.state = "self_closing"
		case c == '=':
			t.state = "before_attr_value"
		case c == '>':
			t.state = "data"
			t.emitTag()
		case c == eof:
			return false
		default:
			t.startAttr()
			t.reconsume("attr_name")
		}
	case "before_attr_value":
		switch {
		case isWs(c):
		case c == '"':
			t.state = "attr_dq"
		case c == '\'':
			t.state = "attr_sq"
		case c == '>':
			t.state = "data"
			t.emitTag()
		default:
			t.reconsume("attr_unq")
		}
	case "attr_dq", "attr_sq":
		q := '"'
		if st == "attr_sq" {
			q = '\''
		}
		switch {
		case c == q:
			t.state = "after_attr_value_q"
		case c == '&':
			t.ret, t.state = st, "charref"
		case c == eof:
			return false
		default:
			t.attr.value = append(t.attr.value, orRepl(c)...)
		}
	case "attr_unq":
		switch {
		case isWs(c):
			t.state = "before_attr_name"
		case c == '&':
			t.ret, t.state = "attr_unq", "charref"
		case c == '>':
			t.state = "data"
			t.emitTag()
		case c == eof:
			return false
		default:
			t.attr.value = append(t.attr.value, orRepl(c)...)
		}
	case "after_attr_value_q":
		switch {
		case isWs(c):
			t.state = "before_attr_name"
		case c == '/':
			t.state = "self_closing"
		case c == '>':
			t.state = "data"
			t.emitTag()
		case c == eof:
			return false
		default:
			t.reconsume("before_attr_name")
		}
	case "self_closing":
		switch {
		case c == '>':
			t.tag.selfClosing = true
			t.state = "data"
			t.emitTag()
		case c == eof:
			return false
		default:
			t.reconsume("before_attr_name")
		}
	case "bogus_comment":
		switch {
		case c == '>':
			t.state = "data"
			t.emitComment()
		case c == eof:
			t.emitComment()
			return false
		default:
			t.comment = append(t.comment, orRepl(c)...)
		}
	case "markup_decl":
		t.i--
		t.comment = t.comment[:0]
		switch {
		case t.peek(2) == "--":
			t.i += 2
			t.state = "comment_start"
		case asciiLower(t.peek(7)) == "doctype":
			t.i += 7
			t.state = "doctype"
		default:
			t.state = "bogus_comment"
		}
	case "comment_start":
		switch c {
		case '-':
			t.state = "comment_start_dash"
		case '>':
			t.state = "data"
			t.emitComment()
		default:
			t.reconsume("comment")
		}
	case "comment_start_dash":
		switch c {
		case '-':
			t.state = "comment_end"
		case '>':
			t.state = "data"
			t.emitComment()
		case eof:
			t.emitComment()
			return false
		default:
			t.comment = append(t.comment, "-"...)
			t.reconsume("comment")
		}
	case "comment":
		switch c {
		case '<':
			t.comment = append(t.comment, "<"...)
			t.state = "comment_lt"
		case '-':
			t.state = "comment_end_dash"
		case eof:
			t.emitComment()
			return false
		default:
			t.comment = append(t.comment, orRepl(c)...)
		}
	case "comment_lt":
		switch c {
		case '!':
			t.comment = append(t.comment, "!"...)
			t.state = "comment_lt_bang"
		case '<':
			t.comment = append(t.comment, "<"...)
		default:
			t.reconsume("comment")
		}
	case "comment_lt_bang":
		if c == '-' {
			t.state = "comment_lt_bang_dash"
		} else {
			t.reconsume("comment")
		}
	case "comment_lt_bang_dash":
		if c == '-' {
			t.state = "comment_lt_bang_dash_dash"
		} else {
			t.reconsume("comment_end_dash")
		}
	case "comment_lt_bang_dash_dash":
		t.reconsume("comment_end")
	case "comment_end_dash":
		switch c {
		case '-':
			t.state = "comment_end"
		case eof:
			t.emitComment()
			return false
		default:
			t.comment = append(t.comment, "-"...)
			t.reconsume("comment")
		}
	case "comment_end":
		switch c {
		case '>':
			t.state = "data"
			t.emitComment()
		case '!':
			t.state = "comment_end_bang"
		case '-':
			t.comment = append(t.comment, "-"...)
		case eof:
			t.emitComment()
			return false
		default:
			t.comment = append(t.comment, "--"...)
			t.reconsume("comment")
		}
	case "comment_end_bang":
		switch c {
		case '-':
			t.comment = append(t.comment, "--!"...)
			t.state = "comment_end_dash"
		case '>':
			t.state = "data"
			t.emitComment()
		case eof:
			t.emitComment()
			return false
		default:
			t.comment = append(t.comment, "--!"...)
			t.reconsume("comment")
		}
	case "doctype":
		switch {
		case isWs(c):
			t.state = "before_doctype_name"
		case c == eof:
			t.newDoctype()
			t.emitDoctype(true)
			return false
		default:
			t.reconsume("before_doctype_name")
		}
	case "before_doctype_name":
		if isWs(c) {
			return true
		}
		t.newDoctype()
		switch c {
		case '>':
			t.state = "data"
			t.emitDoctype(true)
		case eof:
			t.emitDoctype(true)
			return false
		default:
			t.dName, t.hasName = append(t.dName[:0], lowerOrRepl(c)...), true
			t.state = "doctype_name"
		}
	case "doctype_name":
		switch {
		case isWs(c):
			t.state = "after_doctype_name"
		case c == '>':
			t.state = "data"
			t.emitDoctype(false)
		case c == eof:
			t.emitDoctype(true)
			return false
		default:
			t.dName = append(t.dName, lowerOrRepl(c)...)
		}
	case "after_doctype_name":
		switch {
		case isWs(c):
		case c == '>':
			t.state = "data"
			t.emitDoctype(false)
		case c == eof:
			t.emitDoctype(true)
			return false
		default:
			t.i--
			word := asciiLower(t.peek(6))
			if word == "public" || word == "system" {
				t.i += 6
				t.state = "after_doctype_" + word + "_kw"
			} else {
				t.i++
				t.quirks = true
				t.state = "bogus_doctype"
			}
		}
	case "after_doctype_public_kw", "after_doctype_system_kw", "before_doctype_public_id", "before_doctype_system_id":
		which := "system"
		if strings.Contains(st, "public") {
			which = "public"
		}
		switch {
		case isWs(c):
			if strings.HasPrefix(st, "after") {
				t.state = "before_doctype_" + which + "_id"
			}
		case c == '"' || c == '\'':
			t.setDoctypeID(which == "public")
			t.state = quoteState(which, c)
		case c == '>':
			t.state = "data"
			t.emitDoctype(true)
		case c == eof:
			t.emitDoctype(true)
			return false
		default:
			t.quirks = true
			t.reconsume("bogus_doctype")
		}
	case "doctype_public_dq", "doctype_public_sq", "doctype_system_dq", "doctype_system_sq":
		pub := strings.Contains(st, "public")
		q := '\''
		if strings.HasSuffix(st, "dq") {
			q = '"'
		}
		switch {
		case c == q:
			if pub {
				t.state = "after_doctype_public_id"
			} else {
				t.state = "after_doctype_system_id"
			}
		case c == '>':
			t.state = "data"
			t.emitDoctype(true)
		case c == eof:
			t.emitDoctype(true)
			return false
		case pub:
			t.dPublic = append(t.dPublic, orRepl(c)...)
		default:
			t.dSystem = append(t.dSystem, orRepl(c)...)
		}
	case "after_doctype_public_id", "between_doctype_ids":
		switch {
		case isWs(c):
			t.state = "between_doctype_ids"
		case c == '>':
			t.state = "data"
			t.emitDoctype(false)
		case c == '"' || c == '\'':
			t.setDoctypeID(false)
			t.state = quoteState("system", c)
		case c == eof:
			t.emitDoctype(true)
			return false
		default:
			t.quirks = true
			t.reconsume("bogus_doctype")
		}
	case "after_doctype_system_id":
		switch {
		case isWs(c):
		case c == '>':
			t.state = "data"
			t.emitDoctype(false)
		case c == eof:
			t.emitDoctype(true)
			return false
		default:
			t.reconsume("bogus_doctype")
		}
	case "bogus_doctype":
		switch c {
		case '>':
			t.state = "data"
			t.emitDoctype(false)
		case eof:
			t.emitDoctype(false)
			return false
		}
	case "cdata":
		switch c {
		case ']':
			t.state = "cdata_bracket"
		case eof:
			return false
		default:
			t.emit(string(c))
		}
	case "cdata_bracket":
		if c == ']' {
			t.state = "cdata_end"
		} else {
			t.emit("]")
			t.reconsume("cdata")
		}
	case "cdata_end":
		switch c {
		case ']':
			t.emit("]")
		case '>':
			t.state = "data"
		default:
			t.emit("]]")
			t.reconsume("cdata")
		}
	case "charref":
		t.temp = append(t.temp[:0], '&')
		switch {
		case isAlnum(c):
			t.reconsume("named_ref")
		case c == '#':
			t.temp = append(t.temp, '#')
			t.state = "numeric_ref"
		default:
			t.flushRef()
			t.reconsume(t.ret)
		}
	case "named_ref":
		t.namedRef()
	case "ambiguous_amp":
		if isAlnum(c) {
			if t.inAttr() {
				t.attr.value = utf8Append(t.attr.value, c)
			} else {
				t.emit(string(c))
			}
		} else {
			t.reconsume(t.ret)
		}
	case "numeric_ref":
		t.code = 0
		if c == 'x' || c == 'X' {
			t.temp = append(t.temp, string(c)...)
			t.state = "hex_ref_start"
		} else {
			t.reconsume("dec_ref_start")
		}
	case "hex_ref_start", "dec_ref_start":
		hex := st == "hex_ref_start"
		if (hex && isHex(c)) || (!hex && isDigit(c)) {
			if hex {
				t.reconsume("hex_ref")
			} else {
				t.reconsume("dec_ref")
			}
		} else {
			t.flushRef()
			t.reconsume(t.ret)
		}
	case "hex_ref", "dec_ref":
		hex := st == "hex_ref"
		switch {
		case (hex && isHex(c)) || (!hex && isDigit(c)):
			base := 10
			if hex {
				base = 16
			}
			t.code = t.code*base + hexValue(c)
			if t.code > 0x110000 {
				t.code = 0x110000
			}
		case c == ';':
			t.state = "numeric_ref_end"
		default:
			t.reconsume("numeric_ref_end")
		}
	default: // numeric_ref_end
		t.i--
		t.temp = append(t.temp[:0], numericChar(t.code)...)
		t.flushRef()
		t.state = t.ret
	}
	return true
}

func hexValue(c rune) int {
	switch {
	case c >= '0' && c <= '9':
		return int(c - '0')
	case c >= 'a' && c <= 'f':
		return int(c-'a') + 10
	default:
		return int(c-'A') + 10
	}
}

func (t *tokenizer) namedRef() {
	t.i--
	start := t.i
	j := start
	for j < len(t.s) && isAlnum(t.s[j]) && j-start < maxEntity {
		j++
	}
	run := string(t.s[start:j])
	match := ""
	if t.at(j) == ';' {
		if _, ok := entities[run+";"]; ok {
			match = run + ";"
		}
	}
	if match == "" {
		for k := len(run); k > 0; k-- {
			if _, ok := entities[run[:k]]; ok {
				match = run[:k]
				break
			}
		}
	}
	if match == "" {
		t.temp = append(t.temp, run...)
		t.i = j
		t.flushRef()
		t.state = "ambiguous_amp"
		return
	}
	t.i = start + len(match)
	next := t.at(t.i)
	if t.inAttr() && !strings.HasSuffix(match, ";") && (next == '=' || isAlnum(next)) {
		t.temp = append(t.temp, match...)
	} else {
		t.temp = append(t.temp[:0], entities[match]...)
	}
	t.flushRef()
	t.state = t.ret
}

// Tokenize tokenizes html as the parser does: start tags of title, textarea,
// style, xmp, iframe, noembed, noframes, noscript, script and plaintext switch
// the tokenizer state (SPEC section 2.2).
func Tokenize(html string) []Token {
	return newTokenizer(html, "data", nil, true).run()
}

// TokenizeState tokenizes html from state (one of States) without
// element-driven switching, as the html5lib tokenizer tests do. An unknown
// state is treated as "data".
func TokenizeState(html, state string, lastStartTag *string) []Token {
	known := false
	for _, s := range States {
		known = known || s == state
	}
	if !known {
		state = "data"
	}
	return newTokenizer(html, state, lastStartTag, false).run()
}
