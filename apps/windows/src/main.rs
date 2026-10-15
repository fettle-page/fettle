// Fettle for Windows. A window app: no console.
#![windows_subsystem = "windows"]

use fettle_windows::dev::{DevArgs, schedule_shot};
use fettle_windows::{Deps, Fettle};
use slint::ComponentHandle;
use std::rc::Rc;
use std::time::Duration;

/// Skia on the graphics card: as smooth as the screen allows (measured 2026-10-10: ~180 frames a
/// second scrolling on a 180 Hz screen, against ~60 on the processor; ~215 MB against ~70 MB).
const DEFAULT_RENDERER: &str = "skia";

/// The renderer to start again with when `current` can't open the window: Skia on the processor
/// (no graphics card needed), then Slint's own simplest renderer.
fn next_renderer(current: &str) -> Option<&'static str> {
    match current {
        "skia" => Some("skia-software"),
        "skia-software" => Some("software"),
        _ => None,
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // DLLs loaded later by name come from System32 only, like the ones the exe imports
    // (/DEPENDENTLOADFLAG in build.rs), never from the folder fettle.exe sits in: the graphics
    // renderer loads d3d12.dll and others by name, and a fake one beside a portable copy in Downloads
    // would otherwise load into the window. The Visual C++ runtime beside a packaged fettle.exe still
    // loads: its manifest names it.
    #[cfg(windows)]
    // SAFETY: a plain flag for this process, set before any other thread exists.
    unsafe {
        use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories};
        let _ = SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
    }
    // The Deep check's helper (elevated, no window): scan, write the report, done. Asked to write
    // somewhere it won't, it stops too: never the whole app as administrator.
    match fettle_windows::elevate::helper_target(std::env::args()) {
        fettle_windows::elevate::Helper::Write(path) => std::process::exit(fettle_windows::elevate::run_helper(&path)),
        fettle_windows::elevate::Helper::Refuse => std::process::exit(1),
        fettle_windows::elevate::Helper::NotAsked => {}
    }
    let result = run();
    // A process can choose its renderer only once: if the window can't open (no usable graphics
    // card in a virtual machine, a broken driver), start again on the next one down.
    let current = std::env::var("FETTLE_RENDERER").unwrap_or_else(|_| DEFAULT_RENDERER.into());
    if result.as_ref().is_err_and(Stop::tries_another_renderer)
        && let Some(next) = next_renderer(&current)
        && let Ok(exe) = std::env::current_exe()
        && std::process::Command::new(exe).args(std::env::args().skip(1)).env("FETTLE_RENDERER", next).spawn().is_ok()
    {
        return Ok(());
    }
    result.map_err(Stop::into_error)
}

/// Why the app stopped before its window closed.
enum Stop {
    /// The renderer or the window failed: another renderer may do.
    Window(slint::PlatformError),
    /// Something before the window (a development switch's file): another renderer won't help.
    Setup(slint::PlatformError),
}

impl Stop {
    fn tries_another_renderer(&self) -> bool {
        matches!(self, Stop::Window(_))
    }
    fn into_error(self) -> slint::PlatformError {
        match self {
            Stop::Window(e) | Stop::Setup(e) => e,
        }
    }
}

impl From<slint::PlatformError> for Stop {
    fn from(e: slint::PlatformError) -> Stop {
        Stop::Window(e)
    }
}

/// The report `--dev-report` names.
fn dev_report(path: &str) -> Result<fettle_windows::dev::SavedReport, Stop> {
    fettle_windows::dev::SavedReport::load(path).map_err(|e| Stop::Setup(slint::PlatformError::Other(e)))
}

