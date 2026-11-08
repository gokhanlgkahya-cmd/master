#!/bin/bash
E2E="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; ROOT="$(cd "$E2E/../.." && pwd)"; mkdir -p "$E2E/out"
# Gerçek sunucu (CORS açık) + arayüz paketi ayrı kökenden (doğrudan bağlantı, proxy yok).
pkill -f 'target/debug/notai-server'; pkill -f 'http.server 18080'; sleep 1
rm -f $E2E/out/ui-e2e.db*
cd "$ROOT"
NOTAI_AI=external NOTAI_CORS_ORIGINS=http://127.0.0.1:18080 NOTAI_API_TOKEN=gizli NOTAI_DATABASE_URL=sqlite://$E2E/out/ui-e2e.db NOTAI_BIND=127.0.0.1:18787 nohup ./target/debug/notai-server > $E2E/server.log 2>&1 &
(cd crates/ui/dist && nohup python3 -m http.server 18080 --bind 127.0.0.1 > $E2E/static.log 2>&1 &)
sleep 2
