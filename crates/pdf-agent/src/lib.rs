#![forbid(unsafe_code)]

pub mod error;
pub mod types;
pub(crate) mod sandbox;

#[cfg(feature = "writer")]
pub mod generator;

#[cfg(feature = "render")]
pub mod rasterizer;

#[cfg(feature = "reader")]
pub mod reader;

// Re-export high-level public APIs only.
// Upstream crates (pdf_oxide, hayro, fulgur) are strictly encapsulated.
pub use error::{PdfError, Result};
pub use types::{PageImage, PdfReadOptions, PdfReadResult};

#[cfg(feature = "writer")]
pub use generator::PdfGenerator;

#[cfg(feature = "render")]
pub use rasterizer::PdfRasterizer;

#[cfg(feature = "reader")]
pub use reader::PdfReader;

// Compile-time guarantee: All public types MUST be Send + Sync for safe multi-agent concurrency.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PdfError>();
    assert_send_sync::<PageImage>();
    assert_send_sync::<PdfReadResult>();
    assert_send_sync::<PdfReadOptions>();
    #[cfg(feature = "reader")]
    assert_send_sync::<PdfReader>();
    #[cfg(feature = "render")]
    assert_send_sync::<PdfRasterizer>();
    #[cfg(feature = "writer")]
    assert_send_sync::<PdfGenerator>();
};
