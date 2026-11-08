//! Ortam değişkenlerinden yapılandırma. API anahtarları yalnız sunucuda durur.

use std::net::SocketAddr;

use chrono::NaiveTime;
use chrono_tz::Tz;

use crate::error::ConfigError;

/// AI sağlayıcı seçimi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiProvider {
    /// AI kapalı: kayıtlar gelen kutusunda kalır, uygulama AI'sız tam çalışır.
    None,
    /// Anthropic Claude API (`ANTHROPIC_API_KEY`).
    Anthropic { model: Option<String> },
    /// OpenAI-uyumlu yerel uç (Ollama + Gemma).
    Local { base_url: String, model: String },
    /// Dış ajan: sunucu model çağırmaz; S24'teki Hermes / Claude Code gibi ajanlar işleri
    /// `notai` CLI ya da `/api/agent/*` ile alıp cevaplar (cevap yine sıkı doğrulanır).
    External,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    /// Bu sunucunun değişiklik günlüğündeki adı.
    pub origin: String,
    /// `/api` için `Authorization: Bearer` belirteci. Boşsa kimlik doğrulama kapalı
    /// (yalnız Tailscale/ev ağı içinde önerilir).
    pub api_token: Option<String>,
    pub tz: Tz,
    pub user_name: String,
    pub morning_at: NaiveTime,
    pub ai: AiProvider,
    /// Sabah planı bildirimi için ntfy konusu URL'si (ör. `http://s24:8080/notai`).
    pub ntfy_url: Option<String>,
    /// Ses → yazı servisi (Whisper uyumlu): (adres, model).
    pub stt: Option<(String, String)>,
    /// Anlamsal arama vektör servisi (OpenAI uyumlu embeddings): (adres, model).
    pub embed: Option<(String, String)>,
    /// Tarayıcıdan doğrudan bağlanmaya izin verilen kökenler (CORS). Boşsa CORS kapalı
    /// (yalnız aynı köken, uygulama kabuğu ve komut satırı). `*` her kökene izin verir.
    pub cors_origins: Vec<String>,
    /// Obsidian kasası klasörü (`NOTAI_VAULT_DIR`). Ayarlıysa sunucu `NotAI/` alt klasörünü
    /// her değişiklikte günceller; boşsa kapalı.
    pub vault_dir: Option<std::path::PathBuf>,
}

