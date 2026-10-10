use serde_json::{Map, Value, json};
use std::collections::HashMap;
use wmi::{Variant, WMIConnection};

/// Runs a WQL query and returns every row with every property, as JSON.
/// Opens its own connection, because each collector runs on its own thread.
pub fn wmi_query(namespace: &str, wql: &str) -> Result<Value, String> {
    let con = WMIConnection::with_namespace_path(namespace).map_err(|e| format!("WMI {namespace}: {e}"))?;
    let rows: Vec<HashMap<String, Variant>> =
        con.raw_query(wql).map_err(|e| format!("WMI {namespace}: {e}"))?;
    let rows = rows
        .iter()
        .map(|row| {
            let obj: Map<String, Value> = row.iter().map(|(k, v)| (k.clone(), variant_to_json(v))).collect();
            Value::Object(obj)
        })
        .collect();
    Ok(Value::Array(rows))
}

/// Escapes a value for use inside a double-quoted WQL string literal.
pub fn wql_escape(s: &str) -> String {
    s.replace('\\', r"\\").replace('"', r#"\""#)
}

pub fn variant_to_json(v: &Variant) -> Value {
    match v {
        Variant::Empty | Variant::Null => Value::Null,
        Variant::String(s) => json!(s),
        Variant::I1(n) => json!(n),
        Variant::I2(n) => json!(n),
        Variant::I4(n) => json!(n),
        Variant::I8(n) => json!(n),
        Variant::R4(n) => json!(n),
        Variant::R8(n) => json!(n),
        Variant::Bool(b) => json!(b),
        Variant::UI1(n) => json!(n),
        Variant::UI2(n) => json!(n),
        Variant::UI4(n) => json!(n),
        Variant::UI8(n) => json!(n),
        Variant::Array(items) => Value::Array(items.iter().map(variant_to_json).collect()),
        Variant::Unknown(_) => json!("<unknown>"),
        Variant::Object(_) => json!("<object>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_strings_numbers_bools() {
        assert_eq!(variant_to_json(&Variant::String("x".into())), json!("x"));
        assert_eq!(variant_to_json(&Variant::UI8(u64::MAX)), json!(u64::MAX));
        assert_eq!(variant_to_json(&Variant::I4(-5)), json!(-5));
        assert_eq!(variant_to_json(&Variant::Bool(true)), json!(true));
        assert_eq!(variant_to_json(&Variant::Null), json!(null));
        assert_eq!(variant_to_json(&Variant::Empty), json!(null));
    }

    #[test]
    fn wql_escape_backslash_and_quote() {
        assert_eq!(wql_escape(r#"a\b"c"#), r#"a\\b\"c"#);
    }

    #[test]
    fn variant_arrays_recurse() {
        assert_eq!(
            variant_to_json(&Variant::Array(vec![Variant::UI1(65), Variant::UI1(66)])),
            json!([65, 66])
        );
    }
}
