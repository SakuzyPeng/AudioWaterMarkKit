use awmkit::app::i18n;
use fluent_bundle::FluentArgs;
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

    #[error("mapping exists for {0}")]
    MappingExists(String),

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

impl From<awmkit::app::AppError> for CliError {
    fn from(err: awmkit::app::AppError) -> Self {
        use awmkit::app::AppError;
        match err {
            AppError::Message(msg) => CliError::Message(msg),
            AppError::KeyNotFound => CliError::KeyNotFound,
            AppError::InvalidKeyLength { expected, actual } => {
                CliError::InvalidKeyLength { expected, actual }
            }
            AppError::KeyStore(msg) => CliError::KeyStore(msg),
            AppError::MappingExists {
                username,
                existing_tag: _,
            } => CliError::MappingExists(username),
            AppError::Io(err) => CliError::Io(err),
            AppError::Json(err) => CliError::Json(err),
            AppError::Awmkit(err) => CliError::Awmkit(err),
            AppError::TomlDe(err) => CliError::Message(err.to_string()),
            AppError::TomlSer(err) => CliError::Message(err.to_string()),
        }
    }
}

impl CliError {
    pub fn user_message(&self) -> String {
        match self {
            CliError::Message(msg) => msg.clone(),
            CliError::KeyNotFound => i18n::tr("cli-error-key_not_found"),
            CliError::InvalidKeyLength { expected, actual } => {
                let mut args = FluentArgs::new();
                args.set("expected", expected.to_string());
                args.set("actual", actual.to_string());
                i18n::tr_args("cli-error-invalid_key_length", &args)
            }
            CliError::KeyStore(msg) => {
                let mut args = FluentArgs::new();
                args.set("error", msg.as_str());
                i18n::tr_args("cli-error-key_store", &args)
            }
            CliError::AudiowmarkNotFound => i18n::tr("cli-error-audiowmark_not_found"),
            CliError::InputNotFound(path) => {
                let mut args = FluentArgs::new();
                args.set("path", path.as_str());
                i18n::tr_args("cli-error-input_not_found", &args)
            }
            CliError::InvalidGlob(pattern) => {
                let mut args = FluentArgs::new();
                args.set("pattern", pattern.as_str());
                i18n::tr_args("cli-error-invalid_glob", &args)
            }
            CliError::Glob(error) => {
                let mut args = FluentArgs::new();
                args.set("error", error.as_str());
                i18n::tr_args("cli-error-glob", &args)
            }
            CliError::MappingExists(username) => {
                let mut args = FluentArgs::new();
                args.set("username", username.as_str());
                i18n::tr_args("cli-error-mapping_exists", &args)
            }
            CliError::Io(err) => err.to_string(),
            CliError::Hex(err) => err.to_string(),
            CliError::Awmkit(err) => err.to_string(),
            CliError::Json(err) => err.to_string(),
        }
    }
}
