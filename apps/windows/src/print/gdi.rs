//! Printing: Windows' print dialog, then the receipt drawn on the printer page by page with GDI,
//! from the same layout as the PDF. The brand fonts are loaded for this app only.

use super::layout::{DISPLAY, Face, MONO, MONO_BOLD, Op, PageSize, PathStep, Rgb, pages, wordmark_paths};
use fettle_receipt::ReceiptModel;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use windows::Win32::Foundation::{COLORREF, HWND, POINT};
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;

/// Device units per point, across and down: the printer's (or bitmap's) dots per inch over 72.
pub struct Device {
    pub hdc: HDC,
    pub dots_per_point: (f32, f32),
    /// Where the printable area starts on the paper, in device units.
    pub offset: (f32, f32),
}

/// The brand fonts, loaded once for this process only (nothing is installed).
fn load_fonts() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        for data in [DISPLAY, MONO, MONO_BOLD] {
            let count = 0u32;
            unsafe {
                let _ = AddFontMemResourceEx(data.as_ptr().cast(), data.len() as u32, None, &count);
            }
        }
    });
}

fn colour(c: Rgb) -> COLORREF {
    COLORREF(u32::from(c.0) | (u32::from(c.1) << 8) | (u32::from(c.2) << 16))
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Draws ops in one coordinate space: `kx`/`ky` device units per point, minus the printable offset.
struct Painter {
    hdc: HDC,
    kx: f32,
    ky: f32,
    ox: f32,
    oy: f32,
    fonts: RefCell<HashMap<(Face, u32), HFONT>>,
}

fn painter(device: &Device) -> Painter {
    Painter { hdc: device.hdc, kx: device.dots_per_point.0, ky: device.dots_per_point.1, ox: device.offset.0, oy: device.offset.1, fonts: RefCell::new(HashMap::new()) }
}

/// What StartDoc's answer means: None to go on; Ok(false) when "Print to PDF"'s file prompt was
/// cancelled (ERROR_CANCELLED); Err otherwise.
fn start_doc_outcome(ret: i32, last_error: u32) -> Option<Result<bool, String>> {
    match (ret > 0, last_error) {
        (true, _) => None,
        (false, 1223) => Some(Ok(false)),
        _ => Some(Err("the printer refused the job".into())),
    }
}

impl Painter {
    fn p(&self, x: f32, y: f32) -> (i32, i32) {
        ((x * self.kx - self.ox).round() as i32, (y * self.ky - self.oy).round() as i32)
    }

    /// For sizes that aren't along one axis (pen widths): the mean of both scales.
    fn k(&self) -> f32 {
        (self.kx + self.ky) / 2.0
    }

    fn font(&self, face: Face, size: f32) -> HFONT {
        let key = (face, (size * 100.0) as u32);
        *self.fonts.borrow_mut().entry(key).or_insert_with(|| {
            let (name, weight) = match face {
                Face::Display => ("Fettle Display", 800),
                Face::Mono => ("Fettle Mono", 400),
                Face::MonoBold => ("Fettle Mono", 600),
            };
            let name = wide(name);
            unsafe {
                CreateFontW(-(size * self.ky).round() as i32, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET, OUT_TT_ONLY_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, 0, PCWSTR(name.as_ptr()))
            }
        })
    }

    fn pen(&self, c: Rgb, width: f32, dash: Option<(f32, f32)>) -> HPEN {
        let brush = LOGBRUSH { lbStyle: BS_SOLID, lbColor: colour(c), lbHatch: 0 };
        let w = (width * self.k()).round().max(1.0) as u32;
        unsafe {
            match dash {
                Some((on, off)) => {
                    let style = [(on * self.k()).round().max(1.0) as u32, (off * self.k()).round().max(1.0) as u32];
                    ExtCreatePen(PS_GEOMETRIC | PS_USERSTYLE | PS_ENDCAP_FLAT, w, &brush, Some(&style))
                }
                None => ExtCreatePen(PS_GEOMETRIC | PS_SOLID | PS_ENDCAP_FLAT, w, &brush, None),
            }
        }
    }

    fn draw(&self, ops: &[Op]) {
        let h = self.hdc;
        for op in ops {
            unsafe {
                match op {
                    Op::Text { x, y, face, size, text, colour: c, spacing } => {
                        let old = SelectObject(h, self.font(*face, *size).into());
                        SetTextColor(h, colour(*c));
                        SetBkMode(h, TRANSPARENT);
                        SetTextAlign(h, TA_BASELINE | TA_LEFT);
                        let (px, py) = self.p(*x, *y);
                        let units: Vec<u16> = text.encode_utf16().collect();
                        if *face == Face::Display {
                            SetTextCharacterExtra(h, (spacing * self.kx).round() as i32);
                            let _ = TextOutW(h, px, py, &units);
                            SetTextCharacterExtra(h, 0);
                        } else {
                            // Mono text at its exact design advance: GDI rounds each glyph to whole
                            // dots, which would push right-aligned values past the column.
                            let advance = (size * 0.6 + spacing) * self.kx;
                            let dx: Vec<i32> = (0..units.len()).map(|i| ((i + 1) as f32 * advance).round() as i32 - (i as f32 * advance).round() as i32).collect();
                            let _ = ExtTextOutW(h, px, py, ETO_OPTIONS(0), None, PCWSTR(units.as_ptr()), units.len() as u32, Some(dx.as_ptr()));
                        }
                        SelectObject(h, old);
                    }
                    Op::Line { x1, y1, x2, y2, colour: c, width, dash } => {
                        let pen = self.pen(*c, *width, *dash);
                        let old = SelectObject(h, pen.into());
                        let (a, b) = (self.p(*x1, *y1), self.p(*x2, *y2));
                        let _ = MoveToEx(h, a.0, a.1, None);
                        let _ = LineTo(h, b.0, b.1);
                        SelectObject(h, old);
                        let _ = DeleteObject(pen.into());
                    }
                    Op::Rect { x, y, w, h: rh, fill, stroke } => {
                        let brush = fill.map(|c| CreateSolidBrush(colour(c)));
                        let pen = stroke.map(|(c, width, dash)| self.pen(c, width, dash));
                        let old_brush = SelectObject(h, brush.map(Into::into).unwrap_or_else(|| GetStockObject(NULL_BRUSH)));
                        let old_pen = SelectObject(h, pen.map(Into::into).unwrap_or_else(|| GetStockObject(NULL_PEN)));
                        let (a, b) = (self.p(*x, *y), self.p(x + w, y + rh));
                        // Without a pen, GDI leaves out the right and bottom edges: add them back.
                        let extra = i32::from(pen.is_none());
                        let _ = Rectangle(h, a.0, a.1, b.0 + extra, b.1 + extra);
                        SelectObject(h, old_brush);
                        SelectObject(h, old_pen);
                        if let Some(b) = brush {
                            let _ = DeleteObject(b.into());
                        }
                        if let Some(p) = pen {
                            let _ = DeleteObject(p.into());
                        }
                    }
                    Op::Wordmark { x, y, h: wh } => {
                        for (c, steps) in wordmark_paths(*x, *y, *wh) {
                            let brush = CreateSolidBrush(colour(c));
                            let old = SelectObject(h, brush.into());
                            let _ = BeginPath(h);
                            for s in steps {
                                match s {
                                    PathStep::Move(px, py) => {
                                        let q = self.p(px, py);
                                        let _ = MoveToEx(h, q.0, q.1, None);
                                    }
                                    PathStep::Line(px, py) => {
                                        let q = self.p(px, py);
                                        let _ = LineTo(h, q.0, q.1);
                                    }
                                    PathStep::Cubic(a, b, c2, d, e, f) => {
                                        let pts = [self.p(a, b), self.p(c2, d), self.p(e, f)].map(|(x, y)| POINT { x, y });
                                        let _ = PolyBezierTo(h, &pts);
                                    }
                                    PathStep::Close => {
                                        let _ = CloseFigure(h);
                                    }
                                }
                            }
                            let _ = EndPath(h);
                            let _ = FillPath(h);
                            SelectObject(h, old);
                            let _ = DeleteObject(brush.into());
                        }
                    }
                    Op::Rotate { cx, cy, degrees, ops } => {
                        // The inner ops are in points around the centre, drawn at 10 units a point
                        // and scaled, turned and moved into place by the world transform.
                        let (sin, cos) = degrees.to_radians().sin_cos();
                        let (sx, sy) = (self.kx / 10.0, self.ky / 10.0);
                        let (tx, ty) = (cx * self.kx - self.ox, cy * self.ky - self.oy);
                        let previous = GetGraphicsMode(h);
                        SetGraphicsMode(h, GM_ADVANCED);
                        // Turned in points, then scaled to the device's own dots across and down.
                        let xf = XFORM { eM11: cos * sx, eM12: sin * sy, eM21: -sin * sx, eM22: cos * sy, eDx: tx, eDy: ty };
                        let _ = SetWorldTransform(h, &xf);
                        let inner = Painter { hdc: h, kx: 10.0, ky: 10.0, ox: 0.0, oy: 0.0, fonts: RefCell::new(HashMap::new()) };
                        inner.draw(ops);
                        drop(inner);
                        let _ = ModifyWorldTransform(h, None, MWT_IDENTITY);
                        SetGraphicsMode(h, GRAPHICS_MODE(previous));
                    }
                }
            }
        }
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        for (_, font) in self.fonts.borrow_mut().drain() {
            unsafe {
                let _ = DeleteObject(font.into());
            }
        }
    }
}

/// Draws one page's ops.
pub fn draw_page(device: &Device, ops: &[Op]) {
    load_fonts();
    painter(device).draw(ops);
}

/// One page drawn in memory exactly as the printer gets it, `per_point` pixels to a point, on white:
/// (width, height, 0x00RRGGBB pixels, top row first). For the print preview (and the tests).
pub fn render_page(ops: &[Op], size: PageSize, per_point: f32) -> (i32, i32, Vec<u32>) {
    let (w_pt, h_pt) = size.points();
    let (w, h) = ((w_pt * per_point).round() as i32, (h_pt * per_point).round() as i32);
    // SAFETY: a memory DC and a DIB section of exactly w × h 32-bit pixels, read back after
    // GdiFlush, then every GDI object is released.
    unsafe {
        let dc = CreateCompatibleDC(None);
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..Default::default() },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DeleteDC(dc);
            return (0, 0, Vec::new());
        };
        let old = SelectObject(dc, bmp.into());
        let white = CreateSolidBrush(COLORREF(0x00FF_FFFF));
        FillRect(dc, &windows::Win32::Foundation::RECT { left: 0, top: 0, right: w, bottom: h }, white);
        let _ = DeleteObject(white.into());
        draw_page(&Device { hdc: dc, dots_per_point: (per_point, per_point), offset: (0.0, 0.0) }, ops);
        let _ = GdiFlush();
        let px = std::slice::from_raw_parts(bits as *const u32, (w * h) as usize).to_vec();
        SelectObject(dc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(dc);
        (w, h, px)
    }
}

