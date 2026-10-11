//! Shortens a report's text to the limits its schema sets (`limits`), the same ones the share
//! server checks on upload, so a real report is never refused for a long name.

use crate::Report;
use serde_json::Value;

/// The largest integer a JSON number carries exactly (2^53 - 1), and so the largest the upload
/// check accepts for a 64-bit field.
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

/// Cuts every string in `report` to its schema's maxLength, counted in UTF-16 units like the
/// upload check, and always on a character boundary. A 64-bit number too big for the upload check
/// becomes unknown.
pub fn fit_limits(report: &mut Report) {
    let schema = serde_json::to_value(schemars::schema_for!(Report)).unwrap_or(Value::Null);
    let Ok(mut value) = serde_json::to_value(&*report) else { return };
    walk(&mut value, &schema, &schema);
    if let Ok(fitted) = serde_json::from_value(value) {
        *report = fitted;
    }
}

/// Follows `schema` down `value`, cutting each string that has a maxLength.
fn walk(value: &mut Value, schema: &Value, root: &Value) {
    let schema = resolve(schema, root);
    if let Some(all) = schema.get("allOf").and_then(Value::as_array) {
        for part in all {
            walk(value, part, root);
        }
    }
    if let Some(branches) = schema.get("anyOf").or_else(|| schema.get("oneOf")).and_then(Value::as_array) {
        if let Some(branch) = branches.iter().map(|b| resolve(b, root)).find(|b| fits_type(value, b)) {
            walk(value, branch, root);
        }
        return;
    }
    let wide = matches!(schema.get("format").and_then(Value::as_str), Some("uint64" | "int64"));
    if wide && value.as_u64().map_or(value.as_i64().is_some_and(|n| n.unsigned_abs() > MAX_SAFE_INTEGER), |n| n > MAX_SAFE_INTEGER) {
        *value = Value::Null;
        return;
    }
    match value {
        Value::String(s) => {
            if let Some(max) = schema.get("maxLength").and_then(Value::as_u64) {
                cut(s, max as usize);
            }
        }
        Value::Object(map) => {
            if let Some(props) = schema.get("properties").and_then(Value::as_object) {
                for (key, v) in map.iter_mut() {
                    if let Some(prop) = props.get(key) {
                        walk(v, prop, root);
                    }
                }
            }
        }
        Value::Array(items) => {
            if let Some(item) = schema.get("items") {
                for v in items {
                    walk(v, item, root);
                }
            }
        }
        _ => {}
    }
}

/// A "$ref": "#/$defs/Name" points into the root schema's definitions.
fn resolve<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    match schema.get("$ref").and_then(Value::as_str).and_then(|r| r.strip_prefix('#')) {
        Some(pointer) => root.pointer(pointer).unwrap_or(schema),
        None => schema,
    }
}

/// Whether a branch of an anyOf describes this kind of value (a branch without a type might).
fn fits_type(value: &Value, schema: &Value) -> bool {
    let kind = match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    match schema.get("type") {
        Some(Value::String(t)) => t == kind || (t == "number" && kind == "integer"),
        Some(Value::Array(ts)) => ts.iter().any(|t| t == kind || (t == "number" && kind == "integer")),
        _ => kind != "null" || schema.get("const") == Some(&Value::Null),
    }
}

