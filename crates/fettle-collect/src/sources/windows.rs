use serde_json::{Map, Value, json};
use winreg::enums::{HKEY_LOCAL_MACHINE, RegType};
use winreg::types::FromRegValue;
use winreg::{RegKey, RegValue};

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
