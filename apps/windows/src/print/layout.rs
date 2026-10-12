//! The receipt laid out on paper, as drawing operations any backend can draw: the PDF writer and
//! the printer use the same pages. The receipt's look (mono type, dotted leaders, tags, the inked
//! stamp) across the page's width: a small header with the page number on every page, every fold
//! open with its numbers in two columns, and a line, a group of numbers (when it fits on a page) or
//! the footer never split across pages.

use fettle_receipt::{DetailGroup, DetailRow, ReceiptModel};
use fettle_verdicts::Level;

pub const DISPLAY: &[u8] = include_bytes!("../../ui/fonts/bricolage-display-800.ttf");
pub const MONO: &[u8] = include_bytes!("../../ui/fonts/fettle-mono-400.ttf");
pub const MONO_BOLD: &[u8] = include_bytes!("../../ui/fonts/fettle-mono-600.ttf");

/// Page size in points (1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    A4,
    Letter,
    Custom(f32, f32),
}

impl PageSize {
    pub fn points(&self) -> (f32, f32) {
        match self {
            PageSize::A4 => (595.28, 841.89),
            PageSize::Letter => (612.0, 792.0),
            PageSize::Custom(w, h) => (*w, *h),
        }
    }
}

/// 18 mm margins.
pub const MARGIN: f32 = 51.0;
// Mono text: 10 pt, 1.4 line height.
const SIZE: f32 = 10.0;
const CH: f32 = SIZE * 0.6;
const LH: f32 = SIZE * 1.4;
/// The header on every page (the wordmark, when, the page number) and the space under it.
const HEADER: f32 = 36.0;

pub type Rgb = (u8, u8, u8);
pub const INK: Rgb = (0, 0, 0);
const SOFT: Rgb = (0x3A, 0x3A, 0x3A);
const RULE: Rgb = (0xB0, 0xB0, 0xB0);
const ACCENT_INK: Rgb = (0xB9, 0x3A, 0x12);
pub const STAMP_RED: Rgb = (0xD9, 0x48, 0x1E);
const GREEN: Rgb = (0x1F, 0x6A, 0x45);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Display,
    Mono,
    MonoBold,
}

/// A rectangle's outline: its colour, width and, if dashed, (on, off) lengths.
pub type Stroke = (Rgb, f32, Option<(f32, f32)>);

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Text on a baseline at (x, y); `spacing` is extra space after each character.
    Text { x: f32, y: f32, face: Face, size: f32, text: String, colour: Rgb, spacing: f32 },
    /// `dash`: (on, off) lengths.
    Line { x1: f32, y1: f32, x2: f32, y2: f32, colour: Rgb, width: f32, dash: Option<(f32, f32)> },
    Rect { x: f32, y: f32, w: f32, h: f32, fill: Option<Rgb>, stroke: Option<Stroke> },
    /// The fettle. wordmark with its top-left at (x, y), `h` tall.
    Wordmark { x: f32, y: f32, h: f32 },
    /// `ops` drawn around (cx, cy), turned by `degrees` (the inked stamp).
    Rotate { cx: f32, cy: f32, degrees: f32, ops: Vec<Op> },
}

impl Op {
    /// The same op `dy` lower (blocks are laid out from their own top).
    fn shifted(self, dy: f32) -> Op {
        match self {
            Op::Text { x, y, face, size, text, colour, spacing } => Op::Text { x, y: y + dy, face, size, text, colour, spacing },
            Op::Line { x1, y1, x2, y2, colour, width, dash } => Op::Line { x1, y1: y1 + dy, x2, y2: y2 + dy, colour, width, dash },
            Op::Rect { x, y, w, h, fill, stroke } => Op::Rect { x, y: y + dy, w, h, fill, stroke },
            Op::Wordmark { x, y, h } => Op::Wordmark { x, y: y + dy, h },
            // Inner ops are relative to the centre, so only the centre moves.
            Op::Rotate { cx, cy, degrees, ops } => Op::Rotate { cx, cy: cy + dy, degrees, ops },
        }
    }
}

/// A piece that never splits across pages (unless it's taller than a page): its height and what it
/// draws (y from its top).
struct Block {
    h: f32,
    ops: Vec<Op>,
}

/// Word-wraps to `cols` characters; long unbroken names wrap anywhere, never past the paper.
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let cols = cols.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.chars().count() > cols {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            lines.push(word.chars().take(cols).collect());
            word = word.chars().skip(cols).collect();
        }
        let need = if cur.is_empty() { word.chars().count() } else { cur.chars().count() + 1 + word.chars().count() };
        if need > cols && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur += &word;
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

/// The width of display text, from the font's own advances.
pub fn display_width(size: f32, text: &str) -> f32 {
    let Ok(face) = ttf_parser::Face::parse(DISPLAY, 0) else { return text.chars().count() as f32 * size * 0.55 };
    let upem = f32::from(face.units_per_em());
    text.chars().map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map(f32::from).unwrap_or(500.0)).sum::<f32>() * size / upem
}

fn wrap_display(text: &str, size: f32, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let next = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if display_width(size, &next) > width && !cur.is_empty() {
            lines.push(std::mem::replace(&mut cur, word.to_string()));
        } else {
            cur = next;
        }
    }
    lines.push(cur);
    lines
}

/// The receipt's column on the page: its left edge and width.
struct Col {
    x0: f32,
    w: f32,
}

impl Col {
    fn mono(&self, dx: f32, y: f32, text: &str, colour: Rgb) -> Op {
        Op::Text { x: self.x0 + dx, y, face: Face::Mono, size: SIZE, text: text.to_string(), colour, spacing: 0.0 }
    }

