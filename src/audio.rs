//! 音频水印嵌入/检测
//!
//! 封装 audiowmark 命令行工具

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};
use crate::message::{self, MESSAGE_LEN};
use crate::tag::Tag;

#[cfg(feature = "multichannel")]
use crate::multichannel::{ChannelLayout, MultichannelAudio};

/// audiowmark 默认搜索路径
const DEFAULT_SEARCH_PATHS: &[&str] = &[
    "audiowmark",
    "/usr/local/bin/audiowmark",
    "/opt/homebrew/bin/audiowmark",
];
/// audiowmark 0.6.x 候选分数阈值（低于此值通常为伪命中）
const MIN_PATTERN_SCORE: f32 = 1.0;

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
    pub fn strength(mut self, strength: u8) -> Self {
        self.strength = strength.clamp(1, 30);
        self
    }

    /// 设置密钥文件
    pub fn key_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.key_file = Some(path.as_ref().to_path_buf());
        self
    }

    /// 返回 audiowmark 二进制路径
    pub fn binary_path(&self) -> &Path {
        &self.binary_path
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
        validate_input_format(input.as_ref())?;
        let hex = bytes_to_hex(message);

        let mut cmd = self.audiowmark_command();
        cmd.arg("add")
            .arg("--strength")
            .arg(self.strength.to_string());

        if let Some(ref key_file) = self.key_file {
            cmd.arg("--key").arg(key_file);
        }

        cmd.arg(input.as_ref()).arg(output.as_ref()).arg(&hex);

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
        validate_input_format(input.as_ref())?;
        let mut cmd = self.audiowmark_command();
        cmd.arg("get");

        if let Some(ref key_file) = self.key_file {
            cmd.arg("--key").arg(key_file);
        }

        cmd.arg(input.as_ref());

        let output = cmd
            .output()
            .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

        // audiowmark 在没有检测到水印时可能返回非零状态
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 解析输出
        parse_detect_output(&stdout, &stderr)
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
    pub fn is_available(&self) -> bool {
        self.audiowmark_command()
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 获取 audiowmark 版本
    pub fn version(&self) -> Result<String> {
        let output = self.audiowmark_command()
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
        use std::fs;

        let input = input.as_ref();
        let output = output.as_ref();

        // 加载多声道音频
        let audio = MultichannelAudio::from_file(input)?;
        let num_channels = audio.num_channels();

        // 如果是立体声，直接使用普通方法
        if num_channels == 2 {
            return self.embed(input, output, message);
        }

        // 确定声道布局
        let layout = layout.unwrap_or_else(|| audio.layout());
        let pair_names = layout.pair_names();
        let pairs = audio.split_stereo_pairs();

        // 创建临时目录（含线程ID和时间戳，避免并行冲突）
        let temp_dir = std::env::temp_dir().join(format!(
            "awmkit_{}_{:?}_{}",
            std::process::id(),
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&temp_dir)?;

        // 处理每个立体声对
        let mut processed_pairs = Vec::with_capacity(pairs.len());

        for (i, (left, right)) in pairs.into_iter().enumerate() {
            let pair_name = pair_names.get(i).copied().unwrap_or("Unknown");
            let temp_input = temp_dir.join(format!("pair_{i}_in.wav"));
            let temp_output = temp_dir.join(format!("pair_{i}_out.wav"));

            // 保存立体声对到临时文件
            let stereo = MultichannelAudio::new(
                vec![left.clone(), right.clone()],
                audio.sample_rate(),
                audio.sample_format(),
            )?;
            stereo.to_wav(&temp_input)?;

            // 嵌入水印
            match self.embed(&temp_input, &temp_output, message) {
                Ok(()) => {
                    // 加载处理后的立体声
                    let processed = MultichannelAudio::from_wav(&temp_output)?;
                    let processed_pairs_data = processed.split_stereo_pairs();
                    if let Some((l, r)) = processed_pairs_data.into_iter().next() {
                        processed_pairs.push((l, r));
                    } else {
                        processed_pairs.push((left, right));
                    }
                }
                Err(e) => {
                    // 嵌入失败，保留原始数据
                    eprintln!("Warning: Failed to embed in {pair_name}: {e}");
                    processed_pairs.push((left, right));
                }
            }

            // 清理临时文件
            let _ = fs::remove_file(&temp_input);
            let _ = fs::remove_file(&temp_output);
        }

        // 合并所有声道对
        let result = MultichannelAudio::merge_stereo_pairs(
            &processed_pairs,
            audio.sample_rate(),
            audio.sample_format(),
        )?;

        // 保存输出
        result.to_wav(output)?;

        // 清理临时目录
        let _ = fs::remove_dir(&temp_dir);

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
        use std::fs;

        let input = input.as_ref();

        // 加载多声道音频
        let audio = MultichannelAudio::from_file(input)?;
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
        let pair_names = layout.pair_names();

        // 创建临时目录（含线程ID和时间戳，避免并行冲突）
        let temp_dir = std::env::temp_dir().join(format!(
            "awmkit_detect_{}_{:?}_{}",
            std::process::id(),
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&temp_dir)?;

        let mut pairs_results = Vec::new();
        let mut best: Option<DetectResult> = None;

        for (i, _) in audio.split_stereo_pairs().iter().enumerate() {
            let pair_name = pair_names.get(i).copied().unwrap_or("Unknown").to_string();
            let temp_file = temp_dir.join(format!("pair_{i}.wav"));

            // 保存立体声对
            audio.save_stereo_pair(i, &temp_file)?;

            // 检测水印
            let result = self.detect(&temp_file)?;

            // 更新最佳结果 (选择比特错误最少的)
            if let Some(ref r) = result {
                if best.is_none() || r.bit_errors < best.as_ref().map_or(u32::MAX, |b| b.bit_errors)
                {
                    best = Some(r.clone());
                }
            }

            pairs_results.push((i, pair_name, result));

            // 清理临时文件
            let _ = fs::remove_file(&temp_file);
        }

        // 清理临时目录
        let _ = fs::remove_dir(&temp_dir);

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

    fn resolve_binary(fallback_path: Option<&Path>) -> Result<PathBuf> {
        #[cfg(feature = "bundled")]
        {
            if let Ok(path) = crate::bundled::ensure_extracted() {
                return Ok(path);
            }
        }

        if let Some(path) = fallback_path {
            if let Ok(audio) = Self::with_binary(path) {
                return Ok(audio.binary_path);
            }
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

/// 解析 audiowmark get 输出
fn parse_detect_output(stdout: &str, stderr: &str) -> Result<Option<DetectResult>> {
    // 查找 pattern 行
    // 格式: "pattern  all 0101c1d05978131b57f7deb8e22a0b78"
    // 或:   "pattern   single 0101c1d05978131b57f7deb8e22a0b78 0"
    // 或:   "pattern  0:00 00000000000000000000000000000000 0.000 -0.001 CLIP-B" (audiowmark 0.6.x)

    let combined = format!("{}\n{}", stdout, stderr);

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

                    return Ok(Some(DetectResult {
                        raw_message,
                        pattern,
                        detect_score,
                        bit_errors,
                        match_found: true,
                    }));
                }
            }
        }
    }

    // 没有检测到水印
    Ok(None)
}

fn validate_input_format(path: &Path) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        Some("wav") | Some("flac") => Ok(()),
        Some(ext) => Err(Error::InvalidInput(format!(
            "unsupported audio format: .{ext} (supported: wav, flac)"
        ))),
        None => Err(Error::InvalidInput(
            "input file has no extension (supported: wav, flac)".to_string(),
        )),
    }
}

/// 字节数组转 hex 字符串
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
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
        let bytes = hex_to_bytes(hex).unwrap();
        assert_eq!(bytes[0], 0x01);
        assert_eq!(bytes[15], 0xef);
    }

    #[test]
    fn test_parse_detect_output() {
        let stdout = "pattern  all 0101c1d05978131b57f7deb8e22a0b78\n";
        let result = parse_detect_output(stdout, "").unwrap().unwrap();
        assert_eq!(result.pattern, "all");
        assert_eq!(result.detect_score, None);
        assert_eq!(result.raw_message[0], 0x01);
    }

    #[test]
    fn test_parse_detect_with_errors() {
        let stdout = "pattern   single 0101c1d05978131b57f7deb8e22a0b78 3\n";
        let result = parse_detect_output(stdout, "").unwrap().unwrap();
        assert_eq!(result.pattern, "single");
        assert_eq!(result.detect_score, None);
        assert_eq!(result.bit_errors, 3);
    }

    #[test]
    fn test_parse_detect_zero_message_as_not_found() {
        let stdout = "pattern  0:00 00000000000000000000000000000000 0.000 -0.001 CLIP-B\n";
        let result = parse_detect_output(stdout, "").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_detect_skip_zero_and_take_next() {
        let stdout = concat!(
            "pattern  0:00 00000000000000000000000000000000 0.000 -0.001 CLIP-B\n",
            "pattern  0:00 0101c1d05978131b57f7deb8e22a0b78 1.792 0.121 CLIP-B\n"
        );
        let result = parse_detect_output(stdout, "").unwrap().unwrap();
        assert_eq!(result.raw_message[0], 0x01);
        assert!(result
            .detect_score
            .is_some_and(|value| (value - 1.792).abs() < 0.0001));
        assert_eq!(result.bit_errors, 0);
    }

    #[test]
    fn test_parse_detect_ignore_low_score_candidate() {
        let stdout = "pattern  1:28 bb4aaa05ad77bf5e73c8eb37e44f0c94 0.209 0.379 A\n";
        let result = parse_detect_output(stdout, "").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_detect_accept_high_score_candidate() {
        let stdout = "pattern  0:05 023848c0200045fffff7d8743d035cda 1.427 0.065 A\n";
        let result = parse_detect_output(stdout, "").unwrap().unwrap();
        assert_eq!(result.raw_message[0], 0x02);
        assert!(result
            .detect_score
            .is_some_and(|value| (value - 1.427).abs() < 0.0001));
    }
}
