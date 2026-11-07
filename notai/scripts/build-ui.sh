#!/usr/bin/env bash
# Leptos arayüzünü WASM olarak paketler → crates/ui/dist (Tauri kabuğu bunu gömer).
set -euo pipefail
cd "$(dirname "$0")/../crates/ui"
trunk build --release "$@"
