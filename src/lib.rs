//! AWMKit - Audio Watermark Kit
//!
//! 跨语言音频水印消息编解码库，提供 128-bit 自描述、可验证的水印消息格式。
//!
//! # 消息格式
//!
//! ```text
//! ┌──────────┬────────────┬──────────────────┬────────────┐
//! │ Version  │ Time+Slot  │  UserTagPacked   │   HMAC     │
//! │  1 byte  │  4 bytes   │    5 bytes       │  6 bytes   │
//! └──────────┴────────────┴──────────────────┴────────────┘
//!               总计: 16 bytes = 128 bit
//! ```
//!
//! # Example
//!
//! ```
//! use awmkit::{Tag, Message, MESSAGE_LEN, CURRENT_VERSION};
//!
//! let key = b"your-secret-key!"; // 任意长度，推荐 16 或 32 字节
//!
//! // 创建 Tag
//! let tag = Tag::new("SAKUZY").unwrap();
//! println!("Tag: {}", tag);  // SAKUZY_X (含校验位)
//!
//! // 编码消息
//! let msg = Message::encode(CURRENT_VERSION, &tag, key).unwrap();
//! assert_eq!(msg.len(), MESSAGE_LEN);  // 16 bytes
//!
//! // 解码消息
//! let result = Message::decode(&msg, key).unwrap();
//! println!("Identity: {}", result.identity());
//! println!("Time: {}", result.timestamp_utc);
//! ```

pub mod audio;
pub(crate) mod bundled;
pub mod charset;
pub mod error;
pub mod message;
pub mod tag;

#[cfg(feature = "multichannel")]
pub mod multichannel;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "app")]
pub mod app;

// Re-exports
pub use audio::{Audio, DetectResult};
pub use error::{Error, Result};
pub use message::{MessageResult, CURRENT_VERSION, MESSAGE_LEN};
pub use tag::Tag;

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
    /// - `version`: 协议版本 (当前为 2)
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

    /// 编码消息（指定时间戳 + 槽位）
    pub fn encode_with_timestamp_and_slot(
        version: u8,
        tag: &Tag,
        key: &[u8],
        timestamp_minutes: u32,
        key_slot: u8,
    ) -> Result<[u8; 16]> {
        message::encode_with_timestamp_and_slot(version, tag, key, timestamp_minutes, key_slot)
    }

    /// 编码消息（当前时间 + 槽位）
    pub fn encode_with_slot(version: u8, tag: &Tag, key: &[u8], key_slot: u8) -> Result<[u8; 16]> {
        message::encode_with_slot(version, tag, key, key_slot)
    }

    /// 解码消息
    pub fn decode(data: &[u8], key: &[u8]) -> Result<MessageResult> {
        message::decode(data, key)
    }

    /// 读取消息中的版本与槽位（不校验 HMAC）
    pub fn peek_version_and_slot(data: &[u8]) -> Result<(u8, u8)> {
        message::peek_version_and_slot(data)
    }

    /// 仅验证 HMAC
    #[must_use]
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

        let tag_result = Tag::new("SAKUZY");
        assert!(tag_result.is_ok());
        let Ok(tag) = tag_result else {
            return;
        };
        let msg_result = Message::encode(CURRENT_VERSION, &tag, key);
        assert!(msg_result.is_ok());
        let Ok(msg) = msg_result else {
            return;
        };
        let decoded_result = Message::decode(&msg, key);
        assert!(decoded_result.is_ok());
        let Ok(result) = decoded_result else {
            return;
        };

        assert_eq!(result.identity(), "SAKUZY");
        assert_eq!(result.version, CURRENT_VERSION);
    }
}
