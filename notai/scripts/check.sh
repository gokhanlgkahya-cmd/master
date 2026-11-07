#!/usr/bin/env bash
# NotAI "iş bitti" kontrolü: biçim, lint, test. Biri bile kırmızıysa iş bitmemiştir.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== cargo fmt --check"
cargo fmt --all --check
echo "== cargo clippy (-D warnings)"
cargo clippy --workspace --all-targets -- -D warnings
echo "== cargo test"
cargo test --workspace
echo "Tümü yeşil ✔"
