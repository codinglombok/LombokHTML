<?php

declare(strict_types=1);

namespace LombokHTML;

/**
 * LombokHTML: WHATWG HTML tokenizer, a small tree builder, serialization, text
 * extraction, an allowlist sanitizer, a CSS selector subset, page metadata and
 * table extraction, with byte-identical results in PHP, Rust, TypeScript,
 * Python and Go (see docs/SPEC_LombokHTML_v0.2.0.md). No dependencies.
 */
final class Html
{
    /** Maximum number of open elements; deeper start tags are ignored. */
    public const MAX_DEPTH = 256;
    /** Most cells produced for one table; extraction stops there. */
    public const MAX_CELLS = 1000000;

    public const VOID = ['area' => 1, 'base' => 1, 'basefont' => 1, 'bgsound' => 1, 'br' => 1, 'col' => 1, 'embed' => 1,
        'frame' => 1, 'hr' => 1, 'img' => 1, 'input' => 1, 'keygen' => 1, 'link' => 1, 'meta' => 1, 'param' => 1,
        'source' => 1, 'track' => 1, 'wbr' => 1];
    public const HIDDEN = ['script' => 1, 'style' => 1, 'noscript' => 1, 'template' => 1, 'title' => 1];
    private const RAW_PARENTS = ['style' => 1, 'script' => 1, 'xmp' => 1, 'iframe' => 1, 'noembed' => 1, 'noframes' => 1,
        'plaintext' => 1, 'noscript' => 1];
    private const HEADINGS = ['h1' => 1, 'h2' => 1, 'h3' => 1, 'h4' => 1, 'h5' => 1, 'h6' => 1];
    private const SPECIAL = 'address applet area article aside base basefont bgsound blockquote body br button caption center col '
        . 'colgroup dd details dir div dl dt embed fieldset figcaption figure footer form frame frameset h1 h2 h3 h4 h5 h6 head '
        . 'header hgroup hr html iframe img input keygen li link listing main marquee menu meta nav noembed noframes noscript '
        . 'object ol p param plaintext pre script search section select source style summary table tbody td template textarea '
        . 'tfoot th thead title tr track ul wbr xmp';
    private const P_CLOSERS = 'address article aside blockquote center details dialog dir div dl fieldset figcaption figure '
        . 'footer form h1 h2 h3 h4 h5 h6 header hgroup hr li dd dt listing main menu nav ol p plaintext pre search section '
        . 'summary table ul xmp';
    private const BLOCKS = 'address article aside blockquote caption details dialog div dl fieldset figcaption figure footer '
        . 'form header hgroup main nav ol p pre section summary table ul';
    private const SCOPE = 'applet caption html table td th marquee object template';
    private const URL_ATTRS = ['cite' => 1, 'href' => 1, 'src' => 1];

    /** @var array<string, array<string, int>> */
    private static array $sets = [];

    /** @return array<string, int> */
    private static function set(string $words): array
    {
        return self::$sets[$words] ??= array_fill_keys(explode(' ', $words), 1);
    }

    // ------------------------------------------------------------ tokenizer

    /**
     * Tokenizes $html as the parser does: start tags of title, textarea, style,
     * xmp, iframe, noembed, noframes, noscript, script and plaintext switch the
     * tokenizer state (SPEC section 2.2). See Tokenizer for the token form.
     *
     * @return list<array<int, mixed>>
     */
    public static function tokenize(string $html): array
    {
        return Tokenizer::run($html, 'data', null, true);
    }

    /**
     * Tokenizes $html from $state (one of Tokenizer::STATES) without element-driven switching.
     *
     * @return list<array<int, mixed>>
     */
    public static function tokenizeState(string $html, string $state = 'data', ?string $lastStartTag = null): array
    {
        if (!in_array($state, Tokenizer::STATES, true)) {
            throw new \InvalidArgumentException("unknown tokenizer state '$state'");
        }
        return Tokenizer::run($html, $state, $lastStartTag, false);
    }

    // ------------------------------------------------------------ entities

