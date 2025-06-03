use anyhow::Result;
use reqwest::Client;
use tracing::debug;
use serde::{Deserialize, Serialize};

// 使用第三方成熟库
use aho_corasick::AhoCorasick;
use fancy_regex::Regex;
use url::Url;

use super::url_discovery::UrlType;

/// 智能URL分析器 - 使用机器学习和高级算法
pub struct SmartUrlAnalyzer {
    /// HTTP客户端
    client: Client,
    /// 文档模式匹配器
    doc_patterns: AhoCorasick,
    /// URL特征提取器
    feature_extractor: UrlFeatureExtractor,
    /// 分析配置
    config: AnalysisConfig,
}

/// 分析配置
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// 相关性阈值
    pub relevance_threshold: f32,
    /// 最大分析深度
    pub max_analysis_depth: usize,
    /// 内容采样大小
    pub content_sample_size: usize,
    /// 是否启用机器学习特征
    pub enable_ml_features: bool,
    /// 缓存TTL（秒）
    pub cache_ttl_seconds: u64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            relevance_threshold: 0.6,
            max_analysis_depth: 2,
            content_sample_size: 1000,
            enable_ml_features: true,
            cache_ttl_seconds: 3600,
        }
    }
}

/// URL特征提取器
#[derive(Debug, Clone)]
pub struct UrlFeatureExtractor {
    /// 版本模式
    version_patterns: Vec<Regex>,
}

/// URL分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlAnalysisResult {
    /// 原始URL
    pub url: String,
    /// 相关性分数 (0.0-1.0)
    pub relevance_score: f32,
    /// URL类型
    pub url_type: UrlType,
    /// 置信度
    pub confidence: f32,
    /// 特征向量
    pub features: UrlFeatures,
    /// 内容预览
    pub content_preview: Option<String>,
    /// 分析元数据
    pub metadata: AnalysisMetadata,
}

/// URL特征
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlFeatures {
    /// 域名特征
    pub domain_features: DomainFeatures,
    /// 路径特征
    pub path_features: PathFeatures,
    /// 内容特征
    pub content_features: Option<ContentFeatures>,
    /// 语言特征
    pub language_features: LanguageFeatures,
}

/// 域名特征
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainFeatures {
    /// 是否为知名文档站点
    pub is_known_doc_site: bool,
    /// 域名权威性分数
    pub authority_score: f32,
    /// 是否为官方域名
    pub is_official: bool,
    /// 子域名类型
    pub subdomain_type: SubdomainType,
}

/// 子域名类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubdomainType {
    Docs,
    Api,
    Wiki,
    Blog,
    Download,
    Other,
}

/// 路径特征
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathFeatures {
    /// 路径深度
    pub depth: usize,
    /// 包含文档关键词数量
    pub doc_keyword_count: usize,
    /// 包含版本信息
    pub has_version: bool,
    /// 路径类型
    pub path_type: PathType,
    /// 文件扩展名
    pub file_extension: Option<String>,
}

/// 路径类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathType {
    Documentation,
    ApiReference,
    Tutorial,
    Example,
    Changelog,
    Download,
    Other,
}

/// 内容特征
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFeatures {
    /// 内容长度
    pub content_length: usize,
    /// 代码块数量
    pub code_block_count: usize,
    /// API文档结构数量
    pub api_structure_count: usize,
    /// 文档质量分数
    pub quality_score: f32,
    /// 语言检测结果
    pub detected_language: Option<String>,
}

/// 语言特征
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageFeatures {
    /// 目标编程语言匹配度
    pub language_match_score: f32,
    /// URL中的语言指示器
    pub language_indicators: Vec<String>,
    /// 内容语言一致性
    pub language_consistency: f32,
}

/// 分析元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    /// 分析时间戳
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
    /// 分析方法
    pub analysis_method: String,
    /// 处理时间（毫秒）
    pub processing_time_ms: u64,
    /// 是否使用缓存
    pub from_cache: bool,
}

impl SmartUrlAnalyzer {
    /// 创建新的智能URL分析器
    pub async fn new(config: AnalysisConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Grape-MCP-DevTools/1.0")
            .build()?;

        // 创建文档模式匹配器
        let doc_patterns = AhoCorasick::new(&[
            "docs", "documentation", "api", "reference", "guide", "tutorial",
            "manual", "help", "wiki", "readme", "changelog", "examples"
        ])?;

        let feature_extractor = UrlFeatureExtractor::new()?;

        Ok(Self {
            client,
            doc_patterns,
            feature_extractor,
            config,
        })
    }

