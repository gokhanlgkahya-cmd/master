//! Paylaşılan uygulama durumu ve saat soyutlaması.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use notai_ai::LlmClient;
use notai_store::Store;

use crate::config::Config;

/// Saat: testlerde sabitlenebilsin diye soyut.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Elle ilerletilen saat (testler için).
#[derive(Debug)]
pub struct ManualClock(Mutex<DateTime<Utc>>);

impl ManualClock {
    #[must_use]
    pub const fn new(at: DateTime<Utc>) -> Self {
        Self(Mutex::new(at))
    }

    pub fn set(&self, at: DateTime<Utc>) {
        if let Ok(mut g) = self.0.lock() {
            *g = at;
        }
    }
}

impl Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
            .lock()
            .map(|g| *g)
            .unwrap_or_else(|p| *p.into_inner())
    }
}

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub llm: Option<Arc<dyn LlmClient>>,
    pub cfg: Arc<Config>,
    pub clock: Arc<dyn Clock>,
}

impl AppState {
    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }
}
