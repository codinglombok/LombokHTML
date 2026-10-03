<?php

declare(strict_types=1);

namespace LombokHTML;

/** A selector that does not follow the SPEC grammar; $offset counts characters. */
final class SelectorError extends \InvalidArgumentException
{
    public readonly string $errorCode;

    public function __construct(public readonly int $offset)
    {
        parent::__construct("BAD_SELECTOR at character $offset");
        $this->errorCode = 'BAD_SELECTOR';
    }
}
