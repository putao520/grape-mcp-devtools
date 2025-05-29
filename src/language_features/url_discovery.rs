use anyhow::Result;
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use tracing::{info, warn, debug};
use url::Url;

use super::intelligent_scraper::{IntelligentScraper, ContentType};

/// URL发现引擎
pub struct URLDiscoveryEngine {
    http_client: Client,
    url_patterns: UrlPatterns,
    discovery_cache: HashMap<String, Vec<DiscoveredUrl>>,
}

/// URL模式定义
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct UrlPatterns {
    changelog_patterns: Vec<String>,
    release_patterns: Vec<String>,
    documentation_patterns: Vec<String>,
    version_patterns: Vec<String>,
}

/// 发现的URL信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredUrl {
    pub url: String,
    pub url_type: UrlType,
    pub confidence: f32,
    pub source: String,
    pub language: Option<String>,
    pub version: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: u8,
}

/// URL类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum UrlType {
    Changelog,
    ReleaseNotes,
    Documentation,
    ApiReference,
    BlogPost,
    GitHubRelease,
    VersionHistory,
    Unknown,
}

impl URLDiscoveryEngine {
    pub async fn new(http_client: Client) -> Result<Self> {
        Ok(Self {
            http_client,
            url_patterns: Self::init_url_patterns(),
            discovery_cache: HashMap::new(),
        })
    }

    /// 初始化URL模式
    fn init_url_patterns() -> UrlPatterns {
        UrlPatterns {
            changelog_patterns: vec![
                "CHANGELOG".to_string(),
                "CHANGES".to_string(),
                "HISTORY".to_string(),
                "changelog".to_string(),
                "changes".to_string(),
            ],
            release_patterns: vec![
                "releases".to_string(),
                "tags".to_string(),
                "versions".to_string(),
            ],
            documentation_patterns: vec![
                "docs".to_string(),
                "documentation".to_string(),
                "wiki".to_string(),
            ],
            version_patterns: vec![
                r"v?\d+\.\d+\.\d+".to_string(),
                r"\d+\.\d+".to_string(),
            ],
        }
    }

    /// 发现与语言相关的URL
    pub async fn discover_language_urls(&mut self, language: &str, base_urls: Vec<String>) -> Result<Vec<DiscoveredUrl>> {
        info!("🔍 开始发现 {} 相关的URL", language);

        let cache_key = format!("language:{}", language);
        if let Some(cached_urls) = self.discovery_cache.get(&cache_key) {
            info!("🎯 使用缓存的URL发现结果");
            return Ok(cached_urls.clone());
        }

        let mut discovered_urls = Vec::new();
        let mut visited_urls = HashSet::new();

        // 从基础URL开始发现
        for base_url in &base_urls {
            if visited_urls.contains(base_url) {
                continue;
            }

            match self.explore_base_url(base_url, language, &mut visited_urls).await {
                Ok(mut urls) => {
                    discovered_urls.append(&mut urls);
                }
                Err(e) => {
                    warn!("⚠️ 探索基础URL失败 {}: {}", base_url, e);
                }
            }
        }

        // 智能扩展发现
        let expanded_urls = self.intelligent_url_expansion(&discovered_urls, language).await?;
        discovered_urls.extend(expanded_urls);

        // 排序和去重
        self.deduplicate_and_sort_urls(&mut discovered_urls);

        // 缓存结果
        self.discovery_cache.insert(cache_key, discovered_urls.clone());

        info!("✅ 发现了 {} 个相关URL", discovered_urls.len());
        Ok(discovered_urls)
    }

