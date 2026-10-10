//! What a memory stick's labels say. Windows doesn't report timings, but most makers spell the
//! rated speed and CAS latency (CL) out in the part number, so Fettle reads them from there and
//! says so. Unknown formats return nothing rather than a guess. (receipt-ui's memory.ts; its
//! regular expressions are written out by hand here.)

/// `n` ASCII digits at `at`, as a number.
fn digits_at(s: &[u8], at: usize, n: usize) -> Option<u32> {
    let part = s.get(at..at + n)?;
    part.iter().all(u8::is_ascii_digit).then(|| std::str::from_utf8(part).ok()?.parse().ok())?
}

/// `(\d{4})<tag>(\d{2})` at `at`: speed and CL.
fn speed_tag_cl(s: &[u8], at: usize, tag: &[u8]) -> Option<(u32, u32)> {
    let speed = digits_at(s, at, 4)?;
    (s.get(at + 4..at + 4 + tag.len())? == tag).then_some(())?;
    Some((speed, digits_at(s, at + 4 + tag.len(), 2)?))
}

/// `[A-Z0-9]*?` then `(\d{4})<tag>(\d{2})`, from `from`: the shortest run that matches.
fn lazy_then(s: &[u8], from: usize, tag: &[u8]) -> Option<(u32, u32)> {
    let mut at = from;
    loop {
        if let Some(found) = speed_tag_cl(s, at, tag) {
            return Some(found);
        }
        match s.get(at) {
            Some(c) if c.is_ascii_uppercase() || c.is_ascii_digit() => at += 1,
            _ => return None,
        }
    }
}

/// The rated speed (MT/s) and CL printed in a part number, or None when the format is unknown.
pub fn rated_from_part_number(part: Option<&str>) -> Option<(u32, u32)> {
    let p = part.unwrap_or("").trim().to_uppercase();
    let s = p.as_bytes();
    let starts = |prefix: &str| p.starts_with(prefix);
    // G.Skill: F5-6000J2836G16G (DDR5-6000 CL28), F4-3200C16D-16GVKB
    if s.len() >= 3 && s[0] == b'F' && matches!(s[1], b'3' | b'4' | b'5') && s[2] == b'-' {
        let speed = digits_at(s, 3, 4);
        let letter = s.get(7).is_some_and(u8::is_ascii_uppercase);
        if let (Some(speed), true, Some(cl)) = (speed, letter, digits_at(s, 8, 2)) {
            return Some((speed, cl));
        }
    }
    // Corsair: CMK32GX5M2B6000C36, CMW32GX4M2E3200C16
    if starts("CM")
        && let Some(found) = lazy_then(s, 2, b"C")
    {
        return Some(found);
    }
    // Kingston Fury / HyperX: KF560C36… (DDR5-6000), HX432C16… (DDR4-3200)
    if (starts("KF") || starts("HX")) && matches!(s.get(2), Some(b'3' | b'4' | b'5')) {
        if let (Some(speed), Some(b'C'), Some(cl)) = (digits_at(s, 3, 2), s.get(5), digits_at(s, 6, 2)) {
            return Some((speed * 100, cl));
        }
    }
    // Crucial / Crucial Pro / Ballistix: CT16G56C46U5, CP16G60C36U5B, BL16G32C16U4B
    if starts("CT") || starts("CP") || starts("BL") {
        let run = s[2..].iter().take_while(|c| c.is_ascii_digit()).count();
        let at = 2 + run;
        if run > 0 && s.get(at) == Some(&b'G') {
            if let (Some(speed), Some(b'C'), Some(cl)) = (digits_at(s, at + 1, 2), s.get(at + 3), digits_at(s, at + 4, 2)) {
                return Some((speed * 100, cl));
            }
        }
    }
    // TeamGroup: FF3D532G6000HC38ADC01
    for prefix in ["FF", "TF", "TL", "TED"] {
        if starts(prefix)
            && let Some(found) = lazy_then(s, prefix.len(), b"HC")
        {
            return Some(found);
        }
    }
    None
}

/// `CHANNEL\s*-?\s*([A-H])\b` (case-insensitive) at byte `at` of the lowercased text.
fn channel_strict(t: &[u8], at: usize) -> Option<char> {
    let mut i = at + "channel".len();
    while t.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    if t.get(i) == Some(&b'-') {
        i += 1;
    }
    while t.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    let c = *t.get(i).filter(|c| (b'a'..=b'h').contains(c))?;
    let boundary = !t.get(i + 1).is_some_and(|n| n.is_ascii_alphanumeric() || *n == b'_');
    boundary.then(|| (c as char).to_ascii_uppercase())
}

