use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),

    #[error("key not found")]
    KeyNotFound,

    #[error("invalid key length: expected {expected} bytes, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("key store error: {0}")]
    KeyStore(String),

    #[error("mapping exists for {username} (current tag: {existing_tag})")]
    MappingExists {
        username: String,
        existing_tag: String,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    Awmkit(#[from] crate::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