/// Whether Windows can print here: its print service (the Print Spooler, which IT departments often
/// switch off) is running and at least one printer is installed. Both are read without admin rights;
/// when Windows won't say, the print dialog decides.
pub fn printing_available() -> bool {
    use windows::Win32::Graphics::Printing::{EnumPrintersW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL};
    use windows::Win32::System::Services::*;
    use windows::core::w;
    // SAFETY: every handle is checked and closed here; EnumPrintersW is only asked for the size.
    unsafe {
        let Ok(scm) = OpenSCManagerW(None, None, SC_MANAGER_CONNECT) else { return true };
        let running = match OpenServiceW(scm, w!("Spooler"), SERVICE_QUERY_STATUS) {
            Ok(service) => {
                let mut status = SERVICE_STATUS::default();
                let ok = QueryServiceStatus(service, &mut status).is_ok();
                let _ = CloseServiceHandle(service);
                !ok || status.dwCurrentState == SERVICE_RUNNING
            }
            // No Print Spooler at all (removed from this Windows).
            Err(_) => false,
        };
        let _ = CloseServiceHandle(scm);
        if !running {
            return false;
        }
        let (mut needed, mut count) = (0u32, 0u32);
        let _ = EnumPrintersW(PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS, None, 4, None, &mut needed, &mut count);
        needed > 0
    }
}