    /** Decodes character references as in text content (SPEC section 3.1). */
    public static function decodeEntities(string $text): string
    {
        $s = Tokenizer::chars($text);
        $n = count($s);
        $out = '';
        $i = 0;
        while ($i < $n) {
            if ($s[$i] !== '&') {
                $out .= $s[$i++];
                continue;
            }
            $j = $i + 1;
            if ($j < $n && $s[$j] === '#') {
                $k = $j + 1;
                $hex = $k < $n && ($s[$k] === 'x' || $s[$k] === 'X');
                if ($hex) {
                    $k++;
                }
                $start = $k;
                $code = 0;
                while ($k < $n && strlen($s[$k]) === 1 && ($hex ? ctype_xdigit($s[$k]) : ctype_digit($s[$k]))) {
                    $code = min($code * ($hex ? 16 : 10) + (int) hexdec($s[$k]), 0x110000);
                    $k++;
                }
                if ($k === $start) {
                    $out .= implode('', array_slice($s, $i, $k - $i));
                    $i = $k;
                    continue;
                }
                if ($k < $n && $s[$k] === ';') {
                    $k++;
                }
                $out .= Tokenizer::numericChar($code);
                $i = $k;
                continue;
            }
            $k = $j;
            while ($k < $n && Tokenizer::isAlnum($s[$k]) && $k - $j < 32) {
                $k++;
            }
            $run = implode('', array_slice($s, $j, $k - $j));
            $match = null;
            if ($k < $n && $s[$k] === ';' && isset(Entities::TABLE[$run . ';'])) {
                $match = $run . ';';
            } else {
                for ($m = strlen($run); $m > 0; $m--) {
                    if (isset(Entities::TABLE[substr($run, 0, $m)])) {
                        $match = substr($run, 0, $m);
                        break;
                    }
                }
            }
            if ($match === null) {
                $out .= '&' . $run;
                $i = $k;
            } else {
                $out .= Entities::TABLE[$match];
                $i = $j + strlen($match);
            }
        }
        return $out;
    }

    /** Escapes text content: &, U+00A0, <, > (SPEC section 3.2). */
    public static function escapeText(string $s): string
    {
        return strtr($s, ['&' => '&amp;', "\u{A0}" => '&nbsp;', '<' => '&lt;', '>' => '&gt;']);
    }

    /** Escapes an attribute value: &, U+00A0, ", <, > (SPEC section 3.2). */
    public static function escapeAttr(string $s): string
    {
        return strtr($s, ['&' => '&amp;', "\u{A0}" => '&nbsp;', '"' => '&quot;', '<' => '&lt;', '>' => '&gt;']);
    }

    // ------------------------------------------------------------ tree

