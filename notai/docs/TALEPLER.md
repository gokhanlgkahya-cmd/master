# NotAI — Geliştirme Talepleri (2026-11-08)

> Kod yok; PC'de geliştirmeye başlarken bu liste esas alınır. Her talep: amaç, davranış, veri, kabul.
> Ayrıntılı mimari gerekçe: `docs/DEGERLENDIRME.md`. Kullanım rehberi (ekran görüntülü):
> https://claude.ai/artifact/QSxfC2fNLAk1ubJyPmWm4n

## Öncelik sırası

| # | Talep | Neden önce |
|---|---|---|
| T-01 | Todoist düzeyinde güvenilir hatırlatma ve alarm | Kaçırılan hatırlatma güveni bitirir; diğer takip özellikleri buna dayanır |
| T-02 | Basılı tutma menüsü (WhatsApp/Signal/Telegram ortak düzeni) | Yanıtla, ilet, emoji ve sil buradan açılır |
| T-03 | Yanıtla: kaydın devamı | T-02'nin ilk eylemi |
| T-04 | İlet: başka konuya kısayol (mükerrersiz) | T-02'nin ikinci eylemi |
| T-05 | Emoji ile önceliklendirme ve durum | T-02 tepki çubuğu |
| T-06 | Kayıt düzenleme, çoklu seçim, toplu eylem | T-02 ile birlikte |
| T-07 | Kaydırarak sekme geçişi | Bağımsız |
| T-08 | Takvim köprüsü | T-01'den sonra |
| T-09 | "Sadık köpek" takip motoru | T-01, T-05, T-08'e dayanır |
| T-10 | Küçük düzeltmeler (rehber çekimlerinde görülenler) | İstenen zamanda |

---

## T-01 · Todoist düzeyinde güvenilir hatırlatma ve alarm

**Todoist'in güvenilirliği nereden geliyor?**
1. **Telefona kurulan alarm.** Hatırlatma, Android'in alarm servisine (AlarmManager) tam zamanlı kurulur. Uygulama kapalıyken, telefon uykudayken de çalar.
2. **Yeniden kurma.** Telefon yeniden başlayınca, saat dilimi ya da saat değişince alarmlar yeniden kurulur. Yaz saati gibi geçişlerde kayma olmaz.
3. **İkinci kanal.** Sunucudan anlık bildirim (FCM) ve e-posta gelir. Aynı hatırlatma iki kez gösterilmez.
4. **Pil korumasına karşı rehberlik.** Samsung'un "uyku moduna alınan uygulamalar" listesinden çıkarmayı ister.
5. **Güçlü tekrar kuralları.** "Her ayın son cuması", "her 3 iş gününde bir", "tamamlandıktan 2 hafta sonra" gibi.
6. **Konuma bağlı hatırlatma.** "Markete varınca".
7. **Takvimle çift yönlü bağ, anlık senkron, saat/widget/hızlı ekleme** her yerden.

**Bizde bugün:** Hatırlatmalar yalnız uygulama açıkken 30 sn'de bir yoklanıyor. Yedek kanal olarak S24'ten ntfy var. Tekrar kuralları basit (günlük/haftalık/aylık). Konuma bağlı hatırlatma yok.

**Yapabilir miyiz? Evet.** Hiçbiri bulut gerektirmiyor; gerekenler:

| Parça | Ne yapar | Not |
|---|---|---|
| Alarm planlayıcı (çekirdek, saf) | Önümüzdeki 7 günün en yakın ≤50 uyarısını hesaplar; her değişiklikte kurulacak/iptal edilecekleri çıkarır | Test edilebilir; zaman dışarıdan verilir |
| `notai-alarm` eklentisi (Kotlin) | Normal uyarıyı `setExactAndAllowWhileIdle`, alarmı `setAlarmClock` + tam ekran ile kurar. Açılışta, saat ve saat dilimi değişince yeniden kurar. Bildirim eylemleri (Yaptım / Ertele / Yarın) uygulama açılmadan çalışır. | Eylemler bir kuyruğa yazılır, uygulama açılınca veritabanına işlenir; Android'de uygulama arka planda tam açılmayabilir |
| Pil koruması | İlk kurulumda "pil kısıtlamasını kaldır" adımı + Samsung için resimli yönerge | Samsung'da en sık kaçırma nedeni |
| Uyarı günlüğü (senkron) | Gönderildi / görüldü / cevaplandı | Telefon gösterdiyse sunucu ntfy göndermez → tek sefer |
| Tekrar kuralları v2 | Takvim standardı (RRULE) düzeyi + Türkçe doğal dil ("her ayın son cuması", "iş günleri", "tamamlanınca 3 gün sonra") | Ayrı dilim |
| Konuma bağlı hatırlatma | Yere varınca / yerden çıkınca. Hareketler'deki yerler kullanılır (geofence). | İsteğe bağlı; pil maliyeti var |
| Hızlı ekleme | Bildirim alanında kalıcı "+ kayıt", ana ekran widget'ı, paylaş menüsü | |
| Cihaz deneme listesi | Uygulama kapalı, telefon yeniden başlatıldı, uçak modu, pil tasarrufu, sessiz mod, saat dilimi değişti | Her sürümde elle |

