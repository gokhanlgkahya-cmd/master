//! NotAI senkron protokolü.
//!
//! Model: yerel öncelikli (local-first). Her cihaz kendi SQLite'ına yazar; her yazma
//! değişiklik günlüğüne düşer. Senkron iki adımdır:
//!
//! 1. **Push:** cihaz, kendi ürettiği ve henüz göndermediği değişiklikleri sunucuya yollar.
//!    Sunucu birleştirme kurallarıyla uygular (yeni kayıtlar sunucuda AI kuyruğuna girer).
//! 2. **Pull:** cihaz, imlecinden sonraki ve *kendisinden gelmeyen* değişiklikleri çeker ve uygular.
//!
//! Birleştirme kuralları `notai_core::sync` içinde (son yazan kazanır + alan istisnaları).

#![forbid(unsafe_code)]

use std::future::Future;
use std::pin::Pin;

use notai_store::changes::ChangeRecord;
use notai_store::{Applied, Store, StoreError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Tek seferde gönderilen/çekilen en fazla değişiklik.
pub const BATCH: u32 = 200;

const PUSHED_KEY: &str = "sync_pushed_seq";
const PULLED_KEY: &str = "sync_pulled_cursor";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushRequest {
    pub device: String,
    pub changes: Vec<ChangeRecord>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PushResponse {
    pub applied: u32,
    pub skipped: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullResponse {
    pub changes: Vec<ChangeRecord>,
    /// Bir sonraki çekmede kullanılacak imleç.
    pub cursor: i64,
    /// Daha fazla değişiklik var mı?
    pub more: bool,
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("ağ hatası: {0}")]
    Http(#[from] reqwest::Error),
    #[error("sunucu {status} döndü: {body}")]
    Status { status: u16, body: String },
    #[error("reddedildi: {0}")]
    Rejected(String),
}

// ---------- Sunucu tarafı ----------

/// Sunucu: bir cihazın gönderdiği değişiklikleri uygular. Cihaz yalnız kendi adına
/// değişiklik gönderebilir (başka cihaz adına yazma reddedilir).
pub async fn handle_push(store: &Store, req: &PushRequest) -> Result<PushResponse, SyncError> {
    if req.device.trim().is_empty() || req.device == store.origin() {
        return Err(SyncError::Rejected("geçersiz cihaz adı".into()));
    }
    if let Some(bad) = req.changes.iter().find(|c| c.origin != req.device) {
        return Err(SyncError::Rejected(format!(
            "cihaz {} başka bir kaynak ({}) adına değişiklik gönderemez",
            req.device, bad.origin
        )));
    }
    let mut out = PushResponse::default();
    for c in &req.changes {
        match store.apply_remote(c, true).await? {
            Applied::Applied => out.applied += 1,
            Applied::Skipped => out.skipped += 1,
        }
    }
    Ok(out)
}

/// Sunucu: cihazın çekeceği değişiklikler (cihazın kendi gönderdikleri hariç).
pub async fn handle_pull(
    store: &Store,
    device: &str,
    after: i64,
    limit: u32,
) -> Result<PullResponse, SyncError> {
    let limit = limit.clamp(1, 1000);
    let changes = store.changes_for_peer(after, device, limit).await?;
    let cursor = changes.last().map_or(after, |c| c.seq);
    let more = changes.len() >= limit as usize;
    Ok(PullResponse {
        changes,
        cursor,
        more,
    })
}

// ---------- İstemci tarafı ----------

pub type SyncFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, SyncError>> + Send + 'a>>;

/// Sunucuya ulaşma yolu (HTTP ya da testlerde süreç içi).
pub trait Transport: Send + Sync {
    fn push<'a>(&'a self, req: &'a PushRequest) -> SyncFuture<'a, PushResponse>;
    fn pull<'a>(&'a self, device: &'a str, after: i64, limit: u32) -> SyncFuture<'a, PullResponse>;
}

/// Süreç içi taşıyıcı: sunucu deposuna doğrudan bağlanır (testler ve tek makine kurulumu).
#[derive(Debug, Clone)]
pub struct InProcess {
    pub server: Store,
}

impl Transport for InProcess {
    fn push<'a>(&'a self, req: &'a PushRequest) -> SyncFuture<'a, PushResponse> {
        Box::pin(handle_push(&self.server, req))
    }

    fn pull<'a>(&'a self, device: &'a str, after: i64, limit: u32) -> SyncFuture<'a, PullResponse> {
        Box::pin(handle_pull(&self.server, device, after, limit))
    }
}

/// HTTP taşıyıcı: `notai-server`'ın `/api/sync/*` uçları.
#[derive(Debug, Clone)]
pub struct Http {
    client: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl Http {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Result<Self, SyncError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token,
        })
    }

    fn auth(&self, b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => b.bearer_auth(t),
            None => b,
        }
    }

    async fn check(resp: reqwest::Response) -> Result<reqwest::Response, SyncError> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(SyncError::Status {
                status: status.as_u16(),
                body: body.chars().take(300).collect(),
            })
        }
    }
}

impl Transport for Http {
    fn push<'a>(&'a self, req: &'a PushRequest) -> SyncFuture<'a, PushResponse> {
        Box::pin(async move {
            let url = format!("{}/api/sync/push", self.base_url);
            let resp = self.auth(self.client.post(url).json(req)).send().await?;
            Ok(Self::check(resp).await?.json().await?)
        })
    }

    fn pull<'a>(&'a self, device: &'a str, after: i64, limit: u32) -> SyncFuture<'a, PullResponse> {
        Box::pin(async move {
            let url = format!("{}/api/sync/pull", self.base_url);
            let after_s = after.to_string();
            let limit_s = limit.to_string();
            let resp = self
                .auth(self.client.get(url).query(&[
                    ("device", device),
                    ("after", after_s.as_str()),
                    ("limit", limit_s.as_str()),
                ]))
                .send()
                .await?;
            Ok(Self::check(resp).await?.json().await?)
        })
    }
}

/// Bir senkron turunun özeti.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SyncReport {
    pub pushed: u32,
    pub pulled_applied: u32,
    pub pulled_skipped: u32,
}

async fn kv_i64(store: &Store, key: &str) -> Result<i64, SyncError> {
    Ok(store
        .kv_get(key)
        .await?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0))
}

/// İstemci: önce gönder, sonra çek. İmleçler yerel `kv` tablosunda tutulur; yarıda
/// kesilirse kaldığı yerden devam eder (uygulama idempotenttir).
pub async fn sync_once(store: &Store, transport: &dyn Transport) -> Result<SyncReport, SyncError> {
    let mut report = SyncReport::default();
    let device = store.origin().to_owned();

    loop {
        let after = kv_i64(store, PUSHED_KEY).await?;
        let changes = store.own_changes_since(after, BATCH).await?;
        let Some(last) = changes.last().map(|c| c.seq) else {
            break;
        };
        let n = u32::try_from(changes.len()).unwrap_or(u32::MAX);
        transport
            .push(&PushRequest {
                device: device.clone(),
                changes,
            })
            .await?;
        store.kv_set(PUSHED_KEY, &last.to_string()).await?;
        report.pushed += n;
    }

    loop {
        let after = kv_i64(store, PULLED_KEY).await?;
        let page = transport.pull(&device, after, BATCH).await?;
        for c in &page.changes {
            match store.apply_remote(c, false).await? {
                Applied::Applied => report.pulled_applied += 1,
                Applied::Skipped => report.pulled_skipped += 1,
            }
        }
        store.kv_set(PULLED_KEY, &page.cursor.to_string()).await?;
        if !page.more {
            break;
        }
    }
    Ok(report)
}
