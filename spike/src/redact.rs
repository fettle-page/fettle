use serde_json::Value;
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

/// The current machine's names, which must not leak into dumps through free text.
pub struct RedactContext {
    pub usernames: Vec<String>,
    pub hostname: String,
}

impl RedactContext {
    /// Collects every account name on this PC, not just the one running the tool:
    /// an admin run under IT credentials still sees the employee's startup entries.
    pub fn from_env() -> RedactContext {
        let mut usernames: Vec<String> = Vec::new();
        let mut add = |name: String| {
            if !name.is_empty() && !usernames.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                usernames.push(name);
            }
        };
        add(std::env::var("USERNAME").unwrap_or_default());
        if let Some(leaf) = std::env::var("USERPROFILE").ok().and_then(|p| profile_leaf(&p)) {
            add(leaf);
        }
        let profiles = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(PROFILE_LIST);
        for sid in profiles.iter().flat_map(|k| k.enum_keys().flatten()) {
            let path: Option<String> = profiles.as_ref().ok().and_then(|k| k.open_subkey(&sid).ok()).and_then(|k| k.get_value("ProfileImagePath").ok());
            if let Some(leaf) = path.and_then(|p| profile_leaf(&p)) {
                add(leaf);
            }
        }
        RedactContext { usernames, hostname: std::env::var("COMPUTERNAME").unwrap_or_default() }
    }
}

const PROFILE_LIST: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList";

/// The account folder name of a personal profile path, or None for system and shared profiles.
fn profile_leaf(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('\\');
    let (parent, leaf) = trimmed.rsplit_once('\\')?;
    let is_users_dir = parent.rsplit('\\').next().is_some_and(|p| p.eq_ignore_ascii_case("Users"));
    (is_users_dir && !leaf.is_empty() && !key_in(SHARED_PROFILES, leaf)).then(|| leaf.to_string())
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
    "FruId", "UserSID",
];

/// Profile folders that belong to no one in particular.
const SHARED_PROFILES: &[&str] = &["Public", "Default", "Default User", "All Users", "<user>"];

/// Names shorter than this are too likely to match unrelated text.
const MIN_NAME_LEN: usize = 3;

fn key_in(list: &[&str], key: &str) -> bool {
    list.iter().any(|k| k.eq_ignore_ascii_case(key))
}

/// Any key with "serial" in it is private: drive serials also hide in FruId-style
/// fields, and volume and adapter serials have their own names.
fn is_private_key(key: &str) -> bool {
    key_in(PRIVATE, key) || key.to_ascii_lowercase().contains("serial")
}

