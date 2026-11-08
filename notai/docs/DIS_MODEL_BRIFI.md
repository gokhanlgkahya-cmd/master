# NotAI — Başka bir yapay zekâ modeline tanıtım ve düşünme brifi

> Amaç: Karşı modelin hazır cevap vermesi değil, **alternatif üretmesi, ödünleşimleri tartması ve
> soru sorması**. Brif bilerek bizim seçtiğimiz çözümleri anlatmıyor (karşı modeli onlara
> bağlamamak için). Yalnız ürünü, kısıtları ve bugünkü durumu veriyor.
> Kopyala-yapıştır: `---` çizgileri arasındaki metin.

---

## Rolün

Kıdemli bir ürün tasarımcısı ve yazılım mimarı gibi düşün. Aşağıdaki kişisel uygulama için bana
hazır bir cevap verme. Her soruda:
1. **En az üç farklı yaklaşım** üret. Bunlar birbirinin çeşitlemesi değil, gerçekten farklı yaklaşımlar olsun; en az biri alışılmışın dışında olsun.
2. Her birinin **kullanıcıya** (hız, zihinsel yük, alışkanlık) ve **sisteme** (veri modeli, senkron, gizlilik, bakım) etkisini karşılaştır.
3. Hangi durumda hangisinin kazanacağını söyle; tek bir "en iyi"ye erken bağlanma.
4. Karar vermek için bilmen gereken ama bende olmayan bilgiyi **soru olarak** sor.
5. Varsayımlarını açıkça yaz.

## Ürün

**NotAI**, tek kişilik bir kişisel not + görev + yaşam verisi sistemi. İş ve özel hayat bir arada.
Arayüz bir **mesajlaşma uygulaması** gibi:
- **Ana ekran:** Sohbet listesi. Her "konu" bir kişi gibi; yuvarlak simge, son mesaj, saat ve karar bekleyen kayıt rozeti var.
- **Konu içi:** Yazışma. Kullanıcının kayıtları sağda balon; sistemin (NotAI) soruları ve haberleri solda.
- **Ana Günlük:** Her şeyin aktığı ortak bir sohbet.
- **Konu hiyerarşisi:** Ana konu → alt konular. Kayıt tektir ama üst konuda ve Ana Günlük'te de görünür.
- **AI:** Her kaydı arka planda okuyup **öneri** çıkarır: konu, görev, hatırlatma, etiket, kişi, harcama.
  - Öneriler kaydın altında çip olarak görünür; kullanıcı kabul ya da yoksay der, geri alabilir.
  - Belirsizse en çok 2 kısa soru sorar ve cevaplardan öğrenir.
  - İsteğe bağlı otonom mod vardır.
- **Diğer modüller:**
  - Görevler, takvim, hatırlatmalar, sabah planı, haftalık gözden geçirme.
  - Banka ekstresinden harcama analizi, varlık takibi, fiyat/ürün takibi.
  - Yazıdan konum hareketleri ("işe geldim").
  - Sesli not (telefon anında yazıya çevirir, ev sunucusu düzeltir).
  - Notlara soru sorma.
  - Obsidian'a dışa aktarım.

**Kullanıcı:**
- Endüstri mühendisi, yazılımcı değil. Ana cihazı Android (Samsung).
- Hızlı yakalama alışkanlığı WhatsApp'a kendine mesaj atmak.
- Yöntemi Bullet Journal günlük log + PARA (Proje / Alan / Kaynak / Arşiv) + haftalık gözden geçirme.

## Kısıtlar (değişmez)

