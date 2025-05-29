use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use chrono::{DateTime, Utc};

use super::data_models::*;
use super::intelligent_scraper::IntelligentScraper;
use super::content_analyzer::ChangelogAnalyzer;
use super::url_discovery::URLDiscoveryEngine;

/// AI驱动的采集引擎配置
#[derive(Debug, Clone)]
pub struct AICollectorConfig {
    /// OpenAI API密钥
    pub openai_api_key: Option<String>,
    /// 最大并发请求数
    pub max_concurrent_requests: usize,
    /// 请求超时时间（秒）
    pub request_timeout_secs: u64,
    /// 是否启用JavaScript渲染
    pub enable_js_rendering: bool,
    /// 缓存TTL（秒）
    pub cache_ttl_secs: u64,
    /// AI分析置信度阈值
    pub ai_confidence_threshold: f32,
}

impl Default for AICollectorConfig {
    fn default() -> Self {
        Self {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            max_concurrent_requests: 10,
            request_timeout_secs: 30,
            enable_js_rendering: true,
            cache_ttl_secs: 3600, // 1小时
            ai_confidence_threshold: 0.7,
        }
    }
}

/// AI驱动的语言版本采集引擎
pub struct AICollectorEngine {
    config: AICollectorConfig,
    http_client: Client,
    scraper: Arc<IntelligentScraper>,
    analyzer: Arc<ChangelogAnalyzer>,
    _url_discovery: Arc<URLDiscoveryEngine>,
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    language_sources: HashMap<String, LanguageSourceConfig>,
}

/// 缓存结果
#[derive(Debug, Clone)]
struct CachedResult {
    data: Value,
    timestamp: DateTime<Utc>,
    confidence: f32,
}

/// 语言数据源配置
#[derive(Debug, Clone)]
pub struct LanguageSourceConfig {
    pub language: String,
    pub primary_sources: Vec<SourceEndpoint>,
    pub fallback_sources: Vec<SourceEndpoint>,
    pub changelog_patterns: Vec<String>,
    pub release_patterns: Vec<String>,
    pub official_docs: Vec<String>,
}

/// 数据源端点
#[derive(Debug, Clone)]
pub struct SourceEndpoint {
    pub name: String,
    pub base_url: String,
    pub api_type: APIType,
    pub requires_auth: bool,
    pub rate_limit: Option<u32>,
    pub changelog_selectors: Vec<String>,
}

/// API类型
#[derive(Debug, Clone)]
pub enum APIType {
    GitHub,
    REST,
    GraphQL,
    RSS,
    WebPage,
    Documentation,
}

impl AICollectorEngine {
    pub async fn new(config: AICollectorConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
            .build()?;

        let scraper = Arc::new(IntelligentScraper::new(
            http_client.clone(),
            config.enable_js_rendering,
        ).await?);

        let analyzer = Arc::new(ChangelogAnalyzer::new(
            config.openai_api_key.clone(),
        ).await?);

        let url_discovery = Arc::new(URLDiscoveryEngine::new(
            http_client.clone(),
        ).await?);

        let mut engine = Self {
            config,
            http_client,
            scraper,
            analyzer,
            _url_discovery: url_discovery,
            cache: Arc::new(RwLock::new(HashMap::new())),
            language_sources: HashMap::new(),
        };

        // 初始化所有支持语言的数据源配置
        engine.initialize_language_sources().await?;

        Ok(engine)
    }