    /// 探索基础URL
    async fn explore_base_url(&self, base_url: &str, language: &str, visited: &mut HashSet<String>) -> Result<Vec<DiscoveredUrl>> {
        if visited.contains(base_url) {
            return Ok(Vec::new());
        }

        visited.insert(base_url.to_string());
        debug!("🌐 探索基础URL: {}", base_url);

        let mut discovered = Vec::new();

        // 使用智能爬虫获取页面内容
        let scraper = IntelligentScraper::new(self.http_client.clone(), false).await?;
        let scrape_result = scraper.scrape_intelligent(base_url, &[]).await?;

        // 分析内容类型
        let content_type = scraper.detect_content_type(&scrape_result.content).await;
        
        // 基于内容类型创建URL记录
        let url_type = match content_type {
            ContentType::Changelog => UrlType::Changelog,
            ContentType::ReleasePage => UrlType::ReleaseNotes,
            ContentType::Documentation => UrlType::Documentation,
            ContentType::BlogPost => UrlType::BlogPost,
            _ => UrlType::Unknown,
        };

        discovered.push(DiscoveredUrl {
            url: base_url.to_string(),
            url_type,
            confidence: 0.8,
            source: "base_url".to_string(),
            language: Some(language.to_string()),
            version: None,
            title: Some(scrape_result.title),
            description: Some(scrape_result.content.chars().take(200).collect()),
            priority: 1,
        });

        // 从页面链接中发现更多相关URL
        for link in &scrape_result.links {
            if self.is_relevant_url(link, language) {
                let url_info = self.analyze_url(link, language).await?;
                if url_info.confidence > 0.5 {
                    discovered.push(url_info);
                }
            }
        }

        // 使用已知模式生成潜在URL
        let generated_urls = self.generate_potential_urls(base_url, language);
        for potential_url in generated_urls {
            if !visited.contains(&potential_url) && self.url_exists(&potential_url).await {
                let url_info = self.analyze_url(&potential_url, language).await?;
                if url_info.confidence > 0.3 {
                    discovered.push(url_info);
                }
            }
        }

        Ok(discovered)
    }

    /// 智能URL扩展
    async fn intelligent_url_expansion(&self, base_urls: &[DiscoveredUrl], language: &str) -> Result<Vec<DiscoveredUrl>> {
        debug!("🧠 执行智能URL扩展");

        let mut expanded = Vec::new();

        for base_url in base_urls {
            // GitHub特殊处理
            if base_url.url.contains("github.com") {
                let github_urls = self.discover_github_urls(&base_url.url, language).await?;
                expanded.extend(github_urls);
            }

            // 文档站点特殊处理
            if base_url.url_type == UrlType::Documentation {
                let doc_urls = self.discover_documentation_urls(&base_url.url, language).await?;
                expanded.extend(doc_urls);
            }

            // 版本特定URL生成
            if let Some(version) = &base_url.version {
                let version_urls = self.generate_version_specific_urls(&base_url.url, language, version);
                for url in version_urls {
                    if self.url_exists(&url).await {
                        let url_info = self.analyze_url(&url, language).await?;
                        expanded.push(url_info);
                    }
                }
            }
        }

        Ok(expanded)
    }

    /// 发现GitHub相关URL
    async fn discover_github_urls(&self, github_url: &str, language: &str) -> Result<Vec<DiscoveredUrl>> {
        debug!("🐙 发现GitHub相关URL: {}", github_url);

        let mut urls = Vec::new();

        if let Ok(parsed_url) = Url::parse(github_url) {
            if let Some(path_segments) = parsed_url.path_segments() {
                let segments: Vec<&str> = path_segments.collect();
                if segments.len() >= 2 {
                    let owner = segments[0];
                    let repo = segments[1];

                    // 生成GitHub相关URL
                    let github_base = format!("https://github.com/{}/{}", owner, repo);
                    
                    let potential_urls = vec![
                        format!("{}/releases", github_base),
                        format!("{}/tags", github_base),
                        format!("{}/blob/main/CHANGELOG.md", github_base),
                        format!("{}/blob/master/CHANGELOG.md", github_base),
                        format!("{}/blob/main/CHANGES.md", github_base),
                        format!("{}/blob/master/CHANGES.md", github_base),
                        format!("{}/blob/main/HISTORY.md", github_base),
                        format!("{}/blob/master/HISTORY.md", github_base),
                        format!("{}/wiki", github_base),
                        format!("https://{}.github.io/{}", owner, repo),
                    ];

                    for url in potential_urls {
                        if self.url_exists(&url).await {
                            let url_info = self.analyze_url(&url, language).await?;
                            urls.push(url_info);
                        }
                    }

                    // API URL
                    let api_url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
                    urls.push(DiscoveredUrl {
                        url: api_url,
                        url_type: UrlType::GitHubRelease,
                        confidence: 0.9,
                        source: "github_api".to_string(),
                        language: Some(language.to_string()),
                        version: None,
                        title: Some(format!("{}/{} Releases API", owner, repo)),
                        description: Some("GitHub API endpoint for releases".to_string()),
                        priority: 2,
                    });
                }
            }
        }

        Ok(urls)
    }

