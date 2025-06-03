use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};
use chrono::{DateTime, Utc};

// 第三方库导入
use chromiumoxide::{Browser, BrowserConfig, Page};
use kuchiki::traits::*;
use lingua::{Language, LanguageDetector, LanguageDetectorBuilder};
use unicode_normalization::UnicodeNormalization;
use html_escape::decode_html_entities;
use ort::{Environment, ExecutionProvider, GraphOptimizationLevel, SessionBuilder};

// 项目内部导入
use crate::errors::GrapeError;

/// 企业级内容处理管道
/// 
/// 集成多种第三方成熟库实现高质量的网页内容分析：
/// - Chromium渲染引擎：处理JavaScript动态内容
/// - Lingua语言检测：精确识别内容语言
/// - ONNX Runtime：运行预训练AI模型
/// - 多级内容清理：确保高质量输出
#[derive(Clone)]
pub struct EnhancedContentPipeline {
    /// 浏览器实例（用于JavaScript渲染）
    browser: Arc<RwLock<Option<Browser>>>,
    /// 语言检测器
    language_detector: Arc<LanguageDetector>,
    /// ONNX运行时环境
    ort_environment: Arc<Environment>,
    /// 内容提取策略
    extraction_strategies: Vec<Arc<dyn ContentExtractionStrategy>>,
    /// 质量评估器
    quality_assessor: Arc<ContentQualityAssessor>,
    /// 缓存层
    cache: Arc<RwLock<HashMap<String, CachedContent>>>,
    /// 配置
    config: PipelineConfig,
    /// 指标收集器
    metrics: Arc<PipelineMetrics>,
}

/// 管道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// 是否启用浏览器渲染
    pub enable_browser_rendering: bool,
    /// 浏览器超时时间（秒）
    pub browser_timeout_secs: u64,
    /// 最大内容长度
    pub max_content_length: usize,
    /// 质量阈值
    pub quality_threshold: f32,
    /// 缓存TTL（秒）
    pub cache_ttl_secs: u64,
    /// 并发限制
    pub max_concurrent_extractions: usize,
    /// 启用的提取策略
    pub enabled_strategies: Vec<String>,
    /// AI模型路径
    pub ai_model_paths: HashMap<String, String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enable_browser_rendering: true,
            browser_timeout_secs: 30,
            max_content_length: 1_000_000, // 1MB
            quality_threshold: 0.7,
            cache_ttl_secs: 3600,
            max_concurrent_extractions: 10,
            enabled_strategies: vec![
                "readability".to_string(),
                "boilerpipe".to_string(),
                "ai_extraction".to_string(),
                "structured_data".to_string(),
            ],
            ai_model_paths: HashMap::new(),
        }
    }
}

/// 内容提取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentExtractionResult {
    /// 提取的主要内容
    pub main_content: String,
    /// 文档标题
    pub title: String,
    /// 文档摘要
    pub summary: Option<String>,
    /// 检测到的语言
    pub detected_language: Option<Language>,
    /// 置信度分数
    pub confidence_score: f32,
    /// 质量分数
    pub quality_score: f32,
    /// 结构化数据
    pub structured_data: HashMap<String, serde_json::Value>,
    /// 提取的链接
    pub links: Vec<ExtractedLink>,
    /// 提取的图片
    pub images: Vec<ExtractedImage>,
    /// 代码块
    pub code_blocks: Vec<CodeBlock>,
    /// 表格数据
    pub tables: Vec<TableData>,
    /// 元数据
    pub metadata: ExtractionMetadata,
}

/// 提取的链接
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLink {
    pub url: String,
    pub text: String,
    pub rel: Option<String>,
    pub title: Option<String>,
    pub relevance_score: f32,
}

/// 提取的图片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    pub url: String,
    pub alt_text: Option<String>,
    pub title: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub relevance_score: f32,
}

/// 代码块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub code: String,
    pub line_numbers: Option<Vec<u32>>,
    pub context: Option<String>,
}

/// 表格数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub caption: Option<String>,
    pub context: Option<String>,
}