/// The memory channel letter from firmware slot labels ("P0 CHANNEL A", "ChannelB-DIMM0").
pub fn channel_of(bank: Option<&str>, locator: Option<&str>) -> Option<char> {
    let text = format!("{} {}", bank.unwrap_or(""), locator.unwrap_or("")).to_ascii_lowercase();
    let t = text.as_bytes();
    let starts: Vec<usize> = text.match_indices("channel").map(|(i, _)| i).collect();
    starts.iter().find_map(|&at| channel_strict(t, at)).or_else(|| {
        // Then `CHANNEL([A-H])` anywhere, without the word boundary.
        starts.iter().find_map(|&at| t.get(at + 7).filter(|c| (b'a'..=b'h').contains(*c)).map(|c| (*c as char).to_ascii_uppercase()))
    })
}

/// Channel and DIMM number when the labels say them clearly (the DIMM only with a channel).
pub fn slot_of(bank: Option<&str>, locator: Option<&str>) -> (Option<char>, Option<String>) {
    let channel = channel_of(bank, locator);
    // `DIMM\s*_?(\d+)` (case-insensitive) in the locator.
    let loc = locator.unwrap_or("").to_ascii_lowercase();
    let l = loc.as_bytes();
    let dimm = loc.match_indices("dimm").find_map(|(at, _)| {
        let mut i = at + 4;
        while l.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if l.get(i) == Some(&b'_') {
            i += 1;
        }
        let n = l[i.min(l.len())..].iter().take_while(|c| c.is_ascii_digit()).count();
        (n > 0).then(|| loc[i..i + n].to_string())
    });
    (channel, dimm.filter(|_| channel.is_some()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rated_speed_and_cl_from_part_numbers() {
        let cases: [(&str, Option<(u32, u32)>); 14] = [
            ("F5-6000J2836G16G", Some((6000, 28))),     // G.Skill DDR5
            ("F4-3200C16D-16GVKB", Some((3200, 16))),   // G.Skill DDR4
            ("CMK32GX5M2B6000C36", Some((6000, 36))),   // Corsair DDR5
            ("CMW32GX4M2E3200C16", Some((3200, 16))),   // Corsair DDR4
            ("KF560C36BBEK2-32", Some((6000, 36))),     // Kingston Fury DDR5
            ("HX432C16FB3/8", Some((3200, 16))),        // HyperX DDR4
            ("CT16G56C46U5", Some((5600, 46))),         // Crucial DDR5
            ("CP16G60C36U5B", Some((6000, 36))),        // Crucial Pro
            ("BL16G32C16U4B", Some((3200, 16))),        // Ballistix
            ("FF3D532G6000HC38ADC01", Some((6000, 38))), // TeamGroup
            ("CT8G4DFS832A", None),                     // Crucial without a CL in the name
            ("M378A1K43EB2-CWE", None),                 // Samsung OEM
            ("HMCG78AGBUA081N", None),                  // SK hynix OEM
            ("", None),
        ];
        for (part, want) in cases {
            assert_eq!(rated_from_part_number(Some(part)), want, "{part}");
        }
    }

    #[test]
    fn ignores_padding_windows_leaves_around_part_numbers() {
        assert_eq!(rated_from_part_number(Some("  F5-6000J2836G16G              ")), Some((6000, 28)));
        assert_eq!(rated_from_part_number(Some("f5-6000j2836g16g")), Some((6000, 28)));
    }

    #[test]
    fn reads_amd_style_labels() {
        assert_eq!(channel_of(Some("P0 CHANNEL A"), Some("DIMM 1")), Some('A'));
        assert_eq!(slot_of(Some("P0 CHANNEL A"), Some("DIMM 1")), (Some('A'), Some("1".into())));
    }

    #[test]
    fn reads_intel_style_labels() {
        assert_eq!(channel_of(Some("BANK 0"), Some("ChannelB-DIMM0")), Some('B'));
        assert_eq!(slot_of(Some("BANK 0"), Some("ChannelB-DIMM0")), (Some('B'), Some("0".into())));
        assert_eq!(channel_of(None, Some("Controller0-ChannelA")), Some('A'));
    }

    #[test]
    fn gives_up_cleanly_on_labels_it_does_not_know() {
        assert_eq!(channel_of(Some("BANK 2"), Some("DIMM_A2")), None);
        assert_eq!(slot_of(None, None), (None, None));
        // A DIMM number without a channel isn't shown.
        assert_eq!(slot_of(Some("BANK 1"), Some("DIMM 2")), (None, None));
    }
}
