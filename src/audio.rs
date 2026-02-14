//! 音频水印嵌入/检测
//!
//! 封装 audiowmark 命令行工具

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs::File, io::Read};

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

/// audiowmark 默认搜索路径（无官方包，仅供开发者本地编译后使用）
#[cfg(not(feature = "bundled"))]
const DEFAULT_SEARCH_PATHS: &[&str] = &["audiowmark"];
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

        let mut cmd = self.audiowmark_command();
        cmd.arg("add")
            .arg("--strength")
            .arg(self.strength.to_string());

        if let Some(ref key_file) = self.key_file {
            cmd.arg("--key").arg(key_file);
        }

        cmd.arg(&prepared.path).arg(output.as_ref()).arg(&hex);

        let output = cmd
            .output()
            .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::AudiowmarkExec(stderr.to_string()));
        }

        Ok(())
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
        let mut cmd = self.audiowmark_command();
        cmd.arg("get");

        if let Some(ref key_file) = self.key_file {
            cmd.arg("--key").arg(key_file);
        }

        cmd.arg(&prepared.path);

        let output = cmd
            .output()
            .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

        // audiowmark 在没有检测到水印时可能返回非零状态
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 解析输出
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

        let prepared = prepare_input_for_audiowmark(input, "embed_multichannel_input")?;
        let input = prepared.path.as_path();

        // 加载多声道音频以检测声道数。
        // 若文件无法被 Rust 解码器解析（如含 ID3 标签的 FLAC），回退到立体声路径，
        // 由 audiowmark 的 libsndfile 原生处理该格式。
        let mut audio = match MultichannelAudio::from_file(input) {
            Ok(a) => a,
            Err(Error::InvalidInput(_)) => {
                return self.embed(input, output, message);
            }
            Err(e) => return Err(e),
        };
        let num_channels = audio.num_channels();

        // 如果是立体声，直接使用普通方法
        if num_channels == 2 {
            return self.embed(input, output, message);
        }

        // 确定声道布局
        let layout = layout.unwrap_or_else(|| audio.layout());
        validate_layout_channels(layout, num_channels)?;
        let route_plan = build_smart_route_plan(layout, num_channels, DEFAULT_LFE_MODE);
        log_route_warnings("embed", input, &route_plan.warnings);

        let temp_dir = create_temp_dir("awmkit_embed_smart")?;
        let _guard = TempDirGuard {
            path: temp_dir.clone(),
        };

        for (step_idx, step) in route_plan.steps.iter().enumerate() {
            if matches!(step.mode, RouteMode::Skip { .. }) {
                continue;
            }

            let temp_input = temp_dir.join(format!("route_{step_idx}_in.wav"));
            let temp_output = temp_dir.join(format!("route_{step_idx}_out.wav"));
            let stereo = build_stereo_for_route_step(&audio, step)?;
            stereo.to_wav(&temp_input)?;

            match self.embed(&temp_input, &temp_output, message) {
                Ok(()) => {
                    let processed = MultichannelAudio::from_wav(&temp_output)?;
                    if let Err(err) = apply_processed_route_step(&mut audio, step, &processed) {
                        eprintln!(
                            "Warning: Failed to apply routed embed result for {}: {err}",
                            step.name
                        );
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Warning: Failed to embed in route step {}: {err}",
                        step.name
                    );
                }
            }
        }

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
        if media::adm_bwav::probe_adm_bwf(input.as_ref())?.is_some() {
            return Err(Error::AdmUnsupported(
                "ADM/BWF detect is not supported in this phase".to_string(),
            ));
        }

        let prepared = prepare_input_for_audiowmark(input.as_ref(), "detect_multichannel_input")?;
        let input = prepared.path.as_path();

        // 加载多声道音频以检测声道数。
        // 若文件无法被 Rust 解码器解析（如含 ID3 标签的 FLAC），回退到立体声路径，
        // 由 audiowmark 的 libsndfile 原生处理该格式。
        let audio = match MultichannelAudio::from_file(input) {
            Ok(a) => a,
            Err(Error::InvalidInput(_)) => {
                let result = self.detect(input)?;
                return Ok(MultichannelDetectResult {
                    pairs: vec![(0, "FL+FR".to_string(), result.clone())],
                    best: result,
                });
            }
            Err(e) => return Err(e),
        };
        let num_channels = audio.num_channels();

        // 如果是立体声，直接使用普通方法
        if num_channels == 2 {
            let result = self.detect(input)?;
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

        let temp_dir = create_temp_dir("awmkit_detect_smart")?;
        let _guard = TempDirGuard {
            path: temp_dir.clone(),
        };

        let mut pairs_results = Vec::new();
        let mut best: Option<DetectResult> = None;

        for (result_idx, (step_idx, step)) in route_plan.detectable_steps().into_iter().enumerate()
        {
            let temp_file = temp_dir.join(format!("route_{step_idx}_detect.wav"));
            let stereo = build_stereo_for_route_step(&audio, step)?;
            stereo.to_wav(&temp_file)?;

            // 检测水印
            let result = self.detect(&temp_file)?;

            // 更新最佳结果 (选择比特错误最少的)
            if let Some(ref r) = result {
                if best.is_none() || r.bit_errors < best.as_ref().map_or(u32::MAX, |b| b.bit_errors)
                {
                    best = Some(r.clone());
                }
            }

            pairs_results.push((result_idx, step.name.clone(), result));
        }

        Ok(MultichannelDetectResult {
            pairs: pairs_results,
            best,
        })
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
}