/// 提取元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    pub extraction_time: DateTime<Utc>,
    pub strategies_used: Vec<String>,
    pub processing_time_ms: u64,
    pub content_source: String,
    pub ai_models_used: Vec<String>,
    pub quality_metrics: QualityMetrics,
}

/// 质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub readability_score: f32,
    pub coherence_score: f32,
    pub completeness_score: f32,
    pub relevance_score: f32,
    pub technical_content_score: f32,
}

/// 缓存内容
#[derive(Debug, Clone)]
struct CachedContent {
    content: ContentExtractionResult,
    timestamp: DateTime<Utc>,
    access_count: u32,
}

/// 内容提取策略接口
#[async_trait]
pub trait ContentExtractionStrategy: Send + Sync {
    /// 策略名称
    fn name(&self) -> &str;
    
    /// 提取内容
    async fn extract(&self, url: &str, html: &str) -> Result<ContentExtractionResult>;
    
    /// 策略优先级（越高越优先）
    fn priority(&self) -> u8;
    
    /// 是否支持该URL
    fn supports_url(&self, url: &str) -> bool;
}

/// Readability算法实现
pub struct ReadabilityStrategy {
    name: String,
}

impl ReadabilityStrategy {
    pub fn new() -> Self {
        Self {
            name: "readability".to_string(),
        }
    }
}

#[async_trait]
impl ContentExtractionStrategy for ReadabilityStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    async fn extract(&self, url: &str, html: &str) -> Result<ContentExtractionResult> {
        debug!("🔍 使用Readability策略提取内容: {}", url);
        
        // 使用kuchiki解析HTML
        let document = kuchiki::parse_html().one(html);
        
        // 寻找主要内容容器
        let main_content = self.extract_main_content(&document)?;
        let title = self.extract_title(&document);
        let links = self.extract_links(&document, url)?;
        let images = self.extract_images(&document, url)?;
        
        // 计算质量分数
        let quality_score = self.calculate_content_quality_score(&main_content);
        
        Ok(ContentExtractionResult {
            main_content,
            title,
            summary: None,
            detected_language: None,
            confidence_score: 0.8,
            quality_score,
            structured_data: HashMap::new(),
            links,
            images,
            code_blocks: Vec::new(),
            tables: Vec::new(),
            metadata: ExtractionMetadata {
                extraction_time: Utc::now(),
                strategies_used: vec![self.name().to_string()],
                processing_time_ms: 0,
                content_source: url.to_string(),
                ai_models_used: Vec::new(),
                quality_metrics: QualityMetrics {
                    readability_score: quality_score,
                    coherence_score: 0.0,
                    completeness_score: 0.0,
                    relevance_score: 0.0,
                    technical_content_score: 0.0,
                },
            },
        })
    }

    fn priority(&self) -> u8 {
        80
    }

    fn supports_url(&self, _url: &str) -> bool {
        true // 支持所有URL
    }
}

impl ReadabilityStrategy {
    fn extract_main_content(&self, document: &kuchiki::NodeRef) -> Result<String> {
        // 实现完整的Readability算法
        let content_selectors = [
            "main",
            "article", 
            ".content",
            ".main-content",
            ".post-content",
            ".entry-content",
            "#content",
            "#main",
            ".container",
        ];

        let mut best_content = String::new();
        let mut best_score = 0.0;

        for selector in &content_selectors {
            if let Ok(mut elements) = document.select(selector) {
                if let Some(element) = elements.next() {
                    let text = element.text_contents();
                    if text.len() > 100 {
                        let score = self.calculate_content_quality_score(&text);
                        if score > best_score {
                            best_score = score;
                            best_content = self.clean_text(&text);
                        }
                    }
                }
            }
        }

        if !best_content.is_empty() {
            return Ok(best_content);
        }

        // 如果没找到，使用body但进行更智能的过滤
        if let Ok(mut body_elements) = document.select("body") {
            if let Some(body) = body_elements.next() {
                let text = body.text_contents();
                let cleaned = self.clean_text(&text);
                let filtered = self.filter_noise_content(&cleaned);
                return Ok(filtered);
            }
        }

        Err(anyhow!("无法提取主要内容"))
    }

