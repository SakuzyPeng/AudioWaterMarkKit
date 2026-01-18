//! C FFI 导出
//!
//! 提供 C ABI 接口供 ObjC/Swift/其他语言调用

// FFI 模块需要 unsafe 代码
#![allow(unsafe_code)]
#![allow(clippy::unwrap_used)]

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
    AudiowmarkNotFound = -7,
    AudiowmarkExec = -8,
    NoWatermarkFound = -9,
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

// ============================================================================
// Audio Operations
// ============================================================================

use crate::audio::Audio;

/// 不透明的 Audio 句柄
pub struct AWMAudioHandle {
    inner: Audio,
}

/// 检测结果结构体
#[repr(C)]
pub struct AWMDetectResult {
    /// 是否检测到水印
    pub found: bool,
    /// 原始消息 (16 bytes)
    pub raw_message: [u8; 16],
    /// 检测模式 (null-terminated)
    pub pattern: [c_char; 16],
    /// 比特错误数
    pub bit_errors: u32,
}

/// 创建 Audio 实例（自动搜索 audiowmark）
///
/// # Safety
/// 返回的指针需要通过 `awm_audio_free` 释放
#[no_mangle]
pub extern "C" fn awm_audio_new() -> *mut AWMAudioHandle {
    match Audio::new() {
        Ok(audio) => Box::into_raw(Box::new(AWMAudioHandle { inner: audio })),
        Err(_) => ptr::null_mut(),
    }
}

/// 创建 Audio 实例（指定 audiowmark 路径）
///
/// # Safety
/// - `binary_path` 必须是有效的 C 字符串
/// - 返回的指针需要通过 `awm_audio_free` 释放
#[no_mangle]
pub unsafe extern "C" fn awm_audio_new_with_binary(binary_path: *const c_char) -> *mut AWMAudioHandle {
    if binary_path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(binary_path).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match Audio::with_binary(path_str) {
        Ok(audio) => Box::into_raw(Box::new(AWMAudioHandle { inner: audio })),
        Err(_) => ptr::null_mut(),
    }
}

/// 释放 Audio 实例
///
/// # Safety
/// - `handle` 必须是 `awm_audio_new*` 返回的有效指针
#[no_mangle]
pub unsafe extern "C" fn awm_audio_free(handle: *mut AWMAudioHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// 设置水印强度
///
/// # Safety
/// - `handle` 必须是有效的 Audio 句柄
#[no_mangle]
pub unsafe extern "C" fn awm_audio_set_strength(handle: *mut AWMAudioHandle, strength: u8) {
    if !handle.is_null() {
        let audio = &mut (*handle).inner;
        *audio = std::mem::take(audio).strength(strength);
    }
}

/// 设置密钥文件
///
/// # Safety
/// - `handle` 必须是有效的 Audio 句柄
/// - `key_file` 必须是有效的 C 字符串
#[no_mangle]
pub unsafe extern "C" fn awm_audio_set_key_file(handle: *mut AWMAudioHandle, key_file: *const c_char) {
    if handle.is_null() || key_file.is_null() {
        return;
    }

    let path_str = match CStr::from_ptr(key_file).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    let audio = &mut (*handle).inner;
    *audio = std::mem::take(audio).key_file(path_str);
}

/// 嵌入水印到音频
///
/// # Safety
/// - `handle` 必须是有效的 Audio 句柄
/// - `input`, `output` 必须是有效的 C 字符串
/// - `message` 必须指向 16 字节
#[no_mangle]
pub unsafe extern "C" fn awm_audio_embed(
    handle: *const AWMAudioHandle,
    input: *const c_char,
    output: *const c_char,
    message: *const u8,
) -> i32 {
    if handle.is_null() || input.is_null() || output.is_null() || message.is_null() {
        return AWMError::NullPointer as i32;
    }

    let input_str = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    let output_str = match CStr::from_ptr(output).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    let msg: [u8; 16] = slice::from_raw_parts(message, 16).try_into().unwrap();

    match (*handle).inner.embed(input_str, output_str, &msg) {
        Ok(_) => AWMError::Success as i32,
        Err(crate::Error::AudiowmarkNotFound) => AWMError::AudiowmarkNotFound as i32,
        Err(crate::Error::AudiowmarkExec(_)) => AWMError::AudiowmarkExec as i32,
        Err(_) => AWMError::AudiowmarkExec as i32,
    }
}

/// 从音频检测水印
///
/// # Safety
/// - `handle` 必须是有效的 Audio 句柄
/// - `input` 必须是有效的 C 字符串
/// - `result` 必须是有效指针
#[no_mangle]
pub unsafe extern "C" fn awm_audio_detect(
    handle: *const AWMAudioHandle,
    input: *const c_char,
    result: *mut AWMDetectResult,
) -> i32 {
    if handle.is_null() || input.is_null() || result.is_null() {
        return AWMError::NullPointer as i32;
    }

    let input_str = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    match (*handle).inner.detect(input_str) {
        Ok(Some(detect_result)) => {
            (*result).found = true;
            (*result).raw_message = detect_result.raw_message;
            (*result).bit_errors = detect_result.bit_errors;

            // Copy pattern
            let pattern_bytes = detect_result.pattern.as_bytes();
            let copy_len = pattern_bytes.len().min(15);
            for (i, &b) in pattern_bytes[..copy_len].iter().enumerate() {
                (*result).pattern[i] = b as c_char;
            }
            (*result).pattern[copy_len] = 0;

            AWMError::Success as i32
        }
        Ok(None) => {
            (*result).found = false;
            AWMError::NoWatermarkFound as i32
        }
        Err(crate::Error::AudiowmarkNotFound) => AWMError::AudiowmarkNotFound as i32,
        Err(crate::Error::AudiowmarkExec(_)) => AWMError::AudiowmarkExec as i32,
        Err(_) => AWMError::AudiowmarkExec as i32,
    }
}

/// 检查 audiowmark 是否可用
#[no_mangle]
pub unsafe extern "C" fn awm_audio_is_available(handle: *const AWMAudioHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).inner.is_available()
}
