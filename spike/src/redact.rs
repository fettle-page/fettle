//! The kit's privacy pass. Fields that are private by name (serials, product keys, owner and
//! network fields) are hidden here; free text goes through the app's own pass
//! (`fettle_facts::redact::Redactor`), so a dump hides every name a report does: account, full and
//! PC names, the domain, email addresses, profile folders and account SIDs.

use fettle_facts::redact::Redactor;
use serde_json::Value;

/// This PC's names, as the app gathers them.
pub fn this_pc() -> Redactor {
    fettle_collect::names::redactor()
}

/// Product keys: deleted even with --include-private (Fettle never collects them).
const ALWAYS_REMOVED: &[&str] = &[
    "PartialProductKey",
    "ProductKeyID",
    "ProductKeyID2",
    "OA3xOriginalProductKey",
    "DigitalProductId",
    "DigitalProductId4",
    "BackupProductKeyDefault",
];

/// Identifying fields: hidden unless --include-private.
const PRIVATE: &[&str] = &[
    "SerialNumber", "SerialNumberID", "IdentifyingNumber", "UUID", "UniqueId", "ObjectId",
    "CSName", "DNSHostName", "Domain", "Workgroup", "ComputerName", "SystemName", "MachineName",
    "UserName", "PrimaryOwnerName", "RegisteredUser", "RegisteredOwner", "RegisteredOrganization",
    "Organization", "MACAddress", "IPAddress", "PSComputerName", "__SERVER", "__PATH", "__RELPATH",
    "FruId", "UserSID", "ProcessorId", "AssetTag",
];

fn key_in(list: &[&str], key: &str) -> bool {
    list.iter().any(|k| k.eq_ignore_ascii_case(key))
}

/// Any key with "serial" in it is private: drive serials also hide in FruId-style
/// fields, and volume and adapter serials have their own names.
fn is_private_key(key: &str) -> bool {
    key_in(PRIVATE, key) || key.to_ascii_lowercase().contains("serial")
}

