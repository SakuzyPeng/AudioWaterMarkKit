//! AWMKit - Audio Watermark Kit
//!
//! 跨语言音频水印消息编解码库，提供 128-bit 自描述、可验证的水印消息格式。
//!
//! # 消息格式
//!
//! ```text
//! ┌──────────┬────────────┬──────────────────┬────────────┐
//! │ Version  │ Timestamp  │  UserTagPacked   │   HMAC     │
//! │  1 byte  │  4 bytes   │    5 bytes       │  6 bytes   │
//! └──────────┴────────────┴──────────────────┴────────────┘
//!               总计: 16 bytes = 128 bit
//! ```
//!
//! # Example
//!
//! ```
//! use awmkit::{Tag, Message};
//!
//! let key = b"your-32-byte-secret-key-here!!!!";
//!
//! // 创建 Tag
//! let tag = Tag::new("SAKUZY").unwrap();
//! println!("Tag: {}", tag);  // SAKUZY_X (含校验位)
//!
//! // 编码消息
//! let msg = Message::encode(1, &tag, key).unwrap();
//! assert_eq!(msg.len(), 16);
//!
//! // 解码消息
//! let result = Message::decode(&msg, key).unwrap();
//! println!("Identity: {}", result.identity());
//! println!("Time: {}", result.timestamp_utc);
//! ```

pub mod charset;
pub mod error;
pub mod message;
pub mod tag;
pub mod audio;

#[cfg(feature = "multichannel")]
pub mod multichannel;

#[cfg(feature = "ffi")]
pub mod ffi;

// Re-exports
pub use error::{Error, Result};
pub use message::MessageResult;
pub use tag::Tag;
pub use audio::{Audio, DetectResult};

#[cfg(feature = "multichannel")]
pub use multichannel::{ChannelLayout, MultichannelAudio, SampleFormat};

#[cfg(feature = "multichannel")]
pub use audio::MultichannelDetectResult;

/// 消息操作的便捷入口
pub struct Message;

impl Message {
    /// 编码消息
    ///
    /// # Arguments
    /// - `version`: 协议版本 (当前为 1)
    /// - `tag`: Tag 引用
    /// - `key`: HMAC 密钥
    ///
    /// # Returns
    /// 16 bytes 消息
    pub fn encode(version: u8, tag: &Tag, key: &[u8]) -> Result<[u8; 16]> {
        message::encode(version, tag, key)
    }

    /// 编码消息（指定时间戳）
    pub fn encode_with_timestamp(
        version: u8,
        tag: &Tag,
        key: &[u8],
        timestamp_minutes: u32,
    ) -> Result<[u8; 16]> {
        message::encode_with_timestamp(version, tag, key, timestamp_minutes)
    }

    /// 解码消息
    pub fn decode(data: &[u8], key: &[u8]) -> Result<MessageResult> {
        message::decode(data, key)
    }

    /// 仅验证 HMAC
    pub fn verify(data: &[u8], key: &[u8]) -> bool {
        message::verify(data, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api() {
        let key = b"test-key-32-bytes-for-hmac-test!";

        let tag = Tag::new("SAKUZY").unwrap();
        let msg = Message::encode(1, &tag, key).unwrap();
        let result = Message::decode(&msg, key).unwrap();

        assert_eq!(result.identity(), "SAKUZY");
        assert_eq!(result.version, 1);
    }
}
