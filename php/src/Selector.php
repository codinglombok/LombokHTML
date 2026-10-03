<?php

declare(strict_types=1);

namespace LombokHTML;

/** A parsed selector list (SPEC section 8). */
final class Selector
{
    /** @var list<string> */
    private array $s;
    private int $i = 0;

    /** @param list<list<array{0: string, 1: array<string, mixed>}>> $list */
    private function __construct(private array $list = [])
    {
    }

    /** Parses $selector (SPEC section 8.1). */
    public static function parse(string $selector): self
    {
        $p = new self();
        $p->s = Tokenizer::chars($selector);
        $p->list = $p->parseList();
        unset($p->s);
        return $p;
    }

    /**
     * Per-query caches: element-sibling lists and positions per parent, match
     * results per (step, node) and the first sibling matching a step (for "~").
     * They keep matching linear in the number of siblings and in the tree depth;
     * results are unchanged.
     *
     * @var array{sibs: array<int, list<Node>>, pos: array<int, int>, memo: array<string, bool>, first: array<string, int>}
     */
    private array $ctx = ['sibs' => [], 'pos' => [], 'memo' => [], 'first' => []];
    private int $base = 0;

    /** True when $el is an element that matches. */
    public function matches(Node $el): bool
    {
        $this->reset();
        return $this->matchesCached($el);
    }

    /** @internal Starts a new query; cached results are only valid for one tree state. */
    public function reset(): void
    {
        $this->ctx = ['sibs' => [], 'pos' => [], 'memo' => [], 'first' => []];
    }

    /** @internal */
    public function matchesCached(Node $el): bool
    {
        if ($el->kind !== 'element') {
            return false;
        }
        $this->base = 0;
        foreach ($this->list as $parts) {
            if ($this->matchComplex($el, $parts, count($parts) - 1)) {
                return true;
            }
            $this->base += count($parts);
        }
        return false;
    }

    /** @return array{0: list<Node>, 1: int} */
    private function siblings(Node $el): array
    {
        $p = $el->parent;
        if ($p === null) {
            return [[$el], 0];
        }
        $pid = spl_object_id($p);
        if (!isset($this->ctx['sibs'][$pid])) {
            $sib = $el->elementSiblings();
            foreach ($sib as $i => $x) {
                $this->ctx['pos'][spl_object_id($x)] = $i;
            }
            $this->ctx['sibs'][$pid] = $sib;
        }
        return [$this->ctx['sibs'][$pid], $this->ctx['pos'][spl_object_id($el)]];
    }

    // ------------------------------------------------------------ parser

    private function fail(): never
    {
        throw new SelectorError($this->i);
    }

    private function at(int $k = 0): string
    {
        return $this->s[$this->i + $k] ?? '';
    }

    private function ws(): bool
    {
        $start = $this->i;
        while (in_array($this->at(), [' ', "\t", "\n", "\r", "\f"], true)) {
            $this->i++;
        }
        return $this->i > $start;
    }

    private static function isIdent(string $c): bool
    {
        return $c !== '' && (strlen($c) > 1 || ctype_alnum($c) || $c === '_' || $c === '-');
    }

    private function ident(): string
    {
        $start = $this->i;
        while (self::isIdent($this->at())) {
            $this->i++;
        }
        if ($this->i === $start) {
            $this->fail();
        }
        return implode('', array_slice($this->s, $start, $this->i - $start));
    }

    private function indexFrom(string $c, int $from): int
    {
        for ($k = $from, $n = count($this->s); $k < $n; $k++) {
            if ($this->s[$k] === $c) {
                return $k;
            }
        }
        return -1;
    }

    /** @return list<list<array{0: string, 1: array<string, mixed>}>> */
    private function parseList(): array
    {
        $out = [];
        while (true) {
            $this->ws();
            $out[] = $this->complex();
            $this->ws();
            if ($this->i >= count($this->s)) {
                return $out;
            }
            if ($this->at() !== ',') {
                $this->fail();
            }
            $this->i++;
        }
    }

