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

#[cfg(feature = "app")]
use crate::app::{build_audio_proof, EvidenceStore, NewAudioEvidence, TagStore};
#[cfg(feature = "app")]
use rusty_chromaprint::{match_fingerprints, Configuration};
#[cfg(feature = "app")]
use serde::Serialize;

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
    pub key_slot: u8,
    pub tag: [c_char; 9],      // 8 chars + null terminator
    pub identity: [c_char; 8], // 7 chars max + null terminator
}

const CLONE_LIKELY_MAX_SCORE: f64 = 7.0;
const CLONE_LIKELY_MIN_SECONDS: f32 = 6.0;

fn copy_str_to_c_buf(dst: &mut [c_char], text: &str) {
    dst.fill(0);
    let max = dst.len().saturating_sub(1);
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(max);
    for (index, &byte) in bytes[..copy_len].iter().enumerate() {
        dst[index] = byte as c_char;
    }
}

/// Write UTF-8 string into C buffer with two-step size negotiation.
///
/// # Safety
/// - `out_required_len` must be a valid writable pointer.
/// - `out` may be null only when `out_len == 0`.
unsafe fn write_string_with_required(
    value: &str,
    out: *mut c_char,
    out_len: usize,
    out_required_len: *mut usize,
) -> i32 {
    if out_required_len.is_null() {
        return AWMError::NullPointer as i32;
    }

    let required = value.len().saturating_add(1);
    *out_required_len = required;

    if out.is_null() || out_len == 0 {
        return AWMError::Success as i32;
    }

    if out_len < required {
        return AWMError::InvalidMessageLength as i32;
    }

    ptr::copy_nonoverlapping(value.as_ptr(), out as *mut u8, value.len());
    *out.add(value.len()) = 0;
    AWMError::Success as i32
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
            (*result).key_slot = r.key_slot;

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
    /// 是否包含检测分数
    pub has_detect_score: bool,
    /// 检测分数（audiowmark 候选分数）
    pub detect_score: f32,
    /// 比特错误数
    pub bit_errors: u32,
}

/// 克隆校验结果类型
#[repr(i32)]
#[derive(Clone, Copy)]
pub enum AWMCloneCheckKind {
    Exact = 0,
    Likely = 1,
    Suspect = 2,
    Unavailable = 3,
}

/// 克隆校验结果
#[repr(C)]
pub struct AWMCloneCheckResult {
    /// 校验类型
    pub kind: AWMCloneCheckKind,
    /// 是否有指纹分数
    pub has_score: bool,
    /// 指纹匹配分数（越小越像）
    pub score: f64,
    /// 是否有匹配时长
    pub has_match_seconds: bool,
    /// 匹配时长（秒）
    pub match_seconds: f32,
    /// 是否有关联证据 ID
    pub has_evidence_id: bool,
    /// 关联证据 ID
    pub evidence_id: i64,
    /// 原因文本（null-terminated）
    pub reason: [c_char; 128],
}

impl AWMCloneCheckResult {
    fn reset(&mut self) {
        self.kind = AWMCloneCheckKind::Unavailable;
        self.has_score = false;
        self.score = 0.0;
        self.has_match_seconds = false;
        self.match_seconds = 0.0;
        self.has_evidence_id = false;
        self.evidence_id = 0;
        self.reason.fill(0);
    }
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
pub unsafe extern "C" fn awm_audio_new_with_binary(
    binary_path: *const c_char,
) -> *mut AWMAudioHandle {
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
pub unsafe extern "C" fn awm_audio_set_key_file(
    handle: *mut AWMAudioHandle,
    key_file: *const c_char,
) {
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
            (*result).has_detect_score = detect_result.detect_score.is_some();
            (*result).detect_score = detect_result.detect_score.unwrap_or(0.0);

            // Copy pattern
            copy_str_to_c_buf(&mut (*result).pattern, &detect_result.pattern);

            AWMError::Success as i32
        }
        Ok(None) => {
            (*result).found = false;
            (*result).raw_message = [0; 16];
            (*result).pattern.fill(0);
            (*result).has_detect_score = false;
            (*result).detect_score = 0.0;
            (*result).bit_errors = 0;
            AWMError::NoWatermarkFound as i32
        }
        Err(crate::Error::AudiowmarkNotFound) => AWMError::AudiowmarkNotFound as i32,
        Err(crate::Error::AudiowmarkExec(_)) => AWMError::AudiowmarkExec as i32,
        Err(_) => AWMError::AudiowmarkExec as i32,
    }
}