**Yapılması zor olanlar:**
- Google Takvim ile çift yönlü bağ (kimlik doğrulama, bulut).
- Saat uygulaması (Wear OS) ve Google Asistan entegrasyonu.

Bunlar şimdilik kapsam dışı; telefonun takvimiyle köprü (T-08) yeterli.

**Kabul ölçütü:**
- Uygulama kapalı ve telefon yeniden başlatılmışken 2 dk sonraki alarm çalar.
- Uçak modunda çalar.
- Sessiz modda "alarm" türü çalar, "bildirim" türü telefonun ayarına uyar.
- Aynı hatırlatma telefondan ve ntfy'dan iki kez gelmez.
- Bildirimdeki "Ertele 10 dk" uygulamayı açmadan çalışır.

---

## T-02 · Basılı tutma menüsü (WhatsApp · Signal · Telegram ortak düzeni)

**Üç uygulamanın ortak kalıbı:**
- 400-500 ms basılı tutunca balon seçilir ve titreşim gelir.
- Balonun **üstünde** emoji tepki çubuğu çıkar (6-7 emoji + "+").
- Eylemler ya balonun **altında** yüzen bir menüde (Telegram, Signal) ya da **üst çubukta** (WhatsApp) durur.
- Balonu **sağa kaydırmak** yanıtlar (üçünde de).
- Seçim modunda başka balonlara dokunmak onları da seçer.
- Geri tuşu ya da boşluğa dokunmak kapatır.

**NotAI düzeni (önerilen):**
- **Tek basılı tutma:**
  - Arka plan karartılır (%40), balon hafifçe öne çıkar.
  - **Üstte tepki çubuğu (T-05):** kendi kaydında sağa, NotAI mesajında sola hizalı.
  - **Altta eylem menüsü:** dikey, en çok 260 dp geniş. Balon ekranın altına yakınsa menü yukarı açılır.
- **Menüde "Seç"** ya da ikinci bir balona basılı tutma: WhatsApp gibi üst çubuklu çoklu seçim (T-06).
- **Sağa kaydırma:** Yanıtla (T-03). Kaydırma eşiği 64 dp; ok simgesi belirir.
- **Animasyon ve dokunma:** Animasyonlar 150 ms, belirteçlerden. Dokunma hedefleri ≥ 48 dp. Ekran okuyucuda her eylem "Diğer eylemler" listesinde.

**Eylemler (sıra = önem):**

| Öğe | Menü |
|---|---|
| Kendi kaydın | Yanıtla · Kopyala · İlet · Görev yap · Hatırlat · Konuya taşı · Sabitle · Düzenle · Bilgi · Seç · Sil (kırmızı, en altta) |
| NotAI mesajı / sorusu | Yanıtla (= cevapla) · Kopyala · Bağlantıyı aç · İlet · Sil |
| İletilmiş kayıt (T-04) | Kaynağa git · Yanıtla · Kopyala · Bu konudan kaldır |
| Konu satırı (ana ekran) | Üst çubuk: Sabitle · Sessize al · Arşivle · ⋮ (Okundu say · Alt konu ekle · Sil) |
| Görev satırı | Tamamla · Bugün · Yarın · Tarih seç · Öncelik · Alarm kur · Sil |

**Kurallar:**
- **Kopyala:** Metni kopyalar. Çoklu seçimde WhatsApp biçimiyle birleştirir: `[08.11 10:41] metin`.
- **Sil:** Önce çöpe gider. Altta 5 sn "Geri al" şeridi çıkar (Telegram gibi; onay penceresi yok).
- **Metin seçme:** Basılı tutma menüye ayrıldığı için "Bilgi" sayfasında metin seçilebilir hâlde gösterilir.

**Kabul ölçütü:**
- Basılı tutma kaydırmayla karışmaz.
- Menü ekran kenarında taşmaz.
- Açık ve koyu temada okunur.
- Ekran okuyucuyla bütün eylemlere ulaşılır.

