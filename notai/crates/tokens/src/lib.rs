//! `design/tokens.json` → CSS değişkenleri.
//!
//! Kurallar `design/guidelines.md` §2.2'deki sözleşmedir (Faz 1'deki Python üreticisiyle birebir
//! aynı çıktıyı verir; test bunu depodaki `design/tokens.css` ile karşılaştırarak doğrular).

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Number, Value};
use thiserror::Error;

const NS: &str = "app.notai";
const MODES: [&str; 2] = ["light", "dark"];
const GENERIC_FAMILIES: [&str; 14] = [
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
    "ui-rounded",
    "-apple-system",
    "emoji",
    "math",
    "fangsong",
];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenError {
    #[error("JSON okunamadı: {0}")]
    Json(String),
    #[error("{0}: nesne bekleniyordu")]
    NotObject(String),
    #[error("{0}: $type yok")]
    NoType(String),
    #[error("döngüsel takma ad: {0}")]
    Cycle(String),
    #[error("çözülemeyen takma ad: {0}")]
    Unresolved(String),
    #[error("geçersiz değer ({kind}): {value}")]
    BadValue { kind: String, value: String },
    #[error("aynı CSS adı iki kez: {0}")]
    Duplicate(String),
    #[error("açık/koyu rol kümeleri farklı: {0}")]
    ModeMismatch(String),
}

#[derive(Debug, Clone)]
struct Token {
    path: Vec<String>,
    kind: String,
    value: Value,
    ext: Map<String, Value>,
    inherited_ext: Map<String, Value>,
    emit: bool,
}

fn kebab(seg: &str) -> String {
    // Python: re.sub(r"(?<=[a-z0-9])([A-Z])", r"-\1", seg).lower()
    let mut out = String::with_capacity(seg.len() + 4);
    let mut prev: Option<char> = None;
    for c in seg.chars() {
        if c.is_ascii_uppercase()
            && prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('-');
        }
        out.extend(c.to_lowercase());
        prev = Some(c);
    }
    out
}

fn alias_target(v: &Value) -> Option<&str> {
    let s = v.as_str()?;
    let inner = s.strip_prefix('{')?.strip_suffix('}')?;
    if inner.is_empty() || inner.contains(['{', '}']) {
        None
    } else {
        Some(inner)
    }
}

fn walk(
    node: &Value,
    path: &[String],
    inherited_type: Option<&str>,
    emit: bool,
    inherited_ext: &Map<String, Value>,
    out: &mut Vec<Token>,
) -> Result<(), TokenError> {
    let obj = node
        .as_object()
        .ok_or_else(|| TokenError::NotObject(path.join(".")))?;
    let gtype = obj.get("$type").and_then(Value::as_str).or(inherited_type);
    let ext = obj
        .get("$extensions")
        .and_then(|e| e.get(NS))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let emit = emit && ext.get("emit") != Some(&Value::Bool(false));
    if let Some(value) = obj.get("$value") {
        let kind = gtype.ok_or_else(|| TokenError::NoType(path.join(".")))?;
        out.push(Token {
            path: path.to_vec(),
            kind: kind.to_owned(),
            value: value.clone(),
            ext,
            inherited_ext: inherited_ext.clone(),
            emit,
        });
        return Ok(());
    }
    let mut child_inh = inherited_ext.clone();
    for (k, v) in &ext {
        if k != "emit" {
            child_inh.insert(k.clone(), v.clone());
        }
    }
    for (key, child) in obj {
        if key.starts_with('$') {
            continue;
        }
        let mut p = path.to_vec();
        p.push(key.clone());
        walk(child, &p, gtype, emit, &child_inh, out)?;
    }
    Ok(())
}