    /** Parses $html into a document node (SPEC section 4). Never throws. */
    public static function parse(string $html): Node
    {
        $doc = new Node('document');
        /** @var list<Node> $stack */
        $stack = [];
        $special = self::set(self::SPECIAL);
        $scope = self::set(self::SCOPE);
        $buttonScope = $scope + ['button' => 1];
        $listScope = $scope + ['ol' => 1, 'ul' => 1];
        $tableScope = self::set('html table template');
        $sections = self::set('thead tbody tfoot');

        $current = static function () use (&$stack, $doc): Node {
            return $stack === [] ? $doc : $stack[count($stack) - 1];
        };
        $append = static function (Node $node) use ($current): void {
            $p = $current();
            $node->parent = $p;
            $p->children[] = $node;
        };
        $inScope = static function (array $names, array $boundary) use (&$stack): bool {
            for ($i = count($stack) - 1; $i >= 0; $i--) {
                if (isset($names[$stack[$i]->name])) {
                    return true;
                }
                if (isset($boundary[$stack[$i]->name])) {
                    return false;
                }
            }
            return false;
        };
        $popUntil = static function (array $names) use (&$stack): void {
            while ($stack !== []) {
                if (isset($names[array_pop($stack)->name])) {
                    return;
                }
            }
        };
        $top = static function () use (&$stack): string {
            return $stack === [] ? '' : $stack[count($stack) - 1]->name;
        };
        $start = static function (string $name, array $attrs) use (
            &$stack, $special, $buttonScope, $tableScope, $sections, $inScope, $popUntil, $top, $append
        ): bool {
            if ($name === 'li' || $name === 'dd' || $name === 'dt') {
                $targets = $name === 'li' ? ['li' => 1] : ['dd' => 1, 'dt' => 1];
                for ($i = count($stack) - 1; $i >= 0; $i--) {
                    $cur = $stack[$i]->name;
                    if (isset($targets[$cur])) {
                        array_splice($stack, $i);
                        break;
                    }
                    if (isset($special[$cur]) && $cur !== 'address' && $cur !== 'div' && $cur !== 'p') {
                        break;
                    }
                }
            }
            if (isset(self::set(self::P_CLOSERS)[$name]) && $inScope(['p' => 1], $buttonScope)) {
                $popUntil(['p' => 1]);
            }
            if (isset(self::HEADINGS[$name]) && isset(self::HEADINGS[$top()])) {
                array_pop($stack);
            }
            if (($name === 'option' || $name === 'optgroup') && $top() === 'option') {
                array_pop($stack);
            }
            if ($name === 'a') {
                foreach ($stack as $n) {
                    if ($n->name === 'a') {
                        $popUntil(['a' => 1]);
                        break;
                    }
                }
            }
            if (isset(self::set('td th tr thead tbody tfoot')[$name]) && $inScope(['td' => 1, 'th' => 1], $tableScope)) {
                $popUntil(['td' => 1, 'th' => 1]);
            }
            if (isset(self::set('tr thead tbody tfoot')[$name]) && $inScope(['tr' => 1], $tableScope)) {
                $popUntil(['tr' => 1]);
            }
            if (isset($sections[$name]) && $inScope($sections, $tableScope)) {
                $popUntil($sections);
            }
            if (count($stack) >= self::MAX_DEPTH) {
                return false;
            }
            $el = new Node('element', $name, $attrs);
            $append($el);
            if (!isset(self::VOID[$name])) {
                $stack[] = $el;
            }
            return $name === 'pre' || $name === 'listing' || $name === 'textarea';
        };
        $end = static function (string $name) use (
            &$stack, $special, $scope, $buttonScope, $listScope, $tableScope, $inScope, $popUntil, $start
        ): void {
            if ($name === 'br') {
                $start('br', []);
                return;
            }
            if ($name === 'p') {
                [$target, $boundary] = [['p' => 1], $buttonScope];
            } elseif (isset(self::HEADINGS[$name])) {
                [$target, $boundary] = [self::HEADINGS, $scope];
            } elseif ($name === 'li') {
                [$target, $boundary] = [['li' => 1], $listScope];
            } elseif (isset(self::set('table caption tbody thead tfoot tr td th')[$name])) {
                [$target, $boundary] = [[$name => 1], $tableScope];
            } elseif ($name === 'dd' || $name === 'dt' || isset($special[$name])) {
                [$target, $boundary] = [[$name => 1], $scope];
            } else {
                for ($i = count($stack) - 1; $i >= 0; $i--) {
                    $cur = $stack[$i]->name;
                    if ($cur === $name) {
                        array_splice($stack, $i);
                        return;
                    }
                    if (isset($special[$cur])) {
                        return;
                    }
                }
                return;
            }
            if ($inScope($target, $boundary)) {
                $popUntil($target);
            }
        };

        $skipNewline = false;
        foreach (self::tokenize($html) as $tok) {
            if ($tok[0] === 'Character') {
                $data = $tok[1];
                if ($skipNewline && str_starts_with($data, "\n")) {
                    $data = substr($data, 1);
                }
                $skipNewline = false;
                if ($data !== '') {
                    $p = $current();
                    $last = $p->children === [] ? null : $p->children[count($p->children) - 1];
                    if ($last !== null && $last->kind === 'text') {
                        $last->data .= $data;
                    } else {
                        $append(new Node('text', '', [], $data));
                    }
                }
                continue;
            }
            $skipNewline = false;
            if ($tok[0] === 'StartTag') {
                $skipNewline = $start($tok[1], $tok[2]);
            } elseif ($tok[0] === 'EndTag') {
                $end($tok[1]);
            } elseif ($tok[0] === 'Comment') {
                $append(new Node('comment', '', [], $tok[1]));
            }
        }
        return $doc;
    }

    /**
     * @internal
     * @param list<string> $out
     */
    public static function write(Node $n, array &$out): void
    {
        if ($n->kind === 'text') {
            $raw = $n->parent !== null && $n->parent->kind === 'element' && isset(self::RAW_PARENTS[$n->parent->name]);
            $out[] = $raw ? $n->data : self::escapeText($n->data);
        } elseif ($n->kind === 'comment') {
            $out[] = '<!--' . $n->data . '-->';
        } elseif ($n->kind === 'element') {
            $out[] = '<' . $n->name;
            foreach ($n->attrs as [$k, $v]) {
                $out[] = ' ' . $k . '="' . self::escapeAttr($v) . '"';
            }
            $out[] = '>';
            if (isset(self::VOID[$n->name])) {
                return;
            }
            foreach ($n->children as $c) {
                self::write($c, $out);
            }
            $out[] = '</' . $n->name . '>';
        } else {
            foreach ($n->children as $c) {
                self::write($c, $out);
            }
        }
    }

    // ------------------------------------------------------------ text

