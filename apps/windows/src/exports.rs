//! Save and Copy: the receipt as a PDF or the report as JSON through Windows' save dialog, and the
//! receipt's markdown on the clipboard.

use crate::app::{Clipboard, Dialogs};
use crate::copy::t;
use crate::print::{layout::PageSize, pdf};
use fettle_facts::Report;
use fettle_receipt::ReceiptModel;
use fettle_verdicts::Locale;
use std::path::{Path, PathBuf};

/// Windows' own save dialog (IFileSaveDialog), modal to the app's window.
pub struct RealDialogs;

impl Dialogs for RealDialogs {
    fn save(&self, owner: &slint::Window, suggested: &str, filters: &[(String, Vec<String>)]) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_parent(&owner.window_handle()).set_file_name(suggested);
        for (name, extensions) in filters {
            dialog = dialog.add_filter(name, extensions);
        }
        dialog.save_file()
    }
}

/// Windows' print dialog and GDI, from the same layout as the PDF.
pub struct RealPrinter;

impl crate::app::Printer for RealPrinter {
    fn available(&self) -> bool {
        #[cfg(windows)]
        return crate::print::gdi::printing_available();
        #[cfg(not(windows))]
        false
    }
    fn print(&self, owner: &slint::Window, model: &ReceiptModel) -> Result<bool, String> {
        #[cfg(windows)]
        {
            let hwnd = crate::win::hwnd(owner).ok_or("no window")?;
            crate::print::gdi::print(hwnd, model)
        }
        #[cfg(not(windows))]
        {
            let _ = (owner, model);
            Err("printing needs Windows".into())
        }
    }
}

/// The width of a preview page on screen, in logical pixels.
pub const PREVIEW_WIDTH: f32 = 520.0;

/// The receipt's pages as images for the print preview, drawn by the same code that prints them,
/// sharp at the window's `scale`.
pub fn preview_pages(model: &ReceiptModel, size: PageSize, scale: f32) -> Vec<slint::Image> {
    #[cfg(windows)]
    {
        use slint::{Rgba8Pixel, SharedPixelBuffer};
        let per_point = PREVIEW_WIDTH * scale / size.points().0;
        crate::print::layout::pages(model, size)
            .iter()
            .map(|ops| {
                let (w, h, px) = crate::print::gdi::render_page(ops, size, per_point);
                let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w as u32, h as u32);
                for (out, p) in buffer.make_mut_slice().iter_mut().zip(px) {
                    *out = Rgba8Pixel { r: (p >> 16) as u8, g: (p >> 8) as u8, b: p as u8, a: 255 };
                }
                slint::Image::from_rgba8(buffer)
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        let _ = (model, size, scale);
        Vec::new()
    }
}

/// An https link, or plain http to a Worker on this PC (development). The host must be exactly
/// localhost or 127.0.0.1, so "http://localhost.evil.example" and "http://localhost:80@evil.example"
/// (where localhost is only a user name) don't pass.
fn openable(url: &str) -> bool {
    if url.starts_with("https://") {
        return true;
    }
    let Some(rest) = url.strip_prefix("http://") else { return false };
    let authority = rest.split(['/', '\\', '?', '#']).next().unwrap_or("");
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    let port_ok = port.is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    (host == "localhost" || host == "127.0.0.1") && port_ok
}

/// Opens a share link in the default browser (only fettle's own https links).
pub fn open_in_browser(url: &str) -> bool {
    if !openable(url) {
        return false;
    }
    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        use windows::core::{HSTRING, w};
        let r = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(url), None, None, SW_SHOWNORMAL) };
        r.0 as usize > 32
    }
    #[cfg(not(windows))]
    false
}

pub struct RealClipboard;

impl Clipboard for RealClipboard {
    fn set_text(&self, text: &str) -> bool {
        arboard::Clipboard::new().and_then(|mut c| c.set_text(text)).is_ok()
    }
}

/// The print job's name, as the printer queue shows it, in the receipt's language.
pub fn job_name(model: &ReceiptModel) -> String {
    t("export.job_name", model.locale, &[])
}

/// "fettle-2026-10-07", the scan's date where the user is (from the receipt's own stamp).
pub fn suggested_name(model: &ReceiptModel) -> String {
    let when = &model.meta.when;
    match (when.get(0..2), when.get(3..5), when.get(6..10)) {
        (Some(d), Some(m), Some(y)) if when.as_bytes().get(2) == Some(&b'/') => format!("fettle-{y}-{m}-{d}"),
        _ => "fettle".into(),
    }
}

/// The paper Windows is set up for: Letter where it measures in inches, A4 everywhere else.
pub fn paper_size() -> PageSize {
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::{GetLocaleInfoEx, LOCALE_IMEASURE};
        use windows::core::PCWSTR;
        let mut buf = [0u16; 4];
        let n = unsafe { GetLocaleInfoEx(PCWSTR::null(), LOCALE_IMEASURE, Some(&mut buf)) };
        if n > 0 && buf[0] == u16::from(b'1') {
            return PageSize::Letter;
        }
    }
    PageSize::A4
}

fn write_json(path: &Path, report: &Report) -> std::io::Result<()> {
    // The engine version travels with the report, so a shared receipt shows the same verdicts.
    let report = Report { engine_version: Some(fettle_verdicts::ENGINE_VERSION.to_string()), ..report.clone() };
    std::fs::write(path, serde_json::to_string_pretty(&report).map_err(std::io::Error::other)?)
}

