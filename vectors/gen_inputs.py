#!/usr/bin/env python3
"""Author vector INPUTS for LombokHTML. Expected outputs are filled by the Rust reference:
`LOMBOK_REGEN=1 cargo test --test vectors` (in rust/). Deterministic (fixed seed)."""
import json, os, random
C = []
def add(fn, inp, **kw): C.append({"fn": fn, "in": inp, **kw})

DOCS = [
 "", "plain text", "<p>Hello</p>", "<div><p>Hello <b>world</b></p></div>", "<p>a</p><p>b</p>", "<ul><li>one<li>two</ul>",
 "<a href=\"https://x.com\" target='_blank' checked>Go</a>", "<a href=x.com/a?b=1&amp;c=2>unquoted</a>", "<a title=\"x>y\" href='a>b'>t</a>",
 "<br><img src=\"a.png\" alt=a><hr/>", "<input type=text value=\"v\" disabled>", "<b><i>text</b></i>", "<p>hello</span>world</p>", "<div><p>unterminated",
 "<!DOCTYPE html><html><head><title>T</title></head><body><h1>H</h1></body></html>", "<!-- c --><p>x</p><!--unclosed", "<script>if (1 < 2) { alert('<b>') }</script>after",
 "<style>p{color:red}</style><p>x</p>", "5 < 10 and 20 > 10", "<p>text<un", "<", "<>", "</>", "<//>", "<a", "<a b", "<a b=", "<a b='", "<a b=\"x", "<1a>x</1a>", "<p/>x", "<div/>y</div>",
 "&amp; &lt; &gt; &quot; &apos; &nbsp; &copy; &#65; &#x41; &#X41; &#0; &#xD800; &#x110000; &bogus; &amp &", "<p title=\"a&amp;b&#34;c\">x</p>", "<script>a &lt; b</script>", "<textarea>&lt;b&gt;</textarea>",
 "<h1>Title</h1><p>Body text</p>", "<h2>Sub</h2><ul><li>A</li><li>B</li></ul><p>End</p>", "<table><tr><td>A</td><td>B</td></tr></table>", "<p>line<br>break</p>", "<div>a</div><div>b</div>",
 "<head><title>  My  Page </title><meta name=\"Description\" content=\"A great page.\"><meta property=\"og:title\" content=\"OG\"><meta property=\"og:image\" content=\"https://x/i.png\"></head>",
 "<title>a</title><title>b</title>", "<meta name=description content=first><meta name=description content=second>", "<meta property=\"og:\" content=\"empty\">", "<meta property=\"og:a\">",
 "<table><tr><th>Name</th><th>Age</th></tr><tr><td>Alice</td><td>30</td></tr></table>", "<table><thead><tr><th>A</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table>",
 "<table><tr><td colspan=2>M</td><td>C</td></tr></table>", "<table><tr><td rowspan=2>A</td><td>B</td></tr><tr><td>C</td></tr></table>",
 "<table><tr><td>1</td><td rowspan=2>X</td><td>3</td></tr><tr><td>a</td><td>c</td></tr></table>", "<table><tr><td colspan=2 rowspan=2>Z</td><td>1</td></tr><tr><td>2</td></tr></table>",
 "<table><tr><td colspan=4000000000>x</td></tr></table>", "<table><tr><td rowspan=99999999999>x</td></tr></table>", "<table><tr><td colspan=0>z</td><td colspan=-1>n</td><td colspan=abc>t</td></tr></table>",
 "<table><tr><td>outer<table><tr><td>inner</td></tr></table></td></tr></table>", "<table><tr><td>Hello <b>world</b></td></tr></table>", "<table></table>", "<table><tr></tr></table>", "<tr><td>orphan</td></tr>",
 "<div id=main class=\"card x\"><p class=card>a</p><span class=y>b</span></div><p>c</p>", "<a href=\"https://x.com/a.pdf\" rel=\"nofollow noopener\">l</a><a href=\"/b\">m</a>",
 "<div><section><p>deep</p></section><p>direct</p></div><p>outside</p>", "<h1>a</h1><p>b</p><h2>c</h2>", "<div class=a><div class=b><span>x</span></div></div>",
 "<p>日本語 <b>é</b> 😀</p>", "<p>\u0000null</p>", "<P CLASS=UP>UPPER</P>", "<p a=1 a=2>dup</p>",
 "<svg><script>alert(1)</script></svg>x", "<math><mi>x</mi></math>y", "<iframe src=x>fallback</iframe>z", "<object data=x>o</object>", "<embed src=x>after", "<template><p>t</p></template>", "<noscript><p>n</p></noscript>",
 "<form action=x><input name=a><button>go</button></form>", "<select><option>o</option></select>", "<video src=x>v</video><audio src=y>a</audio>",
]
XSS = [
 "<script>alert(1)</script>", "<img src=x onerror=alert(1)>", "<img src=\"javascript:alert(1)\">", "<a href=\"javascript:alert(1)\">x</a>", "<a href=\" JaVaScRiPt:alert(1)\">x</a>", "<a href=\"java\tscript:alert(1)\">x</a>",
 "<a href=\"java\nscript:alert(1)\">x</a>", "<a href=\"&#106;avascript:alert(1)\">x</a>", "<a href=\"&#x6A;avascript:alert(1)\">x</a>", "<a href=\"jav&#x09;ascript:alert(1)\">x</a>", "<a href=\"data:text/html,<script>alert(1)</script>\">x</a>",
 "<a href=\"vbscript:msgbox(1)\">x</a>", "<a href=\"//evil.com/x\">proto-relative</a>", "<a href=\"mailto:a@b.co\">m</a>", "<a href=\"tel:+123\">t</a>", "<a href=\"ftp://x\">f</a>", "<a href=\"#frag\">f</a>", "<a href=\"?q=a:b\">q</a>", "<a href=\"page.html?x=a:b\">p</a>",
 "<body onload=alert(1)>x</body>", "<div onmouseover=\"alert(1)\">x</div>", "<p style=\"background:url(javascript:alert(1))\">x</p>", "<p class=x id=y style=z>k</p>", "<svg onload=alert(1)>", "<svg><a xlink:href=\"javascript:alert(1)\">x</a></svg>",
 "<iframe src=\"javascript:alert(1)\"></iframe>", "<iframe srcdoc=\"<script>alert(1)</script>\"></iframe>", "<object data=\"javascript:alert(1)\"></object>", "<embed src=\"javascript:alert(1)\">", "<base href=\"javascript:alert(1)//\">",
 "<link rel=stylesheet href=\"javascript:alert(1)\">", "<meta http-equiv=\"refresh\" content=\"0;url=javascript:alert(1)\">", "<form action=\"javascript:alert(1)\"><button>x</button></form>", "<button formaction=\"javascript:alert(1)\">x</button>",
 "<math><mtext><table><mglyph><style><img src=x onerror=alert(1)>", "<noscript><p title=\"</noscript><img src=x onerror=alert(1)>\">", "<textarea></textarea><img src=x onerror=alert(1)>", "<title></title><img src=x onerror=alert(1)>",
 "<p title=\"a\" onclick=\"x\" title=\"b\">dup attrs</p>", "<p title='a\" onmouseover=\"x'>quote break</p>", "<p title=a&quot;b>entity quote</p>", "<img src=x alt=\"&lt;script&gt;\">", "&lt;script&gt;alert(1)&lt;/script&gt;",
 "<scr<script>ipt>alert(1)</scr</script>ipt>", "<<script>script>alert(1)<</script>/script>", "<img src=\"x\"onerror=\"alert(1)\">", "<img/src=x/onerror=alert(1)>", "<a href=\"javascript&colon;alert(1)\">x</a>", "<a href=\"\u0001javascript:alert(1)\">x</a>",
 "<p>a</p><script>1</script><p>b</p>", "<div><script>1</script></div>", "<A HREF=\"JAVASCRIPT:alert(1)\">X</A>", "<IMG SRC=x ONERROR=alert(1)>", "<details open ontoggle=alert(1)>x</details>",
]
for d in DOCS + XSS:
    for fn in ("parse","strip_tags","extract_structured_text","sanitize","extract_meta","extract_tables"):
        add(fn, d)
