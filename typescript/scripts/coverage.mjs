// Runs the test suite with Node's built-in coverage and enforces thresholds.
// The --test-coverage-* threshold flags only exist from Node 22.8, so the
// "all files" row of the report is parsed here to support Node 20 as well.
// Optional argument: the directory to run the tests in (default: current directory).
import { spawnSync } from 'node:child_process'

const MIN = { line: 90, branch: 90, funcs: 85 }

const run = spawnSync(process.execPath, ['--test', '--test-reporter=tap', '--experimental-test-coverage'], { encoding: 'utf8', cwd: process.argv[2] ?? '.' })
process.stdout.write(run.stdout)
process.stderr.write(run.stderr)
if (run.status !== 0) process.exit(run.status ?? 1)

const row = run.stdout.split(/\r?\n/).find(l => /^(?:#|ℹ)?\s*all files\s*\|/.test(l))
if (!row) {
  console.error('coverage: "all files" row not found in the report')
  process.exit(1)
}
const [line, branch, funcs] = row.split('|').slice(1, 4).map(v => Number.parseFloat(v))
let ok = true
for (const [name, value] of Object.entries({ line, branch, funcs })) {
  if (!(value >= MIN[name])) {
    console.error(`coverage: ${name} ${value}% is below the ${MIN[name]}% threshold`)
    ok = false
  }
}
if (!ok) process.exit(1)
console.log(`coverage: line ${line}%, branch ${branch}%, funcs ${funcs}% (thresholds ${MIN.line}/${MIN.branch}/${MIN.funcs})`)