fn resolve(
    value: &Value,
    index: &HashMap<String, &Token>,
    seen: &mut Vec<String>,
) -> Result<Value, TokenError> {
    if let Some(target) = alias_target(value) {
        if seen.iter().any(|s| s == target) {
            let mut chain = seen.clone();
            chain.push(target.to_owned());
            return Err(TokenError::Cycle(chain.join(" → ")));
        }
        let tok = index
            .get(target)
            .ok_or_else(|| TokenError::Unresolved(value.as_str().unwrap_or_default().to_owned()))?;
        seen.push(target.to_owned());
        let r = resolve(&tok.value, index, seen);
        seen.pop();
        return r;
    }
    match value {
        Value::Array(a) => a
            .iter()
            .map(|v| resolve(v, index, seen))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| Ok((k.clone(), resolve(v, index, seen)?)))
            .collect::<Result<Map<_, _>, _>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

/// Python'un `str(sayı)` biçimi: tamsayı → "12", ondalık → en kısa gösterim, tam ondalık → "1.0".
fn py_num(n: &Number) -> String {
    if let Some(i) = n.as_i64() {
        i.to_string()
    } else if let Some(u) = n.as_u64() {
        u.to_string()
    } else {
        format!("{:?}", n.as_f64().unwrap_or_default())
    }
}

fn bad(kind: &str, value: &Value) -> TokenError {
    TokenError::BadValue {
        kind: kind.to_owned(),
        value: value.to_string(),
    }
}

fn dim(v: &Value) -> Result<String, TokenError> {
    let obj = v
        .as_object()
        .filter(|o| o.len() == 2)
        .ok_or_else(|| bad("dimension", v))?;
    let n = obj
        .get("value")
        .and_then(Value::as_number)
        .ok_or_else(|| bad("dimension", v))?;
    let unit = obj
        .get("unit")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("dimension", v))?;
    let num = match n.as_f64() {
        Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 1e15 => format!("{f:.0}"),
        _ => py_num(n),
    };
    Ok(format!("{num}{unit}"))
}

fn color(v: &Value) -> Result<String, TokenError> {
    let s = v.as_str().ok_or_else(|| bad("color", v))?;
    let hex = s.strip_prefix('#').ok_or_else(|| bad("color", v))?;
    if (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(s.to_uppercase())
    } else {
        Err(bad("color", v))
    }
}