    /// Whole characters across `width`.
    fn cols(width: f32) -> usize {
        (width / CH) as usize
    }
}

/// Upper case, letter-spaced labels (section headings, "Next step 1", tags).
fn spaced(x: f32, y: f32, size: f32, text: &str, colour: Rgb, spacing: f32) -> Op {
    Op::Text { x, y, face: Face::MonoBold, size, text: text.to_uppercase(), colour, spacing }
}

fn spaced_width(size: f32, text: &str, spacing: f32) -> f32 {
    text.chars().count() as f32 * (size * 0.6 + spacing)
}

fn baseline(line: usize) -> f32 {
    // Mono text sits about 0.7 of the way down its line box.
    line as f32 * LH + LH * 0.7
}

/// A label, a dotted leader and its value, right-aligned, in `width` from `dx`.
fn kv_block(c: &Col, r: &DetailRow, dx: f32, width: f32) -> Block {
    let cols = Col::cols(width);
    let value_cols = ((cols as f32 * 0.78) as usize).max(8);
    let values = wrap(&r.value, value_cols);
    let first_cols = values[0].chars().count();
    // A label too long for the row wraps too, leaving room for the value and a short leader.
    let labels = wrap(&r.label, cols.saturating_sub(first_cols + 3).max(8));
    let mut ops: Vec<Op> = labels.iter().enumerate().map(|(i, l)| c.mono(dx, baseline(i), l, SOFT)).collect();
    // The dotted leader, like a till's item lines, from the label's first line to the value.
    let label_end = c.x0 + dx + labels[0].chars().count() as f32 * CH + CH;
    let value_start = c.x0 + dx + width - first_cols as f32 * CH - CH;
    if value_start > label_end {
        ops.push(Op::Line { x1: label_end, y1: baseline(0) - 2.8, x2: value_start, y2: baseline(0) - 2.8, colour: RULE, width: 0.8, dash: Some((0.8, 2.2)) });
    }
    for (i, v) in values.iter().enumerate() {
        ops.push(c.mono(dx + width - v.chars().count() as f32 * CH, baseline(i), v, INK));
    }
    Block { h: values.len().max(labels.len()) as f32 * LH, ops }
}

fn rule_block(c: &Col, space: f32) -> Block {
    Block { h: space * 2.0, ops: vec![Op::Line { x1: c.x0, y1: space, x2: c.x0 + c.w, y2: space, colour: RULE, width: 0.8, dash: Some((2.4, 2.4)) }] }
}

fn section_head(c: &Col, title: &str) -> (f32, Vec<Op>) {
    let size = SIZE * 0.95;
    let spacing = size * 0.16;
    let w = spaced_width(size, title, spacing);
    let y = 8.0 + size;
    let line = Op::Line { x1: c.x0 + w + CH, y1: y - size * 0.35, x2: c.x0 + c.w, y2: y - size * 0.35, colour: RULE, width: 0.8, dash: Some((2.4, 2.4)) };
    (size + 16.0, vec![spaced(c.x0, y, size, title, INK, spacing), line])
}

/// A status tag at the right of a line's first row; the word is the signal, colour only helps.
fn tag(level: Level, text: &str, right: f32, top: f32) -> (f32, Vec<Op>) {
    let size = SIZE * 0.85;
    let spacing = size * 0.08;
    let w = spaced_width(size, text, spacing) + 7.0;
    let h = size * 1.3 + 3.0;
    let x = right - w;
    let mut ops = Vec::new();
    let fg = match level {
        Level::ActSoon => {
            ops.push(Op::Rect { x, y: top, w, h, fill: Some(ACCENT_INK), stroke: None });
            (255, 255, 255)
        }
        Level::KeepAnEye => {
            ops.push(Op::Rect { x, y: top, w, h, fill: None, stroke: Some((ACCENT_INK, 1.1, None)) });
            ACCENT_INK
        }
        Level::CouldntCheck => {
            ops.push(Op::Rect { x, y: top, w, h, fill: None, stroke: Some((SOFT, 0.75, Some((2.0, 2.0)))) });
            SOFT
        }
        _ => SOFT,
    };
    ops.push(spaced(x + 3.5, top + h - 3.3, size, text, fg, spacing));
    (w, ops)
}

// The numbers behind a section, as print opens every fold: indented, with a dashed edge.
const INDENT: f32 = 2.0 * CH;
const GAP: f32 = 4.0 * CH;

fn has_content(g: &DetailGroup) -> bool {
    !g.rows.is_empty() || g.title.is_some()
}

/// A small group (a drive's free space): its title and rows down one column, `width` wide from `dx`.
fn stacked(c: &Col, g: &DetailGroup, dx: f32, width: f32) -> (f32, Vec<Op>) {
    let mut ops = Vec::new();
    let mut y = 4.0;
    if let Some(t) = &g.title {
        ops.push(Op::Text { x: c.x0 + dx, y: y + baseline(0), face: Face::MonoBold, size: SIZE, text: t.clone(), colour: INK, spacing: 0.0 });
        y += LH + 2.0;
    }
    for r in &g.rows {
        let b = kv_block(c, r, dx, width);
        ops.extend(b.ops.into_iter().map(|op| op.shifted(y)));
        y += b.h;
    }
    (y, ops)
}

