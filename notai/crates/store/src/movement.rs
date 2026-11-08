//! Konum hareketleri: yerler ("Hareketler" altındaki konular), kayıttan hareket çıkarma,
//! konumla teyit, eksikler için NotAI sorusu, cevabı işleme ve rapor verisi.

use chrono::{DateTime, Duration, NaiveTime, Utc};
use notai_core::finance::Money;
use notai_core::movement::{
    self as mv, Check, DEFAULT_PLACES, Fix, MOVES_ROOT, MoveKind, Movement, Place, PlaceInfo,
    PlaceRef, SpendAt,
};
use notai_core::suggestion::{Confidence, Suggestion, SuggestionKind};
use notai_core::text::{QuestionText, TopicName};
use notai_core::topic::{ParaKind, Topic};
use notai_core::{Entry, EntryId, TopicId};
use serde::Serialize;
use sqlx::SqliteConnection;

use crate::Store;
use crate::codec::{from_json, parse_id, ts};
use crate::error::Result;
use crate::log_change;
use crate::topics::{find_by_name, get_topic, insert_topic};

/// Konum gelmezse soru bu kadar bekledikten sonra sorulur (CLI / sunucuda yakalanan kayıtlar).
pub const SETTLE_AFTER_SECS: i64 = 90;

/// Konum denemesinin sonucu (arayüze gösterilir).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocateOutcome {
    pub message: String,
    /// Konum kapalı ve bu hareket için gerekli: "konumu aç" bildirimi gönderilmeli.
    pub need_location: bool,
    /// Bu adımda açılan NotAI sorusu (bildirim için).
    pub question: Option<String>,
}

async fn load_places(conn: &mut SqliteConnection) -> Result<Vec<Place>> {
    let rows = sqlx::query!(
        r#"SELECT p.topic_id AS "topic_id!", p.info AS "info!", t.name AS "name!"
             FROM places p JOIN topics t ON t.id = p.topic_id
            WHERE t.kind != 'archive'"#
    )
    .fetch_all(&mut *conn)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(Place {
                topic: parse_id("yer", &r.topic_id)?,
                name: r.name,
                info: from_json("yer ayarı", &r.info)?,
            })
        })
        .collect()
}

