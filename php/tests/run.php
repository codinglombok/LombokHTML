<?php

declare(strict_types=1);

// Runs every test file. With the pcov extension loaded and --coverage=N, also
// fails when line coverage of src/ is below N percent.

$min = null;
foreach ($argv as $arg) {
    if (str_starts_with($arg, '--coverage=')) {
        $min = (float) substr($arg, strlen('--coverage='));
    }
}
$src = realpath(__DIR__ . '/../src');
$coverage = $min !== null && extension_loaded('pcov');
if ($min !== null && !$coverage) {
    fwrite(STDERR, "pcov extension not loaded; coverage not measured\n");
    exit(1);
}
if ($coverage) {
    \pcov\start();
}
foreach (['api.php', 'vectors.php'] as $file) {
    $GLOBALS['lombok_failures'] = [];
    $GLOBALS['lombok_checks'] = 0;
    require __DIR__ . '/' . $file;
}
if ($coverage) {
    \pcov\stop();
    $files = [];
    $it = new RecursiveIteratorIterator(new RecursiveDirectoryIterator($src, FilesystemIterator::SKIP_DOTS));
    foreach ($it as $f) {
        $files[] = $f->getPathname();
    }
    $hit = 0;
    $total = 0;
    foreach (\pcov\collect(\pcov\inclusive, $files) as $file => $lines) {
        $missed = [];
        foreach ($lines as $line => $count) {
            $total++;
            if ($count > 0) {
                $hit++;
            } else {
                $missed[] = $line;
            }
        }
        if ($missed !== []) {
            echo substr($file, strlen($src) + 1) . ': not run ' . implode(',', $missed) . "\n";
        }
    }
    $pct = $total ? 100 * $hit / $total : 100.0;
    printf("coverage: %.1f%% of %d lines (threshold %.0f%%)\n", $pct, $total, $min);
    if ($pct < $min) {
        exit(1);
    }
}
