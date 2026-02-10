//! 消息编解码
//!
//! 消息格式 (16 bytes = 128 bit):
//! - Version: 1 byte
//! - Timestamp+Slot: 4 bytes (v1: UTC Unix minutes, v2: 27-bit minutes + 5-bit key_slot)
//! - UserTagPacked: 5 bytes (8 × 5bit)
//! - HMAC: 6 bytes (HMAC-SHA256 前 6 字节)

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{Error, Result};
use crate::tag::Tag;

type HmacSha256 = Hmac<Sha256>;

/// 消息长度 (bytes)
pub const MESSAGE_LEN: usize = 16;

/// HMAC 长度 (bytes)
pub const HMAC_LEN: usize = 6;

/// 当前协议版本
pub const CURRENT_VERSION: u8 = 2;

const VERSION_V1: u8 = 1;
const VERSION_V2: u8 = 2;
const KEY_SLOT_BITS: u32 = 5;
const KEY_SLOT_MASK: u32 = (1 << KEY_SLOT_BITS) - 1;
const MAX_TIMESTAMP_V2_MINUTES: u32 = (1 << (32 - KEY_SLOT_BITS)) - 1;
const DEFAULT_KEY_SLOT: u8 = 0;

/// 解码后的消息结果
#[derive(Debug, Clone)]
pub struct MessageResult {
    /// 协议版本
    pub version: u8,
    /// UTC Unix 时间戳 (秒)
    pub timestamp_utc: u64,
    /// UTC Unix 分钟数 (原始值)
    pub timestamp_minutes: u32,
    /// 密钥槽位 (v1 固定为 0，v2 为 0-31)
    pub key_slot: u8,
    /// 解码的 Tag
    pub tag: Tag,
}

impl MessageResult {
    /// 获取身份字符串
    #[must_use]
    pub fn identity(&self) -> &str {
        self.tag.identity()
    }
}

/// 编码消息
///
/// # Arguments
/// - `version`: 协议版本 (当前为 2)
/// - `tag`: 8 字符 Tag
/// - `key`: HMAC 密钥 (建议 32 bytes)
///
/// # Returns
/// 16 bytes 消息
pub fn encode(version: u8, tag: &Tag, key: &[u8]) -> Result<[u8; MESSAGE_LEN]> {
    encode_with_timestamp_and_slot(version, tag, key, current_utc_minutes(), DEFAULT_KEY_SLOT)
}

/// 编码消息（指定时间戳）
pub fn encode_with_timestamp(
    version: u8,
    tag: &Tag,
    key: &[u8],
    timestamp_minutes: u32,
) -> Result<[u8; MESSAGE_LEN]> {
    encode_with_timestamp_and_slot(version, tag, key, timestamp_minutes, DEFAULT_KEY_SLOT)
}

/// 编码消息（指定时间戳 + 槽位）
pub fn encode_with_timestamp_and_slot(
    version: u8,
    tag: &Tag,
    key: &[u8],
    timestamp_minutes: u32,
    key_slot: u8,
) -> Result<[u8; MESSAGE_LEN]> {
    let mut msg = [0u8; MESSAGE_LEN];

    // Version (1 byte)
    msg[0] = version;

    let packed_timestamp = match version {
        VERSION_V1 => {
            if key_slot != DEFAULT_KEY_SLOT {
                return Err(Error::InvalidInput(
                    "v1 does not support non-zero key_slot".to_string(),
                ));
            }
            timestamp_minutes
        }
        VERSION_V2 => pack_timestamp_v2(timestamp_minutes, key_slot)?,
        _ => return Err(Error::UnsupportedVersion(version)),
    };

    // Timestamp/slot (4 bytes, big-endian)
    msg[1..5].copy_from_slice(&packed_timestamp.to_be_bytes());

    // TagPacked (5 bytes)
    msg[5..10].copy_from_slice(&tag.to_packed());

    // HMAC (6 bytes)
    let mac = compute_hmac(key, &msg[..10]);
    msg[10..16].copy_from_slice(&mac);

    Ok(msg)
}

/// 编码消息（使用当前时间戳 + 指定槽位）
pub fn encode_with_slot(
    version: u8,
    tag: &Tag,
    key: &[u8],
    key_slot: u8,
) -> Result<[u8; MESSAGE_LEN]> {
    encode_with_timestamp_and_slot(version, tag, key, current_utc_minutes(), key_slot)
}