pub(crate) async fn upsert_place_raw(
    conn: &mut SqliteConnection,
    topic: TopicId,
    info: &PlaceInfo,
    at: DateTime<Utc>,
) -> Result<()> {
    let id = topic.to_string();
    let data = serde_json::to_string(info)?;
    let at_s = ts(at);
    sqlx::query!(
        "INSERT INTO places (topic_id, info, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (topic_id) DO UPDATE SET info = excluded.info, updated_at = excluded.updated_at",
        id,
        data,
        at_s
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Senkron yükü: yer ayarı + konu kimliği.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct PlacePayload {
    pub topic: TopicId,
    pub info: PlaceInfo,
}

async fn save_place(
    conn: &mut SqliteConnection,
    origin: &str,
    topic: TopicId,
    info: &PlaceInfo,
    at: DateTime<Utc>,
) -> Result<()> {
    upsert_place_raw(conn, topic, info, at).await?;
    let payload = serde_json::to_string(&PlacePayload {
        topic,
        info: info.clone(),
    })?;
    log_change(
        conn,
        origin,
        "place",
        &topic.to_string(),
        Some(&payload),
        at,
    )
    .await
}

pub(crate) async fn upsert_movement_raw(
    conn: &mut SqliteConnection,
    m: &Movement,
    at: DateTime<Utc>,
) -> Result<()> {
    let entry = m.entry.to_string();
    let when = ts(m.at);
    let data = serde_json::to_string(m)?;
    let at_s = ts(at);
    let settled = i64::from(m.settled);
    sqlx::query!(
        "INSERT INTO movements (id, entry_id, at, data, settled, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (id) DO UPDATE SET at = excluded.at, data = excluded.data,
             settled = excluded.settled, updated_at = excluded.updated_at",
        m.id,
        entry,
        when,
        data,
        settled,
        at_s
    )
    .execute(&mut *conn)
    .await?;
    if m.only_movement {
        // Yalnız hareketten ibaret kayıt AI'a gitmez (ayrıntıyı NotAI kendisi sorar).
        sqlx::query!(
            "UPDATE entries SET ai_state = 'done', ai_error = NULL, ai_next_at = NULL
             WHERE id = ?1 AND ai_state = 'pending'",
            entry
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn save_movement(
    conn: &mut SqliteConnection,
    origin: &str,
    m: &Movement,
    at: DateTime<Utc>,
) -> Result<()> {
    upsert_movement_raw(conn, m, at).await?;
    let payload = serde_json::to_string(m)?;
    log_change(conn, origin, "movement", &m.id, Some(&payload), at).await
}

async fn movement_of(conn: &mut SqliteConnection, entry: EntryId) -> Result<Option<Movement>> {
    let id = entry.to_string();
    let row = sqlx::query_scalar!("SELECT data FROM movements WHERE entry_id = ?1", id)
        .fetch_optional(&mut *conn)
        .await?;
    row.map(|d| from_json("hareket", &d)).transpose()
}

async fn moves_since(conn: &mut SqliteConnection, from: DateTime<Utc>) -> Result<Vec<Movement>> {
    let f = ts(from);
    let rows = sqlx::query_scalar!(
        r#"SELECT m.data FROM movements m JOIN entries e ON e.id = m.entry_id
            WHERE m.at >= ?1 AND e.status != 'trashed' ORDER BY m.at"#,
        f
    )
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(|d| from_json("hareket", d)).collect()
}

/// "Hareketler" ana konusu (yoksa açılır).
async fn root(conn: &mut SqliteConnection, origin: &str, now: DateTime<Utc>) -> Result<Topic> {
    let name = TopicName::new(MOVES_ROOT)?;
    if let Some(t) = find_by_name(conn, None, &name).await? {
        return Ok(t);
    }
    let mut t = Topic::new(name, ParaKind::Area);
    t.icon = Some("pin".to_owned());
    insert_topic(conn, origin, &t, now).await?;
    Ok(t)
}

/// Yer konusu: "Hareketler" altında aynı adlı konu varsa onu yer yapar, yoksa açar.
async fn ensure_place(
    conn: &mut SqliteConnection,
    origin: &str,
    name: &str,
    info: PlaceInfo,
    now: DateTime<Utc>,
) -> Result<Place> {
    let r = root(conn, origin, now).await?;
    let tn = TopicName::new(name)?;
    let topic = match find_by_name(conn, Some(r.id), &tn).await? {
        Some(t) => t,
        None => {
            let mut t = Topic::new(tn, ParaKind::Resource);
            t.parent = Some(r.id);
            insert_topic(conn, origin, &t, now).await?;
            t
        }
    };
    save_place(conn, origin, topic.id, &info, now).await?;
    Ok(Place {
        topic: topic.id,
        name: topic.name.as_str().to_owned(),
        info,
    })
}

/// Yerler; hiç yer yoksa Ev ve İş açılır.
async fn places_or_defaults(
    conn: &mut SqliteConnection,
    origin: &str,
    now: DateTime<Utc>,
) -> Result<Vec<Place>> {
    let ps = load_places(conn).await?;
    if !ps.is_empty() {
        return Ok(ps);
    }
    let mut out = Vec::new();
    for (name, aliases, scope) in DEFAULT_PLACES {
        out.push(
            ensure_place(
                conn,
                origin,
                name,
                PlaceInfo::new(aliases, Some(scope)),
                now,
            )
            .await?,
        );
    }
    Ok(out)
}

async fn resolve(
    conn: &mut SqliteConnection,
    origin: &str,
    places: &mut Vec<Place>,
    r: Option<PlaceRef>,
    now: DateTime<Utc>,
) -> Result<Option<TopicId>> {
    Ok(match r {
        None => None,
        Some(PlaceRef::Known(t)) => Some(t),
        Some(PlaceRef::New(name)) => {
            let p = ensure_place(
                conn,
                origin,
                &name,
                PlaceInfo::new(&[name.as_str()], None),
                now,
            )
            .await?;
            let id = p.topic;
            places.push(p);
            Some(id)
        }
    })
}

/// Varıştan önceki, varışı olmayan son kalkış (aynı yolculuk).
fn prev_depart(recent: &[Movement], m: &Movement) -> Option<Movement> {
    let before: Vec<&Movement> = recent
        .iter()
        .filter(|x| {
            x.at < m.at && x.id != m.id && m.at - x.at <= Duration::hours(mv::MAX_TRIP_HOURS)
        })
        .collect();
    match before.last() {
        Some(x) if x.kind == MoveKind::Depart => Some((*x).clone()),
        _ => None,
    }
}

/// Eksikler için soru açar (bekleyen soru yoksa) ve hareketi "yerleşti" yapar.
async fn settle(
    conn: &mut SqliteConnection,
    origin: &str,
    m: &mut Movement,
    places: &[Place],
    now: DateTime<Utc>,
) -> Result<Option<String>> {
    let recent = moves_since(conn, m.at - Duration::hours(mv::MAX_TRIP_HOURS)).await?;
    let prev = (m.kind == MoveKind::Arrive)
        .then(|| prev_depart(&recent, m))
        .flatten();
    let entry = m.entry.to_string();
    let pending = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "n!: i64" FROM suggestions
            WHERE entry_id = ?1 AND kind = 'question' AND decision = 'pending'"#,
        entry
    )
    .fetch_one(&mut *conn)
    .await?;
    let mut asked = None;
    if pending == 0
        && let Some(q) = mv::question(m, places, prev.as_ref())
    {
        let s = Suggestion::new(
            m.entry,
            SuggestionKind::Question {
                text: QuestionText::new(&q)?,
                answer: None,
            },
            Confidence::new(1.0)?,
        );
        crate::suggestions::insert_suggestion(conn, origin, &s, now).await?;
        asked = Some(q);
    }
    m.settled = true;
    save_movement(conn, origin, m, now).await?;
    Ok(asked)
}

fn place_name(places: &[Place], t: Option<TopicId>) -> Option<&str> {
    t.and_then(|t| places.iter().find(|p| p.topic == t))
        .map(|p| p.name.as_str())
}

fn km(m: u32) -> String {
    if m < 1000 {
        format!("{m} m")
    } else {
        format!("{},{} km", m / 1000, (m % 1000) / 100)
    }
}

impl Store {
    /// Tanımlı yerler (Ev, İş ve sonradan öğrenilenler).
    pub async fn places(&self) -> Result<Vec<Place>> {
        let mut conn = self.pool.acquire().await?;
        load_places(&mut conn).await
    }

    /// Bir konuyu yer yapar ya da yer ayarını günceller (adlar, amaç, konumu sıfırlama).
    pub async fn set_place(
        &self,
        topic: TopicId,
        info: &PlaceInfo,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        get_topic(&mut tx, topic).await?;
        save_place(&mut tx, &self.origin, topic, info, now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Kaydın hareketi (varsa).
    pub async fn movement_for(&self, entry: EntryId) -> Result<Option<Movement>> {
        let mut conn = self.pool.acquire().await?;
        movement_of(&mut conn, entry).await
    }

    /// Yeni kayıttan hareket çıkarır (fiil yoksa hiçbir şey yapmaz). Yer bilinmiyorsa açılır;
    /// konusuz kayıt yerin konusuna yazılır. Soru, konum denemesinden sonra (ya da
    /// `SETTLE_AFTER_SECS` sonra) sorulur.
    pub(crate) async fn detect_movement(
        &self,
        entry: &Entry,
        now: DateTime<Utc>,
    ) -> Result<Option<Movement>> {
        if entry.from_notai || !mv::has_cue(entry.text.as_str()) {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        let mut places = places_or_defaults(&mut tx, &self.origin, now).await?;
        let Some(d) = mv::detect(entry.text.as_str(), &places) else {
            tx.commit().await?;
            return Ok(None);
        };
        let mut place = resolve(&mut tx, &self.origin, &mut places, d.place, now).await?;
        let to = resolve(&mut tx, &self.origin, &mut places, d.to, now).await?;
        if d.kind == MoveKind::Depart && place.is_none() {
            // Nereden: son varışın yeri (arada kalkış yoksa, 24 saat içinde).
            let recent = moves_since(&mut tx, entry.captured_at - Duration::hours(24)).await?;
            if let Some(last) = recent.iter().rev().find(|m| m.at <= entry.captured_at)
                && last.kind == MoveKind::Arrive
            {
                place = last.place;
            }
        }
        let m = Movement {
            id: notai_core::EntryId::new().to_string(),
            entry: entry.id,
            at: entry.captured_at,
            kind: d.kind,
            place,
            to,
            mode: d.mode,
            scope: d.scope,
            fix: None,
            check: Check::Waiting,
            only_movement: d.only_movement,
            settled: false,
        };
        save_movement(&mut tx, &self.origin, &m, now).await?;
        tx.commit().await?;
        if entry.topic().is_none() {
            let target = match m.kind {
                MoveKind::Arrive => m.place,
                MoveKind::Depart => m.place.or(m.to),
            };
            let target = match target {
                Some(t) => t,
                None => {
                    let mut conn = self.pool.acquire().await?;
                    root(&mut conn, &self.origin, now).await?.id
                }
            };
            self.move_entry(entry.id, Some(target), now).await?;
        }
        Ok(Some(m))
    }

    /// Arayüzün gönderdiği konum (ya da "alınamadı"): teyit eder, yerin konumunu öğrenir,
    /// metinde yer yoksa en yakın yeri bulur; ardından eksikler için soruyu açar.
    pub async fn locate_movement(
        &self,
        entry: EntryId,
        fix: Option<Fix>,
        now: DateTime<Utc>,
    ) -> Result<Option<LocateOutcome>> {
        let mut tx = self.pool.begin().await?;
        let Some(mut m) = movement_of(&mut tx, entry).await? else {
            return Ok(None);
        };
        let mut places = load_places(&mut tx).await?;
        let mut need_location = false;
        let message = match fix {
            Some(f) => {
                let (check, matched, learn) = mv::verify(&m, &f, &places);
                m.fix = Some(f);
                m.check = check;
                if let Some(t) = matched {
                    m.place = Some(t);
                }
                if let Some(t) = learn
                    && let Some(p) = places.iter_mut().find(|p| p.topic == t)
                {
                    p.info.fix = Some(f);
                    let info = p.info.clone();
                    save_place(&mut tx, &self.origin, t, &info, now).await?;
                }
                let name = place_name(&places, m.place).unwrap_or("yer");
                match check {
                    Check::Confirmed { distance_m } => {
                        format!("Konumla teyit edildi: {name} ({}).", km(distance_m))
                    }
                    Check::Learned => format!(
                        "{name} konumu öğrenildi; sonraki hareketler konumla teyit edilecek."
                    ),
                    Check::Matched { .. } => format!("Konumundan {name} olduğu anlaşıldı."),
                    Check::Far { distance_m } => format!(
                        "Konumun {name} yerine {} uzak görünüyor; hareket teyitsiz kaydedildi.",
                        km(distance_m)
                    ),
                    Check::NoLocation | Check::Waiting => {
                        "Konum hiçbir yere uymadı; yeri soruyorum.".to_owned()
                    }
                }
            }
            None => {
                m.check = Check::NoLocation;
                need_location = mv::location_essential(&m, &places);
                if need_location {
                    match place_name(&places, m.place) {
                        Some(n) => format!(
                            "Konum kapalı. {n} yerini öğrenmem için bir kez konumu açman yeterli."
                        ),
                        None => "Konum kapalı; nereye geldiğini soruyorum.".to_owned(),
                    }
                } else {
                    String::new()
                }
            }
        };
        let question = if m.settled {
            save_movement(&mut tx, &self.origin, &m, now).await?;
            None
        } else {
            settle(&mut tx, &self.origin, &mut m, &places, now).await?
        };
        tx.commit().await?;
        // Konumla bulunan yer: "Hareketler"e düşmüş kayıt o yere taşınır.
        if let Some(p) = m.place
            && let Some(cur) = self.entry(entry).await?.topic()
            && cur != p
        {
            let t = self.topic(cur).await?;
            if t.parent.is_none() && t.name.as_str() == MOVES_ROOT {
                self.move_entry(entry, Some(p), now).await?;
            }
        }
        Ok(Some(LocateOutcome {
            message,
            need_location,
            question,
        }))
    }

    /// Konum gelmeden bekleyen hareketler için soruları açar. Açılan sorular (kayıt, soru).
    pub async fn settle_movements(&self, now: DateTime<Utc>) -> Result<Vec<(EntryId, String)>> {
        let cutoff = ts(now - Duration::seconds(SETTLE_AFTER_SECS));
        let rows = sqlx::query_scalar!(
            "SELECT data FROM movements WHERE settled = 0 AND at <= ?1 ORDER BY at",
            cutoff
        )
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let mut tx = self.pool.begin().await?;
        let places = load_places(&mut tx).await?;
        let mut asked = Vec::new();
        for d in rows {
            let mut m: Movement = from_json("hareket", &d)?;
            if let Some(q) = settle(&mut tx, &self.origin, &mut m, &places, now).await? {
                asked.push((m.entry, q));
            }
        }
        tx.commit().await?;
        Ok(asked)
    }

    /// Aralıktaki hareketler (çöpteki kayıtlar hariç).
    pub async fn movements_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Movement>> {
        let mut conn = self.pool.acquire().await?;
        Ok(moves_since(&mut conn, from)
            .await?
            .into_iter()
            .filter(|m| m.at < to)
            .collect())
    }

    /// Rapor için giderler: kayıttan girilenler saatli (kayıt zamanı), ekstredekiler günlük.
    /// Ekstreyle eşleşmiş elle giriş bir kez sayılır (saatini elle giriş verir).
    pub async fn spends_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<SpendAt>> {
        let txs = self
            .transactions_between(from.date_naive() - Duration::days(1), to.date_naive())
            .await?;
        let mut times: std::collections::HashMap<EntryId, DateTime<Utc>> =
            std::collections::HashMap::new();
        for t in &txs {
            if let Some(e) = t.entry
                && let Ok(entry) = self.entry(e).await
            {
                times.insert(e, entry.captured_at);
            }
        }
        let twin_time: std::collections::HashMap<notai_core::TransactionId, DateTime<Utc>> = txs
            .iter()
            .filter_map(|t| Some((t.matched_to?, *times.get(&t.entry?)?)))
            .collect();
        let noon = NaiveTime::from_hms_opt(9, 0, 0).unwrap_or(NaiveTime::MIN);
        Ok(txs
            .iter()
            .filter(|t| t.amount.0 < 0 && t.matched_to.is_none())
            .map(|t| {
                let exact = t
                    .entry
                    .and_then(|e| times.get(&e).copied())
                    .or_else(|| twin_time.get(&t.id).copied());
                SpendAt {
                    at: exact.unwrap_or_else(|| t.date.and_time(noon).and_utc()),
                    timed: exact.is_some(),
                    amount: Money(-t.amount.0),
                    scope: t.scope,
                    category: t.category.clone(),
                }
            })
            .filter(|s| s.at >= from && s.at < to)
            .collect())
    }
}

/// Soru cevaplanınca: cevaptan yer/ulaşım/amaç çıkarılır, hareket güncellenir. Hareket yalnız
/// hareketse `true` döner (AI'a yeniden gitmez).
pub(crate) async fn apply_answer(
    conn: &mut SqliteConnection,
    origin: &str,
    entry: EntryId,
    answer: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    let Some(mut m) = movement_of(conn, entry).await? else {
        return Ok(false);
    };
    let mut places = load_places(conn).await?;
    let r = mv::refine(answer, &places);
    // "iş" tek başına: yer sorusu varsa İş yeri, değilse amaç.
    let place_missing = match m.kind {
        MoveKind::Arrive => m.place.is_none(),
        MoveKind::Depart => m.to.is_none() && m.place.is_some(),
    };
    let place_ref = r.place.clone().filter(|_| place_missing).or_else(|| {
        (place_missing && r.scope.is_some() && r.mode.is_none())
            .then(|| {
                mv::refine(&format!("{answer}e"), &places)
                    .place
                    .filter(|p| matches!(p, PlaceRef::Known(_)))
            })
            .flatten()
    });
    let place = resolve(conn, origin, &mut places, place_ref, now).await?;
    match m.kind {
        MoveKind::Arrive => m.place = m.place.or(place),
        MoveKind::Depart => m.to = m.to.or(place),
    }
    m.mode = m.mode.or(r.mode);
    m.scope = m.scope.or(r.scope);
    // Yeni yerin amacı bu cevapla öğrenilir.
    if let (Some(s), Some(t)) = (r.scope, place.or(m.place))
        && let Some(p) = places.iter_mut().find(|p| p.topic == t)
        && p.info.scope.is_none()
    {
        p.info.scope = Some(s);
        let info = p.info.clone();
        save_place(conn, origin, t, &info, now).await?;
    }
    m.settled = true;
    save_movement(conn, origin, &m, now).await?;
    Ok(m.only_movement)
}