    /// 分析URL相关性
    pub async fn analyze_url_relevance(
        &self,
        url: &str,
        target_language: &str,
        context: &AnalysisContext,
    ) -> Result<UrlAnalysisResult> {
        let start_time = std::time::Instant::now();
        
        debug!("🔍 分析URL相关性: {} (语言: {})", url, target_language);

        // 提取URL特征
        let features = self.extract_url_features(url, target_language).await?;
        
        // 计算基础相关性分数
        let relevance_score = self.calculate_base_relevance_score(&features, target_language, context);
        
        // 分类URL类型
        let url_type = self.classify_url_type(&features);
        
        // 计算置信度
        let confidence = self.calculate_confidence(&features, relevance_score);
        
        // 尝试获取内容预览
        let content_preview = self.get_content_preview(url).await.ok();
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        Ok(UrlAnalysisResult {
            url: url.to_string(),
            relevance_score,
            url_type,
            confidence,
            features,
            content_preview,
            metadata: AnalysisMetadata {
                analyzed_at: chrono::Utc::now(),
                analysis_method: "smart_ml_analysis".to_string(),
                processing_time_ms: processing_time,
                from_cache: false,
            },
        })
    }

    /// 提取URL特征
    async fn extract_url_features(&self, url: &str, target_language: &str) -> Result<UrlFeatures> {
        let parsed_url = Url::parse(url)?;
        
        let domain_features = self.extract_domain_features(&parsed_url)?;
        let path_features = self.extract_path_features(&parsed_url, target_language)?;
        let language_features = self.extract_language_features(&parsed_url, target_language)?;
        
        // 尝试提取内容特征
        let content_features = match self.get_content_preview(url).await {
            Ok(content) => {
                Some(self.extract_content_features(&content, target_language))
            }
            Err(e) => {
                debug!("⚠️ 无法获取内容预览进行特征提取: {}", e);
                None
            }
        };
        
        Ok(UrlFeatures {
            domain_features,
            path_features,
            content_features,
            language_features,
        })
    }

    /// 提取域名特征
    fn extract_domain_features(&self, url: &Url) -> Result<DomainFeatures> {
        let host = url.host_str().unwrap_or("");
        let domain = url.domain().unwrap_or("");
        
        let is_known_doc_site = self.is_known_documentation_site(domain);
        let authority_score = self.calculate_domain_authority(domain);
        let is_official = self.is_official_domain(domain);
        let subdomain_type = self.classify_subdomain(host);
        
        Ok(DomainFeatures {
            is_known_doc_site,
            authority_score,
            is_official,
            subdomain_type,
        })
    }

    /// 提取路径特征
    fn extract_path_features(&self, url: &Url, _target_language: &str) -> Result<PathFeatures> {
        let path = url.path();
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        let depth = segments.len();
        let doc_keyword_count = self.count_doc_keywords(path);
        let has_version = self.feature_extractor.has_version_info(path);
        let path_type = self.classify_path_type(path);
        let file_extension = self.extract_file_extension(path);
        
        Ok(PathFeatures {
            depth,
            doc_keyword_count,
            has_version,
            path_type,
            file_extension,
        })
    }

    /// 提取语言特征
    fn extract_language_features(&self, url: &Url, target_language: &str) -> Result<LanguageFeatures> {
        let url_str = url.as_str();
        
        let language_match_score = self.calculate_language_match_score(url_str, target_language);
        let language_indicators = self.find_language_indicators(url_str, target_language);
        let language_consistency = self.calculate_language_consistency(url_str, target_language, &language_indicators);
        
        Ok(LanguageFeatures {
            language_match_score,
            language_indicators,
            language_consistency,
        })
    }

    /// 计算基础相关性分数
    fn calculate_base_relevance_score(
        &self,
        features: &UrlFeatures,
        _target_language: &str,
        _context: &AnalysisContext,
    ) -> f32 {
        let mut score = 0.0;
        
        // 域名特征权重
        if features.domain_features.is_known_doc_site {
            score += 0.3;
        }
        score += features.domain_features.authority_score * 0.2;
        if features.domain_features.is_official {
            score += 0.2;
        }
        
        // 路径特征权重
        score += (features.path_features.doc_keyword_count as f32 * 0.05).min(0.2);
        if features.path_features.has_version {
            score += 0.1;
        }
        score += self.get_path_type_score(&features.path_features.path_type);
        
        // 语言特征权重
        score += features.language_features.language_match_score * 0.2;
        
        score.min(1.0)
    }

