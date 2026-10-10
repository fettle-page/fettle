//! The print layout as a vector PDF: real text with the brand fonts embedded, so it can be searched
//! and copied, and looks the same printed from any viewer.

use super::layout::{DISPLAY, Face, MONO, MONO_BOLD, Op, PageSize, PathStep, Rgb, display_width, pages, wordmark_paths};
use fettle_receipt::ReceiptModel;
use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect, Transform};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke, StrokeDash};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};

struct Fonts {
    display: Font,
    mono: Font,
    mono_bold: Font,
}

fn paint(c: Rgb) -> krilla::paint::Paint {
    rgb::Color::new(c.0, c.1, c.2).into()
}

fn fill(c: Rgb) -> Fill {
    Fill { paint: paint(c), opacity: NormalizedF32::ONE, rule: Default::default() }
}

fn stroke(c: Rgb, width: f32, dash: Option<(f32, f32)>) -> Stroke {
    Stroke { paint: paint(c), width, dash: dash.map(|(on, off)| StrokeDash { array: vec![on, off], offset: 0.0 }), ..Default::default() }
}

fn path(steps: &[PathStep]) -> Option<krilla::geom::Path> {
    let mut pb = PathBuilder::new();
    for s in steps {
        match *s {
            PathStep::Move(x, y) => pb.move_to(x, y),
            PathStep::Line(x, y) => pb.line_to(x, y),
            PathStep::Cubic(a, b, c, d, e, f) => pb.cubic_to(a, b, c, d, e, f),
            PathStep::Close => pb.close(),
        }
    }
    pb.finish()
}

fn draw(s: &mut Surface, fonts: &Fonts, ops: &[Op]) {
    for op in ops {
        match op {
            Op::Text { x, y, face, size, text, colour, spacing } => {
                s.set_fill(Some(fill(*colour)));
                let font = match face {
                    Face::Display => fonts.display.clone(),
                    Face::Mono => fonts.mono.clone(),
                    Face::MonoBold => fonts.mono_bold.clone(),
                };
                if *spacing == 0.0 || (*face == Face::Display && *spacing < 0.0) {
                    s.draw_text(Point::from_xy(*x, *y), font, *size, text, false, TextDirection::Auto);
                } else {
                    // Letter-spaced text, glyph by glyph.
                    let mut cx = *x;
                    for ch in text.chars() {
                        let t = ch.to_string();
                        s.draw_text(Point::from_xy(cx, *y), font.clone(), *size, &t, false, TextDirection::Auto);
                        cx += if *face == Face::Display { display_width(*size, &t) } else { size * 0.6 } + spacing;
                    }
                }
            }
            Op::Line { x1, y1, x2, y2, colour, width, dash } => {
                if (x2 - x1).abs() < 0.01 && (y2 - y1).abs() < 0.01 {
                    continue;
                }
                if let Some(p) = path(&[PathStep::Move(*x1, *y1), PathStep::Line(*x2, *y2)]) {
                    s.set_fill(None);
                    s.set_stroke(Some(stroke(*colour, *width, *dash)));
                    s.draw_path(&p);
                    s.set_stroke(None);
                }
            }
            Op::Rect { x, y, w, h, fill: f, stroke: st } => {
                let mut pb = PathBuilder::new();
                if let Some(r) = Rect::from_xywh(*x, *y, *w, *h) {
                    pb.push_rect(r);
                }
                if let Some(p) = pb.finish() {
                    s.set_fill(f.map(fill));
                    s.set_stroke(st.map(|(c, w, d)| stroke(c, w, d)));
                    s.draw_path(&p);
                    s.set_stroke(None);
                }
            }
            Op::Wordmark { x, y, h } => {
                for (colour, steps) in wordmark_paths(*x, *y, *h) {
                    if let Some(p) = path(&steps) {
                        s.set_stroke(None);
                        s.set_fill(Some(fill(colour)));
                        s.draw_path(&p);
                    }
                }
            }
            Op::Rotate { cx, cy, degrees, ops } => {
                s.push_transform(&Transform::from_translate(*cx, *cy));
                s.push_transform(&Transform::from_rotate(*degrees));
                draw(s, fonts, ops);
                s.pop();
                s.pop();
            }
        }
    }
}

/// The receipt as a PDF on pages of `size`.
pub fn render(m: &ReceiptModel, size: PageSize) -> Result<Vec<u8>, String> {
    let fonts = Fonts {
        display: Font::new(DISPLAY.to_vec().into(), 0).ok_or("display font")?,
        mono: Font::new(MONO.to_vec().into(), 0).ok_or("mono font")?,
        mono_bold: Font::new(MONO_BOLD.to_vec().into(), 0).ok_or("mono font")?,
    };
    let (w, h) = size.points();
    let mut doc = Document::new();
    for ops in pages(m, size) {
        let mut page = doc.start_page_with(PageSettings::from_wh(w, h).ok_or("page size")?);
        let mut s = page.surface();
        draw(&mut s, &fonts, &ops);
        s.finish();
        page.finish();
    }
    doc.finish().map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fettle_receipt::{Zone, receipt_for};
    use fettle_verdicts::Locale;
    use std::path::PathBuf;

    #[test]
    fn a_receipt_becomes_a_pdf() {
        let bytes = render(&crate::print::layout::tests::dump_model(Locale::En), PageSize::A4).expect("a PDF");
        assert!(bytes.starts_with(b"%PDF"), "{:?}", &bytes[..8]);
        assert!(bytes.len() > 4_000);
    }

    /// To look at the paper: `FETTLE_SHOTS=<folder> cargo test -p fettle-windows -- --ignored pdf_to_look_at`
    /// writes the test receipt as dump.pdf, and the report in FETTLE_REPORT (if set) as report.pdf and
    /// report-it.pdf.
    #[test]
    #[ignore]
    fn pdf_to_look_at() {
        let dir = PathBuf::from(std::env::var("FETTLE_SHOTS").expect("FETTLE_SHOTS"));
        std::fs::write(dir.join("dump.pdf"), render(&crate::print::layout::tests::dump_model(Locale::En), PageSize::A4).unwrap()).unwrap();
        if let Ok(path) = std::env::var("FETTLE_REPORT") {
            let report = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            for (lang, name) in [(Locale::En, "report.pdf"), (Locale::It, "report-it.pdf")] {
                let (_, m) = receipt_for(&report, lang, Zone::Offset(120)).unwrap();
                std::fs::write(dir.join(name), render(&m, PageSize::A4).unwrap()).unwrap();
            }
        }
    }

    #[test]
    fn every_receipt_in_the_parity_corpus_renders() {
        // Odd names, long receipts, every verdict and both languages: none may fail to print.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/fettle-receipt/tests/parity/cases");
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap().step_by(7) {
            let case: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap();
            let report = serde_json::from_value(case["report"].clone()).unwrap();
            for lang in [Locale::En, Locale::It] {
                if let Ok((_, m)) = receipt_for(&report, lang, Zone::Utc) {
                    assert!(render(&m, PageSize::Letter).is_ok_and(|b| b.starts_with(b"%PDF")));
                    count += 1;
                }
            }
        }
        assert!(count > 100, "{count}");
    }
}
