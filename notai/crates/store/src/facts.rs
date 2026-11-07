//! Kullanıcıyı tanıma bilgileri (ProfileFact).

use chrono::{DateTime, Utc};
use notai_core::ProfileFactId;
use notai_core::profile::{FactStatus, ProfileFact};
use notai_core::text::FactText;

use crate::codec::{parse_id, parse_opt_id, ts};
use crate::error::{Result, StoreError};
use crate::{Store, log_change};

fn status_name(s: FactStatus) -> &'static str {
    match s {
        FactStatus::Proposed => "proposed",
        FactStatus::Confirmed => "confirmed",
    }
}

impl Store {
    pub async fn add_fact(&self, fact: &ProfileFact, now: DateTime<Utc>) -> Result<()> {
        let id = fact.id.to_string();
        let text = fact.text.as_str();
        let topic = fact.topic.map(|t| t.to_string());
        let status = status_name(fact.status);
        let now_s = ts(now);
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "INSERT INTO profile_facts (id, text, topic_id, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            id,
            text,
            topic,
            status,
            now_s
        )
        .execute(&mut *tx)
        .await?;
        let payload = serde_json::to_string(fact)?;
        log_change(&mut tx, &self.origin, "fact", &id, Some(&payload), now).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn facts(&self) -> Result<Vec<ProfileFact>> {
        let rows = sqlx::query!(
            "SELECT id, text, topic_id, status FROM profile_facts ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                let status = match r.status.as_str() {
                    "proposed" => FactStatus::Proposed,
                    "confirmed" => FactStatus::Confirmed,
                    other => return Err(StoreError::corrupt("profil bilgisi", other)),
                };
                Ok(ProfileFact {
                    id: parse_id("profil bilgisi", &r.id)?,
                    text: FactText::new(&r.text)?,
                    topic: parse_opt_id("profil bilgisi", r.topic_id.as_deref())?,
                    status,
                })
            })
            .collect()
    }

    pub async fn confirm_fact(&self, id: ProfileFactId, now: DateTime<Utc>) -> Result<()> {
        let id_s = id.to_string();
        let mut tx = self.pool.begin().await?;
        let res = sqlx::query!(
            "UPDATE profile_facts SET status = 'confirmed' WHERE id = ?1",
            id_s
        )
        .execute(&mut *tx)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::not_found("profil bilgisi", id));
        }
        log_change(
            &mut tx,
            &self.origin,
            "fact",
            &id_s,
            Some("{\"status\":\"confirmed\"}"),
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Bilgiyi siler (kullanıcı ayarıdır; kayıt verisi değildir).
    pub async fn delete_fact(&self, id: ProfileFactId, now: DateTime<Utc>) -> Result<()> {
        let id_s = id.to_string();
        let mut tx = self.pool.begin().await?;
        let res = sqlx::query!("DELETE FROM profile_facts WHERE id = ?1", id_s)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::not_found("profil bilgisi", id));
        }
        log_change(&mut tx, &self.origin, "fact", &id_s, None, now).await?;
        tx.commit().await?;
        Ok(())
    }
}