    /// 获取内容预览
    async fn get_content_preview(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await?;
        let content = response.text().await?;
        
        // 简单的内容预览提取
        let preview = if content.len() > self.config.content_sample_size {
            content.chars().take(self.config.content_sample_size).collect()
        } else {
            content
        };
        
        Ok(preview)
    }

    /// 分类URL类型
    fn classify_url_type(&self, features: &UrlFeatures) -> UrlType {
        match features.path_features.path_type {
            PathType::Documentation => UrlType::Documentation,
            PathType::ApiReference => UrlType::ApiReference,
            PathType::Tutorial => UrlType::Tutorial,
            PathType::Example => UrlType::Example,
            PathType::Changelog => UrlType::Changelog,
            _ => UrlType::Other,
        }
    }

    /// 计算置信度
    fn calculate_confidence(&self, features: &UrlFeatures, relevance_score: f32) -> f32 {
        let mut confidence = relevance_score;
        
        // 基于特征的置信度调整
        if features.domain_features.is_known_doc_site {
            confidence += 0.1;
        }
        if features.domain_features.is_official {
            confidence += 0.1;
        }
        if features.path_features.has_version {
            confidence += 0.05;
        }
        
        confidence.min(1.0)
    }

    /// 计算域名权威性
    fn calculate_domain_authority(&self, domain: &str) -> f32 {
        // 完善的域名权威性计算
        let mut authority_score: f32 = 0.0;
        
        // 知名权威域名
        let high_authority_domains = [
            "github.com", "docs.rs", "pypi.org", "npmjs.com", "maven.org",
            "golang.org", "rust-lang.org", "python.org", "nodejs.org",
            "oracle.com", "microsoft.com", "mozilla.org", "apache.org",
            "stackoverflow.com", "developer.mozilla.org", "w3.org"
        ];
        
        // 中等权威域名
        let medium_authority_domains = [
            "readthedocs.io", "gitbook.io", "confluence.atlassian.com",
            "wiki.archlinux.org", "ubuntu.com", "debian.org", "fedoraproject.org"
        ];
        
        // 检查高权威域名
        if high_authority_domains.iter().any(|&auth| domain.contains(auth)) {
            authority_score += 0.9;
        } else if medium_authority_domains.iter().any(|&auth| domain.contains(auth)) {
            authority_score += 0.7;
        } else {
            // 基于域名特征计算
            if domain.ends_with(".org") {
                authority_score += 0.6;
            } else if domain.ends_with(".edu") {
                authority_score += 0.8;
            } else if domain.ends_with(".gov") {
                authority_score += 0.9;
            } else if domain.ends_with(".com") {
                authority_score += 0.4;
            } else if domain.ends_with(".io") {
                authority_score += 0.5;
            } else {
                authority_score += 0.3;
            }
            
            // 检查子域名特征
            if domain.starts_with("docs.") || domain.contains("documentation") {
                authority_score += 0.2;
            }
            if domain.starts_with("api.") || domain.contains("api") {
                authority_score += 0.15;
            }
            if domain.contains("wiki") {
                authority_score += 0.1;
            }
        }
        
        authority_score.min(1.0)
    }

    /// 检查是否为官方域名
    fn is_official_domain(&self, domain: &str) -> bool {
        let official_domains = [
            "rust-lang.org", "python.org", "nodejs.org", "golang.org",
            "oracle.com", "microsoft.com", "docs.rs", "pypi.org"
        ];
        
        official_domains.iter().any(|&official| domain.contains(official))
    }

    /// 分类子域名
    fn classify_subdomain(&self, host: &str) -> SubdomainType {
        if host.starts_with("docs.") || host.contains("documentation") {
            SubdomainType::Docs
        } else if host.starts_with("api.") || host.contains("api") {
            SubdomainType::Api
        } else if host.contains("wiki") {
            SubdomainType::Wiki
        } else if host.contains("blog") {
            SubdomainType::Blog
        } else if host.contains("download") {
            SubdomainType::Download
        } else {
            SubdomainType::Other
        }
    }

    /// 分类路径类型
    fn classify_path_type(&self, path: &str) -> PathType {
        let path_lower = path.to_lowercase();
        
        if path_lower.contains("api") || path_lower.contains("reference") {
            PathType::ApiReference
        } else if path_lower.contains("tutorial") || path_lower.contains("guide") {
            PathType::Tutorial
        } else if path_lower.contains("example") || path_lower.contains("sample") {
            PathType::Example
        } else if path_lower.contains("changelog") || path_lower.contains("release") {
            PathType::Changelog
        } else if path_lower.contains("download") {
            PathType::Download
        } else if path_lower.contains("doc") || path_lower.contains("manual") {
            PathType::Documentation
        } else {
            PathType::Other
        }
    }

