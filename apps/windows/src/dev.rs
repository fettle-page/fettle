//! Development switches: a snapshot of the app's own window, drawn off screen and never focused,
//! so screenshots never show (or take keystrokes from) whatever else is on the screen.
//!
//!   fettle --dev-shot <png> [--dev-wait <ms>] [--dev-check] [--dev-data <dir>]
//!
//! `--dev-check` starts a check at launch; `--dev-data` keeps settings and the last check in
//! another folder than %LOCALAPPDATA%\Fettle.
//!
//!   fettle --dev-fps <txt> [--dev-check] [--dev-data <dir>]
//!   fettle --dev-fps-live - [--dev-check] [--dev-data <dir>]   (on screen; see measure_live)
//!
//! measures smoothness: off screen, it scrolls the receipt and writes how long each full frame takes
//! to draw (with FETTLE_RENDERER choosing the renderer).

use slint::ComponentHandle;
use std::time::Duration;

#[derive(Default)]
pub struct DevArgs {
    pub shot: Option<String>,
    pub wait_ms: u64,
    pub check: bool,
    pub data: Option<String>,
    /// A panel view to open at launch: settings, share, links or preview (after --dev-check).
    pub open: Option<String>,
    /// Where to write the frame times of a scrolling run.
    pub fps: Option<String>,
    /// An on-screen scrolling run, never focused; Slint's
    /// SLINT_DEBUG_PERFORMANCE counts the frames.
    pub fps_live: Option<String>,
}

impl DevArgs {
    pub fn parse(args: impl Iterator<Item = String>) -> DevArgs {
        let mut out = DevArgs { wait_ms: 1500, ..DevArgs::default() };
        let mut args = args.peekable();
        while let Some(a) = args.next() {
            match a.as_str() {
                "--dev-shot" => out.shot = args.next(),
                "--dev-wait" => out.wait_ms = args.next().and_then(|v| v.parse().ok()).unwrap_or(1500),
                "--dev-check" => out.check = true,
                "--dev-data" => out.data = args.next(),
                "--dev-open" => out.open = args.next(),
                "--dev-fps" => out.fps = args.next(),
                "--dev-fps-live" => out.fps_live = args.next(),
                _ => {}
            }
        }
        out
    }

    /// Whether the window should open off screen and without taking the focus.
    pub fn hidden(&self) -> bool {
        self.shot.is_some() || self.fps.is_some()
    }

    /// Whether the window should open on screen but never take the focus.
    pub fn unfocused(&self) -> bool {
        self.fps_live.is_some()
    }
}

/// After the receipt is in, scrolls the mat in 60 steps and draws the whole window at each one (an
/// off-screen window gets no paint requests, so each frame is drawn on demand), then writes how long
/// a full frame took to draw to `path` and quits. Measures what a frame costs the renderer.
pub fn measure_scrolling(window: &crate::AppWindow, path: String) {
    use std::time::Instant;
    let weak = window.as_weak();
    slint::Timer::single_shot(Duration::from_millis(3500), move || {
        let Some(window) = weak.upgrade() else { return };
        let mut ms = Vec::new();
        for step in 0..60 {
            let phase = step as f32 / 60.0;
            let y = if phase < 0.5 { phase * 2.0 } else { (1.0 - phase) * 2.0 } * 1200.0;
            window.set_mat_scroll(-y);
            let t = Instant::now();
            let _ = window.window().take_snapshot();
            ms.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        ms.sort_by(f64::total_cmp);
        let at = |q: f64| ms.get(((ms.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0.0);
        let size = window.window().size();
        let text = format!(
            "renderer {}\nwindow {}x{} px\nfull frame ms: median {:.1}, p95 {:.1}, max {:.1} (60 frames a second needs under 16.7)\n",
            std::env::var("FETTLE_RENDERER").unwrap_or_else(|_| "default".into()),
            size.width, size.height, at(0.5), at(0.95), at(1.0),
        );
        let _ = std::fs::write(&path, text);
        let _ = window.hide();
        let _ = slint::quit_event_loop();
    });
}

/// On screen (never focused), after the receipt is in: scrolls the mat up and down for six seconds,
/// then quits. Run it with SLINT_DEBUG_PERFORMANCE=refresh_lazy,console and Slint prints the frames
/// a second it really drew (only what changed, as on any screen).
pub fn measure_live(window: &crate::AppWindow, _path: String) {
    use slint::{Timer, TimerMode};
    use std::rc::Rc;
    use std::time::Instant;
    let weak = window.as_weak();
    let scroller = Rc::new(Timer::default());
    let sc = scroller.clone();
    Timer::single_shot(Duration::from_millis(3500), move || {
        let start = Instant::now();
        let w = weak.clone();
        sc.start(TimerMode::Repeated, Duration::from_millis(8), move || {
            let Some(window) = w.upgrade() else { return };
            let t = start.elapsed().as_secs_f64();
            if t > 6.0 {
                let _ = window.hide();
                let _ = slint::quit_event_loop();
                return;
            }
            // Down 1200 px and back, every two seconds.
            let phase = (t % 2.0) / 2.0;
            let y = if phase < 0.5 { phase * 2.0 } else { (1.0 - phase) * 2.0 } * 1200.0;
            window.set_mat_scroll(-(y as f32));
        });
    });
    std::mem::forget(scroller);
}

/// After `wait_ms`, saves the window's own rendering as a PNG and quits.
pub fn schedule_shot(window: &crate::AppWindow, path: String, wait_ms: u64) {
    let weak = window.as_weak();
    slint::Timer::single_shot(Duration::from_millis(wait_ms), move || {
        let Some(window) = weak.upgrade() else { return };
        if let Ok(px) = window.window().take_snapshot() {
            let _ = image::save_buffer(&path, px.as_bytes(), px.width(), px.height(), image::ExtendedColorType::Rgba8);
        }
        let _ = window.hide();
        let _ = slint::quit_event_loop();
    });
}
