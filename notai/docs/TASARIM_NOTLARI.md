# Tasarım notları — 2026-11-08 konuşmalarının özeti (kod yok)

## Altı tasarım sorusuna verilen cevaplar (öneri; karar Gökhan'ın)
1. **Sohbet mi, klasör mü?**
   - Sohbetin artısı: yakalamada karar yükü yok, zaman bağlamı korunur, alışkanlık hazır, AI doğal bir muhatap olur.
   - Zaafı: "güncel hâl yok" (aynı bilgi üç kayıtta değişir), bulmak zor, uzun içerik balona sığmaz.
   - **Öneri:** Sohbet ham günlük olarak kalır; her konunun bir **durum kartı / PARA notu** olur, AI derler, kullanıcı onaylar.
2. **Kaybolan önem:**
   - **Canlı durum kartı:** Alanlar kaynak kayda bağlı; kullanıcı düzeltince alan kilitlenir; çelişen yeni kayıt gelince "güncellenmeli?" işareti çıkar.
   - **Yeniden yüzeye çıkarma:** İlgili yeni kayıt, tarih ya da çelişki olunca eski kayıt alıntıyla geri gelir.
3. **Teyit — risk tablosu:**
   - Para, tarih, kişi: her zaman teyit, belirsizse soru.
   - Konu, etiket: sessiz uygulama + geri al.
   - Arası: çip; bekleyenler haftalık toplu.
   - Öğrenme: kabul/ret oranları türe göre tutulur. Öğrenilen kurallar "Seni tanıma"da görünür ve düzeltilebilir; geri alma artarsa kural askıya alınır.
4. **Sesli not:**
   - konuşurken beliren çipler,
   - sesle düzeltme ("hayır, cuma yap"),
   - WhatsApp hareketleri (basılı tut, kilitle, iptal),
   - kişisel sözlük (Whisper'a ipucu),
   - ses–metin eşleşmesi ve Whisper'ın ne değiştirdiği,
   - uzun sesi bölme,
   - araçta sesli geri bildirim,
   - para/tarih içeren seste her zaman teyit.
5. **Modüller ↔ sohbet:**
   - **Melez model:** Nesne (görev, harcama) kaynaktır; kaydın altındaki çip nesnenin güncel hâlini gösterir; yalnız önemli değişiklikler sohbete tek satır düşer.
   - Raporlar nesne kimliğine göre sayar; iletme kopya değil **bağdır**.
   - Yanıt asıl kayda zincirle bağlanır; AI yanıtı zincirle birlikte okur.
   - Senkron: alan bazında son yazan kazanır + geri dönmeyen kararlar + metin düzenleme geçmişi.
6. **Masaüstü/web girdisi (öncelik sırasıyla):**
   - Android paylaş menüsü (bağlantı kartı + AI özeti),
   - masaüstü kısayol penceresi,
   - tarayıcı eklentisi,
   - e-postayla iletme (faturalar),
   - ekran görüntüsünden metin,
   - pano izleme.

## Hermes ve ajan
- Hermes (Nous Research): MIT lisanslı, Python, Termux'u destekliyor, içinde MCP istemcisi var.
- **Hermes'i APK'ya gömmek önerilmez:** boyut büyür, bağımlılıklar Android'de derlenmeyebilir, güncellemesi zor olur.
- **Önerilen yol:**
  - NotAI → MCP sunucusu (T-11).
  - Ajan iş gelince uyanır (T-12).
  - Dağıtım için NotAI'ye gömülü **Rust ajan çekirdeği** (T-16): Hermes'ten ilham alır, kod alınmaz.
- **Pilin gitme nedeni tahmini:** Ubuntu (proot) + uyanık tutma kilidi + Telegram'ı sürekli dinleme + sürekli açık süreçler.

## Ücretsiz bulut (Kasım 2026 araştırması; koşullar değişebilir)
- **Oracle Always Free:**
  - ARM sunucu: belgede 4 çekirdek / 24 GB; bir kaynağa göre 2026'da 2 / 12'ye indirildi, doğrulanmadı.
  - Boşta kalan makineyi geri alabiliyor (yaklaşık 7 günde %15 altı kullanım); kredi kartı gerekir.
  - NotAI sunucusu için yeterli.
- **Cloudflare Workers + D1:**
  - 5 GB, günlük okuma/yazma sınırları.
  - Workers AI günde 10.000 birim ücretsiz.
  - Sunucunun bu ortam için yeniden yazılması gerekir.
- **Gemini ücretsiz katmanı:** İçerik ürün geliştirmede kullanılabilir ve insanlar okuyabilir → **gizli veri gönderilmez.**
- **Google Sheets:** Veritabanı olarak değil, **çıktı olarak** (Anıl'ın görmesi için ortak defter ve raporlar).

## Dürüst değerlendirme (2026-11-08)
- **Temel ve vizyon sağlam.** En büyük risk teknik değil: **program gerçek hayatta hiç kullanılmadı**, kapsam her konuşmada büyüyor.
- **Öneri:**
  - PC'de yeni özellik eklemeyi durdur.
  - APK → alarm güvenilirliği → Sabun işi ilk paket → 2–3 hafta her gün kullanım.
  - Ölçüt: her gün kendiliğinden açılıyor mu, WhatsApp'a kendine yazma bitti mi?
- **Sınırlı ya da zor olanlar:**
  - el yazısı okuma (her satır onaylanır),
  - Gemma'nın tavanı,
  - Samsung'un pil yönetimi,
  - bankaya otomatik bağlantı (lisans gerekir → PDF yolu kalır),
  - otomatik al-sat emri (SPK),
  - sağlık verisi (KVKK özel nitelikli veri; ticari üründe ağır yük),
  - paket topluluğu (zor olan insan bulmak).
- **Ticari konum:**
  - Genel "AI not uygulaması" olarak şansı yok (Notion, Google, Apple, Samsung).
  - **Niş:** Türkiye'de defter/WhatsApp ile hesap tutan esnaf, atölye ve küçük üretici. Fark: yazarak/konuşarak giriş, defter fotoğrafı, müşteri bazında satış/tahsilat/stok, ortak cari, tahsilatı kovalayan sadık köpek.
  - İkinci niş: serbest meslek (gizlilik vaadi, ama KVKK yükü).
  - **Yol:** önce kendin 1–2 ay kullan → 3–5 kişi (eş, Anıl, 2–3 esnaf) → iki hafta sonra hâlâ kullanıyorlarsa ticari düşünülür.
