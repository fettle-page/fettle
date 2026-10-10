//! The receipt's own strings (section names, detail labels, tags): copy/{en,it}.json, the source
//! since the TypeScript copy was retired.

use fettle_verdicts::{Locale, Param, fill, fmt};
use std::collections::HashMap;
use std::sync::LazyLock;

static EN: LazyLock<HashMap<String, String>> = LazyLock::new(|| serde_json::from_str(include_str!("../copy/en.json")).expect("copy/en.json"));
static IT: LazyLock<HashMap<String, String>> = LazyLock::new(|| serde_json::from_str(include_str!("../copy/it.json")).expect("copy/it.json"));

/// A string from `catalog`, else English (as verdicts do), else the key itself: a missing
/// translation shows English rather than a raw key.
fn lookup<'a>(key: &'a str, catalog: &'a HashMap<String, String>, english: &'a HashMap<String, String>) -> &'a str {
    catalog.get(key).or_else(|| english.get(key)).map(String::as_str).unwrap_or(key)
}

/// One receipt string in the language, with `{param}` slots filled (numbers as `String(n)`).
pub fn ui(key: &str, locale: Locale, params: &[(&str, Param)]) -> String {
    let catalog: &HashMap<String, String> = if locale == Locale::It { &IT } else { &EN };
    let template = lookup(key, catalog, &EN);
    fill(template, |name| {
        params.iter().find(|(n, _)| *n == name).map(|(_, p)| match p {
            Param::Str(s) => s.clone(),
            Param::Int(n) => n.to_string(),
            Param::Float(n) => fmt::js_number(*n),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_come_in_both_languages() {
        assert_eq!(ui("d.board", Locale::En, &[]), "Board");
        assert_eq!(ui("d.board", Locale::It, &[]), "Scheda madre");
    }

    #[test]
    fn a_string_missing_in_italian_falls_back_to_english_then_the_key() {
        let en: HashMap<String, String> = [("d.new".to_string(), "New".to_string())].into();
        let it: HashMap<String, String> = HashMap::new();
        assert_eq!(lookup("d.new", &it, &en), "New");
        assert_eq!(lookup("d.unknown", &it, &en), "d.unknown");
    }

    #[test]
    fn params_fill_as_plain_values() {
        assert_eq!(ui("d.slots_used", Locale::En, &[("used", Param::Int(2)), ("total", Param::Int(4))]), "2 of 4 used");
        // Unlike verdict messages, day-shaped params stay as written.
        assert!(ui("d.drive", Locale::En, &[("letter", "2026-10-07".into())]).contains("2026-10-07"));
    }
}
