//! 多声道音频处理.
//!
//! 支持将多声道音频拆分为立体声对，便于 audiowmark 处理.

use std::path::Path;

use crate::error::{Error, Result};

/// 声道布局.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    /// 立体声 (2ch): FL FR.
    Stereo,
    /// 5.1 环绕 (6ch): FL FR FC LFE BL BR.
    Surround51,
    /// 5.1.2 (8ch): FL FR FC LFE BL BR TFL TFR.
    Surround512,
    /// 7.1 环绕 (8ch): FL FR FC LFE BL BR SL SR.
    Surround71,
    /// 7.1.2 Atmos Bed (10ch): FL FR FC LFE BL BR SL SR Lts Rts.
    Surround712,
    /// 7.1.4 Atmos (12ch): FL FR FC LFE BL BR SL SR TFL TFR TBL TBR.
    Surround714,
    /// 9.1.6 Atmos (16ch): FL FR FC LFE BL BR SL SR FLC FRC TFL TFR TBL TBR TSL TSR.
    Surround916,
    /// 自定义声道数 (必须为偶数).
    Custom(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Internal enum.
pub(crate) enum LfeMode {
    /// Internal variant.
    Skip,
    /// Internal variant.
    Mono,
    /// Internal variant.
    Pair,
}

/// Internal constant.
pub(crate) const DEFAULT_LFE_MODE: LfeMode = LfeMode::Skip;

/// 运行时 LFE 路由模式（默认 `skip`）。.
///
/// 可通过环境变量 `AWMKIT_LFE_MODE` 覆盖：.
/// - `skip`（默认）
/// - `mono`
/// - `pair`
#[must_use]
pub(crate) fn effective_lfe_mode() -> LfeMode {
    let Ok(raw) = std::env::var("AWMKIT_LFE_MODE") else {
        return DEFAULT_LFE_MODE;
    };
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "mono" => LfeMode::Mono,
        "pair" => LfeMode::Pair,
        _ => DEFAULT_LFE_MODE,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Internal enum.
pub(crate) enum RouteMode {
    /// Internal variant.
    Pair(usize, usize),
    /// Internal variant.
    Mono(usize),
    /// Internal variant.
    Skip {
        /// Internal field.
        channel: usize,
        /// Internal field.
        reason: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Internal struct.
pub(crate) struct RouteStep {
    /// Internal field.
    pub name: String,
    /// Internal field.
    pub mode: RouteMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Internal struct.
pub(crate) struct RoutePlan {
    /// Internal field.
    pub layout: ChannelLayout,
    /// Internal field.
    pub channels: usize,
    /// Internal field.
    pub steps: Vec<RouteStep>,
    /// Internal field.
    pub warnings: Vec<String>,
}

impl RoutePlan {
    #[must_use]
    /// Internal helper method.
    pub fn detectable_steps(&self) -> Vec<(usize, &RouteStep)> {
        self.steps
            .iter()
            .enumerate()
            .filter(|(_, step)| !matches!(step.mode, RouteMode::Skip { .. }))
            .collect()
    }
}

#[must_use]
/// Internal helper function.
pub(crate) fn build_smart_route_plan(
    layout: ChannelLayout,
    channels: usize,
    lfe_mode: LfeMode,
) -> RoutePlan {
    match layout {
        ChannelLayout::Stereo if channels == 2 => RoutePlan {
            layout,
            channels,
            steps: vec![pair_step(0, 1, "FL+FR")],
            warnings: Vec::new(),
        },
        ChannelLayout::Surround51 if channels == 6 => RoutePlan {
            layout,
            channels,
            steps: known_surround_steps(layout, lfe_mode),
            warnings: Vec::new(),
        },
        ChannelLayout::Surround512 | ChannelLayout::Surround71 if channels == 8 => RoutePlan {
            layout,
            channels,
            steps: known_surround_steps(layout, lfe_mode),
            warnings: Vec::new(),
        },
        ChannelLayout::Surround712 if channels == 10 => RoutePlan {
            layout,
            channels,
            steps: known_surround_steps(layout, lfe_mode),
            warnings: Vec::new(),
        },
        ChannelLayout::Surround714 if channels == 12 => RoutePlan {
            layout,
            channels,
            steps: known_surround_steps(layout, lfe_mode),
            warnings: Vec::new(),
        },
        ChannelLayout::Surround916 if channels == 16 => RoutePlan {
            layout,
            channels,
            steps: known_surround_steps(layout, lfe_mode),
            warnings: Vec::new(),
        },
        _ => fallback_route_plan(layout, channels),
    }
}

/// Internal helper function.
fn known_surround_steps(layout: ChannelLayout, lfe_mode: LfeMode) -> Vec<RouteStep> {
    let mut steps = Vec::new();
    steps.push(pair_step(0, 1, "FL+FR"));

    match lfe_mode {
        LfeMode::Skip => {
            steps.push(mono_step(2, "FC(mono)"));
            steps.push(skip_step(3, "LFE(skip)", "lfe_skipped"));
        }
        LfeMode::Mono => {
            steps.push(mono_step(2, "FC(mono)"));
            steps.push(mono_step(3, "LFE(mono)"));
        }
        LfeMode::Pair => {
            steps.push(pair_step(2, 3, "FC+LFE"));
        }
    }

    match layout {
        ChannelLayout::Surround51 => {
            steps.push(pair_step(4, 5, "BL+BR"));
        }
        ChannelLayout::Surround512 => {
            steps.push(pair_step(4, 5, "BL+BR"));
            steps.push(pair_step(6, 7, "TFL+TFR"));
        }
        ChannelLayout::Surround71 => {
            steps.push(pair_step(4, 5, "BL+BR"));
            steps.push(pair_step(6, 7, "SL+SR"));
        }
        ChannelLayout::Surround712 => {
            // 7.1.2: FL FR FC LFE BL BR SL SR Lts Rts
            // 前 3 步（FL+FR, FC(mono), LFE）由 known_surround_steps 顶部公共代码添加
            steps.push(pair_step(4, 5, "BL+BR"));
            steps.push(pair_step(6, 7, "SL+SR"));
            steps.push(pair_step(8, 9, "Lts+Rts"));
        }
        ChannelLayout::Surround714 => {
            steps.push(pair_step(4, 5, "BL+BR"));
            steps.push(pair_step(6, 7, "SL+SR"));
            steps.push(pair_step(8, 9, "TFL+TFR"));
            steps.push(pair_step(10, 11, "TBL+TBR"));
        }
        ChannelLayout::Surround916 => {
            steps.push(pair_step(4, 5, "BL+BR"));
            steps.push(pair_step(6, 7, "SL+SR"));
            steps.push(pair_step(8, 9, "FLC+FRC"));
            steps.push(pair_step(10, 11, "TFL+TFR"));
            steps.push(pair_step(12, 13, "TBL+TBR"));
            steps.push(pair_step(14, 15, "TSL+TSR"));
        }
        _ => {}
    }

    steps
}

/// Internal helper function.
fn fallback_route_plan(layout: ChannelLayout, channels: usize) -> RoutePlan {
    let mut steps = Vec::new();
    let mut ch = 0usize;
    while ch < channels {
        if ch + 1 < channels {
            steps.push(pair_step(ch, ch + 1, &format!("CH{}+CH{}", ch + 1, ch + 2)));
            ch += 2;
        } else {
            steps.push(mono_step(ch, &format!("CH{}(mono)", ch + 1)));
            ch += 1;
        }
    }

    RoutePlan {
        layout,
        channels,
        steps,
        warnings: vec![format!(
            "smart routing fallback is used for layout {:?} ({} channels)",
            layout, channels
        )],
    }
}

/// Internal helper function.
fn pair_step(ch_a: usize, ch_b: usize, name: &str) -> RouteStep {
    RouteStep {
        name: name.to_string(),
        mode: RouteMode::Pair(ch_a, ch_b),
    }
}

/// Internal helper function.
fn mono_step(channel: usize, name: &str) -> RouteStep {
    RouteStep {
        name: name.to_string(),
        mode: RouteMode::Mono(channel),
    }
}

/// Internal helper function.
fn skip_step(channel: usize, name: &str, reason: &'static str) -> RouteStep {
    RouteStep {
        name: name.to_string(),
        mode: RouteMode::Skip { channel, reason },
    }
}

impl ChannelLayout {
    /// 获取声道数.
    #[must_use]
    pub const fn channels(&self) -> u16 {
        match self {
            Self::Stereo => 2,
            Self::Surround51 => 6,
            Self::Surround512 | Self::Surround71 => 8,
            Self::Surround712 => 10,
            Self::Surround714 => 12,
            Self::Surround916 => 16,
            Self::Custom(n) => *n,
        }
    }

    /// 获取立体声对数量.
    #[must_use]
    pub const fn stereo_pairs(&self) -> u16 {
        self.channels() / 2
    }

    /// 从声道数推断布局 (默认选择).
    #[must_use]
    pub const fn from_channels(channels: u16) -> Self {
        match channels {
            2 => Self::Stereo,
            6 => Self::Surround51,
            8 => Self::Surround71, // 默认 7.1，可手动指定 5.1.2
            10 => Self::Surround712,
            12 => Self::Surround714,
            16 => Self::Surround916,
            n => Self::Custom(n),
        }
    }

    /// 从声道数推断布局，无已知匹配时返回 `None`（不产生 `Custom` 兜底）。.
    #[must_use]
    pub const fn from_channels_opt(channels: usize) -> Option<Self> {
        match channels {
            2 => Some(Self::Stereo),
            6 => Some(Self::Surround51),
            8 => Some(Self::Surround71),
            10 => Some(Self::Surround712),
            12 => Some(Self::Surround714),
            16 => Some(Self::Surround916),
            _ => None,
        }
    }

    /// 获取各立体声对的名称.
    #[must_use]
    pub fn pair_names(&self) -> Vec<&'static str> {
        match self {
            Self::Stereo => vec!["FL+FR"],
            Self::Surround51 => vec!["FL+FR", "FC+LFE", "BL+BR"],
            Self::Surround512 => vec!["FL+FR", "FC+LFE", "BL+BR", "TFL+TFR"],
            Self::Surround71 => vec!["FL+FR", "FC+LFE", "BL+BR", "SL+SR"],
            Self::Surround712 => vec!["FL+FR", "FC+LFE", "BL+BR", "SL+SR", "Lts+Rts"],
            Self::Surround714 => {
                vec!["FL+FR", "FC+LFE", "BL+BR", "SL+SR", "TFL+TFR", "TBL+TBR"]
            }
            Self::Surround916 => vec![
                "FL+FR", "FC+LFE", "BL+BR", "SL+SR", "FLC+FRC", "TFL+TFR", "TBL+TBR", "TSL+TSR",
            ],
            Self::Custom(n) => {
                let pairs = n / 2;
                (0..pairs)
                    .map(|i| match i {
                        0 => "Pair 1",
                        1 => "Pair 2",
                        2 => "Pair 3",
                        3 => "Pair 4",
                        4 => "Pair 5",
                        5 => "Pair 6",
                        6 => "Pair 7",
                        7 => "Pair 8",
                        _ => "Pair N",
                    })
                    .collect()
            }
        }
    }
}

/// 样本格式.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 16-bit 整数.
    Int16,
    /// 24-bit 整数.
    Int24,
    /// 32-bit 整数.
    Int32,
    /// 32-bit 浮点.
    Float32,
}

impl SampleFormat {
    /// 每样本的位数.
    #[must_use]
    pub const fn bits_per_sample(&self) -> u16 {
        match self {
            Self::Int16 => 16,
            Self::Int24 => 24,
            Self::Int32 | Self::Float32 => 32,
        }
    }
}

/// 多声道音频数据.
#[derive(Debug, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct AudioBuffer {
    /// 每个声道的样本数据 [channel][sample]
    /// 统一存储为 i32，便于处理.
    channels: Vec<Vec<i32>>,
    /// 采样率.
    sample_rate: u32,
    /// 原始样本格式.
    sample_format: SampleFormat,
}

impl AudioBuffer {
    /// 创建新的多声道音频.
    ///
    /// # Errors
    /// 当声道集合为空，或各声道样本长度不一致时返回错误。.
    pub fn new(
        channels: Vec<Vec<i32>>,
        sample_rate: u32,
        sample_format: SampleFormat,
    ) -> Result<Self> {
        if channels.is_empty() {
            return Err(Error::InvalidInput("no channels".into()));
        }

        // 验证所有声道长度一致
        let len = channels[0].len();
        for (i, ch) in channels.iter().enumerate() {
            if ch.len() != len {
                return Err(Error::InvalidInput(format!(
                    "channel {i} length mismatch: expected {len}, got {}",
                    ch.len()
                )));
            }
        }

        Ok(Self {
            channels,
            sample_rate,
            sample_format,
        })
    }

    /// 获取声道数.
    #[must_use]
    pub const fn num_channels(&self) -> usize {
        self.channels.len()
    }

    /// 获取每声道样本数.
    #[must_use]
    pub fn num_samples(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    /// 获取采样率.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 获取样本格式.
    #[must_use]
    pub const fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    /// 获取推断的声道布局.
    #[must_use]
    pub fn layout(&self) -> ChannelLayout {
        let channels = u16::try_from(self.num_channels()).unwrap_or(u16::MAX);
        ChannelLayout::from_channels(channels)
    }

    /// 从 WAV 文件加载.
    ///
    /// # Errors
    /// 当文件无法读取、WAV 头无效、样本格式不支持或样本解析失败时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn from_wav<P: AsRef<Path>>(path: P) -> Result<Self> {
        use hound::WavReader;

        let reader = WavReader::open(path.as_ref())
            .map_err(|e| Error::InvalidInput(format!("failed to open WAV: {e}")))?;

        let spec = reader.spec();
        let num_channels = spec.channels as usize;
        let sample_rate = spec.sample_rate;

        let sample_format = match (spec.sample_format, spec.bits_per_sample) {
            (hound::SampleFormat::Int, 16) => SampleFormat::Int16,
            (hound::SampleFormat::Int, 24) => SampleFormat::Int24,
            (hound::SampleFormat::Int, 32) => SampleFormat::Int32,
            (hound::SampleFormat::Float, 32) => SampleFormat::Float32,
            _ => {
                return Err(Error::InvalidInput(format!(
                    "unsupported sample format: {:?} {}bit",
                    spec.sample_format, spec.bits_per_sample
                )))
            }
        };

        // 读取所有样本
        let all_samples: Vec<i32> = match sample_format {
            SampleFormat::Float32 => reader
                .into_samples::<f32>()
                .map(|s| {
                    s.map(scale_float_to_i32)
                        .map_err(|e| Error::InvalidInput(format!("read error: {e}")))
                })
                .collect::<Result<Vec<_>>>()?,
            _ => reader
                .into_samples::<i32>()
                .map(|s| s.map_err(|e| Error::InvalidInput(format!("read error: {e}"))))
                .collect::<Result<Vec<_>>>()?,
        };

        // 反交错为独立声道
        let num_samples = all_samples.len() / num_channels;
        let mut channels = vec![Vec::with_capacity(num_samples); num_channels];

        for (i, sample) in all_samples.into_iter().enumerate() {
            let ch = i % num_channels;
            channels[ch].push(sample);
        }

        Self::new(channels, sample_rate, sample_format)
    }

    /// 从 FLAC 文件加载.
    ///
    /// # Errors
    /// 当文件无法读取、FLAC 位深不支持或解码失败时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn from_flac<P: AsRef<Path>>(path: P) -> Result<Self> {
        use claxon::FlacReader;

        let mut reader = FlacReader::open(path.as_ref())
            .map_err(|e| Error::InvalidInput(format!("failed to open FLAC: {e}")))?;

        let info = reader.streaminfo();
        let num_channels = info.channels as usize;
        let sample_rate = info.sample_rate;
        let bits_per_sample = info.bits_per_sample;

        let sample_format = match bits_per_sample {
            16 => SampleFormat::Int16,
            24 => SampleFormat::Int24,
            32 => SampleFormat::Int32,
            _ => {
                return Err(Error::InvalidInput(format!(
                    "unsupported FLAC bit depth: {bits_per_sample}"
                )))
            }
        };

        // 读取所有样本
        let mut channels = vec![Vec::new(); num_channels];

        // claxon 返回 Block，每个 Block 包含多帧多声道数据
        let mut block_reader = reader.blocks();
        let mut buffer = Vec::new();

        while let Some(block) = block_reader
            .read_next_or_eof(buffer)
            .map_err(|e| Error::InvalidInput(format!("FLAC decode error: {e}")))?
        {
            for (ch, channel) in channels.iter_mut().enumerate().take(num_channels) {
                let ch_idx = u32::try_from(ch).map_err(|_| {
                    Error::InvalidInput("channel index overflow while decoding FLAC".to_string())
                })?;
                let ch_samples = block.channel(ch_idx);
                channel.extend(ch_samples.iter().copied());
            }

            buffer = block.into_buffer();
        }

        Self::new(channels, sample_rate, sample_format)
    }

    /// 从文件加载 (自动检测格式).
    ///
    /// # Errors
    /// 当扩展名不支持，或底层 WAV/FLAC 读取失败时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);

        match ext.as_deref() {
            Some("wav") => Self::from_wav(path),
            Some("flac") => Self::from_flac(path),
            _ => Err(Error::InvalidInput(format!(
                "unsupported file format: {}",
                path.display()
            ))),
        }
    }

    /// 序列化为 WAV 字节（不写文件，供内存管道使用）.
    ///
    /// # Errors
    /// 当 WAV 头字段溢出、样本超出目标位深范围或序列化失败时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn to_wav_bytes(&self) -> Result<Vec<u8>> {
        let channels = self.num_channels();
        let sample_width: usize = match self.sample_format {
            SampleFormat::Int16 => 2,
            SampleFormat::Int24 => 3,
            SampleFormat::Int32 | SampleFormat::Float32 => 4,
        };
        let num_samples = self.num_samples();
        let data_size = num_samples * channels * sample_width;

        let mut buf = Vec::with_capacity(44 + data_size);

        // RIFF header
        buf.extend_from_slice(b"RIFF");
        let riff_size = u32::try_from(data_size + 36)
            .map_err(|_| Error::InvalidInput("audio data too large for WAV format".to_string()))?;
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");

        // fmt chunk
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        let format_tag: u16 = if matches!(self.sample_format, SampleFormat::Float32) {
            3
        } else {
            1
        };
        buf.extend_from_slice(&format_tag.to_le_bytes());
        let ch_u16 = u16::try_from(channels).map_err(|_| {
            Error::InvalidInput("channel count overflow for WAV header".to_string())
        })?;
        buf.extend_from_slice(&ch_u16.to_le_bytes());
        buf.extend_from_slice(&self.sample_rate.to_le_bytes());
        let sample_width_u32 = u32::try_from(sample_width)
            .map_err(|_| Error::InvalidInput("sample width overflow for WAV header".to_string()))?;
        let byte_rate = self.sample_rate * u32::from(ch_u16) * sample_width_u32;
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        let sample_width_u16 = u16::try_from(sample_width)
            .map_err(|_| Error::InvalidInput("sample width overflow for WAV header".to_string()))?;
        let block_align = ch_u16 * sample_width_u16;
        buf.extend_from_slice(&block_align.to_le_bytes());
        let bits: u16 = sample_width_u16 * 8;
        buf.extend_from_slice(&bits.to_le_bytes());

        // data chunk
        buf.extend_from_slice(b"data");
        let data_size_u32 = u32::try_from(data_size)
            .map_err(|_| Error::InvalidInput("audio data too large for WAV format".to_string()))?;
        buf.extend_from_slice(&data_size_u32.to_le_bytes());

        // 交错样本
        for i in 0..num_samples {
            for ch in &self.channels {
                match self.sample_format {
                    SampleFormat::Int16 => {
                        let v = i16::try_from(ch[i]).map_err(|_| {
                            Error::InvalidInput(format!("sample out of 16-bit range at index {i}"))
                        })?;
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
                    SampleFormat::Int24 => {
                        let s = ch[i];
                        let le = s.to_le_bytes();
                        buf.push(le[0]);
                        buf.push(le[1]);
                        buf.push(le[2]);
                    }
                    SampleFormat::Int32 => {
                        buf.extend_from_slice(&ch[i].to_le_bytes());
                    }
                    SampleFormat::Float32 => {
                        let clamped = (ch[i] >> 16).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
                        let sample_i16 = i16::try_from(clamped).map_err(|_| {
                            Error::InvalidInput(format!("float32 sample out of range at index {i}"))
                        })?;
                        let f = f32::from(sample_i16) / f32::from(i16::MAX);
                        buf.extend_from_slice(&f.to_bits().to_le_bytes());
                    }
                }
            }
        }

        Ok(buf)
    }

    /// 从 WAV 字节反序列化（不读文件，供内存管道使用）.
    ///
    /// 自动处理 audiowmark `--output-format wav-pipe` 输出的 `RIFF ffffffff`
    /// 流式格式（hound 拒绝此格式，需先修复大小字段）。.
    ///
    /// # Errors
    /// 当字节流不是合法 WAV、样本格式不支持或样本解析失败时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn from_wav_bytes(bytes: &[u8]) -> Result<Self> {
        use hound::WavReader;
        use std::io::Cursor;

        let normalized = normalize_wav_pipe_sizes(bytes);
        let reader = WavReader::new(Cursor::new(normalized.as_ref()))
            .map_err(|e| Error::InvalidInput(format!("failed to parse WAV bytes: {e}")))?;

        let spec = reader.spec();
        let num_channels = spec.channels as usize;
        let sample_rate = spec.sample_rate;

        let sample_format = match (spec.sample_format, spec.bits_per_sample) {
            (hound::SampleFormat::Int, 16) => SampleFormat::Int16,
            (hound::SampleFormat::Int, 24) => SampleFormat::Int24,
            (hound::SampleFormat::Int, 32) => SampleFormat::Int32,
            (hound::SampleFormat::Float, 32) => SampleFormat::Float32,
            _ => {
                return Err(Error::InvalidInput(format!(
                    "unsupported sample format: {:?} {}bit",
                    spec.sample_format, spec.bits_per_sample
                )))
            }
        };

        let all_samples: Vec<i32> = match sample_format {
            SampleFormat::Float32 => reader
                .into_samples::<f32>()
                .map(|s| {
                    s.map(scale_float_to_i32)
                        .map_err(|e| Error::InvalidInput(format!("read error: {e}")))
                })
                .collect::<Result<Vec<_>>>()?,
            _ => reader
                .into_samples::<i32>()
                .map(|s| s.map_err(|e| Error::InvalidInput(format!("read error: {e}"))))
                .collect::<Result<Vec<_>>>()?,
        };

        let num_samples = all_samples.len() / num_channels;
        let mut channels = vec![Vec::with_capacity(num_samples); num_channels];
        for (i, sample) in all_samples.into_iter().enumerate() {
            channels[i % num_channels].push(sample);
        }

        Self::new(channels, sample_rate, sample_format)
    }

    /// 保存为 WAV 文件.
    ///
    /// # Errors
    /// 当输出文件无法创建/写入，或样本超出目标位深范围时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn to_wav<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use hound::{SampleFormat as HoundFormat, WavSpec, WavWriter};

        let (hound_format, bits) = match self.sample_format {
            SampleFormat::Int16 => (HoundFormat::Int, 16),
            SampleFormat::Int24 => (HoundFormat::Int, 24),
            SampleFormat::Int32 => (HoundFormat::Int, 32),
            SampleFormat::Float32 => (HoundFormat::Float, 32),
        };

        let channels = u16::try_from(self.num_channels()).map_err(|_| {
            Error::InvalidInput("channel count overflow for WAV writer".to_string())
        })?;

        let spec = WavSpec {
            channels,
            sample_rate: self.sample_rate,
            bits_per_sample: bits,
            sample_format: hound_format,
        };

        let mut writer = WavWriter::create(path.as_ref(), spec)
            .map_err(|e| Error::InvalidInput(format!("failed to create WAV: {e}")))?;

        let num_samples = self.num_samples();

        // 交错写入
        for i in 0..num_samples {
            for ch in &self.channels {
                match self.sample_format {
                    SampleFormat::Int16 => {
                        let sample_i16 = i16::try_from(ch[i]).map_err(|_| {
                            Error::InvalidInput(format!("sample out of 16-bit range at index {i}"))
                        })?;
                        writer
                            .write_sample(sample_i16)
                            .map_err(|e| Error::InvalidInput(format!("write error: {e}")))?;
                    }
                    SampleFormat::Int24 | SampleFormat::Int32 => {
                        writer
                            .write_sample(ch[i])
                            .map_err(|e| Error::InvalidInput(format!("write error: {e}")))?;
                    }
                    SampleFormat::Float32 => {
                        let sample_i16 = i16::try_from(
                            (ch[i] >> 16).clamp(i32::from(i16::MIN), i32::from(i16::MAX)),
                        )
                        .map_err(|_| {
                            Error::InvalidInput(format!("sample out of float32 range at index {i}"))
                        })?;
                        let f = f32::from(sample_i16) / f32::from(i16::MAX);
                        writer
                            .write_sample(f)
                            .map_err(|e| Error::InvalidInput(format!("write error: {e}")))?;
                    }
                }
            }
        }

        writer
            .finalize()
            .map_err(|e| Error::InvalidInput(format!("finalize error: {e}")))?;

        Ok(())
    }

    /// 获取交错格式的样本数据 (用于 FLAC 编码等).
    ///
    /// 返回 `[L0, R0, L1, R1, ...]` 格式的样本.
    #[must_use]
    pub fn interleaved_samples(&self) -> Vec<i32> {
        let num_samples = self.num_samples();
        let num_channels = self.num_channels();
        let mut result = Vec::with_capacity(num_samples * num_channels);

        for i in 0..num_samples {
            for ch in &self.channels {
                result.push(ch[i]);
            }
        }

        result
    }

    /// 拆分为立体声对.
    ///
    /// 返回 `(left_channel, right_channel)` 的向量.
    #[must_use]
    pub fn split_stereo_pairs(&self) -> Vec<(Vec<i32>, Vec<i32>)> {
        self.channels
            .chunks(2)
            .map(|pair| {
                let left = pair[0].clone();
                let right = if pair.len() > 1 {
                    pair[1].clone()
                } else {
                    // 单声道复制到右声道
                    pair[0].clone()
                };
                (left, right)
            })
            .collect()
    }

    /// Internal helper method.
    pub(crate) fn channel_samples(&self, index: usize) -> Result<&[i32]> {
        self.channels
            .get(index)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::InvalidInput(format!("channel index {index} out of range")))
    }

    /// Internal helper method.
    pub(crate) fn replace_channel_samples(
        &mut self,
        index: usize,
        samples: Vec<i32>,
    ) -> Result<()> {
        let expected = self.num_samples();
        if samples.len() != expected {
            return Err(Error::InvalidInput(format!(
                "channel {index} sample length mismatch: expected {expected}, got {}",
                samples.len()
            )));
        }
        let channel = self
            .channels
            .get_mut(index)
            .ok_or_else(|| Error::InvalidInput(format!("channel index {index} out of range")))?;
        *channel = samples;
        Ok(())
    }

    /// 从立体声对合并.
    ///
    /// # Errors
    /// 当输入声道对为空，或合并后各声道样本长度不一致时返回错误。.
    pub fn merge_stereo_pairs(
        pairs: &[(Vec<i32>, Vec<i32>)],
        sample_rate: u32,
        sample_format: SampleFormat,
    ) -> Result<Self> {
        let mut channels = Vec::with_capacity(pairs.len() * 2);

        for (left, right) in pairs {
            channels.push(left.clone());
            channels.push(right.clone());
        }

        Self::new(channels, sample_rate, sample_format)
    }

    /// 保存立体声对到临时 WAV 文件.
    ///
    /// # Errors
    /// 当 `pair_index` 越界、构造立体声对失败或写文件失败时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn save_stereo_pair<P: AsRef<Path>>(&self, pair_index: usize, path: P) -> Result<()> {
        let pairs = self.split_stereo_pairs();
        if pair_index >= pairs.len() {
            return Err(Error::InvalidInput(format!(
                "pair index {pair_index} out of range (max {})",
                pairs.len() - 1
            )));
        }

        let (left, right) = &pairs[pair_index];
        let stereo = Self::new(
            vec![left.clone(), right.clone()],
            self.sample_rate,
            self.sample_format,
        )?;
        stereo.to_wav(path)
    }

    /// 从立体声 WAV 文件加载并替换指定声道对.
    ///
    /// # Errors
    /// 当 `pair_index` 越界、输入并非立体声 WAV，或读取失败时返回错误。.
    #[cfg(feature = "multichannel")]
    pub fn load_stereo_pair<P: AsRef<Path>>(&mut self, pair_index: usize, path: P) -> Result<()> {
        let stereo = Self::from_wav(path)?;
        if stereo.num_channels() != 2 {
            return Err(Error::InvalidInput("expected stereo WAV".into()));
        }

        let left_idx = pair_index * 2;
        let right_idx = pair_index * 2 + 1;

        if right_idx >= self.channels.len() {
            return Err(Error::InvalidInput(format!(
                "pair index {pair_index} out of range"
            )));
        }

        self.channels[left_idx].clone_from(&stereo.channels[0]);
        self.channels[right_idx].clone_from(&stereo.channels[1]);

        Ok(())
    }
}

