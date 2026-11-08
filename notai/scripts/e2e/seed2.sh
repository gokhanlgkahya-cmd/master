#!/bin/bash
E2E="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; ROOT="$(cd "$E2E/../.." && pwd)"; mkdir -p "$E2E/out"
set -e
A=http://127.0.0.1:18787/api
j() { curl -sf -H 'Authorization: Bearer gizli' -H 'Content-Type: application/json' -X POST "$A$1" -d "$2"; }
T1=$(j /topics '{"name":"Kostik tedarik","kind":"project"}' | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
T2=$(j /topics '{"name":"GemOlive","kind":"area"}' | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
T3=$(j /topics '{"name":"Okuma notları","kind":"resource"}' | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
j /entries '{"text":"Kostik tedarikçisini ara – perşembe. 25 kg’lık torba fiyatını sor."}' >/dev/null
j /entries "{\"text\":\"Ayhan Kimya teklifi: NaOH %99, 25 kg torba 1.850 ₺\",\"topic_id\":\"$T1\"}" >/dev/null
j /entries "{\"text\":\"Zeytinyağlı sabunda kür süresi 4–6 hafta.\",\"topic_id\":\"$T2\"}" >/dev/null
E=$(j /entries '{"text":"Eski not: silinecek deneme kaydı"}' | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
j /entries/$E/trash '{}' >/dev/null
TODAY=$(date +%F); TOM=$(date -d tomorrow +%F); YEST=$(date -d yesterday +%F); NEXT=$(date -d '+3 day' +%F)
j /tasks "{\"title\":\"Kostik tekliflerini karşılaştır\",\"due_date\":\"$YEST\",\"topic_id\":\"$T1\",\"priority\":\"high\"}" >/dev/null
j /tasks "{\"title\":\"Ambalaj faturasını muhasebeye ilet\",\"due_date\":\"$TODAY\",\"due_time\":\"17:00\",\"topic_id\":\"$T2\"}" >/dev/null
R=$(j /tasks "{\"title\":\"Kombi servisini ara\",\"due_date\":\"$TOM\",\"due_time\":\"10:00\"}" | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
j /tasks/$R/edit '{"remind_before_minutes":60}' >/dev/null
R2=$(j /tasks "{\"title\":\"Tansiyon ölçümü\",\"due_date\":\"$TODAY\",\"due_time\":\"20:00\"}" | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
j /tasks/$R2/edit '{"remind_before_minutes":15}' >/dev/null
j /tasks "{\"title\":\"Numune yolla\",\"due_date\":\"$NEXT\"}" >/dev/null
j /tasks '{"title":"Sabun kalıbı ölçüsünü öğren"}' >/dev/null
j /facts '{"text":"GemOlive = sabun markam"}' >/dev/null
j /facts '{"text":"Ayhan Kimya = kostik tedarikçisi"}' >/dev/null
echo seeded
