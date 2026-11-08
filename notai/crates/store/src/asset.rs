//! Varlıklar: değer geçmişi, özet ve ajan için fiyatı güncellenecek varlıklar.

use chrono::{DateTime, NaiveDate, Utc};
use notai_core::asset::{AssetInfo, NOTIFY_CHANGE_PERMILLE};
use notai_core::entry::Source;
use notai_core::finance::Money;
use notai_core::text::EntryText;
use notai_core::topic::{ParaKind, Topic};
use notai_core::{Entry, TopicId};
use sqlx::SqliteConnection;

use crate::Store;
use crate::codec::date_str;
use crate::entries::NewEntry;
use crate::error::Result;
use crate::topics::NewTopic;

/// Varlıkların toplandığı ana konunun adı.
pub const ASSETS_ROOT: &str = "Varlıklar";

/// Konunun varlık değerini geçmişe yazar (aynı gün tek kayıt; sonuncusu geçerli).
pub(crate) async fn record_asset_value(conn: &mut SqliteConnection, t: &Topic) -> Result<()> {
    let Some(a) = &t.asset else {
        return Ok(());
    };
    let id = t.id.to_string();
    let day = date_str(a.as_of);
    sqlx::query!(
        "INSERT INTO asset_values (topic_id, as_of, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (topic_id, as_of) DO UPDATE SET value = excluded.value",
        id,
        day,
        a.value.0
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

impl Store {
    /// Tüm varlık konuları (arşiv dahil değil).
    pub async fn asset_topics(&self) -> Result<Vec<Topic>> {
        Ok(self
            .export_snapshot()
            .await?
            .topics
            .into_iter()
            .filter(|t| t.asset.is_some() && t.kind != ParaKind::Archive)
            .collect())
    }

    /// Bugünkü varlıkların `day` günündeki toplam değeri: her varlık için o gün ya da öncesindeki
    /// son kayıt; o günden sonra eklenmiş varlık için ilk kaydı (yeni varlık "kazanç" sayılmaz).
    /// Hiç geçmiş kaydı `day` öncesine uzanmıyorsa `None`.
    pub async fn assets_total_on(&self, day: NaiveDate) -> Result<Option<Money>> {
        let ids: std::collections::HashSet<String> = self
            .asset_topics()
            .await?
            .iter()
            .map(|t| t.id.to_string())
            .collect();
        let rows = sqlx::query!(
            r#"SELECT topic_id AS "topic_id!", as_of AS "as_of!", value AS "value!: i64"
               FROM asset_values ORDER BY topic_id, as_of"#
        )
        .fetch_all(&self.pool)
        .await?;
        let d = date_str(day);
        let mut per: std::collections::BTreeMap<String, (Option<i64>, i64)> =
            std::collections::BTreeMap::new();
        let mut any_past = false;
        for r in rows.into_iter().filter(|r| ids.contains(&r.topic_id)) {
            let slot = per.entry(r.topic_id).or_insert((None, r.value));
            if r.as_of <= d {
                slot.0 = Some(r.value);
                any_past = true;
            }
        }
        if !any_past {
            return Ok(None);
        }
        Ok(Some(Money(
            per.values()
                .map(|(past, first)| past.unwrap_or(*first))
                .sum(),
        )))
    }

    /// "Varlıklar" ana konusu (yoksa açılır).
    pub async fn assets_root(&self, now: DateTime<Utc>) -> Result<Topic> {
        let name = notai_core::text::TopicName::new(ASSETS_ROOT)?;
        if let Some(t) = self.topic_by_name(&name).await? {
            return Ok(t);
        }
        self.create_topic(NewTopic::new(name, ParaKind::Area), now)
            .await
    }

    /// Ajanın bildirdiği birim fiyatla varlığı yeniden değerler. Değişim ≥ %2 ise konuya NotAI
    /// mesajı yazılır (mesaj döner).
    pub async fn price_asset(
        &self,
        topic: TopicId,
        unit_price: Money,
        now: DateTime<Utc>,
        today: NaiveDate,
    ) -> Result<Option<Entry>> {
        let mut t = self.topic(topic).await?;
        let Some(info) = t.asset.clone() else {
            return Err(crate::StoreError::Conflict(format!(
                "\"{}\" bir varlık değil",
                t.name
            )));
        };
        let (new, change) = info.revalue(unit_price, today);
        let old = info.value;
        t.asset = Some(new.clone());
        self.update_topic(&t, now).await?;
        if change.abs() < NOTIFY_CHANGE_PERMILLE {
            return Ok(None);
        }
        let sign = if change > 0 { "+" } else { "−" };
        let text = format!(
            "Değer {}: {} (önceki {}, {sign}%{},{}){}",
            if change > 0 { "arttı" } else { "düştü" },
            new.value.format_abs(),
            old.format_abs(),
            change.abs() / 10,
            change.abs() % 10,
            new.symbol.as_deref().map_or_else(String::new, |s| format!(
                " — {s}: {}",
                unit_price.format_abs()
            ))
        );
        let e = self
            .capture_notai(NewEntry {
                text: EntryText::new(&text)?,
                source: Source::App,
                topic: Some(topic),
                captured_at: now,
                dedupe_key: None,
            })
            .await?;
        Ok(Some(e))
    }

    /// Fiyatı bugün güncellenmemiş, sembollü varlıklar (ajan günde bir çağırır).
    pub async fn assets_to_price(&self, today: NaiveDate) -> Result<Vec<(Topic, AssetInfo)>> {
        Ok(self
            .asset_topics()
            .await?
            .into_iter()
            .filter_map(|t| {
                let a = t.asset.clone()?;
                (a.symbol.is_some() && a.as_of < today).then_some((t, a))
            })
            .collect())
    }
}
