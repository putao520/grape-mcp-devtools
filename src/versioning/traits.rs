use async_trait::async_trait;
use anyhow::Result;
use crate::versioning::models::Package;

#[async_trait]
pub trait PackageProvider: Send + Sync {
    async fn get_package_info(&self, package_name: &str) -> Result<Package>;
    async fn get_dependencies(&self, package: &Package) -> Result<Option<serde_json::Value>>;
} 