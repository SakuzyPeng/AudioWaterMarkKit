use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("{0}")]
    Message(String),

    #[error("key not found; run `awmkit init` or `awmkit key import`")]
    KeyNotFound,

    #[error("invalid key length: expected {expected} bytes, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("key store error: {0}")]
    KeyStore(String),

    #[error("audiowmark not found; use --audiowmark <PATH> or add to PATH")]
    AudiowmarkNotFound,

    #[error("input not found: {0}")]
    InputNotFound(String),

    #[error("invalid glob pattern: {0}")]
    InvalidGlob(String),

    #[error("glob error: {0}")]
    Glob(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Hex(#[from] hex::FromHexError),

    #[error(transparent)]
    Awmkit(#[from] awmkit::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CliError>;
