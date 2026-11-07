//! Görevler ve sabah planı.

use chrono::{DateTime, NaiveDate, Utc};
use notai_core::TaskId;
use notai_core::morning::{self, MorningPlan};
use notai_core::task::{NewTask, Task};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::codec::{TaskRow, date_str, time_str, ts};
use crate::error::{Result, StoreError};
use crate::{Store, log_change};

/// Görevler sekmesi grupları.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskView {
    Overdue,
    Today,
    Upcoming,
    Undated,
}

fn bind_parts(task: &Task) -> Result<TaskBind> {
    Ok(TaskBind {
        id: task.id.to_string(),
        entry: task.entry.map(|e| e.to_string()),
        topic: task.topic.map(|t| t.to_string()),
        due_date: task.due.map(|d| date_str(d.date)),
        due_time: task.due.and_then(|d| d.time).map(time_str),
        priority: task.priority.name(),
        recurrence: task
            .recurrence
            .map(|r| serde_json::to_string(&r))
            .transpose()?,
        reminder: task
            .reminder
            .map(|r| serde_json::to_string(&r))
            .transpose()?,
        state: task.state().name(),
        closed_at: task.state().closed_at().map(ts),
    })
}

struct TaskBind {
    id: String,
    entry: Option<String>,
    topic: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
    priority: &'static str,
    recurrence: Option<String>,
    reminder: Option<String>,
    state: &'static str,
    closed_at: Option<String>,
}

