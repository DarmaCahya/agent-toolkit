#![forbid(unsafe_code)]

//! # agent-canvas
//!
//! Pure-Rust visual computing & calculation artifacts for AI agents.
//! Zero external runtimes (no Node.js, no Chromium, no Python).
//!
//! ## Modules
//! - `canvas`: Core geometric primitives and deterministic SVG writer.
//! - `graph`: AI-forgiving flowchart and architecture diagram layouter.
//! - `chart`: Executive business charts (Donut, Quadrant, Funnel, Bar/Line).
//! - `math`: LaTeX math typesetting and Excel formula logic evaluator.
//! - `component`: QR Code vector generator and KPI metric cards.

pub mod canvas;
pub mod error;
pub mod math;

pub use canvas::{
    Color, Fill, FontMetrics, Palette, Path, PathCommand, Point, Rect, Size, Stroke, SvgCanvas,
    ThemeMode,
};
pub use error::{CanvasError, Result};
pub use math::{calculate_slice_angles, CategoryScale, LinearScale, TickGenerator};
