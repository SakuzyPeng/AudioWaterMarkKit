//! 音频水印嵌入/检测
//!
//! 封装 audiowmark 命令行工具

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::{
    fs,
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
};

use crate::error::{Error, Result};
#[cfg(any(feature = "ffmpeg-decode", feature = "multichannel"))]
use crate::media;
use crate::message::{self, MESSAGE_LEN};
use crate::tag::Tag;

#[cfg(feature = "multichannel")]
use crate::multichannel::{
    build_smart_route_plan, ChannelLayout, MultichannelAudio, RouteMode, RouteStep,
    DEFAULT_LFE_MODE,
};
#[cfg(feature = "multichannel")]
use rayon::prelude::*;

/// audiowmark 默认搜索路径（无官方包，仅供开发者本地编译后使用）
#[cfg(not(feature = "bundled"))]
const DEFAULT_SEARCH_PATHS: &[&str] = &["audiowmark"];

/// 管道 I/O 的用户态缓冲区大小。
///
/// Windows 匿名管道内核缓冲区默认只有 4 KB，直接用 `io::copy` 的 8 KB 块写会频繁
/// 触发系统调用切换。用 `BufWriter`/`BufReader` 在用户态积累更大的块，可显著减少
/// Windows 上的上下文切换次数；在 macOS/Linux 上也能减少 syscall 开销。
const PIPE_BUF_SIZE: usize = 256 * 1024;
/// audiowmark 0.6.x 候选分数阈值（低于此值通常为伪命中）
const MIN_PATTERN_SCORE: f32 = 1.0;

/// 媒体解码能力摘要（用于 doctor/UI 状态）
#[derive(Debug, Clone, Copy)]
pub struct AudioMediaCapabilities {
    /// 当前媒体后端名称
    pub backend: &'static str,
    /// 是否支持 E-AC-3 解码
    pub eac3_decode: bool,
    /// 是否支持 MP4/M4A 容器解封装
    pub container_mp4: bool,
    /// 是否支持 MKV 容器解封装
    pub container_mkv: bool,
    /// 是否支持 MPEG-TS 容器解封装
    pub container_ts: bool,
}

impl AudioMediaCapabilities {
    /// 以逗号分隔形式返回当前可用容器摘要。
    #[must_use]
    pub fn supported_containers_csv(&self) -> String {
        let mut containers = Vec::new();
        if self.container_mp4 {
            containers.push("mp4");
        }
        if self.container_mkv {
            containers.push("mkv");
        }
        if self.container_ts {
            containers.push("ts");
        }
        if containers.is_empty() {
            return "none".to_string();
        }
        containers.join(",")
    }
}

/// 水印嵌入/检测结果
#[derive(Debug, Clone)]
pub struct DetectResult {
    /// 提取的原始消息 (16 bytes)
    pub raw_message: [u8; MESSAGE_LEN],
    /// 检测模式 (all/single)
    pub pattern: String,
    /// audiowmark 候选分数（仅新输出格式可用）
    pub detect_score: Option<f32>,
    /// 比特错误数
    pub bit_errors: u32,
    /// 是否匹配
    pub match_found: bool,
}

/// 多声道检测结果
#[cfg(feature = "multichannel")]
#[derive(Debug, Clone)]
pub struct MultichannelDetectResult {
    /// 各声道对的检测结果 (pair_index, pair_name, result)
    pub pairs: Vec<(usize, String, Option<DetectResult>)>,
    /// 最佳结果 (置信度最高的一个)
    pub best: Option<DetectResult>,
}

/// 音频水印操作器
#[derive(Debug, Clone)]
pub struct Audio {
    /// audiowmark 二进制路径
    binary_path: PathBuf,
    /// 水印强度 (1-30, 默认 10)
    strength: u8,
    /// 密钥文件路径 (可选)
    key_file: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputAudioFormat {
    Wav,
    Flac,
    Mp3,
    Ogg,
    M4a,
    Alac,
    Mp4,
    Mkv,
    Ts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputPrepareStrategy {
    Direct,
    DecodeToWav,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AwmIoMode {
    Pipe,
    File,
}

struct TempDirGuard {
    path: PathBuf,
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct PreparedInput {
    path: PathBuf,
    _guard: Option<TempDirGuard>,
}

static PIPE_IO_FALLBACK_WARNED: OnceLock<()> = OnceLock::new();

#[cfg(feature = "multichannel")]
#[derive(Debug)]
struct EmbedStepTaskResult {
    step_idx: usize,
    step: RouteStep,
    outcome: Result<MultichannelAudio>,
}

#[cfg(feature = "multichannel")]
#[derive(Debug)]
struct DetectStepTaskResult {
    step_idx: usize,
    step: RouteStep,
    outcome: Option<DetectResult>,
}

impl Audio {
    #[cfg(windows)]
    fn audiowmark_command(&self) -> Command {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new(&self.binary_path);
        // CREATE_NO_WINDOW: avoid flashing a console window when invoking audiowmark.
        cmd.creation_flags(0x0800_0000);
        cmd
    }

    #[cfg(not(windows))]
    fn audiowmark_command(&self) -> Command {
        Command::new(&self.binary_path)
    }

    /// 创建 Audio 实例，自动搜索 audiowmark
    pub fn new() -> Result<Self> {
        Self::new_with_fallback_path(None)
    }

    pub(crate) fn new_with_fallback_path(fallback_path: Option<&Path>) -> Result<Self> {
        let binary_path = Self::resolve_binary(fallback_path)?;
        Ok(Self {
            binary_path,
            strength: 10,
            key_file: None,
        })
    }

    /// 指定 audiowmark 路径创建实例
    pub fn with_binary<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(Error::AudiowmarkNotFound);
        }
        Ok(Self {
            binary_path: path,
            strength: 10,
            key_file: None,
        })
    }

    /// 设置水印强度 (1-30)
    #[must_use]
    pub fn strength(mut self, strength: u8) -> Self {
        self.strength = strength.clamp(1, 30);
        self
    }

    /// 设置密钥文件
    #[must_use]
    pub fn key_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.key_file = Some(path.as_ref().to_path_buf());
        self
    }

    /// 返回 audiowmark 二进制路径
    #[must_use]
    pub fn binary_path(&self) -> &Path {
        &self.binary_path
    }

    /// 返回当前媒体解码能力摘要。
    #[must_use]
    pub fn media_capabilities(&self) -> AudioMediaCapabilities {
        media_capabilities()
    }

    /// 嵌入水印消息到音频
    ///
    /// # Arguments
    /// - `input`: 输入音频路径
    /// - `output`: 输出音频路径
    /// - `message`: 16 字节消息
    pub fn embed<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        message: &[u8; MESSAGE_LEN],
    ) -> Result<()> {
        validate_embed_output_path(output.as_ref())?;
        let prepared = prepare_input_for_audiowmark(input.as_ref(), "embed_input")?;
        let hex = bytes_to_hex(message);
        run_audiowmark_add_prepared(self, &prepared.path, output.as_ref(), &hex)
    }

