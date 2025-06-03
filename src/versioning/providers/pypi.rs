use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use crate::versioning::base::VersionChecker;
use crate::versioning::models::{Package, VersionInfo, Registry};

/// PyPI 包信息
#[derive(Debug, Deserialize, Serialize)]
pub struct PyPIPackageInfo {
    pub info: PyPIInfo,
    pub releases: std::collections::HashMap<String, Vec<PyPIRelease>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PyPIInfo {
    pub name: String,
    pub version: String,
    pub summary: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub home_page: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PyPIRelease {
    pub filename: String,
    pub python_version: String,
    pub upload_time: String,
}

/// PyPI 版本检查器
pub struct PyPIChecker {
    client: reqwest::Client,
    base_url: String,
}

impl PyPIChecker {
    /// 解析PyPI的发布时间
    fn parse_release_date(&self, upload_time: &str) -> chrono::DateTime<Utc> {
        // PyPI的时间格式: "2023-10-20T14:30:15"
        DateTime::parse_from_rfc3339(&format!("{}Z", upload_time))
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    }
}

#[async_trait]
impl VersionChecker for PyPIChecker {
    fn registry(&self) -> Registry {
        Registry::PyPI
    }

    async fn check_version(&self, package: &Package) -> Result<VersionInfo> {
        let url = format!("{}/{}/json", self.base_url, package.name);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("PyPI API请求失败: {}", response.status()));
        }
        
        let package_info: PyPIPackageInfo = response.json().await?;
        
        // 解析最新版本的发布日期
        let release_date = if let Some(releases) = package_info.releases.get(&package_info.info.version) {
            if let Some(first_release) = releases.first() {
                self.parse_release_date(&first_release.upload_time)
            } else {
                Utc::now()
            }
        } else {
            Utc::now()
        };

        // 获取所有版本列表
        let mut versions: Vec<String> = package_info.releases.keys().cloned().collect();
        versions.sort_by(|a, b| {
            // 尝试按语义版本排序
            match version_compare::compare(a, b) {
                Ok(version_compare::Cmp::Lt) => std::cmp::Ordering::Less,
                Ok(version_compare::Cmp::Eq) => std::cmp::Ordering::Equal,
                Ok(version_compare::Cmp::Gt) => std::cmp::Ordering::Greater,
                Ok(version_compare::Cmp::Ne) => std::cmp::Ordering::Equal, // 不相等但无法比较大小，视为相等
                Ok(version_compare::Cmp::Le) => std::cmp::Ordering::Less,  // 小于等于，视为小于
                Ok(version_compare::Cmp::Ge) => std::cmp::Ordering::Greater, // 大于等于，视为大于
                Err(_) => std::cmp::Ordering::Equal,
            }
        });
        
        Ok(VersionInfo {
            package: package.clone(),
            latest_stable: package_info.info.version,
            latest_preview: None, // PyPI不区分预览版
            release_date,
            eol_date: None,
            available_versions: versions,
            dependencies: None,
            downloads: None,
        })
    }
    
    async fn list_versions(&self, package: &Package) -> Result<Vec<String>> {
        let url = format!("{}/{}/json", self.base_url, package.name);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("PyPI API请求失败: {}", response.status()));
        }
        
        let package_info: PyPIPackageInfo = response.json().await?;
        
        let mut versions: Vec<String> = package_info.releases.keys().cloned().collect();
        versions.sort_by(|a, b| {
            match version_compare::compare(a, b) {
                Ok(version_compare::Cmp::Lt) => std::cmp::Ordering::Less,
                Ok(version_compare::Cmp::Eq) => std::cmp::Ordering::Equal,
                Ok(version_compare::Cmp::Gt) => std::cmp::Ordering::Greater,
                Ok(version_compare::Cmp::Ne) => std::cmp::Ordering::Equal, // 不相等但无法比较大小，视为相等
                Ok(version_compare::Cmp::Le) => std::cmp::Ordering::Less,  // 小于等于，视为小于
                Ok(version_compare::Cmp::Ge) => std::cmp::Ordering::Greater, // 大于等于，视为大于
                Err(_) => std::cmp::Ordering::Equal,
            }
        });
        
        Ok(versions)
    }

    async fn get_dependencies(&self, _package: &Package) -> Result<Option<serde_json::Value>> {
        // PyPI API不直接提供依赖信息，需要解析setup.py或requirements.txt
        Ok(None)
    }
} 