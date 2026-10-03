<?php

declare(strict_types=1);

// PSR-4 autoloader for src/ so the tests run without Composer.
spl_autoload_register(static function (string $class): void {
    if (str_starts_with($class, 'LombokHTML\\')) {
        $file = __DIR__ . '/../src/' . str_replace('\\', '/', substr($class, strlen('LombokHTML\\'))) . '.php';
        if (is_file($file)) {
            require $file;
        }
    }
});

$GLOBALS['lombok_failures'] = [];
$GLOBALS['lombok_checks'] = 0;

function check(bool $ok, string $what): void
{
    $GLOBALS['lombok_checks']++;
    if (!$ok) {
        $GLOBALS['lombok_failures'][] = $what;
    }
}

/** @param callable(): void $fn */
function throwsCode(callable $fn, string $code, string $what): void
{
    try {
        $fn();
        check(false, "$what: no exception");
    } catch (\LombokHTML\SelectorError $e) {
        check($e->errorCode === $code, "$what: got {$e->errorCode}, want $code");
    }
}

function finish(string $name): void
{
    $f = $GLOBALS['lombok_failures'];
    if ($f !== []) {
        fwrite(STDERR, count($f) . " failures in $name:\n" . implode("\n", $f) . "\n");
        exit(1);
    }
    echo "$name: {$GLOBALS['lombok_checks']} checks passed\n";
}