fn write_pdf(path: &Path, model: &ReceiptModel) -> std::io::Result<()> {
    let bytes = pdf::render(model, paper_size()).map_err(std::io::Error::other)?;
    std::fs::write(path, bytes)
}

/// Asks where, then writes the receipt as a PDF, or the (already redacted) report as JSON when a
/// .json name is chosen. The dock's message: None when cancelled.
pub fn save(dialogs: &dyn Dialogs, owner: &slint::Window, report: &Report, model: &ReceiptModel, lang: Locale) -> Option<&'static str> {
    save_as(dialogs, owner, report, model, lang, true)
}

/// Save offering the PDF only (Print's stand-in when Windows can't print).
pub fn save_pdf(dialogs: &dyn Dialogs, owner: &slint::Window, report: &Report, model: &ReceiptModel, lang: Locale) -> Option<&'static str> {
    save_as(dialogs, owner, report, model, lang, false)
}

fn save_as(dialogs: &dyn Dialogs, owner: &slint::Window, report: &Report, model: &ReceiptModel, lang: Locale, json: bool) -> Option<&'static str> {
    let mut filters = vec![(t("export.filter_pdf", lang, &[]), vec!["pdf".to_string()])];
    if json {
        filters.push((t("export.filter_json", lang, &[]), vec!["json".to_string()]));
    }
    let path = dialogs.save(owner, &suggested_name(model), &filters)?;
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
    let written = if json && ext.as_deref() == Some("json") {
        write_json(&path, report)
    } else {
        write_pdf(&pdf_path(path), model)
    };
    Some(if written.is_ok() { "export.saved" } else { "export.failed" })
}

/// Where to write a PDF the user named `chosen` in the save dialog. A name not ending in .pdf gets
/// ".pdf" added (receipt.txt becomes receipt.txt.pdf, never an existing receipt.pdf), and since
/// Windows only asked before replacing the name the user typed, a name made up here never replaces a
/// file: receipt.txt (2).pdf, (3) and so on.
fn pdf_path(chosen: PathBuf) -> PathBuf {
    if chosen.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
        return chosen;
    }
    let name = chosen.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "fettle".into());
    (1..)
        .map(|n| chosen.with_file_name(if n == 1 { format!("{name}.pdf") } else { format!("{name} ({n}).pdf") }))
        .find(|p| !p.exists())
        .expect("a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_without_pdf_gets_it_added_and_never_replaces_another_file() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/fettle-pdf-name");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // The name chosen in the dialog (Windows already asked before replacing that one).
        assert_eq!(pdf_path(dir.join("receipt.pdf")), dir.join("receipt.pdf"));
        assert_eq!(pdf_path(dir.join("receipt.PDF")), dir.join("receipt.PDF"));
        // Another name keeps what was typed, so receipt.txt never turns into an existing receipt.pdf.
        std::fs::write(dir.join("receipt.pdf"), "someone's file").unwrap();
        assert_eq!(pdf_path(dir.join("receipt.txt")), dir.join("receipt.txt.pdf"));
        assert_eq!(pdf_path(dir.join("receipt")), dir.join("receipt (2).pdf"), "no ending at all, and receipt.pdf is taken");
        // And a name Fettle made up never replaces a file Windows didn't ask about.
        std::fs::write(dir.join("receipt.txt.pdf"), "another file").unwrap();
        assert_eq!(pdf_path(dir.join("receipt.txt")), dir.join("receipt.txt (2).pdf"));
        std::fs::write(dir.join("receipt.txt (2).pdf"), "and another").unwrap();
        assert_eq!(pdf_path(dir.join("receipt.txt")), dir.join("receipt.txt (3).pdf"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn opens_https_links_and_the_local_worker() {
        for url in [
            "https://fettle.page/r/abc",
            "http://localhost",
            "http://localhost/r/abc",
            "http://localhost:8787/r/abc",
            "http://127.0.0.1:8787/r/abc",
            "http://localhost?x=1",
        ] {
            assert!(openable(url), "{url}");
        }
    }

    #[test]
    fn refuses_links_that_only_look_like_the_local_worker() {
        for url in [
            "http://localhost.evil.example/",
            "http://localhostevil.example",
            "http://127.0.0.1.evil.example/",
            "http://localhost@evil.example/",
            "http://localhost:8787@evil.example/",
            "http://127.0.0.1:80@evil.example",
            "http://localhost:/",
            "http://evil.example/",
            "file:///C:/Windows/System32/calc.exe",
            "ms-settings:",
            "",
        ] {
            assert!(!openable(url), "{url}");
        }
    }

    #[test]
    fn the_print_job_is_named_in_the_receipts_language() {
        assert_eq!(job_name(&crate::print::layout::tests::dump_model(Locale::En)), "Fettle receipt");
        assert_eq!(job_name(&crate::print::layout::tests::dump_model(Locale::It)), "Scontrino Fettle");
    }

    #[test]
    fn the_suggested_name_is_the_scan_day() {
        let mut m = crate::print::layout::tests::dump_model(Locale::En);
        m.meta.when = "07/10/2026 22:29".into();
        assert_eq!(suggested_name(&m), "fettle-2026-10-07");
        m.meta.when = "07/10/2026 22:29 UTC".into();
        assert_eq!(suggested_name(&m), "fettle-2026-10-07");
        m.meta.when = "garbage".into();
        assert_eq!(suggested_name(&m), "fettle");
    }
}
