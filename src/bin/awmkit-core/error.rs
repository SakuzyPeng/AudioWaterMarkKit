use awmkit::app::i18n;
use fluent_bundle::FluentArgs;
use thiserror::Error;

#[derive(Error, Debug)]
/// Internal enum.
pub enum CliError {
    #[error("{0}")]
    /// Internal variant.
    Message(String),

    #[error("key not found; run `awmkit init` or `awmkit key import`")]
    /// Internal variant.
    KeyNotFound,

    #[error("invalid key length: expected {expected} bytes, got {actual}")]
    /// Internal variant.
    InvalidKeyLength {
        /// Internal field.
        expected: usize,
        /// Internal field.
        actual: usize,
    },

    #[error("key store error: {0}")]
    /// Internal variant.
    KeyStore(String),

    #[error("audiowmark not found; use --audiowmark <PATH> or add to PATH")]
    /// Internal variant.
    AudiowmarkNotFound,

    #[error("input not found: {0}")]
    /// Internal variant.
    InputNotFound(String),

    #[error("invalid glob pattern: {0}")]
    /// Internal variant.
    InvalidGlob(String),

    #[error("glob error: {0}")]
    /// Internal variant.
    Glob(String),

    #[error("mapping exists for {0}")]
    /// Internal variant.
    MappingExists(String),

    #[error(transparent)]
    /// Internal variant.
    Io(#[from] std::io::Error),

    #[error(transparent)]
    /// Internal variant.
    Hex(#[from] hex::FromHexError),

    #[error(transparent)]
    /// Internal variant.
    Awmkit(#[from] awmkit::Error),

    #[error(transparent)]
    /// Internal variant.
    Json(#[from] serde_json::Error),
}

/// Internal type alias.
pub type Result<T> = std::result::Result<T, CliError>;

impl From<awmkit::app::Failure> for CliError {
    fn from(err: awmkit::app::Failure) -> Self {
        use awmkit::app::Failure;
        match err {
            Failure::Message(msg) => Self::Message(msg),
            Failure::KeyNotFound => Self::KeyNotFound,
            Failure::InvalidKeyLength { expected, actual } => {
                Self::InvalidKeyLength { expected, actual }
            }
            Failure::KeyStore(msg) => Self::KeyStore(msg),
            Failure::MappingExists {
                username,
                existing_tag: _,
            } => Self::MappingExists(username),
            Failure::Io(err) => Self::Io(err),
            Failure::Json(err) => Self::Json(err),
            Failure::Sqlite(err) => Self::Message(err.to_string()),
            Failure::Awmkit(err) => Self::Awmkit(err),
            Failure::TomlDe(err) => Self::Message(err.to_string()),
            Failure::TomlSer(err) => Self::Message(err.to_string()),
        }
    }
}

impl CliError {
    /// Internal helper method.
    pub fn user_message(&self) -> String {
        match self {
            Self::Message(msg) => msg.clone(),
            Self::KeyNotFound => i18n::tr("cli-error-key_not_found"),
            Self::InvalidKeyLength { expected, actual } => {
                let mut args = FluentArgs::new();
                args.set("expected", expected.to_string());
                args.set("actual", actual.to_string());
                i18n::tr_args("cli-error-invalid_key_length", &args)
            }
            Self::KeyStore(msg) => {
                let mut args = FluentArgs::new();
                args.set("error", msg.as_str());
                i18n::tr_args("cli-error-key_store", &args)
            }
            Self::AudiowmarkNotFound => i18n::tr("cli-error-audiowmark_not_found"),
            Self::InputNotFound(path) => {
                let mut args = FluentArgs::new();
                args.set("path", path.as_str());
                i18n::tr_args("cli-error-input_not_found", &args)
            }
            Self::InvalidGlob(pattern) => {
                let mut args = FluentArgs::new();
                args.set("pattern", pattern.as_str());
                i18n::tr_args("cli-error-invalid_glob", &args)
            }
            Self::Glob(error) => {
                let mut args = FluentArgs::new();
                args.set("error", error.as_str());
                i18n::tr_args("cli-error-glob", &args)
            }
            Self::MappingExists(username) => {
                let mut args = FluentArgs::new();
                args.set("username", username.as_str());
                i18n::tr_args("cli-error-mapping_exists", &args)
            }
            Self::Io(err) => err.to_string(),
            Self::Hex(err) => err.to_string(),
            Self::Awmkit(err) => err.to_string(),
            Self::Json(err) => err.to_string(),
        }
    }
}
