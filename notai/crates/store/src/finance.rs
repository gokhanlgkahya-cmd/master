//! Harcamalar: hesap hareketleri, ekstre içe aktarma (mükerrersiz), satıcı kuralları (öğrenme).

use chrono::{DateTime, NaiveDate, Utc};
use notai_core::TransactionId;
use notai_core::finance::{
    MerchantRule, Money, Scope, StatementLine, Transaction, TxSource, categorize, dedupe_keys,
    merchant_key, parse_statement, same_payment,
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::codec::{date_str, parse_date, parse_id, ts};
use crate::error::{Result, StoreError};
use crate::{Store, log_change};

/// PDF ekstrenin metnini çıkarır (yerel; hiçbir yere gönderilmez). Metin içermeyen (taranmış)
/// PDF'te boş dizgi döner: ekstre AI/ajan işi olarak bekler.
pub fn pdf_text(bytes: &[u8]) -> Result<String> {
    pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| StoreError::Conflict(format!("PDF okunamadı: {e}")))
}

/// Ekstre içe aktarma sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    /// Ekstreden okunan satır.
    pub parsed: u32,
    /// Yeni eklenen hareket.
    pub imported: u32,
    /// Daha önce yüklenmiş (mükerrer) satır.
    pub skipped: u32,
    /// Elle girilmiş bir ödemeyle eşleşen satır (raporda bir kez sayılır).
    pub matched: u32,
}

/// AI/ajan için bekleyen ekstre (kurallarla ayrıştırılamadı).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingStatement {
    pub id: String,
    pub account: String,
    pub file_name: String,
    pub text: String,
}

struct TxRow {
    id: String,
    date: String,
    amount: i64,
    description: String,
    category: String,
    scope: String,
    source: String,
    account: Option<String>,
    topic_id: Option<String>,
    entry_id: Option<String>,
    dedupe_key: Option<String>,
    matched_to: Option<String>,
}

impl TryFrom<TxRow> for Transaction {
    type Error = StoreError;
    fn try_from(r: TxRow) -> Result<Self> {
        Ok(Self {
            id: parse_id("hareket", &r.id)?,
            date: parse_date("hareket", &r.date)?,
            amount: Money(r.amount),
            description: r.description,
            category: r.category,
            scope: Scope::parse(&r.scope)?,
            source: if r.source == "statement" {
                TxSource::Statement
            } else {
                TxSource::Manual
            },
            account: r.account,
            topic: r
                .topic_id
                .as_deref()
                .map(|t| parse_id("hareket", t))
                .transpose()?,
            entry: r
                .entry_id
                .as_deref()
                .map(|e| parse_id("hareket", e))
                .transpose()?,
            dedupe_key: r.dedupe_key,
            matched_to: r
                .matched_to
                .as_deref()
                .map(|m| parse_id("hareket", m))
                .transpose()?,
        })
    }
}

