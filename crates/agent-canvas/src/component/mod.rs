#![forbid(unsafe_code)]

//! Business components with pure vector output.
//!
//! - [`qrcode`]: ISO/IEC 18004 QR Code generator (versions 1..=10, ECC L/M/Q/H).
//! - [`kpi_card`]: Executive KPI metric card with a semantic delta badge.

pub mod kpi_card;
pub mod qrcode;
mod text;

pub use kpi_card::{DeltaTrend, KpiCard};
pub use qrcode::{EccLevel, QrCode, QrError, QrMode};
