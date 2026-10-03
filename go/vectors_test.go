package lombokhtml

import (
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"testing"
)

type vcase struct {
	ID       string          `json:"id"`
	Kind     string          `json:"kind"`
	Input    map[string]any  `json:"input"`
	Expected json.RawMessage `json:"expected"`
}

func tokenJSON(t Token) any {
	switch t.Kind {
	case StartTag:
		attrs := []any{}
		for _, a := range t.Attrs {
			attrs = append(attrs, []any{a.Name, a.Value})
		}
		return []any{"StartTag", t.Name, attrs, t.SelfClosing}
	case EndTag:
		return []any{"EndTag", t.Name}
	case Doctype:
		return []any{"DOCTYPE", t.DoctypeName, t.PublicID, t.SystemID, t.Correct}
	default:
		return []any{string(t.Kind), t.Data}
	}
}

func tokensJSON(toks []Token) []any {
	out := []any{}
	for _, t := range toks {
		out = append(out, tokenJSON(t))
	}
	return out
}

func strList(v any) []string {
	out := []string{}
	if v == nil {
		return out
	}
	for _, x := range v.([]any) {
		out = append(out, x.(string))
	}
	return out
}

func policyFrom(v any) *Policy {
	p := NewPolicy()
	m, _ := v.(map[string]any)
	for _, t := range strList(m["tags"]) {
		p.AllowTag(t)
	}
	if attrs, ok := m["attrs"].([]any); ok {
		for _, pair := range attrs {
			a := pair.([]any)
			p.AllowAttr(a[0].(string), a[1].(string))
		}
	}
	for _, s := range strList(m["schemes"]) {
		p.AllowScheme(s)
	}
	return p
}

func runCase(t *testing.T, kind string, i map[string]any) (any, error) {
	s := func(k string) string { v, _ := i[k].(string); return v }
	switch kind {
	case "decode":
		return DecodeEntities(s("text")), nil
	case "escapeText":
		return EscapeText(s("text")), nil
	case "escapeAttr":
		return EscapeAttr(s("text")), nil
	case "tokenize":
		return tokensJSON(Tokenize(s("html"))), nil
	case "parse":
		return Parse(s("html")).Serialize(), nil
	case "textContent":
		return StripTags(s("html")), nil
	case "extractText":
		return HTMLToText(s("html")), nil
	case "sanitize":
		p := policyFrom(i["policy"])
		out := Sanitize(s("html"), p)
		if again := Sanitize(out, p); again != out {
			t.Errorf("sanitizer not idempotent: %q -> %q", out, again)
		}
		return out, nil
	case "safeUrl":
		if sc, ok := i["schemes"]; ok {
			return IsSafeURL(s("url"), strList(sc)...), nil
		}
		return IsSafeURL(s("url")), nil
	case "query":
		els, err := Parse(s("html")).Query(s("selector"))
		if err != nil {
			return nil, err
		}
		out := []string{}
		for _, e := range els {
			out = append(out, e.Serialize())
		}
		return out, nil
	case "meta":
		m := Parse(s("html")).Meta()
		og := []any{}
		for _, p := range m.OG {
			og = append(og, []any{p[0], p[1]})
		}
		return map[string]any{"title": m.Title, "description": m.Description, "canonical": m.Canonical, "lang": m.Lang, "og": og}, nil
	case "tables":
		return Parse(s("html")).Tables(), nil
	}
	t.Fatalf("unknown kind %s", kind)
	return nil, nil
}

func normalize(v any) any {
	b, _ := json.Marshal(v)
	var out any
	_ = json.Unmarshal(b, &out)
	return out
}

func TestVectors(t *testing.T) {
	raw, err := os.ReadFile("../vectors/lombokhtml-vectors-v1.json")
	if err != nil {
		t.Fatal(err)
	}
	var doc struct{ Cases []vcase }
	if err := json.Unmarshal(raw, &doc); err != nil {
		t.Fatal(err)
	}
	if len(doc.Cases) < 100 {
		t.Fatalf("only %d cases", len(doc.Cases))
	}
	for _, c := range doc.Cases {
		got, err := runCase(t, c.Kind, c.Input)
		var res any
		if err != nil {
			var se *SelectorError
			if !errors.As(err, &se) {
				t.Fatalf("%s: %v", c.ID, err)
			}
			res = map[string]any{"error": se.Code()}
		} else {
			res = map[string]any{"ok": got}
		}
		var want any
		_ = json.Unmarshal(c.Expected, &want)
		if !reflect.DeepEqual(normalize(res), want) {
			t.Errorf("%s: got %v want %v", c.ID, normalize(res), want)
		}
	}
}

func TestConformance(t *testing.T) {
	raw, err := os.ReadFile("../conformance/html5lib-tokenizer.json")
	if err != nil {
		t.Fatal(err)
	}
	var doc struct {
		Cases []struct {
			File         string  `json:"file"`
			Description  string  `json:"description"`
			State        string  `json:"state"`
			Input        string  `json:"input"`
			Output       any     `json:"output"`
			LastStartTag *string `json:"lastStartTag"`
		}
	}
	if err := json.Unmarshal(raw, &doc); err != nil {
		t.Fatal(err)
	}
	failed := 0
	for _, c := range doc.Cases {
		got := normalize(tokensJSON(TokenizeState(c.Input, c.State, c.LastStartTag)))
		if !reflect.DeepEqual(got, c.Output) {
			failed++
			if failed <= 20 {
				t.Errorf("%s %s: got %v want %v", c.File, c.Description, got, c.Output)
			}
		}
	}
	if len(doc.Cases) < 7000 {
		t.Fatalf("only %d conformance cases", len(doc.Cases))
	}
}
