//! `gen-tokens [tokens.json] [tokens.css]`: design/tokens.css'i yeniden üretir.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let src = args.next().unwrap_or_else(|| "design/tokens.json".into());
    let dst = args.next().unwrap_or_else(|| "design/tokens.css".into());
    let json = match std::fs::read_to_string(&src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{src} okunamadı: {e}");
            return ExitCode::FAILURE;
        }
    };
    match notai_tokens::generate(&json) {
        Ok(css) => match std::fs::write(&dst, &css) {
            Ok(()) => {
                eprintln!("yazıldı: {dst} ({} satır)", css.lines().count());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{dst} yazılamadı: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("tokens.json hatası: {e}");
            ExitCode::FAILURE
        }
    }
}