    /// 初始化所有语言的数据源配置
    async fn initialize_language_sources(&mut self) -> Result<()> {
        info!("🔧 初始化语言数据源配置...");

        // Rust
        self.language_sources.insert("rust".to_string(), LanguageSourceConfig {
            language: "rust".to_string(),
            primary_sources: vec![
                SourceEndpoint {
                    name: "GitHub Releases".to_string(),
                    base_url: "https://api.github.com/repos/rust-lang/rust".to_string(),
                    api_type: APIType::GitHub,
                    requires_auth: false,
                    rate_limit: Some(5000),
                    changelog_selectors: vec!["releases".to_string()],
                },
                SourceEndpoint {
                    name: "Rust Blog".to_string(),
                    base_url: "https://blog.rust-lang.org".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec![".post-content".to_string(), "article".to_string()],
                },
            ],
            fallback_sources: vec![
                SourceEndpoint {
                    name: "RELEASES.md".to_string(),
                    base_url: "https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec!["body".to_string()],
                },
            ],
            changelog_patterns: vec![
                "Version \\d+\\.\\d+\\.\\d+".to_string(),
                "# \\d+\\.\\d+\\.\\d+".to_string(),
            ],
            release_patterns: vec![
                "v\\d+\\.\\d+\\.\\d+".to_string(),
            ],
            official_docs: vec![
                "https://doc.rust-lang.org/".to_string(),
                "https://forge.rust-lang.org/".to_string(),
            ],
        });

        // Python
        self.language_sources.insert("python".to_string(), LanguageSourceConfig {
            language: "python".to_string(),
            primary_sources: vec![
                SourceEndpoint {
                    name: "Python.org News".to_string(),
                    base_url: "https://www.python.org/downloads/".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec![".download-list-widget".to_string(), ".release-version".to_string()],
                },
                SourceEndpoint {
                    name: "GitHub Releases".to_string(),
                    base_url: "https://api.github.com/repos/python/cpython".to_string(),
                    api_type: APIType::GitHub,
                    requires_auth: false,
                    rate_limit: Some(5000),
                    changelog_selectors: vec!["releases".to_string()],
                },
            ],
            fallback_sources: vec![
                SourceEndpoint {
                    name: "What's New".to_string(),
                    base_url: "https://docs.python.org/3/whatsnew/".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec!["div.body".to_string()],
                },
            ],
            changelog_patterns: vec![
                "Python \\d+\\.\\d+".to_string(),
                "# Python \\d+\\.\\d+".to_string(),
            ],
            release_patterns: vec![
                "v\\d+\\.\\d+\\.\\d+".to_string(),
            ],
            official_docs: vec![
                "https://docs.python.org/3/".to_string(),
                "https://peps.python.org/".to_string(),
            ],
        });

        // JavaScript/Node.js
        self.language_sources.insert("javascript".to_string(), LanguageSourceConfig {
            language: "javascript".to_string(),
            primary_sources: vec![
                SourceEndpoint {
                    name: "Node.js Releases".to_string(),
                    base_url: "https://api.github.com/repos/nodejs/node".to_string(),
                    api_type: APIType::GitHub,
                    requires_auth: false,
                    rate_limit: Some(5000),
                    changelog_selectors: vec!["releases".to_string()],
                },
                SourceEndpoint {
                    name: "MDN JavaScript".to_string(),
                    base_url: "https://developer.mozilla.org/en-US/docs/Web/JavaScript/New_in_JavaScript".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec!["article".to_string()],
                },
            ],
            fallback_sources: vec![
                SourceEndpoint {
                    name: "ECMAScript Spec".to_string(),
                    base_url: "https://tc39.es/ecma262/".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec!["body".to_string()],
                },
            ],
            changelog_patterns: vec![
                "ES\\d+".to_string(),
                "ECMAScript \\d+".to_string(),
                "Node\\.js v\\d+".to_string(),
            ],
            release_patterns: vec![
                "v\\d+\\.\\d+\\.\\d+".to_string(),
            ],
            official_docs: vec![
                "https://nodejs.org/en/docs/".to_string(),
                "https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string(),
            ],
        });

        // Java
        self.language_sources.insert("java".to_string(), LanguageSourceConfig {
            language: "java".to_string(),
            primary_sources: vec![
                SourceEndpoint {
                    name: "OpenJDK Updates".to_string(),
                    base_url: "https://openjdk.org/projects/jdk/".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec![".main-content".to_string()],
                },
                SourceEndpoint {
                    name: "Oracle Java Archive".to_string(),
                    base_url: "https://www.oracle.com/java/technologies/java-archive.html".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec![".cmp-wrapper".to_string()],
                },
            ],
            fallback_sources: vec![
                SourceEndpoint {
                    name: "JEP Index".to_string(),
                    base_url: "https://openjdk.org/jeps/".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec!["table".to_string()],
                },
            ],
            changelog_patterns: vec![
                "Java \\d+".to_string(),
                "JDK \\d+".to_string(),
            ],
            release_patterns: vec![
                "jdk-\\d+".to_string(),
            ],
            official_docs: vec![
                "https://docs.oracle.com/en/java/".to_string(),
                "https://openjdk.org/".to_string(),
            ],
        });

        // Go
        self.language_sources.insert("go".to_string(), LanguageSourceConfig {
            language: "go".to_string(),
            primary_sources: vec![
                SourceEndpoint {
                    name: "Go Releases".to_string(),
                    base_url: "https://api.github.com/repos/golang/go".to_string(),
                    api_type: APIType::GitHub,
                    requires_auth: false,
                    rate_limit: Some(5000),
                    changelog_selectors: vec!["releases".to_string()],
                },
                SourceEndpoint {
                    name: "Go Blog".to_string(),
                    base_url: "https://go.dev/blog/".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec!["article".to_string()],
                },
            ],
            fallback_sources: vec![
                SourceEndpoint {
                    name: "Release Notes".to_string(),
                    base_url: "https://go.dev/doc/devel/release".to_string(),
                    api_type: APIType::WebPage,
                    requires_auth: false,
                    rate_limit: None,
                    changelog_selectors: vec!["div#content".to_string()],
                },
            ],
            changelog_patterns: vec![
                "Go \\d+\\.\\d+".to_string(),
                "go\\d+\\.\\d+".to_string(),
            ],
            release_patterns: vec![
                "go\\d+\\.\\d+\\.\\d+".to_string(),
            ],
            official_docs: vec![
                "https://go.dev/doc/".to_string(),
                "https://pkg.go.dev/".to_string(),
            ],
        });

        info!("✅ 初始化了 {} 种语言的数据源配置", self.language_sources.len());
        Ok(())
    }

