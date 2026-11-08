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

    #[error("Sandbox constraint violation: {0}")]
    Sandbox(String),

    #[error("Invalid PDF stream format: {0}")]
    InvalidPdf(String),
}

impl PdfError {
    /// Returns true if this error represents a recoverable or parameter-tunable condition
    /// (e.g. pagination out-of-bounds, character limits exceeded, or password required).
    pub fn is_retryable(&self) -> bool {
        match self {
            PdfError::LimitExceeded(_)
            | PdfError::Encrypted(_)
            | PdfError::InvalidPage { .. }
            | PdfError::InvalidArgument(_)
            | PdfError::Sandbox(_) => true,
            PdfError::Io(io_err) => {
                matches!(
                    io_err.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::WouldBlock
                )
            }
            PdfError::Read(_)
            | PdfError::Render(_)
            | PdfError::Generate(_)
            | PdfError::InvalidPdf(_) => false,
        }
    }

    /// Returns an agent-centric prompt instruction for autonomous recovery.
    pub fn agent_hint(&self) -> Option<&'static str> {
        match self {
            PdfError::LimitExceeded(_) => {
                Some("Reduce page_limit or use output_file to offload document.")
            }
            PdfError::Encrypted(_) => {
                Some("Document is password-protected. Provide the decryption password in PdfReadOptions.")
            }
            PdfError::InvalidPage { .. } => {
                Some("Requested page is out of bounds. Check page_count() before requesting page ranges.")
            }
            PdfError::InvalidArgument(_) => {
                Some("Invalid argument provided. Verify input parameters and file paths.")
            }
            PdfError::Sandbox(_) => {
                Some("File path violates sandbox constraints. Use relative paths within working directory.")
            }
            PdfError::InvalidPdf(_) | PdfError::Read(_) => {
                Some("Unrecoverable PDF syntax error. Inspect document integrity or provide a different file.")
            }
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, PdfError>;