    /// 计算语言匹配分数
    fn calculate_language_match_score(&self, url: &str, target_language: &str) -> f32 {
        let url_lower = url.to_lowercase();
        let lang_lower = target_language.to_lowercase();
        
        if url_lower.contains(&lang_lower) {
            0.8
        } else {
            // 检查语言别名
            let aliases = match lang_lower.as_str() {
                "rust" => vec!["rs", "cargo"],
                "python" => vec!["py", "pypi"],
                "javascript" => vec!["js", "npm", "node"],
                "java" => vec!["maven", "gradle"],
                "go" => vec!["golang"],
                _ => vec![],
            };
            
            if aliases.iter().any(|alias| url_lower.contains(alias)) {
                0.6
            } else {
                0.2
            }
        }
    }

    /// 查找语言指示器
    fn find_language_indicators(&self, url: &str, target_language: &str) -> Vec<String> {
        let mut indicators = Vec::new();
        let url_lower = url.to_lowercase();
        let lang_lower = target_language.to_lowercase();
        
        if url_lower.contains(&lang_lower) {
            indicators.push(lang_lower);
        }
        
        // 添加语言相关的指示器
        let lang_indicators = match target_language.to_lowercase().as_str() {
            "rust" => vec!["rs", "cargo", "crates"],
            "python" => vec!["py", "pypi", "pip"],
            "javascript" => vec!["js", "npm", "node"],
            "java" => vec!["maven", "gradle", "jdk"],
            "go" => vec!["golang", "pkg.go.dev"],
            _ => vec![],
        };
        
        for indicator in lang_indicators {
            if url_lower.contains(indicator) {
                indicators.push(indicator.to_string());
            }
        }
        
        indicators
    }

    /// 获取路径类型分数
    fn get_path_type_score(&self, path_type: &PathType) -> f32 {
        match path_type {
            PathType::Documentation => 0.3,
            PathType::ApiReference => 0.25,
            PathType::Tutorial => 0.2,
            PathType::Example => 0.15,
            PathType::Changelog => 0.1,
            PathType::Download => 0.05,
            PathType::Other => 0.0,
        }
    }

    /// 检查是否为知名文档站点
    fn is_known_documentation_site(&self, domain: &str) -> bool {
        let doc_sites = [
            "docs.rs", "readthedocs.io", "github.io", "gitbook.io",
            "confluence", "notion.so", "gitiles", "docs.python.org",
            "nodejs.org", "golang.org", "rust-lang.org"
        ];
        
        doc_sites.iter().any(|&site| domain.contains(site))
    }

    /// 统计文档关键词数量
    fn count_doc_keywords(&self, path: &str) -> usize {
        self.doc_patterns.find_iter(path).count()
    }

    /// 提取文件扩展名
    fn extract_file_extension(&self, path: &str) -> Option<String> {
        if let Some(last_segment) = path.split('/').last() {
            if let Some(dot_pos) = last_segment.rfind('.') {
                return Some(last_segment[dot_pos + 1..].to_string());
            }
        }
        None
    }

    /// 计算语言一致性
    fn calculate_language_consistency(&self, url: &str, _target_language: &str, indicators: &[String]) -> f32 {
        let url_lower = url.to_lowercase();
        
        let mut consistency: f32 = 0.0;
        for indicator in indicators {
            if url_lower.contains(indicator) {
                consistency += 0.1;
            }
        }
        
        consistency.min(1.0)
    }

    /// 提取内容特征
    fn extract_content_features(&self, content: &str, target_language: &str) -> ContentFeatures {
        let content_length = content.len();
        
        // 计算代码块数量
        let code_block_count = content.matches("```").count() / 2 + 
                               content.matches("<code>").count() +
                               content.matches("<pre>").count();
        
        // 计算API结构数量（函数、类、方法等）
        let api_structure_count = self.count_api_structures(content, target_language);
        
        // 计算文档质量分数
        let quality_score = self.calculate_content_quality_score(content, target_language);
        
        // 检测语言
        let detected_language = self.detect_primary_language(content, target_language);
        
        ContentFeatures {
            content_length,
            code_block_count,
            api_structure_count,
            quality_score,
            detected_language,
        }
    }
    