/// Shortens `s` to at most `max` UTF-16 units, ending on a whole character.
fn cut(s: &mut String, max: usize) {
    let mut units = 0;
    for (at, c) in s.char_indices() {
        units += c.len_utf16();
        if units > max {
            s.truncate(at);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits;
    use serde_json::Value;

    fn sample() -> Report {
        let case = concat!(env!("CARGO_MANIFEST_DIR"), "/../fettle-receipt/tests/parity/cases/dump-dev-workstation-user-1791321847.json");
        let case: Value = serde_json::from_str(&std::fs::read_to_string(case).unwrap()).unwrap();
        serde_json::from_value(case["report"].clone()).unwrap()
    }

    fn utf16(s: &str) -> usize {
        s.encode_utf16().count()
    }

    // A JSON number is exact only up to 2^53 - 1: a garbage counter above that (from a USB bridge,
    // say) would get the whole report refused on upload, so it goes as unknown instead.
    #[test]
    fn a_number_too_big_to_share_becomes_unknown() {
        let mut report = sample();
        let disk = &mut report.facts.storage.as_mut().unwrap().data.as_mut().unwrap().disks[0];
        disk.size_bytes = Some(1 << 60);
        disk.smart = Some(crate::SmartSummary { power_on_hours: Some((1 << 53) - 1), ..Default::default() });
        fit_limits(&mut report);
        let disk = &report.facts.storage.as_ref().unwrap().data.as_ref().unwrap().disks[0];
        assert_eq!(disk.size_bytes, None);
        assert_eq!(disk.smart.as_ref().unwrap().power_on_hours, Some((1 << 53) - 1));
    }

    #[test]
    fn a_real_report_is_left_as_it_is() {
        let mut report = sample();
        let before = report.clone();
        fit_limits(&mut report);
        assert_eq!(report, before);
    }

    #[test]
    fn long_text_is_cut_to_each_field_s_limit_on_a_character_boundary() {
        let mut report = sample();
        // Emoji are two UTF-16 units each: counting characters would let them through.
        let long = "🙂".repeat(300);
        report.app_version = long.clone();
        let storage = report.facts.storage.as_mut().unwrap();
        storage.check.detail = Some(long.clone());
        let disk = &mut storage.data.as_mut().unwrap().disks[0];
        disk.model = Some(format!("a{long}"));
        let startup = report.facts.startup.as_mut().unwrap().data.as_mut().unwrap();
        startup.items[0].name = Some(long.clone());

        fit_limits(&mut report);

        assert!(utf16(&report.app_version) <= limits::SHORT && utf16(&report.app_version) >= limits::SHORT - 1);
        let storage = report.facts.storage.as_ref().unwrap();
        assert!(utf16(storage.check.detail.as_deref().unwrap()) <= limits::DETAIL);
        let model = storage.data.as_ref().unwrap().disks[0].model.clone().unwrap();
        assert!(utf16(&model) <= limits::NAME, "{}", utf16(&model));
        assert!(model.starts_with("a🙂") && model.chars().skip(1).all(|c| c == '🙂'));
        let name = report.facts.startup.as_ref().unwrap().data.as_ref().unwrap().items[0].name.clone().unwrap();
        assert_eq!(utf16(&name), limits::NAME);
    }

    #[test]
    fn short_enough_text_is_untouched_even_with_wide_characters() {
        let mut report = sample();
        let exact = "é".repeat(limits::SHORT);
        report.app_version = exact.clone();
        fit_limits(&mut report);
        assert_eq!(report.app_version, exact);
    }
}

#[cfg(test)]
mod every_field {
    use super::*;
    use serde_json::Value;

    /// Sets every string the schema limits to 300 emoji, and counts them.
    fn fill(value: &mut Value, schema: &Value, root: &Value, count: &mut usize) {
        let schema = resolve(schema, root);
        if let Some(branches) = schema.get("anyOf").or_else(|| schema.get("oneOf")).and_then(Value::as_array) {
            if let Some(b) = branches.iter().map(|b| resolve(b, root)).find(|b| fits_type(value, b)) {
                fill(value, b, root, count);
            }
            return;
        }
        match value {
            Value::String(s) if schema.get("maxLength").is_some() => {
                *s = "🙂".repeat(300);
                *count += 1;
            }
            Value::Object(map) => {
                for (k, v) in map.iter_mut() {
                    if let Some(p) = schema.get("properties").and_then(|p| p.get(k)) {
                        fill(v, p, root, count);
                    }
                }
            }
            Value::Array(items) => {
                for v in items {
                    if let Some(i) = schema.get("items") {
                        fill(v, i, root, count);
                    }
                }
            }
            _ => {}
        }
    }

    // Writes target/fitcheck/fitted.json for the cross-check against the website's upload schema.
    #[test]
    #[ignore]
    fn write_a_report_with_every_limited_field_overlong_then_fitted() {
        let case = concat!(env!("CARGO_MANIFEST_DIR"), "/../fettle-receipt/tests/parity/cases/dump-dev-workstation-admin-1791321978.json");
        let case: Value = serde_json::from_str(&std::fs::read_to_string(case).unwrap()).unwrap();
        let schema = serde_json::to_value(schemars::schema_for!(Report)).unwrap();
        let mut value = case["report"].clone();
        let mut count = 0;
        fill(&mut value, &schema, &schema, &mut count);
        let mut report: Report = serde_json::from_value(value).unwrap();
        fit_limits(&mut report);
        let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/fitcheck");
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(format!("{out}/fitted.json"), serde_json::to_string(&report).unwrap()).unwrap();
        eprintln!("filled {count} limited strings");
    }
}
