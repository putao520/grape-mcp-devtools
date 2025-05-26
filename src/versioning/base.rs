use async_trait::async_trait;
use anyhow::Result;
use crate::versioning::models::{Package, VersionInfo, Registry};

/// 版本检查器trait
#[async_trait]
pub trait VersionChecker: Send + Sync {
    /// 获取支持的包注册表类型
    fn registry(&self) -> Registry;
    
    /// 获取包的版本信息
    async fn check_version(&self, package: &Package) -> Result<VersionInfo>;
    
    /// 获取包的所有可用版本
    async fn list_versions(&self, package: &Package) -> Result<Vec<String>>;
    
    /// 获取包的依赖信息
    async fn get_dependencies(&self, package: &Package) -> Result<Option<serde_json::Value>>;
    
    /// 检查版本是否需要更新
    async fn needs_update(&self, current: &str, target: &str) -> Result<bool> {
        // 默认实现使用semver比较
        let current = semver::Version::parse(current)?;
        let target = semver::Version::parse(target)?;
        Ok(target > current)
    }
}