for d in ["&#+65;","&#x+41;","&#-65;","&#065;","&#x0041;","&amp; &lt;b&gt; &#65; &#x1F600; &nbsp;|&copy;|&bogus;|&amp","","&","&;","&#;","&#x;","&#99999999999;","&#65","AT&T","&AMP;","&Amp;","x&amp;amp;y","&lt;&lt;","&#128512;","&#xFFFF;","&#x10FFFF;","&#1114112;","&#-1;","&#x-1;","&# 65;"]:
    add("decode_entities", d)
for u in ["https://e.com","HTTP://E.COM","/rel","rel/path","#f","?q=1","","  ","javascript:x"," javascript:x","JaVa\tScRiPt:x","java\u0000script:x","mailto:a@b.co","tel:1","ftp://x","data:image/png;base64,AAAA","//host/x","a:b","a/b:c","x?y:z","x#y:z","http:","\u00a0javascript:x","javascript&colon;x"]:
    add("is_safe_url", u)
SELS = ["p","div p","div > p","div>p","div > section > p",".card","#main","div.card#main","*","[href]","[href^=https]","[href$='.pdf']","[href*=x.com]","[rel~=noopener]","[href='/b']","h2, h1",".a span",".a > span",".a > .b > span","","> p","p >",",",", p","[","[a","[a=","a[b^=]","#",".","p p","div div","P","input[type=text]","span.y","[class~=x]"]
for d in ["<div id=main class=\"card x\"><p class=card>a</p><span class=y>b</span></div><p>c</p>","<a href=\"https://x.com/a.pdf\" rel=\"nofollow noopener\">l</a><a href=\"/b\">m</a>","<div><section><p>deep</p></section><p>direct</p></div><p>outside</p>","<h1>a</h1><p>b</p><h2>c</h2>","<div class=a><div class=b><span>x</span></div></div>","<input type=text><input type=checkbox checked>","<div><div><p>x</p></div><p>y</p></div>"]:
    for s in SELS: add("query", {"html": d, "selector": s})

rng = random.Random(20260928)
PIECES = ["<",">","</","/>","<!--","-->","<p>","</p>","<div>","</div>","<b>","</b>","<a href=\"","javascript:x","\"","'","="," ","\n","<script>","</script>","<style>","</style>","<table>","<tr>","<td colspan=99>","<td rowspan=3>","</td>","</tr>","</table>","onclick=","<img src=x ","&amp;","&#0;","&lt;","&","&#","é","日","<svg>","</svg>","<iframe src=","<li>","<ul>","</ul>","<h1>","</h1>","text","<br>","<title>","<meta property=\"og:x\" content=\"y\">","class=","id=","[","]","#","."]
def soup(k): return "".join(rng.choice(PIECES) for _ in range(rng.randint(0,k)))
for _ in range(400):
    d = soup(28)
    for fn in ("parse","strip_tags","sanitize","extract_tables","extract_structured_text"): add(fn, d)
json.dump({"suite":"lombokhtml","version":1,"cases":C}, open(os.path.join(os.path.dirname(os.path.abspath(__file__)),"lombokhtml-vectors-v1.json"),"w",encoding="utf-8"), ensure_ascii=False)
print(len(C),"cases")