#[cfg(feature = "app")]
fn is_likely(score: f64, match_seconds: f32) -> bool {
    score <= CLONE_LIKELY_MAX_SCORE && match_seconds >= CLONE_LIKELY_MIN_SECONDS
}

#[cfg(feature = "app")]
fn evaluate_clone_check(
    input: &str,
    identity: &str,
    key_slot: u8,
) -> std::result::Result<AWMCloneCheckResult, String> {
    let mut output = AWMCloneCheckResult {
        kind: AWMCloneCheckKind::Unavailable,
        has_score: false,
        score: 0.0,
        has_match_seconds: false,
        match_seconds: 0.0,
        has_evidence_id: false,
        evidence_id: 0,
        reason: [0; 128],
    };

    let evidence_store = EvidenceStore::load().map_err(|e| format!("evidence_store: {e}"))?;
    let proof = build_audio_proof(input).map_err(|e| format!("proof_error: {e}"))?;

    let candidates = evidence_store
        .list_candidates(identity, key_slot)
        .map_err(|e| format!("query_error: {e}"))?;

    if candidates.is_empty() {
        output.kind = AWMCloneCheckKind::Suspect;
        copy_str_to_c_buf(&mut output.reason, "no_evidence");
        return Ok(output);
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.pcm_sha256 == proof.pcm_sha256)
    {
        output.kind = AWMCloneCheckKind::Exact;
        output.has_evidence_id = true;
        output.evidence_id = candidate.id;
        return Ok(output);
    }

    let config = Configuration::default();
    let mut best_match: Option<(i64, f64, f32)> = None;

    for candidate in &candidates {
        if candidate.fp_config_id != config.id() {
            continue;
        }

        let segments = match_fingerprints(&proof.chromaprint, &candidate.chromaprint, &config)
            .map_err(|e| format!("match_error: {e}"))?;

        for segment in segments {
            let duration = segment.duration(&config);
            let score = segment.score;
            match best_match {
                None => best_match = Some((candidate.id, score, duration)),
                Some((_, best_score, best_duration))
                    if duration > best_duration
                        || ((duration - best_duration).abs() < f32::EPSILON
                            && score < best_score) =>
                {
                    best_match = Some((candidate.id, score, duration));
                }
                _ => {}
            }
        }
    }

    if let Some((candidate_id, score, duration)) = best_match {
        output.has_score = true;
        output.score = score;
        output.has_match_seconds = true;
        output.match_seconds = duration;
        if is_likely(score, duration) {
            output.kind = AWMCloneCheckKind::Likely;
            output.has_evidence_id = true;
            output.evidence_id = candidate_id;
        } else {
            output.kind = AWMCloneCheckKind::Suspect;
            copy_str_to_c_buf(&mut output.reason, "threshold_not_met");
        }
        return Ok(output);
    }

    output.kind = AWMCloneCheckKind::Suspect;
    copy_str_to_c_buf(&mut output.reason, "no_similar_segment");
    Ok(output)
}

/// 评估克隆校验结果（优先 SHA256，其次指纹匹配）
///
/// # Safety
/// - `input` 与 `identity` 必须是有效 C 字符串
/// - `result` 必须是有效指针
#[no_mangle]
pub unsafe extern "C" fn awm_clone_check_for_file(
    input: *const c_char,
    identity: *const c_char,
    key_slot: u8,
    result: *mut AWMCloneCheckResult,
) -> i32 {
    if input.is_null() || identity.is_null() || result.is_null() {
        return AWMError::NullPointer as i32;
    }

    (*result).reset();

    let input_str = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };
    let identity_str = match CStr::from_ptr(identity).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    #[cfg(feature = "app")]
    {
        match evaluate_clone_check(input_str, identity_str, key_slot) {
            Ok(value) => {
                *result = value;
                AWMError::Success as i32
            }
            Err(reason) => {
                (*result).kind = AWMCloneCheckKind::Unavailable;
                copy_str_to_c_buf(&mut (*result).reason, &reason);
                AWMError::Success as i32
            }
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = key_slot;
        let _ = input_str;
        let _ = identity_str;
        (*result).kind = AWMCloneCheckKind::Unavailable;
        copy_str_to_c_buf(&mut (*result).reason, "app_feature_disabled");
        AWMError::Success as i32
    }
}

