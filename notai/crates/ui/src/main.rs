//! WASM giriş noktası: uygulamayı sayfaya bağlar.

#![forbid(unsafe_code)]

fn main() {
    leptos::mount::mount_to_body(notai_ui::App);
}
