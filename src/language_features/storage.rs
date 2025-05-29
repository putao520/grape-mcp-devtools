// 存储模块
// 未来可以实现持久化存储功能，如SQLite数据库或文件系统存储

use std::future::Future;
use anyhow::Result;
use super::data_models::LanguageVersion;

/// 语言版本存储trait
#[allow(async_fn_in_trait)]
pub trait LanguageVersionStorage: Send + Sync {
    fn store_version(&self, version: &LanguageVersion) -> impl Future<Output = Result<()>> + Send;
    fn get_version(&self, language: &str, version: &str) -> impl Future<Output = Result<Option<LanguageVersion>>> + Send;
    fn list_versions(&self, language: &str) -> impl Future<Output = Result<Vec<String>>> + Send;
    fn delete_version(&self, language: &str, version: &str) -> impl Future<Output = Result<()>> + Send;
}

// 内存存储实现
pub struct InMemoryStorage {
    // 实现留待未来扩展
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {}
    }
} 