    /** @internal */
    public static function extract(Node $node): string
    {
        $out = '';
        $blocks = self::set(self::BLOCKS);
        $walk = static function (Node $n, bool $pre) use (&$walk, &$out, $blocks): void {
            if ($n->kind === 'text') {
                if ($pre) {
                    $out .= $n->data;
                    return;
                }
                $collapsed = (string) preg_replace('/[\t\n\f\r ]+/', ' ', $n->data);
                $last = substr($out, -1);
                if (str_starts_with($collapsed, ' ') && ($last === '' || $last === ' ' || $last === "\n" || $last === "\t")) {
                    $collapsed = substr($collapsed, 1);
                }
                $out .= $collapsed;
                return;
            }
            if ($n->kind === 'comment') {
                return;
            }
            if ($n->kind === 'document') {
                foreach ($n->children as $c) {
                    $walk($c, $pre);
                }
                return;
            }
            $name = $n->name;
            if (isset(self::HIDDEN[$name])) {
                return;
            }
            if (isset(self::HEADINGS[$name])) {
                $out .= "\n\n" . str_repeat('#', (int) $name[1]) . ' ';
                foreach ($n->children as $c) {
                    $walk($c, $pre);
                }
                $out .= "\n\n";
                return;
            }
            if ($name === 'li') {
                $out .= "\n- ";
            } elseif ($name === 'dd' || $name === 'dt' || $name === 'tr') {
                $out .= "\n";
            } elseif ($name === 'br') {
                $out .= "\n";
                return;
            } elseif ($name === 'hr') {
                $out .= "\n\n---\n\n";
                return;
            }
            $block = isset($blocks[$name]);
            if ($block) {
                $out .= "\n\n";
            }
            $inner = $pre || $name === 'pre' || $name === 'listing' || $name === 'textarea';
            foreach ($n->children as $c) {
                $walk($c, $inner);
            }
            if ($name === 'td' || $name === 'th') {
                $out .= "\t";
            }
            if ($block) {
                $out .= "\n\n";
            }
        };
        $walk($node, false);
        $lines = array_map(static fn (string $l): string => rtrim($l, " \t"), explode("\n", $out));
        $text = (string) preg_replace('/\n{3,}/', "\n\n", implode("\n", $lines));
        return trim($text, "\n");
    }

    /** Parses $html and returns its structure-preserving plain text. */
    public static function htmlToText(string $html): string
    {
        return self::parse($html)->extractText();
    }

    /** Parses $html and returns its text content without markup. */
    public static function stripTags(string $html): string
    {
        return self::parse($html)->textContent();
    }

    // ------------------------------------------------------------ sanitizer

    /**
     * True when $url has no scheme or one of $schemes (lowercase), after removing
     * C0 controls, space and DEL (SPEC section 7.3).
     *
     * @param list<string> $schemes
     */
    public static function isSafeUrl(string $url, array $schemes = Policy::DEFAULT_SCHEMES): bool
    {
        $cleaned = Tokenizer::lower((string) preg_replace('/[\x00-\x20\x7F]/', '', $url));
        $i = strcspn($cleaned, '/?#:');
        if ($i === strlen($cleaned) || $cleaned[$i] !== ':') {
            return true;
        }
        return in_array(substr($cleaned, 0, $i), $schemes, true);
    }

    /** Sanitizes $html (SPEC section 7). The result is a fixed point. */
    public static function sanitize(string $html, ?Policy $policy = null): string
    {
        $p = $policy ?? new Policy();
        $schemes = $p->schemes();
        $out = [];
        $walk = static function (Node $n) use (&$walk, &$out, $p, $schemes): void {
            if ($n->kind === 'text') {
                $out[] = self::escapeText($n->data);
                return;
            }
            if ($n->kind === 'comment') {
                return;
            }
            if ($n->kind === 'element') {
                $action = $p->action($n->name);
                if ($action === 'drop') {
                    return;
                }
                if ($action === 'keep') {
                    $out[] = '<' . $n->name;
                    foreach ($n->attrs as [$k, $v]) {
                        if (!$p->attrOk($n->name, $k)) {
                            continue;
                        }
                        if (isset(self::URL_ATTRS[$k]) && !self::isSafeUrl($v, $schemes)) {
                            continue;
                        }
                        $out[] = ' ' . $k . '="' . self::escapeAttr($v) . '"';
                    }
                    $out[] = '>';
                    if (isset(self::VOID[$n->name])) {
                        return;
                    }
                    // Raw text content would not survive a second pass, so it is dropped.
                    if (!isset(self::RAW_PARENTS[$n->name])) {
                        foreach ($n->children as $c) {
                            $walk($c);
                        }
                    }
                    $out[] = '</' . $n->name . '>';
                    return;
                }
            }
            foreach ($n->children as $c) {
                $walk($c);
            }
        };
        $walk(self::parse($html));
        return implode('', $out);
    }