/// audiowmark `--output-format wav-pipe` 输出 RIFF/data chunk 大小为 `0xFFFF_FFFF`（流式未知长度）。
/// hound 拒绝此格式；将大小字段修复为实际值后再交给 hound 解析。.
///
/// 若 RIFF size 不为 `0xFFFF_FFFF` 则直接借用原始切片，不做任何复制。.
///
/// 注意：audiowmark 在 pipe 模式下会在奇数长度 data 末尾追加 1 字节 WAV 对齐填充。
/// 必须从 fmt chunk 读取 `block_align` 并将 data size 截断到 `block_align` 的整数倍。.
#[cfg(feature = "multichannel")]
fn normalize_wav_pipe_sizes(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return std::borrow::Cow::Borrowed(bytes);
    }

    // RIFF size が 0xFFFFFFFF でなければ修复不要
    if bytes[4..8] != [0xFF_u8; 4] {
        return std::borrow::Cow::Borrowed(bytes);
    }

    let mut patched = bytes.to_vec();

    // 修复 RIFF chunk 大小（整个文件减去 8 字节头）
    let riff_payload = u32::try_from(patched.len().saturating_sub(8)).unwrap_or(u32::MAX);
    patched[4..8].copy_from_slice(&riff_payload.to_le_bytes());

    // 扫描 sub-chunk：先从 fmt 读取 block_align，再修复 data chunk 大小
    let mut pos = 12usize; // 跳过 RIFF(4) + size(4) + WAVE(4)
    let mut block_align: u32 = 1;
    while pos.saturating_add(8) <= patched.len() {
        let chunk_size = u32::from_le_bytes([
            patched[pos + 4],
            patched[pos + 5],
            patched[pos + 6],
            patched[pos + 7],
        ]);
        if &patched[pos..pos + 4] == b"fmt " {
            // fmt payload 布局：AudioFormat(2)+NumChannels(2)+SampleRate(4)+ByteRate(4)+BlockAlign(2)
            // block_align 位于 chunk body 偏移 12，即 pos+20
            if pos + 22 <= patched.len() {
                let ba = u16::from_le_bytes([patched[pos + 20], patched[pos + 21]]);
                if ba > 0 {
                    block_align = u32::from(ba);
                }
            }
        } else if &patched[pos..pos + 4] == b"data" && chunk_size == u32::MAX {
            let raw = u32::try_from(patched.len().saturating_sub(pos + 8)).unwrap_or(u32::MAX);
            // 截断到 block_align 整数倍，排除 WAV 块尾部的对齐填充字节
            let data_payload = raw - (raw % block_align);
            patched[pos + 4..pos + 8].copy_from_slice(&data_payload.to_le_bytes());
            break;
        } else if &patched[pos..pos + 4] == b"data" {
            break; // data chunk 大小已知，不修改
        }
        // chunk_size 为 0xFFFF_FFFF 说明遇到另一个未知长度 chunk，无法前进
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

    std::borrow::Cow::Owned(patched)
}

