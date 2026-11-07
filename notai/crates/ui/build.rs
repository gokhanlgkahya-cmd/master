//! Derleme sırasında tasarım dosyalarından arayüz varlıklarını üretir:
//! - `tokens.css`: design/tokens.json → CSS (notai-tokens; guidelines §2.2)
//! - `sprite.svg`: design/screens/*.html içindeki tüm `<symbol id="i-…">` ikonlarının birleşimi
//!
//! Böylece tasarımda yapılan değişiklik koda kendiliğinden yansır; ikinci kopya yoktur.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let design = manifest.join("../../design");
    let out = PathBuf::from(std::env::var("OUT_DIR")?);

    let tokens_json = design.join("tokens.json");
    println!("cargo:rerun-if-changed={}", tokens_json.display());
    let css = notai_tokens::generate(&std::fs::read_to_string(&tokens_json)?)?;
    std::fs::write(out.join("tokens.css"), css)?;

    let screens = design.join("screens");
    println!("cargo:rerun-if-changed={}", screens.display());
    std::fs::write(out.join("sprite.svg"), sprite(&screens)?)?;
    Ok(())
}

fn sprite(dir: &Path) -> Result<String, Box<dyn Error>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "html"))
        .collect();
    files.sort();
    let mut symbols: BTreeMap<String, String> = BTreeMap::new();
    for f in files {
        println!("cargo:rerun-if-changed={}", f.display());
        let html = std::fs::read_to_string(&f)?;
        let mut rest = html.as_str();
        while let Some(start) = rest.find("<symbol") {
            let tail = &rest[start..];
            let Some(end) = tail.find("</symbol>") else {
                break;
            };
            let sym = &tail[..end + "</symbol>".len()];
            if let Some(id) = sym.split("id=\"").nth(1).and_then(|s| s.split('"').next()) {
                symbols
                    .entry(id.to_owned())
                    .or_insert_with(|| sym.to_owned());
            }
            rest = &tail[end..];
        }
    }
    if !symbols.contains_key("i-notebook") || !symbols.contains_key("i-spark") {
        return Err("tasarım ekranlarında temel ikonlar (i-notebook, i-spark) bulunamadı".into());
    }
    let mut s = String::from(
        "<svg class=\"sprite\" xmlns=\"http://www.w3.org/2000/svg\" aria-hidden=\"true\">",
    );
    for sym in symbols.values() {
        s.push_str(sym);
    }
    s.push_str("</svg>");
    Ok(s)
}
