//! C FFI 导出
//!
//! 提供 C ABI 接口供 ObjC/Swift/其他语言调用

use std::ffi::{c_char, CStr};
use std::ptr;
use std::slice;

use crate::message::{self, CURRENT_VERSION, MESSAGE_LEN};
use crate::tag::Tag;

/// FFI 错误码
#[repr(i32)]
pub enum AWMError {
    Success = 0,
    InvalidTag = -1,
    InvalidMessageLength = -2,
    HmacMismatch = -3,
    NullPointer = -4,
    InvalidUtf8 = -5,
    ChecksumMismatch = -6,
}

/// 解码结果结构体
#[repr(C)]
pub struct AWMResult {
    pub version: u8,
    pub timestamp_utc: u64,
    pub timestamp_minutes: u32,
    pub tag: [c_char; 9],     // 8 chars + null terminator
    pub identity: [c_char; 8], // 7 chars max + null terminator
}

/// 创建 Tag（从身份字符串，自动补齐 + 计算校验位）
///
/// # Safety
/// - `identity` 必须是有效的 C 字符串
/// - `out` 必须指向至少 9 字节的缓冲区
#[no_mangle]
pub unsafe extern "C" fn awm_tag_new(identity: *const c_char, out: *mut c_char) -> i32 {
    if identity.is_null() || out.is_null() {
        return AWMError::NullPointer as i32;
    }

    let identity_str = match CStr::from_ptr(identity).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    match Tag::new(identity_str) {
        Ok(tag) => {
            let tag_str = tag.as_str();
            ptr::copy_nonoverlapping(tag_str.as_ptr(), out as *mut u8, 8);
            *out.add(8) = 0; // null terminator
            AWMError::Success as i32
        }
        Err(_) => AWMError::InvalidTag as i32,
    }
}

/// 验证 Tag 校验位
///
/// # Safety
/// - `tag` 必须是有效的 8 字符 C 字符串
#[no_mangle]
pub unsafe extern "C" fn awm_tag_verify(tag: *const c_char) -> bool {
    if tag.is_null() {
        return false;
    }

    let tag_str = match CStr::from_ptr(tag).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    Tag::parse(tag_str).is_ok()
}

/// 获取 Tag 的身份部分
///
/// # Safety
/// - `tag` 必须是有效的 8 字符 C 字符串
/// - `out` 必须指向至少 8 字节的缓冲区
#[no_mangle]
pub unsafe extern "C" fn awm_tag_identity(tag: *const c_char, out: *mut c_char) -> i32 {
    if tag.is_null() || out.is_null() {
        return AWMError::NullPointer as i32;
    }

    let tag_str = match CStr::from_ptr(tag).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    match Tag::parse(tag_str) {
        Ok(t) => {
            let identity = t.identity();
            ptr::copy_nonoverlapping(identity.as_ptr(), out as *mut u8, identity.len());
            *out.add(identity.len()) = 0;
            AWMError::Success as i32
        }
        Err(_) => AWMError::InvalidTag as i32,
    }
}

/// 编码消息
///
/// # Safety
/// - `tag` 必须是有效的 8 字符 C 字符串
/// - `key` 必须指向 `key_len` 字节的有效内存
/// - `out` 必须指向至少 16 字节的缓冲区
#[no_mangle]
pub unsafe extern "C" fn awm_message_encode(
    version: u8,
    tag: *const c_char,
    key: *const u8,
    key_len: usize,
    out: *mut u8,
) -> i32 {
    if tag.is_null() || key.is_null() || out.is_null() {
        return AWMError::NullPointer as i32;
    }

    let tag_str = match CStr::from_ptr(tag).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    let tag_obj = match Tag::parse(tag_str) {
        Ok(t) => t,
        Err(_) => return AWMError::InvalidTag as i32,
    };

    let key_slice = slice::from_raw_parts(key, key_len);

    match message::encode(version, &tag_obj, key_slice) {
        Ok(msg) => {
            ptr::copy_nonoverlapping(msg.as_ptr(), out, MESSAGE_LEN);
            AWMError::Success as i32
        }
        Err(_) => AWMError::InvalidTag as i32,
    }
}

/// 编码消息（指定时间戳）
#[no_mangle]
pub unsafe extern "C" fn awm_message_encode_with_timestamp(
    version: u8,
    tag: *const c_char,
    key: *const u8,
    key_len: usize,
    timestamp_minutes: u32,
    out: *mut u8,
) -> i32 {
    if tag.is_null() || key.is_null() || out.is_null() {
        return AWMError::NullPointer as i32;
    }

    let tag_str = match CStr::from_ptr(tag).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    let tag_obj = match Tag::parse(tag_str) {
        Ok(t) => t,
        Err(_) => return AWMError::InvalidTag as i32,
    };

    let key_slice = slice::from_raw_parts(key, key_len);

    match message::encode_with_timestamp(version, &tag_obj, key_slice, timestamp_minutes) {
        Ok(msg) => {
            ptr::copy_nonoverlapping(msg.as_ptr(), out, MESSAGE_LEN);
            AWMError::Success as i32
        }
        Err(_) => AWMError::InvalidTag as i32,
    }
}

/// 解码消息
///
/// # Safety
/// - `data` 必须指向 16 字节
/// - `key` 必须指向 `key_len` 字节
/// - `result` 必须是有效指针
#[no_mangle]
pub unsafe extern "C" fn awm_message_decode(
    data: *const u8,
    key: *const u8,
    key_len: usize,
    result: *mut AWMResult,
) -> i32 {
    if data.is_null() || key.is_null() || result.is_null() {
        return AWMError::NullPointer as i32;
    }

    let data_slice = slice::from_raw_parts(data, MESSAGE_LEN);
    let key_slice = slice::from_raw_parts(key, key_len);

    match message::decode(data_slice, key_slice) {
        Ok(r) => {
            (*result).version = r.version;
            (*result).timestamp_utc = r.timestamp_utc;
            (*result).timestamp_minutes = r.timestamp_minutes;

            // Copy tag (8 chars + null)
            let tag_bytes = r.tag.as_bytes();
            for (i, &b) in tag_bytes.iter().enumerate() {
                (*result).tag[i] = b as c_char;
            }
            (*result).tag[8] = 0;

            // Copy identity (up to 7 chars + null)
            let identity = r.tag.identity();
            for (i, b) in identity.bytes().enumerate() {
                (*result).identity[i] = b as c_char;
            }
            (*result).identity[identity.len()] = 0;

            AWMError::Success as i32
        }
        Err(crate::Error::HmacMismatch) => AWMError::HmacMismatch as i32,
        Err(crate::Error::ChecksumMismatch { .. }) => AWMError::ChecksumMismatch as i32,
        Err(_) => AWMError::InvalidTag as i32,
    }
}

/// 仅验证消息 HMAC
#[no_mangle]
pub unsafe extern "C" fn awm_message_verify(
    data: *const u8,
    key: *const u8,
    key_len: usize,
) -> bool {
    if data.is_null() || key.is_null() {
        return false;
    }

    let data_slice = slice::from_raw_parts(data, MESSAGE_LEN);
    let key_slice = slice::from_raw_parts(key, key_len);

    message::verify(data_slice, key_slice)
}

/// 获取当前版本号
#[no_mangle]
pub extern "C" fn awm_current_version() -> u8 {
    CURRENT_VERSION
}

/// 获取消息长度
#[no_mangle]
pub extern "C" fn awm_message_length() -> usize {
    MESSAGE_LEN
}
