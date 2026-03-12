use thiserror::Error;

#[derive(Error, Debug)]
pub enum TuiError {
    #[error("TUI error: {0}")]
    Error(String),

    #[error("Terminal error: {0}")]
    TerminalError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TuiError>;