/// Görevi yazar (günlüğe yazmaz).
pub(crate) async fn insert_task_raw(
    conn: &mut SqliteConnection,
    task: &Task,
    now: DateTime<Utc>,
) -> Result<()> {
    let b = bind_parts(task)?;
    let title = task.title.as_str();
    let now_s = ts(now);
    sqlx::query!(
        "INSERT INTO tasks (id, entry_id, title, topic_id, due_date, due_time, priority, recurrence,
                            reminder, state, closed_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        b.id,
        b.entry,
        title,
        b.topic,
        b.due_date,
        b.due_time,
        b.priority,
        b.recurrence,
        b.reminder,
        b.state,
        b.closed_at,
        now_s
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub(crate) async fn insert_task(
    conn: &mut SqliteConnection,
    origin: &str,
    task: &Task,
    now: DateTime<Utc>,
) -> Result<()> {
    insert_task_raw(conn, task, now).await?;
    let payload = serde_json::to_string(task)?;
    log_change(
        conn,
        origin,
        "task",
        &task.id.to_string(),
        Some(&payload),
        now,
    )
    .await
}

/// Görevi günceller (günlüğe yazmaz).
pub(crate) async fn update_task_raw(
    conn: &mut SqliteConnection,
    task: &Task,
    now: DateTime<Utc>,
) -> Result<()> {
    let b = bind_parts(task)?;
    let title = task.title.as_str();
    let now_s = ts(now);
    let res = sqlx::query!(
        "UPDATE tasks SET title = ?2, topic_id = ?3, due_date = ?4, due_time = ?5, priority = ?6,
                recurrence = ?7, reminder = ?8, state = ?9, closed_at = ?10, updated_at = ?11
         WHERE id = ?1",
        b.id,
        title,
        b.topic,
        b.due_date,
        b.due_time,
        b.priority,
        b.recurrence,
        b.reminder,
        b.state,
        b.closed_at,
        now_s
    )
    .execute(&mut *conn)
    .await?;
    if res.rows_affected() == 0 {
        return Err(StoreError::not_found("görev", &b.id));
    }
    Ok(())
}

async fn update_task(
    conn: &mut SqliteConnection,
    origin: &str,
    task: &Task,
    now: DateTime<Utc>,
) -> Result<()> {
    update_task_raw(conn, task, now).await?;
    let payload = serde_json::to_string(task)?;
    log_change(
        conn,
        origin,
        "task",
        &task.id.to_string(),
        Some(&payload),
        now,
    )
    .await
}

async fn load_task(conn: &mut SqliteConnection, id: TaskId) -> Result<Task> {
    let id_s = id.to_string();
    let row = sqlx::query_as!(
        TaskRow,
        "SELECT id, entry_id, title, topic_id, due_date, due_time, priority, recurrence, reminder,
                state, closed_at
         FROM tasks WHERE id = ?1",
        id_s
    )
    .fetch_optional(&mut *conn)
    .await?;
    row.map(Task::try_from)
        .transpose()?
        .ok_or_else(|| StoreError::not_found("görev", id))
}

/// Görevi (açıksa) iptal eder; çağıranın transaction'ı içinde.
pub(crate) async fn cancel_in_tx(
    conn: &mut SqliteConnection,
    origin: &str,
    id: TaskId,
    now: DateTime<Utc>,
) -> Result<()> {
    let mut task = load_task(conn, id).await?;
    if task.is_open() {
        task.cancel(now)?;
        update_task(conn, origin, &task, now).await?;
    }
    Ok(())
}

impl Store {
    pub async fn create_task(&self, new: NewTask, now: DateTime<Utc>) -> Result<Task> {
        let task = Task::create(new)?;
        let mut tx = self.pool.begin().await?;
        if let Some(topic) = task.topic {
            crate::topics::get_topic(&mut tx, topic).await?;
        }
        insert_task(&mut tx, &self.origin, &task, now).await?;
        tx.commit().await?;
        Ok(task)
    }

    pub async fn task(&self, id: TaskId) -> Result<Task> {
        let mut conn = self.pool.acquire().await?;
        load_task(&mut conn, id).await
    }

    /// Görevin başlığını/tarihini/önceliğini/tekrarını/hatırlatmasını günceller.
    pub async fn save_task(&self, task: &Task, now: DateTime<Utc>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        update_task(&mut tx, &self.origin, task, now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Tüm açık görevler (tarih sırasıyla, tarihsizler sonda).
    pub async fn open_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query_as!(
            TaskRow,
            "SELECT id, entry_id, title, topic_id, due_date, due_time, priority, recurrence, reminder,
                    state, closed_at
             FROM tasks WHERE state = 'open'
             ORDER BY due_date IS NULL, due_date, due_time IS NULL, due_time, created_at"
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Task::try_from).collect()
    }

    /// Görevler sekmesi için bir grup.
    pub async fn tasks(&self, view: TaskView, today: NaiveDate) -> Result<Vec<Task>> {
        let open = self.open_tasks().await?;
        Ok(open
            .into_iter()
            .filter(|t| match view {
                TaskView::Overdue => t.is_overdue(today),
                TaskView::Today => t.is_due_on(today),
                TaskView::Upcoming => t.due.is_some_and(|d| d.date > today),
                TaskView::Undated => t.due.is_none(),
            })
            .collect())
    }

    /// Görevi tamamlar; tekrarlıysa bir sonraki örneği aynı transaction'da oluşturur.
    pub async fn complete_task(
        &self,
        id: TaskId,
        now: DateTime<Utc>,
    ) -> Result<(Task, Option<Task>)> {
        let mut tx = self.pool.begin().await?;
        let mut task = load_task(&mut tx, id).await?;
        let next = task.complete(now)?;
        update_task(&mut tx, &self.origin, &task, now).await?;
        if let Some(n) = &next {
            insert_task(&mut tx, &self.origin, n, now).await?;
        }
        tx.commit().await?;
        Ok((task, next))
    }

    /// Görevi yeniden açar ("geri al").
    pub async fn reopen_task(&self, id: TaskId, now: DateTime<Utc>) -> Result<Task> {
        let mut tx = self.pool.begin().await?;
        let mut task = load_task(&mut tx, id).await?;
        task.reopen();
        update_task(&mut tx, &self.origin, &task, now).await?;
        tx.commit().await?;
        Ok(task)
    }

    pub async fn cancel_task(&self, id: TaskId, now: DateTime<Utc>) -> Result<Task> {
        let mut tx = self.pool.begin().await?;
        let mut task = load_task(&mut tx, id).await?;
        task.cancel(now)?;
        update_task(&mut tx, &self.origin, &task, now).await?;
        tx.commit().await?;
        Ok(task)
    }

    /// Bugünün sabah planı.
    pub async fn morning_plan(&self, today: NaiveDate) -> Result<MorningPlan> {
        let open = self.open_tasks().await?;
        Ok(morning::build(&open, today))
    }
}
