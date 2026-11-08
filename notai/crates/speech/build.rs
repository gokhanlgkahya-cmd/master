//! Eklenti izinlerini üretir ve Android (Kotlin) projesini Tauri'ye tanıtır.

const COMMANDS: &[&str] = &["start", "stop", "cancel", "register_listener"];

fn main() {
    if let Err(e) = tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .try_build()
    {
        eprintln!("notai-speech eklentisi hazırlanamadı: {e}");
        std::process::exit(1);
    }
}
