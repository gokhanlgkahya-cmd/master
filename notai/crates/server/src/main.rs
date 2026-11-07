//! `notai-server` çalıştırılabilir dosyası.

#![forbid(unsafe_code)]

use std::process::ExitCode;
use std::sync::Arc;

use notai_server::{AppState, Config, SystemClock, build_llm, router, scheduler, worker};
use notai_store::Store;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let cfg = Config::from_env().map_err(|e| format!("yapılandırma: {e}"))?;
    let store = Store::open(&cfg.database_url, cfg.origin.clone())
        .await
        .map_err(|e| format!("veritabanı: {e}"))?;
    let llm = build_llm(&cfg.ai, std::env::var("ANTHROPIC_API_KEY").ok())
        .map_err(|e| format!("AI: {e}"))?;
    if cfg.api_token.is_none() {
        tracing::warn!(
            "NOTAI_API_TOKEN boş: API kimlik doğrulamasız (yalnız güvenli ağda kullanın)"
        );
    }
    let bind = cfg.bind;
    let state = AppState {
        store: store.clone(),
        llm,
        cfg: Arc::new(cfg),
        clock: Arc::new(SystemClock),
    };

    let (stop_tx, stop_rx) = watch::channel(false);
    let worker = tokio::spawn(worker::run(state.clone(), stop_rx.clone()));
    let sched = tokio::spawn(scheduler::run(state.clone(), stop_rx));

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| format!("{bind} dinlenemedi: {e}"))?;
    tracing::info!(%bind, "NotAI sunucusu çalışıyor");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("kapatılıyor…");
        })
        .await
        .map_err(|e| format!("sunucu: {e}"))?;

    let _ = stop_tx.send(true);
    let _ = worker.await;
    let _ = sched.await;
    store.close().await;
    Ok(())
}
