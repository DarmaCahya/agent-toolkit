pub mod error;
pub mod generator;
pub mod rasterizer;
pub mod reader;
pub mod types;

// Re-export high-level public APIs only.
// Upstream crates (pdf_oxide, hayro, fulgur) are strictly encapsulated.
pub use error::{PdfError, Result};
pub use generator::PdfGenerator;
pub use rasterizer::PdfRasterizer;
pub use reader::PdfReader;
pub use types::PageImage;
