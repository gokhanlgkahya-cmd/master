#!/bin/bash
E2E="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; ROOT="$(cd "$E2E/../.." && pwd)"; mkdir -p "$E2E/out"
# Kullanım rehberi için tutarlı örnek dünya (Gökhan'ın işleri).
set -e
A=http://127.0.0.1:18787/api
CLI="env NOTAI_URL=http://127.0.0.1:18787 NOTAI_TOKEN=gizli $ROOT/target/debug/notai"
j() { curl -sf -H 'Authorization: Bearer gizli' -H 'Content-Type: application/json' -X POST "$A$1" -d "$2"; }
id() { python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])'; }
SABUN=$(j /topics '{"name":"Sabun işi","kind":"area"}' | id)
MALZ=$(j /topics "{\"name\":\"Malzeme hareketleri\",\"kind\":\"area\",\"parent_id\":\"$SABUN\"}" | id)
YAP=$(j /topics "{\"name\":\"Yapılacaklar\",\"kind\":\"area\",\"parent_id\":\"$SABUN\"}" | id)
PARA=$(j /topics "{\"name\":\"Para alışverişi\",\"kind\":\"area\",\"parent_id\":\"$SABUN\"}" | id)
EV=$(j /topics '{"name":"Ev","kind":"area"}' | id)
ARABA=$(j /topics '{"name":"Araba bakımı","kind":"area"}' | id)
IZMIR=$(j /topics '{"name":"İzmir tatili","kind":"project"}' | id)
TODAY=$(date +%F); TOM=$(date -d tomorrow +%F); YEST=$(date -d yesterday +%F); NEXT=$(date -d '+3 day' +%F)
j /entries "{\"text\":\"Ayhan Kimya teklifi: NaOH %99, 25 kg torba 1.850 ₺\",\"topic_id\":\"$MALZ\"}" >/dev/null
j /entries "{\"text\":\"Depoya 40 koli zeytinyağlı sabun girdi\",\"topic_id\":\"$MALZ\"}" >/dev/null
j /entries "{\"text\":\"Etiket matbaasından numune geldi, renkler soluk\",\"topic_id\":\"$YAP\"}" >/dev/null
j /entries "{\"text\":\"Mehmet Bey 12.000 ₺ havale yaptı (ekim siparişi)\",\"topic_id\":\"$PARA\"}" >/dev/null
j /entries "{\"text\":\"Kombi su kaçırıyor, servis çağrılacak\",\"topic_id\":\"$EV\"}" >/dev/null
j /entries "{\"text\":\"Muayene randevusu 20 Kasım 10:30\",\"topic_id\":\"$ARABA\"}" >/dev/null
j /entries "{\"text\":\"Otel rezervasyonunu teyit et, erken giriş sor\",\"topic_id\":\"$IZMIR\"}" >/dev/null
j /tasks "{\"title\":\"Kostik tekliflerini karşılaştır\",\"due_date\":\"$YEST\",\"topic_id\":\"$YAP\",\"priority\":\"high\"}" >/dev/null
R=$(j /tasks "{\"title\":\"Kombi servisini ara\",\"due_date\":\"$TOM\",\"due_time\":\"10:00\",\"topic_id\":\"$EV\"}" | id)
j /tasks/$R/edit '{"remind_before_minutes":60}' >/dev/null
R2=$(j /tasks "{\"title\":\"Etiket matbaasını ara\",\"due_date\":\"$TODAY\",\"due_time\":\"16:00\",\"topic_id\":\"$YAP\"}" | id)
j /tasks/$R2/edit '{"remind_before_minutes":15}' >/dev/null
j /tasks "{\"title\":\"Numune kolisini kargola\",\"due_date\":\"$NEXT\",\"topic_id\":\"$YAP\"}" >/dev/null
j /facts '{"text":"Sabun markam GemOlive"}' >/dev/null
j /facts '{"text":"Ayhan Kimya = kostik tedarikçisi"}' >/dev/null
# AI'ın işleyeceği iki kayıt: biri öneri (görev + hatırlatma + harcama), biri soru.
P=$(j /entries '{"text":"Yarın 11de Ayhan Kimyaya 25 torba kostik siparişi ver, 46.250 TL ödenecek"}' | id)
echo "{\"topic\":{\"existing_id\":\"$MALZ\",\"new_name\":null,\"new_kind\":null,\"parent_id\":null,\"confidence\":0.9},\"tasks\":[{\"title\":\"Ayhan Kimya'ya kostik siparişi ver\",\"due_date\":\"$TOM\",\"due_time\":\"11:00\",\"priority\":\"high\",\"confidence\":0.88}],\"tags\":[{\"name\":\"tedarik\",\"confidence\":0.7}],\"people\":[{\"name\":\"Ayhan Kimya\",\"confidence\":0.8}],\"reminder\":null,\"question\":null,\"expense\":{\"amount\":46250,\"direction\":\"gider\",\"category\":\"tedarik\",\"scope\":\"is\",\"date\":\"$TOM\",\"confidence\":0.75}}" | $CLI answer $P - >/dev/null || echo "uyarı: öneri yazılamadı"
Q=$(j /entries '{"text":"Ahmet ile görüş"}' | id)
echo '{"topic":null,"tasks":[],"tags":[],"people":[],"reminder":null,"question":"Hangi Ahmet? Ambalajcı Ahmet mi, muhasebeci Ahmet mi? Ne zaman görüşmek istiyorsun?","expense":null}' | $CLI answer $Q - >/dev/null || echo "uyarı: soru yazılamadı"
j /entries '{"text":"Okulda veli toplantısı perşembe akşam"}' >/dev/null
# Varlıklar
j /assets '{"name":"Kadıköy daire","kind":"gayrimenkul","value":"6.250.000"}' >/dev/null
j /assets '{"name":"Altın birikimi","kind":"altin","quantity":"25","unit":"gram","symbol":"gram altın","value":"82.500"}' >/dev/null
j /assets '{"name":"THY hisseleri","kind":"hisse","quantity":"150","unit":"adet","symbol":"THYAO","value":"46.800"}' >/dev/null
# Fiyat takibi
PX=$(j /topics '{"name":"Pixel 10 Pro","kind":"resource"}' | id)
j /topics/$PX/watch '{"query":"Pixel 10 Pro 256 GB","urls":["https://example.com/pixel"],"target_price":"50.000"}' >/dev/null
echo '{"offers":[{"title":"Google Pixel 10 Pro 256 GB","price":"54.999,00","url":"https://www.trendyol.com/pixel-10-pro","store":"Trendyol"},{"title":"Pixel 10 Pro 256 GB","price":"55.499,00","url":"https://www.hepsiburada.com/pixel-10-pro","store":"Hepsiburada"}]}' | $CLI watch-report $PX - >/dev/null
echo "seedR tamam"