    /// 获取支持的语言列表
    pub fn get_supported_languages(&self) -> Vec<String> {
        self.language_sources.keys().cloned().collect()
    }

    /// AI驱动的版本采集
    pub async fn collect_language_versions(&self, language: &str) -> Result<Vec<LanguageVersion>> {
        info!("🤖 开始AI驱动采集: {}", language);

        // 检查缓存
        let cache_key = format!("versions:{}", language);
        if let Some(cached) = self.get_cached_result(&cache_key).await {
            if cached.confidence >= self.config.ai_confidence_threshold {
                info!("🎯 使用高置信度缓存结果");
                return self.parse_cached_versions(cached.data).await;
            }
        }

        // 获取语言源配置
        let source_config = self.language_sources.get(language)
            .ok_or_else(|| anyhow::anyhow!("不支持的语言: {}", language))?;

        let mut all_versions = Vec::new();
        let mut collection_errors = Vec::new();

        // 尝试主要数据源
        for source in &source_config.primary_sources {
            match self.collect_from_source(source, language).await {
                Ok(mut versions) => {
                    info!("✅ 从 {} 获取到 {} 个版本", source.name, versions.len());
                    all_versions.append(&mut versions);
                }
                Err(e) => {
                    warn!("⚠️ 主要数据源失败 {}: {}", source.name, e);
                    collection_errors.push(format!("{}: {}", source.name, e));
                }
            }
        }

        // 如果主要数据源失败，尝试备用数据源
        if all_versions.is_empty() {
            warn!("🔄 尝试备用数据源...");
            for source in &source_config.fallback_sources {
                match self.collect_from_source(source, language).await {
                    Ok(mut versions) => {
                        info!("✅ 从备用源 {} 获取到 {} 个版本", source.name, versions.len());
                        all_versions.append(&mut versions);
                        break; // 成功获取一个备用源即可
                    }
                    Err(e) => {
                        warn!("⚠️ 备用数据源失败 {}: {}", source.name, e);
                        collection_errors.push(format!("fallback-{}: {}", source.name, e));
                    }
                }
            }
        }

        if all_versions.is_empty() {
            return Err(anyhow::anyhow!(
                "所有数据源都失败了: {}",
                collection_errors.join(", ")
            ));
        }

        // 去重和排序
        self.deduplicate_and_sort_versions(&mut all_versions);

        // 缓存结果
        self.cache_result(&cache_key, json!(all_versions), 0.9).await;

        info!("🎉 成功采集到 {} 个 {} 版本", all_versions.len(), language);
        Ok(all_versions)
    }

