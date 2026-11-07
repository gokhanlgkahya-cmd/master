//! NotAI alan modeli.
//!
//! Bu crate hiçbir I/O yapmaz: veritabanı, ağ, saat okumaz. Zaman gereken her fonksiyon
//! `now` ve saat dilimini parametre olarak alır; böylece tüm kurallar deterministik test edilir.

#![forbid(unsafe_code)]

pub mod agenda;
pub mod entry;
pub mod error;
pub mod ids;
pub mod journal;
pub mod look;
pub mod morning;
pub mod profile;
pub mod reminders;
pub mod suggestion;
pub mod sync;
pub mod task;
pub mod text;
pub mod topic;
pub mod tr;

pub use entry::{Entry, EntryStatus, Source};
pub use error::CoreError;
pub use ids::{EntryId, ProfileFactId, SuggestionId, TaskId, TopicId};
pub use suggestion::{Confidence, Decision, Effect, Suggestion, SuggestionKind, TopicTarget};
pub use task::{Due, NewTask, Priority, Recurrence, Reminder, Task, TaskState};
pub use text::{EntryText, FactText, Label, TaskTitle, TopicName};
pub use topic::{ParaKind, Topic};

/// Kullanıcının varsayılan saat dilimi (Gökhan: İstanbul).
pub const DEFAULT_TZ: chrono_tz::Tz = chrono_tz::Europe::Istanbul;
