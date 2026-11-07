#!/usr/bin/env bash
# sqlx derleme zamanı kontrolü için .sqlx/ çevrimdışı önbelleğini yeniden üretir.
# Şema (crates/store/migrations) ya da bir sqlx::query! değişince çalıştır ve .sqlx/'i commit et.
set -euo pipefail
cd "$(dirname "$0")/.."
DB="$PWD/target/sqlx-dev.db"
mkdir -p target
rm -f "$DB" "$DB-wal" "$DB-shm"
export DATABASE_URL="sqlite://$DB"
export SQLX_OFFLINE=false
sqlx database create
sqlx migrate run --source crates/store/migrations
cargo sqlx prepare --workspace -- --all-targets
echo ".sqlx/ güncellendi"