/// A bigger group: its title, then its rows two to a line.
fn two_columns(c: &Col, g: &DetailGroup, col_w: f32) -> (f32, Vec<Op>) {
    let mut ops = Vec::new();
    let mut y = 4.0;
    if let Some(t) = &g.title {
        ops.push(Op::Text { x: c.x0 + INDENT, y: y + baseline(0), face: Face::MonoBold, size: SIZE, text: t.clone(), colour: INK, spacing: 0.0 });
        y += LH + 2.0;
    }
    for pair in g.rows.chunks(2) {
        let mut h: f32 = 0.0;
        for (k, r) in pair.iter().enumerate() {
            let b = kv_block(c, r, INDENT + k as f32 * (col_w + GAP), col_w);
            h = h.max(b.h);
            ops.extend(b.ops.into_iter().map(|op| op.shifted(y)));
        }
        y += h;
    }
    (y, ops)
}

/// A section's groups of numbers, each one block so it stays on one page. Small groups (up to two
/// rows) pair up side by side; bigger ones put their rows two to a line.
fn detail_blocks(c: &Col, groups: &[DetailGroup]) -> Vec<Block> {
    let col_w = (c.w - INDENT - GAP) / 2.0;
    let small = |g: &DetailGroup| g.rows.len() <= 2;
    let groups: Vec<&DetailGroup> = groups.iter().filter(|g| has_content(g)).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < groups.len() {
        let (h, ops) = if small(groups[i]) {
            let pair: Vec<&DetailGroup> = groups[i..].iter().take(2).take_while(|g| small(g)).copied().collect();
            i += pair.len();
            let parts: Vec<(f32, Vec<Op>)> = pair.iter().enumerate().map(|(k, g)| stacked(c, g, INDENT + k as f32 * (col_w + GAP), col_w)).collect();
            (parts.iter().map(|p| p.0).fold(0.0, f32::max), parts.into_iter().flat_map(|p| p.1).collect())
        } else {
            i += 1;
            two_columns(c, groups[i - 1], col_w)
        };
        let mut ops = ops;
        let y = h + 2.0;
        ops.push(Op::Line { x1: c.x0, y1: 0.0, x2: c.x0, y2: y, colour: RULE, width: 0.8, dash: Some((2.4, 2.4)) });
        out.push(Block { h: y + 4.0, ops });
    }
    out
}

fn blocks(c: &Col, m: &ReceiptModel) -> Vec<Block> {
    let mut out = Vec::new();
    // The machine, then the summary everyone reads.
    if !m.machine.is_empty() {
        out.extend(m.machine.iter().map(|r| kv_block(c, r, 0.0, c.w)));
        out.push(rule_block(c, 10.0));
    }
    let size = SIZE * 2.3;
    let summary = wrap_display(&m.summary, size, c.w);
    out.push(Block {
        h: summary.len() as f32 * size * 1.05 + 6.0,
        ops: summary
            .iter()
            .enumerate()
            .map(|(i, l)| Op::Text { x: c.x0, y: (i + 1) as f32 * size * 1.05 - 3.0, face: Face::Display, size, text: l.clone(), colour: INK, spacing: -size * 0.025 })
            .collect(),
    });
    for (i, a) in m.actions.iter().enumerate() {
        let text = wrap(a, Col::cols(c.w));
        let mut ops = vec![spaced(c.x0, 14.0, SIZE * 0.85, &format!("{} {}", m.labels.next_step, i + 1), ACCENT_INK, SIZE * 0.1)];
        ops.extend(text.iter().enumerate().map(|(j, l)| c.mono(0.0, 17.0 + baseline(j), l, INK)));
        out.push(Block { h: 19.0 + text.len() as f32 * LH, ops });
    }
    for s in &m.sections {
        // The heading travels with the section's first line.
        let (head_h, mut head_ops) = section_head(c, &s.title);
        for (i, l) in s.lines.iter().enumerate() {
            let top = if i == 0 { head_h } else { 0.0 };
            let (tag_w, tag_ops) = tag(l.level, &l.tag, c.x0 + c.w, top + 3.0);
            let cols = Col::cols(c.w - tag_w - CH * 2.0);
            let mut ops = std::mem::take(&mut head_ops);
            ops.extend(tag_ops);
            let mut n = 0;
            for t in wrap(&l.headline, cols) {
                ops.push(c.mono(0.0, top + 3.0 + baseline(n), &t, INK));
                n += 1;
            }
            if let Some(d) = &l.detail {
                for t in wrap(d, cols) {
                    ops.push(c.mono(0.0, top + 3.0 + baseline(n), &t, SOFT));
                    n += 1;
                }
            }
            if let Some(a) = &l.action {
                for (j, t) in wrap(a, cols.saturating_sub(2)).iter().enumerate() {
                    ops.push(c.mono(2.0 * CH, top + 5.0 + baseline(n), t, INK));
                    if j == 0 {
                        ops.push(c.mono(0.0, top + 5.0 + baseline(n), ">", INK));
                    }
                    n += 1;
                }
            }
            out.push(Block { h: top + 8.0 + n as f32 * LH, ops });
        }
        let mut groups = detail_blocks(c, &s.details);
        // The section's last line travels with its first group of numbers.
        if !s.lines.is_empty() && !groups.is_empty() {
            let line = out.pop().expect("the section's last line");
            let first = groups.remove(0);
            out.push(Block { h: line.h + first.h, ops: line.ops.into_iter().chain(first.ops.into_iter().map(|op| op.shifted(line.h))).collect() });
        }
        out.extend(groups);
        // Space between sections; the footer brings its own rule.
        if !std::ptr::eq(s, m.sections.last().expect("this section")) {
            out.push(Block { h: 10.0, ops: vec![] });
        }
    }
    out.push(footer(c, m));
    out
}

