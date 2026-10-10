use serde_json::{Map, Value, json};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RegType};
use winreg::types::FromRegValue;
use winreg::{HKEY, RegKey, RegValue};

const CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
/// Named values only: this key also holds DigitalProductId, so it is never enumerated.
const VERSION_VALUES: &[&str] = &[
    "ProductName",
    "EditionID",
    "CompositionEditionID",
    "DisplayVersion",
    "ReleaseId",
    "CurrentBuild",
    "UBR",
    "InstallationType",
];
const SEARCH_MAX_DEPTH: usize = 6;
const SEARCH_MAX_KEYS: usize = 200_000;

/// Registry value as JSON: strings and numbers as-is, anything else as hex.
pub fn reg_value_to_json(v: &RegValue) -> Value {
    match v.vtype {
        RegType::REG_SZ | RegType::REG_EXPAND_SZ => String::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        RegType::REG_MULTI_SZ => Vec::<String>::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        RegType::REG_DWORD => u32::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        RegType::REG_QWORD => u64::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        _ => Value::from(v.bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()),
    }
}

pub fn windows_registry() -> Result<Value, String> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(CURRENT_VERSION)
        .map_err(|e| format!("open {CURRENT_VERSION}: {e}"))?;
    let mut out = Map::new();
    for &name in VERSION_VALUES {
        let v = key.get_raw_value(name).map(|v| reg_value_to_json(&v)).unwrap_or(Value::Null);
        out.insert(name.into(), v);
    }
    Ok(Value::Object(out))
}

pub fn secure_boot() -> Result<Value, String> {
    let path = r"SYSTEM\CurrentControlSet\Control\SecureBoot\State";
    let key = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path).map_err(|e| format!("open {path}: {e}"))?;
    let enabled: u32 = key.get_value("UEFISecureBootEnabled").map_err(|e| format!("UEFISecureBootEnabled: {e}"))?;
    Ok(json!({"uefi_secure_boot_enabled": enabled}))
}

/// True for names that look ESU-related, without matching words like "Result".
pub fn is_esu_name(s: &str) -> bool {
    s.contains("ESU") || s.to_ascii_lowercase().contains("extendedsecurity")
}

/// Subtrees the search never enters: the licensing key holds product-key data
/// (BackupProductKeyDefault), so its values are never even read.
pub fn is_skipped_subkey(name: &str) -> bool {
    name.eq_ignore_ascii_case("SoftwareProtectionPlatform")
}

/// Walks a few registry trees looking for anything ESU-named. The spike compares
/// an enrolled Windows 10 machine against a Windows 11 baseline.
pub fn esu_registry_search() -> Result<Value, String> {
    let roots: [(HKEY, &str, &str, bool); 4] = [
        (HKEY_LOCAL_MACHINE, "HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion", true),
        (HKEY_LOCAL_MACHINE, "HKLM", r"SOFTWARE\Microsoft\WindowsUpdate", true),
        // Subkeys only: this key's own values include product-key data.
        (HKEY_LOCAL_MACHINE, "HKLM", CURRENT_VERSION, false),
        (HKEY_CURRENT_USER, "HKCU", r"Software\Microsoft\Windows\CurrentVersion", true),
    ];
    let mut matches = Vec::new();
    let mut searched = Vec::new();
    for (hive, hive_name, path, read_root_values) in roots {
        let root = format!(r"{hive_name}\{path}");
        let Ok(key) = RegKey::predef(hive).open_subkey(path) else {
            searched.push(json!({"root": root, "opened": false}));
            continue;
        };
        // The cap is per root, so one huge tree can't starve the others.
        let mut visited = 0usize;
        walk(&key, &root, 0, read_root_values, &mut visited, &mut matches);
        searched.push(json!({"root": root, "opened": true, "keys_visited": visited, "capped": visited >= SEARCH_MAX_KEYS}));
    }
    Ok(json!({"matches": matches, "searched": searched}))
}

fn walk(key: &RegKey, path: &str, depth: usize, read_values: bool, visited: &mut usize, matches: &mut Vec<Value>) {
    if *visited >= SEARCH_MAX_KEYS {
        return;
    }
    *visited += 1;
    if read_values {
        for (name, value) in key.enum_values().flatten() {
            if is_esu_name(&name) {
                matches.push(json!({"path": path, "value_name": name, "value": reg_value_to_json(&value)}));
            }
        }
    }
    if depth >= SEARCH_MAX_DEPTH {
        return;
    }
    for name in key.enum_keys().flatten() {
        if is_skipped_subkey(&name) {
            continue;
        }
        let child_path = format!(r"{path}\{name}");
        if is_esu_name(&name) {
            matches.push(json!({"path": child_path}));
        }
        if let Ok(child) = key.open_subkey(&name) {
            walk(&child, &child_path, depth + 1, true, visited, matches);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn licensing_subtree_is_skipped() {
        assert!(is_skipped_subkey("SoftwareProtectionPlatform"));
        assert!(is_skipped_subkey("softwareprotectionplatform"));
        assert!(!is_skipped_subkey("WindowsUpdate"));
    }

    #[test]
    fn esu_name_matching() {
        assert!(is_esu_name("ConsumerESUStatus"));
        assert!(is_esu_name("ESUEnrollment"));
        assert!(is_esu_name("ExtendedSecurityUpdates"));
        assert!(!is_esu_name("Result"));
        assert!(!is_esu_name("resume"));
    }
}
