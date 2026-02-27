use awmkit::app::i18n;
use fluent_bundle::FluentArgs;
use thiserror::Error;

/// Rendered user-facing CLI error with optional diagnostic detail.
pub struct RenderedCliError {
    /// User-facing localized message.
    pub user: String,
    /// Optional diagnostics shown in verbose mode only.
    pub detail: Option<String>,
}

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

    #[error("database error")]
    /// Internal variant.
    Database(String),

    #[error("configuration error")]
    /// Internal variant.
    Config(String),

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
            Failure::Sqlite(err) => Self::Database(err.to_string()),
            Failure::Awmkit(err) => Self::Awmkit(err),
            Failure::TomlDe(err) => Self::Config(err.to_string()),
            Failure::TomlSer(err) => Self::Config(err.to_string()),
        }
    }
}

impl CliError {
    /// Internal helper method.
    pub fn render_user_message(&self) -> RenderedCliError {
        let with_detail = |user: String, detail: String| RenderedCliError {
            user,
            detail: Some(detail),
        };

        match self {
            Self::Message(msg) => RenderedCliError {
                user: msg.clone(),
                detail: None,
            },
            Self::KeyNotFound => RenderedCliError {
                user: i18n::tr("cli-error-key_not_found"),
                detail: None,
            },
            Self::InvalidKeyLength { expected, actual } => {
                let mut args = FluentArgs::new();
                args.set("expected", expected.to_string());
                args.set("actual", actual.to_string());
                RenderedCliError {
                    user: i18n::tr_args("cli-error-invalid_key_length", &args),
                    detail: None,
                }
            }
            Self::KeyStore(msg) => {
                let mut args = FluentArgs::new();
                args.set("error", msg.as_str());
                RenderedCliError {
                    user: i18n::tr_args("cli-error-key_store", &args),
                    detail: None,
                }
            }
            Self::AudiowmarkNotFound => RenderedCliError {
                user: i18n::tr("cli-error-audiowmark_not_found"),
                detail: None,
            },
            Self::InputNotFound(path) => {
                let mut args = FluentArgs::new();
                args.set("path", path.as_str());
                RenderedCliError {
                    user: i18n::tr_args("cli-error-input_not_found", &args),
                    detail: None,
                }
            }
            Self::InvalidGlob(pattern) => {
                let mut args = FluentArgs::new();
                args.set("pattern", pattern.as_str());
                RenderedCliError {
                    user: i18n::tr_args("cli-error-invalid_glob", &args),
                    detail: None,
                }
            }
            Self::Glob(error) => {
                let mut args = FluentArgs::new();
                args.set("error", error.as_str());
                RenderedCliError {
                    user: i18n::tr_args("cli-error-glob", &args),
                    detail: None,
                }
            }
            Self::MappingExists(username) => {
                let mut args = FluentArgs::new();
                args.set("username", username.as_str());
                RenderedCliError {
                    user: i18n::tr_args("cli-error-mapping_exists", &args),
                    detail: None,
                }
            }
            Self::Database(err) => with_detail(i18n::tr("cli-error-database"), err.clone()),
            Self::Config(err) => with_detail(i18n::tr("cli-error-config"), err.clone()),
            Self::Io(err) => with_detail(i18n::tr("cli-error-io"), err.to_string()),
            Self::Hex(err) => with_detail(i18n::tr("cli-error-hex"), err.to_string()),
            Self::Awmkit(err) => with_detail(i18n::tr("cli-error-audio"), err.to_string()),
            Self::Json(err) => with_detail(i18n::tr("cli-error-json"), err.to_string()),
        }
    }
}