/// The dashed rule, then one band: the barcode, how it was checked and the versions, and the one
/// inked stamp. Never split.
fn footer(c: &Col, m: &ReceiptModel) -> Block {
    let small = SIZE * 0.85;
    let top = 16.0;
    let mut ops = vec![Op::Line { x1: c.x0, y1: 8.0, x2: c.x0 + c.w, y2: 8.0, colour: RULE, width: 0.8, dash: Some((2.4, 2.4)) }];
    // The stamp's size first: the words sit between the barcode and the stamp.
    let ink = match m.tone.as_str() {
        "some" | "attention" => STAMP_RED,
        "partial" => SOFT,
        _ => GREEN,
    };
    let word = m.labels.stamp.to_uppercase();
    let wsize = 13.0;
    let ww = display_width(wsize, &word) + word.chars().count() as f32;
    let when_w = m.meta.when.chars().count() as f32 * 4.2;
    let (sw, sh) = (ww.max(when_w) + 14.0, 30.0);
    let bars_w = (c.w * 0.25).min(120.0);
    let (left, right) = (c.x0 + bars_w + 16.0, c.x0 + c.w - sw - 16.0);
    let cols = ((right - left) / (small * 0.6)) as usize;
    let lines: Vec<String> = [&m.meta.scan, &m.meta.footer].iter().flat_map(|t| wrap(t, cols)).collect();
    let line_h = small + 4.0;
    let band = (lines.len() as f32 * line_h + 8.0).max(50.0);
    let text_top = top + (band - lines.len() as f32 * line_h) / 2.0;
    for (i, l) in lines.iter().enumerate() {
        let x = left + (right - left - l.chars().count() as f32 * small * 0.6) / 2.0;
        ops.push(Op::Text { x, y: text_top + (i as f32 + 0.75) * line_h, face: Face::Mono, size: small, text: l.clone(), colour: SOFT, spacing: 0.0 });
    }
    let bar_top = top + (band - 30.0) / 2.0;
    let mut x = c.x0;
    while x + 8.25 <= c.x0 + bars_w {
        for (a, b) in [(0.0, 1.5), (3.0, 3.75), (6.0, 8.25)] {
            ops.push(Op::Rect { x: x + a, y: bar_top, w: b - a, h: 30.0, fill: Some(INK), stroke: None });
        }
        x += 9.75;
    }
    ops.push(Op::Rotate {
        cx: c.x0 + c.w - sw / 2.0 - 3.0,
        cy: top + band / 2.0,
        degrees: -9.0,
        ops: vec![
            Op::Rect { x: -sw / 2.0, y: -sh / 2.0, w: sw, h: sh, fill: None, stroke: Some((ink, 2.2, None)) },
            Op::Text { x: -ww / 2.0, y: 1.0, face: Face::Display, size: wsize, text: word, colour: ink, spacing: 1.0 },
            Op::Text { x: -when_w / 2.0, y: 10.0, face: Face::MonoBold, size: 7.0, text: m.meta.when.clone(), colour: if ink == STAMP_RED { ACCENT_INK } else { ink }, spacing: 0.0 },
        ],
    });
    Block { h: top + band + 4.0, ops }
}

/// The top of every page: the wordmark, when the check was made and "page/pages", over a rule.
fn header(c: &Col, m: &ReceiptModel, page: usize, of: usize) -> Vec<Op> {
    let size = SIZE * 0.85;
    let right = format!("{} · {page}/{of}", m.meta.when);
    vec![
        Op::Wordmark { x: c.x0, y: MARGIN, h: 13.0 },
        Op::Text { x: c.x0 + c.w - right.chars().count() as f32 * size * 0.6, y: MARGIN + 11.0, face: Face::Mono, size, text: right, colour: SOFT, spacing: 0.0 },
        Op::Line { x1: c.x0, y1: MARGIN + 21.0, x2: c.x0 + c.w, y2: MARGIN + 21.0, colour: RULE, width: 0.8, dash: None },
    ]
}

/// How far an op reaches up and down from its block's top.
fn extent(op: &Op) -> (f32, f32) {
    match op {
        Op::Text { y, size, .. } => (y - size, y + size * 0.3),
        Op::Line { y1, y2, .. } => (y1.min(*y2), y1.max(*y2)),
        Op::Rect { y, h, .. } => (*y, y + h),
        Op::Wordmark { y, h, .. } => (*y, y + h),
        Op::Rotate { cy, .. } => (cy - 25.0, cy + 25.0),
    }
}

/// A block taller than `room` (a very long value), cut between its lines into pieces that fit.
/// Vertical lines (a details group's dashed edge) are cut to each piece.
fn split(b: Block, room: f32) -> Vec<Block> {
    if b.h <= room {
        return vec![b];
    }
    let is_edge = |op: &Op| matches!(op, Op::Line { x1, x2, y1, y2, .. } if x1 == x2 && y2 > y1);
    let (edges, mut rest): (Vec<Op>, Vec<Op>) = b.ops.into_iter().partition(is_edge);
    rest.sort_by(|a, b| extent(a).0.total_cmp(&extent(b).0));
    // Where each piece starts; an op that would pass the piece's foot starts the next one.
    let mut starts = vec![0.0f32];
    let mut pieces: Vec<Vec<Op>> = vec![Vec::new()];
    for op in rest {
        let (top, bottom) = extent(&op);
        let start = *starts.last().expect("a piece");
        if bottom > start + room && top > start {
            starts.push(top);
            pieces.push(Vec::new());
        }
        let start = *starts.last().expect("a piece");
        pieces.last_mut().expect("a piece").push(op.shifted(-start));
    }
    // A piece ends where the next starts, and never runs past a page.
    let ends: Vec<f32> = starts.iter().skip(1).copied().chain([b.h]).zip(&starts).map(|(end, start)| end.min(start + room)).collect();
    starts
        .iter()
        .zip(ends)
        .zip(pieces)
        .map(|((&start, end), mut ops)| {
            for e in &edges {
                if let Op::Line { x1, y1, x2, y2, colour, width, dash } = e.clone() {
                    let (a, z) = (y1.max(start), y2.min(end));
                    if z > a {
                        ops.push(Op::Line { x1, y1: a - start, x2, y2: z - start, colour, width, dash });
                    }
                }
            }
            Block { h: end - start, ops }
        })
        .collect()
}

