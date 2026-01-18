//! 错误类型定义

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid character '{0}' in tag")]
    InvalidChar(char),

    #[error("Tag must be exactly 8 characters, got {0}")]
    InvalidTagLength(usize),

    #[error("Identity must be 1-7 characters, got {0}")]
    InvalidIdentityLength(usize),

    #[error("Tag checksum mismatch: expected '{expected}', got '{got}'")]
    ChecksumMismatch { expected: char, got: char },

    #[error("Message must be exactly 16 bytes, got {0}")]
    InvalidMessageLength(usize),

    #[error("HMAC verification failed")]
    HmacMismatch,

    #[error("Unsupported message version: {0}")]
    UnsupportedVersion(u8),
}

pub type Result<T> = std::result::Result<T, Error>;
