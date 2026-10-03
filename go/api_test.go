package lombokhtml

import (
	"errors"
	"reflect"
	"strings"
	"testing"
	"time"
)

func TestDocumentNavigation(t *testing.T) {
	doc := Parse("<div id=a><p>x</p><!--c--></div>")
	div := doc.Elements()[0]
	if !div.Is("div") || div.attrOr("id") != "a" {
		t.Fatal("div")
	}
	if _, ok := div.Attr("class"); ok {
		t.Fatal("class")
	}
	if div.Serialize() != `<div id="a"><p>x</p><!--c--></div>` || div.TextContent() != "x" || div.ExtractText() != "x" {
		t.Fatal(div.Serialize())
	}
	if len((&Node{Kind: ElementNode, Name: "b"}).elementSiblings()) != 1 || len(Parse("").Children) != 0 {
		t.Fatal("siblings")
	}
	if len(Parse(strings.Repeat("<div>", MaxDepth+10)).Elements()) != MaxDepth {
		t.Fatal("depth")
	}
}

func TestTokenizerStates(t *testing.T) {
	title := "title"
	got := TokenizeState("a</title>b", "rcdata", &title)
	want := []Token{{Kind: Character, Data: "a"}, {Kind: EndTag, Name: "title"}, {Kind: Character, Data: "b"}}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("%v", got)
	}
	if TokenizeState("<b>", "nope", nil)[0].Kind != StartTag {
		t.Fatal("unknown state falls back to data")
	}
	if d := Tokenize("<!DOCTYPE html>")[0]; d.Kind != Doctype || *d.DoctypeName != "html" || !d.Correct {
		t.Fatal("doctype")
	}
	if len(States) != 6 {
		t.Fatal("states")
	}
}

func TestPolicy(t *testing.T) {
	p := NewPolicy().AllowTag("IFRAME").AllowAttr("IFRAME", "SRC").AllowScheme("FTP")
	if got := Sanitize("<iframe src='ftp://x' srcdoc=y></iframe>", p); got != `<iframe src="ftp://x"></iframe>` {
		t.Fatal(got)
	}
	if got := Sanitize("<script>x</script><b>y</b>", nil); got != "<b>y</b>" {
		t.Fatal(got)
	}
	if !IsSafeURL("https://e.com") || IsSafeURL(" java\tscript:alert(1)") || !IsSafeURL("ftp://x", "ftp") {
		t.Fatal("urls")
	}
}

func TestSelectors(t *testing.T) {
	doc := Parse("<ul><li class=a>1<li>2</ul>")
	sel, err := CompileSelector("li.a")
	if err != nil || !sel.Matches(doc.Elements()[1]) || sel.Matches(doc) {
		t.Fatal("match")
	}
	_, err = CompileSelector("li >")
	var se *SelectorError
	if !errors.As(err, &se) || se.Code() != "BAD_SELECTOR" || se.Offset != 4 || err.Error() != "BAD_SELECTOR at character 4" {
		t.Fatal(err)
	}
	for _, bad := range []string{`[a="x]`, "li:nth-child(1", ":not(li", "li:nth-child(2n+)", "li:nth-child(2nx)", "[a^]", "li:not(x", "li:nth-child(x"} {
		if _, err := doc.Query(bad); err == nil {
			t.Errorf("%s accepted", bad)
		}
	}
	for _, good := range []string{"[class|=a]", "li:nth-child(-2n+3)", "li:nth-child(n)", "li:nth-child(-n- 0)", "* > li", "ul li ~ li"} {
		if _, err := doc.Query(good); err != nil {
			t.Errorf("%s: %v", good, err)
		}
	}
}

func TestMetaAndTables(t *testing.T) {
	m := Parse("<html lang=en><title> </title><link rel=canonical><link rel=canonical href=/c><meta content=x>").Meta()
	if m.Title != nil || m.Description != nil || *m.Canonical != "/c" || *m.Lang != "en" || len(m.OG) != 0 {
		t.Fatalf("%+v", m)
	}
	got := Parse("<table><tr><td colspan=+2 rowspan=2>a<td colspan=x>b<tr><td colspan=0>c</table>").Tables()
	if !reflect.DeepEqual(got, [][][]string{{{"a", "a", "b"}, {"a", "a", "c"}}}) {
		t.Fatal(got)
	}
	if n := len(Parse("<table><tr><td colspan=99999999999 rowspan=0>z</table>").Tables()[0][0]); n != 1000 {
		t.Fatal(n)
	}
	big := Parse("<table>" + strings.Repeat("<tr><td colspan=1000>x", 1001)).Tables()
	if len(big[0]) != 1001 || len(big[0][1000]) != 0 || MaxCells != 1000000 {
		t.Fatal("max cells")
	}
}

func TestHelpers(t *testing.T) {
	if got := DecodeEntities("&#x110000;&#0;&#xD800;&#128;&#;&bogus;&#x41"); got != "\uFFFD\uFFFD\uFFFD\u20ac&#;&bogus;A" {
		t.Fatal(got)
	}
}

func TestLinearOnLargeInputs(t *testing.T) {
	start := time.Now()
	doc := Parse("<ul>" + strings.Repeat("<li>x", 20000) + "</ul>")
	for _, sel := range []string{"li:first-child", "li ~ li", "ul li + li", "li:nth-last-child(2)"} {
		if _, err := doc.Query(sel); err != nil {
			t.Fatal(err)
		}
	}
	for _, html := range []string{strings.Repeat("a", 200000), `<a href="` + strings.Repeat("a", 200000) + `">`, "<!--" + strings.Repeat("a", 200000), strings.Repeat("1<", 100000)} {
		Tokenize(html)
	}
	HTMLToText(strings.Repeat("<b>x</b> ", 20000))
	if d := time.Since(start); d > 10*time.Second {
		t.Fatal(d)
	}
}