/// 对水印输出文件生成证据并写入数据库
///
/// # Safety
/// - `file_path` 必须是有效 C 字符串
/// - `raw_message` 必须指向 16 字节数据
/// - `key` 必须指向 `key_len` 字节
#[no_mangle]
pub unsafe extern "C" fn awm_evidence_record_file(
    file_path: *const c_char,
    raw_message: *const u8,
    key: *const u8,
    key_len: usize,
) -> i32 {
    if file_path.is_null() || raw_message.is_null() || key.is_null() {
        return AWMError::NullPointer as i32;
    }

    let file_path_str = match CStr::from_ptr(file_path).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };
    let raw: [u8; 16] = slice::from_raw_parts(raw_message, 16).try_into().unwrap();
    let key_slice = slice::from_raw_parts(key, key_len);

    #[cfg(feature = "app")]
    {
        let decoded = match message::decode(&raw, key_slice) {
            Ok(decoded) => decoded,
            Err(crate::Error::HmacMismatch) => return AWMError::HmacMismatch as i32,
            Err(crate::Error::ChecksumMismatch { .. }) => return AWMError::ChecksumMismatch as i32,
            Err(_) => return AWMError::InvalidTag as i32,
        };

        let proof = match build_audio_proof(file_path_str) {
            Ok(proof) => proof,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        let store = match EvidenceStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };

        let row = NewAudioEvidence {
            file_path: file_path_str.to_string(),
            tag: decoded.tag.to_string(),
            identity: decoded.identity().to_string(),
            version: decoded.version,
            key_slot: decoded.key_slot,
            timestamp_minutes: decoded.timestamp_minutes,
            message_hex: hex::encode(raw),
            sample_rate: proof.sample_rate,
            channels: proof.channels,
            sample_count: proof.sample_count,
            pcm_sha256: proof.pcm_sha256,
            chromaprint: proof.chromaprint,
            fp_config_id: proof.fp_config_id,
        };

        match store.insert(&row) {
            Ok(_) => AWMError::Success as i32,
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = file_path_str;
        let _ = raw;
        let _ = key_slice;
        AWMError::AudiowmarkExec as i32
    }
}

// ============================================================================
// Database Operations (requires "app" feature)
// ============================================================================

#[cfg(feature = "app")]
#[derive(Serialize)]
struct FfiAudioEvidence {
    id: i64,
    created_at: u64,
    file_path: String,
    tag: String,
    identity: String,
    version: u8,
    key_slot: u8,
    timestamp_minutes: u32,
    message_hex: String,
    sample_rate: u32,
    channels: u32,
    sample_count: u64,
    pcm_sha256: String,
    chromaprint_blob: String,
    fingerprint_len: usize,
    fp_config_id: u8,
}

#[cfg(feature = "app")]
fn encode_chromaprint_blob_hex(values: &[u32]) -> String {
    let mut out = Vec::with_capacity(values.len().saturating_mul(4));
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    hex::encode(out)
}

/// Query database summary counts for tag mappings and evidence.
///
/// # Safety
/// - `out_tag_count` and `out_evidence_count` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn awm_db_summary(
    out_tag_count: *mut u64,
    out_evidence_count: *mut u64,
) -> i32 {
    if out_tag_count.is_null() || out_evidence_count.is_null() {
        return AWMError::NullPointer as i32;
    }

    #[cfg(feature = "app")]
    {
        let tag_store = match TagStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        let evidence_store = match EvidenceStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };

        *out_tag_count = u64::try_from(tag_store.count()).unwrap_or(u64::MAX);
        *out_evidence_count =
            u64::try_from(evidence_store.count_all().unwrap_or(0)).unwrap_or(u64::MAX);
        return AWMError::Success as i32;
    }

    #[cfg(not(feature = "app"))]
    {
        *out_tag_count = 0;
        *out_evidence_count = 0;
        AWMError::AudiowmarkExec as i32
    }
}

