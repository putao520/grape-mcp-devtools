use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tracing::{info, debug, warn, error};
use std::collections::HashMap;
use chrono::Utc;
use std::time::Duration;

use crate::language_features::data_models::{LanguageFeature, FeatureCategory, LanguageVersion, VersionStatus, FeatureStability, ImpactLevel, VersionMetadata};
use super::collectors::LanguageVersionCollector;

/// 增强的语言版本采集器
pub struct EnhancedLanguageCollector {
    client: Client,
    language: String,
    config: CollectorConfig,
}

/// 采集器配置
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    pub timeout: Duration,
    pub max_retries: u32,
    pub cache_ttl: Duration,
    pub user_agent: String,
    pub api_endpoints: HashMap<String, String>,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        let mut api_endpoints = HashMap::new();
        
        // GitHub API endpoints
        api_endpoints.insert("rust".to_string(), "https://api.github.com/repos/rust-lang/rust/releases".to_string());
        api_endpoints.insert("python".to_string(), "https://api.github.com/repos/python/cpython/tags".to_string()); // 使用tags而不是releases
        api_endpoints.insert("javascript".to_string(), "https://api.github.com/repos/nodejs/node/releases".to_string());
        api_endpoints.insert("java".to_string(), "https://api.github.com/repos/openjdk/jdk/tags".to_string());
        api_endpoints.insert("go".to_string(), "https://api.github.com/repos/golang/go/tags".to_string());
        api_endpoints.insert("csharp".to_string(), "https://api.github.com/repos/dotnet/core/releases".to_string());
        
        // 备用API endpoints
        api_endpoints.insert("python_pypi".to_string(), "https://pypi.org/pypi/python/json".to_string());
        api_endpoints.insert("node_dist".to_string(), "https://nodejs.org/dist/index.json".to_string());
        
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            cache_ttl: Duration::from_secs(3600), // 1小时
            user_agent: "Grape-MCP-DevTools/2.0 (Enhanced Collector)".to_string(),
            api_endpoints,
        }
    }
}

impl EnhancedLanguageCollector {
    pub fn new(language: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self {
            client,
            language,
            config: CollectorConfig::default(),
        }
    }
    
    pub fn with_config(mut self, config: CollectorConfig) -> Self {
        self.config = config;
        self
    }
    
    /// 带重试的HTTP请求
    async fn fetch_with_retry(&self, url: &str) -> Result<Value> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            debug!("尝试获取数据 (第{}次): {}", attempt, url);
            
