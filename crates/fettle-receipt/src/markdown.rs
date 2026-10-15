//! The receipt as compact markdown, for pasting into Discord or Reddit when asking for help.
//! Lists rather than bare lines: Reddit joins single line breaks.

use crate::ReceiptModel;

/// Discord's message limit. Reddit's is far higher, so this one decides.
const MAX: usize = 2000;

/// Text from the PC (models, names) can hold markdown or Discord mentions (`@everyone`, `<@123>`); a
/// backslash keeps it literal on both sites.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if r"\*_~`|<>^[]@".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Length as JavaScript's `.length` counts it.
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Markdown from the receipt, at most 2000 UTF-16 units (Discord's limit, as JavaScript counts).
pub fn markdown(model: &ReceiptModel) -> String {
    let lines: Vec<_> = model.sections.iter().flat_map(|s| &s.lines).collect();
    let width = lines.iter().map(|l| utf16_len(&l.tag)).max().unwrap_or(0);
    let mut out = vec![format!("**Fettle** · {}", esc(&model.meta.when)), format!("**{}**", esc(&model.summary))];
    if !model.machine.is_empty() {
        out.push(String::new());
        out.extend(model.machine.iter().map(|r| format!("- {}: {}", esc(&r.label), esc(&r.value))));
    }
    for section in &model.sections {
        out.push(String::new());
        out.push(format!("**{}**", esc(&section.title)));
        for l in &section.lines {
            let action = l.action.as_ref().map(|a| format!(" → {}", esc(a))).unwrap_or_default();
            let pad = " ".repeat(width.saturating_sub(utf16_len(&l.tag)));
            out.push(format!("- `{}{pad}` {}{action}", l.tag, esc(&l.headline)));
        }
    }
    let footer = vec![String::new(), format!("_{} · {}_", esc(&model.meta.scan), esc(&model.meta.footer))];
    // A very long receipt loses lines from the end, never the footer, and says so.
    let fits = |lines: &[String]| utf16_len(&lines.iter().chain(&footer).cloned().collect::<Vec<_>>().join("\n")) <= MAX;
    if !fits(&out) {
        let with_ellipsis = |out: &[String]| [out, &["…".to_string()]].concat();
        while out.len() > 2 && !fits(&with_ellipsis(&out)) {
            out.pop();
        }
        out.push("…".into());
    }
    out.extend(footer);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Labels, Meta, ModelLine, ModelSection};
    use fettle_verdicts::{Level, Locale, SectionId};

    fn model(lines: usize, headline: &str) -> ReceiptModel {
        let line = |i: usize| ModelLine { id: format!("l{i}"), level: Level::Fine, tag: "OK".into(), level_text: "Fine".into(), headline: headline.into(), detail: None, action: None };
        ReceiptModel {
            locale: Locale::En,
            tone: "fine".into(),
            summary: "In fine fettle.".into(),
            actions: vec![],
            machine: vec![],
            sections: vec![ModelSection { id: SectionId::Storage, title: "Storage".into(), lines: (0..lines).map(line).collect(), details: vec![] }],
            meta: Meta { when: "07/10/2026 12:32 UTC".into(), collected_at: "2026-10-07T12:32:00Z".into(), deep: false, footer: "Fettle 0.1.0".into(), scan: "Quick check".into() },
            labels: Labels { details: "details".into(), next_step: "Next step".into(), machine: "This PC".into(), stamp: "Checked".into() },
        }
    }

    fn utf16(s: &str) -> usize {
        s.encode_utf16().count()
    }

    #[test]
    fn a_long_receipt_keeps_its_footer_within_2000_utf16_units() {
        // Each emoji is two UTF-16 units: counting chars instead would overshoot the limit.
        let md = markdown(&model(200, "🚀💾 drive looks healthy, 0% worn."));
        assert!(utf16(&md) <= 2000, "{}", utf16(&md));
        assert!(md.contains("\n…\n"));
        assert!(md.ends_with("_Quick check · Fettle 0.1.0_"));
        // One more line would not have fitted.
        let kept = md.lines().filter(|l| l.starts_with("- `")).count();
        assert!(kept < 200);
    }

    #[test]
    fn markdown_characters_from_the_pc_are_escaped() {
        let md = markdown(&model(1, "WD_BLACK *fast* `x`|y [z] > ~ ^ \\"));
        assert!(md.contains(r"WD\_BLACK \*fast\* \`x\`\|y \[z\] \> \~ \^ \\"), "{md}");
    }

    #[test]
    fn mentions_and_tags_from_the_pc_stay_plain_text() {
        // A disk renamed "@everyone" or "<@123>" would ping people once pasted into Discord.
        let md = markdown(&model(1, "@everyone <@123> <#456>"));
        assert!(md.contains(r"\@everyone \<\@123\> \<#456\>"), "{md}");
    }
}