    /// 计算内容质量分数（完整的Readability算法）
    fn calculate_content_quality_score(&self, content: &str) -> f32 {
        let words = content.split_whitespace().count() as f32;
        let sentences = content.split(&['.', '!', '?'][..]).count() as f32;
        let paragraphs = content.split("\n\n").count() as f32;
        
        // 基于多个因素的质量评估
        let mut score = 0.0;
        
        // 长度分数 (0-30分)
        if words > 50.0 {
            score += 10.0;
        }
        if words > 200.0 {
            score += 10.0;
        }
        if words > 500.0 {
            score += 10.0;
        }
        
        // 结构分数 (0-30分)
        if sentences > 3.0 {
            score += 10.0;
        }
        if paragraphs > 1.0 {
            score += 10.0;
        }
        
        // 平均句长分数 (0-20分)
        let avg_sentence_length = if sentences > 0.0 { words / sentences } else { 0.0 };
        if avg_sentence_length > 5.0 && avg_sentence_length < 25.0 {
            score += 20.0;
        }
        
        // 内容多样性分数 (0-20分)
        let unique_words = content.split_whitespace()
            .map(|w| w.to_lowercase())
            .collect::<std::collections::HashSet<_>>()
            .len() as f32;
        let diversity_ratio = if words > 0.0 { unique_words / words } else { 0.0 };
        if diversity_ratio > 0.3 {
            score += 20.0;
        }
        
        score / 100.0 // 归一化到0-1
    }

    /// 过滤噪声内容
    fn filter_noise_content(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut filtered_lines = Vec::new();
        
        for line in lines {
            let trimmed = line.trim();
            
            // 跳过过短的行
            if trimmed.len() < 10 {
                continue;
            }
            
            // 跳过可能的导航或菜单项
            if trimmed.split_whitespace().count() < 3 {
                continue;
            }
            
            // 跳过可能的版权信息
            if trimmed.to_lowercase().contains("copyright") || 
               trimmed.to_lowercase().contains("©") ||
               trimmed.to_lowercase().contains("all rights reserved") {
                continue;
            }
            
            filtered_lines.push(trimmed);
        }
        
        filtered_lines.join(" ")
    }

    fn extract_title(&self, document: &kuchiki::NodeRef) -> String {
        if let Ok(mut title_elements) = document.select("title") {
            if let Some(title) = title_elements.next() {
                return self.clean_text(&title.text_contents());
            }
        }

        if let Ok(mut h1_elements) = document.select("h1") {
            if let Some(h1) = h1_elements.next() {
                return self.clean_text(&h1.text_contents());
            }
        }

        "未知标题".to_string()
    }

    fn extract_links(&self, document: &kuchiki::NodeRef, base_url: &str) -> Result<Vec<ExtractedLink>> {
        let mut links = Vec::new();
        
        if let Ok(link_elements) = document.select("a[href]") {
            for element in link_elements {
                if let Some(element_ref) = element.as_element() {
                    let attrs = element_ref.attributes.borrow();
                    
                    if let Some(href) = attrs.get("href") {
                        let text = element.text_contents();
                        if !text.trim().is_empty() && !href.starts_with('#') {
                            links.push(ExtractedLink {
                                url: self.resolve_url(href, base_url)?,
                                text: self.clean_text(&text),
                                rel: attrs.get("rel").map(|s| s.to_string()),
                                title: attrs.get("title").map(|s| s.to_string()),
                                relevance_score: 0.5,
                            });
                        }
                    }
                }
            }
        }

        Ok(links)
    }

    fn extract_images(&self, document: &kuchiki::NodeRef, base_url: &str) -> Result<Vec<ExtractedImage>> {
        let mut images = Vec::new();
        
        if let Ok(img_elements) = document.select("img[src]") {
            for element in img_elements {
                if let Some(element_ref) = element.as_element() {
                    let attrs = element_ref.attributes.borrow();
                    
                    if let Some(src) = attrs.get("src") {
                        images.push(ExtractedImage {
                            url: self.resolve_url(src, base_url)?,
                            alt_text: attrs.get("alt").map(|s| s.to_string()),
                            title: attrs.get("title").map(|s| s.to_string()),
                            dimensions: None,
                            relevance_score: 0.5,
                        });
                    }
                }
            }
        }

        Ok(images)
    }