/// Hareketi yazar (yoksa ekler, varsa günceller; günlüğe yazmaz).
pub(crate) async fn upsert_transaction_raw(
    conn: &mut SqliteConnection,
    t: &Transaction,
    at: DateTime<Utc>,
) -> Result<()> {
    let id = t.id.to_string();
    let date = date_str(t.date);
    let scope = t.scope.name();
    let source = t.source.name();
    let topic = t.topic.map(|x| x.to_string());
    let entry = t.entry.map(|x| x.to_string());
    let matched = t.matched_to.map(|x| x.to_string());
    let at_s = ts(at);
    sqlx::query!(
        "INSERT INTO transactions (id, date, amount, description, category, scope, source, account,
                                   topic_id, entry_id, dedupe_key, matched_to, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
         ON CONFLICT (id) DO UPDATE SET date = excluded.date, amount = excluded.amount,
             description = excluded.description, category = excluded.category, scope = excluded.scope,
             account = excluded.account, topic_id = excluded.topic_id, entry_id = excluded.entry_id,
             matched_to = excluded.matched_to, updated_at = excluded.updated_at",
        id,
        date,
        t.amount.0,
        t.description,
        t.category,
        scope,
        source,
        t.account,
        topic,
        entry,
        t.dedupe_key,
        matched,
        at_s
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub(crate) async fn save_transaction(
    conn: &mut SqliteConnection,
    origin: &str,
    t: &Transaction,
    at: DateTime<Utc>,
) -> Result<()> {
    upsert_transaction_raw(conn, t, at).await?;
    let payload = serde_json::to_string(t)?;
    log_change(
        conn,
        origin,
        "transaction",
        &t.id.to_string(),
        Some(&payload),
        at,
    )
    .await
}

pub(crate) async fn upsert_rule_raw(
    conn: &mut SqliteConnection,
    r: &MerchantRule,
    at: DateTime<Utc>,
) -> Result<()> {
    let scope = r.scope.name();
    let at_s = ts(at);
    sqlx::query!(
        "INSERT INTO merchant_rules (merchant, category, scope, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (merchant) DO UPDATE SET category = excluded.category, scope = excluded.scope,
             updated_at = excluded.updated_at",
        r.merchant,
        r.category,
        scope,
        at_s
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn load_tx(conn: &mut SqliteConnection, id: &str) -> Result<Transaction> {
    let row = sqlx::query_as!(
        TxRow,
        "SELECT id, date, amount, description, category, scope, source, account, topic_id, entry_id,
                dedupe_key, matched_to FROM transactions WHERE id = ?1",
        id
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| StoreError::NotFound {
        what: "hareket",
        id: id.to_owned(),
    })?;
    Transaction::try_from(row)
}

impl Store {
    /// Kullanıcının öğrettiği satıcı kuralları.
    pub async fn merchant_rules(&self) -> Result<Vec<MerchantRule>> {
        let rows = sqlx::query!("SELECT merchant, category, scope FROM merchant_rules")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(MerchantRule {
                    merchant: r.merchant,
                    category: r.category,
                    scope: Scope::parse(&r.scope)?,
                })
            })
            .collect()
    }

    /// Hareketi kaydeder (elle giriş; ör. "Ayhan'a 1.850 ₺ ödendi" kaydından).
    pub async fn add_transaction(&self, t: &Transaction, now: DateTime<Utc>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        save_transaction(&mut tx, &self.origin, t, now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Tarih aralığındaki hareketler (en yeni önce).
    pub async fn transactions_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<Transaction>> {
        let (f, t) = (date_str(from), date_str(to));
        let rows = sqlx::query_as!(
            TxRow,
            "SELECT id, date, amount, description, category, scope, source, account, topic_id, entry_id,
                    dedupe_key, matched_to
             FROM transactions WHERE date >= ?1 AND date <= ?2 ORDER BY date DESC, created_at DESC",
            f,
            t
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Transaction::try_from).collect()
    }

    /// Ekstre metnini içe aktarır: kurallarla ayrıştırır, kategori/kapsam verir, mükerrerleri
    /// atlar, elle girilmiş eş ödemeleri eşleştirir. Hiç satır okunamazsa ekstre AI/ajan işi
    /// olarak bekler (`parsed = 0`).
    pub async fn import_statement(
        &self,
        account: &str,
        file_name: &str,
        text: &str,
        now: DateTime<Utc>,
    ) -> Result<(String, ImportReport)> {
        let lines = parse_statement(text);
        let id = TransactionId::new().to_string();
        let learned = self.merchant_rules().await?;
        let mut tx = self.pool.begin().await?;
        let state = if lines.is_empty() { "pending" } else { "done" };
        let now_s = ts(now);
        sqlx::query!(
            "INSERT INTO statements (id, account, file_name, text, state, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            id,
            account,
            file_name,
            text,
            state,
            now_s
        )
        .execute(&mut *tx)
        .await?;
        let report = self
            .import_lines_in(&mut tx, &id, account, &lines, &learned, now)
            .await?;
        tx.commit().await?;
        Ok((id, report))
    }

    async fn import_lines_in(
        &self,
        tx: &mut SqliteConnection,
        statement: &str,
        account: &str,
        lines: &[StatementLine],
        learned: &[MerchantRule],
        now: DateTime<Utc>,
    ) -> Result<ImportReport> {
        let keys = dedupe_keys(account, lines);
        let mut report = ImportReport {
            parsed: u32::try_from(lines.len()).unwrap_or(u32::MAX),
            imported: 0,
            skipped: 0,
            matched: 0,
        };
        for (line, key) in lines.iter().zip(keys) {
            let dup = sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!: i64" FROM transactions WHERE dedupe_key = ?1"#,
                key
            )
            .fetch_one(&mut *tx)
            .await?;
            if dup > 0 {
                report.skipped += 1;
                continue;
            }
            let (category, scope) = categorize(&line.description, line.amount, learned);
            let t = Transaction {
                id: TransactionId::new(),
                date: line.date,
                amount: line.amount,
                description: line.description.clone(),
                category,
                scope,
                source: TxSource::Statement,
                account: Some(account.to_owned()),
                topic: None,
                entry: None,
                dedupe_key: Some(key),
                matched_to: None,
            };
            save_transaction(tx, &self.origin, &t, now).await?;
            report.imported += 1;
            // Elle girilmiş eş ödeme: eşleştir (raporda ekstredeki sayılır). Elle girilenin
            // kategori/kapsamı (kullanıcının niyeti) ekstre satırına taşınır.
            let lo = date_str(line.date - chrono::Duration::days(3));
            let hi = date_str(line.date + chrono::Duration::days(3));
            let candidates = sqlx::query_as!(
                TxRow,
                "SELECT id, date, amount, description, category, scope, source, account, topic_id,
                        entry_id, dedupe_key, matched_to
                 FROM transactions
                 WHERE source = 'manual' AND matched_to IS NULL AND amount = ?1
                   AND date >= ?2 AND date <= ?3
                 ORDER BY date LIMIT 1",
                line.amount.0,
                lo,
                hi
            )
            .fetch_all(&mut *tx)
            .await?;
            if let Some(row) = candidates.into_iter().next() {
                let mut manual = Transaction::try_from(row)?;
                if same_payment(manual.date, manual.amount, line.date, line.amount) {
                    manual.matched_to = Some(t.id);
                    save_transaction(tx, &self.origin, &manual, now).await?;
                    let mut bank = t.clone();
                    bank.category.clone_from(&manual.category);
                    bank.scope = manual.scope;
                    bank.topic = manual.topic;
                    bank.entry = manual.entry;
                    save_transaction(tx, &self.origin, &bank, now).await?;
                    report.matched += 1;
                }
            }
        }
        let (i, s, m) = (
            i64::from(report.imported),
            i64::from(report.skipped),
            i64::from(report.matched),
        );
        sqlx::query!(
            "UPDATE statements SET imported = imported + ?2, skipped = skipped + ?3, matched = matched + ?4
             WHERE id = ?1",
            statement,
            i,
            s,
            m
        )
        .execute(&mut *tx)
        .await?;
        Ok(report)
    }

    /// Kurallarla ayrıştırılamayıp AI/ajan bekleyen ekstreler.
    pub async fn pending_statements(&self) -> Result<Vec<PendingStatement>> {
        let rows = sqlx::query_as!(
            PendingStatement,
            "SELECT id, account, file_name, text FROM statements WHERE state = 'pending' ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// AI/ajanın ayrıştırdığı satırları içe aktarır (aynı mükerrer/eşleştirme kurallarıyla).
    pub async fn answer_statement(
        &self,
        id: &str,
        lines: &[StatementLine],
        now: DateTime<Utc>,
    ) -> Result<ImportReport> {
        let learned = self.merchant_rules().await?;
        let mut tx = self.pool.begin().await?;
        let account = sqlx::query_scalar!(
            "SELECT account FROM statements WHERE id = ?1 AND state = 'pending'",
            id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| StoreError::NotFound {
            what: "bekleyen ekstre",
            id: id.to_owned(),
        })?;
        let report = self
            .import_lines_in(&mut tx, id, &account, lines, &learned, now)
            .await?;
        sqlx::query!("UPDATE statements SET state = 'done' WHERE id = ?1", id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(report)
    }

    /// Hareketin kategori/kapsamını düzeltir ve **öğrenir**: aynı satıcının diğer ekstre
    /// hareketleri de güncellenir, sonraki ekstrelerde bu kural önce uygulanır.
    pub async fn classify_transaction(
        &self,
        id: &str,
        category: &str,
        scope: Scope,
        now: DateTime<Utc>,
    ) -> Result<u32> {
        let mut tx = self.pool.begin().await?;
        let t = load_tx(&mut tx, id).await?;
        let merchant = merchant_key(&t.description);
        let rule = MerchantRule {
            merchant: merchant.clone(),
            category: category.to_owned(),
            scope,
        };
        upsert_rule_raw(&mut tx, &rule, now).await?;
        let payload = serde_json::to_string(&rule)?;
        log_change(
            &mut tx,
            &self.origin,
            "merchant_rule",
            &merchant,
            Some(&payload),
            now,
        )
        .await?;
        let rows = sqlx::query_as!(
            TxRow,
            "SELECT id, date, amount, description, category, scope, source, account, topic_id, entry_id,
                    dedupe_key, matched_to FROM transactions"
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut changed = 0;
        for r in rows {
            let mut other = Transaction::try_from(r)?;
            let same = other.id == t.id
                || (other.source == TxSource::Statement
                    && merchant_key(&other.description) == merchant);
            if same && (other.category != category || other.scope != scope) {
                other.category = category.to_owned();
                other.scope = scope;
                save_transaction(&mut tx, &self.origin, &other, now).await?;
                changed += 1;
            }
        }
        tx.commit().await?;
        Ok(changed)
    }
}
