//! The PC's name and serial numbers, for the app's "show them on this screen" switch only.
//!
//! This is deliberately separate from the scan: nothing here goes into a `Report`, so it can never
//! be saved, copied or shared. It is read only when the app asks, and product keys are never read.

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrivateDetails {
    pub pc_name: Option<String>,
    pub items: Vec<PrivateItem>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrivateItem {
    pub kind: PrivateKind,
    /// For a disk, its name ("Samsung SSD 9100 PRO 1TB").
    pub label: Option<String>,
    pub serial: String,
}

#[derive(Serialize, Debug, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PrivateKind {
    System,
    Board,
    Disk,
}

/// What firmware writes when nobody filled the serial in. Compared without case.
const PLACEHOLDERS: &[&str] = &["to be filled by o.e.m.", "default string", "system serial number", "none", "n/a", "not specified"];

/// A real serial: trimmed, not blank, not all zeros, not a placeholder.
fn serial(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?.trim();
    let fake = s.is_empty() || s.chars().all(|c| c == '0') || PLACEHOLDERS.contains(&s.to_ascii_lowercase().as_str());
    (!fake).then(|| s.to_string())
}

fn rows(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or_default()
}

/// Builds the details from the four query results (each a JSON array of rows).
pub fn from_rows(computer: &Value, product: &Value, board: &Value, disks: &Value) -> PrivateDetails {
    let first = |v: &Value, key: &str| rows(v).iter().find_map(|r| serial(r.get(key)));
    let mut items = Vec::new();
    if let Some(s) = first(product, "IdentifyingNumber") {
        items.push(PrivateItem { kind: PrivateKind::System, label: None, serial: s });
    }
    if let Some(s) = first(board, "SerialNumber") {
        items.push(PrivateItem { kind: PrivateKind::Board, label: None, serial: s });
    }
    for d in rows(disks) {
        if let Some(s) = serial(d.get("SerialNumber")) {
            let label = d.get("FriendlyName").and_then(Value::as_str).map(str::trim).filter(|l| !l.is_empty()).map(str::to_string);
            items.push(PrivateItem { kind: PrivateKind::Disk, label, serial: s });
        }
    }
    PrivateDetails { pc_name: first(computer, "Name"), items }
}

/// Reads the PC name and the serials now. Each query names its columns; a failed one is skipped.
#[cfg(windows)]
pub fn read() -> PrivateDetails {
    use crate::sources::wmi_query;
    const CIMV2: &str = r"ROOT\CIMV2";
    const STORAGE: &str = r"ROOT\Microsoft\Windows\Storage";
    let q = |ns: &str, wql: &str| wmi_query(ns, wql).unwrap_or(Value::Null);
    from_rows(
        &q(CIMV2, "SELECT Name FROM Win32_ComputerSystem"),
        &q(CIMV2, "SELECT IdentifyingNumber FROM Win32_ComputerSystemProduct"),
        &q(CIMV2, "SELECT SerialNumber FROM Win32_BaseBoard"),
        &q(STORAGE, "SELECT FriendlyName, SerialNumber FROM MSFT_PhysicalDisk"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_name_and_each_real_serial() {
        let got = from_rows(
            &json!([{ "Name": "DESKTOP-ABC123" }]),
            &json!([{ "IdentifyingNumber": " XYZ3PF12 " }]),
            &json!([{ "SerialNumber": "TES12345" }]),
            &json!([{ "FriendlyName": "Samsung SSD 9100 PRO 1TB", "SerialNumber": " T1ESTS2 " }, { "FriendlyName": "CT1000MX500SSD1", "SerialNumber": "1234T56E" }]),
        );
        assert_eq!(got.pc_name.as_deref(), Some("DESKTOP-ABC123"));
        assert_eq!(
            got.items,
            vec![
                PrivateItem { kind: PrivateKind::System, label: None, serial: "XYZ3PF12".into() },
                PrivateItem { kind: PrivateKind::Board, label: None, serial: "TES12345".into() },
                PrivateItem { kind: PrivateKind::Disk, label: Some("Samsung SSD 9100 PRO 1TB".into()), serial: "T1ESTS2".into() },
                PrivateItem { kind: PrivateKind::Disk, label: Some("CT1000MX500SSD1".into()), serial: "1234T56E".into() },
            ]
        );
    }

    #[test]
    fn placeholders_and_blanks_are_not_serials() {
        for fake in ["To Be Filled By O.E.M.", "Default string", "System Serial Number", "None", "", "   ", "0000000000", "0", "to be filled by o.e.m."] {
            let got = from_rows(&json!([]), &json!([{ "IdentifyingNumber": fake }]), &json!([{ "SerialNumber": fake }]), &json!([{ "FriendlyName": "Disk", "SerialNumber": fake }]));
            assert!(got.items.is_empty(), "{fake:?} was kept");
        }
    }

    #[test]
    fn missing_sources_give_nothing_rather_than_failing() {
        assert_eq!(from_rows(&Value::Null, &json!({}), &json!("x"), &json!([{ "SerialNumber": 5 }])), PrivateDetails::default());
    }

    #[test]
    fn it_serialises_for_the_page_in_camel_case() {
        let d = PrivateDetails { pc_name: Some("PC".into()), items: vec![PrivateItem { kind: PrivateKind::Disk, label: Some("D".into()), serial: "S".into() }] };
        assert_eq!(serde_json::to_value(d).unwrap(), json!({ "pcName": "PC", "items": [{ "kind": "disk", "label": "D", "serial": "S" }] }));
    }
}
