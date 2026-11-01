use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum PdfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to read/extract PDF: {0}")]
    Read(String),

    #[error("Failed to rasterize PDF: {0}")]
    Render(String),

    #[error("Failed to generate PDF: {0}")]
    Generate(String),

    #[error("Invalid page number: requested page {page}, document has {total} pages (pages are 1-indexed)")]
    InvalidPage { page: usize, total: usize },

    #[error("PDF document is encrypted/password-protected: {0}")]
    Encrypted(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Safety limit exceeded: {0}")]
    LimitExceeded(String),
}

pub type Result<T> = std::result::Result<T, PdfError>;