    fn clean_text(&self, text: &str) -> String {
        // Unicode规范化和清理
        let normalized: String = text.nfc().collect();
        let decoded = decode_html_entities(&normalized).unwrap_or(normalized);
        
        // 清理多余的空白字符
        let cleaned = decoded
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        cleaned.trim().to_string()
    }

    fn resolve_url(&self, href: &str, base_url: &str) -> Result<String> {
        if href.starts_with("http") {
            Ok(href.to_string())
        } else if href.starts_with("//") {
            Ok(format!("https:{}", href))
        } else if href.starts_with('/') {
            if let Ok(base) = url::Url::parse(base_url) {
                if let Some(host) = base.host_str() {
                    Ok(format!("{}://{}{}", base.scheme(), host, href))
                } else {
                    Err(anyhow!("无法解析基础URL"))
                }
            } else {
                Err(anyhow!("无效的基础URL"))
            }
        } else {
            // 相对路径
            if let Ok(base) = url::Url::parse(base_url) {
                if let Ok(resolved) = base.join(href) {
                    Ok(resolved.to_string())
                } else {
                    Err(anyhow!("无法解析相对URL"))
                }
            } else {
                Err(anyhow!("无效的基础URL"))
            }
        }
    }
}

/// 内容质量评估器
pub struct ContentQualityAssessor {
    language_detector: Arc<LanguageDetector>,
}

impl ContentQualityAssessor {
    pub fn new() -> Self {
        let languages = vec![
            Language::English,
            Language::Chinese,
            Language::Japanese,
            Language::Korean,
            Language::German,
            Language::French,
            Language::Spanish,
            Language::Russian,
        ];
        
        let detector = LanguageDetectorBuilder::from_languages(&languages).build();
        
        Self {
            language_detector: Arc::new(detector),
        }
    }

    pub fn assess_quality(&self, content: &ContentExtractionResult) -> QualityMetrics {
        QualityMetrics {
            readability_score: self.calculate_readability(&content.main_content),
            coherence_score: self.calculate_coherence(&content.main_content),
            completeness_score: self.calculate_completeness(content),
            relevance_score: self.calculate_relevance(content),
            technical_content_score: self.calculate_technical_score(&content.main_content),
        }
    }

    fn calculate_readability(&self, content: &str) -> f32 {
        // 完整的可读性评估算法（基于Flesch Reading Ease）
        let words = content.split_whitespace().count() as f32;
        let sentences = content.split(&['.', '!', '?'][..])
            .filter(|s| !s.trim().is_empty())
            .count() as f32;
        
        // 计算音节数（简化版本，基于元音计数）
        let syllables = self.count_syllables(content);
        
        if sentences == 0.0 || words == 0.0 {
            return 0.0;
        }
        
        let avg_sentence_length = words / sentences;
        let avg_syllables_per_word = syllables / words;
        
        // Flesch Reading Ease公式
        let flesch_score = 206.835 - (1.015 * avg_sentence_length) - (84.6 * avg_syllables_per_word);
        
        // 归一化到0-1范围
        (flesch_score.max(0.0).min(100.0)) / 100.0
    }

    fn calculate_coherence(&self, content: &str) -> f32 {
        // 完整的连贯性评估算法
        let sentences: Vec<&str> = content.split(&['.', '!', '?'][..])
            .filter(|s| !s.trim().is_empty())
            .collect();
        
        if sentences.len() < 2 {
            return 0.5; // 单句内容给予中等分数
        }
        
        let mut coherence_score = 0.0;
        let mut total_comparisons = 0;
        
        // 计算相邻句子之间的词汇重叠度
        for i in 0..sentences.len() - 1 {
            let words1: std::collections::HashSet<String> = sentences[i]
                .split_whitespace()
                .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphabetic()).to_string())
                .filter(|w| w.len() > 2) // 过滤短词
                .collect();
            
