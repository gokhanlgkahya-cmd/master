# Mimari (Faz 3'te uygulanır; Faz 2'de kullanıcıyla netleştirilir)

## Genel görünüm
```
 [Android uygulaması]        [Masaüstü uygulaması]
  Tauri 2 + Leptos            Tauri 2 + Leptos
  yerel SQLite (offline)      yerel SQLite                  ▼
        │      senkron (HTTPS, değişiklik günlüğü)     ┌─────────────────────┐
        └──────────────────────────────────────────────►  notai-server (axum) │  ← S24 Ultra ev sunucusu
                                                       │  SQLite + iş kuyruğu  │
                                                       │  AI işçisi            │──► LLM (Claude/Gemini API
                                                       │  zamanlayıcı          │      veya yerel Gemma)
                                                       └─────────────────────┘
```
**Yerel öncelikli (local-first):** uygulama internetsiz tam çalışır; sunucu senkron, AI ve hatırlatma beynidir.

## Cargo workspace
```
crates/
  core/      domain tipleri, kurallar, PARA, durum makineleri (I/O yok, %100 test)
  store/     sqlx + SQLite, migration'lar, değişiklik günlüğü (sync için)
  sync/      istemci↔sunucu senkron protokolü (son-yazan-kazanır + alan bazlı birleştirme)
  ai/        LLM istemcisi (sağlayıcıdan bağımsız trait), prompt'lar, JSON şema doğrulama
  ui/        Leptos bileşenleri + ekranlar (tokens.json'dan üretilen CSS değişkenleri)
  app/       Tauri 2 kabuğu (Android + masaüstü), bildirimler, ses kaydı, widget
  server/    axum: REST, zamanlayıcı, AI kuyruğu, ses → yazı
```

## Veri modeli (çekirdek)
- **Entry** (her yakalanan şey): id, ham metin, kaynak (uygulama/ses), zaman, durum (gelen_kutusu → işlendi → arşiv)
- **Note** / **Task** (Entry'den türetilir): görevlerde tarih, öncelik, tekrar, hatırlatma
- **Area** (PARA): Proje (bitiş tarihli) · Alan (süreklilik) · Kaynak · Arşiv
- **Tag**, **Link** (not↔not), **Person** (kişiler)
- **Suggestion**: AI'ın önerdiği sınıflandırma/görev/hatırlatma, güven skoru, kullanıcı kararı (kabul/ret/düzelt)
- **ProfileFact**: kullanıcıyı tanıma katmanı ("GemOlive = sabun markası, Alan: İş") — yalnız onaylı bilgiler

## AI hattı (Saner + Ramble + Keep/Gemini'nin birleşimi)
1. Yakala → Entry gelen kutusuna düşer (asla beklemez, AI'sız da çalışır).
2. AI işçisi Entry'yi ProfileFact + son notlar bağlamıyla okur → yapılandırılmış JSON:
   `{tür, alan, etiketler, görevler[{başlık, tarih, öncelik}], kişiler, hatırlatma}`
3. Şema doğrulama → Suggestion olarak kaydedilir.
4. Güven yüksek + kullanıcı "otonom" modu açtıysa otomatik uygulanır (geri alınabilir); değilse çip olarak önerilir.
5. Her kabul/ret/düzeltme öğrenme sinyalidir → ProfileFact ve sınıflandırma örnekleri güncellenir.
6. **Sabah planı** (zamanlayıcı): geciken + bugünkü + önemli görevler, kısa özet.
   **Haftalık gözden geçirme**: işlenmemiş girişler, ilerlemeyen projeler, öneriler.
7. **Sohbetle arama**: notlar üzerinde anlamsal arama (SQLite FTS5 + gömme vektörleri) + LLM cevabı, kaynak notlara bağlantılı.

## Sesli yakalama
Android'de kayıt → sunucuya veya doğrudan LLM'e (Türkçe konuşma tanıma) → canlı metin + yapılandırılmış görev önizlemesi,
"aslında perşembe yap" gibi düzeltmeler aynı oturumda uygulanır.

## Güvenlik ve gizlilik
Sunucu yalnız ev ağı + Tailscale üzerinden erişilebilir. API anahtarları sunucuda, uygulamada değil.
Yerel model seçeneği (Gemma) her zaman açık kalır.
