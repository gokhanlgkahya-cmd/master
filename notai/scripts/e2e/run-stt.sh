#!/bin/bash
E2E="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; ROOT="$(cd "$E2E/../.." && pwd)"; mkdir -p "$E2E/out"
# Sunucuyu aynı veritabanıyla, sahte Whisper'a bağlı yeniden başlatır.
pkill -f 'target/debug/notai-server'; sleep 1
pgrep -f fakewhisper/server.py >/dev/null || (nohup python3 $E2E/fakewhisper/server.py >/dev/null 2>&1 &)
sleep 1
cd "$ROOT"
NOTAI_STT_MODEL=large-v3-turbo NOTAI_STT_BASE_URL=http://127.0.0.1:18900 NOTAI_AI=external NOTAI_CORS_ORIGINS=http://127.0.0.1:18080 NOTAI_API_TOKEN=gizli NOTAI_DATABASE_URL=sqlite://$E2E/out/ui-e2e.db NOTAI_BIND=127.0.0.1:18787 nohup ./target/debug/notai-server > $E2E/server-stt.log 2>&1 &
sleep 2