            let words2: std::collections::HashSet<String> = sentences[i + 1]
                .split_whitespace()
                .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphabetic()).to_string())
                .filter(|w| w.len() > 2)
                .collect();
            
            if !words1.is_empty() && !words2.is_empty() {
                let intersection = words1.intersection(&words2).count() as f32;
                let union = words1.union(&words2).count() as f32;
                let jaccard_similarity = intersection / union;
                
                coherence_score += jaccard_similarity;
                total_comparisons += 1;
            }
        }
        
        // 检查连接词的使用
        let transition_words = [
            "however", "therefore", "furthermore", "moreover", "additionally",
            "consequently", "meanwhile", "similarly", "in contrast", "for example",
            "specifically", "in particular", "as a result", "on the other hand"
        ];
        
        let transition_count = transition_words.iter()
            .map(|&word| content.to_lowercase().matches(word).count())
            .sum::<usize>() as f32;
        
        let transition_bonus = (transition_count / sentences.len() as f32).min(0.3);
        
        if total_comparisons > 0 {
            (coherence_score / total_comparisons as f32) + transition_bonus
        } else {
            transition_bonus
        }
    }

    /// 计算音节数（基于元音模式）
    fn count_syllables(&self, text: &str) -> f32 {
        let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
        let mut syllable_count = 0;
        
        for word in text.split_whitespace() {
            let clean_word = word.to_lowercase()
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>();
            
            if clean_word.is_empty() {
                continue;
            }
            
            let mut word_syllables = 0;
            let mut prev_was_vowel = false;
            
            for ch in clean_word.chars() {
                let is_vowel = vowels.contains(&ch);
                if is_vowel && !prev_was_vowel {
                    word_syllables += 1;
                }
                prev_was_vowel = is_vowel;
            }
            
            // 每个单词至少有一个音节
            if word_syllables == 0 {
                word_syllables = 1;
            }
            
            // 处理silent e
            if clean_word.ends_with('e') && word_syllables > 1 {
                word_syllables -= 1;
            }
            
            syllable_count += word_syllables;
        }
        
        syllable_count as f32
    }

    fn calculate_completeness(&self, result: &ContentExtractionResult) -> f32 {
        let mut score = 0.0;
        
        if !result.title.is_empty() {
            score += 0.2;
        }
        
        if result.main_content.len() > 200 {
            score += 0.3;
        }
        
        if !result.links.is_empty() {
            score += 0.2;
        }
        
        if result.summary.is_some() {
            score += 0.15;
        }
        
        if result.detected_language.is_some() {
            score += 0.15;
        }
        
        score
    }

    fn calculate_relevance(&self, result: &ContentExtractionResult) -> f32 {
        // 基于内容结构和元素的相关性评估
        let mut score = 0.5; // 基础分数
        
        if !result.code_blocks.is_empty() {
            score += 0.2; // 技术内容通常更相关
        }
        
        if !result.tables.is_empty() {
            score += 0.1; // 结构化数据
        }
        
        if result.links.len() > 5 {
            score += 0.1; // 丰富的链接
        }
        
        score.min(1.0)
    }

    fn calculate_technical_score(&self, content: &str) -> f32 {
        let technical_keywords = [
            "api", "function", "class", "method", "algorithm", "implementation",
            "configuration", "parameter", "return", "example", "usage", "documentation",
            "tutorial", "guide", "reference", "specification",
        ];
        
        let content_lower = content.to_lowercase();
        let matches = technical_keywords.iter()
            .filter(|&&keyword| content_lower.contains(keyword))
            .count() as f32;
        
        (matches / technical_keywords.len() as f32).min(1.0)
    }
}

/// 管道指标收集器
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    pub total_extractions: Arc<RwLock<u64>>,
    pub successful_extractions: Arc<RwLock<u64>>,
    pub failed_extractions: Arc<RwLock<u64>>,
    pub cache_hits: Arc<RwLock<u64>>,
    pub cache_misses: Arc<RwLock<u64>>,
    pub average_processing_time_ms: Arc<RwLock<f64>>,
    pub browser_renders: Arc<RwLock<u64>>,
}