    /// 便捷方法：编码消息并嵌入
    pub fn embed_with_tag<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        version: u8,
        tag: &Tag,
        hmac_key: &[u8],
    ) -> Result<[u8; MESSAGE_LEN]> {
        let message = message::encode(version, tag, hmac_key)?;
        self.embed(input, output, &message)?;
        Ok(message)
    }

    /// 从音频检测/提取水印
    ///
    /// # Arguments
    /// - `input`: 音频文件路径
    ///
    /// # Returns
    /// 检测结果，如果没有检测到水印返回 None
    pub fn detect<P: AsRef<Path>>(&self, input: P) -> Result<Option<DetectResult>> {
        let prepared = prepare_input_for_audiowmark(input.as_ref(), "detect_input")?;
        let output = run_audiowmark_get_prepared(self, &prepared.path)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(parse_detect_output(&stdout, &stderr))
    }

    /// 便捷方法：检测并解码消息
    pub fn detect_and_decode<P: AsRef<Path>>(
        &self,
        input: P,
        hmac_key: &[u8],
    ) -> Result<Option<crate::message::MessageResult>> {
        match self.detect(input)? {
            Some(result) => {
                let decoded = message::decode(&result.raw_message, hmac_key)?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    /// 检查 audiowmark 是否可用
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.audiowmark_command()
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 获取 audiowmark 版本
    pub fn version(&self) -> Result<String> {
        let output = self
            .audiowmark_command()
            .arg("--version")
            .output()
            .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }

    /// 多声道嵌入：将水印嵌入所有立体声对
    ///
    /// 流程：
    /// 1. 加载多声道音频
    /// 2. 拆分为立体声对
    /// 3. 对每个立体声对嵌入相同的水印
    /// 4. 合并回多声道音频
    ///
    /// # Arguments
    /// - `input`: 输入音频路径 (WAV/FLAC)
    /// - `output`: 输出音频路径 (WAV)
    /// - `message`: 16 字节消息
    /// - `layout`: 可选的声道布局 (自动检测或手动指定，用于区分 7.1 和 5.1.2)
    #[cfg(feature = "multichannel")]
    pub fn embed_multichannel<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        message: &[u8; MESSAGE_LEN],
        layout: Option<ChannelLayout>,
    ) -> Result<()> {
        let input = input.as_ref();
        let output = output.as_ref();
        validate_embed_output_path(output)?;

        if media::adm_bwav::probe_adm_bwf(input)?.is_some() {
            return media::adm_embed::embed_adm_multichannel(self, input, output, message, layout);
        }

        let mut prepared_fallback: Option<PreparedInput> = None;

        // 加载多声道音频以检测声道数。
        // 优先尝试原始输入，避免对可直接读取的 WAV/FLAC 先做不必要的临时解码。
        // 若失败则先尝试内存解码管线（DecodedPcm → MultichannelAudio，无临时文件）；
        // 仍失败则 prepare/临时文件兜底；最终失败回退到立体声路径。
        let mut audio = match MultichannelAudio::from_file(input) {
            Ok(a) => a,
            Err(Error::InvalidInput(_)) => {
                // 内存管线：decode → MultichannelAudio，跳过临时文件
                match decode_media_to_pcm_i32(input).and_then(decoded_pcm_into_multichannel) {
                    Ok(a) => {
                        // 立体声：字节管线直接完成，无需继续路由
                        if a.num_channels() == 2 {
                            let wav_bytes = a.to_wav_bytes()?;
                            let out_bytes = run_audiowmark_add_bytes(
                                self,
                                wav_bytes,
                                &bytes_to_hex(message),
                            )?;
                            return MultichannelAudio::from_wav_bytes(&out_bytes)?.to_wav(output);
                        }
                        a
                    }
                    Err(_) => {
                        // 兜底：传统临时文件路径
                        let prepared =
                            prepare_input_for_audiowmark(input, "embed_multichannel_input")?;
                        match MultichannelAudio::from_file(&prepared.path) {
                            Ok(a) => {
                                prepared_fallback = Some(prepared);
                                a
                            }
                            Err(Error::InvalidInput(_)) => {
                                return self.embed(prepared.path.as_path(), output, message);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
            Err(e) => return Err(e),
        };
        let num_channels = audio.num_channels();
        // stereo_input 仅用于 prepared_fallback 路径的立体声兜底
        let stereo_input = prepared_fallback
            .as_ref()
            .map_or(input, |prepared| prepared.path.as_path());

        // 如果是立体声（来自 prepared_fallback 路径），直接使用普通方法
        if num_channels == 2 {
            return self.embed(stereo_input, output, message);
        }

        // 确定声道布局
        let layout = layout.unwrap_or_else(|| audio.layout());
        validate_layout_channels(layout, num_channels)?;
        let route_plan = build_smart_route_plan(layout, num_channels, DEFAULT_LFE_MODE);
        log_route_warnings("embed", input, &route_plan.warnings);

        let executable_steps: Vec<(usize, RouteStep)> = route_plan
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| !matches!(step.mode, RouteMode::Skip { .. }))
            .map(|(idx, step)| (idx, step.clone()))
            .collect();
        let parallelism = compute_route_parallelism(executable_steps.len());
        let mut step_results = with_route_thread_pool(parallelism, || {
            executable_steps
                .par_iter()
                .map(|(step_idx, step)| EmbedStepTaskResult {
                    step_idx: *step_idx,
                    step: step.clone(),
                    outcome: run_embed_step_task(self, &audio, step, message),
                })
                .collect::<Vec<_>>()
        })?;

        apply_embed_step_results(&mut audio, &mut step_results);

        // 当前仅支持输出 WAV（FLAC 输出已暂时下线）。
        audio.to_wav(output)?;

        Ok(())
    }

    /// 多声道检测：从所有立体声对检测水印
    ///
    /// 返回每个声道对的检测结果，以及最佳结果
    #[cfg(feature = "multichannel")]
    pub fn detect_multichannel<P: AsRef<Path>>(
        &self,
        input: P,
        layout: Option<ChannelLayout>,
    ) -> Result<MultichannelDetectResult> {
        let input = input.as_ref();
        if media::adm_bwav::probe_adm_bwf(input)?.is_some() {
            return Err(Error::AdmUnsupported(
                "ADM/BWF detect is not supported in this phase".to_string(),
            ));
        }

        let mut prepared_fallback: Option<PreparedInput> = None;

        // 加载多声道音频以检测声道数。
        // 优先尝试原始输入，避免对可直接读取的 WAV/FLAC 先做不必要的临时解码。
        // 若失败则先尝试内存解码管线（DecodedPcm → MultichannelAudio，无临时文件）；
        // 仍失败则 prepare/临时文件兜底；最终失败回退到立体声路径。
        let audio = match MultichannelAudio::from_file(input) {
            Ok(a) => a,
            Err(Error::InvalidInput(_)) => {
                // 内存管线：decode → MultichannelAudio，跳过临时文件
                match decode_media_to_pcm_i32(input).and_then(decoded_pcm_into_multichannel) {
                    Ok(a) => {
                        // 立体声：字节管线直接完成，无需继续路由
                        if a.num_channels() == 2 {
                            let wav_bytes = a.to_wav_bytes()?;
                            let raw = run_audiowmark_get_bytes(self, wav_bytes)?;
                            let stdout = String::from_utf8_lossy(&raw.stdout);
                            let stderr = String::from_utf8_lossy(&raw.stderr);
                            let result = parse_detect_output(&stdout, &stderr);
                            return Ok(MultichannelDetectResult {
                                pairs: vec![(0, "FL+FR".to_string(), result.clone())],
                                best: result,
                            });
                        }
                        a
                    }
                    Err(_) => {
                        // 兜底：传统临时文件路径
                        let prepared =
                            prepare_input_for_audiowmark(input, "detect_multichannel_input")?;
                        match MultichannelAudio::from_file(&prepared.path) {
                            Ok(a) => {
                                prepared_fallback = Some(prepared);
                                a
                            }
                            Err(Error::InvalidInput(_)) => {
                                let result = self.detect(prepared.path.as_path())?;
                                return Ok(MultichannelDetectResult {
                                    pairs: vec![(0, "FL+FR".to_string(), result.clone())],
                                    best: result,
                                });
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
            Err(e) => return Err(e),
        };
        let num_channels = audio.num_channels();
        // stereo_input 仅用于 prepared_fallback 路径的立体声兜底
        let stereo_input = prepared_fallback
            .as_ref()
            .map_or(input, |prepared| prepared.path.as_path());

        // 如果是立体声（来自 prepared_fallback 路径），直接使用普通方法
        if num_channels == 2 {
            let result = self.detect(stereo_input)?;
            return Ok(MultichannelDetectResult {
                pairs: vec![(0, "FL+FR".to_string(), result.clone())],
                best: result,
            });
        }

        // 确定声道布局
        let layout = layout.unwrap_or_else(|| audio.layout());
        validate_layout_channels(layout, num_channels)?;
        let route_plan = build_smart_route_plan(layout, num_channels, DEFAULT_LFE_MODE);
        log_route_warnings("detect", input, &route_plan.warnings);

        let detect_steps: Vec<(usize, RouteStep)> = route_plan
            .detectable_steps()
            .into_iter()
            .map(|(idx, step)| (idx, step.clone()))
            .collect();

        // 顺序检测声道对；bit_errors == 0 是全局最优（不可能更低），立即返回，
        // 跳过剩余声道对。有损伤的文件（所有对 bit_errors > 0）行为与之前相同。
        let mut step_results: Vec<DetectStepTaskResult> = Vec::with_capacity(detect_steps.len());
        for (step_idx, step) in &detect_steps {
            let outcome = run_detect_step_task(self, &audio, step)?;
            let perfect = outcome.as_ref().map_or(false, |r| r.bit_errors == 0);
            step_results.push(DetectStepTaskResult {
                step_idx: *step_idx,
                step: step.clone(),
                outcome,
            });
            if perfect {
                break;
            }
        }

        Ok(finalize_detect_step_results(step_results))
    }

    /// 便捷方法：多声道嵌入 (使用 Tag)
    #[cfg(feature = "multichannel")]
    pub fn embed_multichannel_with_tag<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        version: u8,
        tag: &Tag,
        hmac_key: &[u8],
        layout: Option<ChannelLayout>,
    ) -> Result<[u8; MESSAGE_LEN]> {
        let message = message::encode(version, tag, hmac_key)?;
        self.embed_multichannel(input, output, &message, layout)?;
        Ok(message)
    }

    /// 便捷方法：多声道检测并解码
    #[cfg(feature = "multichannel")]
    pub fn detect_multichannel_and_decode<P: AsRef<Path>>(
        &self,
        input: P,
        hmac_key: &[u8],
        layout: Option<ChannelLayout>,
    ) -> Result<Option<crate::message::MessageResult>> {
        let result = self.detect_multichannel(input, layout)?;
        match result.best {
            Some(detect) => {
                let decoded = message::decode(&detect.raw_message, hmac_key)?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    /// 搜索 audiowmark 二进制
    #[cfg(not(feature = "bundled"))]
    fn find_binary() -> Option<PathBuf> {
        for path in DEFAULT_SEARCH_PATHS {
            let p = Path::new(path);
            if p.exists() {
                return Some(p.to_path_buf());
            }
        }

        // 尝试 which
        if let Ok(output) = Command::new("which").arg("audiowmark").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }

        None
    }

    #[cfg(not(feature = "bundled"))]
    fn strict_runtime_enabled() -> bool {
        std::env::var("AWMKIT_RUNTIME_STRICT")
            .ok()
            .map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false)
    }

    #[cfg(feature = "bundled")]
    fn resolve_binary(_fallback_path: Option<&Path>) -> Result<PathBuf> {
        crate::bundled::ensure_extracted()
    }

    #[cfg(not(feature = "bundled"))]
    fn resolve_binary(fallback_path: Option<&Path>) -> Result<PathBuf> {
        if let Some(path) = fallback_path {
            if let Ok(audio) = Self::with_binary(path) {
                return Ok(audio.binary_path);
            }
        }

        if Self::strict_runtime_enabled() {
            return Err(Error::AudiowmarkNotFound);
        }

        Self::find_binary().ok_or(Error::AudiowmarkNotFound)
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            binary_path: PathBuf::from("audiowmark"),
            strength: 10,
            key_file: None,
        })
    }
}

fn run_audiowmark_add_prepared(
    audio: &Audio,
    prepared_input: &Path,
    output: &Path,
    message_hex: &str,
) -> Result<()> {
    if matches!(effective_awmiomode(), AwmIoMode::File) {
        return run_audiowmark_add_file(audio, prepared_input, output, message_hex);
    }
    match run_audiowmark_add_pipe(audio, prepared_input, output, message_hex) {
        Ok(()) => Ok(()),
        Err(err) if should_fallback_pipe_error(&err) => {
            warn_pipe_fallback_once("add", &err);
            run_audiowmark_add_file(audio, prepared_input, output, message_hex)
        }
        Err(err) => Err(err),
    }
}

fn run_audiowmark_get_prepared(audio: &Audio, prepared_input: &Path) -> Result<Output> {
    if matches!(effective_awmiomode(), AwmIoMode::File) {
        return run_audiowmark_get_file(audio, prepared_input);
    }
    match run_audiowmark_get_pipe(audio, prepared_input) {
        Ok(output) => Ok(output),
        Err(err) if should_fallback_pipe_error(&err) => {
            warn_pipe_fallback_once("get", &err);
            run_audiowmark_get_file(audio, prepared_input)
        }
        Err(err) => Err(err),
    }
}

fn run_audiowmark_add_file(
    audio: &Audio,
    prepared_input: &Path,
    output: &Path,
    message_hex: &str,
) -> Result<()> {
    let mut cmd = audio.audiowmark_command();
    cmd.arg("add")
        .arg("--strength")
        .arg(audio.strength.to_string());

    if let Some(ref key_file) = audio.key_file {
        cmd.arg("--key").arg(key_file);
    }

    cmd.arg(prepared_input).arg(output).arg(message_hex);
    let output = cmd
        .output()
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::AudiowmarkExec(stderr.to_string()));
    }
    Ok(())
}

fn run_audiowmark_get_file(audio: &Audio, prepared_input: &Path) -> Result<Output> {
    let mut cmd = audio.audiowmark_command();
    cmd.arg("get");

    if let Some(ref key_file) = audio.key_file {
        cmd.arg("--key").arg(key_file);
    }

    cmd.arg(prepared_input);
    cmd.output()
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))
}

fn run_audiowmark_add_bytes(
    audio: &Audio,
    input_bytes: Vec<u8>,
    message_hex: &str,
) -> Result<Vec<u8>> {
    if matches!(effective_awmiomode(), AwmIoMode::File) {
        return run_audiowmark_add_bytes_file(audio, input_bytes, message_hex);
    }
    match run_audiowmark_add_bytes_pipe(audio, &input_bytes, message_hex) {
        Ok(output_bytes) => Ok(output_bytes),
        Err(err) if should_fallback_pipe_error(&err) => {
            warn_pipe_fallback_once("add-bytes", &err);
            run_audiowmark_add_bytes_file(audio, input_bytes, message_hex)
        }
        Err(err) => Err(err),
    }
}

fn run_audiowmark_add_bytes_pipe(
    audio: &Audio,
    input_bytes: &[u8],
    message_hex: &str,
) -> Result<Vec<u8>> {
    let mut cmd = audio.audiowmark_command();
    cmd.arg("add")
        .arg("--strength")
        .arg(audio.strength.to_string())
        .arg("--input-format")
        .arg("wav-pipe")
        .arg("--output-format")
        .arg("wav-pipe");

    if let Some(ref key_file) = audio.key_file {
        cmd.arg("--key").arg(key_file);
    }

    cmd.arg("-").arg("-").arg(message_hex);
    let process_output = run_command_with_stdin(&mut cmd, input_bytes)?;
    if !process_output.status.success() {
        let stderr = String::from_utf8_lossy(&process_output.stderr);
        return Err(Error::AudiowmarkExec(stderr.to_string()));
    }
    if !looks_like_wav_stream(&process_output.stdout) {
        return Err(Error::AudiowmarkExec(
            "pipe output is not a valid WAV stream".to_string(),
        ));
    }
    // audiowmark --output-format wav-pipe 输出 RIFF ffffffff（流式未知长度），
    // 修复大小字段使输出可被 hound 等工具正常读取。
    Ok(normalize_wav_pipe_output(process_output.stdout))
}

fn run_audiowmark_get_bytes(audio: &Audio, input_bytes: Vec<u8>) -> Result<Output> {
    if matches!(effective_awmiomode(), AwmIoMode::File) {
        return run_audiowmark_get_bytes_file(audio, input_bytes);
    }
    match run_audiowmark_get_bytes_pipe(audio, &input_bytes) {
        Ok(output) => Ok(output),
        Err(err) if should_fallback_pipe_error(&err) => {
            warn_pipe_fallback_once("get-bytes", &err);
            run_audiowmark_get_bytes_file(audio, input_bytes)
        }
        Err(err) => Err(err),
    }
}

fn run_audiowmark_get_bytes_pipe(audio: &Audio, input_bytes: &[u8]) -> Result<Output> {
    let mut cmd = audio.audiowmark_command();
    cmd.arg("get");

    if let Some(ref key_file) = audio.key_file {
        cmd.arg("--key").arg(key_file);
    }

    cmd.arg("-");
    let output = run_command_with_stdin(&mut cmd, input_bytes)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if is_pipe_compatibility_error(&stderr) {
            return Err(Error::AudiowmarkExec(stderr.to_string()));
        }
    }
    Ok(output)
}

fn run_audiowmark_add_bytes_file(
    audio: &Audio,
    input_bytes: Vec<u8>,
    message_hex: &str,
) -> Result<Vec<u8>> {
    let temp_dir = create_temp_dir("awmkit_add_bytes_file")?;
    let _guard = TempDirGuard {
        path: temp_dir.clone(),
    };
    let input_path = temp_dir.join("input.wav");
    let output_path = temp_dir.join("output.wav");
    fs::write(&input_path, input_bytes)?;
    run_audiowmark_add_file(audio, &input_path, &output_path, message_hex)?;
    let output_bytes = fs::read(&output_path)?;
    Ok(output_bytes)
}

fn run_audiowmark_get_bytes_file(audio: &Audio, input_bytes: Vec<u8>) -> Result<Output> {
    let temp_dir = create_temp_dir("awmkit_get_bytes_file")?;
    let _guard = TempDirGuard {
        path: temp_dir.clone(),
    };
    let input_path = temp_dir.join("input.wav");
    fs::write(&input_path, input_bytes)?;
    run_audiowmark_get_file(audio, &input_path)
}

fn run_audiowmark_add_pipe(
    audio: &Audio,
    prepared_input: &Path,
    output: &Path,
    message_hex: &str,
) -> Result<()> {
    run_audiowmark_add_pipe_streaming(audio, prepared_input, output, message_hex)
}

fn run_audiowmark_get_pipe(audio: &Audio, prepared_input: &Path) -> Result<Output> {
    let mut cmd = audio.audiowmark_command();
    cmd.arg("get");

    if let Some(ref key_file) = audio.key_file {
        cmd.arg("--key").arg(key_file);
    }

    cmd.arg("-");
    run_command_with_stdin_from_file(&mut cmd, prepared_input)
}

fn run_audiowmark_add_pipe_streaming(
    audio: &Audio,
    prepared_input: &Path,
    output: &Path,
    message_hex: &str,
) -> Result<()> {
    let mut cmd = audio.audiowmark_command();
    cmd.arg("add")
        .arg("--strength")
        .arg(audio.strength.to_string())
        .arg("--input-format")
        .arg("wav-pipe")
        .arg("--output-format")
        .arg("wav-pipe");

    if let Some(ref key_file) = audio.key_file {
        cmd.arg("--key").arg(key_file);
    }
    cmd.arg("-").arg("-").arg(message_hex);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stdin handle".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stdout handle".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stderr handle".to_string()))?;
    let input_file = File::open(prepared_input)?;
    let output_file = File::create(output)?;

    let (status, stdin_result, stdout_result, stderr_result) = std::thread::scope(|scope| {
        let stdin_writer = scope.spawn(move || -> std::io::Result<u64> {
            let mut input_file = input_file;
            let mut stdin = BufWriter::with_capacity(PIPE_BUF_SIZE, stdin);
            let n = std::io::copy(&mut input_file, &mut stdin)?;
            stdin.flush()?;
            Ok(n)
        });
        let stdout_reader = scope.spawn(move || {
            let mut stdout = BufReader::with_capacity(PIPE_BUF_SIZE, stdout);
            let mut output_file = output_file;
            std::io::copy(&mut stdout, &mut output_file)
        });
        let stderr_reader = scope.spawn(move || -> std::io::Result<Vec<u8>> {
            let mut stderr = stderr;
            let mut buf = Vec::new();
            stderr.read_to_end(&mut buf)?;
            Ok(buf)
        });

        let status = child.wait();
        let stdin_result = stdin_writer.join();
        let stdout_result = stdout_reader.join();
        let stderr_result = stderr_reader.join();
        (status, stdin_result, stdout_result, stderr_result)
    });

    let status = status.map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let stdin_copied = stdin_result
        .map_err(|_| Error::AudiowmarkExec("stdin streaming thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let _ = stdin_copied;
    let stdout_copied = stdout_result
        .map_err(|_| Error::AudiowmarkExec("stdout streaming thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    if stdout_copied == 0 {
        return Err(Error::AudiowmarkExec(
            "pipe output is empty; expected WAV stream".to_string(),
        ));
    }
    let stderr_bytes = stderr_result
        .map_err(|_| Error::AudiowmarkExec("stderr streaming thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

    if !status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr_bytes);
        return Err(Error::AudiowmarkExec(stderr_text.to_string()));
    }
    validate_wav_output_file(output)?;

    normalize_wav_pipe_file_in_place(output)?;
    Ok(())
}

fn run_command_with_stdin(cmd: &mut Command, stdin_data: &[u8]) -> Result<Output> {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stdin handle".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stdout handle".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stderr handle".to_string()))?;

    let (status, stdin_result, stdout_result, stderr_result) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || -> std::io::Result<u64> {
            let mut src = std::io::Cursor::new(stdin_data);
            let mut stdin = BufWriter::with_capacity(PIPE_BUF_SIZE, stdin);
            let n = std::io::copy(&mut src, &mut stdin)?;
            stdin.flush()?;
            Ok(n)
        });
        let stdout_reader = scope.spawn(move || -> std::io::Result<Vec<u8>> {
            let mut stdout = BufReader::with_capacity(PIPE_BUF_SIZE, stdout);
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf)?;
            Ok(buf)
        });
        let stderr_reader = scope.spawn(move || -> std::io::Result<Vec<u8>> {
            let mut stderr = stderr;
            let mut buf = Vec::new();
            stderr.read_to_end(&mut buf)?;
            Ok(buf)
        });
        let status = child.wait();
        (
            status,
            writer.join(),
            stdout_reader.join(),
            stderr_reader.join(),
        )
    });

    let status = status.map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    stdin_result
        .map_err(|_| Error::AudiowmarkExec("stdin writer thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let stdout = stdout_result
        .map_err(|_| Error::AudiowmarkExec("stdout reader thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let stderr = stderr_result
        .map_err(|_| Error::AudiowmarkExec("stderr reader thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn run_command_with_stdin_from_file(cmd: &mut Command, input_path: &Path) -> Result<Output> {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stdin handle".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stdout handle".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::AudiowmarkExec("failed to take stderr handle".to_string()))?;
    let input_file = File::open(input_path)?;

    let (status, stdin_result, stdout_result, stderr_result) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || -> std::io::Result<u64> {
            let mut input_file = input_file;
            let mut stdin = BufWriter::with_capacity(PIPE_BUF_SIZE, stdin);
            let n = std::io::copy(&mut input_file, &mut stdin)?;
            stdin.flush()?;
            Ok(n)
        });
        let stdout_reader = scope.spawn(move || -> std::io::Result<Vec<u8>> {
            let mut stdout = BufReader::with_capacity(PIPE_BUF_SIZE, stdout);
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf)?;
            Ok(buf)
        });
        let stderr_reader = scope.spawn(move || -> std::io::Result<Vec<u8>> {
            let mut stderr = stderr;
            let mut buf = Vec::new();
            stderr.read_to_end(&mut buf)?;
            Ok(buf)
        });
        let status = child.wait();
        (
            status,
            writer.join(),
            stdout_reader.join(),
            stderr_reader.join(),
        )
    });

    let status = status.map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    stdin_result
        .map_err(|_| Error::AudiowmarkExec("stdin writer thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let stdout = stdout_result
        .map_err(|_| Error::AudiowmarkExec("stdout reader thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;
    let stderr = stderr_result
        .map_err(|_| Error::AudiowmarkExec("stderr reader thread panicked".to_string()))?
        .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn effective_awmiomode() -> AwmIoMode {
    if std::env::var("AWMKIT_DISABLE_PIPE_IO")
        .ok()
        .is_some_and(|value| parse_env_flag(&value))
    {
        AwmIoMode::File
    } else {
        AwmIoMode::Pipe
    }
}

fn parse_env_flag(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
}

fn looks_like_wav_stream(bytes: &[u8]) -> bool {
    if bytes.len() < 12 {
        return false;
    }
    let riff_like = &bytes[0..4];
    let wave = &bytes[8..12];
    (riff_like == b"RIFF" || riff_like == b"RF64" || riff_like == b"BW64") && wave == b"WAVE"
}

/// audiowmark `--output-format wav-pipe` 输出 RIFF/data chunk size 为 `0xFFFF_FFFF`。
/// 修复大小字段，使返回的字节序列成为合法的标准 WAV，可被 hound 等工具直接读取。
fn normalize_wav_pipe_output(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || bytes[4..8] != [0xFF_u8; 4] {
        return bytes;
    }

    // 修复 RIFF chunk 大小
    let riff_payload = u32::try_from(bytes.len().saturating_sub(8)).unwrap_or(u32::MAX);
    bytes[4..8].copy_from_slice(&riff_payload.to_le_bytes());

    // 扫描 sub-chunk，找到 data chunk 并修复其大小
    let mut pos = 12usize;
    while pos.saturating_add(8) <= bytes.len() {
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]);
        if &bytes[pos..pos + 4] == b"data" {
            let data_payload =
                u32::try_from(bytes.len().saturating_sub(pos + 8)).unwrap_or(u32::MAX);
            bytes[pos + 4..pos + 8].copy_from_slice(&data_payload.to_le_bytes());
            break;
        }
        let chunk_size_usize = usize::try_from(chunk_size).unwrap_or(usize::MAX);
        let padded = usize::from(chunk_size_usize % 2 != 0);
        let next_pos = pos
            .saturating_add(8)
            .saturating_add(chunk_size_usize)
            .saturating_add(padded);
        if next_pos <= pos {
            break;
        }
        pos = next_pos;
    }

    bytes
}

fn validate_wav_output_file(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    let mut header = [0_u8; 12];
    file.read_exact(&mut header)
        .map_err(|e| Error::AudiowmarkExec(format!("failed to read output header: {e}")))?;
    if !looks_like_wav_stream(&header) {
        return Err(Error::AudiowmarkExec(
            "pipe output is not a valid WAV stream".to_string(),
        ));
    }
    Ok(())
}

fn normalize_wav_pipe_file_in_place(path: &Path) -> Result<()> {
    let mut file = fs::OpenOptions::new().read(true).write(true).open(path)?;
    let file_len = file.metadata()?.len();
    if file_len < 12 {
        return Ok(());
    }

    let mut header = [0_u8; 12];
    file.read_exact(&mut header)?;
    if &header[0..4] != b"RIFF" || header[4..8] != [0xFF_u8; 4] || &header[8..12] != b"WAVE" {
        return Ok(());
    }

    let riff_payload = u32::try_from(file_len.saturating_sub(8)).unwrap_or(u32::MAX);
    file.seek(SeekFrom::Start(4))?;
    file.write_all(&riff_payload.to_le_bytes())?;

    let mut pos = 12_u64;
    while pos.saturating_add(8) <= file_len {
        file.seek(SeekFrom::Start(pos))?;
        let mut chunk_header = [0_u8; 8];
        file.read_exact(&mut chunk_header)?;
        let chunk_size = u32::from_le_bytes([
            chunk_header[4],
            chunk_header[5],
            chunk_header[6],
            chunk_header[7],
        ]);

        if &chunk_header[0..4] == b"data" {
            let data_payload = u32::try_from(file_len.saturating_sub(pos + 8)).unwrap_or(u32::MAX);
            file.seek(SeekFrom::Start(pos + 4))?;
            file.write_all(&data_payload.to_le_bytes())?;
            break;
        }

        let chunk_size_usize = usize::try_from(chunk_size).unwrap_or(usize::MAX);
        let padded = usize::from(chunk_size_usize % 2 != 0);
        let next_pos = pos
            .saturating_add(8)
            .saturating_add(u64::try_from(chunk_size_usize).unwrap_or(u64::MAX))
            .saturating_add(u64::try_from(padded).unwrap_or(0));
        if next_pos <= pos {
            break;
        }
        pos = next_pos;
    }

    Ok(())
}

fn should_fallback_pipe_error(err: &Error) -> bool {
    matches!(err, Error::AudiowmarkExec(_) | Error::Io(_))
}

fn is_pipe_compatibility_error(stderr: &str) -> bool {
    let normalized = stderr.to_ascii_lowercase();
    normalized.contains("unsupported option")
        || normalized.contains("unrecognized option")
        || normalized.contains("invalid option")
        || normalized.contains("cannot open -")
        || normalized.contains("cannot open '-'")
        || normalized.contains("stdin")
}

fn warn_pipe_fallback_once(operation: &str, err: &Error) {
    if PIPE_IO_FALLBACK_WARNED.get().is_none() {
        let _ = PIPE_IO_FALLBACK_WARNED.set(());
        eprintln!(
            "Warning: audiowmark pipe I/O failed for {operation}, fallback to file I/O: {err}"
        );
    }
}

#[cfg(feature = "multichannel")]
fn validate_layout_channels(layout: ChannelLayout, source_channels: usize) -> Result<()> {
    let layout_channels = usize::from(layout.channels());
    if layout_channels != source_channels {
        return Err(Error::InvalidInput(format!(
            "channel layout mismatch: layout={}ch, source={}ch",
            layout_channels, source_channels
        )));
    }
    Ok(())
}

#[cfg(feature = "multichannel")]
fn log_route_warnings(operation: &str, input: &Path, warnings: &[String]) {
    for warning in warnings {
        eprintln!(
            "Warning: smart route fallback ({operation}) for {}: {warning}",
            input.display()
        );
    }
}

#[cfg(feature = "multichannel")]
fn compute_route_parallelism(step_count: usize) -> usize {
    if step_count <= 1 {
        return 1;
    }
    if let Some(forced) = route_parallelism_override() {
        return step_count.min(forced).max(1);
    }
    // Benchmarks show single-threaded is fastest for typical route sizes (≤20 steps):
    // thread-pool overhead (~200ms) outweighs the parallelism benefit.
    // Users can still opt-in via AWMKIT_ROUTE_PARALLELISM=N.
    1
}

#[cfg(feature = "multichannel")]
fn route_parallelism_override() -> Option<usize> {
    let raw = std::env::var("AWMKIT_ROUTE_PARALLELISM").ok()?;
    let parsed = raw.trim().parse::<usize>().ok()?;
    Some(parsed.max(1))
}

#[cfg(feature = "multichannel")]
fn with_route_thread_pool<F, R>(parallelism: usize, f: F) -> Result<R>
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    if parallelism <= 1 {
        return Ok(f());
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parallelism)
        .build()
        .map_err(|err| {
            Error::AudiowmarkExec(format!("failed to build route thread pool: {err}"))
        })?;
    Ok(pool.install(f))
}

#[cfg(feature = "multichannel")]
fn run_embed_step_task(
    audio_engine: &Audio,
    source_audio: &MultichannelAudio,
    step: &RouteStep,
    message: &[u8; MESSAGE_LEN],
) -> Result<MultichannelAudio> {
    let stereo = build_stereo_for_route_step(source_audio, step)?;
    let input_bytes = stereo.to_wav_bytes()?;
    let output_bytes = run_audiowmark_add_bytes(audio_engine, input_bytes, &bytes_to_hex(message))?;
    MultichannelAudio::from_wav_bytes(&output_bytes)
}

#[cfg(feature = "multichannel")]
fn run_detect_step_task(
    audio_engine: &Audio,
    source_audio: &MultichannelAudio,
    step: &RouteStep,
) -> Result<Option<DetectResult>> {
    let stereo = build_stereo_for_route_step(source_audio, step)?;
    let input_bytes = stereo.to_wav_bytes()?;
    let output = run_audiowmark_get_bytes(audio_engine, input_bytes)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(parse_detect_output(&stdout, &stderr))
}

#[cfg(feature = "multichannel")]
fn apply_embed_step_results(
    target: &mut MultichannelAudio,
    step_results: &mut [EmbedStepTaskResult],
) {
    step_results.sort_by_key(|item| item.step_idx);
    for step_result in step_results {
        match &step_result.outcome {
            Ok(processed) => {
                if let Err(err) = apply_processed_route_step(target, &step_result.step, processed) {
                    eprintln!(
                        "Warning: Failed to apply routed embed result for {}: {err}",
                        step_result.step.name
                    );
                }
            }
            Err(err) => {
                eprintln!(
                    "Warning: Failed to embed in route step {}: {err}",
                    step_result.step.name
                );
            }
        }
    }
}

#[cfg(feature = "multichannel")]
fn finalize_detect_step_results(
    mut step_results: Vec<DetectStepTaskResult>,
) -> MultichannelDetectResult {
    step_results.sort_by_key(|item| item.step_idx);

    let mut pairs_results = Vec::with_capacity(step_results.len());
    let mut best: Option<DetectResult> = None;
    for (result_idx, step_result) in step_results.into_iter().enumerate() {
        let outcome = step_result.outcome;
        if let Some(ref detected) = outcome {
            if best.is_none()
                || detected.bit_errors < best.as_ref().map_or(u32::MAX, |value| value.bit_errors)
            {
                best = Some(detected.clone());
            }
        }
        pairs_results.push((result_idx, step_result.step.name.clone(), outcome));
    }

    MultichannelDetectResult {
        pairs: pairs_results,
        best,
    }
}

#[cfg(feature = "multichannel")]
fn build_stereo_for_route_step(
    audio: &MultichannelAudio,
    step: &RouteStep,
) -> Result<MultichannelAudio> {
    match step.mode {
        RouteMode::Pair(left, right) => {
            let left_samples = audio.channel_samples(left)?.to_vec();
            let right_samples = audio.channel_samples(right)?.to_vec();
            MultichannelAudio::new(
                vec![left_samples, right_samples],
                audio.sample_rate(),
                audio.sample_format(),
            )
        }
        RouteMode::Mono(channel) => {
            let mono = audio.channel_samples(channel)?.to_vec();
            MultichannelAudio::new(
                vec![mono.clone(), mono],
                audio.sample_rate(),
                audio.sample_format(),
            )
        }
        RouteMode::Skip { .. } => Err(Error::InvalidInput(
            "cannot build stereo input from skip route step".to_string(),
        )),
    }
}

#[cfg(feature = "multichannel")]
fn apply_processed_route_step(
    target: &mut MultichannelAudio,
    step: &RouteStep,
    processed: &MultichannelAudio,
) -> Result<()> {
    if processed.num_channels() != 2 {
        return Err(Error::InvalidInput(format!(
            "processed stereo route output expects 2 channels, got {}",
            processed.num_channels()
        )));
    }

    let left = processed.channel_samples(0)?.to_vec();
    match step.mode {
        RouteMode::Pair(left_index, right_index) => {
            let right = processed.channel_samples(1)?.to_vec();
            target.replace_channel_samples(left_index, left)?;
            target.replace_channel_samples(right_index, right)?;
            Ok(())
        }
        RouteMode::Mono(channel) => target.replace_channel_samples(channel, left),
        RouteMode::Skip { .. } => Ok(()),
    }
}

/// 解析 audiowmark get 输出
fn parse_detect_output(stdout: &str, stderr: &str) -> Option<DetectResult> {
    // 查找 pattern 行
    // 格式: "pattern  all 0101c1d05978131b57f7deb8e22a0b78"
    // 或:   "pattern   single 0101c1d05978131b57f7deb8e22a0b78 0"
    // 或:   "pattern  0:00 00000000000000000000000000000000 0.000 -0.001 CLIP-B" (audiowmark 0.6.x)

    let combined = format!("{stdout}\n{stderr}");

    for line in combined.lines() {
        let line = line.trim();
        if line.starts_with("pattern") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let pattern = parts[1].to_string();
                let hex = parts[2];

                if let Some(raw_message) = hex_to_bytes(hex) {
                    // audiowmark 在未命中时会输出全 0 消息；这不应视为有效水印。
                    if raw_message.iter().all(|byte| *byte == 0) {
                        continue;
                    }

                    // audiowmark 0.6.x 输出中，第 4 列是浮点分数，低分通常是伪命中。
                    // 旧版格式第 4 列是 bit_errors（整数），此时不做 score 过滤。
                    let mut detect_score = None;
                    if let Some(score_token) = parts.get(3) {
                        if score_token.contains('.') {
                            if let Ok(score) = score_token.parse::<f32>() {
                                detect_score = Some(score);
                                if score < MIN_PATTERN_SCORE {
                                    continue;
                                }
                            }
                        }
                    }

                    let bit_errors = if parts.len() >= 4 {
                        parts[3].parse().unwrap_or(0)
                    } else {
                        0
                    };

                    return Some(DetectResult {
                        raw_message,
                        pattern,
                        detect_score,
                        bit_errors,
                        match_found: true,
                    });
                }
            }
        }
    }

    // 没有检测到水印
    None
}

fn extension_format_hint(path: &Path) -> Option<InputAudioFormat> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("wav") => Some(InputAudioFormat::Wav),
        Some("flac") => Some(InputAudioFormat::Flac),
        Some("mp3") => Some(InputAudioFormat::Mp3),
        Some("ogg" | "opus") => Some(InputAudioFormat::Ogg),
        Some("m4a") => Some(InputAudioFormat::M4a),
        Some("alac") => Some(InputAudioFormat::Alac),
        Some("mp4") => Some(InputAudioFormat::Mp4),
        Some("mkv" | "mka") => Some(InputAudioFormat::Mkv),
        Some("ts" | "m2ts" | "m2t") => Some(InputAudioFormat::Ts),
        _ => None,
    }
}

fn sniff_input_audio_format(path: &Path) -> Option<InputAudioFormat> {
    let mut file = File::open(path).ok()?;
    let mut header = [0_u8; 16];
    let len = file.read(&mut header).ok()?;
    if len == 0 {
        return None;
    }
    let data = &header[..len];

    if data.starts_with(b"fLaC") {
        return Some(InputAudioFormat::Flac);
    }
    if len >= 12
        && (data.starts_with(b"RIFF") || data.starts_with(b"RF64"))
        && &data[8..12] == b"WAVE"
    {
        return Some(InputAudioFormat::Wav);
    }
    if data.starts_with(b"OggS") {
        return Some(InputAudioFormat::Ogg);
    }
    if len >= 8 && &data[4..8] == b"ftyp" {
        return Some(InputAudioFormat::Mp4);
    }
    if data.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Some(InputAudioFormat::Mkv);
    }
    if data[0] == 0x47 {
        return Some(InputAudioFormat::Ts);
    }
    if data.starts_with(b"ID3") {
        return Some(InputAudioFormat::Mp3);
    }
    if len >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
        return Some(InputAudioFormat::Mp3);
    }

    None
}