/// Windows' print dialog, then every page on the chosen printer. Ok(false) when cancelled.
pub fn print(owner: HWND, model: &ReceiptModel) -> Result<bool, String> {
    use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
    use windows::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
    use windows::Win32::UI::Controls::Dialogs::*;
    let mut dlg = PRINTDLGEXW {
        lStructSize: std::mem::size_of::<PRINTDLGEXW>() as u32,
        hwndOwner: owner,
        Flags: PD_RETURNDC | PD_NOPAGENUMS | PD_NOSELECTION | PD_NOCURRENTPAGE | PD_USEDEVMODECOPIESANDCOLLATE,
        nStartPage: START_PAGE_GENERAL,
        ..Default::default()
    };
    unsafe {
        PrintDlgExW(&mut dlg).map_err(|e| e.to_string())?;
        let free = |g: HGLOBAL| {
            if !g.is_invalid() {
                let _ = GlobalFree(Some(g));
            }
        };
        free(dlg.hDevMode);
        free(dlg.hDevNames);
        if dlg.dwResultAction != PD_RESULT_PRINT || dlg.hDC.is_invalid() {
            if !dlg.hDC.is_invalid() {
                let _ = DeleteDC(dlg.hDC);
            }
            return Ok(false);
        }
        let hdc = dlg.hDC;
        let caps = |i: GET_DEVICE_CAPS_INDEX| GetDeviceCaps(Some(hdc), i) as f32;
        // Many printers have more dots down than across (600 × 1200 dpi): scale each axis.
        let dpp = (caps(LOGPIXELSX) / 72.0, caps(LOGPIXELSY) / 72.0);
        // The paper's real size, so A4, Letter and anything else lay out right.
        let size = PageSize::Custom(caps(PHYSICALWIDTH) / caps(LOGPIXELSX) * 72.0, caps(PHYSICALHEIGHT) / caps(LOGPIXELSY) * 72.0);
        let device = Device { hdc, dots_per_point: dpp, offset: (caps(PHYSICALOFFSETX), caps(PHYSICALOFFSETY)) };
        let name = wide(&crate::exports::job_name(model));
        let doc = DOCINFOW { cbSize: std::mem::size_of::<DOCINFOW>() as i32, lpszDocName: PCWSTR(name.as_ptr()), ..Default::default() };
        let started = StartDocW(hdc, &doc);
        if let Some(outcome) = start_doc_outcome(started, windows::Win32::Foundation::GetLastError().0) {
            let _ = DeleteDC(hdc);
            return outcome;
        }
        let mut ok = true;
        for page in pages(model, size) {
            ok = StartPage(hdc) > 0;
            if ok {
                draw_page(&device, &page);
                ok = EndPage(hdc) > 0;
            }
            if !ok {
                break;
            }
        }
        // A job that stopped halfway is taken back off the printer's queue.
        ok = if ok { EndDoc(hdc) > 0 } else { AbortDoc(hdc) > 0 && false };
        let _ = DeleteDC(hdc);
        if ok { Ok(true) } else { Err("the printer stopped".into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::print::layout::{Face, MARGIN, PageSize, pages};
    use fettle_verdicts::Locale;

    /// An A4 page at 96 dpi in memory, white, drawn on, then read back as 32-bit pixels.
    fn render_page(ops: &[Op]) -> (i32, i32, Vec<u32>) {
        super::render_page(ops, PageSize::A4, 96.0 / 72.0)
    }

    fn inked(px: &[u32], w: i32, x0: i32, y0: i32, x1: i32, y1: i32) -> usize {
        let mut n = 0;
        for y in y0.max(0)..y1 {
            for x in x0.max(0)..x1.min(w) {
                if px[(y * w + x) as usize] & 0x00FF_FFFF < 0x00A0_A0A0 {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn printers_with_different_horizontal_and_vertical_resolution_keep_proportions() {
        // A 600 × 1200 dpi printer: a point is 600/72 dots across and 1200/72 down.
        let device = Device { hdc: HDC::default(), dots_per_point: (600.0 / 72.0, 1200.0 / 72.0), offset: (0.0, 0.0) };
        let painter = painter(&device);
        assert_eq!(painter.p(72.0, 72.0), (600, 1200));
    }

    /// This PC's answer (it can print): `cargo test -p fettle-windows -- --ignored printing_is`.
    #[test]
    #[ignore]
    fn printing_is_available_here() {
        assert!(printing_available());
    }

    #[test]
    fn a_cancelled_print_to_file_is_not_a_failure() {
        // StartDoc fails with ERROR_CANCELLED (1223) when "Microsoft Print to PDF"'s name prompt is cancelled.
        assert_eq!(start_doc_outcome(-1, 1223), Some(Ok(false)));
        assert_eq!(start_doc_outcome(-1, 5), Some(Err("the printer refused the job".into())));
        assert_eq!(start_doc_outcome(7, 0), None);
    }

    #[test]
    fn the_summary_and_the_footer_are_inked_and_the_margins_stay_white() {
        let model = crate::print::layout::tests::dump_model(Locale::It);
        let all = pages(&model, PageSize::A4);
        let s = 96.0 / 72.0;
        let first = &all[0];
        let (w, h, px) = render_page(first);
        // The summary: big display text.
        let Some(Op::Text { x, y, size, .. }) = first.iter().find(|op| matches!(op, Op::Text { face: Face::Display, .. })) else { panic!("no summary") };
        let (sx, sy) = ((x * s) as i32, (y * s) as i32);
        assert!(inked(&px, w, sx, sy - (size * s) as i32, sx + 200, sy) > 200, "the summary is on paper");
        // Nothing in the top and bottom margins.
        let m = (MARGIN * s) as i32 - 2;
        assert_eq!(inked(&px, w, 0, 0, w, m), 0, "top margin");
        assert_eq!(inked(&px, w, 0, h - m, w, h), 0, "bottom margin");
        // The footer's barcode on the last page.
        let last = all.last().unwrap();
        let (w, lh, px) = render_page(last);
        // FETTLE_SHOTS=<folder>: also save both pages as PNGs, to look at what the printer gets.
        if let Ok(dir) = std::env::var("FETTLE_SHOTS") {
            for (name, (pw, ph, pixels)) in [("print-first.png", render_page(first)), ("print-last.png", (w, lh, px.clone()))] {
                let rgba: Vec<u8> = pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]).collect();
                let _ = image::save_buffer(std::path::Path::new(&dir).join(name), &rgba, pw as u32, ph as u32, image::ExtendedColorType::Rgba8);
            }
        }
        let bars = last.iter().filter_map(|op| match op { Op::Rect { x, y, fill: Some(_), .. } => Some((*x, *y)), _ => None }).next_back().expect("the barcode");
        let (bx, by) = ((bars.0 * s) as i32, (bars.1 * s) as i32);
        assert!(inked(&px, w, bx - 60, by, bx + 10, by + 30) > 100, "the barcode is on paper");
    }
}
