//! The receipt for a report, in English or Italian: the model, details, formats and markdown,
//! pinned by the parity corpus in `tests/parity`.

pub mod copy;
pub mod details;
pub mod format;
mod markdown;
pub mod memory;
mod model;

pub use copy::ui;
pub use details::{DetailGroup, DetailRow};
pub use format::Zone;
pub use markdown::markdown;
pub use model::{Labels, Meta, ModelLine, ModelSection, ReceiptModel, build};

use fettle_facts::Report;
use fettle_verdicts::{Locale, Receipt};

/// The verdicts and the receipt for a report, evaluated at its own scan time. Refuses a scan time
/// that doesn't exist (as the TypeScript engine does): age rules would silently flip.
pub fn receipt_for(report: &Report, locale: Locale, zone: Zone) -> Result<(Receipt, ReceiptModel), String> {
    let now = fettle_verdicts::fmt::parse_rfc3339(&report.collected_at).ok_or_else(|| format!("not a valid scan time: {}", report.collected_at))?;
    let receipt = fettle_verdicts::evaluate(&report.facts, now, report.elevated);
    let model = build(report, &receipt, locale, zone);
    Ok((receipt, model))
}
