//! 音频水印嵌入/检测
//!
//! 封装 audiowmark 命令行工具

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};
use crate::message::{self, MESSAGE_LEN};
use crate::tag::Tag;

/// audiowmark 默认搜索路径
const DEFAULT_SEARCH_PATHS: &[&str] = &[
    "audiowmark",
    "/usr/local/bin/audiowmark",
    "/opt/homebrew/bin/audiowmark",
];

/// 水印嵌入/检测结果
#[derive(Debug, Clone)]
pub struct DetectResult {
    /// 提取的原始消息 (16 bytes)
    pub raw_message: [u8; MESSAGE_LEN],
    /// 检测模式 (all/single)
    pub pattern: String,
    /// 比特错误数
    pub bit_errors: u32,
    /// 是否匹配
    pub match_found: bool,
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
    /// 创建 Audio 实例，自动搜索 audiowmark
    pub fn new() -> Result<Self> {
        Self::find_binary()
            .map(|path| Self {
                binary_path: path,
                strength: 10,
                key_file: None,
            })
            .ok_or_else(|| Error::AudiowmarkNotFound)
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

    /// 嵌入水印消息到音频
    ///
    /// # Arguments
    /// - `input`: 输入音频路径
    /// - `output`: 输出音频路径
    /// - `message`: 16 字节消息
    pub fn embed<P: AsRef<Path>>(&self, input: P, output: P, message: &[u8; MESSAGE_LEN]) -> Result<()> {
        let hex = bytes_to_hex(message);

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("add")
            .arg("--short")
            .arg("16")
            .arg("--strength")
            .arg(self.strength.to_string());

        if let Some(ref key_file) = self.key_file {
            cmd.arg("--key").arg(key_file);
        }

        cmd.arg(input.as_ref())
            .arg(output.as_ref())
            .arg(&hex);

        let output = cmd.output().map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

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
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("get")
            .arg("--short")
            .arg("16");

        if let Some(ref key_file) = self.key_file {
            cmd.arg("--key").arg(key_file);
        }

        cmd.arg(input.as_ref());

        let output = cmd.output().map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

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
        Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 获取 audiowmark 版本
    pub fn version(&self) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .map_err(|e| Error::AudiowmarkExec(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
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

    let combined = format!("{}\n{}", stdout, stderr);

    for line in combined.lines() {
        let line = line.trim();
        if line.starts_with("pattern") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let pattern = parts[1].to_string();
                let hex = parts[2];

                if let Some(raw_message) = hex_to_bytes(hex) {
                    let bit_errors = if parts.len() >= 4 {
                        parts[3].parse().unwrap_or(0)
                    } else {
                        0
                    };

                    return Ok(Some(DetectResult {
                        raw_message,
                        pattern,
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
        let bytes = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
                     0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
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
        assert_eq!(result.raw_message[0], 0x01);
    }

    #[test]
    fn test_parse_detect_with_errors() {
        let stdout = "pattern   single 0101c1d05978131b57f7deb8e22a0b78 3\n";
        let result = parse_detect_output(stdout, "").unwrap().unwrap();
        assert_eq!(result.pattern, "single");
        assert_eq!(result.bit_errors, 3);
    }
}