    /// 从特定数据源采集版本信息
    async fn collect_from_source(&self, source: &SourceEndpoint, language: &str) -> Result<Vec<LanguageVersion>> {
        debug!("🔍 从数据源采集: {} ({})", source.name, source.base_url);

        match source.api_type {
            APIType::GitHub => self.collect_from_github(source, language).await,
            APIType::WebPage => self.collect_from_webpage(source, language).await,
            APIType::REST => self.collect_from_rest_api(source, language).await,
            APIType::RSS => self.collect_from_rss(source, language).await,
            APIType::GraphQL => self.collect_from_graphql(source, language).await,
            APIType::Documentation => self.collect_from_documentation(source, language).await,
        }
    }

    /// 从GitHub API采集
    async fn collect_from_github(&self, source: &SourceEndpoint, language: &str) -> Result<Vec<LanguageVersion>> {
        let releases_url = format!("{}/releases", source.base_url);
        let response = self.http_client.get(&releases_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Grape-MCP-DevTools/2.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API请求失败: {}", response.status()));
        }

        let releases: Vec<Value> = response.json().await?;
        let mut versions = Vec::new();

        for release in releases {
            if let Ok(version) = self.parse_github_release(&release, language).await {
                versions.push(version);
            }
        }

        Ok(versions)
    }

    /// 从网页采集
    async fn collect_from_webpage(&self, source: &SourceEndpoint, language: &str) -> Result<Vec<LanguageVersion>> {
        // 使用智能爬虫获取内容
        let content = self.scraper.scrape_intelligent(&source.base_url, &source.changelog_selectors).await?;
        
        // 使用AI分析器提取版本信息
        let analysis_result = self.analyzer.analyze_changelog_content(&content.content, language).await?;
        
        // 转换为LanguageVersion对象
        self.convert_analysis_to_versions(analysis_result, language).await
    }

