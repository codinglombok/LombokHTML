<?php

declare(strict_types=1);

namespace LombokHTML;

/**
 * WHATWG HTML tokenizer (SPEC section 2), for HTML content (no foreign content).
 *
 * Tokens are arrays: ['StartTag', name, [[attr, value], ...], selfClosing],
 * ['EndTag', name], ['Character', data], ['Comment', data],
 * ['DOCTYPE', name|null, publicId|null, systemId|null, correct].
 *
 * @internal Use Html::tokenize() and Html::tokenizeState().
 */
final class Tokenizer
{
    public const STATES = ['data', 'rcdata', 'rawtext', 'script', 'plaintext', 'cdata'];
    private const MAX_ENTITY = 32;
    private const R = "\u{FFFD}";
    private const REPLACEMENTS = [
        0x80 => 0x20AC, 0x82 => 0x201A, 0x83 => 0x0192, 0x84 => 0x201E, 0x85 => 0x2026, 0x86 => 0x2020, 0x87 => 0x2021,
        0x88 => 0x02C6, 0x89 => 0x2030, 0x8A => 0x0160, 0x8B => 0x2039, 0x8C => 0x0152, 0x8E => 0x017D, 0x91 => 0x2018,
        0x92 => 0x2019, 0x93 => 0x201C, 0x94 => 0x201D, 0x95 => 0x2022, 0x96 => 0x2013, 0x97 => 0x2014, 0x98 => 0x02DC,
        0x99 => 0x2122, 0x9A => 0x0161, 0x9B => 0x203A, 0x9C => 0x0153, 0x9E => 0x017E, 0x9F => 0x0178,
    ];
    private const RAWTEXT = ['style' => 1, 'xmp' => 1, 'iframe' => 1, 'noembed' => 1, 'noframes' => 1, 'noscript' => 1];

    /** @var list<string> */
    private array $s;
    private int $n;
    private int $i = 0;
    private string $state;
    private string $ret = 'data';
    /** @var list<array<int, mixed>> */
    private array $out = [];
    /** @var array{0: bool, 1: string, 2: list<array{0: string, 1: string}>, 3: bool}|null */
    private ?array $tag = null;
    private int $attrIndex = -1;
    private string $temp = '';
    private string $comment = '';
    /** @var array{0: ?string, 1: ?string, 2: ?string, 3: bool} */
    private array $doctype = [null, null, null, false];
    private int $code = 0;

    private function __construct(string $text, string $state, private ?string $lastStart, private bool $switching)
    {
        $this->s = self::chars(str_replace(["\r\n", "\r"], "\n", $text));
        $this->n = count($this->s);
        $this->state = $state;
    }

    /**
     * Splits UTF-8 text into characters; each byte of an ill-formed sequence becomes U+FFFD.
     *
     * @return list<string>
     */
    public static function chars(string $text): array
    {
        if (preg_match('//u', $text) === 1) {
            return preg_split('//u', $text, -1, PREG_SPLIT_NO_EMPTY) ?: [];
        }
        preg_match_all('/[\x00-\x7F]|[\xC2-\xDF][\x80-\xBF]|\xE0[\xA0-\xBF][\x80-\xBF]|[\xE1-\xEC\xEE\xEF][\x80-\xBF]{2}'
            . '|\xED[\x80-\x9F][\x80-\xBF]|\xF0[\x90-\xBF][\x80-\xBF]{2}|[\xF1-\xF3][\x80-\xBF]{3}|\xF4[\x80-\x8F][\x80-\xBF]{2}|[\x80-\xFF]/s', $text, $m);
        $out = [];
        foreach ($m[0] as $c) {
            $out[] = strlen($c) === 1 && ord($c) >= 0x80 ? self::R : $c;
        }
        return $out;
    }

    /** Encodes a code point as UTF-8. */
    public static function utf8(int $cp): string
    {
        if ($cp < 0x80) {
            return chr($cp);
        }
        if ($cp < 0x800) {
            return chr(0xC0 | ($cp >> 6)) . chr(0x80 | ($cp & 0x3F));
        }
        if ($cp < 0x10000) {
            return chr(0xE0 | ($cp >> 12)) . chr(0x80 | (($cp >> 6) & 0x3F)) . chr(0x80 | ($cp & 0x3F));
        }
        return chr(0xF0 | ($cp >> 18)) . chr(0x80 | (($cp >> 12) & 0x3F)) . chr(0x80 | (($cp >> 6) & 0x3F)) . chr(0x80 | ($cp & 0x3F));
    }

