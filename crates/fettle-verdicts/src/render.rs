//! Renders a message in a language: the catalog's template with its `{param}` slots filled.

use crate::{Locale, MessageRef, Param, fmt};
use std::collections::HashMap;
use std::sync::LazyLock;

static EN: LazyLock<HashMap<String, String>> = LazyLock::new(|| serde_json::from_str(include_str!("../copy/en.json")).expect("copy/en.json"));
static IT: LazyLock<HashMap<String, String>> = LazyLock::new(|| serde_json::from_str(include_str!("../copy/it.json")).expect("copy/it.json"));

/// The template for a key: the locale's, else English's, else the key itself.
pub fn template(key: &str, locale: Locale) -> &str {
    let catalog: &'static HashMap<String, String> = if locale == Locale::It { &IT } else { &EN };
    catalog.get(key).or_else(|| EN.get(key)).map(String::as_str).unwrap_or(key)
}

/// Fills `{name}` slots (`name` as JavaScript's `\w+`) from `lookup`; unknown slots stay as written.
pub fn fill(template: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let len = after.bytes().take_while(|c| c.is_ascii_alphanumeric() || *c == b'_').count();
        if len > 0 && after.as_bytes().get(len) == Some(&b'}') {
            match lookup(&after[..len]) {
                Some(value) => out.push_str(&value),
                None => out.push_str(&rest[open..open + len + 2]),
            }
            rest = &after[len + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Fills `{name}` slots; YYYY-MM-DD string params become long local dates. Unknown slots stay.
pub fn render(m: &MessageRef, locale: Locale) -> String {
    fill(template(&m.key, locale), |name| m.params.as_ref()?.get(name).map(|p| param_text(p, locale)))
}

/// One param as TypeScript prints it in a template; a day that doesn't exist is shown as written.
pub(crate) fn param_text(p: &Param, locale: Locale) -> String {
    match p {
        Param::Str(s) if fmt::looks_like_day(s) => fmt::date_long(s, locale),
        Param::Str(s) => s.clone(),
        Param::Int(n) => n.to_string(),
        Param::Float(n) => fmt::js_number(*n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_params_in_the_language() {
        let m = MessageRef::with("battery.fine", &[("pct", Param::Int(87))]);
        assert!(render(&m, Locale::En).contains("87"));
        assert_ne!(render(&m, Locale::En), render(&m, Locale::It));
    }

    #[test]
    fn day_params_become_long_dates() {
        let m = MessageRef::with("windows.supported", &[("product", "Windows 11 Pro".into()), ("until", "2028-10-09".into())]);
        assert!(render(&m, Locale::En).contains("9 October 2028"), "{}", render(&m, Locale::En));
        assert!(render(&m, Locale::It).contains("9 ottobre 2028"));
    }

    #[test]
    fn unknown_keys_and_slots_stay_visible() {
        assert_eq!(render(&MessageRef::key("no.such.key"), Locale::It), "no.such.key");
        assert!(render(&MessageRef::key("battery.fine"), Locale::En).contains("{pct}"));
    }
}
