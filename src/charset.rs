//! 字符集定义
//!
//! 32 字符 Base32 变体，排除易混淆字符 O/0/I/1/L

/// 字符集：A-Z (去掉 O, I, L) + 2-9 + _
pub const CHARSET: &[u8; 32] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789_";

/// 校验位计算用素数
pub const PRIMES: [u32; 7] = [3, 5, 7, 11, 13, 17, 19];

/// 字符转索引 (0-31)，无效字符返回 None
#[inline]
pub fn char_to_index(c: u8) -> Option<u8> {
    let c = c.to_ascii_uppercase();
    CHARSET.iter().position(|&x| x == c).map(|i| i as u8)
}

/// 索引转字符
#[inline]
pub fn index_to_char(i: u8) -> Option<u8> {
    CHARSET.get(i as usize).copied()
}

/// 验证字符是否在字符集内
#[inline]
pub fn is_valid_char(c: u8) -> bool {
    char_to_index(c).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_charset_length() {
        assert_eq!(CHARSET.len(), 32);
    }

    #[test]
    fn test_excluded_chars() {
        // O, 0, I, 1, L 应被排除
        assert!(char_to_index(b'O').is_none());
        assert!(char_to_index(b'0').is_none());
        assert!(char_to_index(b'I').is_none());
        assert!(char_to_index(b'1').is_none());
        assert!(char_to_index(b'L').is_none());
    }

    #[test]
    fn test_valid_chars() {
        assert_eq!(char_to_index(b'A'), Some(0));
        assert_eq!(char_to_index(b'Z'), Some(22));
        assert_eq!(char_to_index(b'2'), Some(23));
        assert_eq!(char_to_index(b'9'), Some(30));
        assert_eq!(char_to_index(b'_'), Some(31));
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(char_to_index(b'a'), char_to_index(b'A'));
        assert_eq!(char_to_index(b'z'), char_to_index(b'Z'));
    }

    #[test]
    fn test_round_trip() {
        for i in 0..32u8 {
            let c = index_to_char(i).unwrap();
            assert_eq!(char_to_index(c), Some(i));
        }
    }
}