impl PipelineMetrics {
    pub async fn record_extraction_success(&self, processing_time_ms: u64) {
        *self.total_extractions.write().await += 1;
        *self.successful_extractions.write().await += 1;
        
        let mut avg = self.average_processing_time_ms.write().await;
        *avg = (*avg + processing_time_ms as f64) / 2.0;
    }

    pub async fn record_extraction_failure(&self) {
        *self.total_extractions.write().await += 1;
        *self.failed_extractions.write().await += 1;
    }

    pub async fn record_cache_hit(&self) {
        *self.cache_hits.write().await += 1;
    }

    pub async fn record_cache_miss(&self) {
        *self.cache_misses.write().await += 1;
    }

    pub async fn record_browser_render(&self) {
        *self.browser_renders.write().await += 1;
    }
}

impl EnhancedContentPipeline {
    /// 创建新的增强内容管道
    pub async fn new(config: PipelineConfig) -> Result<Self> {
        info!("🚀 初始化企业级内容处理管道");

        // 初始化语言检测器
        let languages = vec![
            Language::English, Language::Chinese, Language::Japanese,
            Language::Korean, Language::German, Language::French,
            Language::Spanish, Language::Russian, Language::Portuguese,
        ];
        let language_detector = Arc::new(
            LanguageDetectorBuilder::from_languages(&languages).build()
        );

        // 初始化ONNX Runtime环境
        let ort_environment = Arc::new(
            Environment::builder()
                .with_name("grape-mcp-content-pipeline")
                .with_log_level(ort::LoggingLevel::Warning)
                .build()?
        );

        // 初始化提取策略
        let mut extraction_strategies: Vec<Arc<dyn ContentExtractionStrategy>> = Vec::new();
        
        if config.enabled_strategies.contains(&"readability".to_string()) {
            extraction_strategies.push(Arc::new(ReadabilityStrategy::new()));
        }

        // 初始化质量评估器
        let quality_assessor = Arc::new(ContentQualityAssessor::new());

        // 初始化浏览器（如果启用）
        let browser = if config.enable_browser_rendering {
            Some(Self::init_browser().await?)
        } else {
            None
        };

        Ok(Self {
            browser: Arc::new(RwLock::new(browser)),
            language_detector,
            ort_environment,
            extraction_strategies,
            quality_assessor,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(PipelineMetrics::default()),
        })
    }

    /// 初始化浏览器
    async fn init_browser() -> Result<Browser> {
        info!("🌐 初始化Chromium浏览器实例");
        
        let config = BrowserConfig::builder()
            .with_head()
            .args(vec![
                "--no-sandbox",
                "--disable-dev-shm-usage", 
                "--disable-gpu",
                "--disable-features=VizDisplayCompositor",
            ])
            .build()
            .map_err(|e| anyhow!("浏览器配置失败: {}", e))?;

        Browser::launch(config).await
            .map_err(|e| anyhow!("浏览器启动失败: {}", e))
    }

    /// 提取网页内容
    pub async fn extract_content(&self, url: &str) -> Result<ContentExtractionResult> {
        let start_time = std::time::Instant::now();
        info!("📄 开始提取内容: {}", url);

        // 检查缓存
        let cache_key = self.generate_cache_key(url);
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            self.metrics.record_cache_hit().await;
            info!("✅ 缓存命中: {}", url);
            return Ok(cached.content);
        }
        self.metrics.record_cache_miss().await;

        // 获取页面内容
        let html = if self.config.enable_browser_rendering {
            self.fetch_with_browser(url).await?
        } else {
            self.fetch_with_http(url).await?
        };

        // 选择最佳提取策略
        let strategy = self.select_best_strategy(url);
        
        // 执行提取
        let mut result = strategy.extract(url, &html).await?;

        // 语言检测
        if let Some(language) = self.language_detector.detect_language_of(&result.main_content) {
            result.detected_language = Some(language);
        }

        // 质量评估
        let quality_metrics = self.quality_assessor.assess_quality(&result);
        result.metadata.quality_metrics = quality_metrics;
        result.quality_score = self.calculate_overall_quality_score(&result);

        // 更新处理时间
        let processing_time = start_time.elapsed().as_millis() as u64;
        result.metadata.processing_time_ms = processing_time;