    /** @return list<array{0: string, 1: array<string, mixed>}> */
    private function complex(): array
    {
        $parts = [['', $this->compound()]];
        while (true) {
            $save = $this->i;
            $hadWs = $this->ws();
            $c = $this->at();
            if ($c === '' || $c === ',') {
                $this->i = $save;
                return $parts;
            }
            if ($c === '>' || $c === '+' || $c === '~') {
                $this->i++;
                $this->ws();
                $parts[] = [$c, $this->compound()];
            } elseif ($hadWs) {
                $parts[] = [' ', $this->compound()];
            } else {
                $this->fail();
            }
        }
    }

    /** @return array{tag: ?string, simple: list<array<int, mixed>>} */
    private function compound(): array
    {
        $tag = null;
        $simple = [];
        if ($this->at() === '*') {
            $this->i++;
            $tag = '*';
        } elseif (self::isIdent($this->at())) {
            $tag = Tokenizer::lower($this->ident());
        }
        while (true) {
            $c = $this->at();
            if ($c === '#' || $c === '.') {
                $this->i++;
                $simple[] = [$c === '#' ? 'id' : 'class', $this->ident()];
            } elseif ($c === '[') {
                $this->i++;
                $this->ws();
                $name = Tokenizer::lower($this->ident());
                $this->ws();
                $op = null;
                $val = '';
                if ($this->at() === '=') {
                    $op = '=';
                    $this->i++;
                } elseif (in_array($this->at(), ['~', '|', '^', '$', '*'], true) && $this->at(1) === '=') {
                    $op = $this->at() . '=';
                    $this->i += 2;
                }
                if ($op !== null) {
                    $this->ws();
                    $q = $this->at();
                    if ($q === "'" || $q === '"') {
                        $end = $this->indexFrom($q, $this->i + 1);
                        if ($end < 0) {
                            $this->fail();
                        }
                        $val = implode('', array_slice($this->s, $this->i + 1, $end - $this->i - 1));
                        $this->i = $end + 1;
                    } else {
                        $val = $this->ident();
                    }
                    $this->ws();
                }
                if ($this->at() !== ']') {
                    $this->fail();
                }
                $this->i++;
                $simple[] = ['attr', $name, $op, $val];
            } elseif ($c === ':') {
                $this->i++;
                $name = Tokenizer::lower($this->ident());
                if (in_array($name, ['first-child', 'last-child', 'only-child', 'empty'], true)) {
                    $simple[] = [$name];
                } elseif (in_array($name, ['nth-child', 'nth-last-child', 'not'], true)) {
                    if ($this->at() !== '(') {
                        $this->fail();
                    }
                    $this->i++;
                    $this->ws();
                    if ($name === 'not') {
                        $item = ['not', $this->compound()];
                        $this->ws();
                    } else {
                        $end = $this->indexFrom(')', $this->i);
                        if ($end < 0) {
                            $this->fail();
                        }
                        $ab = self::parseNth(implode('', array_slice($this->s, $this->i, $end - $this->i)));
                        if ($ab === null) {
                            $this->fail();
                        }
                        $this->i = $end;
                        $item = [$name, $ab[0], $ab[1]];
                    }
                    if ($this->at() !== ')') {
                        $this->fail();
                    }
                    $this->i++;
                    $simple[] = $item;
                } else {
                    $this->fail();
                }
            } else {
                break;
            }
        }
        if ($tag === null && $simple === []) {
            $this->fail();
        }
        return ['tag' => $tag, 'simple' => $simple];
    }

    /** @return array{0: int, 1: int}|null */
    private static function parseNth(string $text): ?array
    {
        $t = Tokenizer::lower(trim($text, "\t\n\f\r "));
        if ($t === 'odd') {
            return [2, 1];
        }
        if ($t === 'even') {
            return [2, 0];
        }
        if (preg_match('/^[+-]?[0-9]{1,9}$/D', $t) === 1) {
            return [0, (int) $t];
        }
        if (preg_match('/^([+-]?[0-9]{0,9})n(?:[\t\n\f\r ]*([+-])[\t\n\f\r ]*([0-9]{1,9}))?$/D', $t, $m) !== 1) {
            return null;
        }
        $a = match ($m[1]) {
            '', '+' => 1,
            '-' => -1,
            default => (int) $m[1],
        };
        $b = isset($m[3]) && $m[3] !== '' ? (int) $m[3] * ($m[2] === '-' ? -1 : 1) : 0;
        return [$a, $b];
    }