---

## T-03 · Yanıtla: kaydın devamı

**Amaç:** Eski bir kayda dönüp ekleme yapmak. Ek, yeni bir kayıt olarak değil, **o verinin devamı** olarak değerlendirilmeli.

**Davranış:**
- **Yanıt başlatma:** Sağa kaydır ya da menüde "Yanıtla". Yazma çubuğunun üstünde alıntı şeridi çıkar: kaynak konunun simgesi, ilk satır, ✕ (vazgeç).
- **Yanıtın görünümü:** Gönderilen yanıt, balonunun üstünde küçük alıntı kutusuyla görünür. Alıntıya dokununca asıl kayda gider ve onu kısa süre vurgular.
- **Konusu:** Yanıt, asıl kaydın konusuna aittir. Başka bir konudan yanıtlansa bile asıl konuda zincir olarak görünür.
- **Zincir görünümü:** Asıl kaydın altında "2 yanıt" bağlantısı olur. Dokununca zincir açılır (asıl + yanıtlar, zaman sırasıyla).
- **AI:** Yanıtı tek başına değil, **zincirle birlikte** okur.
  - "Perşembeye kaldı" yanıtı, asıl kayıttaki görevin tarihini değiştirmeyi önerir; yeni görev açmaz. Bunun için yeni öneri türü gerekir: *görevi güncelle*.
  - Zincir, asıl kaydın etiketlerini ve kişilerini devralır.
- **Arama:** Zincirdeki her yanıt ayrı bulunur, ama sonuçta asılla birlikte gösterilir.
- **Raporlar:** Yanıttaki harcama asıl kaydın kapsamını devralır; mükerrer sayılmaz.
- **Obsidian:** Yanıt, asıl bloğun altında girintili madde olarak yazılır.
- **NotAI mesajına yanıt:** Örneğin fiyat haberine "hedefi 48.000 yap" demek, ilgili ayarı değiştirmeyi önerir.

**Veri:**
- Kayda `reply_to` (asıl kayıt) eklenir. Senkronlanır.
- Zincirin kökü hesaplanabilir; ayrıca saklamaya gerek yok.

**Kabul ölçütü:**
- 3 gün önceki bir kayda yanıt verilir; yanıt asıl konuda zincirde görünür.
- AI yeni görev değil, tarih güncellemesi önerir.
- Ana Günlük'te tek satır olarak sayılır.

---

## T-04 · İlet: başka konuya kısayol (mükerrersiz)

**Amaç:** Bir kaydı başka bir konuda da okunabilir yapmak. WhatsApp'taki "ilet" gibi, ama **kopya değil, kısayol**.

**Davranış:**
- **İletme:** Menüde "İlet" → konu seçici. Arama, son kullanılanlar ve çoklu seçim vardır. İsteğe bağlı kısa not eklenebilir ("bunu da düşün").
- **Hedef konuda görünüm:**
  - Kayıt **solda**, gelen mesaj gibi durur.
  - Yanında **kaynak konunun yuvarlak simgesi** olur.
  - Üstünde "↪ Sabun işi'nden · 8 Kas" başlığı bulunur.
  - Varsa iletirken yazılan not da görünür.
- **Dokununca** kaynak konuda asıl kayda gidilir.
- **Tek kayıt, iki görünüm:**
  - Asıl kayıt düzenlenirse (T-06) iletilmiş görünüm de değişir.
  - Asıl silinirse iletilmiş yerde "Kayıt silindi (çöpte)" görünür; geri alınınca döner.
- **Mükerrer yok:**
  - Ana Günlük'te, aramada, raporlarda, harcama toplamlarında ve rozet sayılarında kayıt **bir kez** sayılır.
  - Hedef konunun "son mesaj" satırında "↪ iletildi: …" olarak görünebilir.
  - İletilen konuya ait alt/üst dal toplamlarına girmez.
- **Kaldırma:** Hedefte basılı tut → "Bu konudan kaldır". Yalnız bağ silinir, kayıt kalır.
- **Bilgi sayfası:** "Şu konularda da görünüyor: Ev, Muhasebe".
- **AI:** "Bu kayıt Ev konusunu da ilgilendiriyor" diye **iletme önerisi** verebilir (yeni öneri türü: *bağla*).
- **Obsidian:** Hedef konunun notuna kopya değil, blok gömme yazılır: `![[Sabun işi#^n1a2b3c]]`.
- **Hiyerarşi:** Kaydı zaten yansıdığı bir üst konuya iletmek anlamsızdır. Seçicide bu konular soluk gösterilir.