/// The blocks on pages, each at its y between `top` and `bottom`. Space between sections never starts
/// a page. The last block is the footer: when it alone won't fit, the empty space between sections on
/// that page is taken back first, so it isn't left on a page of its own.
fn place(blocks: Vec<Block>, top: f32, bottom: f32) -> Vec<Vec<(f32, Block)>> {
    let mut laid: Vec<Vec<(f32, Block)>> = vec![Vec::new()];
    let mut y = top;
    let count = blocks.len();
    for (i, b) in blocks.into_iter().enumerate() {
        let page = laid.last_mut().expect("a page");
        if b.ops.is_empty() && (y + b.h > bottom || page.is_empty()) {
            continue;
        }
        if i + 1 == count && y + b.h > bottom && !page.is_empty() {
            let gaps: f32 = page.iter().filter(|(_, b)| b.ops.is_empty()).map(|(_, b)| b.h).sum();
            if y + b.h - gaps <= bottom {
                page.retain(|(_, b)| !b.ops.is_empty());
                y = top;
                for (at, block) in page.iter_mut() {
                    *at = y;
                    y += block.h;
                }
            }
        }
        if y + b.h > bottom && !page.is_empty() {
            laid.push(Vec::new());
            y = top;
        }
        let h = b.h;
        laid.last_mut().expect("a page").push((y, b));
        y += h;
    }
    laid
}

/// The receipt on pages of `size`: each page's ops in page coordinates (points from the top left),
/// the header first.
pub fn pages(m: &ReceiptModel, size: PageSize) -> Vec<Vec<Op>> {
    let (page_w, page_h) = size.points();
    let c = Col { x0: MARGIN, w: page_w - 2.0 * MARGIN };
    let (top, bottom) = (MARGIN + HEADER, page_h - MARGIN);
    let laid = place(blocks(&c, m).into_iter().flat_map(|b| split(b, bottom - top)).collect(), top, bottom);
    let of = laid.len();
    laid.into_iter()
        .enumerate()
        .map(|(i, blocks)| {
            let mut ops = header(&c, m, i + 1, of);
            for (y, b) in blocks {
                ops.extend(b.ops.into_iter().map(|op| op.shifted(y)));
            }
            ops
        })
        .collect()
}

/// The wordmark's outlines, in its own units (viewBox -4 -142 548 150): (fill colour, path data).
pub const WORDMARK: [(Rgb, &str); 2] = [
    (INK, "M50 0L50-71.60L70-71.60L70-98.80L50-98.80L50-101.40C50-106.20 52.40-109.20 57.60-109.20C62.20-109.20 66.60-106.80 70-103.80L79.80-129.60C74.80-132.20 65-136 53.40-136C27.40-136 14.80-120.20 14-98.80L2-98.80L2-71.60L14-71.60L14 0ZM125.40 2C148 2 161-7.60 168.60-17.40L146.80-38.20C143-29.60 135.80-26.80 126-26.80C116-26.80 108.20-30.20 106-39.80L169.40-39.80C170-42.20 170.40-49.60 170.40-54.40C170.40-78.20 155.80-100.80 122-100.80C91.80-100.80 72.60-79.40 72.60-49.40C72.60-19.40 91.40 2 125.40 2ZM122.20-74C133-74 137.60-68.40 138.20-58.40L105.40-58.40C106.20-68.80 112.40-74 122.20-74ZM224 2C240 2 250.60-5.60 254.80-10.40L245-37.40C241.80-32.40 236.60-29.20 232.20-29.20C227.60-29.20 225-31.80 225-37L225-71.60L249-71.60L249-98.80L225-98.80L225-126L189-110L189-98.80L175-98.80L175-71.60L189-71.60L189-31.80C189-12.60 198.40 2 224 2ZM301.80 2C317.80 2 328.40-5.60 332.60-10.40L322.80-37.40C319.60-32.40 314.40-29.20 310-29.20C305.40-29.20 302.80-31.80 302.80-37L302.80-71.60L326.80-71.60L326.80-98.80L302.80-98.80L302.80-126L266.80-110L266.80-98.80L252.80-98.80L252.80-71.60L266.80-71.60L266.80-31.80C266.80-12.60 276.20 2 301.80 2ZM378.60 0L378.60-134L342.60-134L342.60 0ZM443.40 2C466 2 479-7.60 486.60-17.40L464.80-38.20C461-29.60 453.80-26.80 444-26.80C434-26.80 426.20-30.20 424-39.80L487.40-39.80C488-42.20 488.40-49.60 488.40-54.40C488.40-78.20 473.80-100.80 440-100.80C409.80-100.80 390.60-79.40 390.60-49.40C390.60-19.40 409.40 2 443.40 2ZM440.20-74C451-74 455.60-68.40 456.20-58.40L423.40-58.40C424.20-68.80 430.40-74 440.20-74Z"),
    (STAMP_RED, "M499.8 -40H539.8V2L534.8 -4L529.8 2L524.8 -4L519.8 2L514.8 -4L509.8 2L504.8 -4L499.8 2Z"),
];

