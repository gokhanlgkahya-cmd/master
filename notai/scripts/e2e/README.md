# Tarayıcı denemeleri (Playwright, gerçek sunucuya karşı)

Gerekenler: Rust araç zinciri, `trunk`, Node + Playwright (Chromium), Python 3.

```bash
./scripts/build-ui.sh                       # arayüz paketi (crates/ui/dist)
cargo build -p notai-server -p notai-cli
bash scripts/e2e/run.sh                     # sunucu :18787 + arayüz :18080, boş veritabanı (scripts/e2e/out/)
bash scripts/e2e/seedR.sh                   # rehberdeki örnek dünya (Sabun işi, Ayhan Kimya…)
node scripts/e2e/rehber.js                  # 26 ekran görüntüsü → scripts/e2e/out/rehber/
```

| Betik | Ne dener |
|---|---|
| `rehber.js` (+ `seedR.sh`) | Kullanım rehberinin 26 adımı |
| `voice.js` (+ `run-stt.sh`, `fakewhisper/`) | Sesli not: Whisper yokken bekleme, sahte Whisper açılınca düzeltme |
| `moves.js` | Konum hareketleri (konum izinli/izinsiz) |
| `asset.js` · `watch.js` · `fin.js` | Varlıklar · fiyat takibi · banka ekstresi |
| `qa.js` · `tree.js` · `wa.js` | AI sorusu · alt konular · WhatsApp düzeni (bazıları `seed2.sh` ister) |

Windows'ta Git Bash ya da WSL kullan. Uygulama kapalıyken çalışan alarm ve Android eklentileri burada denenemez; telefonda denenir.
