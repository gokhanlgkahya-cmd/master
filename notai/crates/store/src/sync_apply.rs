//! Uzaktaki cihazdan gelen değişiklikleri uygulama (senkron).
//!
//! Her uygulanan değişiklik yerel günlüğe **uzaktaki kaynak ve zamanla** yazılır; böylece
//! (a) aynı varlığın yerel sürümü güncellenir, (b) değişiklik diğer cihazlara yayılır,
//! (c) kaynak cihaza geri yankılanmaz.

use chrono::DateTime;
use chrono::Utc;
use notai_core::entry::Entry;
use notai_core::profile::{FactStatus, ProfileFact};
use notai_core::suggestion::Suggestion;
use notai_core::sync::{Version, remote_wins, suggestion_remote_wins};
use notai_core::task::Task;
use notai_core::text::TopicName;
use notai_core::topic::Topic;
use serde::Deserialize;
use sqlx::SqliteConnection;

use crate::changes::ChangeRecord;
use crate::codec::{SuggestionRow, from_json, parse_ts, ts};
use crate::entries::update_entry_fields;
use crate::error::{Result, StoreError};
use crate::topics::name_key;
use crate::{Store, log_change};

/// Uygulama sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    Applied,
    /// Yereldeki sürüm daha yeni ya da değişiklik zaten uygulanmış.
    Skipped,
}

#[derive(Debug, Deserialize)]
struct LabelPayload {
    entry: String,
    kind: String,
    name: String,
}

async fn local_version(
    conn: &mut SqliteConnection,
    entity: &str,
    id: &str,
) -> Result<Option<Version>> {
    let row = sqlx::query!(
        "SELECT at, origin FROM change_log WHERE entity = ?1 AND entity_id = ?2
         ORDER BY at DESC, origin DESC LIMIT 1",
        entity,
        id
    )
    .fetch_optional(&mut *conn)
    .await?;
    row.map(|r| Ok(Version::new(parse_ts("değişiklik", &r.at)?, r.origin)))
        .transpose()
}

async fn exists(conn: &mut SqliteConnection, entity: &str, id: &str) -> Result<bool> {
    let n: i64 = match entity {
        "entry" => {
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!: i64" FROM entries WHERE id = ?1"#,
                id
            )
            .fetch_one(&mut *conn)
            .await?
        }
        "topic" => {
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!: i64" FROM topics WHERE id = ?1"#,
                id
            )
            .fetch_one(&mut *conn)
            .await?
        }
        "task" => {
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!: i64" FROM tasks WHERE id = ?1"#,
                id
            )
            .fetch_one(&mut *conn)
            .await?
        }
        "suggestion" => {
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!: i64" FROM suggestions WHERE id = ?1"#,
                id
            )
            .fetch_one(&mut *conn)
            .await?
        }
        "fact" => {
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!: i64" FROM profile_facts WHERE id = ?1"#,
                id
            )
            .fetch_one(&mut *conn)
            .await?
        }
        "transaction" => {
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!: i64" FROM transactions WHERE id = ?1"#,
                id
            )
            .fetch_one(&mut *conn)
            .await?
        }
        _ => 0,
    };
    Ok(n > 0)
}

