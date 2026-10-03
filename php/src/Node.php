<?php

declare(strict_types=1);

namespace LombokHTML;

/**
 * A tree node. $kind is 'document', 'element', 'text' or 'comment'. $name and
 * $attrs are set for elements, $data for text and comments.
 */
final class Node
{
    public ?Node $parent = null;
    /** @var list<Node> */
    public array $children = [];

    /** @param list<array{0: string, 1: string}> $attrs */
    public function __construct(
        public readonly string $kind,
        public readonly string $name = '',
        public readonly array $attrs = [],
        public string $data = '',
    ) {
    }

    /** Value of the first attribute called $name. */
    public function attr(string $name): ?string
    {
        foreach ($this->attrs as [$k, $v]) {
            if ($k === $name) {
                return $v;
            }
        }
        return null;
    }

    /** True for an element called $name. */
    public function is(string $name): bool
    {
        return $this->kind === 'element' && $this->name === $name;
    }

    /**
     * Elements below this node in document order.
     *
     * @return list<Node>
     */
    public function elements(): array
    {
        $out = [];
        $todo = array_reverse($this->children);
        while ($todo !== []) {
            $n = array_pop($todo);
            if ($n->kind === 'element') {
                $out[] = $n;
                for ($i = count($n->children) - 1; $i >= 0; $i--) {
                    $todo[] = $n->children[$i];
                }
            }
        }
        return $out;
    }

    /**
     * Element children of the parent, this node included.
     *
     * @return list<Node>
     */
    public function elementSiblings(): array
    {
        if ($this->parent === null) {
            return [$this];
        }
        return array_values(array_filter($this->parent->children, static fn (Node $c): bool => $c->kind === 'element'));
    }

    /** Outer HTML of an element, inner HTML of the document (SPEC section 5). */
    public function serialize(): string
    {
        $out = [];
        Html::write($this, $out);
        return implode('', $out);
    }

    /** Text below this node, skipping script, style, noscript, template and title (SPEC section 6.1). */
    public function textContent(): string
    {
        $out = '';
        $todo = [$this];
        while ($todo !== []) {
            $n = array_pop($todo);
            if ($n->kind === 'text') {
                $out .= $n->data;
            } elseif ($n->kind === 'document' || ($n->kind === 'element' && !isset(Html::HIDDEN[$n->name]))) {
                for ($i = count($n->children) - 1; $i >= 0; $i--) {
                    $todo[] = $n->children[$i];
                }
            }
        }
        return $out;
    }

    /** Structure-preserving plain text (SPEC section 6.2). */
    public function extractText(): string
    {
        return Html::extract($this);
    }

    /**
     * Elements below this node matching $selector, in document order (SPEC section 8).
     *
     * @return list<Node>
     * @throws SelectorError
     */
    public function query(string $selector): array
    {
        $sel = Selector::parse($selector);
        $sel->reset();
        $out = array_values(array_filter($this->elements(), static fn (Node $e): bool => $sel->matchesCached($e)));
        $sel->reset();
        return $out;
    }

    /**
     * Page metadata (SPEC section 9).
     *
     * @return array{title: ?string, description: ?string, canonical: ?string, lang: ?string, og: list<array{0: string, 1: string}>}
     */
    public function meta(): array
    {
        return Html::meta($this);
    }

    /**
     * Every table as a grid of cell texts (SPEC section 10).
     *
     * @return list<list<list<string>>>
     */
    public function tables(): array
    {
        return Html::tables($this);
    }
}
