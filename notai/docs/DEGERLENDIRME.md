# NotAI — Genel Değerlendirme ve Yeni Mimari Tanımlar (2026-11-08)

> Bu belge kod içermez. Programın bugünkü durumunu, dış bağlantılarını, raporlarını, rakiplerden
> farkını ve sıradaki büyük konuların (kaydırarak sekme geçişi, basılı tutma menüsü, takvim/alarm,
> "sadık köpek" takip motoru) **tanımlarını** verir. PC'de kod yazarken bu belge ve
> `docs/PROGRESS.md` esas alınır.

---

## 1. Kısa durum

| Konu | Durum |
|---|---|
| Kod | Rust, ~34 000 satır; 11 crate (çekirdek, depo, AI, sunucu, senkron, arayüz, kabuk, CLI, konuşma eklentisi…). 226 otomatik test, hepsi yeşil. |
| Veritabanı | SQLite, 12 şema adımı (migration). Telefonda ve S24 sunucusunda aynı yapı. |
| Uygulama ekranları | Ana ekran (sohbet listesi), konu akışı, alt konu açılışı, Görevler, Takvim (hafta/ay), Hatırlatmalar, Ara/Sor, Haftalık gözden geçirme, Harcamalar · Varlıklar · Hareketler, Ayarlar. |
| Sunucu | 50 civarı uç (REST), zamanlayıcı (30 sn), AI işçisi, ses işçisi, Obsidian yazıcı. |
| Gerçek cihaz | **APK henüz hiç derlenmedi.** Android'e özgü parçalar (bildirime dokunma, konuşma eklentisi, konum izni) telefonda sınanmadı. |
| En kritik eksik | Telefon hatırlatmaları uygulama **açıkken** çalışıyor (30 sn'de bir yoklama). Android uygulamayı uyutunca hatırlatma telefondan gelmez; yalnız sunucunun ntfy bildirimi kalır. Alarm ve "sadık köpek" için bu çözülmeli (§8.3). |

## 2. Program ne yapıyor? (özellik haritası)

**Yakalama**
- Yazı (1 dokunuş), ses (telefon anında yazar, Whisper düzeltir), fotoğraf/kamera/belge, konum, kişi, liste, hızlı görev/hatırlatma/etkinlik.
- Her şey önce telefona yazılır (internetsiz). WhatsApp'tan yakalama yok (karar).

**Düzenleme (AI)**
- Her kayıt için AI önerileri: konu, görev, etiket, kişi, hatırlatma, harcama. Çip olarak gelir; kabul/yoksay; geri alınabilir.
- Kayıt net değilse NotAI soru sorar (en çok 2), cevaptan öğrenir.
- Otonom mod (isteğe bağlı): yüksek güvenli önerileri kendisi uygular.
- Konu hiyerarşisi: ana konu → alt konular (en çok 5 kat). Kayıtlar üst konulara ve Ana Günlük'e mükerrersiz yansır.
- Kullanıcıyı tanıma: "seni tanıma" bilgileri ve son kararlar AI'a örnek olur.

**Takip**
- Görevler (Bugün / Yaklaşan / Geciken; öncelik, tekrar).
- Hatırlatmalar: varsayılan 15 dk önce / saatsizse 09:00; ertele 10 dk / 1 sa / yarın.
- Sabah planı 09:00 (en çok 3 madde). Haftalık gözden geçirme.

**Yaşam verisi**
- Harcamalar: banka ekstresi PDF (yerelde okunur), iş/ev/kişisel, mükerrersiz.
- Varlıklar: menkul/gayrimenkul; ajan günde bir fiyatlar.
- Fiyat/ürün takibi: yalnız değişince haber.
- Konum hareketleri: "işe geldim" → kalış/yolculuk/masraf.

**Bilgiye ulaşma**
- Ara/Sor: tam metin + anlamsal arama; cevap yalnız kendi notlarından.
- Obsidian canlı klasörü.

## 3. Mimari ve veri alışverişi

```
 [Telefon: NotAI uygulaması]                         [S24 Ultra: ev sunucusu]
  Tauri 2 + Leptos (Rust→WASM)                        notai-server (axum) + SQLite
  SQLite (her şey önce burada)   ⇄ senkron (60 sn) ⇄  AI işçisi · ses işçisi · zamanlayıcı
  yerel bildirim · konum · ses                        Obsidian yazıcı · ntfy gönderici
            │                                                  │
            │ (Tailscale ile şifreli ağ)                        ├─ AI: NOTAI_AI = external → Hermes / Claude Code
            │                                                  │       (`notai` CLI ile işi alır, cevabı yazar)
            │                                                  │   ya da local (Ollama/Gemma) ya da anthropic (API)
            └──────────────────────────────────────────────────├─ Whisper (whisper.cpp, S24'te) — ses → yazı
                                                               ├─ Embeddings (Ollama bge-m3) — anlamsal arama
                                                               ├─ ntfy — telefona anlık bildirim
                                                               └─ Obsidian kasası (klasör; Syncthing vb. ile taşınır)
```

**Senkron.** Her değişiklik bir "değişiklik günlüğü"ne yazılır. Telefon dakikada bir kendi değişikliklerini
sunucuya iter, sunucununkileri çeker. Çakışmada son yazan kazanır; alan kurallarıyla desteklenir (karar
verilmiş öneri geri dönmez vb.). Senkronlanan varlıklar: konu, kayıt, etiket, öneri, görev, bilgi, ek,
hesap hareketi, satıcı kuralı, yer, hareket.

**Dışarı çıkan veri (gizlilik tablosu)**

| Veri | Nereye gider | Not |
|---|---|---|
| Kayıtlar, görevler, ekler | Yalnız kendi S24'ün (Tailscale) | Belirteçle korunur (`NOTAI_API_TOKEN`). |
| AI sınıflandırma metni | `external`: ajan neyse ona. **Claude Code kullanılırsa metin Anthropic'e gider.** `local` (Gemma): evden çıkmaz. `anthropic`: Anthropic API. | Ajanın hangi modeli kullandığı ayrı bir karar. |
| Banka ekstresi | Yerelde okunur; kurallarla okunamazsa ajana gider | "Banka verisini hangi AI işleyecek" kararı açık. |
| Ses | S24'teki Whisper | Telefon tanıyıcısı Google'ınsa (çevrimiçi) ses Google'a da gider; çevrimdışı Türkçe paketiyle gitmez. |
| Fiyat/varlık takibi | Ajan internette arar | Yalnız ürün adı/sembol gider. |
| Bildirim | ntfy | **ntfy.sh herkese açık bir sunucu**; kendi ntfy'ın yoksa konu adı gizli tutulmalı ya da S24'e ntfy kurulmalı. |
| Obsidian | Yerel klasör | Klasörü nereye eşitlediğin senin kararın. |

**Güvenlik ve dayanıklılık açıkları**
- Yedek yok: S24 SQLite'ı için günlük yedek (ör. `sqlite3 .backup` + Syncthing) tanımlanmalı.
- Veritabanı şifreli değil: telefon kaybına karşı Android cihaz şifrelemesine güveniliyor.
- Ekler değişiklik günlüğünde base64 duruyor: büyük ses/fotoğraf veritabanını büyütür; ileride dosya olarak ayrılmalı.

## 4. Çıkarabildiği raporlar

| Rapor | İçerik | Nerede |
|---|---|---|
| Sabah planı | Bugünün en önemli 3 işi, gecikenler | 09:00 bildirim + akış kartı |
| Haftalık gözden geçirme | İşlenmemişler, ilerlemeyen projeler (≥7 gün), yaklaşan 7 gün, geciken, geçen hafta biten | Ana ekran iğneli satır |
| Aylık harcama | Gelir/gider/birikim, iş/ev/kişisel, kategoriler, düzenli ödemeler, yorumlar (%20 birikim hedefi, artışlar, abonelikler) | Harcamalar |
| Varlıklar | Toplam, 30 günlük değişim (yeni varlık kazanç sayılmaz), türlere göre | Varlıklar |
| Hareketler | Yolculuk sayısı iş/özel, yolda süre, ulaşım türleri, yer başına kaç kez / ne kadar kaldın / ne harcadın, yol masrafı | Hareketler |
| Fiyat takibi | İlk sonuç, düşüş, daha ucuz bağlantı, hedef fiyat | Konu yazışması + bildirim |
| Ara/Sor | Soruya notlarından kaynaklı cevap | Ara/Sor |
| Obsidian | Konu notları, günlük notlar, görevler (Tasks biçimi), profil | Kasa klasörü |

**Eksik raporlar** (önerilen):
- Aylık "hayat özeti": iş/özel zaman dağılımı + masraf + tamamlanan işler.
- Proje sağlık kartı: son hareket, açık iş, bekleyenler, harcama.
- Kişi bazlı rapor: "Ayhan ile bu ay": konuşmalar, ödemeler, bekleyen sözler.
- Taahhüt raporu: verdiğin sözler / beklediğin cevaplar (§8.4).
- Dışa aktarım: PDF/Excel (muhasebeci için iş harcamaları).

## 5. Piyasadakilerden farkı

| | NotAI | Saner.AI | Todoist + Ramble | Google Keep + Gemini | Obsidian / Notion |
|---|---|---|---|---|---|
| Veri kimde | **Kendi telefonun + kendi S24'ün** | Bulut | Bulut | Google | Yerel / bulut |
| İnternetsiz | **Tam** | Hayır | Kısmen | Kısmen | Obsidian evet |
| AI ne yapar | Konu/görev/hatırlatma/harcama önerir, soru sorar, öğrenir; karar senin | Etiket, plan | Konuşmadan görev | Özet, soru | Eklentilerle |
| Türkçe | Türkçe kurallar (ekler, tarih, para, yer adları) | Zayıf | Orta | İyi | — |
| Hiyerarşi | Konu → alt konu, kayıt tek, yukarı yansır | Düz | Proje/bölüm | Düz | Klasör |
| Para / varlık / konum | **Hepsi aynı sistemde** | Yok | Yok | Yok | Elle |
| Hatırlatma / alarm | Var, ama arka planda zayıf (§8.3) | Var | **Çok güçlü** | Var | Zayıf |
| Takvim | Kendi takvimi; dış takvimle bağ yok | Google takvim | Google takvim | Google takvim | Eklenti |
| Ortak çalışma | Yok (tek kişi) | Yok | Var | Var | Var |
| Olgunluk | Prototip; cihazda sınanmadı | Ürün | Ürün | Ürün | Ürün |

**Asıl farkı:** "Not + görev + para + varlık + hareket" tek yazışma düzeninde, verisi senin
cihazlarında. Ayrıca AI'ın körü körüne değil, öneri + soru + geri al düzeniyle çalışması.

**Zayıf yanı:**
- Rakiplerin yıllardır cilaladığı alarm/takvim güvenilirliği henüz yok.
- Cihazda hiç denenmedi.

## 6. Geliştirilebilir yönler (öncelik sırasıyla)

1. **Güvenilir hatırlatma ve alarm** (Android AlarmManager, açılışta yeniden kurma) — §8.3.
2. **Sadık köpek takip motoru** — §8.4.
3. **Telefonda ilk gerçek derleme ve deneme** (APK; konuşma eklentisi; bildirime dokunma).
4. **Basılı tutma menüsü + kayıt düzenleme + çoklu seçim** — §8.2.
5. **Kaydırarak sekme geçişi** — §8.1.
6. Yedekleme (S24 → günlük yedek) ve geri yükleme.
7. Ana ekran widget'ı ve paylaş menüsü: başka uygulamadan "NotAI'ye paylaş" ile kayıt.
8. Kişiler: kişi kartı, kişiye bağlı söz/bekleyen, rehberden seçme.
9. Ekleri dosya olarak saklama (veritabanı şişmesin).
10. Raporların PDF/Excel dışa aktarımı.
11. Dağıtım için AI kurgusu (başkasına verirken) — açık karar.

## 7. WhatsApp arayüzüyle benzerlik ve farklar

| Öğe | WhatsApp | NotAI bugün | Not |
|---|---|---|---|
| Ana ekran | Sohbet listesi, yuvarlak profil, son mesaj, saat, okunmamış rozeti | **Aynı** (konular; rozet = karar bekleyen kayıt) | |
| Arama + filtre hapları | Var | **Var** (Tümü · İşlenmemiş · Proje · Alan · Kaynak) | |
| Sabit sohbetler | Var | **Var** (Ana Günlük, Haftalık, Harcamalar) | |
| Yeni sohbet düğmesi | Sağ altta | **Var** ("Yeni kayıt") | |
| Sohbet ekranı | Balonlar, gün ayraçları, en yeni altta | **Aynı**; karşı taraf = NotAI (soru, fiyat haberi) | |
| Yazma çubuğu | Hap alan, ataç + kamera içinde, mikrofon/gönder | **Aynı** | |
| Sesli mesaj | Basılı tut kaydet, kaydır iptal | **Farklı**: dokun–konuş–Gönder; metne çevrilir | Basılı tut seçeneği eklenebilir. |
| Alt sekmeler | Sohbetler · Güncellemeler · Topluluklar · Aramalar; **kaydırarak geçiş** | 5 sekme (Günlük · Görevler · Takvim · Hatırlatma · Ara/Sor); **kaydırma yok** | §8.1 |
| Basılı tutma | Seçim modu: yanıtla, yıldızla, kopyala, ilet, sil, sabitle; çoklu seçim | **Yok** | §8.2 |
| Mesajı kaydırınca yanıtla | Var | Yok | Karşılığı: "bu kayda ekle / alt not". |
| Bildirim | Anlık, sohbete götürür | Var (dokununca konu açılır; cihazda denenmedi) | |
| Sessize al | Var | Yok | Takip motoru için gerekli (§8.4). |
| Okundu bilgisi | Mavi tik | Yok | Karşılığı: "AI işledi / sen karar verdin" işareti. |
| Durum / hikâye | Var | Yok | Karşılığı: sabah planı kartı. |

## 8. Yeni tanımlar

### 8.1 Ana menüde kaydırarak sekme geçişi

**Amaç:** Üst düzey ekranlar arasında WhatsApp gibi sağa/sola kaydırarak geçmek.

**Kapsam.**
- Alt çubuktaki 5 sekme yatay bir şerit olur: Günlük ↔ Görevler ↔ Takvim ↔ Hatırlatma ↔ Ara/Sor.
- Harcamalar · Varlıklar · Hareketler bölmeleri de kendi içinde kaydırılır.

**Davranış kuralları.**
- **Yalnız üst düzey ekranlarda** çalışır. Konu yazışmasında ve sayfalarda kaydırma sekme değiştirmez.
  Yazışmada sağa kaydırma ileride "geri"ye ayrılabilir.
- **Ne zaman kaydırma sayılır:**
  - yatay hareket ≥ 56 px ve dikey hareketin 1,5 katından fazla, ya da
  - hızlı fiske (≥ 0,4 px/ms).
  - Aksi hâlde dikey kaydırmadır (liste kayar).
- **Kenar boşluğu:** Ekranın sol/sağ kenarındaki 24 dp Android'in "geri" hareketine bırakılır.
- **İçerik takibi:** Sayfa parmağı izler. Bırakınca en yakın sekmeye yerleşir.
  - Süre `motion.duration` belirtecinden gelir.
  - "Hareketi azalt" açıksa animasyonsuz geçer.
- **Komşu sekmeler:**
  - Komşu sekme önceden yüklenir (bir sağ, bir sol).
  - Bellekte en çok 3 ekran tutulur.
  - Sekmenin kaydırma konumu ve yarım kalmış yazı korunur.
- **Alt çubuk** her zaman görünür kalır; dokunarak geçiş aynen sürer (erişilebilirlik).
- **Yön:** İlk sekmede sola, son sekmede sağa kaydırma esneyip geri döner; döngü yok.

**Mimari tanım.**
- Arayüzde bir "sayfa şeridi" (pager) bileşeni: sekme listesi, etkin dizin, sürükleme durumu.
- Ekran durumu: `Screen` yapısı "üst düzey sekme" ve "iç ekran" diye ikiye ayrılır.
  Kaydırma yalnız üst düzeyi değiştirir.
- Dokunma olayları pointer events ile okunur. Çakışan iç kaydırmalar (yatay haplar, takvim şeridi)
  olayı kendisi tüketir; şerit yalnız tüketilmemiş hareketi alır.

**Kabul ölçütü:**
- 5 sekme arasında kaydırma çalışır.
- Dikey liste kaydırması bozulmaz.
- Android geri hareketi çalışır.
- Ekran okuyucu ile sekmeler alt çubuktan erişilebilir kalır.

### 8.2 Basılı tutunca seçenekler (seçim modu)

**Amaç:** WhatsApp'taki gibi bir öğeye basılı tutunca seçim moduna geçmek ve öğeye uygun eylemleri görmek.

**Tetikleyici.**
- 450 ms basılı tutma, hafif titreşim.
- Tutarken parmak 10 px'ten fazla kayarsa iptal olur (kaydırma sayılır).

**Seçim modu.**
- Üst çubuk eylem çubuğuna dönüşür: seçili sayı + eylem simgeleri + "⋮ Diğer".
- Seçimdeyken başka öğelere dokunmak onları da seçer (çoklu seçim).
- Geri tuşu ya da boşluğa dokunmak seçimi bitirir.

**Öğeye göre eylemler** (sıra = önem; ilk 3-4'ü simge, kalanı "Diğer" menüsünde):

| Öğe | Eylemler |
|---|---|
| Kayıt (balon) | Görev yap · Hatırlat… · Konuya taşı · Kopyala · Düzenle · Sabitle (konunun başına) · Paylaş · Metni seç · Bilgi (kaynak, AI kararları, ekler) · Sil (çöpe; "Geri al" şeridi) |
| NotAI mesajı / sorusu | Cevapla · Geç · Kopyala · Bağlantıyı aç · Sil |
| Konu satırı (ana ekran) | Sabitle · Sessize al (8 sa / 1 hafta / Her zaman) · Okundu say (rozetleri temizle) · Alt konu ekle · Arşivle · Sil |
| Görev | Tamamla · Bugün / Yarın / Seç… · Öncelik · Alarm kur (§8.3) · Düzenle · Sil |
| Hatırlatma | Ertele 10 dk / 1 sa / Yarın · Alarm'a çevir · Kapat |
| Harcama satırı | Kategori · İş/Ev/Kişisel · Böl (iki kapsama) · Kayda git · Sil |

**Kurallar.**
- **Toplu eylem:**
  - Çoklu seçimde yalnız ortak eylemler görünür (ör. 5 kayıt → Konuya taşı, Görev yap, Sil).
  - Tek bir geri alma kaydı oluşur.
- **Düzenle (yeni yetenek):**
  - Kaydın metni değiştirilebilir. Önceki metin "düzenlendi" işaretiyle saklanır.
  - Karara bağlanmamış AI önerileri yeni metinle yeniden üretilir; verilmiş kararlar korunur.
  - Senkronda metin alanı için "son yazan kazanır" geçerlidir.
- **Metni seç:**
  - WebView'un kendi metin seçimi basılı tutmayla çakıştığı için ayrı bir eylemdir.
  - Seçildiğinde balon seçilebilir hâle gelir (kopyala/paylaş Android'in kendi menüsünden).
- **Silme her zaman çöpe gider**, 5 sn "Geri al" şeridi çıkar (veri kaybı sıfır kuralı).

**Mimari tanım.**
- Arayüzde ortak bir "seçim bağlamı": seçili öğe türü + kimlikler, eylem kümesi, mod açık/kapalı.
- Eylemler `notai-api`de tek bir eylem listesi olarak tanımlanır; sunucu ve kabuk aynı eylem adlarını işler.
  Böylece toplu eylem tek istekte, tek işlemde (transaction) yapılır.
- Depoda yeni: "kayıt düzenleme" (metin + düzenleme geçmişi), "konu sessize alma" (bitiş zamanı),
  "kayıt sabitleme".

**Kabul ölçütü:**
- Basılı tutma kaydırmayla karışmaz.
- 3 öğe seçilip tek seferde konuya taşınır ve tek "Geri al" ile döner.
- Ekran okuyucuda eylemler "Diğer eylemler" menüsüyle erişilebilir.

### 8.3 Takvim, hatırlatıcı bağlantısı ve alarm

**Bugün:**
- Görevlerin tarih/saati ve hatırlatması var.
- Telefon bunları uygulama açıkken 30 sn'de bir yoklayarak bildiriyor.
- Sunucu ntfy ile ayrıca gönderiyor.
- Dış takvimle bağ yok.

**Hedef:** Uygulama kapalıyken, telefon uykudayken, yeniden başlatıldıktan sonra da hatırlatma ve alarm
**tam zamanında** çalsın. Takvim telefonun takvimiyle konuşsun.

**Üç uyarı türü:**

| Tür | Davranış | Örnek |
|---|---|---|
| Bildirim | Normal bildirim, sesli/sessiz telefon ayarına uyar | "Kombi servisini ara" |
| Alarm | Tam ekran, alarm sesi, ekran kilidinde açılır, "Ertele / Kapat / Yaptım"; sessiz modda da çalar | "06:30 havaalanına çık" |
| Sessiz | Yalnız uygulama içinde ve sabah planında | düşük öncelik |

**Mimari tanım.**
1. **Alarm planlayıcı (telefon, yerel):**
   - Kaynak her zaman yerel veritabanıdır.
   - Her değişiklikten sonra (görev ekle/düzenle/tamamla, senkronla gelen değişiklik) "önümüzdeki
     7 günün en yakın 50 uyarısı" hesaplanır ve Android'e kurulur. Fazlası iptal edilir (uzlaştırma).
     Bu hesap çekirdekte saf bir işlevdir (test edilebilir); Android yalnız uygular.
2. **Android tarafı (yeni Kotlin eklentisi "notai-alarm"):**
   - Bildirim türü: `AlarmManager.setExactAndAllowWhileIdle`.
   - Alarm türü: `setAlarmClock` + tam ekran (full-screen intent) etkinliği.
   - Telefon yeniden başlayınca (BOOT_COMPLETED) planlayıcı yeniden çalışır.
   - Gereken izinler: tam zamanlı alarm (USE_EXACT_ALARM), tam ekran bildirim, bildirim, açılışta başlama.
   - Bildirim eylemleri (Yaptım / Ertele 10 dk / 1 sa / Yarın) uygulamayı açmadan veritabanına yazar.
3. **Çift kanal ama tek sefer:**
   - Sunucu ntfy'ı yedek olarak gönderir, ancak telefon "kurdum ve çaldı" bilgisini senkronla bildirirse
     sunucu göndermez.
   - Bunun için kanalların ortak bir "uyarı günlüğü" (gönderildi / görüldü / cevaplandı) tutulur.
     Aynı uyarı iki kez gelmez.
4. **Takvim bağlantısı.** Önerilen yol iki parçadır:
   - **(a) Yazma:** Saatli görevler ve etkinlikler telefonda ayrı bir "NotAI" takvimine yazılır
     (Android CalendarContract). Samsung/Google Takvim'de görünür. Tek yönlüdür: düzenleme NotAI'de yapılır.
   - **(b) Okuma:** Telefondaki diğer takvimlerin (iş toplantıları) etkinlikleri NotAI Takvim'inde gri
     olarak görünür.
     - Sabah planı ve takip motoru bu dolu saatleri dikkate alır: toplantıdayken dürtmez.
     - Okuma izni: READ_CALENDAR.
   - **Alternatifler:**
     - Sunucudan ICS aboneliği: kolay, ama Google 8-24 saatte bir yeniler.
     - Google Calendar API ile çift yönlü: kimlik doğrulaması ağır; bulut bağımlılığı.
5. **Etkinlik türü:**
   - Bugün "saatli görev"dir.
   - İleride başlangıç–bitiş, yer, katılımcılar, tekrar kuralı olan ayrı "etkinlik" türü tanımlanır.
   - Takvimde blok olarak görünür.

**Kabul ölçütü:**
- Uygulama kapatılıp telefon yeniden başlatıldığında 2 dk sonraki alarm çalar.
- Uçak modunda çalar.
- Aynı hatırlatma hem telefondan hem ntfy'dan gelmez.

### 8.4 "Sadık köpek" — takip motoru

**Amaç:** Verdiğin sözleri, beklediğin cevapları ve ilerlemeyen işleri unutmamak. NotAI bir iş
kapanana ya da sen açıkça karar verene kadar (yaptım / ertele / bırak / devrettim) peşini bırakmaz.
Ama seni bezdirmez; dikkatini doğru zamanda ister.

**Kavramlar.**
- **Taahhüt:** takip edilen her şey.
  - **Görev:** "Kostik siparişini ver".
  - **Söz:** "Ayhan'a perşembe dönerim dedim".
  - **Bekleyen:** "Ayhan'dan teklif bekliyorum" (top karşıda).
  - **Durgun konu:** projede 7+ gündür hareket yok.
  - **Tekrarlayan yükümlülük:** her ayın 5'inde kira; abonelik yenilemesi; varlık değerlemesi.
  - AI bunları kayıttan çıkarır ("…dönerim dedim" → söz; "…bekliyorum" → bekleyen). Sen de basılı
    tutma menüsünden işaretleyebilirsin.
- **Onay (karar):** Her dürtme bir cevap ister: **Yaptım · Ertele (…) · Hâlâ bekliyorum · Bırak · Devrettim (kime)**.
  Cevap verilmeyen dürtme "görülmedi" sayılır; köpek tekrar gelir.

**Israr basamakları.**

| Basamak | Ne zaman | Nasıl |
|---|---|---|
| 0 · Sessiz | Vade uzak | Yalnız listede |
| 1 · Nazik | Vade günü / söz zamanı | Bildirim (sabah planında da) |
| 2 · Israrlı | Gecikti, cevap yok | Aynı gün çalışma saatinde 2-3 kez; bildirimde hızlı cevaplar |
| 3 · Gündem | 2+ gün gecikti | Sabah planının en üstü + ilgili konuda NotAI mesajı: "Kostik siparişi 3 gündür bekliyor; ne yapalım?" |
| 4 · Hesap sorma | 7+ gün ya da 3 kez ertelendi | Haftalık gözden geçirmede tek tek: "Hâlâ önemli mi? Bırak / Devret / Yeni tarih" |

**Bezdirmeme kuralları.**
- **Sessiz saatler:** 22:00–08:00, değiştirilebilir. Alarm türü hariç.
- **Takvim:** Toplantıdayken (§8.3 okuma) dürtme toplantı sonrasına kayar.
- **Günlük dürtme bütçesi:** en çok N (ör. 6) anlık bildirim. Fazlası tek bir "bugün bekleyen 4 iş" özetinde toplanır.
- **Sessize alınmış konu** dürtmez (yalnız basamak 4'te sorulur).
- **Öğrenme:**
  - Hep aynı saatte "ertele"yorsan dürtme o saate kayar.
  - 3 kez görmezden gelinen tür seyrekleşir ve "bu tür hatırlatmalar işine yarıyor mu?" diye sorulur.
- **Öncelik etkisi:** Yüksek öncelik basamakları hızlı çıkar; düşük öncelik basamak 2'yi atlar.

**Dikkate davet** ("bizi de dikkate davet etmesi").
NotAI yalnız vadesi gelenleri değil, dikkat isteyen örüntüleri de getirir. Her biri ilgili konuda bir
NotAI mesajıdır, hızlı cevaplıdır:
- "Sabun işi'nde 9 gündür kayıt yok; son konuşulan: numune."
- "Bu ay iş yolculuğu masrafın geçen aydan %40 fazla."
- "Ayhan'a 2 söz verdin, 1'i gecikti."
- "Kira 3 gün sonra; geçen ay 2 gün geç ödendi."

**Mimari tanım.**
- **Çekirdek (saf, test edilebilir):** "takip motoru" işlevi:
  - Girdiler: taahhütler, uyarı günlüğü, takvim doluluğu, ayarlar (sessiz saatler, bütçe), şimdi.
  - Çıktı: o anda gönderilecek dürtmeler (kime, hangi kanal, hangi basamak, hangi hızlı cevaplar).
  - Saat ve rastlantı dışarıdan verilir; aynı girdi aynı çıktıyı verir.
- **Veri:**
  - Görevlere ek olarak "söz" ve "bekleyen" türleri (kim, ne zamana kadar).
  - "Uyarı günlüğü": dürtme, kanal, zaman, cevap. Senkronlanır; iki cihaz aynı dürtmeyi göndermez.
  - Ayarlar: sessiz saatler, günlük bütçe, çalışma saatleri.
- **Çalıştığı yerler:**
  - Telefon: alarm planlayıcının yanında, internetsiz.
  - Sunucu: yedek; telefon sessizse ntfy.
  - Kim gönderirse uyarı günlüğüne yazar; diğeri atlar.
- **Kanallar:** uygulama içi şerit, telefon bildirimi (hızlı cevaplı), ntfy, sabah planı, akşam özeti
  (isteğe bağlı 19:00 "gün sonu: 2 iş açık kaldı"), haftalık gözden geçirme.
- **Ölçüm:** Kaç dürtme cevaplandı, kaçı görmezden gelindi, ortalama tepki süresi. Ayarlar ekranında
  "takip karnesi" olarak gösterilir.

**Kabul ölçütü:**
- Geciken bir görev için basamaklar sırayla gelir.
- Gece dürtmez.
- "Yaptım" deyince susar.
- Aynı dürtme telefon ve sunucudan iki kez gelmez.

## 9. Mimariye eklenecekler (özet)

| Katman | Yeni |
|---|---|
| Çekirdek | Takip motoru · alarm planlayıcı (uzlaştırma) · taahhüt türleri (söz, bekleyen) · kayıt düzenleme kuralları · seçim eylemleri listesi |
| Depo | Uyarı günlüğü (senkron) · kayıt düzenleme geçmişi · konu sessize alma · kayıt sabitleme · takvim bağları · ayarlar (sessiz saat, bütçe, çalışma saatleri) |
| Android | "notai-alarm" eklentisi (AlarmManager, tam ekran alarm, açılışta yeniden kurma, bildirim eylemleri) · takvim köprüsü (CalendarContract okuma/yazma) |
| Arayüz | Sayfa şeridi (kaydırarak sekme) · seçim modu + eylem çubuğu · uyarı türü seçimi (bildirim/alarm) · takvimde dış etkinlikler · takip karnesi |
| Sunucu | Takip motorunun yedek çalıştırıcısı · uyarı günlüğüne göre tekilleştirilmiş ntfy · yedekleme görevi |

## 10. PC'de yol haritası (dilimler)

Her dilim: çalışır + test edilmiş + `check.sh` yeşil + cihazda denendi.

1. **İlk APK:** `docs/ANDROID.md` ile derle, telefona kur. Konuşma eklentisi, bildirime dokunma,
   konum izni ve senkron cihazda denenir; çıkan hatalar düzeltilir.
2. **Alarm planlayıcı + notai-alarm eklentisi** (§8.3/1-3): uygulama kapalıyken tam zamanında; tek sefer.
3. **Basılı tutma + seçim modu + kayıt düzenleme** (§8.2).
4. **Kaydırarak sekme geçişi** (§8.1).
5. **Takip motoru v1** (§8.4): görev + bekleyen + söz; basamak 1-3; sessiz saat; bütçe; hızlı cevaplar.
6. **Takvim köprüsü** (§8.3/4): NotAI takvimine yazma + telefon takvimini okuma; toplantıda dürtmeme.
7. **Takip motoru v2:** basamak 4, dikkate davet örüntüleri, öğrenme, takip karnesi.
8. Yedekleme, ekleri dosyaya ayırma, raporların dışa aktarımı.

## 11. Açık kararlar

1. Uyarı türlerinin varsayılanı:
   - Yalnız açıkça "alarm" denince mi tam ekran alarm,
   - yoksa yüksek öncelikli görevlerde kendiliğinden mi?
2. Takvim yolu: önerilen telefon takvimi (yaz + oku) mi, Google Takvim API mi?
3. Sadık köpeğin varsayılan sertliği:
   - sessiz saatler,
   - günlük bildirim bütçesi,
   - akşam özeti açık mı?
4. Kaydırarak geçişte sekme sırası ve 5 sekme kalsın mı? WhatsApp'taki gibi 4'e inip Ara/Sor üstteki
   arama çubuğuna mı taşınsın?
5. Basılı tutmada "Görev yap" mı ilk eylem olsun, "Konuya taşı" mı?
6. Önceden açık kalanlar:
   - banka verisini işleyecek AI,
   - dağıtım için AI kurgusu,
   - ajanın (Claude Code) kayıt metinlerini Anthropic'e göndermesinin kabulü.
