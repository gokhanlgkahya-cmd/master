//! Ses → yazı kuyruğu: telefonun anında tanıdığı metin (ya da yer tutucu) ev sunucusundaki
//! Whisper ile düzeltilir. Sunucuya ulaşılamazsa kayıt bekler; senkronla gelince işlenir.

use chrono::{DateTime, Duration, Utc};
use notai_core::EntryId;
use notai_core::text::EntryText;
use sqlx::SqliteConnection;

use crate::Store;
use crate::codec::{parse_id, ts};
use crate::entries::{load_entry, update_entry};
use crate::error::Result;

/// Telefon metni çıkaramadıysa kayda bu yazılır; Whisper metni gelince değişir.
pub const VOICE_PLACEHOLDER: &str = "Sesli not (yazıya çevriliyor…)";
/// Whisper hiç başaramazsa yer tutucunun yerine.
pub const VOICE_FAILED: &str = "Sesli not (yazıya çevrilemedi; sesi dinleyebilirsin)";
/// Bu kadar denemeden sonra vazgeçilir.
pub const MAX_ATTEMPTS: i64 = 6;

/// İşlenecek ses.
#[derive(Debug, Clone)]
pub struct TranscriptJob {
    pub entry: EntryId,
    pub mime: String,
    pub audio: Vec<u8>,
    /// Kayıttaki şu anki metin (telefonun tanıdığı ya da yer tutucu).
    pub current: String,
}

/// Sonuç: metin değişti mi, vazgeçildi mi?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptOutcome {
    Corrected,
    Unchanged,
    Retry,
    GaveUp,
}

