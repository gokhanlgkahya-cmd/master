//! Doğrulanmış metin tipleri. Boş ya da çok uzun metin bu tiplere hiç giremez.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Uzunluk karakter (Unicode scalar) sayısıyla ölçülür; "ğ" tek karakterdir.
fn validate(field: &'static str, raw: &str, max: usize) -> Result<String, CoreError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Empty { field });
    }
    let actual = trimmed.chars().count();
    if actual > max {
        return Err(CoreError::TooLong { field, max, actual });
    }
    Ok(trimmed.to_owned())
}

macro_rules! text_type {
    ($(#[$meta:meta])* $name:ident, $field:literal, $max:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// İzin verilen en uzun karakter sayısı.
            pub const MAX_CHARS: usize = $max;

            /// Baştaki/sondaki boşlukları kırpar ve doğrular.
            pub fn new(raw: impl AsRef<str>) -> Result<Self, CoreError> {
                validate($field, raw.as_ref(), Self::MAX_CHARS).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = CoreError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

text_type!(
    /// Yakalanan kaydın ham metni (en fazla 20 000 karakter).
    EntryText,
    "kayıt metni",
    20_000
);
text_type!(
    /// Konu adı (ör. "GemOlive", "Kostik tedarik").
    TopicName,
    "konu adı",
    80
);
text_type!(
    /// Görev başlığı.
    TaskTitle,
    "görev başlığı",
    200
);
text_type!(
    /// Kullanıcıyı tanıma bilgisi ("GemOlive = sabun markası, Alan: İş").
    FactText,
    "profil bilgisi",
    300
);
text_type!(
    /// AI'ın kullanıcıya sorduğu netleştirme sorusu ("Bu hangi tedarikçi için?").
    QuestionText,
    "AI sorusu",
    200
);
text_type!(
    /// Kullanıcının AI sorusuna verdiği cevap.
    AnswerText,
    "cevap",
    500
);
text_type!(
    /// Etiket, kişi adı gibi kısa etiketler.
    Label,
    "etiket",
    60
);

impl EntryText {
    /// Liste önizlemesi: ilk satır, en fazla `max_chars` karakter, gerekirse "…" ile.
    #[must_use]
    pub fn preview(&self, max_chars: usize) -> String {
        let first_line = self.0.lines().next().unwrap_or_default();
        if first_line.chars().count() <= max_chars {
            return first_line.to_owned();
        }
        let mut out: String = first_line
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect();
        out.truncate(out.trim_end().len());
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_accepts_turkish() {
        let t = EntryText::new("  Kostik tedarikçisini ara – perşembe  ").unwrap();
        assert_eq!(t.as_str(), "Kostik tedarikçisini ara – perşembe");
    }

    #[test]
    fn rejects_blank() {
        assert_eq!(
            EntryText::new(" \n\t ").unwrap_err(),
            CoreError::Empty {
                field: "kayıt metni"
            }
        );
    }

    #[test]
    fn length_is_counted_in_characters_not_bytes() {
        // 80 adet "ğ" = 160 bayt ama 80 karakter: sınırda geçerli.
        let name = "ğ".repeat(80);
        assert!(TopicName::new(&name).is_ok());
        let too_long = "ş".repeat(81);
        assert_eq!(
            TopicName::new(&too_long).unwrap_err(),
            CoreError::TooLong {
                field: "konu adı",
                max: 80,
                actual: 81
            }
        );
    }

    #[test]
    fn serde_validates_on_deserialize() {
        let ok: TaskTitle = serde_json::from_str("\"Logo: stok kartı\"").unwrap();
        assert_eq!(ok.as_str(), "Logo: stok kartı");
        assert!(serde_json::from_str::<TaskTitle>("\"   \"").is_err());
    }

    #[test]
    fn preview_cuts_on_characters_with_ellipsis() {
        let t = EntryText::new("Değer akışı haritalandırması\nikinci satır").unwrap();
        assert_eq!(t.preview(100), "Değer akışı haritalandırması");
        assert_eq!(t.preview(12), "Değer akışı…");
    }
}
