use crate::app::error::Result;
use crate::app::keystore::KeyStore;
use crate::app::settings::AppSettings;
use crate::app::tag_store::TagStore;
use crate::bundled;
use std::fs;

/// # Errors
/// 当本地缓存目录删除或配置文件删除失败时返回错误。
pub fn clear_local_cache() -> Result<()> {
    let cache_root = bundled::cache_root()?;
    if cache_root.exists() {
        fs::remove_dir_all(&cache_root)?;
    }
    AppSettings::remove_config()?;
    Ok(())
}

/// # Errors
/// 当缓存清理、标签库清空或密钥库删除失败时返回错误。
pub fn reset_all() -> Result<()> {
    clear_local_cache()?;
    let mut store = TagStore::load()?;
    store.clear()?;
    let keystore = KeyStore::new()?;
    if keystore.exists() {
        keystore.delete()?;
    }
    Ok(())
}