**Veri:**
- Yeni ilişki: **bağ** (kayıt, hedef konu, not, kim, ne zaman). Senkronlanır.
- Kayıt tablosu değişmez; bağ kaydın kopyası değildir.

**Kabul ölçütü:**
- "Sabun işi"ndeki bir harcama "Muhasebe"ye iletilir. Muhasebe'de soldan, kaynak simgesiyle görünür.
- Aylık harcama toplamı değişmez.
- Aramada bir kez çıkar.
- Kaynaktan düzenleme hedefe yansır.

---

## T-05 · Emoji ile önceliklendirme ve durum

**Amaç:** Basılı tutunca çıkan tepki çubuğuyla tek dokunuşta öncelik ve durum vermek.

**Varsayılan emojiler ve anlamları** (Ayarlar'dan değiştirilebilir):

| Emoji | Anlam | Etkisi |
|---|---|---|
| ❗ | Acil | Görev önceliği "Yüksek"; görev yoksa görev önerir |
| ⭐ | Önemli | Yıldızlılar listesine girer (ana ekranda filtre hapı) |
| ✅ | Yapıldı | Bağlı görev tamamlanır; kayıt "tamam" işareti alır |
| ⏰ | Hatırlat | Hızlı saat seçici açılır (1 sa · bu akşam · yarın 09:00 · seç) |
| 👀 | Takipte | "Sadık köpek" listesine girer (T-09) |
| ❓ | Sonra bak | Haftalık gözden geçirmeye düşer |
| + | Diğer | Serbest emoji; etiket gibi davranır |

**Görünüm:**
- Seçilen emoji balonun alt köşesinde küçük bir rozet olur (WhatsApp tepkisi gibi).
- Aynı emojiye tekrar dokunmak kaldırır.

**Veri:** Kayda bağlı "işaretler" kümesi. Senkronlanır; aynı alan için son yazan kazanır.

**Kabul ölçütü:**
- ❗ verilince görev "Yüksek" olur.
- ⭐ filtresi çalışır.
- Kaldırınca etkisi geri alınır.

---

## T-06 · Kayıt düzenleme, çoklu seçim, toplu eylem

**Düzenle:**
- Kaydın metni değiştirilir; balonda "düzenlendi" yazar.
- Önceki metin saklanır ("Bilgi"de görülür).
- Kararı verilmemiş AI önerileri yeni metinle yeniden üretilir; verilmiş kararlar korunur.

**Çoklu seçim:**
- Üst çubukta seçili sayı ve eylemler görünür.
- Yalnız ortak eylemler gösterilir: İlet · Konuya taşı · Görev yap · Kopyala · Sil.

**Toplu eylem:** Tek işlemde yapılır ve tek "Geri al" ile döner.

**Veri:** Kayıt düzenleme geçmişi.

---

## T-07 · Kaydırarak sekme geçişi

- Alt sekmeler arasında sağa/sola kaydırmayla geçilir. Harcamalar · Varlıklar · Hareketler bölmeleri de aynı şekilde.
- Yalnız üst düzey ekranlarda çalışır; yazışmanın içinde kaydırma sekme değiştirmez.
- Hareket kuralları:
  - Yatay hareket ≥ 56 px ve dikeyin 1,5 katı, ya da hızlı fiske.
  - Ekran kenarındaki 24 dp Android'in "geri" hareketine bırakılır.
- Komşu sekmeler önceden yüklenir; kaydırma konumu ve yarım kalan yazı korunur.
- Alt çubuk her zaman çalışır.
- **Açık karar:** Sekme sayısı 5 mi kalsın, yoksa WhatsApp gibi 4'e mi insin (Ara/Sor üstteki arama çubuğuna)?

---

## T-08 · Takvim köprüsü

- **Yazma:** Saatli görev ve etkinlikler telefonda ayrı bir "NotAI" takvimine yazılır; Samsung/Google Takvim'de görünür. Tek yönlü.
- **Okuma:** Telefondaki diğer takvimlerin etkinlikleri NotAI Takvim'inde gri görünür.
  - Sabah planı ve "sadık köpek" toplantıdayken dürtmez.
- Ayrı **etkinlik türü:** başlangıç–bitiş, yer, katılımcı, tekrar kuralı.
- **Açık karar:** Telefon takvimi mi, yoksa Google Takvim API ile çift yönlü bağ mı?

---

## T-09 · "Sadık köpek" takip motoru

Ayrıntı `docs/DEGERLENDIRME.md` §8.4'te; özet:
- **Takip edilenler:** görev, söz ("perşembe dönerim"), bekleyen ("Ayhan'dan teklif bekliyorum"), durgun konu, tekrarlayan yükümlülük, 👀 işaretliler.
- **Her dürtme kesin cevap ister:** Yaptım · Ertele · Hâlâ bekliyorum · Bırak · Devrettim. Cevap gelmezse tekrar gelir.
- **Israr basamakları:** nazik → ısrarlı → sabah planının başı + konuda NotAI mesajı → haftalık "hâlâ önemli mi?".
- **Bezdirmeme:** sessiz saatler, toplantıda erteleme, günlük bildirim bütçesi, sessize alınmış konu, görmezden gelinen türü seyreltme.
- **Dikkate davet:** "9 gündür hareket yok", "yol masrafı %40 arttı", "Ayhan'a verdiğin sözlerden 1'i gecikti".
- **Açık karar:** varsayılan ısrar düzeyi.

---

## T-10 · Küçük düzeltmeler (rehber çekimlerinde görüldü)

1. **Görevi düzenle:**
   - Görevin kurulu hatırlatması varken "Önce hatırlat" alanı "Değiştirme" gösteriyor. Mevcut değer gösterilmeli (ör. "1 saat önce").
   - Aynı ekranda "Tekrar" alanı için de geçerli.
2. **Başlık simgesi:** Harcamalar · Varlıklar · Hareketler başlığında simge hep cüzdan. Sekmeye göre değişmeli (ev / konum).
3. **Hareketler örnek görünümü:** Ardışık hareketlerde "Yolda 0 dk" görünüyor. 1 dk'nın altındaki yolculuklar süreye katılmasın ya da "—" gösterilsin.
4. **Ara/Sor:** AI kapalıyken "AI kapalı; ilgili kayıtlar" bilgisi var. Ayarlar'a götüren bir bağlantı eklenmeli.

---

## Gökhan'ın notları (cevap değil, akılda tutulacaklar)

### N-1 · Üretim kaydı, sonrasındaki takibi doğurmalı (2026-11-08)
> "Bugün 100 adet Şirinler sabunu bastık ama daha kimseye verilmedi. Bu kayıt, sevkiyat ve tahsilat
> sonuçlarının takibini dikte etmeli. Burada akıllı gibi olan bu."

**Anlamı:** Bir kayıt yalnız olanı değil, **olması gerekeni** de doğurur. NotAI bunu "açık uç" olarak tutar:
- **Ne açılır:** Üretim kaydı (100 adet Şirinler) stokta 100 adet açar. Bu, "henüz sevk edilmedi" durumundaki bir **açık uç** demektir.
- **Sevkiyat geldikçe:** "Kavlak'a 40 Şirinler" gibi sevkiyat kayıtları açık ucu azaltır: kalan 60. Her sevkiyat da kendi açık ucunu açar: tahsilat bekleniyor.
- **Tahsilat geldikçe:** Tahsilat kaydı sevkiyatın açık ucunu kapatır.
- **Sadık köpek:** Açık uçları izler.
  - "Şirinler'den 60 adet 10 gündür depoda."
  - "Kavlak'a giden 40 adetin parası 15 gündür görünmüyor."
  - Senden açık cevap ister: sevk ettim / bekliyor / iptal / ödendi.
- **Hangi kayıt neyi açar:** Her kayıt türü (üretim, sevkiyat, satış, sipariş, söz) hangi açık ucu açtığını ve neyin onu kapattığını bilir. Bu, alt konu şablonlarının (T-14/T-15) bir parçası olur.
- **Raporlar:** "Depoda bekleyen ürün", "tahsil edilmemiş sevkiyat", "üretimden satışa ortalama süre".

### N-2 · Tanımsız kayıt kalmasın: 5S mantığı (2026-11-08)
> "Foto çektim attım, bir şey yazmadım. Bunun ne olduğunu, nereye konumlanacağını öğrenmek istemesi
> lazım. Sınıflandırılmayan not karmaşa sebebidir. 5S mantığı: çöpleri at, tanımsız bir şey kalmasın,
> düzenli tut, disipline et."

**Kural:** Hiçbir kayıt tanımsız kalmaz. Açıklamasız fotoğraf, ses ya da belge geldiğinde NotAI sorar.
- **Önce tahmin eder, hızlı cevaplı sorar:** "Bu fotoğraf ne? Fatura · Ürün/etiket · Müşteriye ait · Sevkiyat · Başka…"
  - Görüntüdeki yazı okunur ve görüntü anlaşılmaya çalışılır. Gemma 3'ün görüntü okuyabilen sürümü ya da büyük ajan kullanılabilir.
  - Tahminler ilgili konulardan ve kartlardan seçilir.
- **Cevapsız kalırsa sadık köpek gibi geri gelir:** hemen soru → akşam özeti → haftalık gözden geçirme.
- **Sessizce kaybolamaz:** "Diğer olarak bırak" demek de bir karardır ve kaydı tanımlı sayar.

**5S karşılıkları:**

| 5S | NotAI'de |
|---|---|
| **Ayıkla** (Seiri) | Anlamsız, tekrar ya da eskimiş kayıtlar için "atayım mı?" önerisi (çöpe; geri alınabilir). Aynı fotoğrafın iki kez gelmesini yakalama. |
| **Düzenle** (Seiton) | Her kaydın bir yeri olur: konu, alt konu ya da kart. Konunun üstündeki "dağıtılmamış" bölümü boşaltılmaya çalışılır. |
| **Temizle** (Seiso) | Eskiyen bilgiyi güncelleme, kapanmış açık uçları kapatma, kopyaları birleştirme. |
| **Standartlaştır** (Seiketsu) | Alt konu şablonları. Tek adlandırma ("Kavlak" / "Kavlak Market" → tek kart). Aynı tür kayıt hep aynı kutucuklarla tutulur. |
| **Disiplin** (Shitsuke) | Sadık köpek + günlük ve haftalık "düzen turu". "Düzen karnesi": tanımsız kayıt sayısı, bekleyen açık uç, birleştirilmemiş kopya. Hedef: tanımsız kayıt sayısı 0. |

### N-3 · Ortak defteri (Anıl ile) fotoğrafla aktarma (2026-11-08)
> "İş için ortağım Anıl ile bir defter tutuyoruz. Fotoğraf çekip gönderdiğimde gelir, gider, satılan
> mal, alınan mal miktarlarını not alıp gerekli yerlere işlemesi lazım."

**Defterin düzeni (her satır):**

| Tarih | Anıl · tanım | Anıl · gelir | Anıl · gider | Gökhan · tanım | Gökhan · gelir | Gökhan · gider |
|---|---|---|---|---|---|---|

- Her satıra bir açıklama ve tutar yazılıyor.
- Ortaklar arası nakit de deftere işleniyor. Anıl Gökhan'a nakit verirse: Anıl'da **gider**, Gökhan'da **gelir**.

**İstenen davranış:**
1. **Fotoğraf → tablo.**
   - Sayfa fotoğrafı ek olarak saklanır.
   - El yazısı satır satır okunur: tarih, kişi, tanım, gelir/gider, tutar.
   - El yazısı Türkçe okumak küçük Gemma'nın işi değil; görüntü okuyabilen büyük modele (ajan) gider.
2. **Satır satır teyit.**
   - Okunan tablo, sayfa görüntüsünün yanında gösterilir; her satırda ✓ ya da düzelt.
   - Para olduğu için toplu ve sessiz kabul yok.
   - Sayfada toplam yazıyorsa okunan satırların toplamıyla karşılaştırılır; tutmazsa uyarı verilir.
3. **Mükerrersiz.**
   - Aynı sayfa ileride yine çekilir (defter doldukça). Daha önce aktarılmış satırlar tanınıp atlanır, yalnız yeni satırlar eklenir. Tanıma ölçütü: tarih + kişi + tanım + tutar. Banka ekstresindeki yöntemin aynısı.
   - Defterdeki satır, NotAI'ye daha önce yazılmış bir kayıtla aynı olay olabilir. Örneğin "Kavlak'a 500 adet" kaydı ile defterdeki "Kavlak 500 ad · 12.000". Bu durumda ikisi eşleştirilir, iki kez sayılmaz.
4. **Gerekli yerlere işleme.**
   - **Para:** İş gelir/gideri olarak harcamalara işlenir (kapsam: iş, konu: Sabun işi), kim ödedi/aldı bilgisiyle.
   - **Mal:** Tanımdaki miktarlar ("Kavlak 500 ad Şirinler", "shrink 2 rulo") Satılan sabun / Satın alınan… alt konularına işlenir. Müşteri ve ürün kartlarına bağlanır, stok ve açık uçlar (N-1) güncellenir.
   - **Ortaklar arası nakit:** "Anıl gider + Gökhan gelir, aynı gün, aynı tutar, nakit" bir **iç transferdir**.
     - İşin gelir/gider raporuna girmez.
     - Ortakların **cari hesabını** değiştirir: kim işe ne koydu, ne çekti, kim kime borçlu.
5. **Raporlar.**
   - İşin aylık gelir/gider ve kârı.
   - Her ortağın cari durumu.
   - Satılan ve alınan mal miktarları (kg/adet, müşteri ve ürün bazında).
6. **Öğrenme.**
   - El yazısındaki özel adlar sözlüğe girer (Şirinler, Kavlak, Mete Zeytin, shrink).
   - Düzelttiğin okumalar sonraki sayfalarda kullanılır.

**Yeri:** Sabun işi altında "Ortak defter" (Anıl kartı = ortak). Her defter satırı oradan ilgili alt konulara ve kartlara bağlanır; kopyalanmaz.

**İleri adım (karar değil):** Anıl da NotAI'ye girebilirse defterin dijitali tutulur; fotoğraf yalnız geçiş için kalır. Bu, şu anki tek kullanıcı varsayımını değiştirir.

### N-4 · İkon: anlaşılınca üretilir, oturunca sabitlenir (2026-11-08)
> "İçerik güncellenip ne olduğu anlaşıldığında ikon üretilir veya webden bulunur; görünürlüğün artması
> hedeflenir. Kalıplaşmış ikon güncellenmez; kullanıcı talebi olursa güncellenir veya kullanıcı kendi
> fotoğrafını yükleyebilir."

**Bugün:** İkon, konu adı ve son kayıtlardaki anahtar kelimelerle sabit bir simge setinden seçiliyor. Kayıt geldikçe **değişebiliyor**.

**İstenen:**
1. **Belirsiz dönem:** Konu yeniyken ve ne olduğu anlaşılmamışken geçici simge kullanılır (tür simgesi + renk).
2. **Anlaşılınca:** Konu anlaşıldığında daha tanınır bir ikon üretilir ya da bulunur. Örneğin bir müşterinin logosu, ürünün görseli, markanın simgesi.
   - Kaynaklar: yerel simge seti → webden logo/favicon (kurum ve marka için) → AI ile üretilen basit simge.
   - Webden aramada yalnız konu adı dışarı gider.
3. **Kalıplaşma:** İkon bir kez "oturunca" sabitlenir; kayıt geldikçe artık değişmez. Ne zaman oturduğu kararı açık (aşağıdaki soru listesi).
4. **Kullanıcı her zaman önce gelir:** "Simgeyi değiştir" ya da kendi fotoğrafını yükleme her zaman mümkündür ve otomatik seçimin üstündedir. Bugün de var.
5. **Görünürlük:** Ana ekranda ikonlar birbirinden kolay ayrılmalı; aynı simge ve renk iki konuda yan yana gelmemeli.

### N-5 · WhatsApp gibi sıralama ve renkli rozetler (2026-11-08)
> "Son giriş yapılan sohbet yukarı gelir. Sistemden gelen hatırlatma, mesaj vb. sayılı, renkli rozetle
> o da yukarı gelsin; sanki birileriyle yazışıyoruz. Okundukça rozet rengi değişecek, sınıflandırılınca
> kalkacak. Sınıflandırma ve atama gereken işler farklı renkte rozetle bizi yönlendirmeli."

- **Sıralama:** Ana ekranda ve her konunun içinde, en son hareket gören sohbet en üste çıkar. Hareket sayılanlar: senin kaydın, NotAI mesajı, hatırlatma, soru, fiyat haberi. İğneliler en üstte kalır.
- **Üç rozet durumu:**

| Durum | Rozet | Ne zaman kalkar |
|---|---|---|
| Okunmamış (yeni kayıt, NotAI mesajı, hatırlatma) | Dolgun renk + sayı | Açıp görünce "okundu" rengine döner |
| Okundu ama iş bekliyor (sınıflandırma, atama, soru, onay) | Farklı renk + sayı (ör. turuncu) | İş bitince (yerine kondu, cevaplandı) |
| Tamam | Yok | — |

- **Yukarı taşınma:** Alt konudaki rozet üst konuya ve ana ekrana toplanarak çıkar. Sabun işi'nde 3 varsa içine girince hangi alt başlıkta olduğu görünür. Telegram'daki klasör sayaçları gibi.
- **Konu içinde:** Okunmamış kayıtlar "okunmamış mesajlar" çizgisiyle ayrılır. Ekranda göründükçe okundu sayılır.
- **Sadık köpek bağı:** "İş bekliyor" rozeti sıfırlanmadıkça günlük düzen turunda hatırlatılır (N-2).

### N-6 · İşe özel "uzmanlık paketleri" ve modüler güncelleme (2026-11-08)
> "Kayıt, sınıflandırma, formül, rapor, hatırlatma ana kemik olmak üzere işlere özel ehilleşen bir
> sistem olmalı. Şablonlar kullandıkça oluşmalı, başkalarına konusu anlaşıldığında önerilmeli; ileride
> GitHub/HuggingFace'e hazır skill olarak yüklenebilmeli. Güncellemeler genel değil modüler olmalı;
> herkes kullanmayacağı skilli programla taşımamalı."

**Ana kemik (çekirdek, herkeste aynı):** kayıt · sınıflandırma · formül · rapor · hatırlatma/takip.

**Uzmanlık paketi:** Bir işin kurallarını taşıyan, çekirdeğe takılıp çıkarılabilen paket.
- **İçerik:**
  - tanıma ipuçları,
  - alt konu yapısı,
  - kayıt şablonları (kutucuklar),
  - kartlar,
  - **formüller** (deterministik, test edilebilir; LLM değil),
  - açık uç kuralları (N-1),
  - raporlar,
  - hatırlatma ve takip kuralları,
  - yerel LLM için uzmanlık: konu sözlüğü, küçük istemler, örnekler ve paketin kendi **sınav kâğıdı**,
  - gizlilik etiketleri.
- **Örnek paketler:**
  1. **Ortak cari** (Anıl–Gökhan): Her ortak için net = aldığı gelir − ödediği gider. Toplam net ikiye bölünür; fazlası olan, farkın yarısını diğerine borçlu sayılır. Örnek: Anıl +30.000, Gökhan +10.000 → Anıl, Gökhan'a 10.000 öder. Uzun süreli kayıt; ortak defter (N-3) bu paketle işler.
  2. **Seans takibi** (psikolog):
     - **Kartlar:** danışan kartı (kod adla), seans kaydı (tarih, süre, ücret, ödeme), haftalık program.
     - **Kayıt:** sesli seans notu.
     - **Raporlar:** aylık seans sayısı ve gelir, ödenmemiş seanslar.
     - **Hatırlatmalar:** seans öncesi, ödeme.
     - **Gizlilik:** En sıkı düzeyde. Yalnız cihazda kalır, buluta ve dışa aktarıma gitmez. KVKK.
  3. **Al-sat takibi** (finansal işlemler):
     - **Kart:** enstrüman (sembol, pozisyon, giriş, stop, hedef).
     - **Takip:** fiyat takibi (mevcut varlık ve fiyat takibi modülleri); stop'a yaklaşınca uyarı, stop gelince alarm (T-01).
     - **Raporlar:** günlük ve aylık kâr/zarar.
     - **Bağlayıcı:** fiyat verisi kaynağı.
- **Nereden gelir:** Kullandıkça oluşur. AI bir konudaki tekrar eden düzeni fark eder ("bu konuda seans, ücret ve haftalık düzen var") ve hazır paket önerir ya da senin kayıtlarından yeni paket taslağı çıkarır. Sen onaylarsın, yerel paket olur. İstersen kişisel veriden arındırılıp paylaşılır (GitHub / HuggingFace).
- **Modüler güncelleme:**
  - Çekirdek küçük kalır.
  - Her paketin kendi sürümü vardır; yalnız kurulu paketler güncellenir.
  - Claude Code bağlandığında iyileştirmeyi paket paket yapar: önce paketin sınav kâğıdını çalıştırır, sonra değiştirir, sonra yeniden sınar.
  - Kullanılmayan paket programda taşınmaz.
- **Güvenlik:** Paket **veridir, kod değil**: şablon, formül dili, kural. Paylaşılan paket cihazda keyfi kod çalıştıramaz. Kod gerektiren ender durumlar (dış fiyat kaynağı bağlantısı gibi) ayrı ve izole "bağlayıcı" olarak, açık izinle çalışır.

## T-16 · NotAI'ye gömülü küçük ajan çekirdeği (Rust) — öneri
Hermes'in Python kodunu gömmek yerine, gereken çekirdeği Rust ile NotAI'ye yazmak:
- LLM sürücüsü (Gemma / OpenAI uyumlu / Anthropic; büyük kısmı var),
- araç kullanma döngüsü (adım ve süre sınırlı),
- araçlar: NotAI içi araçlar + web arama/okuma + bildirim/Telegram'a gönderme,
- tetikleyiciler: kayıt, saat, Asistan sohbeti; iş bitince kapanır,
- yetki ve onay.

Aynı kod hem uygulamada hem sunucuda çalışır. Başkasına gönderilen sürümde Termux gerekmez. Uzmanlık paketleri (N-6) bu çekirdeğin araç ve kural kaynağıdır.