- **Yerel öncelikli:** Her şey önce telefona yazılır, internetsiz çalışır. Kullanıcının evindeki ikinci bir telefon (7/24 sunucu) ile dakikada bir senkron yapılır; çakışmada son yazan kazanır. Bulut zorunluluğu yok.
- **AI kaynağı:** Evdeki sunucuda yerel model, ya da kullanıcının kendi AI ajanı. AI çıktısı her zaman şemayla doğrulanır; körü körüne yazılmaz.
- **Veri kaybı sıfır:** Silme önce çöpe gider; her değişiklik geri alınabilir.
- **Dil ve kitle:** Türkçe öncelikli, tek kullanıcı.
- **Teknoloji:** Rust (Tauri + Leptos/WASM + SQLite). Bu kısıt yalnız uygulanabilirlik için; fikirleri sınırlamasın.

## Bugünkü bilinen zayıflıklar

- Hatırlatmalar uygulama açıkken güvenilir; kapalıyken telefondan gelmeyebiliyor.
- Basılı tutma menüsü, yanıtlama, iletme, kaydırarak sekme geçişi yok.
- Kronolojik akışta eski ama önemli kayıtlar aşağıda kayboluyor.
- Gerçek cihazda henüz uzun süreli kullanılmadı.

## Sorular

**1. Sohbet mi, klasör/sayfa mı?**
Mesajlaşma tabanlı bir not arayüzü, klasik klasör/sayfa yapısına göre alışkanlık, yakalama hızı ve
bilgiye geri dönme hızı açısından hangi fırsatları sunar? Hangi temel UX engelleriyle karşılaşır?
Düşün:
- Yazmak kolay, bulmak zor mu?
- Uzun metin, liste, tablo sohbete sığar mı?
- Yıllar sonra 10 000 kayıtta ne olur?
- Hibrit modeller nasıl olabilir? Sohbet + belge, sohbet + tuval, sohbet + akıllı klasör…

**2. Akışta kaybolmayan önem.**
Kronolojik akışta zamanla aşağıda kalan kritik notlar ve proje durumları görünürlüğünü nasıl korur?
Hangi sabitleme, özetleme, "canlı durum kartı", tuval ya da pano kurguları mümkün?
- Bu özetleri kim üretir (kullanıcı mı, AI mı)?
- Ne zaman eskir?
- Sohbetle nasıl çift yönlü bağlı kalır?

**3. Yormayan teyit.**
Serbest metinden (harcama, görev, kişi, malzeme hareketi) arka planda yapı çıkaran bir AI'da, yanlış
eşleşmeler ve belirsizlikler kullanıcıyı yormadan nasıl teyit edilir?
- Satır içi çip mi, toplu gözden geçirme mi, sessiz kabul + geri al mı, güven eşikleri mi, soru mu?
- Hangi durumda hangisi?
- AI'ın kullanıcıdan öğrenmesi teyit yükünü zamanla nasıl azaltır?
- Yanlış öğrenme nasıl fark edilir ve düzeltilir?

**4. Sesli not.**
Ses kaydı metne çevrilip içindeki eylemler (görev, takvim etkinliği, harcama) ayrıştırılırken
deneyimi zenginleştirecek arayüz dokunuşları neler olabilir? Örneğin:
- canlı yazıya dökme,
- konuşurken çıkan çipler,
- "onu perşembe yap" gibi canlı düzeltme,
- uzun sesin bölümlenmesi,
- sesle metnin eşleştirilmesi (metne dokununca o saniyeyi çalma),
- gürültü ve yanlış tanımaya karşı güvenlik ağları.

**5. Modüller ve sohbet: tek gerçeklik.**
Harcama/varlık, takvim ve görev modüllerinin sohbet akışıyla **çift yönlü** çalışması veri
karmaşasını önlemek için nasıl kurgulanmalı?
- **Tek doğru kaynak:** Kaydın kendisi mi, modüldeki nesne mi? Biri değişince öbürüne ne olur?
- **Aynı bilginin birden çok konuda görünmesi:** Örneğin bir kaydı başka bir konuya "ilet"mek. Bunu kopya oluşturmadan nasıl yaparız?
- **Mükerrer saymamak:** Raporlarda ve toplamlarda aynı kayıt iki kez sayılmamalı.
- **Kaydın devamı:** Eski bir kayda "yanıt" verildiğinde, yanıt o verinin devamı olarak nasıl yorumlanmalı?
- **Senkron çakışmaları:** İki cihaz ve "son yazan kazanır" kuralıyla bunlar nasıl ele alınır?