    // ------------------------------------------------------------ matching

    private static function nthOk(int $a, int $b, int $pos): bool
    {
        if ($a === 0) {
            return $pos === $b;
        }
        if ($a > 0) {
            return $pos >= $b && ($pos - $b) % $a === 0;
        }
        return $pos <= $b && ($b - $pos) % (-$a) === 0;
    }

    /** @param array{tag: ?string, simple: list<array<int, mixed>>} $comp */
    private function matchCompound(Node $el, array $comp): bool
    {
        if ($comp['tag'] !== null && $comp['tag'] !== '*' && $el->name !== $comp['tag']) {
            return false;
        }
        foreach ($comp['simple'] as $s) {
            $ok = match ($s[0]) {
                'id' => $el->attr('id') === $s[1],
                'class' => in_array($s[1], Html::splitWs($el->attr('class') ?? ''), true),
                'attr' => self::matchAttr($el->attr($s[1]), $s[2], $s[3]),
                'empty' => self::isEmpty($el),
                'not' => !$this->matchCompound($el, $s[1]),
                default => $this->matchPosition($el, $s),
            };
            if (!$ok) {
                return false;
            }
        }
        return true;
    }

    private static function matchAttr(?string $v, ?string $op, string $val): bool
    {
        if ($v === null) {
            return false;
        }
        return match ($op) {
            null => true,
            '=' => $v === $val,
            '~=' => in_array($val, Html::splitWs($v), true),
            '|=' => $v === $val || str_starts_with($v, $val . '-'),
            '^=' => $val !== '' && str_starts_with($v, $val),
            '$=' => $val !== '' && str_ends_with($v, $val),
            default => $val !== '' && str_contains($v, $val),
        };
    }

    private static function isEmpty(Node $el): bool
    {
        foreach ($el->children as $c) {
            if ($c->kind === 'element' || $c->kind === 'text') {
                return false;
            }
        }
        return true;
    }

    /** @param array<int, mixed> $s */
    private function matchPosition(Node $el, array $s): bool
    {
        [$sib, $idx] = $this->siblings($el);
        $pos = $idx + 1;
        return match ($s[0]) {
            'first-child' => $pos === 1,
            'last-child' => $pos === count($sib),
            'only-child' => count($sib) === 1,
            'nth-child' => self::nthOk($s[1], $s[2], $pos),
            default => self::nthOk($s[1], $s[2], count($sib) - $pos + 1),
        };
    }

    /** @param list<array{0: string, 1: array<string, mixed>}> $parts */
    private function matchComplex(Node $el, array $parts, int $k): bool
    {
        $key = ($this->base + $k) . ':' . spl_object_id($el);
        return $this->ctx['memo'][$key] ??= $this->matchStep($el, $parts, $k);
    }

    /** @param list<array{0: string, 1: array<string, mixed>}> $parts */
    private function matchStep(Node $el, array $parts, int $k): bool
    {
        [$comb, $comp] = $parts[$k];
        if (!$this->matchCompound($el, $comp)) {
            return false;
        }
        if ($k === 0) {
            return true;
        }
        if ($comb === '>') {
            $p = $el->parent;
            return $p !== null && $p->kind === 'element' && $this->matchComplex($p, $parts, $k - 1);
        }
        if ($comb === ' ') {
            for ($p = $el->parent; $p !== null && $p->kind === 'element'; $p = $p->parent) {
                if ($this->matchComplex($p, $parts, $k - 1)) {
                    return true;
                }
            }
            return false;
        }
        [$sib, $idx] = $this->siblings($el);
        if ($comb === '+') {
            return $idx > 0 && $this->matchComplex($sib[$idx - 1], $parts, $k - 1);
        }
        // "~": some earlier sibling matches step k-1; remember the first such sibling per parent.
        $fkey = ($this->base + $k) . ':' . spl_object_id($el->parent);
        if (!isset($this->ctx['first'][$fkey])) {
            $first = count($sib);
            foreach ($sib as $i => $x) {
                if ($this->matchComplex($x, $parts, $k - 1)) {
                    $first = $i;
                    break;
                }
            }
            $this->ctx['first'][$fkey] = $first;
        }
        return $this->ctx['first'][$fkey] < $idx;
    }
}
