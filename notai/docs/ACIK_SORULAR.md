# NotAI — Açık sorular (PC'ye geçmeden önce cevaplanacak)

> Derleme: 2026-11-08. Her sorunun yanında **önerim** var; cevap vermezsen öneri varsayılan olur.
> Kaynak: `DEGERLENDIRME.md`, `TALEPLER.md`, `TASARIM_NOTLARI.md`.
> **Nasıl cevaplanır:** Sorunun altına `> Cevap: …` satırı ekle; yalnız önerime katılmadıklarını yazman da yeter.
> Cevapsız bırakılan soruda öneri geçerli sayılır.

## A. Altyapı ve kurulum
1. S24 sürekli şarja takılı mı, yoksa şarj gerçekten bitiyor mu? — *Önerim: şarjda kalsın, "Pil koruması %85" açık olsun.*
2. Hermes hangi modeli kullanıyor (API anahtarı mı: OpenRouter/Nous/Anthropic; yoksa yerel)? Hermes'in tek seferlik komut modunun adı ne (S24'te `hermes --help` çıktısı)?
3. Ajan sürekli açık mı kalsın, yoksa iş gelince mi uyansın? (T-12) — *Önerim: iş gelince uyansın; Ubuntu (proot) kaldırılsın.*
4. NotAI, Hermes'e MCP aracı olarak açılsın ve uygulamada bir "Asistan" sohbeti olsun mu? (T-11) — *Önerim: evet.*
5. Bildirimler ntfy.sh'den mi (herkese açık sunucu), yoksa S24'e kurulacak kendi ntfy'ından mı? — *Önerim: kendi ntfy'ın.*
6. Yedek nereye alınsın (PC, harici disk, Syncthing klasörü)? Ne sıklıkla? — *Önerim: günde bir, PC'ye Syncthing ile.*
7. PC hangi işletim sisteminde (Windows mu)? İlk APK derlemesi ne zaman yapılsın? — *Önerim: PC'ye geçince ilk iş.*
8. Whisper S24'e kurulsun mu? Hangi model (large-v3-turbo mu, daha hafif small mu)? — *Önerim: önce large-v3-turbo, yavaşsa small.*

## B. Yapay zekâ ve gizlilik
9. Banka verisini (ekstre) hangi AI işlesin: yalnız yerel kurallar + Gemma mı, gerektiğinde büyük ajan mı? — *Önerim: kurallar + Gemma; okunamayan sayfa sana sorularak büyük ajana.*
10. Claude Code kullanıldığında kayıt metinlerinin Anthropic'e gitmesi kabul mü? Kabulse hangi konular hariç tutulsun (ör. para, sağlık)?
11. Gemma: tam olarak hangi model (Gemma 3 4B / 3n E4B / E2B), hangi araçla (Ollama?) ve hangi sıkıştırmayla?
12. Gemma'nın geçmişte yanlış anladığı 5–10 gerçek örnek ve beklediğin doğru cevap ("sınav kâğıdının" ilk maddeleri).
13. "Jev klonu / laya" = eğitilmiş açık model (Llama) ile sınıflandırma mı? — *Önerim: konu seçimi için hafif sınıflandırıcı + kutucuk doldurmak için Gemma; veri birikince ek eğitim.*
14. Otonom moda güven: "Etiket ve konuyu sormadan koy, paraya ve tarihe dokunma" uyar mı? — *Önerim: evet.*
15. Uygulamayı başkasına verirken AI nasıl kurulacak? (Ertelenebilir.)

## C. Konular, kartlar, sınıflandırma (T-14, T-15, N-1, N-2)
16. Hiyerarşi en çok 3 kat olsun mu? (Kodda bugün 5.) — *Önerim: 3.*
17. Sabun işi'nin alt konu listesi (örneğin: Satılan sabun · Alınan para · Satın alınan shrink/malzeme · Üretim · Ortak defter)?
18. Konunun üstündeki kendi akışı yalnız **dağıtılmamış** kayıtları mı göstersin, yoksa tümünü mü? — *Önerim: yalnız dağıtılmamışlar.*
19. Ana sohbette alt konuların kayıtları tam mı görünsün, yoksa "Satılan sabun: 3 yeni" gibi katlanmış mı? — *Önerim: Ana Günlük'te tam, konu sohbetinde katlanmış.*
20. Müşteri kartları ana konu seviyesinde mi yaşasın (satış ve tahsilat aynı kartta)? — *Önerim: evet.*
21. Ürün kartları da olsun mu ("80 g zeytinyağlı", "150 g eşek sütlü", "Şirinler")? — *Önerim: evet; "hangi üründen kaç kg" raporu için gerekli.*
22. AI yeni kart açmak için ne zaman sorsun? — *Önerim: ilk birkaç kez sorsun, sonra öğrenip sormadan açsın ve akşam özetinde söylesin.*
23. Açık uç süreleri: depoda bekleyen ürün için kaç gün, tahsil edilmemiş sevkiyat için kaç gün sonra dürtsün? — *Önerim: 10 gün / 15 gün.*
24. Açıklamasız fotoğraf geldiğinde soru hemen mi gelsin (bildirim), yoksa akşam toplu mu? — *Önerim: uygulama açıksa hemen, değilse akşam özetinde.*

