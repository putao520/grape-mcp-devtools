use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tracing::{info, debug};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use super::data_models::*;

/// 数据采集器特质
#[async_trait]
pub trait LanguageVersionCollector: Send + Sync {
    /// 获取支持的语言名称
    fn language(&self) -> &str;
    
    /// 获取所有可用版本列表
    async fn get_versions(&self) -> Result<Vec<String>>;
    
    /// 获取特定版本的详细信息
    async fn get_version_details(&self, version: &str) -> Result<LanguageVersion>;
    
    /// 获取最新版本
    async fn get_latest_version(&self) -> Result<LanguageVersion>;
    
    /// 检查是否支持某个版本
    async fn is_version_supported(&self, version: &str) -> bool;
}

/// Rust版本采集器
pub struct RustVersionCollector {
    client: Client,
    github_api_base: String,
}

impl RustVersionCollector {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            github_api_base: "https://api.github.com/repos/rust-lang/rust".to_string(),
        }
    }
    
    async fn fetch_github_releases(&self) -> Result<Vec<Value>> {
        let url = format!("{}/releases", self.github_api_base);
        debug!("获取Rust GitHub releases: {}", url);
        
        let response = self.client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Grape-MCP-DevTools/1.0")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API请求失败: {}", response.status()));
        }
        
        let releases: Vec<Value> = response.json().await?;
        Ok(releases)
    }
    
    fn parse_rust_version(&self, release: &Value) -> Result<LanguageVersion> {
        let tag_name = release["tag_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("无法获取版本标签"))?;
            
        let version = tag_name.trim_start_matches('v');
        
        let release_date = release["published_at"].as_str()
            .ok_or_else(|| anyhow::anyhow!("无法获取发布日期"))?;
        let release_date = DateTime::parse_from_rfc3339(release_date)?
            .with_timezone(&Utc);
            
        let is_prerelease = release["prerelease"].as_bool().unwrap_or(false);
        let is_stable = !is_prerelease;
        
        let body = release["body"].as_str().unwrap_or("");
        let features = self.extract_rust_features(body);
        
        Ok(LanguageVersion {
            language: "rust".to_string(),
            version: version.to_string(),
            release_date,
            is_stable,
            is_lts: false, // Rust不使用LTS模式
            status: if is_stable { VersionStatus::Current } else { VersionStatus::Preview },
            features,
            syntax_changes: vec![], // 需要进一步解析
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: release["html_url"].as_str().map(|s| s.to_string()),
                download_url: release["assets"].as_array()
                    .and_then(|assets| assets.first())
                    .and_then(|asset| asset["browser_download_url"].as_str())
                    .map(|s| s.to_string()),
                source_url: Some(format!("https://github.com/rust-lang/rust/tree/{}", tag_name)),
                documentation_url: Some(format!("https://doc.rust-lang.org/{}/", version)),
                changelog_url: Some(format!("https://github.com/rust-lang/rust/blob/master/RELEASES.md#{}", 
                    version.replace('.', ""))),
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    fn extract_rust_features(&self, release_notes: &str) -> Vec<LanguageFeature> {
        let mut features = Vec::new();
        
        // 简单的特性提取逻辑，实际实现需要更复杂的解析
        for line in release_notes.lines() {
            if line.starts_with("- ") || line.starts_with("* ") {
                let description = line.trim_start_matches("- ").trim_start_matches("* ");
                if !description.is_empty() {
                    features.push(LanguageFeature {
                        name: self.extract_feature_name(description),
                        description: description.to_string(),
                        category: self.categorize_feature(description),
                        examples: vec![],
                        proposal_link: None,
                        documentation_link: None,
                        stability: FeatureStability::Stable,
                        tags: vec![],
                        impact: ImpactLevel::Medium,
                    });
                }
            }
        }
        
        features
    }
    
    fn extract_feature_name(&self, description: &str) -> String {
        // 提取特性名称的简单逻辑
        description.split(':').next()
            .unwrap_or(description)
            .split('(').next()
            .unwrap_or(description)
            .trim()
            .to_string()
    }
    
    fn categorize_feature(&self, description: &str) -> FeatureCategory {
        let desc_lower = description.to_lowercase();
        
        if desc_lower.contains("async") || desc_lower.contains("await") {
            FeatureCategory::Async
        } else if desc_lower.contains("type") || desc_lower.contains("trait") {
            FeatureCategory::TypeSystem
        } else if desc_lower.contains("macro") {
            FeatureCategory::Macros
        } else if desc_lower.contains("std") || desc_lower.contains("library") {
            FeatureCategory::StandardLibrary
        } else if desc_lower.contains("cargo") || desc_lower.contains("tool") {
            FeatureCategory::Toolchain
        } else if desc_lower.contains("syntax") {
            FeatureCategory::Syntax
        } else if desc_lower.contains("performance") || desc_lower.contains("optimization") {
            FeatureCategory::Performance
        } else {
            FeatureCategory::Other("General".to_string())
        }
    }
}

#[async_trait]
impl LanguageVersionCollector for RustVersionCollector {
    fn language(&self) -> &str {
        "rust"
    }
    
    async fn get_versions(&self) -> Result<Vec<String>> {
        let releases = self.fetch_github_releases().await?;
        let mut versions = Vec::new();
        
        for release in releases {
            if let Some(tag_name) = release["tag_name"].as_str() {
                let version = tag_name.trim_start_matches('v');
                versions.push(version.to_string());
            }
        }
        
        info!("获取到 {} 个Rust版本", versions.len());
        Ok(versions)
    }
    
    async fn get_version_details(&self, version: &str) -> Result<LanguageVersion> {
        let releases = self.fetch_github_releases().await?;
        
        for release in releases {
            if let Some(tag_name) = release["tag_name"].as_str() {
                let release_version = tag_name.trim_start_matches('v');
                if release_version == version {
                    return self.parse_rust_version(&release);
                }
            }
        }
        
        Err(anyhow::anyhow!("未找到Rust版本: {}", version))
    }
    
    async fn get_latest_version(&self) -> Result<LanguageVersion> {
        let releases = self.fetch_github_releases().await?;
        
        if let Some(latest_release) = releases.first() {
            return self.parse_rust_version(latest_release);
        }
        
        Err(anyhow::anyhow!("无法获取最新Rust版本"))
    }
    
    async fn is_version_supported(&self, version: &str) -> bool {
        match self.get_versions().await {
            Ok(versions) => versions.contains(&version.to_string()),
            Err(_) => false,
        }
    }
}

/// Python版本采集器
pub struct PythonVersionCollector {
    client: Client,
}

impl PythonVersionCollector {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    async fn fetch_python_releases(&self) -> Result<Vec<Value>> {
        let url = "https://api.github.com/repos/python/cpython/releases";
        debug!("获取Python GitHub releases: {}", url);
        
        let response = self.client
            .get(url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Grape-MCP-DevTools/1.0")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API请求失败: {}", response.status()));
        }
        
        let releases: Vec<Value> = response.json().await?;
        Ok(releases)
    }
    
    fn parse_python_version(&self, release: &Value) -> Result<LanguageVersion> {
        let tag_name = release["tag_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("无法获取版本标签"))?;
            
        let version = tag_name.trim_start_matches('v');
        
        let release_date = release["published_at"].as_str()
            .ok_or_else(|| anyhow::anyhow!("无法获取发布日期"))?;
        let release_date = DateTime::parse_from_rfc3339(release_date)?
            .with_timezone(&Utc);
            
        let is_prerelease = release["prerelease"].as_bool().unwrap_or(false);
        let is_stable = !is_prerelease;
        
        let body = release["body"].as_str().unwrap_or("");
        let features = self.extract_python_features(body);
        
        Ok(LanguageVersion {
            language: "python".to_string(),
            version: version.to_string(),
            release_date,
            is_stable,
            is_lts: false, // Python不使用LTS模式
            status: if is_stable { VersionStatus::Current } else { VersionStatus::Preview },
            features,
            syntax_changes: vec![], // 需要进一步解析
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: release["html_url"].as_str().map(|s| s.to_string()),
                download_url: release["assets"].as_array()
                    .and_then(|assets| assets.first())
                    .and_then(|asset| asset["browser_download_url"].as_str())
                    .map(|s| s.to_string()),
                source_url: Some(format!("https://github.com/python/cpython/tree/{}", tag_name)),
                documentation_url: Some(format!("https://docs.python.org/{}/", version)),
                changelog_url: Some(format!("https://github.com/python/cpython/blob/{}/CHANGELOG", version)),
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    fn extract_python_features(&self, release_notes: &str) -> Vec<LanguageFeature> {
        let mut features = Vec::new();
        
        // 简单的特性提取逻辑，实际实现需要更复杂的解析
        for line in release_notes.lines() {
            if line.starts_with("- ") || line.starts_with("* ") {
                let description = line.trim_start_matches("- ").trim_start_matches("* ");
                if !description.is_empty() {
                    features.push(LanguageFeature {
                        name: self.extract_python_feature_name(description),
                        description: description.to_string(),
                        category: self.categorize_python_feature(description),
                        examples: vec![],
                        proposal_link: None,
                        documentation_link: None,
                        stability: FeatureStability::Stable,
                        tags: vec![],
                        impact: ImpactLevel::Medium,
                    });
                }
            }
        }
        
        features
    }
    
    fn extract_python_feature_name(&self, description: &str) -> String {
        // 提取特性名称的简单逻辑
        description.split(':').next()
            .unwrap_or(description)
            .split('(').next()
            .unwrap_or(description)
            .trim()
            .to_string()
    }
    
    fn categorize_python_feature(&self, description: &str) -> FeatureCategory {
        let desc_lower = description.to_lowercase();
        
        if desc_lower.contains("async") || desc_lower.contains("await") {
            FeatureCategory::Async
        } else if desc_lower.contains("type") || desc_lower.contains("annotation") {
            FeatureCategory::TypeSystem
        } else if desc_lower.contains("decorator") || desc_lower.contains("@") {
            FeatureCategory::Syntax
        } else if desc_lower.contains("stdlib") || desc_lower.contains("library") || desc_lower.contains("module") {
            FeatureCategory::StandardLibrary
        } else if desc_lower.contains("pip") || desc_lower.contains("tool") {
            FeatureCategory::Toolchain
        } else if desc_lower.contains("syntax") || desc_lower.contains("expression") {
            FeatureCategory::Syntax
        } else if desc_lower.contains("performance") || desc_lower.contains("optimization") {
            FeatureCategory::Performance
        } else {
            FeatureCategory::Other("General".to_string())
        }
    }
}

#[async_trait]
impl LanguageVersionCollector for PythonVersionCollector {
    fn language(&self) -> &str {
        "python"
    }
    
    async fn get_versions(&self) -> Result<Vec<String>> {
        let releases = self.fetch_python_releases().await?;
        let mut versions = Vec::new();
        
        for release in releases {
            if let Some(tag_name) = release["tag_name"].as_str() {
                let version = tag_name.trim_start_matches('v');
                versions.push(version.to_string());
            }
        }
        
        info!("获取到 {} 个Python版本", versions.len());
        Ok(versions)
    }
    
    async fn get_version_details(&self, version: &str) -> Result<LanguageVersion> {
        let releases = self.fetch_python_releases().await?;
        
        for release in releases {
            if let Some(tag_name) = release["tag_name"].as_str() {
                let release_version = tag_name.trim_start_matches('v');
                if release_version == version {
                    return self.parse_python_version(&release);
                }
            }
        }
        
        Err(anyhow::anyhow!("未找到Python版本: {}", version))
    }
    
    async fn get_latest_version(&self) -> Result<LanguageVersion> {
        let releases = self.fetch_python_releases().await?;
        
        if let Some(latest_release) = releases.first() {
            return self.parse_python_version(latest_release);
        }
        
        Err(anyhow::anyhow!("无法获取最新Python版本"))
    }
    
    async fn is_version_supported(&self, version: &str) -> bool {
        match self.get_versions().await {
            Ok(versions) => versions.contains(&version.to_string()),
            Err(_) => false,
        }
    }
}

/// 采集器工厂
pub struct CollectorFactory;

impl CollectorFactory {
    pub fn create_collector(language: &str) -> Result<Box<dyn LanguageVersionCollector>> {
        match language.to_lowercase().as_str() {
            "rust" => Ok(Box::new(RustVersionCollector::new())),
            "python" => Ok(Box::new(PythonVersionCollector::new())),
            // 更多语言采集器...
            _ => Err(anyhow::anyhow!("不支持的语言: {}", language)),
        }
    }
    
    pub fn supported_languages() -> Vec<&'static str> {
        vec!["rust", "python"]
    }
} 