/// 解码消息
///
/// # Arguments
/// - `data`: 16 bytes 消息
/// - `key`: HMAC 密钥
///
/// # Returns
/// 解码结果，HMAC 验证失败返回错误
pub fn decode(data: &[u8], key: &[u8]) -> Result<MessageResult> {
    if data.len() != MESSAGE_LEN {
        return Err(Error::InvalidMessageLength(data.len()));
    }

    // 验证 HMAC
    let expected_mac = compute_hmac(key, &data[..10]);
    if !constant_time_eq(&data[10..16], &expected_mac) {
        return Err(Error::HmacMismatch);
    }

    // 解析字段
    let version = data[0];

    // SAFETY: 已验证 data.len() == 16，切片长度固定为 4
    let packed_timestamp_bytes: [u8; 4] = data[1..5]
        .try_into()
        .map_err(|_| Error::InvalidMessageLength(data.len()))?;
    let packed_timestamp = u32::from_be_bytes(packed_timestamp_bytes);
    let (timestamp_minutes, key_slot) = match version {
        VERSION_V1 => (packed_timestamp, DEFAULT_KEY_SLOT),
        VERSION_V2 => unpack_timestamp_v2(packed_timestamp),
        _ => return Err(Error::UnsupportedVersion(version)),
    };

    let mut tag_packed = [0u8; 5];
    tag_packed.copy_from_slice(&data[5..10]);
    let tag = Tag::from_packed(&tag_packed)?;

    Ok(MessageResult {
        version,
        timestamp_utc: u64::from(timestamp_minutes) * 60,
        timestamp_minutes,
        key_slot,
        tag,
    })
}

/// 仅验证 HMAC（不解析内容）
#[must_use]
pub fn verify(data: &[u8], key: &[u8]) -> bool {
    if data.len() != MESSAGE_LEN {
        return false;
    }

    let expected_mac = compute_hmac(key, &data[..10]);
    constant_time_eq(&data[10..16], &expected_mac)
}

/// 读取消息头中的版本与槽位（不校验 HMAC）
pub fn peek_version_and_slot(data: &[u8]) -> Result<(u8, u8)> {
    if data.len() != MESSAGE_LEN {
        return Err(Error::InvalidMessageLength(data.len()));
    }
    let version = data[0];
    let packed_timestamp_bytes: [u8; 4] = data[1..5]
        .try_into()
        .map_err(|_| Error::InvalidMessageLength(data.len()))?;
    let packed_timestamp = u32::from_be_bytes(packed_timestamp_bytes);
    let key_slot = match version {
        VERSION_V1 => DEFAULT_KEY_SLOT,
        VERSION_V2 => unpack_timestamp_v2(packed_timestamp).1,
        _ => return Err(Error::UnsupportedVersion(version)),
    };
    Ok((version, key_slot))
}

/// 计算 HMAC-SHA256 并截取前 6 字节
fn compute_hmac(key: &[u8], data: &[u8]) -> [u8; HMAC_LEN] {
    // HMAC-SHA256 接受任意长度密钥，new_from_slice 不会失败
    #[allow(clippy::expect_used)]
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    let result = mac.finalize().into_bytes();

    let mut out = [0u8; HMAC_LEN];
    out.copy_from_slice(&result[..HMAC_LEN]);
    out
}

/// 常量时间比较
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn pack_timestamp_v2(timestamp_minutes: u32, key_slot: u8) -> Result<u32> {
    if timestamp_minutes > MAX_TIMESTAMP_V2_MINUTES {
        return Err(Error::InvalidInput(format!(
            "timestamp_minutes out of range for v2: {timestamp_minutes} > {MAX_TIMESTAMP_V2_MINUTES}"
        )));
    }
    if u32::from(key_slot) > KEY_SLOT_MASK {
        return Err(Error::InvalidInput(format!(
            "key_slot out of range for v2: {key_slot} > {KEY_SLOT_MASK}"
        )));
    }
    Ok((timestamp_minutes << KEY_SLOT_BITS) | u32::from(key_slot))
}

const fn unpack_timestamp_v2(packed_timestamp: u32) -> (u32, u8) {
    let timestamp_minutes = packed_timestamp >> KEY_SLOT_BITS;
    #[allow(clippy::cast_possible_truncation)]
    let key_slot = (packed_timestamp & KEY_SLOT_MASK) as u8;
    (timestamp_minutes, key_slot)
}

