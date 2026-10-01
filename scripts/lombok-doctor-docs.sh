#!/usr/bin/env bash
# lombok doctor docs — ARCHITECTURE_UTAMA_v3.3 §5 / MASTERPLAN_UTAMA §5.
# Checks, from a repo root:
#   1. all 12 standard documents exist in docs/ (naming: <jenis>_<Repo>_v<ver>.md)
#   2. version in filenames == manifest version (rust/Cargo.toml or typescript/package.json)
#   3. SPEC_ contains the mandatory normative sentence verbatim
#   4. SHA-256 of vectors/<package>-vectors-v1.json == hash stated in SPEC_
# Usage: lombok-doctor-docs.sh <RepoName> [repo_root]
set -u
REPO="${1:?usage: $0 <RepoName> [repo_root]}"
ROOT="${2:-.}"
PKG="$(echo "$REPO" | tr '[:upper:]' '[:lower:]')"
cd "$ROOT" || exit 2
fail=0
err() { echo "FAIL: $*"; fail=1; }

# --- manifest version
VER=""
if [ -f rust/Cargo.toml ]; then
  VER="$(grep -m1 '^version' rust/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"
elif [ -f typescript/package.json ]; then
  VER="$(grep -m1 '"version"' typescript/package.json | sed 's/.*: *"\(.*\)".*/\1/')"
fi
[ -n "$VER" ] || { err "cannot determine manifest version"; exit 1; }
echo "repo=$REPO version=$VER"

# --- 1+2: 12 documents with matching version
DOCS="masterplan architecture changelog map structure_repo full_summary_project guide_how_to_use how_to_dist development_ide API Lang SPEC"
count=0
for d in $DOCS; do
  f="docs/${d}_${REPO}_v${VER}.md"
  if [ -f "$f" ]; then count=$((count+1)); else err "missing $f"; fi
done
echo "documents: $count/12"

# --- 3: normative sentence
SPEC="docs/SPEC_${REPO}_v${VER}.md"
S="This document is the normative cross-language contract. Every language port MUST produce byte-identical output for all specified inputs. Deviations from this specification are bugs."
if [ -f "$SPEC" ]; then
  grep -qF "$S" "$SPEC" || err "SPEC lacks mandatory normative sentence"

  # --- 4: vector hash
  VEC="vectors/${PKG}-vectors-v1.json"
  if [ -f "$VEC" ]; then
    ACTUAL="$(sha256sum "$VEC" | cut -d' ' -f1)"
    grep -q "$ACTUAL" "$SPEC" || err "vector SHA-256 in SPEC does not match $VEC (actual $ACTUAL)"
  else
    err "missing $VEC"
  fi
fi

# --- 5: license files present and consistent with manifest (needed for registry publish)
LIC=""
[ -f rust/Cargo.toml ] && LIC="$(grep -m1 '^license' rust/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"
if echo "$LIC" | grep -q " OR MIT"; then
  [ -f LICENSE-MIT ] || err "manifest license '$LIC' but LICENSE-MIT missing"
  [ -f LICENSE-APACHE ] || err "manifest license '$LIC' but LICENSE-APACHE missing"
elif [ -n "$LIC" ]; then
  [ -f LICENSE ] || err "LICENSE missing"
fi
for f in LICENSE LICENSE-APACHE; do
  [ -f "$f" ] && grep -q "Apache License" "$f" && [ "$(wc -c < "$f")" -gt 10000 ] || true
done
[ -f LICENSE ] && [ "$(wc -c < LICENSE)" -lt 1000 ] && err "LICENSE looks like a placeholder (<1000 bytes)"

# --- 6: PRINSIP_UNIVERSAL U1 — no ownership claims ("part of <application/framework>")
APPS='RAG[A-Za-z]*|Clarion|DocFlow|PDF|AgenticAuto|Miner|DNSProxy|Proxy'
for f in README.md docs/*.md; do
  [ -f "$f" ] || continue
  case "$f" in docs/map_*) continue;; esac
  hits="$(grep -n -i -E "(peran di rag|role in rag|(part of|bagian dari|modul dari|komponen dari)[^.]{0,20}Lombok(${APPS})\\b|khusus (untuk )?(rag|Lombok[A-Za-z]+))" "$f" | grep -v -i -E "bukan|tidak|not |never|jangan|salah|diperiksa|ditolak|dilarang" || true)"
  if [ -n "$hits" ]; then err "ownership claim in $f (U1): $(echo "$hits" | head -1 | cut -c1-120)"; fi
done
# masterplan doc must carry the universal-principles section (U1-U12)
MP="docs/masterplan_${REPO}_v${VER}.md"
if [ -f "$MP" ]; then
  grep -q "^## 2\. Prinsip Universal" "$MP" || err "masterplan lacks '## 2. Prinsip Universal' section"
  n="$(grep -c -E '^\| U(1[0-2]|[1-9]) \|' "$MP")"
  [ "$n" -eq 12 ] || err "universal evidence table has $n/12 rows"
fi

# --- 7: every port that has a manifest must run the shared vectors (ADR-002/015)
[ -f rust/Cargo.toml ] && { [ -f rust/tests/vectors.rs ] || err "rust/tests/vectors.rs missing (vectors not executed)"; }
[ -f typescript/package.json ] && { [ -f typescript/test/vectors.test.ts ] || err "typescript/test/vectors.test.ts missing (vectors not executed)"; }
[ -f python/pyproject.toml ] && { ls python/tests/*vectors* >/dev/null 2>&1 || err "python vectors test missing"; }
[ -f go/go.mod ] && { ls go/*vectors*_test.go >/dev/null 2>&1 || err "go vectors test missing"; }
[ -f php/composer.json ] && { ls php/tests/*ectors* >/dev/null 2>&1 || err "php vectors test missing"; }
if [ -f "$VEC" ]; then
  n="$(grep -o '"fn"' "$VEC" | wc -l)"
  [ "$n" -ge 100 ] || err "vector file has only $n cases (<100)"
fi

[ $fail -eq 0 ] && echo "OK: lombok doctor docs passed" || { echo "lombok doctor docs FAILED"; exit 1; }
