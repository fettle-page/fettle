//! The receipt on paper: one layout, drawn into a PDF (krilla) or onto a printer (GDI).

#[cfg(windows)]
pub mod gdi;
pub mod layout;
pub mod pdf;