/// Ses eki eklenince (yerel ya da senkron): kayıt sesliyse kuyruğa al.
pub(crate) async fn enqueue_if_voice(
    conn: &mut SqliteConnection,
    entry: &str,
    attachment: &str,
    kind: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    if kind != "audio" {
        return Ok(());
    }
    let at = ts(now);
    sqlx::query!(
        "INSERT OR IGNORE INTO transcripts (entry_id, attachment_id, state, created_at, updated_at)
         SELECT id, ?2, 'pending', ?3, ?3 FROM entries WHERE id = ?1 AND source = 'voice'",
        entry,
        attachment,
        at
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

fn backoff(attempts: i64) -> Duration {
    Duration::minutes(match attempts {
        ..=1 => 1,
        2 => 5,
        3 => 30,
        4 => 120,
        _ => 360,
    })
}

impl Store {
    /// Sesli kayıt: telefonun anında tanıdığı metin (yoksa yer tutucu) kayıt olur, ses eki
    /// eklenir ve Whisper kuyruğuna girer. Yer tutucu AI'a gitmez (metin gelince gider).
    pub async fn capture_voice(
        &self,
        draft: Option<&str>,
        audio: Option<(Vec<u8>, String)>,
        topic: Option<notai_core::TopicId>,
        now: DateTime<Utc>,
    ) -> Result<notai_core::Entry> {
        let draft = draft.map(str::trim).filter(|t| !t.is_empty());
        if draft.is_none() && audio.is_none() {
            return Err(crate::StoreError::Conflict("ses de metin de yok".into()));
        }
        let text = EntryText::new(draft.unwrap_or(VOICE_PLACEHOLDER))?;
        let captured = self
            .capture(crate::NewEntry {
                text,
                source: notai_core::Source::Voice,
                topic,
                captured_at: now,
                dedupe_key: None,
            })
            .await?;
        let e = captured.entry().clone();
        if draft.is_none() {
            let id = e.id.to_string();
            sqlx::query!(
                "UPDATE entries SET ai_state = 'done' WHERE id = ?1 AND ai_state = 'pending'",
                id
            )
            .execute(&self.pool)
            .await?;
        }
        if let Some((data, mime)) = audio {
            self.add_attachment(
                crate::NewAttachment {
                    entry: e.id,
                    kind: crate::AttachmentKind::Audio,
                    name: "ses kaydı".into(),
                    mime,
                    data: Some(data),
                    value: None,
                },
                now,
            )
            .await?;
        }
        Ok(e)
    }

    /// Sıradaki ses (zamanı gelmiş, kaydı çöpte olmayan).
    pub async fn next_transcript(&self, now: DateTime<Utc>) -> Result<Option<TranscriptJob>> {
        let now_s = ts(now);
        let row = sqlx::query!(
            r#"SELECT t.entry_id AS "entry_id!", a.mime AS "mime!", a.data AS "data?", e.text AS "text!"
                 FROM transcripts t
                 JOIN attachments a ON a.id = t.attachment_id
                 JOIN entries e ON e.id = t.entry_id
                WHERE t.state = 'pending' AND (t.next_at IS NULL OR t.next_at <= ?1)
                  AND e.status != 'trashed'
                ORDER BY t.created_at LIMIT 1"#,
            now_s
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(r) = row else {
            return Ok(None);
        };
        Ok(Some(TranscriptJob {
            entry: parse_id("kayıt", &r.entry_id)?,
            mime: r.mime,
            audio: r.data.unwrap_or_default(),
            current: r.text,
        }))
    }

    /// Whisper sonucunu işler: metin farklıysa kayıt düzeltilir (senkronla telefona gider),
    /// gerekirse AI yeniden sınıflandırır; hata olursa artan aralıkla yeniden denenir.
    pub async fn finish_transcript(
        &self,
        entry: EntryId,
        result: std::result::Result<String, String>,
        now: DateTime<Utc>,
    ) -> Result<TranscriptOutcome> {
        let id = entry.to_string();
        let now_s = ts(now);
        let mut tx = self.pool.begin().await?;
        let mut e = load_entry(&mut tx, entry).await?;
        let was_placeholder = e.text.as_str() == VOICE_PLACEHOLDER;
        let outcome = match result {
            Ok(text) => {
                let new = EntryText::new(text.trim())?;
                let changed = new.as_str() != e.text.as_str();
                if changed {
                    e.text = new;
                    update_entry(&mut tx, &self.origin, &e, now).await?;
                    // Kararı verilmiş öneri yoksa AI düzeltilmiş metinle yeniden bakar.
                    let decided = sqlx::query_scalar!(
                        r#"SELECT COUNT(*) AS "n!: i64" FROM suggestions
                            WHERE entry_id = ?1 AND decision != 'pending' AND kind != 'question'"#,
                        id
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    if decided == 0 {
                        sqlx::query!(
                            "DELETE FROM suggestions WHERE entry_id = ?1 AND decision = 'pending' AND kind != 'question'",
                            id
                        )
                        .execute(&mut *tx)
                        .await?;
                        crate::suggestions::requeue_ai(&mut tx, entry).await?;
                    }
                }
                sqlx::query!(
                    "UPDATE transcripts SET state = 'done', error = NULL, updated_at = ?2 WHERE entry_id = ?1",
                    id,
                    now_s
                )
                .execute(&mut *tx)
                .await?;
                if changed {
                    TranscriptOutcome::Corrected
                } else {
                    TranscriptOutcome::Unchanged
                }
            }
            Err(err) => {
                let attempts = sqlx::query_scalar!(
                    r#"SELECT attempts AS "n!: i64" FROM transcripts WHERE entry_id = ?1"#,
                    id
                )
                .fetch_one(&mut *tx)
                .await?
                    + 1;
                if attempts >= MAX_ATTEMPTS {
                    sqlx::query!(
                        "UPDATE transcripts SET state = 'failed', attempts = ?2, error = ?3, updated_at = ?4
                         WHERE entry_id = ?1",
                        id,
                        attempts,
                        err,
                        now_s
                    )
                    .execute(&mut *tx)
                    .await?;
                    if was_placeholder {
                        e.text = EntryText::new(VOICE_FAILED)?;
                        update_entry(&mut tx, &self.origin, &e, now).await?;
                    }
                    TranscriptOutcome::GaveUp
                } else {
                    let next = ts(now + backoff(attempts));
                    sqlx::query!(
                        "UPDATE transcripts SET attempts = ?2, error = ?3, next_at = ?4, updated_at = ?5
                         WHERE entry_id = ?1",
                        id,
                        attempts,
                        err,
                        next,
                        now_s
                    )
                    .execute(&mut *tx)
                    .await?;
                    TranscriptOutcome::Retry
                }
            }
        };
        tx.commit().await?;
        // Yer tutucunun yerine gelen metin bir hareketse ("işe geldim") hareket çıkarılır.
        if was_placeholder && outcome == TranscriptOutcome::Corrected {
            let e = self.entry(entry).await?;
            if let Err(err) = self.detect_movement(&e, now).await {
                tracing::warn!(error = %err, "hareket çıkarılamadı");
            }
        }
        Ok(outcome)
    }

    /// Bekleyen ses sayısı (durum için).
    pub async fn pending_transcripts(&self) -> Result<i64> {
        Ok(sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "n!: i64" FROM transcripts WHERE state = 'pending'"#
        )
        .fetch_one(&self.pool)
        .await?)
    }
}
