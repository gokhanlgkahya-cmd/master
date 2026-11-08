# Sesli not: telefon anında yazar, ev sunucusu düzeltir

Karar (Gökhan, 2026-11-08): **ikisi birden.**

1. Mikrofona dokunup konuşursun. Telefonun tanıyıcısı söylediğini **anında** yazıya çevirir
   (konuşurken şeritte görünür). Kayıt hemen telefona yazılır; internet gerekmez.
2. Ses dosyası (Android 13+) kayda eklenir ve senkronla S24'teki ev sunucusuna gider.
3. Ev sunucusu sesi **Whisper** ile yeniden yazıya çevirir; telefonun metninden farklıysa kaydı
   düzeltir. Düzeltme senkronla telefona döner. Henüz karar verilmemiş AI önerileri düzeltilmiş
   metinle yeniden üretilir.
4. Telefon hiç metin çıkaramadıysa kayıt "Sesli not (yazıya çevriliyor…)" olarak görünür; Whisper
   metni gelince yerine geçer. Sunucuya ulaşılamazsa ses bekler: 1, 5, 30 dk, 2 sa, 6 sa sonra yeniden
   denenir; 6 denemede olmazsa "yazıya çevrilemedi" yazar, ses yine dinlenebilir.

Masaüstünde (ve tarayıcıda) telefon tanıyıcısı yoktur: ses kaydedilir, metni Whisper yazar.

## S24'e Whisper kurulumu (Termux, bir kez)

```bash
pkg install git cmake clang ffmpeg
git clone https://github.com/ggml-org/whisper.cpp
cd whisper.cpp
cmake -B build && cmake --build build -j --config Release
# Türkçede iyi ve hızlı: large-v3-turbo (≈550 MB). Yavaş gelirse: small (≈480 MB) ya da base.
sh ./models/download-ggml-model.sh large-v3-turbo-q5_0
```

Çalıştır (NotAI'nin beklediği OpenAI uyumlu adresle):

```bash
./build/bin/whisper-server -m models/ggml-large-v3-turbo-q5_0.bin -l tr -t 6 \
  --host 127.0.0.1 --port 8000 \
  --inference-path /v1/audio/transcriptions --convert
```

- `--convert`: masaüstünden gelen webm/ogg sesi ffmpeg ile çevirir (telefon WAV gönderir).
- `-t 6`: işlemci çekirdeği; telefon ısınırsa 4 yap.

NotAI sunucusunu Whisper'a bağla (diğer ayarların yanına):

```bash
NOTAI_STT_MODEL=large-v3-turbo NOTAI_STT_BASE_URL=http://127.0.0.1:8000 ./notai-server
```

Kontrol: sunucu günlüğünde `ses yazıya çevrildi … outcome=Corrected` satırı görünür.
İkisini Termux açılışında başlatmak için `~/.termux/boot/` altına küçük bir betik koyabilirsin
(Termux:Boot uygulaması gerekir).

## Teknik özet

- Android: `crates/speech` (Tauri eklentisi; Kotlin `SpeechPlugin`): `SpeechRecognizer` + Android 13+'ta
  `AudioRecord` → aynı ses tanıyıcıya (`EXTRA_AUDIO_SOURCE`, bölümlü oturum) ve WAV'a (16 kHz mono,
  en çok 280 sn ≈ 9 MB). Eski Android: yalnız metin (sessizlikte tanıma kendiliğinden yeniden başlar).
- Kayıt: `Store::capture_voice` (metin ya da yer tutucu + ses eki). Ses eki eklenince (yerel ya da
  senkron) `transcripts` kuyruğuna girer (migration 0012, senkronlanmaz).
- Sunucu: işçi önce sesleri (`worker::transcribe_one`), sonra AI işlerini yapar. `POST /api/voice`
  Whisper yoksa / hata verirse de kaydı yazar (`pending: true`), sonra yeniden dener.
