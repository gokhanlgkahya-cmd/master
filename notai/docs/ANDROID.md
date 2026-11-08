# Android APK'yı kendi bilgisayarında derleme

NotAI'nin Android uygulaması aynı Rust kodundan (Tauri 2) derlenir. Kod hazır; aşağıdaki adımlar
bir kez kurulum, sonra tek komutla derleme içindir. Windows'a göre yazıldı; Linux/macOS'ta
aynı adımlar geçerli (yalnız ortam değişkeni yazımı farklı).

## 1. Bir kez: araçları kur
1. **Android Studio** — https://developer.android.com/studio (kurulumda "Android SDK" seçili kalsın).
2. Android Studio → *Settings → Languages & Frameworks → Android SDK*:
   - *SDK Platforms*: **Android 14 (API 34)**
   - *SDK Tools*: **Android SDK Build-Tools 34**, **NDK (Side by side)**, **Android SDK Command-line Tools**, **Android SDK Platform-Tools**
3. **Ortam değişkenleri** (Windows: *Sistem → Gelişmiş sistem ayarları → Ortam Değişkenleri*):
   - `ANDROID_HOME` = `C:\Users\<kullanıcı>\AppData\Local\Android\Sdk`
   - `NDK_HOME` = `%ANDROID_HOME%\ndk\<sürüm>` (klasördeki sürüm numarası)
   - `JAVA_HOME` = `C:\Program Files\Android\Android Studio\jbr`
4. **Rust** — https://rustup.rs, ardından bir terminalde:
   ```powershell
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android wasm32-unknown-unknown
   cargo install trunk --locked
   cargo install tauri-cli --version "^2" --locked
   ```

## 2. Derle
```powershell
git clone https://github.com/gokhanlgkahya-cmd/master.git
cd master\notai\crates\app
cargo tauri android init          # yalnız ilk sefer: crates/app/gen/android projesini üretir
cargo tauri android build --apk   # arayüzü (trunk) ve Rust kodunu derler, APK'yı imzasız üretir
```
APK: `crates\app\gen\android\app\build\outputs\apk\universal\release\app-universal-release-unsigned.apk`

### Bir kez: izinler (konum + mikrofon)
`cargo tauri android init` sonrası `crates\app\gen\android\app\src\main\AndroidManifest.xml`
dosyasını aç; `<uses-permission android:name="android.permission.INTERNET" />` satırının altına ekle:
```xml
<uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />
<uses-permission android:name="android.permission.ACCESS_COARSE_LOCATION" />
<uses-permission android:name="android.permission.RECORD_AUDIO" />
```
- **Konum:** "işe geldim", "evden çıkıyorum" gibi hareket kayıtlarını teyit etmek ve yerlerin
  konumunu öğrenmek için (yalnız hareket kaydı yazdığında, tek seferlik; arka planda izlenmez).
- **Mikrofon:** sesli kayıt için.
İlk kullanımda Android izin sorar; "Uygulamayı kullanırken" yeterli.

Telefona yüklemek için en kolay yol geliştirme modu:
```powershell
cargo tauri android dev           # USB ile bağlı telefonda (USB hata ayıklama açık) çalıştırır
```

### İmzalı APK (telefona kalıcı kurulum)
```powershell
keytool -genkey -v -keystore notai.jks -keyalg RSA -keysize 2048 -validity 10000 -alias notai
```
Ardından Tauri belgelerindeki "Android code signing" adımıyla `gen/android/keystore.properties`
dosyasını oluştur ve `cargo tauri android build --apk` komutunu tekrar çalıştır.
(Anahtar dosyasını ve parolasını depoya koyma.)

## 3. İlk açılış
1. Uygulama internetsiz çalışır (veri telefondaki SQLite'ta).
2. Ayarlar → **Ev sunucusu**: S24'teki `notai-server` adresi (ör. Tailscale IP'si `http://100.x.y.z:8787`)
   ve `NOTAI_API_TOKEN` değeri → *Kaydet* → *Şimdi senkronla*.
3. Android bildirim izni istenince **İzin ver** (hatırlatmalar ve sabah planı için).

## Sorun çıkarsa
- `NDK not found` → `NDK_HOME` yolunu kontrol et.
- `failed to run trunk` → `cargo install trunk --locked`, ardından tekrar dene.
- Derleme ilk seferde 10–20 dakika sürebilir (tüm bağımlılıklar dört Android mimarisi için derlenir);
  yalnız telefonun mimarisi için daha hızlı: `cargo tauri android build --apk --target aarch64`.