    public static function numericChar(int $code): string
    {
        if ($code === 0 || $code > 0x10FFFF || ($code >= 0xD800 && $code <= 0xDFFF)) {
            return self::R;
        }
        return self::utf8(self::REPLACEMENTS[$code] ?? $code);
    }

    public static function lower(string $s): string
    {
        return strtr($s, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz');
    }

    public static function isAlnum(string $c): bool
    {
        return strlen($c) === 1 && ctype_alnum($c);
    }

    private static function isAlpha(string $c): bool
    {
        return strlen($c) === 1 && ctype_alpha($c);
    }

    private static function isWs(string $c): bool
    {
        return $c === "\t" || $c === "\n" || $c === "\f" || $c === ' ';
    }

    private static function orR(string $c): string
    {
        return $c === "\0" ? self::R : $c;
    }

    /** @return list<array<int, mixed>> */
    public static function run(string $text, string $state, ?string $lastStart, bool $switching): array
    {
        $t = new self($text, $state, $lastStart, $switching);
        while (true) {
            $c = $t->i < $t->n ? $t->s[$t->i] : '';
            $t->i++;
            if (!$t->step($c)) {
                return $t->out;
            }
        }
    }

    private function emit(string $s): void
    {
        $last = count($this->out) - 1;
        if ($last >= 0 && $this->out[$last][0] === 'Character') {
            $this->out[$last][1] .= $s;
        } else {
            $this->out[] = ['Character', $s];
        }
    }

    private function emitTag(): void
    {
        [$end, $name, $attrs, $sc] = $this->tag;
        if ($end) {
            $this->out[] = ['EndTag', $name];
        } else {
            $seen = [];
            $kept = [];
            foreach ($attrs as [$k, $v]) {
                if (!isset($seen[$k])) {
                    $seen[$k] = true;
                    $kept[] = [$k, $v];
                }
            }
            $this->out[] = ['StartTag', $name, $kept, $sc];
            $this->lastStart = $name;
            if ($this->switching) {
                if ($name === 'title' || $name === 'textarea') {
                    $this->state = 'rcdata';
                } elseif (isset(self::RAWTEXT[$name])) {
                    $this->state = 'rawtext';
                } elseif ($name === 'script') {
                    $this->state = 'script';
                } elseif ($name === 'plaintext') {
                    $this->state = 'plaintext';
                }
            }
        }
        $this->tag = null;
    }

    private function appropriate(): bool
    {
        return $this->tag !== null && $this->tag[0] && $this->tag[1] === $this->lastStart;
    }

    private function startAttr(): void
    {
        $this->tag[2][] = ['', ''];
        $this->attrIndex = count($this->tag[2]) - 1;
    }

    private function addName(string $s): void
    {
        $this->tag[2][$this->attrIndex][0] .= $s;
    }

    private function addValue(string $s): void
    {
        $this->tag[2][$this->attrIndex][1] .= $s;
    }

    private function inAttr(): bool
    {
        return $this->ret === 'attr_dq' || $this->ret === 'attr_sq' || $this->ret === 'attr_unq';
    }

    private function flushRef(): void
    {
        if ($this->inAttr()) {
            $this->addValue($this->temp);
        } else {
            $this->emit($this->temp);
        }
    }

    private function reconsume(string $state): void
    {
        $this->i--;
        $this->state = $state;
    }

    private function peek(int $len): string
    {
        return implode('', array_slice($this->s, $this->i, $len));
    }

    private function emitComment(): void
    {
        $this->out[] = ['Comment', $this->comment];
    }

    private function emitDoctype(bool $quirks = false): void
    {
        if ($quirks) {
            $this->doctype[3] = true;
        }
        $d = $this->doctype;
        $this->out[] = ['DOCTYPE', $d[0], $d[1], $d[2], !$d[3]];
    }

    private function lt(string $c, string $base, string $openState): void
    {
        if ($c === '/') {
            $this->temp = '';
            $this->state = $openState;
        } else {
            $this->emit('<');
            $this->reconsume($base);
        }
    }

    private function endOpen(string $c, string $base, string $nameState): void
    {
        if (self::isAlpha($c)) {
            $this->tag = [true, '', [], false];
            $this->reconsume($nameState);
        } else {
            $this->emit('</');
            $this->reconsume($base);
        }
    }

    private function endName(string $c, string $base): void
    {
        if (self::isWs($c) && $this->appropriate()) {
            $this->state = 'before_attr_name';
        } elseif ($c === '/' && $this->appropriate()) {
            $this->state = 'self_closing';
        } elseif ($c === '>' && $this->appropriate()) {
            $this->state = 'data';
            $this->emitTag();
        } elseif (self::isAlpha($c)) {
            $this->tag[1] .= self::lower($c);
            $this->temp .= $c;
        } else {
            $this->emit('</' . $this->temp);
            $this->tag = null;
            $this->reconsume($base);
        }
    }

    /** Consumes $c ('' at end of input); returns false when tokenizing is done. */
    private function step(string $c): bool
    {
        $st = $this->state;
        $eof = $c === '';
        switch ($st) {
            case 'data':
                if ($c === '&') {
                    $this->ret = 'data';
                    $this->state = 'charref';
                } elseif ($c === '<') {
                    $this->state = 'tag_open';
                } elseif ($eof) {
                    return false;
                } else {
                    $this->emit($c);
                }
                return true;
            case 'rcdata':
                if ($c === '&') {
                    $this->ret = 'rcdata';
                    $this->state = 'charref';
                } elseif ($c === '<') {
                    $this->state = 'rcdata_lt';
                } elseif ($eof) {
                    return false;
                } else {
                    $this->emit(self::orR($c));
                }
                return true;
            case 'rawtext':
            case 'script':
                if ($c === '<') {
                    $this->state = $st . '_lt';
                } elseif ($eof) {
                    return false;
                } else {
                    $this->emit(self::orR($c));
                }
                return true;
            case 'plaintext':
                if ($eof) {
                    return false;
                }
                $this->emit(self::orR($c));
                return true;
            case 'tag_open':
                if ($c === '!') {
                    $this->state = 'markup_decl';
                } elseif ($c === '/') {
                    $this->state = 'end_tag_open';
                } elseif (self::isAlpha($c)) {
                    $this->tag = [false, '', [], false];
                    $this->reconsume('tag_name');
                } elseif ($c === '?') {
                    $this->comment = '';
                    $this->reconsume('bogus_comment');
                } elseif ($eof) {
                    $this->emit('<');
                    return false;
                } else {
                    $this->emit('<');
                    $this->reconsume('data');
                }
                return true;
            case 'end_tag_open':
                if (self::isAlpha($c)) {
                    $this->tag = [true, '', [], false];
                    $this->reconsume('tag_name');
                } elseif ($c === '>') {
                    $this->state = 'data';
                } elseif ($eof) {
                    $this->emit('</');
                    return false;
                } else {
                    $this->comment = '';
                    $this->reconsume('bogus_comment');
                }
                return true;
            case 'tag_name':
                if (self::isWs($c)) {
                    $this->state = 'before_attr_name';
                } elseif ($c === '/') {
                    $this->state = 'self_closing';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitTag();
                } elseif ($eof) {
                    return false;
                } else {
                    $this->tag[1] .= $c === "\0" ? self::R : self::lower($c);
                }
                return true;
            case 'rcdata_lt':
                $this->lt($c, 'rcdata', 'rcdata_end_open');
                return true;
            case 'rcdata_end_open':
                $this->endOpen($c, 'rcdata', 'rcdata_end_name');
                return true;
            case 'rcdata_end_name':
                $this->endName($c, 'rcdata');
                return true;
            case 'rawtext_lt':
                $this->lt($c, 'rawtext', 'rawtext_end_open');
                return true;
            case 'rawtext_end_open':
                $this->endOpen($c, 'rawtext', 'rawtext_end_name');
                return true;
            case 'rawtext_end_name':
                $this->endName($c, 'rawtext');
                return true;
            case 'script_lt':
                if ($c === '/') {
                    $this->temp = '';
                    $this->state = 'script_end_open';
                } elseif ($c === '!') {
                    $this->state = 'script_esc_start';
                    $this->emit('<!');
                } else {
                    $this->emit('<');
                    $this->reconsume('script');
                }
                return true;
            case 'script_end_open':
                $this->endOpen($c, 'script', 'script_end_name');
                return true;
            case 'script_end_name':
                $this->endName($c, 'script');
                return true;
            case 'script_esc_start':
            case 'script_esc_start_dash':
                if ($c === '-') {
                    $this->state = $st === 'script_esc_start' ? 'script_esc_start_dash' : 'script_esc_dash_dash';
                    $this->emit('-');
                } else {
                    $this->reconsume('script');
                }
                return true;
            case 'script_esc':
            case 'script_esc_dash':
            case 'script_esc_dash_dash':
                if ($c === '-') {
                    $this->state = $st === 'script_esc' ? 'script_esc_dash' : 'script_esc_dash_dash';
                    $this->emit('-');
                } elseif ($c === '<') {
                    $this->state = 'script_esc_lt';
                } elseif ($c === '>' && $st === 'script_esc_dash_dash') {
                    $this->state = 'script';
                    $this->emit('>');
                } elseif ($eof) {
                    return false;
                } else {
                    $this->state = 'script_esc';
                    $this->emit(self::orR($c));
                }
                return true;
            case 'script_esc_lt':
                if ($c === '/') {
                    $this->temp = '';
                    $this->state = 'script_esc_end_open';
                } elseif (self::isAlpha($c)) {
                    $this->temp = '';
                    $this->emit('<');
                    $this->reconsume('script_dbl_esc_start');
                } else {
                    $this->emit('<');
                    $this->reconsume('script_esc');
                }
                return true;
            case 'script_esc_end_open':
                $this->endOpen($c, 'script_esc', 'script_esc_end_name');
                return true;
            case 'script_esc_end_name':
                $this->endName($c, 'script_esc');
                return true;
            case 'script_dbl_esc_start':
            case 'script_dbl_esc_end':
                $start = $st === 'script_dbl_esc_start';
                if (self::isWs($c) || $c === '/' || $c === '>') {
                    $this->state = (($this->temp === 'script') === $start) ? 'script_dbl_esc' : 'script_esc';
                    $this->emit($c);
                } elseif (self::isAlpha($c)) {
                    $this->temp .= self::lower($c);
                    $this->emit($c);
                } else {
                    $this->reconsume($start ? 'script_esc' : 'script_dbl_esc');
                }
                return true;
            case 'script_dbl_esc':
            case 'script_dbl_esc_dash':
            case 'script_dbl_esc_dash_dash':
                if ($c === '-') {
                    $this->state = $st === 'script_dbl_esc' ? 'script_dbl_esc_dash' : 'script_dbl_esc_dash_dash';
                    $this->emit('-');
                } elseif ($c === '<') {
                    $this->state = 'script_dbl_esc_lt';
                    $this->emit('<');
                } elseif ($c === '>' && $st === 'script_dbl_esc_dash_dash') {
                    $this->state = 'script';
                    $this->emit('>');
                } elseif ($eof) {
                    return false;
                } else {
                    $this->state = 'script_dbl_esc';
                    $this->emit(self::orR($c));
                }
                return true;
            case 'script_dbl_esc_lt':
                if ($c === '/') {
                    $this->temp = '';
                    $this->state = 'script_dbl_esc_end';
                    $this->emit('/');
                } else {
                    $this->reconsume('script_dbl_esc');
                }
                return true;
            case 'before_attr_name':
                if (self::isWs($c)) {
                    return true;
                }
                if ($eof || $c === '/' || $c === '>') {
                    $this->reconsume('after_attr_name');
                } else {
                    $this->startAttr();
                    if ($c === '=') {
                        $this->addName('=');
                        $this->state = 'attr_name';
                    } else {
                        $this->reconsume('attr_name');
                    }
                }
                return true;
            case 'attr_name':
                if ($eof || self::isWs($c) || $c === '/' || $c === '>') {
                    $this->reconsume('after_attr_name');
                } elseif ($c === '=') {
                    $this->state = 'before_attr_value';
                } else {
                    $this->addName($c === "\0" ? self::R : self::lower($c));
                }
                return true;
            case 'after_attr_name':
                if (self::isWs($c)) {
                    return true;
                }
                if ($c === '/') {
                    $this->state = 'self_closing';
                } elseif ($c === '=') {
                    $this->state = 'before_attr_value';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitTag();
                } elseif ($eof) {
                    return false;
                } else {
                    $this->startAttr();
                    $this->reconsume('attr_name');
                }
                return true;
            case 'before_attr_value':
                if (self::isWs($c)) {
                    return true;
                }
                if ($c === '"') {
                    $this->state = 'attr_dq';
                } elseif ($c === "'") {
                    $this->state = 'attr_sq';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitTag();
                } else {
                    $this->reconsume('attr_unq');
                }
                return true;
            case 'attr_dq':
            case 'attr_sq':
                if ($c === ($st === 'attr_dq' ? '"' : "'")) {
                    $this->state = 'after_attr_value_q';
                } elseif ($c === '&') {
                    $this->ret = $st;
                    $this->state = 'charref';
                } elseif ($eof) {
                    return false;
                } else {
                    $this->addValue(self::orR($c));
                }
                return true;
            case 'attr_unq':
                if (self::isWs($c)) {
                    $this->state = 'before_attr_name';
                } elseif ($c === '&') {
                    $this->ret = 'attr_unq';
                    $this->state = 'charref';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitTag();
                } elseif ($eof) {
                    return false;
                } else {
                    $this->addValue(self::orR($c));
                }
                return true;
            case 'after_attr_value_q':
                if (self::isWs($c)) {
                    $this->state = 'before_attr_name';
                } elseif ($c === '/') {
                    $this->state = 'self_closing';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitTag();
                } elseif ($eof) {
                    return false;
                } else {
                    $this->reconsume('before_attr_name');
                }
                return true;
            case 'self_closing':
                if ($c === '>') {
                    $this->tag[3] = true;
                    $this->state = 'data';
                    $this->emitTag();
                } elseif ($eof) {
                    return false;
                } else {
                    $this->reconsume('before_attr_name');
                }
                return true;
            case 'bogus_comment':
                if ($c === '>') {
                    $this->state = 'data';
                    $this->emitComment();
                } elseif ($eof) {
                    $this->emitComment();
                    return false;
                } else {
                    $this->comment .= self::orR($c);
                }
                return true;
            case 'markup_decl':
                $this->i--;
                $this->comment = '';
                if ($this->peek(2) === '--') {
                    $this->i += 2;
                    $this->state = 'comment_start';
                } elseif (self::lower($this->peek(7)) === 'doctype') {
                    $this->i += 7;
                    $this->state = 'doctype';
                } else {
                    $this->state = 'bogus_comment';
                }
                return true;
            case 'comment_start':
                if ($c === '-') {
                    $this->state = 'comment_start_dash';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitComment();
                } else {
                    $this->reconsume('comment');
                }
                return true;
            case 'comment_start_dash':
                if ($c === '-') {
                    $this->state = 'comment_end';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitComment();
                } elseif ($eof) {
                    $this->emitComment();
                    return false;
                } else {
                    $this->comment .= '-';
                    $this->reconsume('comment');
                }
                return true;
            case 'comment':
                if ($c === '<') {
                    $this->comment .= '<';
                    $this->state = 'comment_lt';
                } elseif ($c === '-') {
                    $this->state = 'comment_end_dash';
                } elseif ($eof) {
                    $this->emitComment();
                    return false;
                } else {
                    $this->comment .= self::orR($c);
                }
                return true;
            case 'comment_lt':
                if ($c === '!') {
                    $this->comment .= '!';
                    $this->state = 'comment_lt_bang';
                } elseif ($c === '<') {
                    $this->comment .= '<';
                } else {
                    $this->reconsume('comment');
                }
                return true;
            case 'comment_lt_bang':
                $c === '-' ? $this->state = 'comment_lt_bang_dash' : $this->reconsume('comment');
                return true;
            case 'comment_lt_bang_dash':
                $c === '-' ? $this->state = 'comment_lt_bang_dash_dash' : $this->reconsume('comment_end_dash');
                return true;
            case 'comment_lt_bang_dash_dash':
                $this->reconsume('comment_end');
                return true;
            case 'comment_end_dash':
                if ($c === '-') {
                    $this->state = 'comment_end';
                } elseif ($eof) {
                    $this->emitComment();
                    return false;
                } else {
                    $this->comment .= '-';
                    $this->reconsume('comment');
                }
                return true;
            case 'comment_end':
                if ($c === '>') {
                    $this->state = 'data';
                    $this->emitComment();
                } elseif ($c === '!') {
                    $this->state = 'comment_end_bang';
                } elseif ($c === '-') {
                    $this->comment .= '-';
                } elseif ($eof) {
                    $this->emitComment();
                    return false;
                } else {
                    $this->comment .= '--';
                    $this->reconsume('comment');
                }
                return true;
            case 'comment_end_bang':
                if ($c === '-') {
                    $this->comment .= '--!';
                    $this->state = 'comment_end_dash';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitComment();
                } elseif ($eof) {
                    $this->emitComment();
                    return false;
                } else {
                    $this->comment .= '--!';
                    $this->reconsume('comment');
                }
                return true;
            case 'doctype':
                if (self::isWs($c)) {
                    $this->state = 'before_doctype_name';
                } elseif ($eof) {
                    $this->doctype = [null, null, null, false];
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->reconsume('before_doctype_name');
                }
                return true;
            case 'before_doctype_name':
                if (self::isWs($c)) {
                    return true;
                }
                $this->doctype = [null, null, null, false];
                if ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype(true);
                } elseif ($eof) {
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->doctype[0] = $c === "\0" ? self::R : self::lower($c);
                    $this->state = 'doctype_name';
                }
                return true;
            case 'doctype_name':
                if (self::isWs($c)) {
                    $this->state = 'after_doctype_name';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype();
                } elseif ($eof) {
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->doctype[0] .= $c === "\0" ? self::R : self::lower($c);
                }
                return true;
            case 'after_doctype_name':
                if (self::isWs($c)) {
                    return true;
                }
                if ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype();
                } elseif ($eof) {
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->i--;
                    $word = self::lower($this->peek(6));
                    if ($word === 'public' || $word === 'system') {
                        $this->i += 6;
                        $this->state = "after_doctype_{$word}_kw";
                    } else {
                        $this->i++;
                        $this->doctype[3] = true;
                        $this->state = 'bogus_doctype';
                    }
                }
                return true;
            case 'after_doctype_public_kw':
            case 'after_doctype_system_kw':
            case 'before_doctype_public_id':
            case 'before_doctype_system_id':
                $idx = str_contains($st, 'public') ? 1 : 2;
                $which = $idx === 1 ? 'public' : 'system';
                if (self::isWs($c)) {
                    if (str_starts_with($st, 'after')) {
                        $this->state = "before_doctype_{$which}_id";
                    }
                } elseif ($c === '"' || $c === "'") {
                    $this->doctype[$idx] = '';
                    $this->state = "doctype_{$which}_" . ($c === '"' ? 'dq' : 'sq');
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype(true);
                } elseif ($eof) {
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->doctype[3] = true;
                    $this->reconsume('bogus_doctype');
                }
                return true;
            case 'doctype_public_dq':
            case 'doctype_public_sq':
            case 'doctype_system_dq':
            case 'doctype_system_sq':
                $idx = str_contains($st, 'public') ? 1 : 2;
                if ($c === (str_ends_with($st, 'dq') ? '"' : "'")) {
                    $this->state = $idx === 1 ? 'after_doctype_public_id' : 'after_doctype_system_id';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype(true);
                } elseif ($eof) {
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->doctype[$idx] .= self::orR($c);
                }
                return true;
            case 'after_doctype_public_id':
            case 'between_doctype_ids':
                if (self::isWs($c)) {
                    $this->state = 'between_doctype_ids';
                } elseif ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype();
                } elseif ($c === '"' || $c === "'") {
                    $this->doctype[2] = '';
                    $this->state = $c === '"' ? 'doctype_system_dq' : 'doctype_system_sq';
                } elseif ($eof) {
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->doctype[3] = true;
                    $this->reconsume('bogus_doctype');
                }
                return true;
            case 'after_doctype_system_id':
                if (self::isWs($c)) {
                    return true;
                }
                if ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype();
                } elseif ($eof) {
                    $this->emitDoctype(true);
                    return false;
                } else {
                    $this->reconsume('bogus_doctype');
                }
                return true;
            case 'bogus_doctype':
                if ($c === '>') {
                    $this->state = 'data';
                    $this->emitDoctype();
                } elseif ($eof) {
                    $this->emitDoctype();
                    return false;
                }
                return true;
            case 'cdata':
                if ($c === ']') {
                    $this->state = 'cdata_bracket';
                } elseif ($eof) {
                    return false;
                } else {
                    $this->emit($c);
                }
                return true;
            case 'cdata_bracket':
                if ($c === ']') {
                    $this->state = 'cdata_end';
                } else {
                    $this->emit(']');
                    $this->reconsume('cdata');
                }
                return true;
            case 'cdata_end':
                if ($c === ']') {
                    $this->emit(']');
                } elseif ($c === '>') {
                    $this->state = 'data';
                } else {
                    $this->emit(']]');
                    $this->reconsume('cdata');
                }
                return true;
            case 'charref':
                $this->temp = '&';
                if (self::isAlnum($c)) {
                    $this->reconsume('named_ref');
                } elseif ($c === '#') {
                    $this->temp .= '#';
                    $this->state = 'numeric_ref';
                } else {
                    $this->flushRef();
                    $this->reconsume($this->ret);
                }
                return true;
            case 'named_ref':
                $this->namedRef();
                return true;
            case 'ambiguous_amp':
                if (self::isAlnum($c)) {
                    $this->inAttr() ? $this->addValue($c) : $this->emit($c);
                } else {
                    $this->reconsume($this->ret);
                }
                return true;
            case 'numeric_ref':
                $this->code = 0;
                if ($c === 'x' || $c === 'X') {
                    $this->temp .= $c;
                    $this->state = 'hex_ref_start';
                } else {
                    $this->reconsume('dec_ref_start');
                }
                return true;
            case 'hex_ref_start':
            case 'dec_ref_start':
                $hex = $st === 'hex_ref_start';
                if (strlen($c) === 1 && ($hex ? ctype_xdigit($c) : ctype_digit($c))) {
                    $this->reconsume($hex ? 'hex_ref' : 'dec_ref');
                } else {
                    $this->flushRef();
                    $this->reconsume($this->ret);
                }
                return true;
            case 'hex_ref':
            case 'dec_ref':
                $hex = $st === 'hex_ref';
                if (strlen($c) === 1 && ($hex ? ctype_xdigit($c) : ctype_digit($c))) {
                    $this->code = min($this->code * ($hex ? 16 : 10) + (int) hexdec($c), 0x110000);
                } elseif ($c === ';') {
                    $this->state = 'numeric_ref_end';
                } else {
                    $this->reconsume('numeric_ref_end');
                }
                return true;
            default: // numeric_ref_end
                $this->i--;
                $this->temp = self::numericChar($this->code);
                $this->flushRef();
                $this->state = $this->ret;
                return true;
        }
    }

    private function namedRef(): void
    {
        $this->i--;
        $start = $this->i;
        $j = $start;
        while ($j < $this->n && self::isAlnum($this->s[$j]) && $j - $start < self::MAX_ENTITY) {
            $j++;
        }
        $run = implode('', array_slice($this->s, $start, $j - $start));
        $match = null;
        if ($j < $this->n && $this->s[$j] === ';' && isset(Entities::TABLE[$run . ';'])) {
            $match = $run . ';';
        } else {
            for ($k = strlen($run); $k > 0; $k--) {
                if (isset(Entities::TABLE[substr($run, 0, $k)])) {
                    $match = substr($run, 0, $k);
                    break;
                }
            }
        }
        if ($match === null) {
            $this->temp .= $run;
            $this->i = $j;
            $this->flushRef();
            $this->state = 'ambiguous_amp';
            return;
        }
        $this->i = $start + strlen($match);
        $next = $this->i < $this->n ? $this->s[$this->i] : '';
        if ($this->inAttr() && !str_ends_with($match, ';') && ($next === '=' || self::isAlnum($next))) {
            $this->temp .= $match;
        } else {
            $this->temp = Entities::TABLE[$match];
        }
        $this->flushRef();
        $this->state = $this->ret;
    }
}