fn fmt(tok: &Token, value: &Value) -> Result<String, TokenError> {
    match tok.kind.as_str() {
        "color" => color(value),
        "dimension" | "duration" => dim(value),
        "number" => {
            let n = value.as_number().ok_or_else(|| bad("number", value))?;
            let unit = tok
                .ext
                .get("cssUnit")
                .or_else(|| tok.inherited_ext.get("cssUnit"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if n.as_f64() == Some(0.0) {
                Ok("0".to_owned())
            } else {
                Ok(format!("{}{unit}", py_num(n)))
            }
        }
        "fontWeight" => match value {
            Value::Number(n) => Ok(py_num(n)),
            Value::String(s) => Ok(s.clone()),
            other => Err(bad("fontWeight", other)),
        },
        "fontFamily" => {
            let fams: Vec<&str> = match value {
                Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
                Value::String(s) => vec![s.as_str()],
                other => return Err(bad("fontFamily", other)),
            };
            Ok(fams
                .iter()
                .map(|f| {
                    if GENERIC_FAMILIES.contains(f) {
                        (*f).to_owned()
                    } else {
                        format!("\"{f}\"")
                    }
                })
                .collect::<Vec<_>>()
                .join(", "))
        }
        "cubicBezier" => {
            let a = value
                .as_array()
                .filter(|a| a.len() == 4)
                .ok_or_else(|| bad("cubicBezier", value))?;
            let parts = a
                .iter()
                .map(|x| {
                    x.as_number()
                        .map(py_num)
                        .ok_or_else(|| bad("cubicBezier", value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("cubic-bezier({})", parts.join(", ")))
        }
        "shadow" => {
            let layers: Vec<&Value> = match value {
                Value::Array(a) => a.iter().collect(),
                other => vec![other],
            };
            let mut parts = Vec::new();
            for s in layers {
                let inset = s.get("inset").and_then(Value::as_bool).unwrap_or(false);
                let dims = ["offsetX", "offsetY", "blur", "spread"]
                    .iter()
                    .map(|k| s.get(*k).ok_or_else(|| bad("shadow", s)).and_then(dim))
                    .collect::<Result<Vec<_>, _>>()?;
                let c = color(s.get("color").ok_or_else(|| bad("shadow", s))?)?;
                parts.push(format!(
                    "{}{} {c}",
                    if inset { "inset " } else { "" },
                    dims.join(" ")
                ));
            }
            Ok(parts.join(", "))
        }
        "string" | "strokeStyle" => value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| bad(&tok.kind, value)),
        other => Err(bad("$type", &Value::String(other.to_owned()))),
    }
}

fn css_name(path: &[String]) -> (Option<&'static str>, String) {
    let join = |segs: &[String]| segs.iter().map(|s| kebab(s)).collect::<Vec<_>>().join("-");
    if path.len() > 3
        && path[0] == "color"
        && path[1] == "theme"
        && let Some(mode) = MODES.iter().find(|m| **m == path[2])
    {
        return (Some(mode), format!("--color-{}", join(&path[3..])));
    }
    if path.len() > 2
        && path[0] == "shadow"
        && let Some(mode) = MODES.iter().find(|m| **m == path[1])
    {
        return (Some(mode), format!("--shadow-{}", join(&path[2..])));
    }
    (None, format!("--{}", join(path)))
}

/// tokens.json içeriğinden CSS üretir.
pub fn generate(src: &str) -> Result<String, TokenError> {
    let raw: Value = serde_json::from_str(src).map_err(|e| TokenError::Json(e.to_string()))?;
    let mut tokens = Vec::new();
    walk(&raw, &[], None, true, &Map::new(), &mut tokens)?;
    let index: HashMap<String, &Token> = tokens.iter().map(|t| (t.path.join("."), t)).collect();

    let mut common = Vec::new();
    let mut light = Vec::new();
    let mut dark = Vec::new();
    let mut reduced = Vec::new();
    let mut names: HashMap<(Option<&str>, String), String> = HashMap::new();

    for tok in &tokens {
        let resolved = resolve(&tok.value, &index, &mut Vec::new())?;
        if !tok.emit {
            fmt(tok, &resolved)?;
            continue;
        }
        let (mode, name) = css_name(&tok.path);
        let key = (mode, name.clone());
        if let Some(prev) = names.get(&key) {
            return Err(TokenError::Duplicate(format!(
                "{name} ({prev} ve {})",
                tok.path.join(".")
            )));
        }
        names.insert(key, tok.path.join("."));
        let value = fmt(tok, &resolved)?;
        let comment = match tok.value.as_str() {
            Some(s) if alias_target(&tok.value).is_some() => format!(" /* {s} */"),
            _ => String::new(),
        };
        let line = format!("  {name}: {value};{comment}");
        match mode {
            None => common.push(line),
            Some("light") => light.push(line),
            Some(_) => dark.push(line),
        }
        if let Some(rm) = tok.ext.get("reducedMotion") {
            let rtok = match alias_target(rm) {
                Some(t) => *index
                    .get(t)
                    .ok_or_else(|| TokenError::Unresolved(t.to_owned()))?,
                None => tok,
            };
            let v = fmt(rtok, &resolve(rm, &index, &mut Vec::new())?)?;
            reduced.push(format!("    {name}: {v};"));
        }
    }

    let set = |m: &str| -> BTreeSet<&String> {
        names
            .keys()
            .filter(|(mm, _)| *mm == Some(m))
            .map(|(_, n)| n)
            .collect()
    };
    let (l, d) = (set("light"), set("dark"));
    if l != d {
        let diff: Vec<_> = l.symmetric_difference(&d).map(|s| s.as_str()).collect();
        return Err(TokenError::ModeMismatch(diff.join(", ")));
    }

    let dark_body: Vec<String> = dark.iter().map(|l| format!("  {l}")).collect();
    let mut out: Vec<String> = vec![
        "/* ÜRETİLMİŞ DOSYA — ELLE DÜZENLEMEYİN.".into(),
        " * Kaynak: design/tokens.json · Üretici: tokens.json → CSS (kurallar: design/guidelines.md §2).".into(),
        " * Değişiklik tokens.json'da yapılır, sonra bu dosya yeniden üretilir. */".into(),
        String::new(),
        ":root {".into(),
        "  color-scheme: light;".into(),
    ];
    out.extend(common);
    out.push(String::new());
    out.push("  /* Açık tema (varsayılan) */".into());
    out.extend(light);
    out.extend([
        "}".into(),
        String::new(),
        "/* Koyu tema: sistem ayarı (kullanıcı \"Açık\" seçmediyse) */".into(),
        "@media (prefers-color-scheme: dark) {".into(),
        "  :root:not([data-theme=\"light\"]) {".into(),
        "    color-scheme: dark;".into(),
        dark_body.join("\n"),
        "  }".into(),
        "}".into(),
        String::new(),
        "/* Koyu tema: kullanıcı Ayarlar'da \"Koyu\" seçti */".into(),
        ":root[data-theme=\"dark\"] {".into(),
        "  color-scheme: dark;".into(),
    ]);
    out.extend(dark);
    out.extend([
        "}".into(),
        String::new(),
        "/* Azaltılmış hareket: yaylar → kısa opacity geçişi (transform'ları bileşen CSS'i kapatır) */".into(),
        "@media (prefers-reduced-motion: reduce) {".into(),
        "  :root {".into(),
    ]);
    out.extend(reduced);
    out.extend(["  }".into(), "}".into(), String::new()]);
    Ok(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_case() {
        assert_eq!(kebab("lineHeight"), "line-height");
        assert_eq!(kebab("touchMin"), "touch-min");
        assert_eq!(kebab("2xl"), "2xl");
        assert_eq!(kebab("surface-raised"), "surface-raised");
    }

    #[test]
    fn aliases_modes_and_comments() {
        let css = generate(
            r##"{
              "color": {
                "$type": "color",
                "neutral": { "900": { "$value": "#26231f" }, "0": { "$value": "#ffffff" } },
                "theme": {
                  "light": { "text": { "$value": "{color.neutral.900}" } },
                  "dark": { "text": { "$value": "{color.neutral.0}" } }
                }
              },
              "space": { "$type": "dimension", "4": { "$value": { "value": 16, "unit": "px" } } },
              "font": { "letterSpacing": { "$type": "number",
                         "$extensions": { "app.notai": { "cssUnit": "em" } },
                         "tight": { "$value": -0.01 }, "none": { "$value": 0 } } }
            }"##,
        )
        .unwrap();
        assert!(css.contains("  --color-neutral-900: #26231F;"));
        assert!(css.contains("  --color-text: #26231F; /* {color.neutral.900} */"));
        assert!(css.contains("    --color-text: #FFFFFF; /* {color.neutral.0} */"));
        assert!(css.contains("  --space-4: 16px;"));
        assert!(css.contains("  --font-letter-spacing-tight: -0.01em;"));
        assert!(css.contains("  --font-letter-spacing-none: 0;"));
    }

    #[test]
    fn errors_are_reported() {
        let cycle = r#"{"a":{"$type":"color","x":{"$value":"{a.y}"},"y":{"$value":"{a.x}"}}}"#;
        assert!(matches!(generate(cycle), Err(TokenError::Cycle(_))));
        let missing = r#"{"a":{"$type":"color","x":{"$value":"{a.z}"}}}"#;
        assert!(matches!(generate(missing), Err(TokenError::Unresolved(_))));
        let mismatch = r##"{"color":{"$type":"color","theme":{"light":{"a":{"$value":"#000000"}},"dark":{}}}}"##;
        assert!(matches!(
            generate(mismatch),
            Err(TokenError::ModeMismatch(_))
        ));
        let bad_hex = r#"{"c":{"$type":"color","x":{"$value":"kırmızı"}}}"#;
        assert!(matches!(
            generate(bad_hex),
            Err(TokenError::BadValue { .. })
        ));
    }
}
