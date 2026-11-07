//! Masaüstü giriş noktası.

#![forbid(unsafe_code)]
// Windows sürümünde ek konsol penceresi açılmasın.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    notai_app::run();
}
