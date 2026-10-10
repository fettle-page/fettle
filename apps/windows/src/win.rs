//! Windows: tint Windows' own title bar to the theme, read the display language, measure memory,
//! and capture the app's own window for development snapshots.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};

pub fn hwnd(window: &slint::Window) -> Option<HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle();
    match handle.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as _)),
        _ => None,
    }
}

const fn colorref(rgb: u32) -> u32 {
    // 0xRRGGBB → COLORREF 0x00BBGGRR
    ((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF)
}

/// The title bar stays Windows' own (Snap Layouts, Alt+Tab, the system menu all as Windows makes
/// them); it only takes the side panel's colour, so the window reads as one piece.
pub fn tint_title_bar(window: &slint::Window, dark: bool, oled: bool) -> bool {
    let Some(hwnd) = hwnd(window) else { return false };
    let (caption, text) = if oled {
        (0x000000, 0xEDEBE4)
    } else if dark {
        (0x202422, 0xEDEBE4)
    } else {
        (0xEEF1EF, 0x1D1C1A)
    };
    unsafe {
        let dark_mode: i32 = dark as i32;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark_mode as *const _ as _, 4);
        // Windows 11 only; Windows 10 ignores these and keeps its dark or light caption.
        let caption = colorref(caption);
        let text = colorref(text);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR, &caption as *const _ as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_TEXT_COLOR, &text as *const _ as _, 4);
    }
    true
}

/// Windows' display language, as a Fettle language (Italian, or else English). Not the region
/// format: English Windows with Italian dates still speaks English.
pub fn system_lang() -> fettle_verdicts::Locale {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    lang_from_langid(unsafe { GetUserDefaultUILanguage() })
}

/// Whether Windows' apps are set to dark (Settings, Personalisation, Colours). None when Windows
/// doesn't say (older versions): Fettle then stays light.
pub fn system_dark() -> Option<bool> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: a DWORD read into a local of the right size.
    let read = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"), w!("AppsUseLightTheme"), RRF_RT_REG_DWORD, None, Some(&mut value as *mut u32 as *mut _), Some(&mut size))
    };
    read.is_ok().then_some(value == 0)
}

/// The machine-wide update-check default an all-users install wrote (HKLM\Software\Fettle,
/// UpdateCheck: 0 off, anything else on). None when there's none.
pub fn machine_updates() -> Option<bool> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: a DWORD read into a local of the right size.
    let read = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, w!(r"Software\Fettle"), w!("UpdateCheck"), RRF_RT_REG_DWORD, None, Some(&mut value as *mut u32 as *mut _), Some(&mut size)) };
    read.is_ok().then_some(value != 0)
}

/// Working set and private bytes, in MB.
pub fn memory_mb() -> (f64, f64) {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
    use windows::Win32::System::Threading::GetCurrentProcess;
    let mut c = PROCESS_MEMORY_COUNTERS_EX { cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32, ..Default::default() };
    unsafe {
        let _ = GetProcessMemoryInfo(GetCurrentProcess(), &mut c as *mut _ as _, c.cb);
    }
    (c.WorkingSetSize as f64 / 1048576.0, c.PrivateUsage as f64 / 1048576.0)
}

/// The window as Windows draws it, frame included, as RGBA (for checking the title bar's tint).
pub fn print_window(window: &slint::Window) -> Option<(u32, u32, Vec<u8>)> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let hwnd = hwnd(window)?;
    unsafe {
        let mut r = RECT::default();
        GetWindowRect(hwnd, &mut r).ok()?;
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(dc, bmp.into());
        // PW_RENDERFULLCONTENT (2): what DWM composes, title bar included.
        let _ = PrintWindow(hwnd, dc, PRINT_WINDOW_FLAGS(2));
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..Default::default() },
            ..Default::default()
        };
        let mut px = vec![0u8; (w * h * 4) as usize];
        GetDIBits(dc, bmp, 0, h as u32, Some(px.as_mut_ptr() as _), &mut info, DIB_RGB_COLORS);
        SelectObject(dc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
        for p in px.chunks_mut(4) {
            p.swap(0, 2);
            p[3] = 255;
        }
        Some((w as u32, h as u32, px))
    }
}

/// The Fettle language for a Windows UI language id (its primary language: 0x10 is Italian).
pub fn lang_from_langid(langid: u16) -> fettle_verdicts::Locale {
    if langid & 0x3FF == 0x10 { fettle_verdicts::Locale::It } else { fettle_verdicts::Locale::En }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fettle_verdicts::Locale;

    #[test]
    fn the_display_language_decides() {
        assert_eq!(lang_from_langid(0x0410), Locale::It); // Italian (Italy)
        assert_eq!(lang_from_langid(0x0810), Locale::It); // Italian (Switzerland)
        assert_eq!(lang_from_langid(0x0409), Locale::En);
        assert_eq!(lang_from_langid(0x0407), Locale::En); // German: Fettle speaks English then
    }
}