    /// 从REST API采集
    async fn collect_from_rest_api(&self, source: &SourceEndpoint, language: &str) -> Result<Vec<LanguageVersion>> {
        info!("🌐 从REST API采集版本信息: {}", source.base_url);
        
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("User-Agent", "Grape-MCP-DevTools/2.0".parse().unwrap());
        
        // 如果需要认证
        if source.requires_auth {
            if let Ok(token) = std::env::var("API_TOKEN") {
                headers.insert("Authorization", format!("Bearer {}", token).parse().unwrap());
            }
        }
        
        let response = self.http_client
            .get(&source.base_url)
            .headers(headers)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("REST API请求失败: {}", response.status()));
        }
        
        let data: Value = response.json().await?;
        self.parse_rest_api_response(data, language).await
    }

    async fn collect_from_rss(&self, source: &SourceEndpoint, language: &str) -> Result<Vec<LanguageVersion>> {
        info!("📡 从RSS采集版本信息: {}", source.base_url);
        
        let response = self.http_client
            .get(&source.base_url)
            .header("Accept", "application/rss+xml, application/xml, text/xml")
            .header("User-Agent", "Grape-MCP-DevTools/2.0")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("RSS请求失败: {}", response.status()));
        }
        
        let rss_content = response.text().await?;
        self.parse_rss_content(&rss_content, language).await
    }

    async fn collect_from_graphql(&self, source: &SourceEndpoint, language: &str) -> Result<Vec<LanguageVersion>> {
        info!("🔮 从GraphQL采集版本信息: {}", source.base_url);
        
        // GraphQL查询示例（需要根据具体API调整）
        let query = json!({
            "query": "query { releases(first: 100) { nodes { tagName publishedAt description url } } }"
        });
        
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("User-Agent", "Grape-MCP-DevTools/2.0".parse().unwrap());
        
        // 如果需要认证
        if source.requires_auth {
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                headers.insert("Authorization", format!("Bearer {}", token).parse().unwrap());
            }
        }
        
        let response = self.http_client
            .post(&source.base_url)
            .headers(headers)
            .json(&query)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GraphQL请求失败: {}", response.status()));
        }
        
        let data: Value = response.json().await?;
        self.parse_graphql_response(data, language).await
    }

    async fn collect_from_documentation(&self, source: &SourceEndpoint, language: &str) -> Result<Vec<LanguageVersion>> {
        info!("📚 从文档站点采集版本信息: {}", source.base_url);
        
        // 使用智能爬虫获取内容
        let content = self.scraper.scrape_intelligent(&source.base_url, &source.changelog_selectors).await?;
        
        // 使用AI分析器提取版本信息
        let analysis_result = self.analyzer.analyze_changelog_content(&content.content, language).await?;
        
        // 转换为LanguageVersion对象
        self.convert_documentation_analysis_to_versions(analysis_result, language, &source.base_url).await
    }

    /// 解析GitHub release为LanguageVersion
    async fn parse_github_release(&self, release: &Value, language: &str) -> Result<LanguageVersion> {
        let tag_name = release["tag_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("无法获取版本标签"))?;
        let version = tag_name.trim_start_matches('v');
        
        let release_date = release["published_at"].as_str()
            .ok_or_else(|| anyhow::anyhow!("无法获取发布日期"))?;
        let release_date = DateTime::parse_from_rfc3339(release_date)?
            .with_timezone(&Utc);
            
        let is_prerelease = release["prerelease"].as_bool().unwrap_or(false);
        let body = release["body"].as_str().unwrap_or("");

        // 使用AI分析器解析release notes
        let changelog_analysis = if !body.is_empty() {
            self.analyzer.analyze_release_notes(body, language).await?
        } else {
            Default::default()
        };

        Ok(LanguageVersion {
            language: language.to_string(),
            version: version.to_string(),
            release_date,
            is_stable: !is_prerelease,
            is_lts: self.is_lts_version(language, version).await.unwrap_or(false),
            status: if !is_prerelease { VersionStatus::Current } else { VersionStatus::Preview },
            features: changelog_analysis.features,
            syntax_changes: changelog_analysis.syntax_changes,
            deprecations: changelog_analysis.deprecations,
            breaking_changes: changelog_analysis.breaking_changes,
            performance_improvements: changelog_analysis.performance_improvements,
            stdlib_changes: changelog_analysis.stdlib_changes,
            toolchain_changes: changelog_analysis.toolchain_changes,
            metadata: VersionMetadata {
                release_notes_url: release["html_url"].as_str().map(|s| s.to_string()),
                download_url: None,
                source_url: Some(format!("{}/tree/{}", 
                    release["html_url"].as_str().unwrap_or("").replace("/releases/tag/", ""), 
                    tag_name)),
                documentation_url: None,
                changelog_url: None,
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }

    /// 转换AI分析结果为版本列表
    async fn convert_analysis_to_versions(&self, analysis: Value, language: &str) -> Result<Vec<LanguageVersion>> {
        let mut versions = Vec::new();
        
        // 从AI分析结果中提取版本信息
        if let Some(versions_array) = analysis.get("versions").and_then(|v| v.as_array()) {
            for version_data in versions_array {
                if let Ok(version) = self.parse_version_from_analysis(version_data, language).await {
                    versions.push(version);
                }
            }
        }
        
        Ok(versions)
    }

    /// 去重和排序版本列表
    fn deduplicate_and_sort_versions(&self, versions: &mut Vec<LanguageVersion>) {
        // 去重
        let mut seen = std::collections::HashSet::new();
        versions.retain(|v| seen.insert(format!("{}:{}", v.language, v.version)));

        // 按发布日期倒序排序
        versions.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    }

    /// 获取缓存结果
    async fn get_cached_result(&self, key: &str) -> Option<CachedResult> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(key) {
            let age = Utc::now().signed_duration_since(cached.timestamp);
            if age.num_seconds() < self.config.cache_ttl_secs as i64 {
                return Some(cached.clone());
            }
        }
        None
    }

    /// 缓存结果
    async fn cache_result(&self, key: &str, data: Value, confidence: f32) {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), CachedResult {
            data,
            timestamp: Utc::now(),
            confidence,
        });
    }

    /// 解析缓存的版本数据
    async fn parse_cached_versions(&self, data: Value) -> Result<Vec<LanguageVersion>> {
        let mut versions = Vec::new();
        
        if let Some(versions_array) = data.get("versions").and_then(|v| v.as_array()) {
            for version_data in versions_array {
                if let Ok(version) = serde_json::from_value::<LanguageVersion>(version_data.clone()) {
                    versions.push(version);
                }
            }
        }
        
        Ok(versions)
    }

    /// 清除缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("🧹 清除AI采集器缓存");
    }

    /// 获取采集统计信息
    pub async fn get_collection_stats(&self) -> CollectionStats {
        let cache = self.cache.read().await;
        CollectionStats {
            supported_languages: self.language_sources.len(),
            cached_results: cache.len(),
            total_data_sources: self.language_sources.values()
                .map(|config| config.primary_sources.len() + config.fallback_sources.len())
                .sum(),
        }
    }

    /// 解析REST API响应
    async fn parse_rest_api_response(&self, data: Value, language: &str) -> Result<Vec<LanguageVersion>> {
        let mut versions = Vec::new();
        
        // 尝试不同的API响应格式
        if let Some(releases) = data.get("releases").and_then(|v| v.as_array()) {
            for release in releases {
                if let Ok(version) = self.parse_api_release(release, language).await {
                    versions.push(version);
                }
            }
        } else if let Some(versions_array) = data.as_array() {
            for version_data in versions_array {
                if let Ok(version) = self.parse_api_release(version_data, language).await {
                    versions.push(version);
                }
            }
        }
        
        Ok(versions)
    }

    /// 解析RSS内容
    async fn parse_rss_content(&self, content: &str, language: &str) -> Result<Vec<LanguageVersion>> {
        let mut versions = Vec::new();
        
        // 使用简单的XML解析提取RSS项目
        for line in content.lines() {
            if line.trim().starts_with("<title>") && line.contains("v") {
                if let Some(version_str) = self.extract_version_from_rss_title(line) {
                    let version = LanguageVersion {
                        language: language.to_string(),
                        version: version_str,
                        release_date: Utc::now(), // RSS通常需要更复杂的日期解析
                        is_stable: true,
                        is_lts: false,
                        status: VersionStatus::Current,
                        features: vec![],
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
                    };
                    versions.push(version);
                }
            }
        }
        
        Ok(versions)
    }

    /// 解析GraphQL响应
    async fn parse_graphql_response(&self, data: Value, language: &str) -> Result<Vec<LanguageVersion>> {
        let mut versions = Vec::new();
        
        if let Some(releases) = data.get("data")
            .and_then(|d| d.get("releases"))
            .and_then(|r| r.get("nodes"))
            .and_then(|n| n.as_array()) {
            
            for release in releases {
                if let Ok(version) = self.parse_graphql_release(release, language).await {
                    versions.push(version);
                }
            }
        }
        
        Ok(versions)
    }

    /// 转换文档分析结果为版本列表
    async fn convert_documentation_analysis_to_versions(&self, analysis: Value, language: &str, source_url: &str) -> Result<Vec<LanguageVersion>> {
        let mut versions = Vec::new();
        
        if let Some(features) = analysis.get("features").and_then(|f| f.as_array()) {
            // 从特性分析中提取版本信息
            for feature in features {
                if let Some(version_str) = feature.get("version").and_then(|v| v.as_str()) {
                    let version = LanguageVersion {
                        language: language.to_string(),
                        version: version_str.to_string(),
                        release_date: Utc::now(),
                        is_stable: true,
                        is_lts: false,
                        status: VersionStatus::Current,
                        features: vec![], // 可以从分析结果中提取
                        syntax_changes: vec![],
                        deprecations: vec![],
                        breaking_changes: vec![],
                        performance_improvements: vec![],
                        stdlib_changes: vec![],
                        toolchain_changes: vec![],
                        metadata: VersionMetadata {
                            release_notes_url: Some(source_url.to_string()),
                            download_url: None,
                            source_url: Some(source_url.to_string()),
                            documentation_url: Some(source_url.to_string()),
                            changelog_url: None,
                            upgrade_guide_url: None,
                            tags: HashMap::new(),
                        },
                    };
                    versions.push(version);
                }
            }
        }
        
        Ok(versions)
    }

    /// 从AI分析中解析版本
    async fn parse_version_from_analysis(&self, version_data: &Value, language: &str) -> Result<LanguageVersion> {
        let version_str = version_data.get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少版本字符串"))?;

        let release_date = version_data.get("release_date")
            .and_then(|d| d.as_str())
            .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());

        Ok(LanguageVersion {
            language: language.to_string(),
            version: version_str.to_string(),
            release_date,
            is_stable: version_data.get("is_stable").and_then(|s| s.as_bool()).unwrap_or(true),
            is_lts: version_data.get("is_lts").and_then(|l| l.as_bool()).unwrap_or(false),
            status: VersionStatus::Current,
            features: vec![],
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: version_data.get("release_notes_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                download_url: version_data.get("download_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                source_url: version_data.get("source_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                documentation_url: version_data.get("documentation_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                changelog_url: version_data.get("changelog_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                upgrade_guide_url: version_data.get("upgrade_guide_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                tags: HashMap::new(),
            },
        })
    }

    /// 解析API发布信息
    async fn parse_api_release(&self, release: &Value, language: &str) -> Result<LanguageVersion> {
        let version_str = release.get("tag_name")
            .or_else(|| release.get("version"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少版本信息"))?
            .trim_start_matches('v');

        let release_date = release.get("published_at")
            .or_else(|| release.get("created_at"))
            .and_then(|d| d.as_str())
            .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());

        Ok(LanguageVersion {
            language: language.to_string(),
            version: version_str.to_string(),
            release_date,
            is_stable: !release.get("prerelease").and_then(|p| p.as_bool()).unwrap_or(false),
            is_lts: false,
            status: VersionStatus::Current,
            features: vec![],
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: release.get("html_url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                download_url: None,
                source_url: None,
                documentation_url: None,
                changelog_url: None,
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }

    /// 解析GraphQL发布信息
    async fn parse_graphql_release(&self, release: &Value, language: &str) -> Result<LanguageVersion> {
        let version_str = release.get("tagName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少版本标签"))?
            .trim_start_matches('v');

        let release_date = release.get("publishedAt")
            .and_then(|d| d.as_str())
            .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());

        Ok(LanguageVersion {
            language: language.to_string(),
            version: version_str.to_string(),
            release_date,
            is_stable: true,
            is_lts: false,
            status: VersionStatus::Current,
            features: vec![],
            syntax_changes: vec![],
            deprecations: vec![],
            breaking_changes: vec![],
            performance_improvements: vec![],
            stdlib_changes: vec![],
            toolchain_changes: vec![],
            metadata: VersionMetadata {
                release_notes_url: release.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                download_url: None,
                source_url: None,
                documentation_url: None,
                changelog_url: None,
                upgrade_guide_url: None,
                tags: HashMap::new(),
            },
        })
    }

    /// 从RSS标题中提取版本
    fn extract_version_from_rss_title(&self, title_line: &str) -> Option<String> {
        // 简单的版本提取逻辑，可以改进
        if let Some(start) = title_line.find("v") {
            if let Some(end) = title_line[start..].find("</title>") {
                let version_part = &title_line[start..start + end];
                return Some(version_part.trim_start_matches('v').to_string());
            }
        }
        None
    }

    /// 判断是否为LTS版本
    async fn is_lts_version(&self, _language: &str, _version: &str) -> Result<bool> {
        // 实现判断LTS版本的逻辑
        // 这里可以根据语言和版本的特征来判断是否为LTS版本
        // 例如，可以根据版本号的格式、发布周期、官方声明等来判断
        // 这里只是一个示例，实际实现需要根据具体情况来决定
        Ok(false)
    }
}

/// 采集统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollectionStats {
    pub supported_languages: usize,
    pub cached_results: usize,
    pub total_data_sources: usize,
}

/// 变更日志分析结果
#[derive(Debug, Clone, Default)]
pub struct ChangelogAnalysisResult {
    pub features: Vec<LanguageFeature>,
    pub syntax_changes: Vec<SyntaxChange>,
    pub deprecations: Vec<Deprecation>,
    pub breaking_changes: Vec<BreakingChange>,
    pub performance_improvements: Vec<PerformanceImprovement>,
    pub stdlib_changes: Vec<StdlibChange>,
    pub toolchain_changes: Vec<ToolchainChange>,
} 