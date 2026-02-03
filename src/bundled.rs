//! Bundled audiowmark 二进制管理
//!
//! 编译时嵌入平台特定的 audiowmark 二进制（zstd 压缩），运行时解压到缓存目录。

use crate::error::{Error, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// 平台特定的嵌入二进制（条件编译）
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const AUDIOWMARK_COMPRESSED: &[u8] =
    include_bytes!("../bundled/audiowmark-macos-arm64.zst");
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const AUDIOWMARK_SHA256: &str =
    "6326ff890108c4b89b12578e088e826de32b97597c87ed0f97af2830b063b4df";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const AUDIOWMARK_COMPRESSED: &[u8] =
    include_bytes!("../bundled/audiowmark-macos-x86_64.zst");
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const AUDIOWMARK_SHA256: &str = "placeholder"; // TODO: 待添加

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const AUDIOWMARK_COMPRESSED: &[u8] =
    include_bytes!("../bundled/audiowmark-windows-x86_64.exe.zst");
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const AUDIOWMARK_SHA256: &str = "placeholder"; // TODO: 待添加

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const AUDIOWMARK_COMPRESSED: &[u8] =
    include_bytes!("../bundled/audiowmark-linux-x86_64.zst");
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const AUDIOWMARK_SHA256: &str = "placeholder"; // TODO: 待添加

/// Bundled 二进制管理器
pub struct BundledBinary {
    cache_dir: PathBuf,
}

impl BundledBinary {
    /// 创建实例
    pub fn new() -> Result<Self> {
        let cache_dir = get_cache_dir()?;
        Ok(Self { cache_dir })
    }

    /// 确保二进制已解压，返回路径
    pub fn ensure_extracted(&self) -> Result<PathBuf> {
        let binary_path = self.cache_dir.join(binary_name());

        // 如果已存在且校验通过，直接返回
        if binary_path.exists() && verify_binary(&binary_path)? {
            return Ok(binary_path);
        }

        // 创建缓存目录
        fs::create_dir_all(&self.cache_dir)?;

        // 解压到临时文件
        let temp_path = self.cache_dir.join(format!("{}.tmp", binary_name()));
        extract_binary(&temp_path)?;

        // 验证解压结果
        if !verify_binary(&temp_path)? {
            fs::remove_file(&temp_path)?;
            return Err(Error::InvalidInput(
                "bundled binary verification failed after extraction".to_string(),
            ));
        }

        // 设置可执行权限（Unix）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_path, perms)?;
        }

        // 原子重命名
        fs::rename(&temp_path, &binary_path)?;

        Ok(binary_path)
    }

    /// 获取二进制路径（不自动解压）
    pub fn path(&self) -> PathBuf {
        self.cache_dir.join(binary_name())
    }
}

/// 解压 bundled 二进制到指定路径
fn extract_binary(dest: &Path) -> Result<()> {
    let decompressed = zstd::decode_all(AUDIOWMARK_COMPRESSED)
        .map_err(|e| Error::InvalidInput(format!("failed to decompress bundled binary: {e}")))?;

    let mut file = fs::File::create(dest)?;
    file.write_all(&decompressed)?;
    file.sync_all()?;

    Ok(())
}

/// 验证二进制文件（简单校验：文件大小 > 0）
fn verify_binary(path: &Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(meta) => Ok(meta.len() > 0),
        Err(_) => Ok(false),
    }
}

/// 获取平台特定的缓存目录
fn get_cache_dir() -> Result<PathBuf> {
    let base_dir = if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        dirs::home_dir()
            .ok_or_else(|| Error::InvalidInput("could not find home directory".to_string()))?
            .join(".awmkit")
    } else if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .ok_or_else(|| Error::InvalidInput("could not find local data directory".to_string()))?
            .join("awmkit")
    } else {
        return Err(Error::InvalidInput("unsupported platform".to_string()));
    };

    Ok(base_dir.join("bin"))
}

/// 获取平台特定的二进制文件名
#[cfg(not(target_os = "windows"))]
fn binary_name() -> &'static str {
    "audiowmark"
}

#[cfg(target_os = "windows")]
fn binary_name() -> &'static str {
    "audiowmark.exe"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_binary_extraction() {
        let bundled = BundledBinary::new().unwrap();
        let path = bundled.ensure_extracted().unwrap();
        assert!(path.exists());
        assert!(path.is_file());
    }
}
