//! Kullanıcıyı tanıma katmanı: "GemOlive = sabun markası, Alan: İş".
//!
//! AI bir bilgi önerebilir ama yalnız kullanıcının onayladığı bilgiler istemlere girer.

use serde::{Deserialize, Serialize};

use crate::ids::{ProfileFactId, TopicId};
use crate::text::FactText;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Proposed,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileFact {
    pub id: ProfileFactId,
    pub text: FactText,
    pub topic: Option<TopicId>,
    pub status: FactStatus,
}

impl ProfileFact {
    /// Kullanıcının kendi girdiği bilgi doğrudan onaylıdır.
    #[must_use]
    pub fn confirmed(text: FactText, topic: Option<TopicId>) -> Self {
        Self {
            id: ProfileFactId::new(),
            text,
            topic,
            status: FactStatus::Confirmed,
        }
    }

    /// AI'ın önerdiği bilgi onay bekler.
    #[must_use]
    pub fn proposed(text: FactText, topic: Option<TopicId>) -> Self {
        Self {
            status: FactStatus::Proposed,
            ..Self::confirmed(text, topic)
        }
    }

    pub fn confirm(&mut self) {
        self.status = FactStatus::Confirmed;
    }
}

/// İstemlere girecek bilgiler: yalnız onaylılar.
pub fn for_prompt(facts: &[ProfileFact]) -> impl Iterator<Item = &ProfileFact> {
    facts.iter().filter(|f| f.status == FactStatus::Confirmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_confirmed_facts_reach_prompt() {
        let a = ProfileFact::confirmed(
            FactText::new("GemOlive = sabun markası, Alan: İş").unwrap(),
            None,
        );
        let mut b = ProfileFact::proposed(
            FactText::new("Ayhan Kimya = kostik tedarikçisi").unwrap(),
            None,
        );
        assert_eq!(for_prompt(&[a.clone(), b.clone()]).count(), 1);
        b.confirm();
        assert_eq!(for_prompt(&[a, b]).count(), 2);
    }
}
