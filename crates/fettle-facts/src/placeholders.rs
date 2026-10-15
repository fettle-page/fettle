//! What firmware writes when the maker never filled a field in ("Default string", "To Be Filled
//! By O.E.M."). The scan leaves these out of serials and the board name, and the receipt leaves
//! them out of board names saved by Fettle 1.0.4 and earlier.

/// Compared without case, as whole words.
const PLACEHOLDERS: &[&str] = &[
    "to be filled by o.e.m.",
    "to be filled by oem",
    "default string",
    "system manufacturer",
    "system product name",
    "system serial number",
    "system version",
    "base board manufacturer",
    "base board product name",
    "base board serial number",
    "type2 - board vendor name1",
    "type2 - board product name1",
    "o.e.m.",
    "not applicable",
    "not available",
    "not specified",
    "none",
    "n/a",
];

/// Whether the whole text is a placeholder (spaces around it and case don't count).
pub fn is_placeholder(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    !words.is_empty() && placeholder_at(&words) == Some(words.len())
}

/// How many words at the start of `words` make a placeholder, if they do (the longest one).
fn placeholder_at(words: &[&str]) -> Option<usize> {
    PLACEHOLDERS
        .iter()
        .map(|p| p.split(' ').collect::<Vec<_>>())
        .filter(|p| p.len() <= words.len() && p.iter().zip(words).all(|(p, w)| w.eq_ignore_ascii_case(p)))
        .map(|p| p.len())
        .max()
}

/// A name joined from firmware fields ("Manufacturer Product") without its placeholder parts;
/// None when nothing real is left.
pub fn without_placeholders(name: &str) -> Option<String> {
    let words: Vec<&str> = name.split_whitespace().collect();
    let mut kept = Vec::new();
    let mut i = 0;
    while i < words.len() {
        match placeholder_at(&words[i..]) {
            Some(n) => i += n,
            None => {
                kept.push(words[i]);
                i += 1;
            }
        }
    }
    (!kept.is_empty()).then(|| kept.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_placeholders_are_recognised_in_any_case() {
        for p in ["Default string", "  TO BE FILLED BY O.E.M. ", "System Product Name", "N/A", "none"] {
            assert!(is_placeholder(p), "{p}");
        }
        for real in ["", "Example Boards Ltd.", "Default", "EX-B650 TEST", "Default string 2"] {
            assert!(!is_placeholder(real), "{real}");
        }
    }

    #[test]
    fn placeholder_parts_are_dropped_from_a_joined_name() {
        assert_eq!(without_placeholders("Default string Default string"), None);
        assert_eq!(without_placeholders("To Be Filled By O.E.M. To Be Filled By O.E.M."), None);
        assert_eq!(without_placeholders("Example Boards Ltd. Default string"), Some("Example Boards Ltd.".into()));
        assert_eq!(without_placeholders("Type2 - Board Vendor Name1 EX-B650 TEST"), Some("EX-B650 TEST".into()));
        assert_eq!(without_placeholders("Example Boards Ltd. EX-B650 TEST"), Some("Example Boards Ltd. EX-B650 TEST".into()));
        assert_eq!(without_placeholders("   "), None);
    }
}
