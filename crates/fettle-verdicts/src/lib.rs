//! Turns a report's facts into verdict lines. Fettle's only verdict engine since 2026-10-10 (it began
//! as a port of the TypeScript one); the app and the website (as WebAssembly) both run it, and the
//! parity corpus in `crates/fettle-receipt/tests/parity` pins what it says.

pub mod fmt;
pub mod lifecycle;
mod locale;
mod render;
mod rules;
mod summary;
pub mod thresholds;
mod types;

pub use locale::Locale;
pub use render::{fill, render, template};
pub use summary::summarize;
pub use types::{Level, Line, MessageRef, Param, Receipt, SectionId, SectionVerdict, Summary};

use fettle_facts::Facts;

/// Recorded in each report, so a shared receipt can show the verdicts the user saw.
pub const ENGINE_VERSION: &str = "1.0.0";

/// Turns raw facts into a receipt. Pure: the time (ms since 1970, for age rules) and whether the
/// scan ran as administrator come in as arguments.
pub fn evaluate(facts: &Facts, now_ms: i64, elevated: bool) -> Receipt {
    let mut sections = vec![rules::storage::storage_rules(facts.storage.as_ref(), elevated)];
    sections.extend(rules::others::battery_rules(facts.battery.as_ref()));
    sections.push(rules::others::memory_rules(facts.memory.as_ref()));
    sections.push(rules::windows::windows_rules(facts.windows.as_ref(), now_ms, elevated, facts.system.as_ref().and_then(|s| s.data.as_ref()).and_then(|d| d.cpu_name.as_deref())));
    sections.push(rules::others::startup_rules(facts.startup.as_ref()));
    sections.push(rules::others::graphics_rules(facts.graphics.as_ref(), now_ms));
    Receipt { summary: summary::summarize(&sections), sections, engine_version: ENGINE_VERSION.into() }
}
