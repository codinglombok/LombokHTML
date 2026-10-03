<?php

declare(strict_types=1);

namespace LombokHTML;

/** Sanitizer policy: the defaults plus extra tags, attributes and schemes (SPEC section 7). */
final class Policy
{
    public const DEFAULT_SCHEMES = ['http', 'https', 'mailto', 'tel'];

    /** @var array<string, true> */
    private array $tags = [];
    /** @var array<string, true> */
    private array $drop = [];
    /** @var array<string, true> */
    private array $attrs = [];
    /** @var array<string, true> */
    private array $schemes = [];

    public function __construct()
    {
        foreach (explode(' ', 'a abbr b blockquote br caption cite code dd del dfn div dl dt em figcaption figure h1 h2 h3 h4 '
            . 'h5 h6 hr i img ins kbd li mark ol p pre q s samp small span strong sub sup table tbody td tfoot th thead time tr u ul') as $t) {
            $this->tags[$t] = true;
        }
        foreach (explode(' ', 'applet audio base button canvas embed frame frameset head iframe link math meta noembed '
            . 'noframes noscript object option plaintext script select style svg template textarea title video xmp') as $t) {
            $this->drop[$t] = true;
        }
        foreach (explode(' ', '*:dir *:lang *:title a:href blockquote:cite img:alt img:height img:src img:width ol:start '
            . 'q:cite td:colspan td:rowspan th:colspan th:rowspan th:scope time:datetime') as $a) {
            $this->attrs[$a] = true;
        }
        foreach (self::DEFAULT_SCHEMES as $s) {
            $this->schemes[$s] = true;
        }
    }

    /** Keeps $tag (ASCII case-insensitive), also when it is on the drop list. */
    public function allowTag(string $tag): self
    {
        $t = Tokenizer::lower($tag);
        if ($t === 'plaintext') {
            return $this; // cannot be closed again, so never kept
        }
        $this->tags[$t] = true;
        unset($this->drop[$t]);
        return $this;
    }

    /** Keeps attribute $attr on $tag; tag "*" means every kept element. */
    public function allowAttr(string $tag, string $attr): self
    {
        $this->attrs[Tokenizer::lower($tag) . ':' . Tokenizer::lower($attr)] = true;
        return $this;
    }

    /** Accepts URLs with $scheme in href, src and cite. */
    public function allowScheme(string $scheme): self
    {
        $this->schemes[Tokenizer::lower($scheme)] = true;
        return $this;
    }

    /** @internal 'drop', 'keep' or 'unwrap'. */
    public function action(string $tag): string
    {
        return isset($this->drop[$tag]) ? 'drop' : (isset($this->tags[$tag]) ? 'keep' : 'unwrap');
    }

    /** @internal */
    public function attrOk(string $tag, string $attr): bool
    {
        return isset($this->attrs["$tag:$attr"]) || isset($this->attrs["*:$attr"]);
    }

    /**
     * @internal
     * @return list<string>
     */
    public function schemes(): array
    {
        return array_map('strval', array_keys($this->schemes));
    }
}
