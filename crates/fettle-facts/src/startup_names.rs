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
    let name = raw.trim();
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