## D. Ortak defter (N-3)
25. Defter sayfalarında toplam satırı var mı? (Varsa okunan toplamla karşılaştırırız.)
26. Ortak cari hesabın başlangıç bakiyesi ne (bugüne kadar kim işe ne koydu)?
27. Anıl ileride NotAI'yi kullanacak mı? (Kullanacaksa programı çok kullanıcılı yapmak gerekir.) — *Önerim: şimdilik fotoğrafla aktarım.*

## E. Arayüz ve etkileşim (T-02…T-07, N-4)
28. Alt sekmeler: 5 mi kalsın, WhatsApp gibi 4'e mi insin? — *Önerim: 5; ileride raporlar için karar 33'e bağlı.*
29. Basılı tutmada ilk eylem ne olsun: "Yanıtla" mı, "Görev yap" mı? — *Önerim: Yanıtla.*
30. Emoji anlamları: ❗ acil · ⭐ önemli · ✅ yapıldı · ⏰ hatırlat · 👀 takipte · ❓ sonra bak uygun mu?
31. İkon ne zaman "kalıplaşmış" sayılsın: belli sayıda kayıttan sonra mı, sen onaylayınca mı? Webden logo aramaya izin var mı? — *Önerim: AI önerir, sen onaylayınca sabitlenir; webden arama açık.*
32. Görevler ekranında bir değişiklik (tarih değişti, tamamlandı) sohbette görünsün mü? — *Önerim: yalnız önemli değişiklikler, tek satır.*
33. Raporlar nereden açılsın? (Görsel: "NotAI Raporları" sayfası.) — *Önerim: ana ekranda iğneli "Raporlar" satırı + her konunun içinde durum/rapor kartı + Ara/Sor'dan "bu ay Mete Zeytin'e kaç adet?" gibi serbest soru.*

## F. Hatırlatma, takvim, takip (T-01, T-08, T-09)
34. Tam ekran alarm ne zaman kurulsun: yalnız sen deyince mi, yüksek öncelikte kendiliğinden mi? — *Önerim: yalnız sen deyince.*
35. Takvim bağı: telefonun takvimi mi (yaz + oku), Google Takvim API mi, şimdilik yok mu? — *Önerim: telefonun takvimi.*
36. Sadık köpeğin ısrar düzeyi: hafif / dengeli / sıkı? Sessiz saatler? Akşam özeti saati? — *Önerim: dengeli; 22:00–08:00 sessiz; akşam özeti 19:00.*

## G. Kullanım alışkanlıkları (tasarımı inceltmek için)
37. Geriye dönüp en çok neye bakıyorsun: bir rakama mı, bir karara mı, "ne yapmıştım?" sorusuna mı?
38. Senin için "durum" çoğunlukla rakam mı (stok, fiyat, borç), yoksa karar ve söz mü?
39. Sesli notu en çok nerede kullanıyorsun (araba, atölye, yürürken)?
40. Masaüstünden ne kadar giriş yapıyorsun? (Tarayıcı eklentisi ve kısayolun önceliğini belirler.)

## H. Dağıtım, paketler, bulut (eklendi: 2026-11-08)
41. Başkasına gönderilen sürüm nasıl çalışsın: tek başına (sunucusuz) + gömülü ajan mı, yoksa hazır sunucu kurulumuyla mı? — *Önerim: tek başına çalışsın; sunucu isteğe bağlı olsun (Oracle Always Free için hazır kurulum).*
42. Senin sunucun S24'te mi kalsın, Oracle Always Free'ye mi taşınsın? — *Önerim: S24'te kalsın (veri evde), ajan iş gelince uyansın.*
43. İş raporları ve ortak defter Google Sheets'e de yazılsın mı (Anıl görsün diye)? — *Önerim: evet, yalnız okunur çıktı.*
44. Gömülü Rust ajan çekirdeği (T-16) Hermes'in yerine mi geçsin, yanında mı dursun? — *Önerim: çekirdek NotAI'de; gerçek Hermes isteğe bağlı, MCP ile.*
45. İlk uzmanlık paketleri hangileri olsun (ortak cari, sabun satış/üretim, seans takibi, al-sat)? Hangisi önce? — *Önerim: sabun satış/üretim + ortak cari.*
46. Eşin kendi telefonunda ayrı bir NotAI mı kullanacak (ayrı veri, aynı paketler)? — *Önerim: evet, ayrı kurulum; seans paketi en sıkı gizlilikle.*
47. Paketler herkese açık paylaşılacaksa nerede: GitHub mı, HuggingFace mi? Lisans? — *Önerim: GitHub, MIT; kişisel veriden arındırılmış şablon ve sınav kâğıdıyla.*
48. Ortak cari kuralını doğru anladım mı: her ortağın neti = aldığı − ödediği; toplam net yarı yarıya; fazlası olan farkın yarısını öbürüne verir?

49. iOS ne zaman? Mac erişimi var mı? Çevrende iPhone kullanan hedef kullanıcı var mı? — *Önerim: Android oturduktan sonra (T-17).*