fn classify_input_prepare_strategy(path: &Path) -> InputPrepareStrategy {
    if let Some(sniffed) = sniff_input_audio_format(path) {
        return match sniffed {
            InputAudioFormat::Wav | InputAudioFormat::Flac => InputPrepareStrategy::Direct,
            _ => InputPrepareStrategy::DecodeToWav,
        };
    }

    if extension_format_hint(path).is_some() {
        return InputPrepareStrategy::DecodeToWav;
    }

    InputPrepareStrategy::DecodeToWav
}

fn validate_embed_output_path(path: &Path) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("wav") => Ok(()),
        Some(ext) => Err(Error::InvalidOutputFormat(format!(
            "unsupported output format: .{ext} (supported: wav)"
        ))),
        None => Err(Error::InvalidOutputFormat(
            "output file has no extension (supported: wav)".to_string(),
        )),
    }
}

fn prepare_input_for_audiowmark(input: &Path, purpose: &str) -> Result<PreparedInput> {
    match classify_input_prepare_strategy(input) {
        InputPrepareStrategy::Direct => Ok(PreparedInput {
            path: input.to_path_buf(),
            _guard: None,
        }),
        InputPrepareStrategy::DecodeToWav => {
            let temp_dir = create_temp_dir(purpose)?;
            let temp_wav = temp_dir.join("input.wav");
            decode_to_wav(input, &temp_wav)?;
            Ok(PreparedInput {
                path: temp_wav,
                _guard: Some(TempDirGuard { path: temp_dir }),
            })
        }
    }
}

