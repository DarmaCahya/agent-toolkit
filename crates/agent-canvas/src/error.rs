use thiserror::Error;

pub type Result<T> = std::result::Result<T, CanvasError>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum CanvasError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Layout error: {0}")]
    LayoutError(String),

    #[error("Render error: {0}")]
    RenderError(String),
}
