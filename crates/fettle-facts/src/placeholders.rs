//! What firmware writes when the maker never filled a field in ("Default string", "To Be Filled
//! By O.E.M."). The scan leaves these out of serials and the board name, field by field, and the
//! receipt leaves them out of board names saved by Fettle 1.0.4 and earlier.

/// Firmware's own phrases, never part of a real name: left out wherever they are.
/// Compared without case, as whole words.
const FIRMWARE: &[&str] = &[
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
];

/// Placeholders only as a whole field: inside a name they can be real words ("None Edition").
const GENERIC: &[&str] = &["not applicable", "not available", "not specified", "none", "n/a"];

/// Whether the whole text is a placeholder (spaces around it and case don't count).
pub fn is_placeholder(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    !words.is_empty() && placeholder_at(FIRMWARE.iter().chain(GENERIC), &words) == Some(words.len())
}

/// How many words at the start of `words` make one of `phrases`, if they do (the longest one).
fn placeholder_at<'a>(phrases: impl Iterator<Item = &'a &'a str>, words: &[&str]) -> Option<usize> {
    phrases
        .map(|p| p.split(' ').collect::<Vec<_>>())
        .filter(|p| p.len() <= words.len() && p.iter().zip(words).all(|(p, w)| w.eq_ignore_ascii_case(p)))
        .map(|p| p.len())
        .max()
}

/// `words` without the parts that make one of `phrases`.
fn strip<'a>(words: &[&'a str], phrases: &[&str]) -> Vec<&'a str> {
    let mut kept = Vec::new();
    let mut i = 0;
    while i < words.len() {
        match placeholder_at(phrases.iter(), &words[i..]) {
            Some(n) => i += n,
            None => {
                kept.push(words[i]);
                i += 1;
            }
        }
    }
    kept
}

/// A board name saved already joined ("Manufacturer Product") without firmware's own phrases;
/// generic words such as "None" stay, as they may be part of the name. None when nothing real
/// is left: the text is all placeholders, generic words included ("Default string N/A").
pub fn without_placeholders(name: &str) -> Option<String> {
    let words: Vec<&str> = name.split_whitespace().collect();
    let kept = strip(&words, FIRMWARE);
    let all: Vec<&str> = FIRMWARE.iter().chain(GENERIC).copied().collect();
    (!strip(&kept, &all).is_empty()).then(|| kept.join(" "))
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

    #[test]
    fn generic_words_inside_a_real_name_are_kept() {
        // "None" or "Not Available" can be part of a real name; only firmware's own phrases go.
        assert_eq!(without_placeholders("Example Boards Ltd. None Edition"), Some("Example Boards Ltd. None Edition".into()));
        assert_eq!(without_placeholders("Example Boards Ltd. Not Available Series"), Some("Example Boards Ltd. Not Available Series".into()));
        assert_eq!(without_placeholders("Default string None Edition"), Some("None Edition".into()));
        // But a name that is nothing but placeholders is still dropped, generic words included.
        assert_eq!(without_placeholders("N/A"), None);
        assert_eq!(without_placeholders(" none "), None);
        assert_eq!(without_placeholders("Default string N/A"), None);
        assert_eq!(without_placeholders("N/A N/A"), None);
    }
}