/// A step of an outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathStep {
    Move(f32, f32),
    Line(f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// Parses the wordmark's SVG path data (absolute M, L, H, V, C, Z), scaled and moved.
pub fn path_steps(d: &str, scale: f32, dx: f32, dy: f32) -> Vec<PathStep> {
    let mut nums = Vec::new();
    let mut cmds = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, nums: &mut Vec<f32>| {
        if !cur.is_empty() {
            nums.push(cur.parse().unwrap_or(0.0));
            cur.clear();
        }
    };
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            flush(&mut cur, &mut nums);
            cmds.push((ch, nums.len()));
        } else if ch == '-' && !cur.is_empty() {
            flush(&mut cur, &mut nums);
            cur.push(ch);
        } else if ch == ' ' || ch == ',' {
            flush(&mut cur, &mut nums);
        } else {
            cur.push(ch);
        }
    }
    flush(&mut cur, &mut nums);
    let (px, py) = (|v: f32| v * scale + dx, |v: f32| v * scale + dy);
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let mut out = Vec::new();
    for (i, &(c, start)) in cmds.iter().enumerate() {
        let end = cmds.get(i + 1).map(|&(_, s)| s).unwrap_or(nums.len());
        let a = &nums[start..end];
        match c {
            'M' | 'L' => {
                for (k, pair) in a.chunks(2).enumerate() {
                    (x, y) = (pair[0], pair[1]);
                    out.push(if c == 'M' && k == 0 { PathStep::Move(px(x), py(y)) } else { PathStep::Line(px(x), py(y)) });
                }
            }
            'H' => {
                x = a[0];
                out.push(PathStep::Line(px(x), py(y)));
            }
            'V' => {
                y = a[0];
                out.push(PathStep::Line(px(x), py(y)));
            }
            'C' => {
                for k in a.chunks(6) {
                    out.push(PathStep::Cubic(px(k[0]), py(k[1]), px(k[2]), py(k[3]), px(k[4]), py(k[5])));
                    (x, y) = (k[4], k[5]);
                }
            }
            'Z' => out.push(PathStep::Close),
            _ => {}
        }
    }
    out
}