**6. Masaüstünden ve webden hızlı girdi.**
Masaüstünde ya da webde gezinirken hızlı bilgi aktarmak için bu yapıya hangi alternatif girdi
yolları eklenebilir? Örneğin:
- web kırpma, bağlantı kaydetme,
- paylaş menüsü, tarayıcı eklentisi, e-postayla iletme,
- pano izleme, ekran görüntüsünden metin,
- kısayol tuşu.

Gelen içerik hangi konuya, hangi biçimde (bağlantı kartı, özet, alıntı) düşmeli? Sohbet düzenini
bozmadan nasıl gösterilmeli?

**7. Güvenilir hatırlatma ve "ısrarcı takip".**
Todoist/TickTick düzeyinde güvenilir hatırlatmanın (uygulama kapalıyken, yeniden başlatmadan sonra,
pil koruması altında) mimari bileşenleri neler olmalı?

Bunun üstüne, kullanıcı açıkça cevap verene kadar ("yaptım / ertele / bırak") işin peşini bırakmayan
ama kullanıcıyı bezdirmeyen bir **takip** davranışı nasıl kurgulanır?
- Sessiz saatler, toplantı farkındalığı, bildirim bütçesi.
- Görmezden gelinen uyarıların öğrenilmesi.
- Verilen sözler ve beklenen cevaplar ("Ayhan'dan teklif bekliyorum") nasıl takip edilir?

**8. Etkileşim dili.**
Basılı tutma, kaydırma ve emoji tepkileri gibi WhatsApp/Signal/Telegram'dan tanıdık hareketler bu
üründe nasıl **yeni anlamlar** kazanabilir? Örneğin emoji = öncelik/durum, sağa kaydırma = devam
notu.
- Tanıdıklık ile yeni anlam çakışınca ne olur?
- Erişilebilirlik ve yanlışlıkla tetiklenme nasıl ele alınır?

**9. Gizlilik ve AI yönlendirme.**
Hangi veri hangi AI'a gidebilir? Örneğin banka verisi, sesler, kişisel notlar; yerel model ya da bulut
model. Kullanıcı bunu nasıl görür ve yönetir?
- "Yerel öncelikli" sözü, AI'ın yararıyla nasıl dengelenir?

**10. Zamanla büyüme.**
Ürün yıllarca kullanıldığında hangi tasarım kararları sorun çıkarır?
- Arşivleme, özetleme, konu hiyerarşisinin derinleşmesi, eski etiketlerin anlamını yitirmesi.
- Başka bir uygulamaya taşınabilirlik (Obsidian/Markdown).

## Cevap biçimi

- Her soru için: **yaklaşımlar** (en az 3) → **karşılaştırma tablosu** (kullanıcı etkisi, sistem etkisi,
  risk, yapım zorluğu) → **hangi koşulda hangisi** → **bana soruların**.
- Sonunda: sorular arasında birbirine bağlı kararlar. Örneğin "5. sorudaki tercih 2. ve 7. soruyu şöyle etkiler".
- Kısa ve somut ol. Mümkünse küçük bir senaryoyla anlat: "Sabah 08:10'da kullanıcı…".

---

## Karşı modele ek olarak verilebilecekler

- Ekran görüntüleri: `docs/rehber/*.webp` (26 adım; sıra ve açıklamalar `docs/rehber/index.html` içinde).
- Ürün kuralları: `CLAUDE.md` (çözüm içermez).
- **Bilerek verilmeyenler** (karşı modeli bağlamamak için): `docs/DEGERLENDIRME.md` ve `docs/TALEPLER.md`.
  Karşı modelin önerileri geldikten sonra bunlarla karşılaştırılabilir.
