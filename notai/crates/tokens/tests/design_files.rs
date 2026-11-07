//! Depodaki design/tokens.css, design/tokens.json'dan üretilenle birebir aynı olmalı.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

fn design(file: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../design")
        .join(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn committed_css_matches_generator() {
    let generated = notai_tokens::generate(&design("tokens.json")).unwrap();
    let committed = design("tokens.css");
    if generated != committed {
        let first_diff = generated
            .lines()
            .zip(committed.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b);
        panic!(
            "design/tokens.css güncel değil; `cargo run -p notai-tokens --bin gen-tokens` çalıştırın. İlk fark: {first_diff:?}"
        );
    }
}
