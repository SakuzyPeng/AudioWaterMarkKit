//! 多声道音频处理
//!
//! 支持将多声道音频拆分为立体声对，便于 audiowmark 处理

use std::path::Path;

use crate::error::{Error, Result};

/// 声道布局
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    /// 立体声 (2ch): FL FR
    Stereo,
    /// 5.1 环绕 (6ch): FL FR FC LFE BL BR
    Surround51,
    /// 5.1.2 (8ch): FL FR FC LFE BL BR TFL TFR
    Surround512,
    /// 7.1 环绕 (8ch): FL FR FC LFE BL BR SL SR
    Surround71,
    /// 7.1.4 Atmos (12ch): FL FR FC LFE BL BR SL SR TFL TFR TBL TBR
    Surround714,
    /// 9.1.6 Atmos (16ch): FL FR FC LFE BL BR SL SR FLC FRC TFL TFR TBL TBR TSL TSR
    Surround916,
    /// 自定义声道数 (必须为偶数)
    Custom(u16),
}

impl ChannelLayout {
    /// 获取声道数
    #[must_use]
    pub const fn channels(&self) -> u16 {
        match self {
            Self::Stereo => 2,
            Self::Surround51 => 6,
            Self::Surround512 | Self::Surround71 => 8,
            Self::Surround714 => 12,
            Self::Surround916 => 16,
            Self::Custom(n) => *n,
        }
    }

    /// 获取立体声对数量
    #[must_use]
    pub const fn stereo_pairs(&self) -> u16 {
        self.channels() / 2
    }

    /// 从声道数推断布局 (默认选择)
    #[must_use]
    pub const fn from_channels(channels: u16) -> Self {
        match channels {
            2 => Self::Stereo,
            6 => Self::Surround51,
            8 => Self::Surround71, // 默认 7.1，可手动指定 5.1.2
            12 => Self::Surround714,
            16 => Self::Surround916,
            n => Self::Custom(n),
        }
    }

    /// 获取各立体声对的名称
    #[must_use]
    pub fn pair_names(&self) -> Vec<&'static str> {
        match self {
            Self::Stereo => vec!["FL+FR"],
            Self::Surround51 => vec!["FL+FR", "FC+LFE", "BL+BR"],
            Self::Surround512 => vec!["FL+FR", "FC+LFE", "BL+BR", "TFL+TFR"],
            Self::Surround71 => vec!["FL+FR", "FC+LFE", "BL+BR", "SL+SR"],
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

/// 样本格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 16-bit 整数
    Int16,
    /// 24-bit 整数
    Int24,
    /// 32-bit 整数
    Int32,
    /// 32-bit 浮点
    Float32,
}

impl SampleFormat {
    /// 每样本的位数
    #[must_use]
    pub const fn bits_per_sample(&self) -> u16 {
        match self {
            Self::Int16 => 16,
            Self::Int24 => 24,
            Self::Int32 | Self::Float32 => 32,
        }
    }
}

/// 多声道音频数据
#[derive(Debug, Clone)]
pub struct MultichannelAudio {
    /// 每个声道的样本数据 [channel][sample]
    /// 统一存储为 i32，便于处理
    channels: Vec<Vec<i32>>,
    /// 采样率
    sample_rate: u32,
    /// 原始样本格式
    sample_format: SampleFormat,
}

impl MultichannelAudio {
    /// 创建新的多声道音频
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

    /// 获取声道数
    #[must_use]
    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    /// 获取每声道样本数
    #[must_use]
    pub fn num_samples(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    /// 获取采样率
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 获取样本格式
    #[must_use]
    pub const fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    /// 获取推断的声道布局
    #[must_use]
    pub fn layout(&self) -> ChannelLayout {
        ChannelLayout::from_channels(self.num_channels() as u16)
    }

    /// 从 WAV 文件加载
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
                    s.map(|v| (v * 2_147_483_647.0) as i32)
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

    /// 从 FLAC 文件加载
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
            for ch in 0..num_channels {
                let ch_samples = block.channel(ch as u32);
                channels[ch].extend(ch_samples.iter().copied());
            }

            buffer = block.into_buffer();
        }

        Self::new(channels, sample_rate, sample_format)
    }

    /// 从文件加载 (自动检测格式)
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

    /// 保存为 WAV 文件
    #[cfg(feature = "multichannel")]
    pub fn to_wav<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use hound::{SampleFormat as HoundFormat, WavSpec, WavWriter};

        let (hound_format, bits) = match self.sample_format {
            SampleFormat::Int16 => (HoundFormat::Int, 16),
            SampleFormat::Int24 => (HoundFormat::Int, 24),
            SampleFormat::Int32 => (HoundFormat::Int, 32),
            SampleFormat::Float32 => (HoundFormat::Float, 32),
        };

        let spec = WavSpec {
            channels: self.num_channels() as u16,
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
                        writer
                            .write_sample(ch[i] as i16)
                            .map_err(|e| Error::InvalidInput(format!("write error: {e}")))?;
                    }
                    SampleFormat::Int24 | SampleFormat::Int32 => {
                        writer
                            .write_sample(ch[i])
                            .map_err(|e| Error::InvalidInput(format!("write error: {e}")))?;
                    }
                    SampleFormat::Float32 => {
                        let f = ch[i] as f32 / 2_147_483_647.0;
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

    /// 获取交错格式的样本数据 (用于 FLAC 编码等)
    ///
    /// 返回 `[L0, R0, L1, R1, ...]` 格式的样本
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

    /// 拆分为立体声对
    ///
    /// 返回 `(left_channel, right_channel)` 的向量
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

    /// 从立体声对合并
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

    /// 保存立体声对到临时 WAV 文件
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

    /// 从立体声 WAV 文件加载并替换指定声道对
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

        self.channels[left_idx] = stereo.channels[0].clone();
        self.channels[right_idx] = stereo.channels[1].clone();

        Ok(())
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

        let audio = MultichannelAudio::new(channels.clone(), 48000, SampleFormat::Int24)
            .expect("create audio");

        let pairs = audio.split_stereo_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, vec![1, 2, 3, 4]);
        assert_eq!(pairs[0].1, vec![5, 6, 7, 8]);
        assert_eq!(pairs[1].0, vec![9, 10, 11, 12]);
        assert_eq!(pairs[1].1, vec![13, 14, 15, 16]);

        let merged = MultichannelAudio::merge_stereo_pairs(&pairs, 48000, SampleFormat::Int24)
            .expect("merge");
        assert_eq!(merged.num_channels(), 4);
    }
}
