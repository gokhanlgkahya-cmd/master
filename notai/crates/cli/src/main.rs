//! `notai` ikilisi.

#![forbid(unsafe_code)]

use std::io::Read;
use std::process::ExitCode;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match notai_cli::parse(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let base = std::env::var("NOTAI_URL").unwrap_or_else(|_| "http://127.0.0.1:8787".into());
    let client = match notai_cli::Client::new(base, std::env::var("NOTAI_TOKEN").ok()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let stdin = || {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("stdin okunamadı: {e}"))?;
        Ok(s)
    };
    match notai_cli::run(cmd, &client, stdin).await {
        Ok(out) => {
            #[allow(clippy::print_stdout)]
            {
                println!("{out}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("hata: {e}");
            ExitCode::FAILURE
        }
    }
}