fn create_temp_dir(prefix: &str) -> Result<PathBuf> {
    use std::fs;

    let path = std::env::temp_dir().join(format!(
        "{prefix}_{}_{:?}_{}",
        std::process::id(),
        std::thread::current().id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    fs::create_dir_all(&path)?;
    Ok(path)
}

fn decode_to_wav(input: &Path, output_wav: &Path) -> Result<()> {
    use hound::{SampleFormat as HoundSampleFormat, WavSpec, WavWriter};

    let decoded = decode_media_to_pcm_i32(input)?;
    let spec = WavSpec {
        channels: decoded.channels,
        sample_rate: decoded.sample_rate,
        bits_per_sample: decoded.bits_per_sample,
        sample_format: HoundSampleFormat::Int,
    };

    let mut writer = WavWriter::create(output_wav, spec)
        .map_err(|e| Error::InvalidInput(format!("failed to create WAV: {e}")))?;

    for sample in decoded.samples {
        let clamped = clamp_sample_to_bits(sample, decoded.bits_per_sample);
        if decoded.bits_per_sample == 16 {
            let sample_i16 = i16::try_from(clamped).map_err(|_| {
                Error::InvalidInput(format!("16-bit sample out of range after clamp: {clamped}"))
            })?;
            writer
                .write_sample(sample_i16)
                .map_err(|e| Error::InvalidInput(format!("write error: {e}")))?;
        } else {
            writer
                .write_sample(clamped)
                .map_err(|e| Error::InvalidInput(format!("write error: {e}")))?;
        }
    }

    writer
        .finalize()
        .map_err(|e| Error::InvalidInput(format!("finalize error: {e}")))?;
    Ok(())
}

pub(crate) struct DecodedPcm {
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) bits_per_sample: u16,
    pub(crate) samples: Vec<i32>,
}

#[cfg(feature = "ffmpeg-decode")]
fn decode_media_to_pcm_i32(input: &Path) -> Result<DecodedPcm> {
    media::decode_media_to_pcm_i32(input)
}

#[cfg(not(feature = "ffmpeg-decode"))]
fn decode_media_to_pcm_i32(_input: &Path) -> Result<DecodedPcm> {
    Err(Error::FfmpegLibraryNotFound(
        "ffmpeg-decode feature is disabled".to_string(),
    ))
}

/// 当前构建可用的媒体能力摘要。
#[must_use]
pub fn media_capabilities() -> AudioMediaCapabilities {
    #[cfg(feature = "ffmpeg-decode")]
    {
        return media::media_capabilities();
    }

    #[cfg(not(feature = "ffmpeg-decode"))]
    {
        AudioMediaCapabilities {
            backend: "ffmpeg",
            eac3_decode: false,
            container_mp4: false,
            container_mkv: false,
            container_ts: false,
        }
    }
}

/// 直接从已解码的 PCM 数据构建 `MultichannelAudio`，跳过"写临时 WAV → 再读回"的冗余 I/O。
///
/// `decode_to_wav` 路径：`DecodedPcm`（内存） → 磁盘 → `from_wav`（内存）
/// 本函数路径：`DecodedPcm`（内存） → `MultichannelAudio`（内存），无磁盘接触。
#[cfg(feature = "multichannel")]
fn decoded_pcm_into_multichannel(decoded: DecodedPcm) -> Result<MultichannelAudio> {
    use crate::multichannel::SampleFormat;

    let num_channels = decoded.channels as usize;
    if num_channels == 0 {
        return Err(Error::InvalidInput(
            "decoded PCM has no channels".to_string(),
        ));
    }
    let sample_format = match decoded.bits_per_sample {
        16 => SampleFormat::Int16,
        24 => SampleFormat::Int24,
        32 => SampleFormat::Int32,
        b => {
            return Err(Error::InvalidInput(format!(
                "unsupported decoded bit depth: {b}"
            )))
        }
    };
    let total = decoded.samples.len();
    if total % num_channels != 0 {
        return Err(Error::InvalidInput(format!(
            "decoded sample count {total} is not divisible by channel count {num_channels}"
        )));
    }
    let num_samples = total / num_channels;
    let mut channels = vec![Vec::with_capacity(num_samples); num_channels];
    for (i, sample) in decoded.samples.into_iter().enumerate() {
        let clamped = clamp_sample_to_bits(sample, decoded.bits_per_sample);
        channels[i % num_channels].push(clamped);
    }
    MultichannelAudio::new(channels, decoded.sample_rate, sample_format)
}

fn clamp_sample_to_bits(sample: i32, bits_per_sample: u16) -> i32 {
    let bits = bits_per_sample.clamp(1, 32);
    let min = -(1_i64 << (bits - 1));
    let max = (1_i64 << (bits - 1)) - 1;
    let clamped = i64::from(sample).clamp(min, max);
    i32::try_from(clamped).unwrap_or(if clamped < 0 { i32::MIN } else { i32::MAX })
}

/// 字节数组转 hex 字符串
fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

/// hex 字符串转字节数组
fn hex_to_bytes(hex: &str) -> Option<[u8; MESSAGE_LEN]> {
    if hex.len() != MESSAGE_LEN * 2 {
        return None;
    }

    let mut result = [0u8; MESSAGE_LEN];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).ok()?;
        result[i] = u8::from_str_radix(s, 16).ok()?;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    #[test]
    fn test_bytes_to_hex() {
        let bytes = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ];
        assert_eq!(bytes_to_hex(&bytes), "0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn test_hex_to_bytes() {
        let hex = "0123456789abcdef0123456789abcdef";
        let maybe_bytes = hex_to_bytes(hex);
        assert!(maybe_bytes.is_some());
        if let Some(bytes) = maybe_bytes {
            assert_eq!(bytes[0], 0x01);
            assert_eq!(bytes[15], 0xef);
        }
    }

    #[test]
    fn test_parse_detect_output() {
        let stdout = "pattern  all 0101c1d05978131b57f7deb8e22a0b78\n";
        let parsed = parse_detect_output(stdout, "");
        assert!(parsed.is_some());
        if let Some(result) = parsed {
            assert_eq!(result.pattern, "all");
            assert_eq!(result.detect_score, None);
            assert_eq!(result.raw_message[0], 0x01);
        }
    }

    #[test]
    fn test_parse_detect_with_errors() {
        let stdout = "pattern   single 0101c1d05978131b57f7deb8e22a0b78 3\n";
        let parsed = parse_detect_output(stdout, "");
        assert!(parsed.is_some());
        if let Some(result) = parsed {
            assert_eq!(result.pattern, "single");
            assert_eq!(result.detect_score, None);
            assert_eq!(result.bit_errors, 3);
        }
    }

    #[test]
    fn test_parse_detect_zero_message_as_not_found() {
        let stdout = "pattern  0:00 00000000000000000000000000000000 0.000 -0.001 CLIP-B\n";
        let parsed = parse_detect_output(stdout, "");
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_detect_skip_zero_and_take_next() {
        let stdout = concat!(
            "pattern  0:00 00000000000000000000000000000000 0.000 -0.001 CLIP-B\n",
            "pattern  0:00 0101c1d05978131b57f7deb8e22a0b78 1.792 0.121 CLIP-B\n"
        );
        let parsed = parse_detect_output(stdout, "");
        assert!(parsed.is_some());
        if let Some(result) = parsed {
            assert_eq!(result.raw_message[0], 0x01);
            assert!(result
                .detect_score
                .is_some_and(|value| (value - 1.792).abs() < 0.0001));
            assert_eq!(result.bit_errors, 0);
        }
    }

    #[test]
    fn test_parse_detect_ignore_low_score_candidate() {
        let stdout = "pattern  1:28 bb4aaa05ad77bf5e73c8eb37e44f0c94 0.209 0.379 A\n";
        let parsed = parse_detect_output(stdout, "");
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_detect_accept_high_score_candidate() {
        let stdout = "pattern  0:05 023848c0200045fffff7d8743d035cda 1.427 0.065 A\n";
        let parsed = parse_detect_output(stdout, "");
        assert!(parsed.is_some());
        if let Some(result) = parsed {
            assert_eq!(result.raw_message[0], 0x02);
            assert!(result
                .detect_score
                .is_some_and(|value| (value - 1.427).abs() < 0.0001));
        }
    }

    #[test]
    fn test_validate_input_format_exts() {
        assert!(extension_format_hint(Path::new("demo.wav")).is_some());
        assert!(extension_format_hint(Path::new("demo.flac")).is_some());
        assert!(extension_format_hint(Path::new("demo.mp3")).is_some());
        assert!(extension_format_hint(Path::new("demo.ogg")).is_some());
        assert!(extension_format_hint(Path::new("demo.opus")).is_some());
        assert!(extension_format_hint(Path::new("demo.m4a")).is_some());
        assert!(extension_format_hint(Path::new("demo.alac")).is_some());
        assert!(extension_format_hint(Path::new("demo.mp4")).is_some());
        assert!(extension_format_hint(Path::new("demo.mkv")).is_some());
        assert!(extension_format_hint(Path::new("demo.mka")).is_some());
        assert!(extension_format_hint(Path::new("demo.ts")).is_some());
        assert!(extension_format_hint(Path::new("demo.m2ts")).is_some());
        assert!(extension_format_hint(Path::new("demo.m2t")).is_some());
        assert!(extension_format_hint(Path::new("demo.unknown")).is_none());
    }

    #[test]
    fn test_media_capabilities_snapshot() {
        let caps = media_capabilities();
        assert_eq!(caps.backend, "ffmpeg");
        if cfg!(feature = "ffmpeg-decode") {
            assert!(caps.container_mp4);
            assert!(caps.container_mkv);
        } else {
            assert!(!caps.container_mp4);
            assert!(!caps.container_mkv);
        }
    }

    #[test]
    fn test_output_wav_only() {
        assert!(validate_embed_output_path(Path::new("out.wav")).is_ok());
        assert!(matches!(
            validate_embed_output_path(Path::new("out.flac")),
            Err(Error::InvalidOutputFormat(_))
        ));
        assert!(matches!(
            validate_embed_output_path(Path::new("out.m4a")),
            Err(Error::InvalidOutputFormat(_))
        ));
    }

    #[test]
    fn test_probe_prefers_header_over_extension() {
        let path = unique_temp_file("probe_header_over_ext.mp3");
        let write_result = std::fs::write(
            &path,
            [
                0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45,
            ],
        );
        assert!(write_result.is_ok());
        let strategy = classify_input_prepare_strategy(&path);
        assert_eq!(strategy, InputPrepareStrategy::Direct);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_probe_detects_non_wav_content_with_wav_extension() {
        let path = unique_temp_file("probe_non_wav.wav");
        let write_result = std::fs::write(&path, [0x49, 0x44, 0x33, 0x04, 0x00, 0x00]);
        assert!(write_result.is_ok());
        let strategy = classify_input_prepare_strategy(&path);
        assert_eq!(strategy, InputPrepareStrategy::DecodeToWav);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_parse_env_flag_truthy_values() {
        assert!(parse_env_flag("1"));
        assert!(parse_env_flag("true"));
        assert!(parse_env_flag("YES"));
        assert!(parse_env_flag(" on "));
        assert!(!parse_env_flag("0"));
        assert!(!parse_env_flag("false"));
    }

    #[test]
    fn test_looks_like_wav_stream() {
        assert!(looks_like_wav_stream(b"RIFF\x00\x00\x00\x00WAVE"));
        assert!(looks_like_wav_stream(b"RF64\x00\x00\x00\x00WAVE"));
        assert!(looks_like_wav_stream(b"BW64\x00\x00\x00\x00WAVE"));
        assert!(!looks_like_wav_stream(b"ID3\x04\x00\x00\x00\x00\x00\x00"));
        assert!(!looks_like_wav_stream(b""));
    }

    #[test]
    fn test_validate_wav_output_file_rejects_non_wav() {
        let path = unique_temp_file("validate_non_wav.bin");
        let write_result = std::fs::write(&path, b"not-a-wav-stream");
        assert!(write_result.is_ok());

        let validated = validate_wav_output_file(&path);
        assert!(matches!(validated, Err(Error::AudiowmarkExec(_))));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_normalize_wav_pipe_file_in_place_updates_sizes() {
        let path = unique_temp_file("normalize_wav_pipe.wav");
        let wav_pipe = wav_pipe_fixture_bytes();
        let write_result = std::fs::write(&path, &wav_pipe);
        assert!(write_result.is_ok());

        let normalized = normalize_wav_pipe_file_in_place(&path);
        assert!(normalized.is_ok());

        let bytes = std::fs::read(&path);
        assert!(bytes.is_ok());
        let bytes = bytes.unwrap_or_default();
        assert!(bytes.len() >= 48);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            40
        );
        assert_eq!(
            u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
            4
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_effective_awmiomode_respects_env() {
        with_disable_pipe_env(None, || {
            assert_eq!(effective_awmiomode(), AwmIoMode::Pipe);
        });
        with_disable_pipe_env(Some("1"), || {
            assert_eq!(effective_awmiomode(), AwmIoMode::File);
        });
        with_disable_pipe_env(Some("true"), || {
            assert_eq!(effective_awmiomode(), AwmIoMode::File);
        });
        with_disable_pipe_env(Some("0"), || {
            assert_eq!(effective_awmiomode(), AwmIoMode::Pipe);
        });
    }

    #[test]
    fn test_pipe_compatibility_error_detection() {
        assert!(is_pipe_compatibility_error(
            "unsupported option '--stdin' for command 'get'"
        ));
        assert!(is_pipe_compatibility_error("cannot open '-'"));
        assert!(is_pipe_compatibility_error("failed to read from stdin"));
        assert!(!is_pipe_compatibility_error("no watermark found"));
    }

    #[test]
    fn test_should_fallback_pipe_error_kinds() {
        assert!(should_fallback_pipe_error(&Error::AudiowmarkExec(
            "pipe not supported".to_string(),
        )));
        assert!(should_fallback_pipe_error(&Error::Io(
            std::io::Error::other("broken pipe",)
        )));
        assert!(!should_fallback_pipe_error(&Error::InvalidInput(
            "bad input".to_string(),
        )));
    }

    #[cfg(feature = "multichannel")]
    #[test]
    fn test_apply_processed_route_step_mono_only_updates_target_channel() {
        let source = MultichannelAudio::new(
            vec![
                vec![1, 2, 3],
                vec![10, 20, 30],
                vec![100, 200, 300],
                vec![1000, 2000, 3000],
            ],
            48_000,
            crate::multichannel::SampleFormat::Int24,
        );
        assert!(source.is_ok());
        let Ok(mut source) = source else {
            return;
        };

        let processed = MultichannelAudio::new(
            vec![vec![7, 8, 9], vec![9, 9, 9]],
            48_000,
            crate::multichannel::SampleFormat::Int24,
        );
        assert!(processed.is_ok());
        let Ok(processed) = processed else {
            return;
        };

        let step = RouteStep {
            name: "FC(mono)".to_string(),
            mode: RouteMode::Mono(2),
        };
        let applied = apply_processed_route_step(&mut source, &step, &processed);
        assert!(applied.is_ok());

        let ch0 = source.channel_samples(0);
        let ch1 = source.channel_samples(1);
        let ch2 = source.channel_samples(2);
        let ch3 = source.channel_samples(3);
        assert!(ch0.is_ok() && ch1.is_ok() && ch2.is_ok() && ch3.is_ok());
        assert_eq!(ch0.unwrap_or(&[]), &[1, 2, 3]);
        assert_eq!(ch1.unwrap_or(&[]), &[10, 20, 30]);
        assert_eq!(ch2.unwrap_or(&[]), &[7, 8, 9]);
        assert_eq!(ch3.unwrap_or(&[]), &[1000, 2000, 3000]);
    }

    #[cfg(feature = "multichannel")]
    #[test]
    fn test_skip_route_keeps_lfe_unmodified() {
        let source = MultichannelAudio::new(
            vec![
                vec![1, 2, 3],
                vec![10, 20, 30],
                vec![100, 200, 300],
                vec![1000, 2000, 3000],
                vec![11, 22, 33],
                vec![44, 55, 66],
            ],
            48_000,
            crate::multichannel::SampleFormat::Int24,
        );
        assert!(source.is_ok());
        let Ok(mut source) = source else {
            return;
        };

        let step = RouteStep {
            name: "LFE(skip)".to_string(),
            mode: RouteMode::Skip {
                channel: 3,
                reason: "lfe_skipped",
            },
        };
        let processed = MultichannelAudio::new(
            vec![vec![7, 7, 7], vec![8, 8, 8]],
            48_000,
            crate::multichannel::SampleFormat::Int24,
        );
        assert!(processed.is_ok());
        let Ok(processed) = processed else {
            return;
        };

        let before_lfe = source.channel_samples(3).map(|s| s.to_vec());
        assert!(before_lfe.is_ok());
        let apply = apply_processed_route_step(&mut source, &step, &processed);
        assert!(apply.is_ok());
        let after_lfe = source.channel_samples(3).map(|s| s.to_vec());
        assert!(after_lfe.is_ok());
        assert_eq!(
            before_lfe.unwrap_or_default(),
            after_lfe.unwrap_or_default()
        );
    }

    #[cfg(feature = "multichannel")]
    #[test]
    fn test_route_detectable_steps_skip_lfe() {
        let plan = build_smart_route_plan(ChannelLayout::Surround51, 6, DEFAULT_LFE_MODE);
        let detectable = plan.detectable_steps();
        assert_eq!(detectable.len(), 3);
        assert_eq!(detectable[0].1.name, "FL+FR");
        assert_eq!(detectable[1].1.name, "FC(mono)");
        assert_eq!(detectable[2].1.name, "BL+BR");
    }

    #[cfg(feature = "multichannel")]
    #[test]
    fn test_compute_route_parallelism_bounds() {
        assert_eq!(compute_route_parallelism(0), 1);
        assert_eq!(compute_route_parallelism(1), 1);
        // Default is single-threaded; thread-pool overhead outweighs benefit for small step counts.
        assert_eq!(compute_route_parallelism(8), 1);
        assert_eq!(compute_route_parallelism(100), 1);
    }

    #[cfg(feature = "multichannel")]
    #[test]
    fn test_apply_embed_step_results_sorted_and_non_blocking() {
        let source = MultichannelAudio::new(
            vec![vec![1, 2], vec![10, 20], vec![100, 200], vec![1000, 2000]],
            48_000,
            crate::multichannel::SampleFormat::Int24,
        );
        assert!(source.is_ok());
        let Ok(mut source) = source else {
            return;
        };

        let mono_processed = MultichannelAudio::new(
            vec![vec![7, 8], vec![9, 9]],
            48_000,
            crate::multichannel::SampleFormat::Int24,
        );
        assert!(mono_processed.is_ok());
        let Ok(mono_processed) = mono_processed else {
            return;
        };

        let mut step_results = vec![
            EmbedStepTaskResult {
                step_idx: 1,
                step: RouteStep {
                    name: "FC(mono)".to_string(),
                    mode: RouteMode::Mono(2),
                },
                outcome: Ok(mono_processed),
            },
            EmbedStepTaskResult {
                step_idx: 0,
                step: RouteStep {
                    name: "FL+FR".to_string(),
                    mode: RouteMode::Pair(0, 1),
                },
                outcome: Err(Error::AudiowmarkExec("mock embed failure".to_string())),
            },
        ];

        apply_embed_step_results(&mut source, &mut step_results);
        assert_eq!(step_results[0].step_idx, 0);
        assert_eq!(step_results[1].step_idx, 1);

        let ch0 = source.channel_samples(0);
        let ch1 = source.channel_samples(1);
        let ch2 = source.channel_samples(2);
        let ch3 = source.channel_samples(3);
        assert!(ch0.is_ok() && ch1.is_ok() && ch2.is_ok() && ch3.is_ok());
        assert_eq!(ch0.unwrap_or(&[]), &[1, 2]);
        assert_eq!(ch1.unwrap_or(&[]), &[10, 20]);
        assert_eq!(ch2.unwrap_or(&[]), &[7, 8]);
        assert_eq!(ch3.unwrap_or(&[]), &[1000, 2000]);
    }

    #[cfg(feature = "multichannel")]
    #[test]
    fn test_finalize_detect_step_results_sorted_and_best() {
        let step_results = vec![
            DetectStepTaskResult {
                step_idx: 2,
                step: RouteStep {
                    name: "BL+BR".to_string(),
                    mode: RouteMode::Pair(4, 5),
                },
                outcome: Some(DetectResult {
                    raw_message: [2; MESSAGE_LEN],
                    pattern: "single".to_string(),
                    detect_score: Some(1.2),
                    bit_errors: 5,
                    match_found: true,
                }),
            },
            DetectStepTaskResult {
                step_idx: 0,
                step: RouteStep {
                    name: "FL+FR".to_string(),
                    mode: RouteMode::Pair(0, 1),
                },
                outcome: Some(DetectResult {
                    raw_message: [1; MESSAGE_LEN],
                    pattern: "single".to_string(),
                    detect_score: Some(1.8),
                    bit_errors: 1,
                    match_found: true,
                }),
            },
        ];

        let finalized = finalize_detect_step_results(step_results);
        assert_eq!(finalized.pairs.len(), 2);
        assert_eq!(finalized.pairs[0].1, "FL+FR");
        assert_eq!(finalized.pairs[1].1, "BL+BR");
        assert!(finalized.best.is_some());
        assert_eq!(finalized.best.map(|value| value.bit_errors), Some(1));
    }

    fn unique_temp_file(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "awmkit_audio_test_{}_{}_{}",
            std::process::id(),
            nanos,
            name
        ))
    }

    fn wav_pipe_fixture_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&[0xFF_u8; 4]);
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&192_000_u32.to_le_bytes());
        bytes.extend_from_slice(&4_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&[0xFF_u8; 4]);
        bytes.extend_from_slice(&[1_u8, 2_u8, 3_u8, 4_u8]);
        bytes
    }

    fn with_disable_pipe_env(value: Option<&str>, f: impl FnOnce()) {
        static ENV_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = match lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let prev = std::env::var("AWMKIT_DISABLE_PIPE_IO").ok();
        match value {
            Some(v) => {
                std::env::set_var("AWMKIT_DISABLE_PIPE_IO", v);
            }
            None => {
                std::env::remove_var("AWMKIT_DISABLE_PIPE_IO");
            }
        }

        f();

        match prev {
            Some(v) => std::env::set_var("AWMKIT_DISABLE_PIPE_IO", v),
            None => std::env::remove_var("AWMKIT_DISABLE_PIPE_IO"),
        }
    }
}