async fn insert_entry_snapshot(
    conn: &mut SqliteConnection,
    e: &Entry,
    queue_ai: bool,
    at: DateTime<Utc>,
) -> Result<()> {
    let id = e.id.to_string();
    let text = e.text.as_str();
    let source = e.source.as_str();
    let captured = ts(e.captured_at);
    let topic = e.topic().map(|t| t.to_string());
    let status = e.status().name();
    let trashed = e.status().trashed_at().map(ts);
    let ai_state = if queue_ai && !e.from_notai {
        "pending"
    } else {
        "done"
    };
    let author = e.from_notai.then_some("notai");
    let at_s = ts(at);
    sqlx::query!(
        "INSERT INTO entries (id, text, source, captured_at, topic_id, status, trashed_at, ai_state, updated_at, author)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        id,
        text,
        source,
        captured,
        topic,
        status,
        trashed,
        ai_state,
        at_s,
        author
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Bir ad için boş olan ilk " (n)" ekli adı bulur.
async fn free_name(
    conn: &mut SqliteConnection,
    parent: Option<notai_core::TopicId>,
    base: &TopicName,
) -> Result<TopicName> {
    for n in 2..=99 {
        let candidate = TopicName::new(format!("{base} ({n})"))?;
        let key = name_key(parent, &candidate);
        let taken = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "n!: i64" FROM topics WHERE name_key = ?1"#,
            key
        )
        .fetch_one(&mut *conn)
        .await?;
        if taken == 0 {
            return Ok(candidate);
        }
    }
    Err(StoreError::Conflict(format!(
        "\"{base}\" için boş ad bulunamadı"
    )))
}

/// Konu ekler. Aynı adda başka kimlikli konu varsa veri kaybetmemek için ikisi de tutulur:
/// **kimliği küçük olan (önce oluşturulan) adını korur**, diğeri " (2)" eki alır. Kural
/// deterministik olduğu için tüm cihazlar aynı sonuca varır.
async fn insert_topic_snapshot(
    conn: &mut SqliteConnection,
    t: &Topic,
    at: DateTime<Utc>,
) -> Result<()> {
    let id = t.id.to_string();
    let key = name_key(t.parent, &t.name);
    let existing = sqlx::query!("SELECT id, name FROM topics WHERE name_key = ?1", key)
        .fetch_optional(&mut *conn)
        .await?;
    let mut name = t.name.clone();
    if let Some(ex) = existing {
        if ex.id.as_str() > id.as_str() {
            // Yereldeki daha yeni: yerelinkini yeniden adlandır, gelen adı korusun.
            let renamed = free_name(conn, t.parent, &TopicName::new(&ex.name)?).await?;
            let rkey = name_key(t.parent, &renamed);
            let rname = renamed.as_str();
            sqlx::query!(
                "UPDATE topics SET name = ?2, name_key = ?3 WHERE id = ?1",
                ex.id,
                rname,
                rkey
            )
            .execute(&mut *conn)
            .await?;
        } else {
            name = free_name(conn, t.parent, &t.name).await?;
        }
    }
    let key = name_key(t.parent, &name);
    let parent = t.parent.map(|p| p.to_string());
    let watch = t.watch.as_ref().map(serde_json::to_string).transpose()?;
    let asset = t.asset.as_ref().map(serde_json::to_string).transpose()?;
    let name_s = name.as_str();
    let kind = t.kind.name();
    let due = t.kind.due().map(crate::codec::date_str);
    let pinned = i64::from(t.pinned);
    let at_s = ts(at);
    sqlx::query!(
        "INSERT INTO topics (id, name, name_key, kind, due_date, icon, avatar, pinned, created_at, updated_at, parent_id, watch, asset)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?9, ?7, ?8, ?8, ?10, ?11, ?12)",
        id,
        name_s,
        key,
        kind,
        due,
        t.icon,
        pinned,
        at_s,
        t.avatar,
        parent,
        watch,
        asset
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn update_topic_snapshot(
    conn: &mut SqliteConnection,
    t: &Topic,
    at: DateTime<Utc>,
) -> Result<()> {
    let id = t.id.to_string();
    let kind = t.kind.name();
    let due = t.kind.due().map(crate::codec::date_str);
    let pinned = i64::from(t.pinned);
    let key = name_key(t.parent, &t.name);
    let parent = t.parent.map(|p| p.to_string());
    let watch = t.watch.as_ref().map(serde_json::to_string).transpose()?;
    let asset = t.asset.as_ref().map(serde_json::to_string).transpose()?;
    let name = t.name.as_str();
    let at_s = ts(at);
    // Ad başka konuyla çakışırsa adı değiştirme, diğer alanları uygula.
    let clash = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "n!: i64" FROM topics WHERE name_key = ?1 AND id != ?2"#,
        key,
        id
    )
    .fetch_one(&mut *conn)
    .await?;
    if clash > 0 {
        sqlx::query!(
            "UPDATE topics SET kind = ?2, due_date = ?3, icon = ?4, pinned = ?5, updated_at = ?6, avatar = ?7,
                    watch = ?8, asset = ?9
             WHERE id = ?1",
            id,
            kind,
            due,
            t.icon,
            pinned,
            at_s,
            t.avatar,
            watch,
            asset
        )
        .execute(&mut *conn)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE topics SET name = ?2, name_key = ?3, kind = ?4, due_date = ?5, icon = ?6, pinned = ?7,
                    updated_at = ?8, avatar = ?9, parent_id = ?10, watch = ?11, asset = ?12
             WHERE id = ?1",
            id,
            name,
            key,
            kind,
            due,
            t.icon,
            pinned,
            at_s,
            t.avatar,
            parent,
            watch,
            asset
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn upsert_fact(
    conn: &mut SqliteConnection,
    f: &ProfileFact,
    at: DateTime<Utc>,
) -> Result<()> {
    let id = f.id.to_string();
    let text = f.text.as_str();
    let topic = f.topic.map(|t| t.to_string());
    let status = match f.status {
        FactStatus::Proposed => "proposed",
        FactStatus::Confirmed => "confirmed",
    };
    let at_s = ts(at);
    sqlx::query!(
        "INSERT INTO profile_facts (id, text, topic_id, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (id) DO UPDATE SET text = excluded.text, topic_id = excluded.topic_id,
                                        status = excluded.status",
        id,
        text,
        topic,
        status,
        at_s
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

impl Store {
    /// Uzaktan gelen tek bir değişikliği uygular. `queue_ai`: yeni gelen kayıtlar bu cihazda
    /// AI kuyruğuna girsin mi (yalnız sunucuda `true`).
    pub async fn apply_remote(&self, c: &ChangeRecord, queue_ai: bool) -> Result<Applied> {
        if c.origin == self.origin {
            // Kendi değişikliğimiz geri döndü: yok say.
            return Ok(Applied::Skipped);
        }
        let mut tx = self.pool.begin().await?;
        let remote = Version::new(c.at, c.origin.clone());
        let local = local_version(&mut tx, &c.entity, &c.entity_id).await?;
        let wins = remote_wins(local.as_ref(), &remote);
        let present = exists(&mut tx, &c.entity, &c.entity_id).await?;
        let id = c.entity_id.as_str();

        let applied = match (c.entity.as_str(), c.payload.as_deref()) {
            ("entry", Some(p)) => {
                let e: Entry = from_json("senkron kaydı", p)?;
                if !present {
                    insert_entry_snapshot(&mut tx, &e, queue_ai, c.at).await?;
                    true
                } else if wins {
                    update_entry_fields(&mut tx, &e, c.at).await?;
                    true
                } else {
                    false
                }
            }
            ("entry", None) => {
                // Kalıcı silme her zaman kazanır.
                sqlx::query!("DELETE FROM entries WHERE id = ?1", id)
                    .execute(&mut *tx)
                    .await?;
                present
            }
            ("topic", Some(p)) => {
                let t: Topic = from_json("senkron konusu", p)?;
                if !present {
                    insert_topic_snapshot(&mut tx, &t, c.at).await?;
                    crate::asset::record_asset_value(&mut tx, &t).await?;
                    true
                } else if wins {
                    update_topic_snapshot(&mut tx, &t, c.at).await?;
                    crate::asset::record_asset_value(&mut tx, &t).await?;
                    true
                } else {
                    false
                }
            }
            ("suggestion", Some(p)) => {
                let s: Suggestion = from_json("senkron önerisi", p)?;
                if !present {
                    if exists(&mut tx, "entry", &s.entry.to_string()).await? {
                        crate::suggestions::insert_suggestion_raw(&mut tx, &s, c.at).await?;
                        true
                    } else {
                        tracing::warn!(suggestion = %s.id, "önerinin kaydı yok, atlandı");
                        false
                    }
                } else {
                    let row = sqlx::query_as!(
                        SuggestionRow,
                        "SELECT id, entry_id, payload, confidence, decision, decided_at FROM suggestions WHERE id = ?1",
                        id
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let current = Suggestion::try_from(row)?;
                    if suggestion_remote_wins(
                        current.decision(),
                        s.decision(),
                        local.as_ref(),
                        &remote,
                    ) {
                        crate::suggestions::write_decision_raw(&mut tx, &s).await?;
                        // Başka cihazda cevaplanan AI sorusu: AI çalıştıran yerde (sunucu) kayıt
                        // cevapla yeniden sınıflandırılsın.
                        if queue_ai && matches!(s.question_and_answer(), Some((_, Some(_)))) {
                            crate::suggestions::requeue_ai(&mut tx, s.entry).await?;
                        }
                        true
                    } else {
                        false
                    }
                }
            }
            ("task", Some(p)) => {
                let t: Task = from_json("senkron görevi", p)?;
                if !present {
                    crate::tasks::insert_task_raw(&mut tx, &t, c.at).await?;
                    true
                } else if wins {
                    crate::tasks::update_task_raw(&mut tx, &t, c.at).await?;
                    true
                } else {
                    false
                }
            }
            ("label", Some(p)) => {
                let l: LabelPayload = from_json("senkron etiketi", p)?;
                let res = sqlx::query!(
                    "INSERT OR IGNORE INTO entry_labels (entry_id, kind, name)
                     SELECT ?1, ?2, ?3 WHERE EXISTS (SELECT 1 FROM entries WHERE id = ?1)",
                    l.entry,
                    l.kind,
                    l.name
                )
                .execute(&mut *tx)
                .await?;
                res.rows_affected() > 0
            }
            ("fact", Some(p)) => {
                let f: ProfileFact = from_json("senkron profil bilgisi", p)?;
                if !present || wins {
                    upsert_fact(&mut tx, &f, c.at).await?;
                    true
                } else {
                    false
                }
            }
            ("fact", None) => {
                if wins {
                    sqlx::query!("DELETE FROM profile_facts WHERE id = ?1", id)
                        .execute(&mut *tx)
                        .await?;
                }
                wins && present
            }
            ("transaction", Some(p)) => {
                let t: notai_core::finance::Transaction = from_json("senkron hareketi", p)?;
                if !present || wins {
                    crate::finance::upsert_transaction_raw(&mut tx, &t, c.at).await?;
                    true
                } else {
                    false
                }
            }
            ("transaction", None) => {
                if wins {
                    sqlx::query!("DELETE FROM transactions WHERE id = ?1", id)
                        .execute(&mut *tx)
                        .await?;
                }
                wins && present
            }
            ("merchant_rule", Some(p)) => {
                let r: notai_core::finance::MerchantRule = from_json("senkron satıcı kuralı", p)?;
                if wins || local.is_none() {
                    crate::finance::upsert_rule_raw(&mut tx, &r, c.at).await?;
                    true
                } else {
                    false
                }
            }
            ("attachment", Some(p)) => {
                let a: crate::extras::AttachmentPayload = from_json("senkron eki", p)?;
                let entry_ok = exists(&mut tx, "entry", &a.entry).await?;
                if entry_ok {
                    let data = crate::extras::decode_attachment(&a)?;
                    crate::extras::insert_attachment(&mut tx, &a, data.as_deref(), c.at).await?;
                }
                entry_ok
            }
            (other, _) => {
                return Err(StoreError::corrupt(
                    "senkron",
                    format!("bilinmeyen varlık: {other}"),
                ));
            }
        };

        if applied {
            log_change(
                &mut tx,
                &c.origin,
                &c.entity,
                &c.entity_id,
                c.payload.as_deref(),
                c.at,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(if applied {
            Applied::Applied
        } else {
            Applied::Skipped
        })
    }
}
