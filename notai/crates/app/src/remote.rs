//! Ev sunucusuna iletilen işler (AI, ses, otonom mod ayarı). Uygulama yerel öncelikli
//! çalışır; bunlar yalnız sunucu ayarlıysa kullanılabilir.

use std::time::Duration;

use notai_store::Store;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::service::{SYNC_TOKEN, SYNC_URL};

pub const NO_SERVER: &str =
    "Bu özellik ev sunucusu gerektirir: Ayarlar → Ev sunucusu bölümünden adres ve belirteç girin.";

/// Ev sunucusu bağlantısı.
pub struct Remote {
    http: reqwest::Client,
    base: String,
    token: Option<String>,
}

impl Remote {
    /// Sunucu ayarlıysa bağlantı döner.
    pub async fn from_store(store: &Store) -> Result<Option<Self>, String> {
        let url = store.kv_get(SYNC_URL).await.map_err(|e| e.to_string())?;
        let Some(base) = url.filter(|u| !u.trim().is_empty()) else {
            return Ok(None);
        };
        let token = store
            .kv_get(SYNC_TOKEN)
            .await
            .map_err(|e| e.to_string())?
            .filter(|t| !t.is_empty());
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Some(Self {
            http,
            base: base.trim_end_matches('/').to_owned(),
            token,
        }))
    }

    pub async fn require(store: &Store) -> Result<Self, String> {
        Self::from_store(store)
            .await?
            .ok_or_else(|| NO_SERVER.to_owned())
    }

    fn auth(&self, b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => b.bearer_auth(t),
            None => b,
        }
    }

    async fn read<R: DeserializeOwned>(resp: reqwest::Response) -> Result<R, String> {
        let status = resp.status();
        if status.is_success() {
            return resp.json().await.map_err(|e| e.to_string());
        }
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        Err(body["error"]
            .as_str()
            .map_or_else(|| format!("ev sunucusu {status} döndü"), str::to_owned))
    }

    pub async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R, String> {
        let r = self.auth(self.http.get(format!("{}/api{path}", self.base)));
        Self::read(r.send().await.map_err(|e| e.to_string())?).await
    }

    pub async fn post<R: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<R, String> {
        let r = self.auth(
            self.http
                .post(format!("{}/api{path}", self.base))
                .json(body),
        );
        Self::read(r.send().await.map_err(|e| e.to_string())?).await
    }

    pub async fn post_bytes<R: DeserializeOwned>(
        &self,
        path: &str,
        mime: &str,
        data: Vec<u8>,
    ) -> Result<R, String> {
        let r = self.auth(
            self.http
                .post(format!("{}/api{path}", self.base))
                .header(reqwest::header::CONTENT_TYPE, mime)
                .body(data),
        );
        Self::read(r.send().await.map_err(|e| e.to_string())?).await
    }
}
