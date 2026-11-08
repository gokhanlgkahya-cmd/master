#!/usr/bin/env bash
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; BRANCH="${NOTAI_BRANCH:-$(git -C "$ROOT" branch --show-current)}"
# Kullanım: commit-if-green.sh "mesaj"  — check.sh yeşilse commit + push, değilse hataları göster.
set -u
cd "$ROOT"
LOG=$ROOT/target/check.log
cargo fmt --all
if ./scripts/check.sh > "$LOG" 2>&1; then
  cd "$ROOT/.." && git add -A notai && git commit -q -F - <<MSG && git push -q origin "$BRANCH" && git log --oneline -1
$1

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01BUAUBu5Nw2vJdZD4dGscSc
MSG
else
  echo "KONTROL KIRMIZI — commit yok"; grep -E "^(error|warning)|-->|FAILED|panicked" -A6 "$LOG" | head -60
fi
