import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
    DEFAULT_SCHEMES, decodeEntities, HtmlNode, htmlToText, isSafeUrl, MAX_CELLS, MAX_DEPTH, parse, Policy, sanitize, Selector,
    SelectorError, tokenize, tokenizeState,
} from '../src/index.js';

test('document navigation', () => {
    const doc = parse('<div id=a><p>x</p><!--c--></div>');
    const [div] = doc.elements();
    assert.ok(div.is('div'));
    assert.equal(div.attr('id'), 'a');
    assert.equal(div.attr('class'), null);
    assert.equal(div.serialize(), '<div id="a"><p>x</p><!--c--></div>');
    assert.equal(div.textContent(), 'x');
    assert.equal(div.extractText(), 'x');
    assert.deepEqual(new HtmlNode('element', 'b').elementSiblings().length, 1);
    assert.equal(parse('').children.length, 0);
});

test('depth limit', () => {
    assert.equal(parse('<div>'.repeat(MAX_DEPTH + 10)).elements().length, MAX_DEPTH);
});

test('tokenizer states', () => {
    assert.deepEqual(tokenizeState('a</title>b', 'rcdata', 'title'), [
        { type: 'Character', data: 'a' }, { type: 'EndTag', name: 'title' }, { type: 'Character', data: 'b' },
    ]);
    assert.deepEqual(tokenizeState('<x>', 'plaintext'), [{ type: 'Character', data: '<x>' }]);
    assert.deepEqual(tokenizeState('a]]>b', 'cdata'), [{ type: 'Character', data: 'ab' }]);
    assert.deepEqual(tokenize('<!DOCTYPE html>'), [
        { type: 'DOCTYPE', name: 'html', publicId: null, systemId: null, correct: true },
    ]);
});

test('policy builder', () => {
    const p = new Policy().allowTag('IFRAME').allowAttr('IFRAME', 'SRC').allowScheme('FTP');
    assert.equal(sanitize("<iframe src='ftp://x' srcdoc=y></iframe>", p), '<iframe src="ftp://x"></iframe>');
    assert.equal(sanitize('<script>x</script><b>y</b>'), '<b>y</b>');
    assert.ok(isSafeUrl('https://e.com'));
    assert.ok(!isSafeUrl(' java\tscript:alert(1)'));
    assert.ok(isSafeUrl('ftp://x', ['ftp']));
    assert.equal(DEFAULT_SCHEMES.length, 4);
});

test('selectors', () => {
    const doc = parse('<ul><li class=a>1<li>2</ul>');
    const sel = Selector.parse('li.a');
    assert.ok(sel.matches(doc.elements()[1]));
    assert.ok(!sel.matches(doc));
    assert.throws(() => Selector.parse('li >'), (e: unknown) => {
        assert.ok(e instanceof SelectorError);
        assert.equal(e.code, 'BAD_SELECTOR');
        assert.equal(e.offset, 4);
        assert.equal(e.message, 'BAD_SELECTOR at character 4');
        return true;
    });
    assert.equal(doc.query('li:nth-last-child(-n+1)').length, 1);
    assert.equal(doc.query('ul li ~ li').length, 1);
    for (const bad of ['[a="x]', 'li:nth-child(1', ':not(li', 'li:nth-child(2n+)', 'li:nth-child(2nx)', '[a^]', 'li:not(x']) {
        assert.throws(() => doc.query(bad), SelectorError, bad);
    }
    for (const good of ['[class|=a]', 'li:nth-child(-2n+3)', 'li:nth-child(n)', 'li:nth-child(-n- 0)', '* > li']) {
        doc.query(good);
    }
});

test('meta and tables', () => {
    const m = parse('<html lang=en><title> </title><link rel=canonical><link rel=canonical href=/c><meta content=x>').meta();
    assert.deepEqual(m, { title: null, description: null, canonical: '/c', lang: 'en', og: [] });
    assert.deepEqual(parse('<table><tr><td colspan=+2 rowspan=2>a<td colspan=x>b<tr><td colspan=0>c</table>').tables(),
        [[['a', 'a', 'b'], ['a', 'a', 'c']]]);
    assert.equal(parse('<table><tr><td colspan=99999999999 rowspan=0>z</table>').tables()[0][0].length, 1000);
    assert.deepEqual(parse('<table><tr></table>').tables(), [[[]]]);
    assert.equal(MAX_CELLS, 1_000_000);
    const t = parse('<table>' + '<tr><td colspan=1000>x'.repeat(1001)).tables();
    assert.equal(t[0].length, 1001);
    assert.equal(t[0][1000].length, 0);
});

test('helpers', () => {
    assert.equal(decodeEntities('&#x110000;&#0;&#xD800;&#128;&#;&bogus;'), '���€&#;&bogus;');
});

test('linear on large inputs', () => {
    const start = Date.now();
    const doc = parse('<ul>' + '<li>x'.repeat(20000) + '</ul>');
    for (const sel of ['li:first-child', 'li ~ li', 'ul li + li', 'li:nth-last-child(2)']) doc.query(sel);
    for (const html of ['a'.repeat(200000), '<a href="' + 'a'.repeat(200000) + '">', '<!--' + 'a'.repeat(200000), '1<'.repeat(100000)]) {
        tokenize(html);
    }
    htmlToText('<b>x</b> '.repeat(20000));
    assert.ok(Date.now() - start < 10000, `${Date.now() - start} ms`);
});
