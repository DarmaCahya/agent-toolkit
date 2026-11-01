pub mod error;
pub mod types;

#[cfg(feature = "writer")]
pub mod generator;

#[cfg(feature = "render")]
pub mod rasterizer;

#[cfg(feature = "reader")]
pub mod reader;

// Re-export high-level public APIs only.
// Upstream crates (pdf_oxide, hayro, fulgur) are strictly encapsulated.
pub use error::{PdfError, Result};
pub use types::*;

#[cfg(feature = "writer")]
pub use generator::PdfGenerator;

#[cfg(feature = "render")]
pub use rasterizer::PdfRasterizer;

#[cfg(feature = "reader")]
pub use reader::PdfReader;
