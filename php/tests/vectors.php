<?php

declare(strict_types=1);

// Runs the shared vectors (vectors/lombokhtml-vectors-v1.json) and the
// html5lib-tests tokenizer conformance file.

require_once __DIR__ . '/bootstrap.php';

use LombokHTML\Html;
use LombokHTML\Policy;
use LombokHTML\SelectorError;

$doc = json_decode((string) file_get_contents(__DIR__ . '/../../vectors/lombokhtml-vectors-v1.json'), true, 512, JSON_THROW_ON_ERROR);
$conf = json_decode((string) file_get_contents(__DIR__ . '/../../conformance/html5lib-tokenizer.json'), true, 512, JSON_THROW_ON_ERROR);

function policyFrom(array $p): Policy
{
    $out = new Policy();
    foreach ($p['tags'] ?? [] as $t) {
        $out->allowTag($t);
    }
    foreach ($p['attrs'] ?? [] as [$t, $a]) {
        $out->allowAttr($t, $a);
    }
    foreach ($p['schemes'] ?? [] as $s) {
        $out->allowScheme($s);
    }
    return $out;
}

function runCase(string $kind, array $i): mixed
{
    return match ($kind) {
        'decode' => Html::decodeEntities($i['text']),
        'escapeText' => Html::escapeText($i['text']),
        'escapeAttr' => Html::escapeAttr($i['text']),
        'tokenize' => Html::tokenize($i['html']),
        'parse' => Html::parse($i['html'])->serialize(),
        'textContent' => Html::stripTags($i['html']),
        'extractText' => Html::htmlToText($i['html']),
        'sanitize' => (function () use ($i) {
            $p = policyFrom($i['policy'] ?? []);
            $out = Html::sanitize($i['html'], $p);
            check(Html::sanitize($out, $p) === $out, "sanitizer not idempotent for {$i['html']}");
            return $out;
        })(),
        'safeUrl' => isset($i['schemes']) ? Html::isSafeUrl($i['url'], $i['schemes']) : Html::isSafeUrl($i['url']),
        'query' => array_map(static fn ($e) => $e->serialize(), Html::parse($i['html'])->query($i['selector'])),
        'meta' => Html::parse($i['html'])->meta(),
        'tables' => Html::parse($i['html'])->tables(),
    };
}

check(count($doc['cases']) >= 100, 'at least 100 vector cases');
foreach ($doc['cases'] as $c) {
    try {
        $got = ['ok' => runCase($c['kind'], $c['input'])];
    } catch (SelectorError $e) {
        $got = ['error' => $e->errorCode];
    }
    $a = json_encode($got, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
    $b = json_encode($c['expected'], JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
    check($a === $b, "{$c['id']}: got $a want $b");
}

check(count($conf['cases']) > 7000, 'conformance case count');
foreach ($conf['cases'] as $c) {
    $got = Html::tokenizeState($c['input'], $c['state'], $c['lastStartTag'] ?? null);
    $a = json_encode($got, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
    $b = json_encode($c['output'], JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
    check($a === $b, "{$c['file']} {$c['description']}: got $a want $b");
}

finish('vectors');
