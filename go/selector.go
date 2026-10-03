package lombokhtml

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

// SelectorError is returned for a selector that does not follow the SPEC
// grammar. Offset counts characters (runes) from the start of the selector.
type SelectorError struct {
	Offset int
}

// Code is the error code shared by every port.
func (e *SelectorError) Code() string { return "BAD_SELECTOR" }

func (e *SelectorError) Error() string { return fmt.Sprintf("BAD_SELECTOR at character %d", e.Offset) }

type simple struct {
	kind  string // id, class, attr, first-child, last-child, only-child, empty, nth-child, nth-last-child, not
	name  string
	op    string
	value string
	a, b  int
	inner *compound
}

type compound struct {
	tag    string // "" when absent
	simple []simple
}

type part struct {
	comb rune
	comp *compound
}

// Selector is a parsed selector list (SPEC section 8).
type Selector struct {
	list [][]part
}

type selParser struct {
	s []rune
	i int
}

type selFail struct{ offset int }

func (p *selParser) fail() { panic(selFail{p.i}) }

func (p *selParser) at(k int) rune {
	if p.i+k < len(p.s) {
		return p.s[p.i+k]
	}
	return eof
}

func isSelWs(c rune) bool { return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' }

func isIdent(c rune) bool {
	return isAlnum(c) || c == '_' || c == '-' || c > 0x7f
}

func (p *selParser) ws() bool {
	start := p.i
	for isSelWs(p.at(0)) {
		p.i++
	}
	return p.i > start
}

func (p *selParser) ident() string {
	start := p.i
	for p.i < len(p.s) && isIdent(p.s[p.i]) {
		p.i++
	}
	if p.i == start {
		p.fail()
	}
	return string(p.s[start:p.i])
}

func (p *selParser) list() [][]part {
	var out [][]part
	for {
		p.ws()
		out = append(out, p.complex())
		p.ws()
		if p.i >= len(p.s) {
			return out
		}
		if p.s[p.i] != ',' {
			p.fail()
		}
		p.i++
	}
}

func (p *selParser) complex() []part {
	parts := []part{{0, p.compound()}}
	for {
		save := p.i
		hadWs := p.ws()
		c := p.at(0)
		switch {
		case c == eof || c == ',':
			p.i = save
			return parts
		case c == '>' || c == '+' || c == '~':
			p.i++
			p.ws()
			parts = append(parts, part{c, p.compound()})
		case hadWs:
			parts = append(parts, part{' ', p.compound()})
		default:
			p.fail()
		}
	}
}

func (p *selParser) indexFrom(r rune, from int) int {
	for k := from; k < len(p.s); k++ {
		if p.s[k] == r {
			return k
		}
	}
	return -1
}

func (p *selParser) compound() *compound {
	comp := &compound{}
	if p.at(0) == '*' {
		p.i++
		comp.tag = "*"
	} else if p.i < len(p.s) && isIdent(p.s[p.i]) {
		comp.tag = asciiLower(p.ident())
	}
	for {
		c := p.at(0)
		switch c {
		case '#', '.':
			p.i++
			kind := "class"
			if c == '#' {
				kind = "id"
			}
			comp.simple = append(comp.simple, simple{kind: kind, value: p.ident()})
		case '[':
			p.i++
			p.ws()
			name := asciiLower(p.ident())
			p.ws()
			op := ""
			if p.at(0) == '=' {
				op = "="
				p.i++
			} else if strings.ContainsRune("~|^$*", p.at(0)) && p.at(0) != eof && p.at(1) == '=' {
				op = string(p.at(0)) + "="
				p.i += 2
			}
			val := ""
			if op != "" {
				p.ws()
				if q := p.at(0); q == '\'' || q == '"' {
					end := p.indexFrom(q, p.i+1)
					if end < 0 {
						p.fail()
					}
					val = string(p.s[p.i+1 : end])
					p.i = end + 1
				} else {
					val = p.ident()
				}
				p.ws()
			}
			if p.at(0) != ']' {
				p.fail()
			}
			p.i++
			comp.simple = append(comp.simple, simple{kind: "attr", name: name, op: op, value: val})
		case ':':
			p.i++
			name := asciiLower(p.ident())
			switch name {
			case "first-child", "last-child", "only-child", "empty":
				comp.simple = append(comp.simple, simple{kind: name})
			case "nth-child", "nth-last-child", "not":
				if p.at(0) != '(' {
					p.fail()
				}
				p.i++
				p.ws()
				var item simple
				if name == "not" {
					item = simple{kind: "not", inner: p.compound()}
					p.ws()
				} else {
					end := p.indexFrom(')', p.i)
					if end < 0 {
						p.fail()
					}
					a, b, ok := parseNth(string(p.s[p.i:end]))
					if !ok {
						p.fail()
					}
					p.i = end
					item = simple{kind: name, a: a, b: b}
				}
				if p.at(0) != ')' {
					p.fail()
				}
				p.i++
				comp.simple = append(comp.simple, item)
			default:
				p.fail()
			}
		default:
			if comp.tag == "" && len(comp.simple) == 0 {
				p.fail()
			}
			return comp
		}
	}
}

var (
	nthInt = regexp.MustCompile(`^[+-]?[0-9]{1,9}$`)
	nthExp = regexp.MustCompile(`^([+-]?[0-9]{0,9})n(?:[\t\n\f\r ]*([+-])[\t\n\f\r ]*([0-9]{1,9}))?$`)
)

func parseNth(text string) (int, int, bool) {
	t := asciiLower(strings.Trim(text, "\t\n\f\r "))
	switch t {
	case "odd":
		return 2, 1, true
	case "even":
		return 2, 0, true
	}
	if nthInt.MatchString(t) {
		b, _ := strconv.Atoi(t)
		return 0, b, true
	}
	m := nthExp.FindStringSubmatch(t)
	if m == nil {
		return 0, 0, false
	}
	a := 0
	switch m[1] {
	case "", "+":
		a = 1
	case "-":
		a = -1
	default:
		a, _ = strconv.Atoi(m[1])
	}
	b := 0
	if m[3] != "" {
		b, _ = strconv.Atoi(m[3])
		if m[2] == "-" {
			b = -b
		}
	}
	return a, b, true
}

func nthOK(a, b, pos int) bool {
	switch {
	case a == 0:
		return pos == b
	case a > 0:
		return pos >= b && (pos-b)%a == 0
	default:
		return pos <= b && (b-pos)%(-a) == 0
	}
}

func contains(list []string, s string) bool {
	for _, x := range list {
		if x == s {
			return true
		}
	}
	return false
}

// matchCtx holds per-query caches: element-sibling positions per parent,
// match results per (step, node) and the first sibling matching a step (for
// "~"). They keep matching linear in the number of siblings and in the tree
// depth; results are unchanged.
type matchCtx struct {
	sibs  map[*Node][]*Node
	pos   map[*Node]int
	memo  map[stepKey]bool
	first map[stepKey]int
}

type stepKey struct {
	step int
	node *Node
}

func newMatchCtx() *matchCtx {
	return &matchCtx{sibs: map[*Node][]*Node{}, pos: map[*Node]int{}, memo: map[stepKey]bool{}, first: map[stepKey]int{}}
}

func (c *matchCtx) siblings(el *Node) ([]*Node, int) {
	p := el.Parent
	if p == nil {
		return []*Node{el}, 0
	}
	sib, ok := c.sibs[p]
	if !ok {
		sib = el.elementSiblings()
		for i, x := range sib {
			c.pos[x] = i
		}
		c.sibs[p] = sib
	}
	return sib, c.pos[el]
}

func matchCompound(el *Node, comp *compound, ctx *matchCtx) bool {
	if comp.tag != "" && comp.tag != "*" && el.Name != comp.tag {
		return false
	}
	for _, s := range comp.simple {
		var ok bool
		switch s.kind {
		case "id":
			v, has := el.Attr("id")
			ok = has && v == s.value
		case "class":
			ok = contains(splitASCIIWs(el.attrOr("class")), s.value)
		case "attr":
			v, has := el.Attr(s.name)
			val := s.value
			switch {
			case !has:
				ok = false
			case s.op == "":
				ok = true
			case s.op == "=":
				ok = v == val
			case s.op == "~=":
				ok = contains(splitASCIIWs(v), val)
			case s.op == "|=":
				ok = v == val || strings.HasPrefix(v, val+"-")
			case s.op == "^=":
				ok = val != "" && strings.HasPrefix(v, val)
			case s.op == "$=":
				ok = val != "" && strings.HasSuffix(v, val)
			default:
				ok = val != "" && strings.Contains(v, val)
			}
		case "empty":
			ok = true
			for _, c := range el.Children {
				if c.Kind == ElementNode || c.Kind == TextNode {
					ok = false
				}
			}
		case "not":
			ok = !matchCompound(el, s.inner, ctx)
		default:
			sib, idx := ctx.siblings(el)
			pos := idx + 1
			switch s.kind {
			case "first-child":
				ok = pos == 1
			case "last-child":
				ok = pos == len(sib)
			case "only-child":
				ok = len(sib) == 1
			case "nth-child":
				ok = nthOK(s.a, s.b, pos)
			default:
				ok = nthOK(s.a, s.b, len(sib)-pos+1)
			}
		}
		if !ok {
			return false
		}
	}
	return true
}

type matcher struct {
	parts []part
	base  int // offset of this complex selector's steps in the cache keys
	ctx   *matchCtx
}

func (m *matcher) match(el *Node, k int) bool {
	key := stepKey{m.base + k, el}
	if hit, ok := m.ctx.memo[key]; ok {
		return hit
	}
	hit := m.step(el, k)
	m.ctx.memo[key] = hit
	return hit
}

func (m *matcher) step(el *Node, k int) bool {
	if !matchCompound(el, m.parts[k].comp, m.ctx) {
		return false
	}
	if k == 0 {
		return true
	}
	switch m.parts[k].comb {
	case '>':
		p := el.Parent
		return p != nil && p.Kind == ElementNode && m.match(p, k-1)
	case ' ':
		for p := el.Parent; p != nil && p.Kind == ElementNode; p = p.Parent {
			if m.match(p, k-1) {
				return true
			}
		}
		return false
	}
	sib, idx := m.ctx.siblings(el)
	if m.parts[k].comb == '+' {
		return idx > 0 && m.match(sib[idx-1], k-1)
	}
	// "~": some earlier sibling matches step k-1; remember the first such sibling per parent.
	fkey := stepKey{m.base + k, el.Parent}
	first, ok := m.ctx.first[fkey]
	if !ok {
		first = len(sib)
		for i, x := range sib {
			if m.match(x, k-1) {
				first = i
				break
			}
		}
		m.ctx.first[fkey] = first
	}
	return first < idx
}

// CompileSelector parses selector (SPEC section 8.1).
func CompileSelector(selector string) (sel *Selector, err error) {
	p := &selParser{s: []rune(selector)}
	defer func() {
		if r := recover(); r != nil {
			f, ok := r.(selFail)
			if !ok {
				panic(r)
			}
			sel, err = nil, &SelectorError{Offset: f.offset}
		}
	}()
	return &Selector{list: p.list()}, nil
}

// Matches reports whether el is an element matching the selector.
func (s *Selector) Matches(el *Node) bool {
	return s.matchesIn(el, newMatchCtx())
}

func (s *Selector) matchesIn(el *Node, ctx *matchCtx) bool {
	if el.Kind != ElementNode {
		return false
	}
	base := 0
	for _, parts := range s.list {
		m := matcher{parts: parts, base: base, ctx: ctx}
		if m.match(el, len(parts)-1) {
			return true
		}
		base += len(parts)
	}
	return false
}

// Query returns the elements below n matching selector in document order (SPEC section 8.2).
func (n *Node) Query(selector string) ([]*Node, error) {
	sel, err := CompileSelector(selector)
	if err != nil {
		return nil, err
	}
	out := []*Node{}
	ctx := newMatchCtx()
	for _, el := range n.Elements() {
		if sel.matchesIn(el, ctx) {
			out = append(out, el)
		}
	}
	return out, nil
}