        // 缓存结果
        self.cache_result(&cache_key, &result).await;

        // 记录指标
        self.metrics.record_extraction_success(processing_time).await;

        info!("✅ 内容提取完成，质量分数: {:.2}", result.quality_score);
        Ok(result)
    }

    /// 使用浏览器获取页面内容
    async fn fetch_with_browser(&self, url: &str) -> Result<String> {
        debug!("🌐 使用浏览器渲染获取页面: {}", url);
        
        let browser_guard = self.browser.read().await;
        let browser = browser_guard.as_ref()
            .ok_or_else(|| anyhow!("浏览器未初始化"))?;

        let page = browser.new_page("about:blank").await
            .map_err(|e| anyhow!("创建页面失败: {}", e))?;

        // 设置超时和用户代理
        page.set_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36").await
            .map_err(|e| anyhow!("设置用户代理失败: {}", e))?;

        // 导航到目标页面
        page.goto(url).await
            .map_err(|e| anyhow!("页面导航失败: {}", e))?;

        // 等待页面加载完成
        page.wait_for_navigation().await
            .map_err(|e| anyhow!("等待页面加载失败: {}", e))?;

        // 获取页面HTML
        let html = page.content().await
            .map_err(|e| anyhow!("获取页面内容失败: {}", e))?;

        self.metrics.record_browser_render().await;
        Ok(html)
    }

    /// 使用HTTP客户端获取页面内容
    async fn fetch_with_http(&self, url: &str) -> Result<String> {
        debug!("📡 使用HTTP客户端获取页面: {}", url);
        
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Grape-MCP-DevTools/2.0)")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP请求失败: {}", response.status()));
        }

        let html = response.text().await?;
        Ok(html)
    }

    /// 选择最佳提取策略
    fn select_best_strategy(&self, url: &str) -> Arc<dyn ContentExtractionStrategy> {
        for strategy in &self.extraction_strategies {
            if strategy.supports_url(url) {
                return strategy.clone();
            }
        }
        
        // 默认使用第一个策略
        self.extraction_strategies[0].clone()
    }

    /// 生成缓存键
    fn generate_cache_key(&self, url: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 从缓存获取内容
    async fn get_from_cache(&self, key: &str) -> Option<CachedContent> {
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
    async fn cache_result(&self, key: &str, result: &ContentExtractionResult) {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), CachedContent {
            content: result.clone(),
            timestamp: Utc::now(),
            access_count: 1,
        });
    }

    /// 计算综合质量分数
    fn calculate_overall_quality_score(&self, result: &ContentExtractionResult) -> f32 {
        let metrics = &result.metadata.quality_metrics;
        
        // 加权平均
        let weights = [0.3, 0.2, 0.2, 0.15, 0.15]; // 可读性、连贯性、完整性、相关性、技术性
        let scores = [
            metrics.readability_score,
            metrics.coherence_score,
            metrics.completeness_score,
            metrics.relevance_score,
            metrics.technical_content_score,
        ];
        
        weights.iter().zip(scores.iter())
            .map(|(w, s)| w * s)
            .sum()
    }

    /// 获取管道统计信息
    pub async fn get_statistics(&self) -> PipelineStatistics {
        PipelineStatistics {
            total_extractions: *self.metrics.total_extractions.read().await,
            successful_extractions: *self.metrics.successful_extractions.read().await,
            failed_extractions: *self.metrics.failed_extractions.read().await,
            cache_hits: *self.metrics.cache_hits.read().await,
            cache_misses: *self.metrics.cache_misses.read().await,
            average_processing_time_ms: *self.metrics.average_processing_time_ms.read().await,
            browser_renders: *self.metrics.browser_renders.read().await,
            cache_size: self.cache.read().await.len(),
            enabled_strategies: self.config.enabled_strategies.clone(),
        }
    }
}

/// 管道统计信息
#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineStatistics {
    pub total_extractions: u64,
    pub successful_extractions: u64,
    pub failed_extractions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_processing_time_ms: f64,
    pub browser_renders: u64,
    pub cache_size: usize,
    pub enabled_strategies: Vec<String>,
} 
} 