pub fn redact(value: &mut Value, names: &Redactor, include_private: bool) {
    match value {
        Value::Object(map) => {
            map.retain(|k, _| !key_in(ALWAYS_REMOVED, k));
            for (k, v) in map.iter_mut() {
                if include_private {
                    redact(v, names, include_private);
                } else if is_private_key(k) {
                    *v = Value::String("<private>".into());
                } else if let (Value::String(s), true) = (&mut *v, k.to_ascii_lowercase().contains("version")) {
                    // A version number can look like an IPv4 address ("2.0.0.0"): names only.
                    *s = names.text(s);
                } else {
                    redact(v, names, include_private);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| redact(v, names, include_private)),
        Value::String(s) if !include_private => *s = names.detail(s),
        _ => {}
    }
}

/// Like `redact`, plus fields that are private only in one source: Win32_OperatingSystem's
/// Description is the description the owner typed for the PC.
pub fn redact_source(source: &str, value: &mut Value, names: &Redactor, include_private: bool) {
    if source == "os" && !include_private {
        let rows: Vec<&mut Value> = match &mut *value {
            Value::Array(rows) => rows.iter_mut().collect(),
            row => vec![row],
        };
        for row in rows {
            if let Some(d) = row.get_mut("Description") {
                *d = Value::String("<private>".into());
            }
        }
    }
    redact(value, names, include_private);
}

/// An error message, which may also quote network addresses and servers.
pub fn redact_str(s: &str, names: &Redactor, include_private: bool) -> String {
    if include_private { s.to_string() } else { names.detail(s) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn names() -> Redactor {
        Redactor::new(["alex", "jdoe", "sam"], "DESKTOP-ABC").with_domain("CONTOSO").with_full_name("Alex Rossi")
    }

    #[test]
    fn serial_like_keys_hidden() {
        let mut v = json!({
            "FriendlyName": "Samsung SSD 9100 PRO 1TB",
            "FruId": "F1AKES2E345678R",
            "AdapterSerialNumber": "F1AKES2E345678R     _0001",
            "VolumeSerialNumber": "ABCD1234",
            "UserSID": "S-1-5-21-1-2-3-1001",
            "BusType": 17
        });
        redact(&mut v, &names(), false);
        assert_eq!(v, json!({
            "FriendlyName": "Samsung SSD 9100 PRO 1TB",
            "FruId": "<private>",
            "AdapterSerialNumber": "<private>",
            "VolumeSerialNumber": "<private>",
            "UserSID": "<private>",
            "BusType": 17
        }));
    }

    #[test]
    fn processor_ids_and_asset_tags_hidden() {
        let mut v = json!([{"Name": "AMD Ryzen 7 5800X", "ProcessorId": "0000000000000000", "AssetTag": "ACME-0001"}]);
        redact(&mut v, &names(), false);
        assert_eq!(v, json!([{"Name": "AMD Ryzen 7 5800X", "ProcessorId": "<private>", "AssetTag": "<private>"}]));
    }

    #[test]
    fn the_computer_description_is_hidden_but_other_descriptions_stay() {
        // Win32_OperatingSystem.Description is the description the owner typed for the PC.
        let mut os = json!([{"Caption": "Microsoft Windows 11 Pro", "Description": "Office PC 3"}]);
        redact_source("os", &mut os, &names(), false);
        assert_eq!(os, json!([{"Caption": "Microsoft Windows 11 Pro", "Description": "<private>"}]));
        let mut gpu = json!([{"Description": "AMD Radeon Graphics"}]);
        redact_source("video_controllers", &mut gpu, &names(), false);
        assert_eq!(gpu, json!([{"Description": "AMD Radeon Graphics"}]));
        let mut kept = json!([{"Description": "Office PC 3"}]);
        redact_source("os", &mut kept, &names(), true);
        assert_eq!(kept, json!([{"Description": "Office PC 3"}]));
    }

    #[test]
    fn account_sids_in_strings_replaced_in_any_case() {
        let n = names();
        assert_eq!(redact_str(r"HKU\S-1-5-21-111111111-2222222222-333333333-1001\Software\Run", &n, false), r"HKU\<sid>\Software\Run");
        assert_eq!(redact_str(r"HKU\S-1-12-1-1111111111-2222222222-3333333333-4444444444\Software\Run", &n, false), r"HKU\<sid>\Software\Run");
        assert_eq!(redact_str("s-1-12-1-1111111111-2222222222-3333333333-4444444444 and s-1-5-21-1-2-3-1001", &n, false), "<sid> and <sid>");
        assert_eq!(redact_str("S-1-5-18 and S-1-5-32-544", &n, false), "S-1-5-18 and S-1-5-32-544");
    }

    #[test]
    fn usbstor_instance_hidden() {
        assert_eq!(
            redact_str(r"USBSTOR\DISK&VEN_SANDISK&PROD_ULTRA&REV_1.00\4C530001230101115&0", &names(), false),
            r"USBSTOR\DISK&VEN_SANDISK&PROD_ULTRA&REV_1.00\<private>"
        );
    }

    #[test]
    fn names_profile_folders_and_the_domain_are_hidden() {
        let n = names();
        assert_eq!(redact_str(r"DESKTOP-ABC\jdoe", &n, false), r"<pc-name>\<user>");
        assert_eq!(redact_str(r"C:\Users\mystery\AppData\x.exe", &n, false), r"C:\Users\<user>\AppData\x.exe");
        assert_eq!(redact_str(r"C:\Users\Public\Desktop", &n, false), r"C:\Users\Public\Desktop");
        // Win32_StartupCommand.User and a hotfix's InstalledBy name the account with its domain.
        let mut v = json!([{"User": r"CONTOSO\jdoe", "InstalledBy": r"CONTOSO\alex", "Command": "rossi.exe for Alex Rossi"}]);
        redact(&mut v, &n, false);
        assert_eq!(v, json!([{"User": r"<domain>\<user>", "InstalledBy": r"<domain>\<user>", "Command": "<user>.exe for <user>"}]));
    }

    #[test]
    fn names_are_hidden_as_whole_words_only() {
        // An account called "sam" mustn't turn "Samsung" into "<user>sung".
        let mut v = json!({"FriendlyName": "Samsung SSD 860 EVO", "Note": "sam's backup"});
        redact(&mut v, &names(), false);
        assert_eq!(v, json!({"FriendlyName": "Samsung SSD 860 EVO", "Note": "<user>'s backup"}));
    }

    #[test]
    fn email_addresses_are_hidden() {
        assert_eq!(redact_str("sign-in failed for mario.rossi@example.com", &names(), false), "sign-in failed for <email>");
    }

    #[test]
    fn data_keeps_version_numbers_that_look_like_addresses() {
        let mut v = json!({"Version": "2.0.0.0", "DriverVersion": "1.2.3.4"});
        redact(&mut v, &names(), false);
        assert_eq!(v, json!({"Version": "2.0.0.0", "DriverVersion": "1.2.3.4"}));
    }

    #[test]
    fn data_hides_network_servers_and_addresses() {
        // Win32_StartupCommand can run a logon script from a server.
        let mut v = json!([{"Command": r"\\fs01\netlogon\map.bat", "Location": "dc at 10.0.0.7"}]);
        redact(&mut v, &names(), false);
        assert_eq!(v, json!([{"Command": r"\\<server>\netlogon\map.bat", "Location": "dc at <ip>"}]));
    }

    #[test]
    fn private_keys_hidden_by_default_and_kept_when_included() {
        let mut v = json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "S123"}]);
        redact(&mut v, &names(), false);
        assert_eq!(v, json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "<private>"}]));
        let mut v = json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "S123", "Note": "jdoe"}]);
        redact(&mut v, &names(), true);
        assert_eq!(v, json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "S123", "Note": "jdoe"}]));
    }

    #[test]
    fn key_matching_is_case_insensitive() {
        let mut v = json!({"serialnumber": "S1"});
        redact(&mut v, &names(), false);
        assert_eq!(v, json!({"serialnumber": "<private>"}));
    }

    #[test]
    fn product_keys_always_removed() {
        let mut v = json!({"OA3xOriginalProductKey": "X", "PartialProductKey": "ABCDE", "Name": "Windows"});
        redact(&mut v, &names(), true);
        assert_eq!(v, json!({"Name": "Windows"}));
    }

    #[test]
    fn a_pc_name_that_starts_with_the_username_is_hidden_whole() {
        // Seen on a laptop: replacing the username first left "<user>ARI12" behind.
        let n = Redactor::new(["mario"], "MARIOARI12");
        assert_eq!(redact_str("MARIOARI12", &n, false), "<pc-name>");
        let n = Redactor::new(["mario"], "MARIO-PC");
        assert_eq!(redact_str(r"MARIO-PC\mario", &n, false), r"<pc-name>\<user>");
    }
}