    /// 发现文档相关URL
    async fn discover_documentation_urls(&self, doc_url: &str, language: &str) -> Result<Vec<DiscoveredUrl>> {
        debug!("📚 发现文档相关URL: {}", doc_url);

        let mut urls = Vec::new();

        if let Ok(parsed_url) = Url::parse(doc_url) {
            if let Some(host) = parsed_url.host_str() {
                let base_scheme = parsed_url.scheme();
                let base_url = format!("{}://{}", base_scheme, host);

                // 常见文档路径
                let doc_paths = vec![
                    "/changelog",
                    "/changes",
                    "/releases",
                    "/history",
                    "/whatsnew",
                    "/news",
                    "/updates",
                    "/versions",
                ];

                for path in doc_paths {
                    let potential_url = format!("{}{}", base_url, path);
                    if self.url_exists(&potential_url).await {
                        let url_info = self.analyze_url(&potential_url, language).await?;
                        urls.push(url_info);
                    }
                }

                // 语言特定的文档路径
                let lang_specific_paths = self.get_language_specific_doc_paths(language);
                for path in lang_specific_paths {
                    let potential_url = format!("{}{}", base_url, path);
                    if self.url_exists(&potential_url).await {
                        let url_info = self.analyze_url(&potential_url, language).await?;
                        urls.push(url_info);
                    }
                }
            }
        }

        Ok(urls)
    }

    /// 获取语言特定的文档路径
    fn get_language_specific_doc_paths(&self, language: &str) -> Vec<String> {
        match language {
            "rust" => vec![
                "/stable".to_string(),
                "/beta".to_string(), 
                "/nightly".to_string(),
                "/book".to_string(),
                "/reference".to_string(),
                "/cargo".to_string(),
            ],
            "python" => vec![
                "/3/whatsnew".to_string(),
                "/3/library".to_string(),
                "/3/reference".to_string(),
                "/dev/whatsnew".to_string(),
            ],
            "javascript" => vec![
                "/docs/Web/JavaScript/New_in_JavaScript".to_string(),
                "/en-US/docs/Web/JavaScript".to_string(),
            ],
            "java" => vec![
                "/javase".to_string(),
                "/java".to_string(),
                "/jdk".to_string(),
            ],
            "go" => vec![
                "/doc/devel/release".to_string(),
                "/blog".to_string(),
                "/pkg".to_string(),
            ],
            _ => Vec::new(),
        }
    }

    /// 生成版本特定URL
    fn generate_version_specific_urls(&self, base_url: &str, _language: &str, version: &str) -> Vec<String> {
        let mut urls = Vec::new();

        if let Ok(parsed_url) = Url::parse(base_url) {
            let base = format!("{}://{}", parsed_url.scheme(), parsed_url.host_str().unwrap_or(""));
            
            // 版本特定的路径模式
            let version_patterns = vec![
                format!("/v{}", version),
                format!("/{}", version),
                format!("/version/{}", version),
                format!("/release/{}", version),
                format!("/tag/{}", version),
                format!("/releases/tag/v{}", version),
                format!("/releases/tag/{}", version),
            ];

            for pattern in version_patterns {
                urls.push(format!("{}{}", base, pattern));
            }
        }

        urls
    }

    /// 生成潜在URL
    fn generate_potential_urls(&self, base_url: &str, _language: &str) -> Vec<String> {
        let mut urls = Vec::new();

        if let Ok(parsed_url) = Url::parse(base_url) {
            let base = format!("{}://{}", parsed_url.scheme(), parsed_url.host_str().unwrap_or(""));
            
            // 常见的changelog和release路径
            let common_paths = vec![
                "/changelog",
                "/CHANGELOG",
                "/Changelog",
                "/changelog.md",
                "/CHANGELOG.md",
                "/changes",
                "/CHANGES",
                "/changes.md",
                "/CHANGES.md",
                "/releases",
                "/release",
                "/releases.html",
                "/news",
                "/history",
                "/whatsnew",
                "/updates",
                "/versions",
            ];

            for path in common_paths {
                urls.push(format!("{}{}", base, path));
            }
        }

        urls
    }

