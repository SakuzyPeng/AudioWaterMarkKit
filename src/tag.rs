//! Tag 编解码与校验.
//!
//! Tag 结构: 7 字符身份 + 1 校验位 = 8 字符 = 40 bit (5-bit packed).

use crate::charset::{char_to_index, index_to_char, is_valid_char, CHARSET, PRIMES};
use crate::error::{Error, Result};

/// 8 字符 Tag，包含 7 字符身份 + 1 校验位.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// 8 字符 ASCII (大写).
    chars: [u8; 8],
}

impl Tag {
    /// 从身份字符串创建 Tag（自动补齐 + 计算校验位）.
    ///
    /// # Example
    /// ```
    /// use awmkit::Tag;
    /// let tag = Tag::new("SAKUZY").unwrap();
    /// assert!(tag.verify());
    /// assert_eq!(tag.identity(), "SAKUZY");
    /// ```
    ///
    /// # Errors
    /// 当身份长度不在 1..=7 或包含非法字符时返回错误。.
    pub fn new(identity: &str) -> Result<Self> {
        let identity = identity.to_ascii_uppercase();
        let len = identity.len();

        if len == 0 || len > 7 {
            return Err(Error::InvalidIdentityLength(len));
        }

        // 验证字符
        for c in identity.bytes() {
            if !is_valid_char(c) {
                return Err(Error::InvalidChar(c as char));
            }
        }

        // 补齐到 7 字符
        let mut tag7 = [b'_'; 7];
        tag7[..len].copy_from_slice(identity.as_bytes());

        // 计算校验位
        let check = calc_checksum(tag7);

        let mut chars = [0u8; 8];
        chars[..7].copy_from_slice(&tag7);
        chars[7] = check;

        Ok(Self { chars })
    }

    /// 解析 8 字符 Tag 字符串（验证校验位）.
    ///
    /// # Errors
    /// 当长度不是 8、包含非法字符或校验位不匹配时返回错误。.
    ///
    /// # Panics
    /// 不会主动 panic；内部 `unwrap` 依赖固定长度切片不变式。.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.to_ascii_uppercase();

        if s.len() != 8 {
            return Err(Error::InvalidTagLength(s.len()));
        }

        let bytes = s.as_bytes();

        // 验证所有字符
        for &c in bytes {
            if !is_valid_char(c) {
                return Err(Error::InvalidChar(c as char));
            }
        }

        let mut chars = [0u8; 8];
        chars.copy_from_slice(bytes);

        let tag = Self { chars };

        // 验证校验位
        if !tag.verify() {
            // chars 长度固定为 8，切片 [..7] 必定成功
            #[allow(clippy::unwrap_used)]
            let expected = calc_checksum(chars[..7].try_into().unwrap());
            return Err(Error::ChecksumMismatch {
                expected: expected as char,
                got: chars[7] as char,
            });
        }