impl Config {
    /// Varsayılan değerlerle (testler ve ilk kurulum için).
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], 8787)),
            database_url: "sqlite://notai.db".into(),
            origin: "sunucu".into(),
            api_token: None,
            tz: notai_core::DEFAULT_TZ,
            user_name: "Gökhan".into(),
            morning_at: NaiveTime::from_hms_opt(9, 0, 0).unwrap_or(NaiveTime::MIN),
            ai: AiProvider::None,
            ntfy_url: None,
            stt: None,
            embed: None,
            cors_origins: Vec::new(),
            vault_dir: None,
        }
    }

    /// `NOTAI_*` ortam değişkenlerinden okur.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Sınanabilir okuma: değişken adı → değer.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let mut cfg = Self::defaults();
        let non_empty = |k: &str| get(k).filter(|v| !v.trim().is_empty());

        if let Some(v) = non_empty("NOTAI_BIND") {
            cfg.bind = v
                .parse()
                .map_err(|_| ConfigError::Invalid("NOTAI_BIND", v))?;
        }
        if let Some(v) = non_empty("NOTAI_DATABASE_URL") {
            cfg.database_url = v;
        }
        if let Some(v) = non_empty("NOTAI_ORIGIN") {
            cfg.origin = v;
        }
        cfg.api_token = non_empty("NOTAI_API_TOKEN");
        if let Some(v) = non_empty("NOTAI_TZ") {
            cfg.tz = v.parse().map_err(|_| ConfigError::Invalid("NOTAI_TZ", v))?;
        }
        if let Some(v) = non_empty("NOTAI_USER_NAME") {
            cfg.user_name = v;
        }
        if let Some(v) = non_empty("NOTAI_MORNING_AT") {
            cfg.morning_at = NaiveTime::parse_from_str(&v, "%H:%M")
                .map_err(|_| ConfigError::Invalid("NOTAI_MORNING_AT", v))?;
        }
        cfg.ntfy_url = non_empty("NOTAI_NTFY_URL");
        cfg.vault_dir = non_empty("NOTAI_VAULT_DIR").map(std::path::PathBuf::from);
        if let Some(v) = non_empty("NOTAI_CORS_ORIGINS") {
            cfg.cors_origins = v
                .split(',')
                .map(|o| o.trim().trim_end_matches('/').to_owned())
                .filter(|o| !o.is_empty())
                .collect();
            let valid = |o: &str| {
                o == "*"
                    || ((o.starts_with("http://") || o.starts_with("https://"))
                        && o.is_ascii()
                        && !o.contains([' ', '"']))
            };
            if !cfg.cors_origins.iter().all(|o| valid(o)) {
                return Err(ConfigError::Invalid("NOTAI_CORS_ORIGINS", v));
            }
        }
        cfg.stt = non_empty("NOTAI_STT_MODEL").map(|m| {
            (
                non_empty("NOTAI_STT_BASE_URL").unwrap_or_else(|| "http://127.0.0.1:8000".into()),
                m,
            )
        });
        cfg.embed = non_empty("NOTAI_EMBED_MODEL").map(|m| {
            (
                non_empty("NOTAI_EMBED_BASE_URL")
                    .unwrap_or_else(|| "http://127.0.0.1:11434".into()),
                m,
            )
        });
        cfg.ai = match non_empty("NOTAI_AI").as_deref() {
            None | Some("none") => AiProvider::None,
            Some("external") => AiProvider::External,
            Some("anthropic") => AiProvider::Anthropic {
                model: non_empty("NOTAI_AI_MODEL"),
            },
            Some("local") => AiProvider::Local {
                base_url: non_empty("NOTAI_AI_BASE_URL")
                    .unwrap_or_else(|| "http://127.0.0.1:11434".into()),
                model: non_empty("NOTAI_AI_MODEL").ok_or(ConfigError::Missing("NOTAI_AI_MODEL"))?,
            },
            Some(other) => return Err(ConfigError::Invalid("NOTAI_AI", other.to_owned())),
        };
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn defaults_are_safe() {
        let c = cfg(&[]).unwrap();
        assert_eq!(c.bind.to_string(), "127.0.0.1:8787");
        assert_eq!(c.ai, AiProvider::None);
        assert_eq!(c.tz, chrono_tz::Europe::Istanbul);
        assert_eq!(c.morning_at.format("%H:%M").to_string(), "09:00");
    }

    #[test]
    fn parses_values_and_rejects_bad_ones() {
        let c = cfg(&[
            ("NOTAI_AI", "local"),
            ("NOTAI_AI_MODEL", "gemma3:12b"),
            ("NOTAI_MORNING_AT", "07:30"),
            ("NOTAI_API_TOKEN", "gizli"),
        ])
        .unwrap();
        assert_eq!(
            c.ai,
            AiProvider::Local {
                base_url: "http://127.0.0.1:11434".into(),
                model: "gemma3:12b".into()
            }
        );
        assert_eq!(c.api_token.as_deref(), Some("gizli"));
        let c = cfg(&[
            ("NOTAI_STT_MODEL", "whisper-large-v3"),
            ("NOTAI_EMBED_MODEL", "bge-m3"),
        ])
        .unwrap();
        assert_eq!(
            c.stt,
            Some(("http://127.0.0.1:8000".into(), "whisper-large-v3".into()))
        );
        assert_eq!(
            c.embed,
            Some(("http://127.0.0.1:11434".into(), "bge-m3".into()))
        );
        let c = cfg(&[(
            "NOTAI_CORS_ORIGINS",
            "http://localhost:8080/, http://100.64.0.2:8080",
        )])
        .unwrap();
        assert_eq!(
            c.cors_origins,
            ["http://localhost:8080", "http://100.64.0.2:8080"]
        );
        assert!(
            cfg(&[]).unwrap().cors_origins.is_empty(),
            "varsayılan: CORS kapalı"
        );
        assert!(cfg(&[("NOTAI_CORS_ORIGINS", "localhost:8080")]).is_err());
        assert!(cfg(&[("NOTAI_AI", "local")]).is_err());
        assert!(cfg(&[("NOTAI_TZ", "Mars/Olympus")]).is_err());
        assert!(cfg(&[("NOTAI_MORNING_AT", "sabah")]).is_err());
        assert!(cfg(&[("NOTAI_AI", "gpt")]).is_err());
    }
}