fn run() -> Result<(), Stop> {
    let dev = DevArgs::parse(std::env::args().skip(1));
    select_backend(dev.hidden(), dev.unfocused())?;
    // Deep-check reports a helper left behind (the app stopped waiting, or was closed).
    fettle_windows::elevate::remove_leftovers(&fettle_windows::store::FileStore::default_dir());
    let mut deps = Deps::default();
    if let Some(dir) = &dev.data {
        deps.store = std::rc::Rc::new(fettle_windows::store::FileStore::new(dir.into()));
    }
    if let Some(path) = &dev.report {
        deps.scanner = std::sync::Arc::new(dev_report(path)?);
        // A Deep check that's under way for good: no Windows prompt, never a result.
        if dev.open.as_deref() == Some("deep") {
            deps.deep_scan = std::sync::Arc::new(|_| loop {
                std::thread::sleep(Duration::from_secs(3600));
            });
        }
    }
    let app = Fettle::new(deps)?;
    if let Some((width, height)) = dev.size {
        app.window().window().set_size(slint::LogicalSize::new(width, height));
    }
    tint_title_bar_when_ready(&app);
    // A development check: check straight away.
    if dev.check {
        app.dispatch(fettle_windows::state::Event::Start);
    }
    if let Some(panel) = dev.open.as_deref() {
        use fettle_windows::state::{Event, Panel};
        // The preview opens the way Print does; the others as views.
        let event = match panel {
            "share" => Event::Open(Panel::Share),
            "links" => Event::Open(Panel::Links),
            "preview" => Event::PrintAsked,
            "deep" if dev.report.is_some() => Event::DeepAsked,
            _ => Event::Open(Panel::Settings),
        };
        // After a development check, once its receipt is in.
        if dev.check {
            let app = std::rc::Rc::new(app);
            let opener = app.clone();
            slint::Timer::single_shot(Duration::from_millis(2500), move || opener.dispatch(event));
            if let Some(path) = dev.shot.clone() {
                schedule_shot(app.window(), path, dev.wait_ms);
            }
            return Ok(app.window().run()?);
        }
        app.dispatch(event);
    }
    if let Some(path) = dev.shot.clone() {
        schedule_shot(app.window(), path, dev.wait_ms);
    }
    if let Some(path) = dev.fps.clone() {
        fettle_windows::dev::measure_scrolling(app.window(), path);
    }
    if let Some(path) = dev.fps_live.clone() {
        fettle_windows::dev::measure_live(app.window(), path);
    }
    Ok(app.window().run()?)
}

/// Skia on the graphics card first (FETTLE_RENDERER picks another); `main` restarts on the next
/// one down, Skia on the processor and then the plain software renderer, if the window can't open.
fn select_backend(hidden: bool, unfocused: bool) -> Result<(), slint::PlatformError> {
    let renderer = std::env::var("FETTLE_RENDERER").unwrap_or_else(|_| DEFAULT_RENDERER.into());
    let selector = |renderer: String| {
        let s = slint::BackendSelector::new().backend_name("winit".into()).renderer_name(renderer);
        if hidden {
            // Development snapshots: off screen and never focused.
            s.with_winit_window_attributes_hook(|a| a.with_active(false).with_position(slint::winit_030::winit::dpi::PhysicalPosition::new(-6000, -6000)))
        } else if unfocused {
            // An on-screen measurement: shown, but never taking the focus from the user.
            s.with_winit_window_attributes_hook(|a| a.with_active(false))
        } else {
            s
        }
    };
    selector(renderer).select()
}

/// Windows' own title bar takes the side panel's colour once the window exists.
fn tint_title_bar_when_ready(app: &Fettle) {
    let weak = app.window().as_weak();
    let timer = Rc::new(slint::Timer::default());
    let t = timer.clone();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(5), move || {
        let Some(w) = weak.upgrade() else { return t.stop() };
        let theme = w.global::<fettle_windows::Theme>();
        if fettle_windows::win::tint_title_bar(w.window(), theme.get_dark(), theme.get_oled()) {
            t.stop();
        }
    });
    std::mem::forget(timer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_window_that_wont_open_tries_another_renderer() {
        // A mistyped --dev-report path isn't the graphics card's fault: say so once, don't restart twice.
        let stop = dev_report("no-such-report.json").err().expect("a missing report");
        assert!(!stop.tries_another_renderer());
        assert!(Stop::Window(slint::PlatformError::NoPlatform).tries_another_renderer());
    }

    #[test]
    fn the_graphics_card_first_then_the_processor_then_the_simplest() {
        assert_eq!(DEFAULT_RENDERER, "skia");
        assert_eq!(next_renderer("skia"), Some("skia-software"));
        assert_eq!(next_renderer("skia-software"), Some("software"));
        assert_eq!(next_renderer("software"), None);
        assert_eq!(next_renderer("anything else"), None);
    }
}
