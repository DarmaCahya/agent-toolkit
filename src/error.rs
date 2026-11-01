use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to read/extract PDF: {0}")]
    Read(String),

    #[error("Failed to rasterize PDF: {0}")]
    Render(String),

    #[error("Failed to generate PDF: {0}")]
    Generate(String),

    #[error("Invalid page index: requested {page}, total pages {total}")]
    InvalidPage { page: usize, total: usize },
}

pub type Result<T> = std::result::Result<T, PdfError>;
