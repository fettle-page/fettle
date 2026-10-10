//! The app's own strings in English and Italian (copy/*.json; the receipt's strings come from
//! fettle-receipt). Every new key goes into both files.

use fettle_verdicts::{Locale, Param, fill, fmt};
use std::collections::HashMap;
use std::sync::LazyLock;

static EN: LazyLock<HashMap<String, String>> = LazyLock::new(|| serde_json::from_str(include_str!("../copy/en.json")).expect("copy/en.json"));
static IT: LazyLock<HashMap<String, String>> = LazyLock::new(|| serde_json::from_str(include_str!("../copy/it.json")).expect("copy/it.json"));

/// An app string with `{name}` slots filled; English when the Italian is missing, else the key.
pub fn t(key: &str, lang: Locale, params: &[(&str, Param)]) -> String {
    let catalog: &HashMap<String, String> = if lang == Locale::It { &IT } else { &EN };
    let template = catalog.get(key).or_else(|| EN.get(key)).map(String::as_str).unwrap_or(key);
    fill(template, |name| {
        params.iter().find(|(n, _)| *n == name).map(|(_, p)| match p {
            Param::Str(s) => s.clone(),
            Param::Int(n) => n.to_string(),
            Param::Float(n) => fmt::js_number(*n),
        })
    })
}

/// For the Slint side, which asks by key alone: `ui.<key>` is a receipt string, anything else an app string.
pub fn any(key: &str, lang: Locale) -> String {
    match key.strip_prefix("ui.") {
        Some(receipt_key) => fettle_receipt::ui(receipt_key, lang, &[]),
        None => t(key, lang, &[]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_string_exists_in_both_languages() {
        let missing: Vec<_> = EN.keys().filter(|k| !IT.contains_key(*k)).chain(IT.keys().filter(|k| !EN.contains_key(*k))).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }

    #[test]
    fn app_and_receipt_strings_come_through_one_door() {
        assert_eq!(any("home.button", Locale::It), "Controlla questo PC");
        assert_eq!(any("ui.d.board", Locale::It), "Scheda madre");
        assert_eq!(t("status.quick_time", Locale::En, &[("s", Param::Float(0.7))]), "Quick check · 0.7 s");
    }
}