    /// 检查URL是否相关
    fn is_relevant_url(&self, url: &str, language: &str) -> bool {
        let url_lower = url.to_lowercase();
        let lang_lower = language.to_lowercase();

        // 检查是否包含语言名称
        if url_lower.contains(&lang_lower) {
            return true;
        }

        // 检查是否匹配已知模式
        for pattern in &self.url_patterns.changelog_patterns {
            if url_lower.contains(pattern) {
                return true;
            }
        }

        for pattern in &self.url_patterns.release_patterns {
            if url_lower.contains(pattern) {
                return true;
            }
        }

        for pattern in &self.url_patterns.documentation_patterns {
            if url_lower.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// 分析URL
    async fn analyze_url(&self, url: &str, language: &str) -> Result<DiscoveredUrl> {
        debug!("🔍 分析URL: {}", url);

        let mut confidence: f32 = 0.1;
        let mut url_type = UrlType::Unknown;
        let mut priority = 5u8;

        let url_lower = url.to_lowercase();
        let lang_lower = language.to_lowercase();

        // 基于URL路径分析
        if url_lower.contains(&lang_lower) {
            confidence += 0.3;
        }

        // GitHub URL特殊处理
        if url_lower.contains("github.com") {
            confidence += 0.2;
            if url_lower.contains("/releases") {
                url_type = UrlType::GitHubRelease;
                confidence += 0.3;
                priority = 1;
            }
        }

        // 模式匹配
        for pattern in &self.url_patterns.changelog_patterns {
            if url_lower.contains(pattern) {
                url_type = UrlType::Changelog;
                confidence += 0.4;
                priority = priority.min(2);
                break;
            }
        }

        for pattern in &self.url_patterns.release_patterns {
            if url_lower.contains(pattern) {
                if url_type == UrlType::Unknown {
                    url_type = UrlType::ReleaseNotes;
                }
                confidence += 0.3;
                priority = priority.min(2);
                break;
            }
        }

        for pattern in &self.url_patterns.documentation_patterns {
            if url_lower.contains(pattern) {
                if url_type == UrlType::Unknown {
                    url_type = UrlType::Documentation;
                }
                confidence += 0.2;
                priority = priority.min(3);
                break;
            }
        }

        // 版本检测
        let version = self.extract_version_from_url(url);

        Ok(DiscoveredUrl {
            url: url.to_string(),
            url_type,
            confidence: confidence.min(1.0),
            source: "url_analysis".to_string(),
            language: Some(language.to_string()),
            version,
            title: None,
            description: None,
            priority,
        })
    }

    /// 从URL中提取版本信息
    fn extract_version_from_url(&self, url: &str) -> Option<String> {
        // 简单的版本匹配，查找v开头的版本号
        let patterns = ["v1.", "v2.", "v3.", "v4.", "v5.", "/1.", "/2.", "/3.", "/4.", "/5."];
        
        for pattern in &patterns {
            if let Some(start_pos) = url.find(pattern) {
                let version_start = if pattern.starts_with('/') {
                    start_pos + 1
                } else {
                    start_pos + 1 // 跳过'v'
                };
                
                // 找到版本号的结束位置
                if let Some(remaining) = url.get(version_start..) {
                    if let Some(end_pos) = remaining.find(|c: char| !c.is_ascii_digit() && c != '.') {
                        if let Some(version) = remaining.get(..end_pos) {
                            if !version.is_empty() {
                                return Some(version.to_string());
                            }
                        }
                    } else {
                        // 版本号到字符串结尾
                        return Some(remaining.to_string());
                    }
                }
            }
        }
        
        None
    }

    /// 检查URL是否存在
    async fn url_exists(&self, url: &str) -> bool {
        match self.http_client.head(url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// 去重和排序URL
    fn deduplicate_and_sort_urls(&self, urls: &mut Vec<DiscoveredUrl>) {
        // 去重
        let mut seen = HashSet::new();
        urls.retain(|url| seen.insert(url.url.clone()));

        // 排序：优先级 -> 置信度 -> 类型
        urls.sort_by(|a, b| {
            a.priority.cmp(&b.priority)
                .then_with(|| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| {
                    match (&a.url_type, &b.url_type) {
                        (UrlType::Changelog, _) => std::cmp::Ordering::Less,
                        (_, UrlType::Changelog) => std::cmp::Ordering::Greater,
                        (UrlType::ReleaseNotes, _) => std::cmp::Ordering::Less,
                        (_, UrlType::ReleaseNotes) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    }
                })
        });
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.discovery_cache.clear();
        info!("🧹 清除URL发现缓存");
    }

    /// 获取发现统计信息
    pub fn get_discovery_stats(&self) -> DiscoveryStats {
        DiscoveryStats {
            cached_languages: self.discovery_cache.len(),
            total_discovered_urls: self.discovery_cache.values().map(|urls| urls.len()).sum(),
            pattern_count: self.url_patterns.changelog_patterns.len() 
                + self.url_patterns.release_patterns.len()
                + self.url_patterns.documentation_patterns.len(),
        }
    }

    /// 发现所有相关URL
    #[allow(dead_code)]
    async fn discover_all_related_urls(&self, base_url: &str, language: &str) -> Result<Vec<DiscoveredUrl>> {
        let mut all_urls = Vec::new();
        
        // 发现变更日志URL
        for pattern in &self.url_patterns.changelog_patterns {
            let url = format!("{}/{}", base_url.trim_end_matches('/'), pattern);
            if self.url_exists(&url).await {
                all_urls.push(DiscoveredUrl {
                    url,
                    url_type: UrlType::Changelog,
                    confidence: 0.8,
                    source: "pattern".to_string(),
                    language: Some(language.to_string()),
                    version: None,
                    title: None,
                    description: None,
                    priority: 2,
                });
            }
        }
        
        // 发现发布页面URL
        for pattern in &self.url_patterns.release_patterns {
            let url = format!("{}/{}", base_url.trim_end_matches('/'), pattern);
            if self.url_exists(&url).await {
                all_urls.push(DiscoveredUrl {
                    url,
                    url_type: UrlType::ReleaseNotes,
                    confidence: 0.7,
                    source: "pattern".to_string(),
                    language: Some(language.to_string()),
                    version: None,
                    title: None,
                    description: None,
                    priority: 2,
                });
            }
        }
        
        // 发现文档URL
        for pattern in &self.url_patterns.documentation_patterns {
            let url = format!("{}/{}", base_url.trim_end_matches('/'), pattern);
            if self.url_exists(&url).await {
                all_urls.push(DiscoveredUrl {
                    url,
                    url_type: UrlType::Documentation,
                    confidence: 0.9,
                    source: "pattern".to_string(),
                    language: Some(language.to_string()),
                    version: None,
                    title: None,
                    description: None,
                    priority: 1,
                });
            }
        }
        
        Ok(all_urls)
    }

    /// 检查变更日志URL
    #[allow(dead_code)]
    async fn discover_changelog_urls(&self, base_url: &str, language: &str) -> Result<Vec<DiscoveredUrl>> {
        let mut changelog_urls = Vec::new();
        
        for pattern in &self.url_patterns.changelog_patterns {
            let url = format!("{}/{}", base_url.trim_end_matches('/'), pattern);
            if self.url_exists(&url).await {
                changelog_urls.push(DiscoveredUrl {
                    url,
                    url_type: UrlType::Changelog,
                    confidence: 0.8,
                    source: "pattern".to_string(),
                    language: Some(language.to_string()),
                    version: None,
                    title: None,
                    description: None,
                    priority: 2,
                });
            }
        }
        
        for pattern in &self.url_patterns.release_patterns {
            let url = format!("{}/{}", base_url.trim_end_matches('/'), pattern);
            if self.url_exists(&url).await {
                changelog_urls.push(DiscoveredUrl {
                    url,
                    url_type: UrlType::ReleaseNotes,
                    confidence: 0.7,
                    source: "pattern".to_string(),
                    language: Some(language.to_string()),
                    version: None,
                    title: None,
                    description: None,
                    priority: 2,
                });
            }
        }
        
        Ok(changelog_urls)
    }

    /// 检查文档URL
    #[allow(dead_code)]
    async fn discover_documentation_urls_simple(&self, base_url: &str, language: &str) -> Result<Vec<DiscoveredUrl>> {
        let mut doc_urls = Vec::new();
        
        for pattern in &self.url_patterns.documentation_patterns {
            let url = format!("{}/{}", base_url.trim_end_matches('/'), pattern);
            if self.url_exists(&url).await {
                doc_urls.push(DiscoveredUrl {
                    url,
                    url_type: UrlType::Documentation,
                    confidence: 0.9,
                    source: "pattern".to_string(),
                    language: Some(language.to_string()),
                    version: None,
                    title: None,
                    description: None,
                    priority: 1,
                });
            }
        }
        
        Ok(doc_urls)
    }
}

// 实现PartialEq用于URL类型比较
impl PartialEq for UrlType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// 发现统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryStats {
    pub cached_languages: usize,
    pub total_discovered_urls: usize,
    pub pattern_count: usize,
} 