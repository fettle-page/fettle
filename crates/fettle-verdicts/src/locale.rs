//! The languages Fettle speaks.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    En,
    It,
}

impl Locale {
    /// "it-IT" → It; anything Fettle doesn't speak falls back to English.
    pub fn from_tag(tag: &str) -> Locale {
        match tag.chars().take(2).collect::<String>().to_lowercase().as_str() {
            "it" => Locale::It,
            _ => Locale::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::It => "it",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_first_two_letters() {
        assert_eq!(Locale::from_tag("it-IT"), Locale::It);
        assert_eq!(Locale::from_tag("IT"), Locale::It);
        assert_eq!(Locale::from_tag("en-GB"), Locale::En);
        assert_eq!(Locale::from_tag("de-DE"), Locale::En);
        assert_eq!(Locale::from_tag(""), Locale::En);
        assert_eq!(Locale::It.code(), "it");
    }
}
