<?php

declare(strict_types=1);

// Public API checks beyond the shared vectors.

require_once __DIR__ . '/bootstrap.php';

use LombokHTML\Html;
use LombokHTML\Node;
use LombokHTML\Policy;
use LombokHTML\Selector;
use LombokHTML\SelectorError;

$doc = Html::parse('<div id=a><p>x</p><!--c--></div>');
$div = $doc->elements()[0];
check($div->is('div') && $div->attr('id') === 'a' && $div->attr('class') === null, 'div attrs');
check($div->serialize() === '<div id="a"><p>x</p><!--c--></div>', 'serialize');
check($div->textContent() === 'x' && $div->extractText() === 'x', 'text');
check(count((new Node('element', 'b'))->elementSiblings()) === 1, 'detached siblings');
check(Html::parse('')->children === [], 'empty');
check(count(Html::parse(str_repeat('<div>', Html::MAX_DEPTH + 10))->elements()) === Html::MAX_DEPTH, 'depth');

check(Html::tokenizeState('a</title>b', 'rcdata', 'title') === [['Character', 'a'], ['EndTag', 'title'], ['Character', 'b']], 'rcdata');
check(Html::tokenizeState('a]]>b', 'cdata') === [['Character', 'ab']], 'cdata');
check(Html::tokenize('<!DOCTYPE html>') === [['DOCTYPE', 'html', null, null, true]], 'doctype');
try {
    Html::tokenizeState('x', 'nope');
    check(false, 'unknown state accepted');
} catch (\InvalidArgumentException) {
    check(true, 'unknown state');
}
check(Html::tokenize("a\xFFb") === [['Character', "a\u{FFFD}b"]], 'ill-formed UTF-8 byte becomes U+FFFD');

$p = (new Policy())->allowTag('IFRAME')->allowAttr('IFRAME', 'SRC')->allowScheme('FTP');
check(Html::sanitize("<iframe src='ftp://x' srcdoc=y></iframe>", $p) === '<iframe src="ftp://x"></iframe>', 'policy');
check(Html::sanitize('<script>x</script><b>y</b>') === '<b>y</b>', 'default policy');
check(Html::isSafeUrl('https://e.com') && !Html::isSafeUrl(" java\tscript:alert(1)") && Html::isSafeUrl('ftp://x', ['ftp']), 'urls');

$ul = Html::parse('<ul><li class=a>1<li>2</ul>');
$sel = Selector::parse('li.a');
check($sel->matches($ul->elements()[1]) && !$sel->matches($ul), 'matches');
try {
    Selector::parse('li >');
    check(false, 'bad selector accepted');
} catch (SelectorError $e) {
    check($e->errorCode === 'BAD_SELECTOR' && $e->offset === 4 && $e->getMessage() === 'BAD_SELECTOR at character 4', 'selector error');
}
foreach (['[a="x]', 'li:nth-child(1', ':not(li', 'li:nth-child(2n+)', 'li:nth-child(2nx)', '[a^]', 'li:not(x'] as $bad) {
    try {
        $ul->query($bad);
        check(false, "$bad accepted");
    } catch (SelectorError) {
        check(true, $bad);
    }
}
check(count($ul->query('li:nth-last-child(-n+1)')) === 1 && count($ul->query('ul li ~ li')) === 1, 'combinators');
foreach (['[class|=a]', 'li:nth-child(-2n+3)', 'li:nth-child(n)', 'li:nth-child(-n- 0)', '* > li'] as $good) {
    $ul->query($good);
}

$m = Html::parse('<html lang=en><title> </title><link rel=canonical><link rel=canonical href=/c><meta content=x>')->meta();
check($m === ['title' => null, 'description' => null, 'canonical' => '/c', 'lang' => 'en', 'og' => []], 'meta');
check(Html::parse('<table><tr><td colspan=+2 rowspan=2>a<td colspan=x>b<tr><td colspan=0>c</table>')->tables()
    === [[['a', 'a', 'b'], ['a', 'a', 'c']]], 'spans');
check(count(Html::parse('<table><tr><td colspan=99999999999 rowspan=0>z</table>')->tables()[0][0]) === 1000, 'colspan cap');
check(Html::parse('<table><tr></table>')->tables() === [[[]]], 'empty row');
$big = Html::parse('<table>' . str_repeat('<tr><td colspan=1000>x', 1001))->tables();
check(count($big[0]) === 1001 && $big[0][1000] === [], 'max cells');
check(Html::decodeEntities('&#x110000;&#0;&#xD800;&#128;&#;&bogus;') === "\u{FFFD}\u{FFFD}\u{FFFD}\u{20AC}&#;&bogus;", 'decode');

$start = microtime(true);
$big = Html::parse('<ul>' . str_repeat('<li>x', 20000) . '</ul>');
foreach (['li:first-child', 'li ~ li', 'ul li + li', 'li:nth-last-child(2)'] as $sel) {
    $big->query($sel);
}
foreach ([str_repeat('a', 200000), '<a href="' . str_repeat('a', 200000) . '">', '<!--' . str_repeat('a', 200000), str_repeat('1<', 100000)] as $html) {
    Html::tokenize($html);
}
Html::htmlToText(str_repeat('<b>x</b> ', 20000));
check(microtime(true) - $start < 10, 'linear on large inputs');

finish('api');
