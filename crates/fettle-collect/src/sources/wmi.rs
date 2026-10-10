use serde_json::{Map, Value, json};
use std::collections::HashMap;
use wmi::{Variant, WMIConnection};

/// WMI's "invalid query", which is also what a column this Windows doesn't have gives.
const INVALID_QUERY: &str = "0x80041017";

/// Runs a WQL query and returns every row as JSON. Opens its own connection, because each source
/// runs on its own thread. If an older Windows lacks one of the named columns, WMI rejects the
/// whole query; then it asks for every column once and keeps only the named ones, so one missing
/// column costs that column, not the source.
pub fn wmi_query(namespace: &str, wql: &str) -> Result<Value, String> {
    let con = WMIConnection::with_namespace_path(namespace).map_err(|e| format!("WMI {namespace}: {e}"))?;
    let rows: Vec<HashMap<String, Variant>> = match con.raw_query(wql) {
        Ok(rows) => rows,
        Err(e) => {
            let error = format!("WMI {namespace}: {e}");
            let Some((everything, keep)) = all_columns(wql).filter(|_| error.contains(INVALID_QUERY)) else { return Err(error) };
            let mut rows: Vec<HashMap<String, Variant>> = con.raw_query(&everything).map_err(|_| error)?;
            rows.iter_mut().for_each(|row| row.retain(|k, _| keep.iter().any(|c| c.eq_ignore_ascii_case(k))));
            rows
        }
    };
    let rows = rows
        .iter()
        .map(|row| {
            let obj: Map<String, Value> = row.iter().map(|(k, v)| (k.clone(), variant_to_json(v))).collect();
            Value::Object(obj)
        })
        .collect();
    Ok(Value::Array(rows))
}

/// "SELECT a, b FROM rest" → ("SELECT * FROM rest", [a, b]); None for a query that already asks for everything.
fn all_columns(wql: &str) -> Option<(String, Vec<String>)> {
    let (list, rest) = wql.strip_prefix("SELECT ")?.split_once(" FROM ")?;
    let columns: Vec<String> = list.split(',').map(|c| c.trim().to_string()).collect();
    (!columns.iter().any(|c| c == "*")).then(|| (format!("SELECT * FROM {rest}"), columns))
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
    fn a_query_can_fall_back_to_all_columns() {
        assert_eq!(
            all_columns("SELECT Name, NumberOfCores FROM Win32_Processor"),
            Some(("SELECT * FROM Win32_Processor".to_string(), vec!["Name".to_string(), "NumberOfCores".to_string()]))
        );
        assert_eq!(
            all_columns("SELECT DeviceID, Size FROM Win32_LogicalDisk WHERE DriveType = 3").unwrap().0,
            "SELECT * FROM Win32_LogicalDisk WHERE DriveType = 3"
        );
        assert_eq!(all_columns("SELECT * FROM Win32_Processor"), None);
    }

    /// An older Windows may lack a column we name; WMI then rejects the whole query. The fallback
    /// asks for everything and keeps only the named columns (the unknown one is simply absent).
    #[test]
    fn a_missing_column_doesnt_lose_the_whole_source() {
        let rows = wmi_query(r"ROOT\CIMV2", "SELECT Name, NoSuchColumnInWindows FROM Win32_Processor").unwrap();
        let row = rows.as_array().unwrap()[0].as_object().unwrap();
        assert!(row.get("Name").and_then(Value::as_str).is_some_and(|n| !n.is_empty()));
        assert!(row.keys().all(|k| k == "Name" || k.starts_with("__")), "{:?}", row.keys().collect::<Vec<_>>());
    }

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
    fn variant_arrays_recurse() {
        assert_eq!(
            variant_to_json(&Variant::Array(vec![Variant::UI1(65), Variant::UI1(66)])),
            json!([65, 66])
        );
    }
}