            match self.fetch_json(url).await {
                Ok(data) => {
                    debug!("成功获取数据: {} (第{}次尝试)", url, attempt);
                    return Ok(data);
                }
                Err(e) => {
                    warn!("获取数据失败 (第{}次尝试): {} - {}", attempt, url, e);
                    last_error = Some(e);
                    
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
    
    /// 基础HTTP请求
    async fn fetch_json(&self, url: &str) -> Result<Value> {
        let response = self.client
            .get(url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", &self.config.user_agent)
            .timeout(self.config.timeout)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP请求失败: {} - {}", response.status(), url));
        }
        
        let data: Value = response.json().await?;
        Ok(data)
    }
    
    /// 获取版本列表（支持多种数据源）
    async fn fetch_versions_multi_source(&self) -> Result<Vec<String>> {
        match self.language.as_str() {
            "python" => self.fetch_python_versions().await,
            "rust" => self.fetch_rust_versions().await,
            "javascript" | "node" => self.fetch_node_versions().await,
            "java" => self.fetch_java_versions().await,
            "go" => self.fetch_go_versions().await,
            "csharp" => self.fetch_csharp_versions().await,
            _ => self.fetch_generic_versions().await,
        }
    }
    
    /// Python版本获取（多数据源）
    async fn fetch_python_versions(&self) -> Result<Vec<String>> {
        // 首先尝试GitHub tags API
        if let Ok(versions) = self.fetch_python_from_github().await {
            if !versions.is_empty() {
                return Ok(versions);
            }
        }
        
        // 备用：尝试Python官方API
        if let Ok(versions) = self.fetch_python_from_official().await {
            if !versions.is_empty() {
                return Ok(versions);
            }
        }
        
        // 最后备用：使用动态获取的最新版本信息，而不是硬编码
        warn!("⚠️ 所有API都失败，尝试从备用源获取版本信息: {}", self.language);
        
        // 尝试通过不同的备用API获取版本
        let backup_versions = self.fetch_backup_versions().await.unwrap_or_else(|_| {
            warn!("⚠️ 备用版本获取也失败，使用最小配置");
            // 只返回一个通用的latest标记，让后续处理决定具体版本
            vec!["latest".to_string()]
        });
        
        if backup_versions.is_empty() || backup_versions == vec!["latest".to_string()] {
            // 如果真的无法获取任何版本信息，记录错误并返回空列表
            error!("❌ 无法为语言 {} 获取任何版本信息，请检查网络连接或API配置", self.language);
            return Err(anyhow::anyhow!("语言 {} 的所有版本获取方法都失败了", self.language));
        }
        
        Ok(backup_versions)
    }
    
    async fn fetch_python_from_github(&self) -> Result<Vec<String>> {
        let url = self.config.api_endpoints.get("python")
            .ok_or_else(|| anyhow::anyhow!("未配置Python API端点"))?;
            
        let data = self.fetch_with_retry(url).await?;
        let mut versions = Vec::new();
        
        if let Some(tags) = data.as_array() {
            for tag in tags.iter().take(50) { // 限制数量
                if let Some(name) = tag["name"].as_str() {
                    // 过滤Python版本标签
                    if name.starts_with("v3.") || name.starts_with("v2.") {
                        let version = name.trim_start_matches('v');
                        // 只包含稳定版本（不包含alpha, beta, rc）
                        if !version.contains("a") && !version.contains("b") && !version.contains("rc") {
                            versions.push(version.to_string());
                        }
                    }
                }
            }
        }
        
        info!("从GitHub获取到 {} 个Python版本", versions.len());
        Ok(versions)
    }
    
    async fn fetch_python_from_official(&self) -> Result<Vec<String>> {
        // 使用Python.org的官方API
        let urls_to_try = vec![
            "https://api.github.com/repos/python/cpython/releases?per_page=20",
            "https://endoflife.date/api/python.json",
            "https://pypi.org/pypi/python/json", // 这个可能不存在，但可以尝试
        ];
        
        for url in urls_to_try {
            match self.fetch_with_retry(url).await {
                Ok(data) => {
                    let mut versions = Vec::new();
                    
                    if url.contains("github.com") {
                        // GitHub releases格式
                        if let Some(releases) = data.as_array() {
                            for release in releases.iter() {
                                if let Some(tag_name) = release["tag_name"].as_str() {
                                    let version = tag_name.trim_start_matches('v');
                                    // 只包含稳定版本
                                    if version.starts_with("3.") && 
                                       !version.contains("a") && 
                                       !version.contains("b") && 
                                       !version.contains("rc") {
                                        versions.push(version.to_string());
                                    }
                                }
                            }
                        }
                    } else if url.contains("endoflife.date") {
                        // End of Life API格式
                        if let Some(cycles) = data.as_array() {
                            for cycle in cycles.iter() {
                                if let Some(version) = cycle["cycle"].as_str() {
                                    if version.starts_with("3.") {
                                        versions.push(version.to_string());
                                    }
                                }
                            }
                        }
                    }
                    
                    if !versions.is_empty() {
                        info!("✅ 从Python官方源获取到 {} 个版本", versions.len());
                        return Ok(versions);
                    }
                }
                Err(e) => {
                    debug!("⚠️ Python官方API {} 失败: {}", url, e);
                    continue;
                }
            }
        }
        
        // 如果所有官方源都失败
        warn!("⚠️ 所有Python官方API都失败");
        Ok(vec![])
    }
    
    /// Rust版本获取
    async fn fetch_rust_versions(&self) -> Result<Vec<String>> {
        let url = self.config.api_endpoints.get("rust")
            .ok_or_else(|| anyhow::anyhow!("未配置Rust API端点"))?;
            
        let data = self.fetch_with_retry(url).await?;
        let mut versions = Vec::new();
        
        if let Some(releases) = data.as_array() {
            for release in releases.iter().take(30) {
                if let Some(tag_name) = release["tag_name"].as_str() {
                    let version = tag_name.trim_start_matches('v');
                    versions.push(version.to_string());
                }
            }
        }
        
        info!("从GitHub获取到 {} 个Rust版本", versions.len());
        Ok(versions)
    }
    
    /// Node.js版本获取
    async fn fetch_node_versions(&self) -> Result<Vec<String>> {
        // 首先尝试GitHub releases
        if let Ok(versions) = self.fetch_node_from_github().await {
            if !versions.is_empty() {
                return Ok(versions);
            }
        }
        
        // 备用：Node.js官方分发API
        self.fetch_node_from_dist().await
    }
    
    async fn fetch_node_from_github(&self) -> Result<Vec<String>> {
        let url = self.config.api_endpoints.get("javascript")
            .ok_or_else(|| anyhow::anyhow!("未配置JavaScript API端点"))?;
            
        let data = self.fetch_with_retry(url).await?;
        let mut versions = Vec::new();
        
        if let Some(releases) = data.as_array() {
            for release in releases.iter().take(20) {
                if let Some(tag_name) = release["tag_name"].as_str() {
                    let version = tag_name.trim_start_matches('v');
                    versions.push(version.to_string());
                }
            }
        }
        
        Ok(versions)
    }
    
    async fn fetch_node_from_dist(&self) -> Result<Vec<String>> {
        let url = self.config.api_endpoints.get("node_dist")
            .ok_or_else(|| anyhow::anyhow!("未配置Node.js分发API端点"))?;
            
        let data = self.fetch_with_retry(url).await?;
        let mut versions = Vec::new();
        
        if let Some(releases) = data.as_array() {
            for release in releases.iter().take(20) {
                if let Some(version) = release["version"].as_str() {
                    let version = version.trim_start_matches('v');
                    versions.push(version.to_string());
                }
            }
        }
        
        Ok(versions)
    }
    
    /// Java版本获取
    async fn fetch_java_versions(&self) -> Result<Vec<String>> {
        let url = self.config.api_endpoints.get("java")
            .ok_or_else(|| anyhow::anyhow!("未配置Java API端点"))?;
            
        let data = self.fetch_with_retry(url).await?;
        let mut versions = Vec::new();
        
        if let Some(tags) = data.as_array() {
            for tag in tags.iter().take(30) {
                if let Some(name) = tag["name"].as_str() {
                    // 过滤JDK版本标签
                    if name.starts_with("jdk-") {
                        let version = name.trim_start_matches("jdk-");
                        versions.push(version.to_string());
                    }
                }
            }
        }
        
        info!("从GitHub获取到 {} 个Java版本", versions.len());
        Ok(versions)
    }
    
    /// Go版本获取
    async fn fetch_go_versions(&self) -> Result<Vec<String>> {
        let url = self.config.api_endpoints.get("go")
            .ok_or_else(|| anyhow::anyhow!("未配置Go API端点"))?;
            
        let data = self.fetch_with_retry(url).await?;
        let mut versions = Vec::new();
        
        if let Some(tags) = data.as_array() {
            for tag in tags.iter().take(30) {
                if let Some(name) = tag["name"].as_str() {
                    // 过滤Go版本标签
                    if name.starts_with("go") && name.len() > 2 {
                        let version = name.trim_start_matches("go");
                        versions.push(version.to_string());
                    }
                }
            }
        }
        
        info!("从GitHub获取到 {} 个Go版本", versions.len());
        Ok(versions)
    }
    
    /// C#版本获取
    async fn fetch_csharp_versions(&self) -> Result<Vec<String>> {
        let url = self.config.api_endpoints.get("csharp")
            .ok_or_else(|| anyhow::anyhow!("未配置C# API端点"))?;
            
        let data = self.fetch_with_retry(url).await?;
        let mut versions = Vec::new();
        
        if let Some(releases) = data.as_array() {
            for release in releases.iter().take(20) {
                if let Some(tag_name) = release["tag_name"].as_str() {
                    let version = tag_name.trim_start_matches('v');
                    versions.push(version.to_string());
                }
            }
        }
        
        info!("从GitHub获取到 {} 个C#版本", versions.len());
        Ok(versions)
    }
    
    /// 通用版本获取
    async fn fetch_generic_versions(&self) -> Result<Vec<String>> {
        info!("尝试通用版本获取方法: {}", self.language);
        
        // 尝试从常见的版本API获取
        let potential_apis = vec![
            format!("https://api.github.com/repos/{}/releases", self.get_github_repo()),
            format!("https://registry.npmjs.org/{}", self.language),
            format!("https://pypi.org/pypi/{}/json", self.language),
        ];
        
        for api_url in potential_apis {
            match self.try_fetch_from_api(&api_url).await {
                Ok(versions) if !versions.is_empty() => {
                    info!("✅ 成功从 {} 获取到 {} 个版本", api_url, versions.len());
                    return Ok(versions);
                }
                Ok(_) => {
                    debug!("📭 API {} 返回空版本列表", api_url);
                }
                Err(e) => {
                    debug!("⚠️ API {} 调用失败: {}", api_url, e);
                }
            }
        }
        
        warn!("⚠️ 所有通用API都失败，返回基础版本列表: {}", self.language);
        // 使用备用版本获取方法，而不是硬编码
        self.fetch_backup_versions().await
    }
    
    /// 尝试从API获取版本
    async fn try_fetch_from_api(&self, api_url: &str) -> Result<Vec<String>> {
        let client = reqwest::Client::new();
        let response = client
            .get(api_url)
            .header("User-Agent", "Grape-MCP-DevTools/2.0")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP错误: {}", response.status()));
        }
        
        let data: serde_json::Value = response.json().await?;
        let mut versions = Vec::new();
        
        // 尝试不同的JSON结构解析版本
        if let Some(releases) = data.as_array() {
            // GitHub releases格式
            for release in releases.iter().take(10) {
                if let Some(tag_name) = release["tag_name"].as_str() {
                    let version = tag_name.trim_start_matches('v');
                    if !version.contains("alpha") && !version.contains("beta") && !version.contains("rc") {
                        versions.push(version.to_string());
                    }
                }
            }
        } else if let Some(versions_obj) = data["versions"].as_object() {
            // npm/pypi格式
            for version_key in versions_obj.keys().take(10) {
                if !version_key.contains("alpha") && !version_key.contains("beta") && !version_key.contains("rc") {
                    versions.push(version_key.clone());
                }
            }
        }
        
        Ok(versions)
    }
    
    /// 获取可能的GitHub仓库名
    fn get_github_repo(&self) -> String {
        match self.language.as_str() {
            "rust" => "rust-lang/rust".to_string(),
            "python" => "python/cpython".to_string(),
            "node" | "javascript" => "nodejs/node".to_string(),
            "go" => "golang/go".to_string(),
            "java" => "openjdk/jdk".to_string(),
            _ => format!("{}/{}", self.language, self.language),
        }
    }
    
    /// 解析版本详情
    async fn parse_version_details(&self, version: &str) -> Result<LanguageVersion> {
        match self.language.as_str() {
            "python" => self.parse_python_version_details(version).await,
            "rust" => self.parse_rust_version_details(version).await,
            "javascript" | "node" => self.parse_node_version_details(version).await,
            "java" => self.parse_java_version_details(version).await,
            "go" => self.parse_go_version_details(version).await,
            "csharp" => self.parse_csharp_version_details(version).await,
            _ => self.parse_generic_version_details(version).await,
        }
    }
    
    async fn parse_python_version_details(&self, version: &str) -> Result<LanguageVersion> {
        Ok(LanguageVersion {
            language: "python".to_string(),
            version: version.to_string(),
            release_date: Utc::now(), // 实际应该从API获取
            is_stable: !version.contains("a") && !version.contains("b") && !version.contains("rc"),
            is_lts: false,
            status: VersionStatus::Current,
            features: self.generate_sample_features("python", version),
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: Some(format!("https://docs.python.org/{}/whatsnew/{}.html", version, version)),
                download_url: Some(format!("https://www.python.org/downloads/release/python-{}/", version.replace('.', ""))),
                source_url: Some(format!("https://github.com/python/cpython/tree/v{}", version)),
                documentation_url: Some(format!("https://docs.python.org/{}/", version)),
                changelog_url: Some(format!("https://docs.python.org/{}/whatsnew/changelog.html", version)),
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    async fn parse_rust_version_details(&self, version: &str) -> Result<LanguageVersion> {
        Ok(LanguageVersion {
            language: "rust".to_string(),
            version: version.to_string(),
            release_date: Utc::now(),
            is_stable: true,
            is_lts: false,
            status: VersionStatus::Current,
            features: self.generate_sample_features("rust", version),
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: Some(format!("https://github.com/rust-lang/rust/releases/tag/{}", version)),
                download_url: Some(format!("https://forge.rust-lang.org/infra/channel-releases.html#{}", version)),
                source_url: Some(format!("https://github.com/rust-lang/rust/tree/{}", version)),
                documentation_url: Some(format!("https://doc.rust-lang.org/{}/", version)),
                changelog_url: Some(format!("https://github.com/rust-lang/rust/blob/master/RELEASES.md#{}", version.replace('.', ""))),
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    async fn parse_node_version_details(&self, version: &str) -> Result<LanguageVersion> {
        Ok(LanguageVersion {
            language: "javascript".to_string(),
            version: version.to_string(),
            release_date: Utc::now(),
            is_stable: true,
            is_lts: version.contains("lts") || version.ends_with(".0"),
            status: VersionStatus::Current,
            features: self.generate_sample_features("javascript", version),
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: Some(format!("https://nodejs.org/en/blog/release/v{}/", version)),
                download_url: Some(format!("https://nodejs.org/dist/v{}/", version)),
                source_url: Some(format!("https://github.com/nodejs/node/tree/v{}", version)),
                documentation_url: Some(format!("https://nodejs.org/docs/v{}/api/", version)),
                changelog_url: Some(format!("https://github.com/nodejs/node/blob/v{}/CHANGELOG.md", version)),
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    async fn parse_java_version_details(&self, version: &str) -> Result<LanguageVersion> {
        Ok(LanguageVersion {
            language: "java".to_string(),
            version: version.to_string(),
            release_date: Utc::now(),
            is_stable: true,
            is_lts: version.starts_with("11") || version.starts_with("17") || version.starts_with("21"),
            status: VersionStatus::Current,
            features: self.generate_sample_features("java", version),
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: Some(format!("https://openjdk.org/projects/jdk/{}/", version)),
                download_url: Some(format!("https://jdk.java.net/{}/", version)),
                source_url: Some(format!("https://github.com/openjdk/jdk/tree/jdk-{}", version)),
                documentation_url: Some(format!("https://docs.oracle.com/en/java/javase/{}/", version)),
                changelog_url: None,
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    async fn parse_go_version_details(&self, version: &str) -> Result<LanguageVersion> {
        Ok(LanguageVersion {
            language: "go".to_string(),
            version: version.to_string(),
            release_date: Utc::now(),
            is_stable: true,
            is_lts: false,
            status: VersionStatus::Current,
            features: self.generate_sample_features("go", version),
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: Some(format!("https://golang.org/doc/go{}", version)),
                download_url: Some(format!("https://golang.org/dl/#go{}", version)),
                source_url: Some(format!("https://github.com/golang/go/tree/go{}", version)),
                documentation_url: Some(format!("https://golang.org/doc/")),
                changelog_url: Some(format!("https://golang.org/doc/go{}", version)),
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    async fn parse_csharp_version_details(&self, version: &str) -> Result<LanguageVersion> {
        Ok(LanguageVersion {
            language: "csharp".to_string(),
            version: version.to_string(),
            release_date: Utc::now(),
            is_stable: true,
            is_lts: version.starts_with("6.") || version.starts_with("8."),
            status: VersionStatus::Current,
            features: self.generate_sample_features("csharp", version),
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: Some(format!("https://docs.microsoft.com/en-us/dotnet/core/releases/{}", version)),
                download_url: Some(format!("https://dotnet.microsoft.com/download/dotnet/{}", version)),
                source_url: Some(format!("https://github.com/dotnet/core/tree/v{}", version)),
                documentation_url: Some(format!("https://docs.microsoft.com/en-us/dotnet/")),
                changelog_url: None,
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    async fn parse_generic_version_details(&self, version: &str) -> Result<LanguageVersion> {
        Ok(LanguageVersion {
            language: self.language.clone(),
            version: version.to_string(),
            release_date: Utc::now(),
            is_stable: true,
            is_lts: false,
            status: VersionStatus::Current,
            features: self.generate_sample_features(&self.language, version),
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: None,
                download_url: None,
                source_url: None,
                documentation_url: None,
                changelog_url: None,
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }
    
    /// 生成示例特性（实际应该从真实数据解析）
    fn generate_sample_features(&self, language: &str, version: &str) -> Vec<LanguageFeature> {
        match language {
            "python" => vec![
                LanguageFeature {
                    name: format!("Python {} 新特性", version),
                    description: format!("Python {} 版本的主要改进和新功能", version),
                    category: FeatureCategory::StandardLibrary,
                    examples: vec![],
                    proposal_link: None,
                    documentation_link: None,
                    stability: FeatureStability::Stable,
                    tags: vec!["python".to_string(), version.to_string()],
                    impact: ImpactLevel::Medium,
                },
            ],
            "rust" => vec![
                LanguageFeature {
                    name: format!("Rust {} 稳定化特性", version),
                    description: format!("Rust {} 版本稳定化的语言特性", version),
                    category: FeatureCategory::Syntax,
                    examples: vec![],
                    proposal_link: None,
                    documentation_link: None,
                    stability: FeatureStability::Stable,
                    tags: vec!["rust".to_string(), version.to_string()],
                    impact: ImpactLevel::Medium,
                },
            ],
            _ => vec![],
        }
    }
    
    /// 备用版本获取方法 - 尝试从镜像站点或缓存中获取
    async fn fetch_backup_versions(&self) -> Result<Vec<String>> {
        info!("🔄 尝试备用版本源获取: {}", self.language);
        
        // 尝试不同的备用源
        let backup_sources = match self.language.as_str() {
            "python" => vec![
                "https://endoflife.date/api/python.json",
                "https://api.github.com/repos/python/cpython/tags?per_page=10",
            ],
            "rust" => vec![
                "https://forge.rust-lang.org/infra/channel-releases.html", // 需要HTML解析
                "https://api.github.com/repos/rust-lang/rust/releases?per_page=10",
            ],
            "javascript" | "node" => vec![
                "https://nodejs.org/dist/index.json",
                "https://endoflife.date/api/nodejs.json",
            ],
            "java" => vec![
                "https://endoflife.date/api/java.json",
                "https://api.adoptium.net/v3/info/available_releases",
            ],
            "go" => vec![
                "https://go.dev/dl/?mode=json",
                "https://api.github.com/repos/golang/go/tags?per_page=10",
            ],
            _ => vec![], // 对于不支持的语言，返回空列表
        };
        
        // 尝试每个备用源
        for source_url in backup_sources {
            match self.try_fetch_from_backup_source(source_url).await {
                Ok(versions) if !versions.is_empty() => {
                    info!("✅ 成功从备用源获取 {} 个版本: {}", versions.len(), source_url);
                    return Ok(versions);
                }
                Ok(_) => {
                    debug!("📭 备用源 {} 返回空版本列表", source_url);
                }
                Err(e) => {
                    debug!("❌ 备用源 {} 失败: {}", source_url, e);
                }
            }
        }
        
        // 如果所有备用源都失败，返回错误而不是硬编码
        Err(anyhow::anyhow!("所有备用版本源都失败，语言: {}", self.language))
    }
    
    /// 从备用源获取版本
    async fn try_fetch_from_backup_source(&self, source_url: &str) -> Result<Vec<String>> {
        let response = self.client
            .get(source_url)
            .header("User-Agent", &self.config.user_agent)
            .timeout(self.config.timeout)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP错误: {}", response.status()));
        }
        
        let data: Value = response.json().await?;
        let mut versions = Vec::new();
        
        // 解析不同的API响应格式
        if source_url.contains("endoflife.date") {
            // End of Life API格式
            if let Some(releases) = data.as_array() {
                for release in releases.iter().take(10) {
                    if let Some(cycle) = release["cycle"].as_str() {
                        versions.push(cycle.to_string());
                    }
                }
            }
        } else if source_url.contains("nodejs.org/dist") {
            // Node.js 官方分发格式
            if let Some(releases) = data.as_array() {
                for release in releases.iter().take(10) {
                    if let Some(version) = release["version"].as_str() {
                        let clean_version = version.trim_start_matches('v');
                        if release["lts"].as_bool().unwrap_or(false) || versions.len() < 5 {
                            versions.push(clean_version.to_string());
                        }
                    }
                }
            }
        } else if source_url.contains("adoptium.net") {
            // Adoptium JDK API格式
            if let Some(available_releases) = data.get("available_releases") {
                if let Some(releases) = available_releases.as_array() {
                    for release in releases.iter().take(10) {
                        if let Some(version) = release.as_u64() {
                            versions.push(version.to_string());
                        }
                    }
                }
            }
        } else if source_url.contains("go.dev/dl") {
            // Go 官方下载API格式
            if let Some(releases) = data.as_array() {
                for release in releases.iter().take(10) {
                    if let Some(version) = release["version"].as_str() {
                        let clean_version = version.trim_start_matches("go");
                        if release["stable"].as_bool().unwrap_or(true) {
                            versions.push(clean_version.to_string());
                        }
                    }
                }
            }
        } else {
            // 默认尝试GitHub releases格式
            if let Some(releases) = data.as_array() {
                for release in releases.iter().take(10) {
                    if let Some(tag_name) = release["tag_name"].as_str() {
                        let version = tag_name.trim_start_matches('v');
                        if !version.contains("alpha") && !version.contains("beta") && !version.contains("rc") {
                            versions.push(version.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(versions)
    }
}

#[async_trait]
impl LanguageVersionCollector for EnhancedLanguageCollector {
    fn language(&self) -> &str {
        &self.language
    }
    
    async fn get_versions(&self) -> Result<Vec<String>> {
        self.fetch_versions_multi_source().await
    }
    
    async fn get_version_details(&self, version: &str) -> Result<LanguageVersion> {
        self.parse_version_details(version).await
    }
    
    async fn get_latest_version(&self) -> Result<LanguageVersion> {
        let versions = self.get_versions().await?;
        
        if let Some(latest_version) = versions.first() {
            self.get_version_details(latest_version).await
        } else {
            Err(anyhow::anyhow!("无法获取最新{}版本", self.language))
        }
    }
    
    async fn is_version_supported(&self, version: &str) -> bool {
        match self.get_versions().await {
            Ok(versions) => versions.contains(&version.to_string()),
            Err(_) => false,
        }
    }
}

/// 增强的采集器工厂
pub struct EnhancedCollectorFactory;

impl EnhancedCollectorFactory {
    pub fn create_collector(language: &str) -> Result<Box<dyn LanguageVersionCollector>> {
        let collector = EnhancedLanguageCollector::new(language.to_string());
        Ok(Box::new(collector))
    }
    
    pub fn supported_languages() -> Vec<&'static str> {
        vec!["rust", "python", "javascript", "java", "go", "csharp", "cpp", "php", "ruby", "swift"]
    }
    
    pub fn create_with_config(language: &str, config: CollectorConfig) -> Result<Box<dyn LanguageVersionCollector>> {
        let collector = EnhancedLanguageCollector::new(language.to_string()).with_config(config);
        Ok(Box::new(collector))
    }
} 