        Ok(tag)
    }

    /// 从 5 bytes packed 数据解码.
    ///
    /// # Errors
    /// 当 packed 数据包含非法索引或校验位不匹配时返回错误。.
    ///
    /// # Panics
    /// 不会主动 panic；内部 `unwrap` 依赖固定长度切片不变式。.
    pub fn from_packed(data: &[u8; 5]) -> Result<Self> {
        let mut bits: u64 = 0;
        for &b in data {
            bits = (bits << 8) | u64::from(b);
        }

        let mut chars = [0u8; 8];
        for i in (0..8).rev() {
            #[allow(clippy::cast_possible_truncation)]
            let idx = (bits & 0x1F) as u8;
            chars[i] = index_to_char(idx).ok_or(Error::InvalidChar(idx as char))?;
            bits >>= 5;
        }

        let tag = Self { chars };

        if !tag.verify() {
            // chars 长度固定为 8，切片 [..7] 必定成功
            #[allow(clippy::unwrap_used)]
            let expected = calc_checksum(chars[..7].try_into().unwrap());
            return Err(Error::ChecksumMismatch {
                expected: expected as char,
                got: chars[7] as char,
            });
        }

        Ok(tag)
    }

    /// 编码为 5 bytes packed 数据.
    ///
    /// # Panics
    /// 不会主动 panic；内部 `unwrap` 依赖 `Tag` 已完成字符集校验的不变式。.
    #[must_use]
    pub fn to_packed(&self) -> [u8; 5] {
        let mut bits: u64 = 0;
        for &c in &self.chars {
            // 字符已验证在 CHARSET 中，char_to_index 必定成功
            #[allow(clippy::unwrap_used)]
            let idx = u64::from(char_to_index(c).unwrap());
            bits = (bits << 5) | idx;
        }

        let mut out = [0u8; 5];
        for i in (0..5).rev() {
            #[allow(clippy::cast_possible_truncation)]
            let byte = (bits & 0xFF) as u8;
            out[i] = byte;
            bits >>= 8;
        }
        out
    }

    /// 验证校验位.
    ///
    /// # Panics
    /// 不会主动 panic；内部 `unwrap` 依赖固定长度切片不变式。.
    #[must_use]
    pub fn verify(&self) -> bool {
        // chars 长度固定为 8，切片 [..7] 必定成功
        #[allow(clippy::unwrap_used)]
        let expected = calc_checksum(self.chars[..7].try_into().unwrap());
        self.chars[7] == expected
    }

    /// 获取身份部分（去除尾部 _）.
    ///
    /// # Panics
    /// 不会主动 panic；内部 `from_utf8(...).unwrap()` 依赖 `Tag` 仅包含 ASCII 字符。.
    #[must_use]
    pub fn identity(&self) -> &str {
        // 所有字符都是 ASCII，from_utf8 必定成功
        #[allow(clippy::unwrap_used)]
        let s = std::str::from_utf8(&self.chars[..7]).unwrap();
        s.trim_end_matches('_')
    }

    /// 获取完整 8 字符 Tag.
    ///
    /// # Panics
    /// 不会主动 panic；内部 `from_utf8(...).unwrap()` 依赖 `Tag` 仅包含 ASCII 字符。.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // 所有字符都是 ASCII，from_utf8 必定成功
        #[allow(clippy::unwrap_used)]
        std::str::from_utf8(&self.chars).unwrap()
    }

    /// 获取字节数组.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 8] {
        &self.chars
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Tag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.len() <= 7 {
            Self::new(s)
        } else {
            Self::parse(s)
        }
    }
}

/// 计算 7 字符的校验位.
fn calc_checksum(tag7: [u8; 7]) -> u8 {
    let total: u32 = tag7
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let idx = u32::from(char_to_index(c).unwrap_or(0));
            idx * PRIMES[i]
        })
        .sum();

    #[allow(clippy::cast_possible_truncation)]
    let index = (total % 32) as usize;
    CHARSET[index]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new() {
        let tag = Tag::new("SAKUZY").unwrap();
        assert_eq!(tag.identity(), "SAKUZY");
        assert_eq!(tag.chars[6], b'_'); // 自动补齐
        assert!(tag.verify());
    }

    #[test]
    fn test_tag_new_short() {
        let tag = Tag::new("AB").unwrap();
        assert_eq!(tag.identity(), "AB");
        assert!(tag.verify());
    }

    #[test]
    fn test_tag_new_full() {
        let tag = Tag::new("ABCDEFG").unwrap();
        assert_eq!(tag.identity(), "ABCDEFG");
        assert!(tag.verify());
    }

    #[test]
    fn test_tag_parse() {
        let tag1 = Tag::new("SAKUZY").unwrap();
        let tag2 = Tag::parse(tag1.as_str()).unwrap();
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn test_invalid_char() {
        assert!(Tag::new("SAKUZY0").is_err()); // 0 被排除
        assert!(Tag::new("SAKUZYI").is_err()); // I 被排除
    }

    #[test]
    fn test_case_insensitive() {
        let tag1 = Tag::new("sakuzy").unwrap();
        let tag2 = Tag::new("SAKUZY").unwrap();
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn test_packed_round_trip() {
        let tag = Tag::new("SAKUZY").unwrap();
        let packed = tag.to_packed();
        assert_eq!(packed.len(), 5);

        let tag2 = Tag::from_packed(&packed).unwrap();
        assert_eq!(tag, tag2);
    }

    #[test]
    fn test_checksum_mismatch() {
        let result = Tag::parse("SAKUZY_A"); // 错误的校验位
        assert!(matches!(result, Err(Error::ChecksumMismatch { .. })));
    }
}
