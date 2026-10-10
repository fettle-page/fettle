//! Fettle for Windows: the receipt and the side panel drawn natively by Slint, under Windows' own
//! title bar. The scan is fettle-collect; the verdicts and the receipt are fettle-receipt.

slint::include_modules!();

pub mod app;
pub mod dev;
pub mod elevate;
pub mod exports;
pub mod copy;
pub mod scan;
pub mod print;
pub mod sections;
pub mod share;
pub mod state;
pub mod store;
pub mod update;
pub mod view;
pub mod zone;
#[cfg(windows)]
pub mod win;

pub use app::{Deps, Fettle};