pub fn redact(value: &mut Value, ctx: &RedactContext, include_private: bool) {
    match value {
        Value::Object(map) => {
            map.retain(|k, _| !key_in(ALWAYS_REMOVED, k));
            for (k, v) in map.iter_mut() {
                if !include_private && is_private_key(k) {
                    *v = Value::String("<private>".into());
                } else {
                    redact(v, ctx, include_private);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| redact(v, ctx, include_private)),
        Value::String(s) => *s = redact_str(s, ctx, include_private),
        _ => {}
    }
}

pub fn redact_str(s: &str, ctx: &RedactContext, include_private: bool) -> String {
    if include_private {
        return s.to_string();
    }
    let mut s = replace_account_sids(s);
    s = hide_usbstor_instance(&s);
    s = replace_profile_folders(&s);
    // The PC name first: it often starts with the username ("MARIOARI12"), and replacing the
    // username first would leave the rest of the PC name behind.
    s = replace_ignore_case(&s, &ctx.hostname, "<pc-name>");
    for name in &ctx.usernames {
        s = replace_ignore_case(&s, name, "<user>");
    }
    s
}

/// Local and domain account SIDs (S-1-5-21-…) identify this PC and its users.
/// Well-known SIDs like S-1-5-18 are left alone.
fn replace_account_sids(s: &str) -> String {
    const PREFIX: &str = "S-1-5-21-";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find(PREFIX) {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos + PREFIX.len()..];
        let len = tail.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(tail.len());
        out.push_str("<sid>");
        rest = &tail[len..];
    }
    out.push_str(rest);
    out
}

/// USB disk device IDs end with the device's serial: USBSTOR\<model>\<serial>&0.
fn hide_usbstor_instance(s: &str) -> String {
    let is_usbstor = s.get(..8).is_some_and(|p| p.eq_ignore_ascii_case(r"USBSTOR\"));
    match s.rfind('\\') {
        Some(pos) if is_usbstor && pos > 7 => format!("{}<private>", &s[..=pos]),
        _ => s.to_string(),
    }
}

/// Replaces the folder name after any `\Users\`, so profiles of other or renamed
/// accounts don't leak even when their name isn't known.
fn replace_profile_folders(s: &str) -> String {
    const MARKER: &str = r"\users\";
    // ASCII lowercasing keeps byte offsets identical to `s`.
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while let Some(found) = lower[i..].find(MARKER) {
        let seg_start = i + found + MARKER.len();
        let seg_len = s[seg_start..].find(['\\', '"', '\'']).unwrap_or(s.len() - seg_start);
        let segment = &s[seg_start..seg_start + seg_len];
        out.push_str(&s[i..seg_start]);
        if segment.is_empty() || key_in(SHARED_PROFILES, segment) {
            out.push_str(segment);
        } else {
            out.push_str("<user>");
        }
        i = seg_start + seg_len;
    }
    out.push_str(&s[i..]);
    out
}

fn replace_ignore_case(s: &str, needle: &str, with: &str) -> String {
    if needle.chars().count() < MIN_NAME_LEN {
        return s.to_string();
    }
    let (hay, pat) = (s.as_bytes(), needle.as_bytes());
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let end = i + pat.len();
        if end <= s.len() && s.is_char_boundary(end) && hay[i..end].eq_ignore_ascii_case(pat) {
            out.push_str(with);
            i = end;
        } else {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> RedactContext {
        RedactContext { usernames: vec!["alex".into(), "jdoe".into()], hostname: "DESKTOP-ABC".into() }
    }

    #[test]
    fn profile_leaf_keeps_only_personal_profiles() {
        assert_eq!(profile_leaf(r"C:\Users\jdoe"), Some("jdoe".into()));
        assert_eq!(profile_leaf(r"C:\Users\jdoe.CONTOSO\"), Some("jdoe.CONTOSO".into()));
        assert_eq!(profile_leaf(r"C:\WINDOWS\system32\config\systemprofile"), None);
        assert_eq!(profile_leaf(r"C:\Users\Public"), None);
        assert_eq!(profile_leaf(""), None);
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
        redact(&mut v, &ctx(), false);
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
    fn account_sids_in_strings_replaced() {
        let c = ctx();
        assert_eq!(
            redact_str(r"HKU\S-1-5-21-111111111-2222222222-333333333-1001\Software\Run", &c, false),
            r"HKU\<sid>\Software\Run"
        );
        assert_eq!(redact_str("S-1-5-18", &c, false), "S-1-5-18");
    }

    #[test]
    fn usbstor_instance_hidden() {
        assert_eq!(
            redact_str(r"USBSTOR\DISK&VEN_SANDISK&PROD_ULTRA&REV_1.00\4C530001230101115&0", &ctx(), false),
            r"USBSTOR\DISK&VEN_SANDISK&PROD_ULTRA&REV_1.00\<private>"
        );
    }

    #[test]
    fn every_profile_name_replaced() {
        assert_eq!(redact_str(r"DESKTOP-ABC\jdoe", &ctx(), false), r"<pc-name>\<user>");
    }

    #[test]
    fn unknown_profile_folder_replaced() {
        let c = ctx();
        assert_eq!(redact_str(r"C:\Users\mystery\AppData\x.exe", &c, false), r"C:\Users\<user>\AppData\x.exe");
        assert_eq!(redact_str(r"C:\Users\Public\Desktop", &c, false), r"C:\Users\Public\Desktop");
    }

    #[test]
    fn private_keys_hidden_by_default() {
        let mut v = json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "S123"}]);
        redact(&mut v, &ctx(), false);
        assert_eq!(v, json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "<private>"}]));
    }

    #[test]
    fn private_keys_kept_when_included() {
        let mut v = json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "S123"}]);
        redact(&mut v, &ctx(), true);
        assert_eq!(v, json!([{"Name": "AMD Ryzen 7 5800X", "SerialNumber": "S123"}]));
    }

    #[test]
    fn key_matching_is_case_insensitive() {
        let mut v = json!({"serialnumber": "S1"});
        redact(&mut v, &ctx(), false);
        assert_eq!(v, json!({"serialnumber": "<private>"}));
    }

    #[test]
    fn product_keys_always_removed() {
        let mut v = json!({"OA3xOriginalProductKey": "X", "PartialProductKey": "ABCDE", "Name": "Windows"});
        redact(&mut v, &ctx(), true);
        assert_eq!(v, json!({"Name": "Windows"}));
    }

    #[test]
    fn username_in_paths_replaced() {
        let mut v = json!({"command": r"C:\Users\Alex\AppData\Local\app.exe --tray"});
        redact(&mut v, &ctx(), false);
        assert_eq!(v, json!({"command": r"C:\Users\<user>\AppData\Local\app.exe --tray"}));
    }

    #[test]
    fn hostname_in_nested_strings_replaced() {
        let mut v = json!({"a": [{"Caption": "desktop-abc"}]});
        redact(&mut v, &ctx(), false);
        assert_eq!(v, json!({"a": [{"Caption": "<pc-name>"}]}));
    }

    #[test]
    fn short_names_not_substituted() {
        let c = RedactContext { usernames: vec!["al".into()], hostname: "PC".into() };
        assert_eq!(redact_str("Realtek PCIe", &c, false), "Realtek PCIe");
    }

    #[test]
    fn a_pc_name_that_starts_with_the_username_is_hidden_whole() {
        // Seen on a laptop: replacing the username first left "<user>ARI12" behind.
        let c = RedactContext { usernames: vec!["mario".into()], hostname: "MARIOARI12".into() };
        assert_eq!(redact_str("MARIOARI12", &c, false), "<pc-name>");
        let c = RedactContext { usernames: vec!["mario".into()], hostname: "MARIO-PC".into() };
        assert_eq!(redact_str(r"MARIO-PC\mario", &c, false), r"<pc-name>\<user>");
    }

    #[test]
    fn redact_str_cleans_error_text() {
        assert_eq!(
            redact_str(r"open C:\Users\alex\x failed", &ctx(), false),
            r"open C:\Users\<user>\x failed"
        );
    }
}
