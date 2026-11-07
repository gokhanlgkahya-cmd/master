//! Newtype kimlikler. Bir `EntryId` yanlışlıkla `TaskId` yerine verilemez.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CoreError;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Zaman sıralı (UUID v7) yeni kimlik üretir.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Var olan bir UUID'yi sarar.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = CoreError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s)
                    .map(Self)
                    .map_err(|_| CoreError::InvalidId(s.to_owned()))
            }
        }
    };
}

id_type!(
    /// Yakalanan bir kaydın kimliği.
    EntryId
);
id_type!(
    /// Bir konunun (PARA: Proje/Alan/Kaynak/Arşiv) kimliği.
    TopicId
);
id_type!(
    /// Bir görevin kimliği.
    TaskId
);
id_type!(
    /// AI önerisinin kimliği.
    SuggestionId
);
id_type!(
    /// Kullanıcıyı tanıma katmanındaki bir bilginin kimliği.
    ProfileFactId
);
id_type!(
    /// Bir hesap hareketinin (harcama/gelir) kimliği.
    TransactionId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_through_strings() {
        let id = EntryId::new();
        let parsed: EntryId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn invalid_id_is_rejected() {
        let err = "kostik".parse::<TaskId>().unwrap_err();
        assert_eq!(err, CoreError::InvalidId("kostik".into()));
    }

    #[test]
    fn new_ids_are_time_ordered() {
        let a = TopicId::new();
        let b = TopicId::new();
        assert!(a <= b);
    }

    #[test]
    fn serde_is_transparent() {
        let id = SuggestionId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
    }
}