    /// 计算API结构数量
    fn count_api_structures(&self, content: &str, target_language: &str) -> usize {
        let mut count = 0;
        
        match target_language {
            "rust" => {
                count += content.matches("fn ").count();
                count += content.matches("struct ").count();
                count += content.matches("enum ").count();
                count += content.matches("trait ").count();
                count += content.matches("impl ").count();
            }
            "python" => {
                count += content.matches("def ").count();
                count += content.matches("class ").count();
                count += content.matches("@property").count();
            }
            "javascript" | "typescript" => {
                count += content.matches("function ").count();
                count += content.matches("class ").count();
                count += content.matches("const ").count();
                count += content.matches("interface ").count();
            }
            "java" => {
                count += content.matches("public class ").count();
                count += content.matches("public interface ").count();
                count += content.matches("public void ").count();
                count += content.matches("public static ").count();
            }
            "go" => {
                count += content.matches("func ").count();
                count += content.matches("type ").count();
                count += content.matches("struct ").count();
                count += content.matches("interface ").count();
            }
            _ => {
                // 通用模式
                count += content.matches("function").count();
                count += content.matches("method").count();
                count += content.matches("class").count();
            }
        }
        
        count
    }
    
    /// 计算内容质量分数
    fn calculate_content_quality_score(&self, content: &str, _target_language: &str) -> f32 {
        let mut score = 0.0;
        
        // 长度合理性
        if content.len() > 500 && content.len() < 50000 {
            score += 0.3;
        } else if content.len() >= 200 {
            score += 0.1;
        }
        
        // 结构化内容
        if content.contains("# ") || content.contains("## ") {
            score += 0.2; // 有标题结构
        }
        
        // 代码示例
        if content.contains("```") || content.contains("<code>") {
            score += 0.2;
        }
        
        // 文档关键词
        let doc_keywords = ["example", "usage", "tutorial", "guide", "documentation", "api", "reference"];
        let keyword_count = doc_keywords.iter()
            .filter(|&keyword| content.to_lowercase().contains(keyword))
            .count();
        score += (keyword_count as f32 * 0.05).min(0.3);
        
        score.min(1.0)
    }
    
    /// 检测主要编程语言
    fn detect_primary_language(&self, content: &str, target_language: &str) -> Option<String> {
        // 简单的语言检测基于关键词频率
        let mut language_scores = std::collections::HashMap::new();
        
        // Rust特征
        let rust_keywords = ["fn ", "struct ", "enum ", "trait ", "impl ", "let ", "mut ", "match "];
        let rust_score = rust_keywords.iter().map(|k| content.matches(k).count()).sum::<usize>();
        language_scores.insert("rust", rust_score);
        
        // Python特征
        let python_keywords = ["def ", "class ", "import ", "from ", "if __name__", "elif ", "return "];
        let python_score = python_keywords.iter().map(|k| content.matches(k).count()).sum::<usize>();
        language_scores.insert("python", python_score);
        
        // JavaScript特征
        let js_keywords = ["function ", "const ", "let ", "var ", "=> ", "async ", "await "];
        let js_score = js_keywords.iter().map(|k| content.matches(k).count()).sum::<usize>();
        language_scores.insert("javascript", js_score);
        
        // 找到分数最高的语言
        let detected = language_scores.iter()
            .max_by_key(|(_, &score)| score)
            .filter(|(_, &score)| score > 0)
            .map(|(lang, _)| lang.to_string());
        
        // 如果检测到的语言与目标语言匹配，返回目标语言
        if detected.as_ref().map_or(false, |d| d == target_language) {
            Some(target_language.to_string())
        } else {
            detected
        }
    }
}

impl UrlFeatureExtractor {
    fn new() -> Result<Self> {
        let version_patterns = vec![
            Regex::new(r"v?\d+\.\d+(\.\d+)?")?,
            Regex::new(r"version[/-]?\d+")?,
            Regex::new(r"release[/-]?\d+")?,
        ];
        
        Ok(Self {
            version_patterns,
        })
    }

    fn has_version_info(&self, path: &str) -> bool {
        self.version_patterns.iter().any(|pattern| {
            pattern.is_match(path).unwrap_or(false)
        })
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub package_name: String,
    pub target_language: String,
    pub search_intent: SearchIntent,
}

#[derive(Debug, Clone)]
pub enum SearchIntent {
    Documentation,
    ApiReference,
    Tutorial,
    Example,
    Changelog,
}

#[derive(Debug, Clone)]
struct ContentAnalysis {
    preview: String,
    quality_score: f32,
    code_block_count: usize,
    api_structure_count: usize,
    detected_language: Option<String>,
}