/// The wordmark at (x, y), `h` tall: its outlines in page units.
pub fn wordmark_paths(x: f32, y: f32, h: f32) -> Vec<(Rgb, Vec<PathStep>)> {
    let scale = h / 150.0;
    WORDMARK.iter().map(|(colour, d)| (*colour, path_steps(d, scale, x + 4.0 * scale, y + 142.0 * scale))).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use fettle_receipt::{DetailGroup, DetailRow, Zone, receipt_for};
    use fettle_verdicts::Locale;

    pub(crate) fn dump_model(lang: Locale) -> ReceiptModel {
        receipt_for(&crate::store::tests::dump_report(), lang, Zone::Utc).expect("a receipt").1
    }

    /// A receipt long enough for several pages: the dump's sections, five times over.
    fn long_model() -> ReceiptModel {
        let mut m = dump_model(Locale::En);
        let sections = m.sections.clone();
        for _ in 0..4 {
            m.sections.extend(sections.clone());
        }
        m
    }

    fn texts(ops: &[Op]) -> Vec<String> {
        let mut out = Vec::new();
        for op in ops {
            match op {
                Op::Text { text, .. } => out.push(text.clone()),
                Op::Rotate { ops, .. } => out.extend(texts(ops)),
                _ => {}
            }
        }
        out
    }

    fn lowest(op: &Op) -> f32 {
        match op {
            Op::Text { y, .. } => *y,
            Op::Line { y1, y2, .. } => y1.max(*y2),
            Op::Rect { y, h, .. } => y + h,
            Op::Wordmark { y, h, .. } => y + h,
            Op::Rotate { cy, .. } => cy + 25.0,
        }
    }

    #[test]
    fn a_long_receipt_takes_several_pages_and_stays_inside_the_margins() {
        for size in [PageSize::A4, PageSize::Letter] {
            let pages = pages(&long_model(), size);
            assert!(pages.len() >= 2, "{size:?}: {} pages", pages.len());
            let (_, h) = size.points();
            for (i, page) in pages.iter().enumerate() {
                for op in page {
                    assert!(lowest(op) <= h - MARGIN + 0.5, "{size:?} page {i}: {op:?} below the margin");
                }
            }
        }
    }

    #[test]
    fn the_footer_is_whole_on_the_last_page() {
        let pages = pages(&long_model(), PageSize::A4);
        let last = texts(pages.last().unwrap());
        let m = long_model();
        assert!(last.contains(&m.meta.footer), "{last:?}");
        assert!(pages.last().unwrap().iter().any(|op| matches!(op, Op::Rotate { .. })), "the stamp");
        for page in &pages[..pages.len() - 1] {
            assert!(!page.iter().any(|op| matches!(op, Op::Rotate { .. })));
        }
    }

    #[test]
    fn every_word_of_the_receipt_is_on_paper() {
        let m = dump_model(Locale::It);
        let printed = pages(&m, PageSize::A4).iter().flat_map(|p| texts(p)).collect::<Vec<_>>().join(" ");
        let mut words: Vec<String> = vec![m.summary.clone(), m.meta.when.clone(), m.meta.scan.clone()];
        for s in &m.sections {
            words.push(s.title.to_uppercase());
            for l in &s.lines {
                words.push(l.headline.clone());
                words.extend(l.action.clone());
            }
            for g in &s.details {
                words.extend(g.title.clone());
                words.extend(g.rows.iter().map(|r| r.value.clone()));
            }
        }
        for phrase in words {
            for word in phrase.split_whitespace() {
                assert!(printed.contains(word), "{word:?} from {phrase:?} is missing");
            }
        }
    }

    fn group_titles(m: &ReceiptModel) -> Vec<String> {
        m.sections.iter().flat_map(|s| s.details.iter().filter_map(|g| g.title.clone())).collect()
    }

    #[test]
    fn a_details_title_never_ends_a_page_alone() {
        let m = long_model();
        let titles = group_titles(&m);
        assert!(!titles.is_empty(), "the dump has titled groups");
        // Every page height from short to tall, so each title lands at a page's foot somewhere.
        for h in (300..900).step_by(2) {
            let pages = pages(&m, PageSize::Custom(595.0, h as f32));
            for (i, page) in pages.iter().enumerate().take(pages.len() - 1) {
                let last = page.iter().filter_map(|op| if let Op::Text { y, text, .. } = op { Some((*y, text)) } else { None }).max_by(|a, b| a.0.total_cmp(&b.0)).expect("text on the page");
                assert!(!titles.contains(last.1), "height {h}, page {i} ends with the title {:?}", last.1);
            }
        }
    }

    #[test]
    fn a_row_taller_than_a_page_continues_on_the_next_one() {
        let mut m = dump_model(Locale::En);
        let words: Vec<String> = (0..900).map(|i| format!("w{i}")).collect();
        m.sections[0].details[0].rows[0].value = words.join(" ");
        let (_, h) = PageSize::A4.points();
        let pages = pages(&m, PageSize::A4);
        for (i, page) in pages.iter().enumerate() {
            for op in page {
                assert!(lowest(op) <= h - MARGIN + 0.5, "page {i}: {op:?} below the margin");
                if let Op::Text { y, .. } = op {
                    assert!(*y >= MARGIN, "page {i}: {op:?} above the margin");
                }
            }
        }
        let printed: std::collections::HashSet<String> = pages.iter().flat_map(|p| texts(p)).flat_map(|t| t.split_whitespace().map(String::from).collect::<Vec<_>>()).collect();
        for w in &words {
            assert!(printed.contains(w), "{w} is missing");
        }
    }

    /// Where a text op's ink ends on the right (mono advances are 0.6 em).
    fn right_edge(op: &Op) -> Option<(f32, f32)> {
        match op {
            Op::Text { x, face: Face::Display, size, text, spacing, .. } => Some((*x, x + display_width(*size, text) + spacing * text.chars().count() as f32)),
            Op::Text { x, size, text, spacing, .. } => Some((*x, x + text.chars().count() as f32 * (size * 0.6 + spacing))),
            _ => None,
        }
    }

    #[test]
    fn the_receipt_uses_the_page_width() {
        for size in [PageSize::A4, PageSize::Letter] {
            let (page_w, _) = size.points();
            let pages = pages(&dump_model(Locale::En), size);
            let edges: Vec<(f32, f32)> = pages.iter().flatten().filter_map(right_edge).collect();
            let left = edges.iter().map(|e| e.0).fold(f32::MAX, f32::min);
            let right = edges.iter().map(|e| e.1).fold(0.0, f32::max);
            assert!((left - MARGIN).abs() < 1.0, "{size:?}: text starts at {left}");
            assert!(right <= page_w - MARGIN + 0.5, "{size:?}: text runs to {right}");
            assert!(right > page_w - MARGIN - 8.0, "{size:?}: text stops at {right}, short of the right margin");
        }
    }

    #[test]
    fn every_page_has_the_header_and_its_number() {
        let pages = pages(&long_model(), PageSize::A4);
        let n = pages.len();
        for (i, page) in pages.iter().enumerate() {
            assert!(page.iter().any(|op| matches!(op, Op::Wordmark { .. })), "page {i}: the wordmark");
            let t = texts(page);
            assert!(t.iter().any(|t| t.ends_with(&format!("{}/{n}", i + 1))), "page {i}: {t:?}");
        }
    }

    /// The dump's receipt with a section of twelve groups whose rows are all different.
    fn grouped_model() -> ReceiptModel {
        let mut m = dump_model(Locale::En);
        m.sections[0].details = (0..12)
            .map(|g| DetailGroup { title: Some(format!("G{g}")), rows: (0..7).map(|r| DetailRow { label: format!("r{g}x{r}"), value: format!("v{g}x{r}") }).collect() })
            .collect();
        m
    }

    #[test]
    fn a_group_that_fits_on_a_page_stays_on_one() {
        let m = grouped_model();
        for h in (360..900).step_by(7) {
            let pages = pages(&m, PageSize::Custom(595.0, h as f32));
            let page_of = |text: &str| pages.iter().position(|p| texts(p).iter().any(|t| t == text)).unwrap_or_else(|| panic!("{text} not printed"));
            for g in 0..12 {
                let title = page_of(&format!("G{g}"));
                for r in 0..7 {
                    assert_eq!(page_of(&format!("v{g}x{r}")), title, "height {h}: G{g} row {r}");
                }
            }
        }
    }

    #[test]
    fn group_numbers_sit_in_two_columns() {
        let pages = pages(&grouped_model(), PageSize::A4);
        let at = |text: &str| pages.iter().flatten().find_map(|op| match op {
            Op::Text { x, y, text: t, .. } if t == text => Some((*x, *y)),
            _ => None,
        });
        let (a, b) = (at("r0x0").unwrap(), at("r0x1").unwrap());
        assert_eq!(a.1, b.1, "side by side");
        assert!(b.0 > a.0 + 150.0, "{a:?} {b:?}");
    }

    #[test]
    fn small_groups_sit_side_by_side() {
        let mut m = dump_model(Locale::En);
        m.sections[0].details = (0..5).map(|g| DetailGroup { title: Some(format!("Drive {g}")), rows: vec![DetailRow { label: "Free".into(), value: format!("{g}00 GB") }] }).collect();
        let pages = pages(&m, PageSize::A4);
        let at = |text: &str| pages.iter().flatten().find_map(|op| match op {
            Op::Text { x, y, text: t, .. } if t == text => Some((*x, *y)),
            _ => None,
        }).unwrap_or_else(|| panic!("{text} not printed"));
        let (a, b, c) = (at("Drive 0"), at("Drive 1"), at("Drive 2"));
        assert_eq!(a.1, b.1, "Drive 0 and 1 share a line");
        assert!(b.0 > a.0 + 150.0);
        assert!(c.1 > a.1 && c.0 == a.0, "Drive 2 starts the next pair");
        assert!(pages.iter().flatten().any(|op| matches!(op, Op::Text { text, .. } if text == "400 GB")), "the odd one out is printed too");
    }

    #[test]
    fn a_sections_last_line_stays_with_its_numbers() {
        let m = grouped_model();
        let last = m.sections[0].lines.last().unwrap().headline.clone();
        for h in (360..900).step_by(5) {
            let pages = pages(&m, PageSize::Custom(595.0, h as f32));
            let page_of = |text: &str| pages.iter().position(|p| texts(p).iter().any(|t| last_words(t, text))).unwrap();
            assert_eq!(page_of(&last), page_of("G0"), "height {h}");
        }
    }

    /// Whether a printed line is the given text's last wrapped line (or the whole text).
    fn last_words(printed: &str, text: &str) -> bool {
        printed == text || (!printed.is_empty() && text.ends_with(printed) && text.len() > printed.len())
    }

    #[test]
    fn the_rule_above_the_footer_goes_with_it() {
        let (page_w, _) = PageSize::A4.points();
        for h in (360..900).step_by(5) {
            let pages = pages(&long_model(), PageSize::Custom(page_w, h as f32));
            let last = pages.last().unwrap();
            let stamp_y = last.iter().find_map(|op| if let Op::Rotate { cy, .. } = op { Some(*cy) } else { None }).unwrap();
            let rule = last.iter().any(|op| matches!(op, Op::Line { x1, x2, y1, dash: Some(_), .. } if (*x1 - MARGIN).abs() < 0.1 && (*x2 - (page_w - MARGIN)).abs() < 0.1 && *y1 < stamp_y));
            assert!(rule, "height {h}: the footer's rule is on another page");
        }
    }

    #[test]
    fn no_page_starts_with_a_gap() {
        for h in (360..900).step_by(3) {
            let pages = pages(&long_model(), PageSize::Custom(595.0, h as f32));
            for (i, page) in pages.iter().enumerate() {
                // Past the header's three ops: the first thing on the page sits right under it.
                let first = page[3..].iter().map(|op| extent(op).0).fold(f32::MAX, f32::min);
                assert!(first < MARGIN + HEADER + 9.0, "height {h}, page {i}: starts at {first}");
            }
        }
    }

    #[test]
    fn the_footer_is_one_band() {
        let c = Col { x0: MARGIN, w: 493.0 };
        assert!(footer(&c, &dump_model(Locale::It)).h <= 80.0);
    }

    #[test]
    fn the_dump_receipt_prints_on_three_a4_pages_with_the_footer_on_the_last() {
        // Its "Next step: run a Deep check" once made it just long enough to push the footer alone onto
        // a page of its own; the gaps between sections now make room for it beside the last section.
        let pages = pages(&dump_model(Locale::En), PageSize::A4);
        assert_eq!(pages.len(), 3);
        let last = &pages[2];
        assert!(last.iter().any(|op| matches!(op, Op::Rotate { .. })), "the stamp is on the last page");
        // More than the page's header (three ops) and the footer: a section shares the page.
        let footer_ops = footer(&Col { x0: MARGIN, w: 595.0 - 2.0 * MARGIN }, &dump_model(Locale::En)).ops.len();
        assert!(last.len() > 3 + footer_ops, "the footer is alone on the last page");
    }

    /// A block of `h` points with something on it (an empty one is space between sections).
    fn ink(h: f32) -> Block {
        Block { h, ops: vec![Op::Line { x1: 0.0, y1: 0.0, x2: 1.0, y2: 0.0, colour: INK, width: 1.0, dash: None }] }
    }

    fn gap() -> Block {
        Block { h: 10.0, ops: vec![] }
    }

    #[test]
    fn the_footer_takes_back_the_gaps_rather_than_sit_alone_on_a_page() {
        // 87 to 791: three sections and their two gaps reach 736, and the 70 pt footer is 15 over.
        let (top, bottom) = (87.0, 791.0);
        let laid = place(vec![ink(479.0), gap(), ink(100.0), gap(), ink(50.0), ink(70.0)], top, bottom);
        assert_eq!(laid.len(), 1, "the gaps (20 pt) make room for it");
        let last = laid[0].last().unwrap();
        assert!(last.0 + last.1.h <= bottom, "inside the margin: {}", last.0 + last.1.h);
        assert_eq!(laid[0].iter().filter(|(_, b)| b.ops.is_empty()).count(), 0, "the gaps went");
        // 25 over and only 20 pt of gaps: the footer takes a page of its own, and the gaps stay.
        let laid = place(vec![ink(489.0), gap(), ink(100.0), gap(), ink(50.0), ink(70.0)], top, bottom);
        assert_eq!(laid.len(), 2);
        assert_eq!(laid[0].iter().filter(|(_, b)| b.ops.is_empty()).count(), 2);
    }

    #[test]
    fn wrapping_never_overflows_the_column() {
        for text in ["", "a", "Fosi Audio USB Audio Device Control Panel Autostart.lnk", &"x".repeat(100)] {
            for line in wrap(text, 20) {
                assert!(line.chars().count() <= 20, "{line}");
            }
        }
    }
}
