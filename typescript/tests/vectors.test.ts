import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import {
    decodeEntities, escapeAttr, escapeText, htmlToText, isSafeUrl, parse, Policy, sanitize, SelectorError, stripTags,
    tokenize, tokenizeState, type InitialState, type Token,
} from '../src/index.js';

const load = (rel: string) => JSON.parse(readFileSync(fileURLToPath(new URL(rel, import.meta.url)), 'utf8'));
const doc = load('../../../vectors/lombokhtml-vectors-v1.json') as {
    cases: { id: string; kind: string; input: Record<string, any>; expected: unknown }[];
};
const conf = load('../../../conformance/html5lib-tokenizer.json') as {
    cases: { file: string; description: string; state: InitialState; input: string; output: unknown; lastStartTag?: string }[];
};

function tokenJson(t: Token): unknown {
    switch (t.type) {
        case 'StartTag': return ['StartTag', t.name, t.attrs, t.selfClosing];
        case 'EndTag': return ['EndTag', t.name];
        case 'DOCTYPE': return ['DOCTYPE', t.name, t.publicId, t.systemId, t.correct];
        default: return [t.type, t.data];
    }
}

function policy(p: { tags?: string[]; attrs?: [string, string][]; schemes?: string[] } | undefined): Policy {
    const out = new Policy();
    for (const t of p?.tags ?? []) out.allowTag(t);
    for (const [t, a] of p?.attrs ?? []) out.allowAttr(t, a);
    for (const s of p?.schemes ?? []) out.allowScheme(s);
    return out;
}

function run(kind: string, i: Record<string, any>): unknown {
    switch (kind) {
        case 'decode': return decodeEntities(i.text);
        case 'escapeText': return escapeText(i.text);
        case 'escapeAttr': return escapeAttr(i.text);
        case 'tokenize': return tokenize(i.html).map(tokenJson);
        case 'parse': return parse(i.html).serialize();
        case 'textContent': return stripTags(i.html);
        case 'extractText': return htmlToText(i.html);
        case 'sanitize': {
            const p = policy(i.policy);
            const out = sanitize(i.html, p);
            assert.equal(sanitize(out, p), out, 'sanitizer not idempotent');
            return out;
        }
        case 'safeUrl': return i.schemes ? isSafeUrl(i.url, i.schemes) : isSafeUrl(i.url);
        case 'query': return parse(i.html).query(i.selector).map((e) => e.serialize());
        case 'meta': return parse(i.html).meta();
        case 'tables': return parse(i.html).tables();
        default: throw new Error(kind);
    }
}

test('vector file has at least 100 cases', () => {
    assert.ok(doc.cases.length >= 100);
});

for (const c of doc.cases) {
    test(c.id, () => {
        let got: unknown;
        try {
            got = { ok: run(c.kind, c.input) };
        } catch (e) {
            if (!(e instanceof SelectorError)) throw e;
            got = { error: e.code };
        }
        assert.deepEqual(got, c.expected);
    });
}

test('html5lib-tests tokenizer conformance', () => {
    const failed: string[] = [];
    for (const c of conf.cases) {
        const got = tokenizeState(c.input, c.state, c.lastStartTag ?? null).map(tokenJson);
        try {
            assert.deepEqual(got, c.output);
        } catch {
            failed.push(`${c.file} ${c.description}`);
        }
    }
    assert.deepEqual(failed.slice(0, 20), []);
    assert.ok(conf.cases.length > 7000);
});
