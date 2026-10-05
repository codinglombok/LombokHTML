# LombokHTML — HOW TO DIST v0.1.0

Cara mendistribusikan `LombokHTML` ke setiap saluran: GitHub, registry paket, server/VPS, Docker, shared hosting, dan lokal. Workflow otomatis ada di `.github/workflows/ci.yml` dan `.github/workflows/release.yml` (memanggil reusable workflow `codinglombok/.github/.github/workflows/lombok-ci.yml` dan `lombok-release.yml` — lihat proposal ADR-019).

## 1. Upload Pertama ke GitHub (bootstrap, sekali saja)

```bash
# Dari root repo lokal LombokHTML/
gh repo create codinglombok/LombokHTML --public --license apache-2.0 \
  --description "Zero-dep HTML ingestion: tolerant parser, DOM, text extraction, sanitizer, selectors, meta, tables"
git init -b main
git add -A
git commit -m "feat: initial LombokHTML v0.1.0 (Rust core)"
git remote add origin https://github.com/codinglombok/LombokHTML.git
git push -u origin main
```

Setelah repo ada: aktifkan branch protection pada `main` (require PR review + status checks `lombok-ci` hijau), dan tambahkan secrets registry di **Settings → Secrets and variables → Actions** (lihat §3).

## 2. Alur Rilis Reguler (setiap PR merge ke `main`)

```
PR merge ke main
   │
   ▼
lombok-ci.yml (reusable)      — lint, test matrix per bahasa, coverage ≥90%, CodeQL, lombok doctor docs
   │  (hijau)
   ▼
release-please (bot)          — buka/update "release PR" berisi CHANGELOG.md dari commit feat:/fix:
   │  (PR release di-merge oleh maintainer)
   ▼
tag dibuat otomatis: vX.Y.Z (+ go/vX.Y.Z bila ada port Go)
   │
   ▼
lombok-release.yml (reusable) — build artefak tiap bahasa yang ada → publish (§3) → SBOM + provenance
```

Tidak ada langkah manual `npm publish` / `cargo publish` — semua lewat tag yang dibuat `release-please`, dibaca oleh `lombok-release.yml`.

## 3. Publish per Registry (dijalankan otomatis oleh `lombok-release.yml`, hanya untuk bahasa yang ada di repo)

| Registry | Dipicu bila ada | Perintah inti CI | Secret dibutuhkan |
|---|---|---|---|
| **crates.io** | `rust/Cargo.toml` | `cargo publish --locked` | `CARGO_REGISTRY_TOKEN` (trusted publishing OIDC lebih disarankan) |
| **npm** | `typescript/package.json` | `npm publish --provenance --access public` | OIDC (`id-token: write`) — tanpa token statis |
| **PyPI** | `python/pyproject.toml` | `python -m build && twine upload` | Trusted publishing OIDC |
| **Packagist** | `php/composer.json` | webhook otomatis saat tag di-push (tidak perlu step CI) | — (repo sudah terhubung ke Packagist) |
| **Go module** | `go/go.mod` | tag `go/vX.Y.Z` cukup — `go get` menarik langsung dari tag | — |
| **GitHub Packages** | selalu | `--ignore-scripts`, target ghcr.io/npm/Maven/NuGet/RubyGems sesuai isi repo | `GITHUB_TOKEN` bawaan |

Detail konfigurasi ada di `lombok-release.yml` (lihat repo `.github` pusat) — repo ini hanya memanggilnya lewat `uses:` di `.github/workflows/release.yml`.

## 4. Server / VPS (jika repo ini juga menyediakan biner/CLI)

Tidak berlaku: ini library, bukan layanan. Tidak ada biner/daemon untuk dideploy; dipasang sebagai dependensi oleh aplikasi yang dideploy (lihat topologi ARCHITECTURE_UTAMA §9).

## 5. Docker

Tidak ada image sendiri untuk library. Aplikasi konsumen cukup `cargo build --release` (atau install lewat registry) di dalam Dockerfile-nya; tidak ada dependensi sistem operasi.

## 6. Shared Hosting

Inti Rust tidak dipasang langsung di shared hosting. Untuk shared hosting gunakan **port PHP native** (terjadwal) lewat Composer — vendor diunggah, document root `public/` (ARCHITECTURE_UTAMA §9). Sampai port PHP ada, shared hosting belum didukung.

## 7. Lokal (Development)

```bash
git clone https://github.com/codinglombok/LombokHTML.git
cd LombokHTML
# Rust (referensi)
cd rust
cargo test --release
cargo build --no-default-features   # bukti no_std + alloc
cargo clippy --all-targets -- -D warnings

# Regenerasi ekspektasi vektor dari referensi Rust (tinjau diff sebelum commit!)
LOMBOK_REGEN=1 cargo test --release --test vectors

# TypeScript
cd ../typescript && npm install && npm test
```

## 8. Verifikasi Sebelum Rilis (`lombok doctor`)

```bash
# Dari root monorepo katalog, atau dari root repo ini:
./scripts/lombok-doctor-docs.sh LombokHTML   # 12 dokumen ada, versi cocok, hash vector cocok
cd rust && cargo test && cargo clippy -- -D warnings
```

CI menjalankan langkah yang sama pada setiap PR (`lombok-ci.yml`) — kegagalan lokal berarti kegagalan di CI juga.