/// Internal helper function.
fn scale_float_to_i32(sample: f32) -> i32 {
    use num_traits::ToPrimitive;

    const I32_MIN_F64: f64 = -2_147_483_648.0_f64;
    const I32_MAX_F64: f64 = 2_147_483_647.0_f64;

    if !sample.is_finite() {
        return 0;
    }

    let scaled = f64::from(sample) * 2_147_483_647.0_f64;
    let rounded = scaled.round();
    if rounded <= I32_MIN_F64 {
        i32::MIN
    } else if rounded >= I32_MAX_F64 {
        i32::MAX
    } else {
        rounded.to_i32().unwrap_or_else(|| {
            if rounded.is_sign_negative() {
                i32::MIN
            } else {
                i32::MAX
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_layout() {
        assert_eq!(ChannelLayout::Stereo.channels(), 2);
        assert_eq!(ChannelLayout::Surround51.channels(), 6);
        assert_eq!(ChannelLayout::Surround71.channels(), 8);
        assert_eq!(ChannelLayout::Surround512.channels(), 8);
        assert_eq!(ChannelLayout::Surround714.channels(), 12);
        assert_eq!(ChannelLayout::Surround916.channels(), 16);

        assert_eq!(ChannelLayout::Surround51.stereo_pairs(), 3);
        assert_eq!(ChannelLayout::Surround71.stereo_pairs(), 4);
        assert_eq!(ChannelLayout::Surround714.stereo_pairs(), 6);
        assert_eq!(ChannelLayout::Surround916.stereo_pairs(), 8);
    }

    #[test]
    fn test_split_merge() {
        let channels = vec![
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            vec![9, 10, 11, 12],
            vec![13, 14, 15, 16],
        ];

        let audio_result = AudioBuffer::new(channels, 48000, SampleFormat::Int24);
        assert!(audio_result.is_ok());
        let Ok(audio) = audio_result else {
            return;
        };

        let pairs = audio.split_stereo_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, vec![1, 2, 3, 4]);
        assert_eq!(pairs[0].1, vec![5, 6, 7, 8]);
        assert_eq!(pairs[1].0, vec![9, 10, 11, 12]);
        assert_eq!(pairs[1].1, vec![13, 14, 15, 16]);

        let merged_result = AudioBuffer::merge_stereo_pairs(&pairs, 48000, SampleFormat::Int24);
        assert!(merged_result.is_ok());
        let Ok(merged) = merged_result else {
            return;
        };
        assert_eq!(merged.num_channels(), 4);
        assert_eq!(merged.num_samples(), 4);
    }

    #[test]
    fn test_smart_plan_surround51_default() {
        let plan = build_smart_route_plan(ChannelLayout::Surround51, 6, DEFAULT_LFE_MODE);
        assert!(plan.warnings.is_empty());
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.steps[0].mode, RouteMode::Pair(0, 1));
        assert_eq!(plan.steps[1].mode, RouteMode::Mono(2));
        assert_eq!(
            plan.steps[2].mode,
            RouteMode::Skip {
                channel: 3,
                reason: "lfe_skipped"
            }
        );
        assert_eq!(plan.steps[3].mode, RouteMode::Pair(4, 5));
    }

    #[test]
    fn test_smart_plan_surround714_default() {
        let plan = build_smart_route_plan(ChannelLayout::Surround714, 12, DEFAULT_LFE_MODE);
        assert!(plan.warnings.is_empty());
        assert_eq!(
            plan.steps
                .iter()
                .map(|s| s.mode.clone())
                .collect::<Vec<_>>(),
            vec![
                RouteMode::Pair(0, 1),
                RouteMode::Mono(2),
                RouteMode::Skip {
                    channel: 3,
                    reason: "lfe_skipped"
                },
                RouteMode::Pair(4, 5),
                RouteMode::Pair(6, 7),
                RouteMode::Pair(8, 9),
                RouteMode::Pair(10, 11),
            ]
        );
    }

    #[test]
    fn test_smart_plan_surround916_default() {
        let plan = build_smart_route_plan(ChannelLayout::Surround916, 16, DEFAULT_LFE_MODE);
        assert!(plan.warnings.is_empty());
        assert_eq!(
            plan.steps
                .iter()
                .map(|s| s.mode.clone())
                .collect::<Vec<_>>(),
            vec![
                RouteMode::Pair(0, 1),
                RouteMode::Mono(2),
                RouteMode::Skip {
                    channel: 3,
                    reason: "lfe_skipped"
                },
                RouteMode::Pair(4, 5),
                RouteMode::Pair(6, 7),
                RouteMode::Pair(8, 9),
                RouteMode::Pair(10, 11),
                RouteMode::Pair(12, 13),
                RouteMode::Pair(14, 15),
            ]
        );
    }

    #[test]
    fn test_smart_plan_custom_odd_channels() {
        let plan = build_smart_route_plan(ChannelLayout::Custom(7), 7, DEFAULT_LFE_MODE);
        assert_eq!(
            plan.steps
                .iter()
                .map(|s| s.mode.clone())
                .collect::<Vec<_>>(),
            vec![
                RouteMode::Pair(0, 1),
                RouteMode::Pair(2, 3),
                RouteMode::Pair(4, 5),
                RouteMode::Mono(6),
            ]
        );
        assert_eq!(plan.detectable_steps().len(), 4);
        assert_eq!(plan.warnings.len(), 1);
    }

    #[test]
    fn test_channel_replace_affects_only_target() {
        let audio = AudioBuffer::new(
            vec![
                vec![1, 2, 3],
                vec![10, 20, 30],
                vec![100, 200, 300],
                vec![1000, 2000, 3000],
            ],
            48_000,
            SampleFormat::Int24,
        );
        assert!(audio.is_ok());
        let Ok(mut audio) = audio else {
            return;
        };

        let replace = audio.replace_channel_samples(2, vec![7, 8, 9]);
        assert!(replace.is_ok());
        let ch0 = audio.channel_samples(0);
        let ch1 = audio.channel_samples(1);
        let ch2 = audio.channel_samples(2);
        let ch3 = audio.channel_samples(3);
        assert!(ch0.is_ok() && ch1.is_ok() && ch2.is_ok() && ch3.is_ok());
        assert_eq!(ch0.unwrap_or(&[]), &[1, 2, 3]);
        assert_eq!(ch1.unwrap_or(&[]), &[10, 20, 30]);
        assert_eq!(ch2.unwrap_or(&[]), &[7, 8, 9]);
        assert_eq!(ch3.unwrap_or(&[]), &[1000, 2000, 3000]);
    }
}