/// 获取当前 UTC Unix 分钟数
fn current_utc_minutes() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    // 系统时间在 UNIX_EPOCH 之后是合理假设
    #[allow(clippy::unwrap_used)]
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    #[allow(clippy::cast_possible_truncation)]
    let minutes = (secs / 60) as u32;
    minutes
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const TEST_KEY: &[u8] = b"test-key-32-bytes-for-hmac-test!";

    #[test]
    fn test_encode_decode() {
        let tag = Tag::new("SAKUZY").unwrap();
        let msg = encode(CURRENT_VERSION, &tag, TEST_KEY).unwrap();

        assert_eq!(msg.len(), 16);

        let result = decode(&msg, TEST_KEY).unwrap();
        assert_eq!(result.version, CURRENT_VERSION);
        assert_eq!(result.identity(), "SAKUZY");
        assert_eq!(result.key_slot, 0);
    }

    #[test]
    fn test_fixed_timestamp() {
        let tag = Tag::new("TEST").unwrap();
        let ts_minutes = 29_049_600_u32; // 2026-01-18 00:00 UTC

        let msg = encode_with_timestamp(CURRENT_VERSION, &tag, TEST_KEY, ts_minutes).unwrap();
        let result = decode(&msg, TEST_KEY).unwrap();

        assert_eq!(result.timestamp_minutes, ts_minutes);
        assert_eq!(result.timestamp_utc, u64::from(ts_minutes) * 60);
        assert_eq!(result.key_slot, 0);
    }

    #[test]
    fn test_wrong_key() {
        let tag = Tag::new("SAKUZY").unwrap();
        let msg = encode(CURRENT_VERSION, &tag, TEST_KEY).unwrap();

        let wrong_key = b"wrong-key-32-bytes-for-hmac!!!!";
        let result = decode(&msg, wrong_key);

        assert!(matches!(result, Err(Error::HmacMismatch)));
    }

    #[test]
    fn test_tampered_message() {
        let tag = Tag::new("SAKUZY").unwrap();
        let mut msg = encode(CURRENT_VERSION, &tag, TEST_KEY).unwrap();

        // 篡改 timestamp
        msg[2] ^= 0x01;

        let result = decode(&msg, TEST_KEY);
        assert!(matches!(result, Err(Error::HmacMismatch)));
    }

    #[test]
    fn test_verify() {
        let tag = Tag::new("SAKUZY").unwrap();
        let msg = encode(CURRENT_VERSION, &tag, TEST_KEY).unwrap();

        assert!(verify(&msg, TEST_KEY));
        assert!(!verify(&msg, b"wrong-key"));
    }

    #[test]
    fn test_message_format_v1() {
        let tag = Tag::new("ABCDEFG").unwrap();
        let ts_minutes = 0x0102_0304_u32;

        let msg = encode_with_timestamp(VERSION_V1, &tag, TEST_KEY, ts_minutes).unwrap();

        // 验证结构
        assert_eq!(msg[0], VERSION_V1); // version
        assert_eq!(&msg[1..5], &[0x01, 0x02, 0x03, 0x04]); // timestamp big-endian
                                                           // msg[5..10] = tag packed
                                                           // msg[10..16] = hmac
    }

    #[test]
    fn test_decode_v1_compat() {
        let tag = Tag::new("SAKUZY").unwrap();
        let ts_minutes = 12_345_678_u32;
        let msg = encode_with_timestamp(VERSION_V1, &tag, TEST_KEY, ts_minutes).unwrap();
        let result = decode(&msg, TEST_KEY).unwrap();
        assert_eq!(result.version, VERSION_V1);
        assert_eq!(result.timestamp_minutes, ts_minutes);
        assert_eq!(result.key_slot, 0);
    }

    #[test]
    fn test_v2_timestamp_key_slot_pack_unpack() {
        let packed = pack_timestamp_v2(0x07ff_ffff, 31).unwrap();
        let (minutes, slot) = unpack_timestamp_v2(packed);
        assert_eq!(minutes, 0x07ff_ffff);
        assert_eq!(slot, 31);
    }

    #[test]
    fn test_v2_encode_non_zero_key_slot() {
        let tag = Tag::new("TEST").unwrap();
        let ts_minutes = 0x07ff_ffff;
        let msg =
            encode_with_timestamp_and_slot(VERSION_V2, &tag, TEST_KEY, ts_minutes, 31).unwrap();
        let result = decode(&msg, TEST_KEY).unwrap();
        assert_eq!(result.version, VERSION_V2);
        assert_eq!(result.timestamp_minutes, ts_minutes);
        assert_eq!(result.key_slot, 31);
    }

    #[test]
    fn test_v2_timestamp_range_limit() {
        let tag = Tag::new("TEST").unwrap();
        let err = encode_with_timestamp(VERSION_V2, &tag, TEST_KEY, MAX_TIMESTAMP_V2_MINUTES + 1)
            .unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }
}