/// List tag mappings as JSON.
///
/// Two-step usage:
/// 1) call with `out=nullptr, out_len=0` to get required length
/// 2) allocate buffer and call again to receive utf-8 json string
///
/// # Safety
/// - `out_required_len` must be valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn awm_db_tag_list_json(
    limit: u32,
    out: *mut c_char,
    out_len: usize,
    out_required_len: *mut usize,
) -> i32 {
    #[cfg(feature = "app")]
    {
        let normalized_limit = usize::try_from(limit).unwrap_or(usize::MAX).max(1);
        let store = match TagStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        let list = store.list_recent(normalized_limit);
        let json = match serde_json::to_string(&list) {
            Ok(json) => json,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        return write_string_with_required(&json, out, out_len, out_required_len);
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = limit;
        let _ = write_string_with_required("[]", out, out_len, out_required_len);
        AWMError::AudiowmarkExec as i32
    }
}

/// Lookup tag by username (case-insensitive).
///
/// Returns empty string when mapping is not found.
///
/// # Safety
/// - `username` must be a valid C string.
/// - `out_required_len` must be valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn awm_db_tag_lookup(
    username: *const c_char,
    out_tag: *mut c_char,
    out_len: usize,
    out_required_len: *mut usize,
) -> i32 {
    if username.is_null() {
        return AWMError::NullPointer as i32;
    }
    let username_str = match CStr::from_ptr(username).to_str() {
        Ok(value) => value,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    #[cfg(feature = "app")]
    {
        let store = match TagStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        let tag = match store.lookup_tag_ci(username_str) {
            Ok(tag) => tag.unwrap_or_default(),
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        return write_string_with_required(&tag, out_tag, out_len, out_required_len);
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = username_str;
        let _ = write_string_with_required("", out_tag, out_len, out_required_len);
        AWMError::AudiowmarkExec as i32
    }
}

/// Save tag mapping only when username does not exist.
///
/// # Safety
/// - pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn awm_db_tag_save_if_absent(
    username: *const c_char,
    tag: *const c_char,
    out_inserted: *mut bool,
) -> i32 {
    if username.is_null() || tag.is_null() || out_inserted.is_null() {
        return AWMError::NullPointer as i32;
    }

    let username_str = match CStr::from_ptr(username).to_str() {
        Ok(value) => value,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };
    let tag_str = match CStr::from_ptr(tag).to_str() {
        Ok(value) => value,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    if username_str.trim().is_empty() {
        return AWMError::InvalidTag as i32;
    }

    #[cfg(feature = "app")]
    {
        let parsed_tag = match Tag::parse(tag_str) {
            Ok(tag) => tag,
            Err(_) => return AWMError::InvalidTag as i32,
        };
        let mut store = match TagStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        match store.save_if_absent(username_str, &parsed_tag) {
            Ok(inserted) => {
                *out_inserted = inserted;
                AWMError::Success as i32
            }
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = (username_str, tag_str);
        *out_inserted = false;
        AWMError::AudiowmarkExec as i32
    }
}

/// Remove tag mappings by usernames JSON array.
///
/// # Safety
/// - `usernames_json` must be valid UTF-8 C string.
/// - `out_deleted` must be valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn awm_db_tag_remove_json(
    usernames_json: *const c_char,
    out_deleted: *mut u32,
) -> i32 {
    if usernames_json.is_null() || out_deleted.is_null() {
        return AWMError::NullPointer as i32;
    }
    let usernames_json_str = match CStr::from_ptr(usernames_json).to_str() {
        Ok(value) => value,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    #[cfg(feature = "app")]
    {
        let usernames: Vec<String> = match serde_json::from_str(usernames_json_str) {
            Ok(values) => values,
            Err(_) => return AWMError::InvalidTag as i32,
        };
        let mut store = match TagStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };
        match store.remove_usernames(&usernames) {
            Ok(deleted) => {
                *out_deleted = u32::try_from(deleted).unwrap_or(u32::MAX);
                AWMError::Success as i32
            }
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = usernames_json_str;
        *out_deleted = 0;
        AWMError::AudiowmarkExec as i32
    }
}

/// List evidence records as JSON.
///
/// # Safety
/// - `out_required_len` must be valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn awm_db_evidence_list_json(
    limit: u32,
    out: *mut c_char,
    out_len: usize,
    out_required_len: *mut usize,
) -> i32 {
    #[cfg(feature = "app")]
    {
        let normalized_limit = usize::try_from(limit).unwrap_or(usize::MAX).max(1);
        let store = match EvidenceStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };

        let rows = match store.list_filtered(None, None, None, normalized_limit) {
            Ok(rows) => rows,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };

        let payload: Vec<FfiAudioEvidence> = rows
            .into_iter()
            .map(|row| FfiAudioEvidence {
                id: row.id,
                created_at: row.created_at,
                file_path: row.file_path,
                tag: row.tag,
                identity: row.identity,
                version: row.version,
                key_slot: row.key_slot,
                timestamp_minutes: row.timestamp_minutes,
                message_hex: row.message_hex,
                sample_rate: row.sample_rate,
                channels: row.channels,
                sample_count: row.sample_count,
                pcm_sha256: row.pcm_sha256,
                chromaprint_blob: encode_chromaprint_blob_hex(&row.chromaprint),
                fingerprint_len: row.chromaprint.len(),
                fp_config_id: row.fp_config_id,
            })
            .collect();

        let json = match serde_json::to_string(&payload) {
            Ok(json) => json,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };

        return write_string_with_required(&json, out, out_len, out_required_len);
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = limit;
        let _ = write_string_with_required("[]", out, out_len, out_required_len);
        AWMError::AudiowmarkExec as i32
    }
}

/// Remove evidence records by ids JSON array.
///
/// # Safety
/// - `ids_json` must be valid UTF-8 C string.
/// - `out_deleted` must be valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn awm_db_evidence_remove_json(
    ids_json: *const c_char,
    out_deleted: *mut u32,
) -> i32 {
    if ids_json.is_null() || out_deleted.is_null() {
        return AWMError::NullPointer as i32;
    }
    let ids_json_str = match CStr::from_ptr(ids_json).to_str() {
        Ok(value) => value,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    #[cfg(feature = "app")]
    {
        let ids: Vec<i64> = match serde_json::from_str(ids_json_str) {
            Ok(values) => values,
            Err(_) => return AWMError::InvalidTag as i32,
        };
        let store = match EvidenceStore::load() {
            Ok(store) => store,
            Err(_) => return AWMError::AudiowmarkExec as i32,
        };

        let mut deleted: u32 = 0;
        for id in ids {
            match store.remove_by_id(id) {
                Ok(true) => deleted = deleted.saturating_add(1),
                Ok(false) => {}
                Err(_) => return AWMError::AudiowmarkExec as i32,
            }
        }
        *out_deleted = deleted;
        return AWMError::Success as i32;
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = ids_json_str;
        *out_deleted = 0;
        AWMError::AudiowmarkExec as i32
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

/// 获取 audiowmark 二进制路径
///
/// # Safety
/// - `handle` 必须是有效的 Audio 句柄
/// - `out` 必须指向至少 `out_len` 字节的缓冲区
/// - 返回 `AWM_SUCCESS` 并写入 null-terminated 路径，或错误码
#[no_mangle]
pub unsafe extern "C" fn awm_audio_binary_path(
    handle: *const AWMAudioHandle,
    out: *mut c_char,
    out_len: usize,
) -> i32 {
    if handle.is_null() || out.is_null() || out_len == 0 {
        return AWMError::NullPointer as i32;
    }

    let path_str = (*handle).inner.binary_path().to_string_lossy();
    let bytes = path_str.as_bytes();
    let max = out_len.saturating_sub(1);
    let copy_len = bytes.len().min(max);
    ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, copy_len);
    *out.add(copy_len) = 0;
    AWMError::Success as i32
}

// ============================================================================
// Key Management (requires "app" feature)
// ============================================================================

/// 检查密钥是否已存在
#[no_mangle]
pub extern "C" fn awm_key_exists() -> bool {
    #[cfg(feature = "app")]
    {
        match crate::app::KeyStore::new() {
            Ok(ks) => ks.exists(),
            Err(_) => false,
        }
    }
    #[cfg(not(feature = "app"))]
    {
        false
    }
}

/// 获取当前生效的密钥存储后端标签
///
/// 返回值说明：
/// - "keyring (service: ...)"：通过 keyring 读取
/// - "dpapi (...)"：通过 DPAPI 回退文件读取（Windows）
/// - "none"：当前未配置密钥
///
/// # Safety
/// - `out` 必须指向至少 `out_len` 字节的缓冲区
#[no_mangle]
pub unsafe extern "C" fn awm_key_backend_label(out: *mut c_char, out_len: usize) -> i32 {
    if out.is_null() || out_len == 0 {
        return AWMError::NullPointer as i32;
    }

    #[cfg(feature = "app")]
    {
        let label = match crate::app::KeyStore::new().and_then(|ks| ks.load_with_backend()) {
            Ok((_key, backend)) => backend.label(),
            Err(_) => "none".to_string(),
        };

        let bytes = label.as_bytes();
        let max = out_len.saturating_sub(1);
        let copy_len = bytes.len().min(max);
        ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, copy_len);
        *out.add(copy_len) = 0;
        return AWMError::Success as i32;
    }

    #[cfg(not(feature = "app"))]
    {
        *out = 0;
        let _ = out_len;
        AWMError::AudiowmarkExec as i32
    }
}

/// 加载密钥到输出缓冲区
///
/// # Safety
/// - `out_key` 必须指向至少 `out_key_cap` 字节的缓冲区
/// - `out_key_cap` 必须 >= 32
/// - 成功时写入 32 字节密钥
#[no_mangle]
pub unsafe extern "C" fn awm_key_load(out_key: *mut u8, out_key_cap: usize) -> i32 {
    if out_key.is_null() {
        return AWMError::NullPointer as i32;
    }

    #[cfg(feature = "app")]
    {
        if out_key_cap < crate::app::KEY_LEN {
            return AWMError::InvalidMessageLength as i32;
        }
        match crate::app::KeyStore::new().and_then(|ks| ks.load()) {
            Ok(key) => {
                ptr::copy_nonoverlapping(key.as_ptr(), out_key, key.len());
                AWMError::Success as i32
            }
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = out_key_cap;
        AWMError::AudiowmarkExec as i32
    }
}

/// 生成新密钥并保存，返回密钥内容
///
/// # Safety
/// - `out_key` 必须指向至少 `out_key_cap` 字节的缓冲区
/// - `out_key_cap` 必须 >= 32
#[no_mangle]
pub unsafe extern "C" fn awm_key_generate_and_save(out_key: *mut u8, out_key_cap: usize) -> i32 {
    if out_key.is_null() {
        return AWMError::NullPointer as i32;
    }

    #[cfg(feature = "app")]
    {
        if out_key_cap < crate::app::KEY_LEN {
            return AWMError::InvalidMessageLength as i32;
        }
        let key = crate::app::generate_key();
        match crate::app::KeyStore::new().and_then(|ks| {
            ks.save(&key)?;
            Ok(())
        }) {
            Ok(()) => {
                ptr::copy_nonoverlapping(key.as_ptr(), out_key, key.len());
                AWMError::Success as i32
            }
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = out_key_cap;
        AWMError::AudiowmarkExec as i32
    }
}

/// 获取当前激活槽位
///
/// # Safety
/// - `out_slot` 必须是有效指针
#[no_mangle]
pub unsafe extern "C" fn awm_key_active_slot_get(out_slot: *mut u8) -> i32 {
    if out_slot.is_null() {
        return AWMError::NullPointer as i32;
    }

    #[cfg(feature = "app")]
    {
        match crate::app::KeyStore::new().and_then(|ks| ks.active_slot()) {
            Ok(slot) => {
                *out_slot = slot;
                AWMError::Success as i32
            }
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = out_slot;
        AWMError::AudiowmarkExec as i32
    }
}

/// 设置当前激活槽位
#[no_mangle]
pub extern "C" fn awm_key_active_slot_set(slot: u8) -> i32 {
    #[cfg(feature = "app")]
    {
        match crate::app::KeyStore::new().and_then(|ks| ks.set_active_slot(slot)) {
            Ok(()) => AWMError::Success as i32,
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }
    #[cfg(not(feature = "app"))]
    {
        let _ = slot;
        AWMError::AudiowmarkExec as i32
    }
}

/// 检查指定槽位是否存在密钥
#[no_mangle]
pub extern "C" fn awm_key_exists_slot(slot: u8) -> bool {
    #[cfg(feature = "app")]
    {
        match crate::app::KeyStore::new() {
            Ok(ks) => ks.exists_slot(slot),
            Err(_) => false,
        }
    }
    #[cfg(not(feature = "app"))]
    {
        let _ = slot;
        false
    }
}

/// 生成并保存指定槽位密钥
///
/// # Safety
/// - `out_key` 必须指向至少 `out_key_cap` 字节的缓冲区
/// - `out_key_cap` 必须 >= 32
#[no_mangle]
pub unsafe extern "C" fn awm_key_generate_and_save_slot(
    slot: u8,
    out_key: *mut u8,
    out_key_cap: usize,
) -> i32 {
    if out_key.is_null() {
        return AWMError::NullPointer as i32;
    }

    #[cfg(feature = "app")]
    {
        if out_key_cap < crate::app::KEY_LEN {
            return AWMError::InvalidMessageLength as i32;
        }
        let key = crate::app::generate_key();
        match crate::app::KeyStore::new().and_then(|ks| {
            ks.save_slot(slot, &key)?;
            Ok(())
        }) {
            Ok(()) => {
                ptr::copy_nonoverlapping(key.as_ptr(), out_key, key.len());
                AWMError::Success as i32
            }
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = (slot, out_key_cap);
        AWMError::AudiowmarkExec as i32
    }
}

/// 删除指定槽位密钥，并返回删除后激活槽位
///
/// # Safety
/// - `out_new_active_slot` 必须是有效指针
#[no_mangle]
pub unsafe extern "C" fn awm_key_delete_slot(slot: u8, out_new_active_slot: *mut u8) -> i32 {
    if out_new_active_slot.is_null() {
        return AWMError::NullPointer as i32;
    }

    #[cfg(feature = "app")]
    {
        match crate::app::KeyStore::new().and_then(|ks| ks.delete_slot_and_reconcile_active(slot)) {
            Ok(new_active_slot) => {
                *out_new_active_slot = new_active_slot;
                AWMError::Success as i32
            }
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }
    #[cfg(not(feature = "app"))]
    {
        let _ = (slot, out_new_active_slot);
        AWMError::AudiowmarkExec as i32
    }
}

/// 删除已保存的密钥
#[no_mangle]
pub extern "C" fn awm_key_delete() -> i32 {
    #[cfg(feature = "app")]
    {
        match crate::app::KeyStore::new().and_then(|ks| {
            let active_slot = ks.active_slot()?;
            let _ = ks.delete_slot_and_reconcile_active(active_slot)?;
            Ok(())
        }) {
            Ok(()) => AWMError::Success as i32,
            Err(_) => AWMError::AudiowmarkExec as i32,
        }
    }
    #[cfg(not(feature = "app"))]
    {
        AWMError::AudiowmarkExec as i32
    }
}

// ============================================================================
// Tag Suggestion (requires "app" feature)
// ============================================================================

/// 从用户名生成推荐标签（SHA256 哈希 → Base32 编码）
///
/// # Safety
/// - `username` 必须是有效的 C 字符串
/// - `out_tag` 必须指向至少 9 字节的缓冲区
#[no_mangle]
pub unsafe extern "C" fn awm_tag_suggest(username: *const c_char, out_tag: *mut c_char) -> i32 {
    if username.is_null() || out_tag.is_null() {
        return AWMError::NullPointer as i32;
    }

    let username_str = match CStr::from_ptr(username).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    #[cfg(feature = "app")]
    {
        match crate::app::TagStore::suggest(username_str) {
            Ok(tag) => {
                let tag_str = tag.as_str();
                ptr::copy_nonoverlapping(tag_str.as_ptr(), out_tag as *mut u8, 8);
                *out_tag.add(8) = 0;
                AWMError::Success as i32
            }
            Err(_) => AWMError::InvalidTag as i32,
        }
    }

    #[cfg(not(feature = "app"))]
    {
        let _ = username_str;
        AWMError::InvalidTag as i32
    }
}

// ============================================================================
// Multichannel Operations
// ============================================================================

#[cfg(feature = "multichannel")]
use crate::multichannel::ChannelLayout;

/// 声道布局枚举
#[repr(i32)]
#[derive(Clone, Copy)]
pub enum AWMChannelLayout {
    /// 立体声 (2ch)
    Stereo = 0,
    /// 5.1 环绕 (6ch)
    Surround51 = 1,
    /// 5.1.2 (8ch)
    Surround512 = 2,
    /// 7.1 环绕 (8ch)
    Surround71 = 3,
    /// 7.1.4 Atmos (12ch)
    Surround714 = 4,
    /// 9.1.6 Atmos (16ch)
    Surround916 = 5,
    /// 自动检测
    Auto = -1,
}

#[cfg(feature = "multichannel")]
impl AWMChannelLayout {
    fn to_rust_layout(self) -> Option<ChannelLayout> {
        match self {
            Self::Stereo => Some(ChannelLayout::Stereo),
            Self::Surround51 => Some(ChannelLayout::Surround51),
            Self::Surround512 => Some(ChannelLayout::Surround512),
            Self::Surround71 => Some(ChannelLayout::Surround71),
            Self::Surround714 => Some(ChannelLayout::Surround714),
            Self::Surround916 => Some(ChannelLayout::Surround916),
            Self::Auto => None,
        }
    }
}

/// 多声道检测结果 - 单个声道对
#[repr(C)]
pub struct AWMPairResult {
    /// 声道对索引
    pub pair_index: u32,
    /// 是否检测到水印
    pub found: bool,
    /// 原始消息 (16 bytes)
    pub raw_message: [u8; 16],
    /// 比特错误数
    pub bit_errors: u32,
}

/// 多声道检测结果
#[repr(C)]
pub struct AWMMultichannelDetectResult {
    /// 声道对数量
    pub pair_count: u32,
    /// 各声道对结果 (最多 8 对)
    pub pairs: [AWMPairResult; 8],
    /// 是否有最佳结果
    pub has_best: bool,
    /// 最佳结果的原始消息
    pub best_raw_message: [u8; 16],
    /// 最佳结果的比特错误数
    pub best_bit_errors: u32,
}

/// 多声道嵌入水印
///
/// # Safety
/// - `handle` 必须是有效的 Audio 句柄
/// - `input`, `output` 必须是有效的 C 字符串
/// - `message` 必须指向 16 字节
#[cfg(feature = "multichannel")]
#[no_mangle]
pub unsafe extern "C" fn awm_audio_embed_multichannel(
    handle: *const AWMAudioHandle,
    input: *const c_char,
    output: *const c_char,
    message: *const u8,
    layout: AWMChannelLayout,
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
    let rust_layout = layout.to_rust_layout();

    match (*handle)
        .inner
        .embed_multichannel(input_str, output_str, &msg, rust_layout)
    {
        Ok(_) => AWMError::Success as i32,
        Err(crate::Error::AudiowmarkNotFound) => AWMError::AudiowmarkNotFound as i32,
        Err(crate::Error::AudiowmarkExec(_)) => AWMError::AudiowmarkExec as i32,
        Err(_) => AWMError::AudiowmarkExec as i32,
    }
}

/// 多声道检测水印
///
/// # Safety
/// - `handle` 必须是有效的 Audio 句柄
/// - `input` 必须是有效的 C 字符串
/// - `result` 必须是有效指针
#[cfg(feature = "multichannel")]
#[no_mangle]
pub unsafe extern "C" fn awm_audio_detect_multichannel(
    handle: *const AWMAudioHandle,
    input: *const c_char,
    layout: AWMChannelLayout,
    result: *mut AWMMultichannelDetectResult,
) -> i32 {
    if handle.is_null() || input.is_null() || result.is_null() {
        return AWMError::NullPointer as i32;
    }

    let input_str = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => return AWMError::InvalidUtf8 as i32,
    };

    let rust_layout = layout.to_rust_layout();

    match (*handle).inner.detect_multichannel(input_str, rust_layout) {
        Ok(mc_result) => {
            // 初始化结果
            (*result).pair_count = mc_result.pairs.len() as u32;
            (*result).has_best = mc_result.best.is_some();

            // 复制各声道对结果
            for (i, (pair_idx, _name, detect_opt)) in mc_result.pairs.iter().enumerate() {
                if i >= 8 {
                    break;
                }
                (*result).pairs[i].pair_index = *pair_idx as u32;
                if let Some(detect) = detect_opt {
                    (*result).pairs[i].found = true;
                    (*result).pairs[i].raw_message = detect.raw_message;
                    (*result).pairs[i].bit_errors = detect.bit_errors;
                } else {
                    (*result).pairs[i].found = false;
                    (*result).pairs[i].raw_message = [0; 16];
                    (*result).pairs[i].bit_errors = 0;
                }
            }

            // 复制最佳结果
            if let Some(best) = &mc_result.best {
                (*result).best_raw_message = best.raw_message;
                (*result).best_bit_errors = best.bit_errors;
            } else {
                (*result).best_raw_message = [0; 16];
                (*result).best_bit_errors = 0;
            }

            if mc_result.best.is_some() {
                AWMError::Success as i32
            } else {
                AWMError::NoWatermarkFound as i32
            }
        }
        Err(crate::Error::AudiowmarkNotFound) => AWMError::AudiowmarkNotFound as i32,
        Err(crate::Error::AudiowmarkExec(_)) => AWMError::AudiowmarkExec as i32,
        Err(_) => AWMError::AudiowmarkExec as i32,
    }
}

/// 获取声道布局的声道数
#[no_mangle]
pub extern "C" fn awm_channel_layout_channels(layout: AWMChannelLayout) -> u32 {
    match layout {
        AWMChannelLayout::Stereo => 2,
        AWMChannelLayout::Surround51 => 6,
        AWMChannelLayout::Surround512 | AWMChannelLayout::Surround71 => 8,
        AWMChannelLayout::Surround714 => 12,
        AWMChannelLayout::Surround916 => 16,
        AWMChannelLayout::Auto => 0,
    }
}
