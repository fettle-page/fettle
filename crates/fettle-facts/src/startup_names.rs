//! Startup item names, cleaned before a report is built.
//!
//! Some apps register their startup entry with a long hex code that can derive from the user's
//! profile (Edge: `MicrosoftEdgeAutoLaunch_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`). A shared report must
//! not carry that fingerprint, so the code is dropped and well-known names are made readable.

/// Long enough that ordinary names ("Build_2026", "App_DEADBEEF") are never mistaken for a code.
const MIN_CODE_LEN: usize = 16;
const UNNAMED: &str = "Unnamed startup item";

/// Product names for the "<Product>AutoLaunch_<code>" entries browsers register.
const KNOWN: &[(&str, &str)] = &[("MicrosoftEdge", "Microsoft Edge"), ("GoogleChrome", "Google Chrome")];

/// The name to show and share for a startup item.
pub fn friendly_startup_name(raw: &str) -> String {
    let name = strip_guid(raw.trim());
    if name.is_empty() || is_code(name) {
        return UNNAMED.to_string();
    }
    let name = strip_code(name);
    let name = name.strip_suffix("AutoLaunch").unwrap_or(name);
    match KNOWN.iter().find(|(id, _)| *id == name) {
        Some((_, product)) => (*product).to_string(),
        None if name.is_empty() => UNNAMED.to_string(),
        None => name.to_string(),
    }
}

fn is_code(s: &str) -> bool {
    s.len() >= MIN_CODE_LEN && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// A GUID: 8-4-4-4-12 hex digits, with or without braces.
fn is_guid(s: &str) -> bool {
    let s = s.strip_prefix('{').and_then(|s| s.strip_suffix('}')).unwrap_or(s);
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5 && groups.iter().zip([8, 4, 4, 4, 12]).all(|(g, len)| g.len() == len && g.chars().all(|c| c.is_ascii_hexdigit()))
}

/// "Name_{GUID}", "Name-GUID" or "Name {GUID}" → "Name"; a name that is only a GUID → "".
fn strip_guid(name: &str) -> &str {
    let len = if name.ends_with('}') { 38 } else { 36 };
    match name.len().checked_sub(len) {
        Some(at) if name.is_char_boundary(at) && is_guid(&name[at..]) => name[..at].trim_end_matches(['_', '-', ' ']),
        _ => name,
    }
}

/// "Name_<code>" or "Name-<code>" → "Name"; anything else unchanged.
fn strip_code(name: &str) -> &str {
    match name.rfind(['_', '-']) {
        Some(at) if is_code(&name[at + 1..]) => name[..at].trim_end(),
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::friendly_startup_name as name;

    #[test]
    fn edge_and_chrome_get_their_product_names() {
        assert_eq!(name("MicrosoftEdgeAutoLaunch_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"), "Microsoft Edge");
        assert_eq!(name("GoogleChromeAutoLaunch_8C0D9D7E4B1F4F2AA1B2C3D4E5F60718"), "Google Chrome");
    }

    #[test]
    fn any_long_hex_code_at_the_end_is_dropped() {
        assert_eq!(name("SomeAppAutoLaunch_0123456789ABCDEF0123"), "SomeApp");
        assert_eq!(name("Updater_a1b2c3d4e5f60718a1b2c3d4"), "Updater");
        assert_eq!(name("Tool-0123456789abcdef0123456789abcdef"), "Tool");
    }

    #[test]
    fn a_guid_at_the_end_is_dropped() {
        // A per-install GUID would be a fingerprint as much as a hex code.
        assert_eq!(name("App_{0F1E2D3C-4B5A-6978-8796-A5B4C3D2E1F0}"), "App");
        assert_eq!(name("Tool-0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0"), "Tool");
        assert_eq!(name("Sync Helper {0F1E2D3C-4B5A-6978-8796-A5B4C3D2E1F0}"), "Sync Helper");
        assert_eq!(name("{0F1E2D3C-4B5A-6978-8796-A5B4C3D2E1F0}"), "Unnamed startup item");
        assert_eq!(name("Backup {not a guid}"), "Backup {not a guid}");
    }

    #[test]
    fn ordinary_names_are_left_alone() {
        for plain in ["OneDrive", "Steam", "Discord", "Logitech Options", "EA app", "HP Support Assistant", "Build_2026", "App_DEADBEEF"] {
            assert_eq!(name(plain), plain, "{plain}");
        }
    }

    #[test]
    fn a_name_that_is_only_a_code_keeps_something_readable() {
        assert_eq!(name("0123456789ABCDEF0123456789ABCDEF"), "Unnamed startup item");
        assert_eq!(name("   "), "Unnamed startup item");
    }
}