    // ------------------------------------------------------------ meta and tables

    private static function collapse(string $s): string
    {
        return trim((string) preg_replace('/[\t\n\f\r ]+/', ' ', $s), ' ');
    }

    /** @return list<string> */
    public static function splitWs(string $s): array
    {
        return preg_split('/[\t\n\f\r ]+/', $s, -1, PREG_SPLIT_NO_EMPTY) ?: [];
    }

    /**
     * @internal
     * @return array{title: ?string, description: ?string, canonical: ?string, lang: ?string, og: list<array{0: string, 1: string}>}
     */
    public static function meta(Node $doc): array
    {
        $out = ['title' => null, 'description' => null, 'canonical' => null, 'lang' => null, 'og' => []];
        $seenTitle = false;
        foreach ($doc->elements() as $el) {
            $name = $el->name;
            if ($name === 'title' && !$seenTitle) {
                $seenTitle = true;
                $raw = '';
                foreach ($el->children as $c) {
                    if ($c->kind === 'text') {
                        $raw .= $c->data;
                    }
                }
                $t = self::collapse($raw);
                $out['title'] = $t === '' ? null : $t;
            } elseif ($name === 'html') {
                $out['lang'] ??= $el->attr('lang');
            } elseif ($name === 'meta') {
                $content = $el->attr('content');
                if ($content === null) {
                    continue;
                }
                if (Tokenizer::lower($el->attr('name') ?? '') === 'description' && $out['description'] === null) {
                    $out['description'] = $content;
                }
                $prop = $el->attr('property') ?? '';
                if (Tokenizer::lower(substr($prop, 0, 3)) === 'og:') {
                    $out['og'][] = [substr($prop, 3), $content];
                }
            } elseif ($name === 'link' && $out['canonical'] === null) {
                if (in_array('canonical', self::splitWs(Tokenizer::lower($el->attr('rel') ?? '')), true)) {
                    $out['canonical'] = $el->attr('href');
                }
            }
        }
        return $out;
    }

    private static function parseSpan(?string $v, int $default, int $maximum): int
    {
        if ($v === null || preg_match('/^[\t\n\f\r ]*\+?([0-9]+)/', $v, $m) !== 1) {
            return $default;
        }
        $n = strlen($m[1]) <= 9 ? (int) $m[1] : $maximum;
        return $n === 0 ? $default : min($n, $maximum);
    }

    /** @param list<Node> $rows */
    private static function collectRows(Node $n, array &$rows): void
    {
        foreach ($n->children as $c) {
            if ($c->kind !== 'element' || $c->name === 'table') {
                continue;
            }
            if ($c->name === 'tr') {
                $rows[] = $c;
            } else {
                self::collectRows($c, $rows);
            }
        }
    }

    /**
     * @internal
     * @return list<list<list<string>>>
     */
    public static function tables(Node $doc): array
    {
        $tables = [];
        foreach ($doc->elements() as $table) {
            if ($table->name !== 'table') {
                continue;
            }
            $rows = [];
            self::collectRows($table, $rows);
            $grid = [];
            $pending = [];
            $total = 0;
            $stop = false;
            foreach ($rows as $tr) {
                $row = [];
                $next = [];
                foreach ($pending as $col => [$left, $text]) {
                    $row[$col] = $text;
                    if ($left > 1) {
                        $next[$col] = [$left - 1, $text];
                    }
                }
                $col = 0;
                foreach ($tr->children as $cell) {
                    if (!$cell->is('td') && !$cell->is('th')) {
                        continue;
                    }
                    $text = self::collapse($cell->textContent());
                    $cs = self::parseSpan($cell->attr('colspan'), 1, 1000);
                    $rs = self::parseSpan($cell->attr('rowspan'), 1, 65534);
                    for ($x = 0; $x < $cs; $x++) {
                        while (isset($row[$col])) {
                            $col++;
                        }
                        if (++$total > self::MAX_CELLS) {
                            $stop = true;
                            break;
                        }
                        $row[$col] = $text;
                        if ($rs > 1) {
                            $next[$col] = [$rs - 1, $text];
                        }
                        $col++;
                    }
                    if ($stop) {
                        break;
                    }
                }
                $pending = $next;
                $width = $row === [] ? 0 : max(array_keys($row)) + 1;
                $line = [];
                for ($c = 0; $c < $width; $c++) {
                    $line[] = $row[$c] ?? '';
                }
                $grid[] = $line;
                if ($stop) {
                    break;
                }
            }
            $tables[] = $grid;
        }
        return $tables;
    }
}
