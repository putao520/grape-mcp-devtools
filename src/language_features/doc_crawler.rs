use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;
use tracing::{info, debug};
use chrono::{DateTime, Utc};
use url::Url;
use regex;
use tokio::sync::RwLock;
use std::sync::Arc;

use super::intelligent_scraper::{IntelligentScraper, ContentType, ScrapeResult};
use super::content_analyzer::ChangelogAnalyzer;

/// AI驱动的文档爬取和识别系统
pub struct DocCrawlerEngine {
    http_client: Client,
    scraper: Arc<IntelligentScraper>,
    analyzer: Arc<ChangelogAnalyzer>,
    doc_cache: Arc<RwLock<HashMap<String, CachedDocContent>>>,
    config: DocCrawlerConfig,
}

/// 文档爬取配置
#[derive(Debug, Clone)]
pub struct DocCrawlerConfig {
    /// 最大爬取深度
    pub max_crawl_depth: usize,
    /// 每个库的最大页面数
    pub max_pages_per_library: usize,
    /// 并发限制
    pub concurrent_limit: usize,
    /// 缓存TTL
    pub cache_ttl_hours: u64,
    /// AI分析启用
    pub enable_ai_analysis: bool,
    /// 内容质量阈值
    pub content_quality_threshold: f32,
}

impl Default for DocCrawlerConfig {
    fn default() -> Self {
        Self {
            max_crawl_depth: 3,
            max_pages_per_library: 50,
            concurrent_limit: 5,
            cache_ttl_hours: 24,
            enable_ai_analysis: true,
            content_quality_threshold: 0.7,
        }
    }
}

/// 缓存的文档内容
#[derive(Debug, Clone)]
struct CachedDocContent {
    content: LibraryDocumentation,
    timestamp: DateTime<Utc>,
    quality_score: f32,
}

/// 库文档信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibraryDocumentation {
    /// 库名称
    pub library_name: String,
    /// 语言
    pub language: String,
    /// 版本
    pub version: Option<String>,
    /// 描述
    pub description: String,
    /// 主要特性
    pub features: Vec<LibraryFeature>,
    /// API文档
    pub api_documentation: Vec<ApiDoc>,
    /// 教程和指南
    pub tutorials: Vec<Tutorial>,
    /// 代码示例
    pub examples: Vec<LibraryCodeExample>,
    /// 安装指南
    pub installation: Option<InstallationGuide>,
    /// 依赖信息
    pub dependencies: Vec<Dependency>,
    /// 元数据
    pub metadata: DocMetadata,
}

/// 库特性
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibraryFeature {
    pub name: String,
    pub description: String,
    pub category: String,
    pub maturity: FeatureMaturity,
    pub code_examples: Vec<String>,
    pub documentation_urls: Vec<String>,
}

/// 特性成熟度
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FeatureMaturity {
    Experimental,
    Beta,
    Stable,
    Deprecated,
}

/// API文档
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiDoc {
    pub module_name: String,
    pub functions: Vec<FunctionDoc>,
    pub classes: Vec<ClassDoc>,
    pub types: Vec<TypeDoc>,
    pub constants: Vec<ConstantDoc>,
}

/// 函数文档
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionDoc {
    pub name: String,
    pub signature: String,
    pub description: String,
    pub parameters: Vec<ParameterDoc>,
    pub return_type: Option<String>,
    pub examples: Vec<String>,
    pub source_url: Option<String>,
}

/// 参数文档
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParameterDoc {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub optional: bool,
    pub default_value: Option<String>,
}

/// 类文档
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassDoc {
    pub name: String,
    pub description: String,
    pub methods: Vec<FunctionDoc>,
    pub properties: Vec<PropertyDoc>,
    pub inheritance: Vec<String>,
    pub source_url: Option<String>,
}

/// 属性文档
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PropertyDoc {
    pub name: String,
    pub prop_type: String,
    pub description: String,
    pub readable: bool,
    pub writable: bool,
}

/// 类型文档
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeDoc {
    pub name: String,
    pub description: String,
    pub type_definition: String,
    pub usage_examples: Vec<String>,
}

/// 常量文档
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstantDoc {
    pub name: String,
    pub value: String,
    pub description: String,
    pub const_type: String,
}

/// 教程
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tutorial {
    pub title: String,
    pub difficulty: TutorialDifficulty,
    pub description: String,
    pub content: String,
    pub code_examples: Vec<String>,
    pub duration_minutes: Option<u32>,
    pub source_url: String,
}

/// 教程难度
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TutorialDifficulty {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

/// 代码示例
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibraryCodeExample {
    pub title: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub category: String,
    pub complexity: ExampleComplexity,
    pub source_url: Option<String>,
}

/// 示例复杂度
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExampleComplexity {
    Simple,
    Moderate,
    Complex,
    Advanced,
}

/// 安装指南
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallationGuide {
    pub package_managers: HashMap<String, String>,
    pub manual_installation: Option<String>,
    pub system_requirements: Vec<String>,
    pub optional_dependencies: Vec<String>,
    pub configuration_steps: Vec<String>,
}

/// 依赖
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_requirement: String,
    pub dependency_type: DependencyType,
    pub optional: bool,
    pub description: Option<String>,
}

/// 依赖类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DependencyType {
    Runtime,
    Development,
    Build,
    Optional,
    Peer,
}

/// 文档元数据
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocMetadata {
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation_url: Option<String>,
    pub license: Option<String>,
    pub maintainers: Vec<String>,
    pub last_updated: DateTime<Utc>,
    pub source_urls: Vec<String>,
    pub quality_score: f32,
    pub completeness_score: f32,
}

impl DocCrawlerEngine {
    pub async fn new(
        http_client: Client,
        scraper: Arc<IntelligentScraper>,
        analyzer: Arc<ChangelogAnalyzer>,
        config: DocCrawlerConfig,
    ) -> Result<Self> {
        Ok(Self {
            http_client,
            scraper,
            analyzer,
            doc_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// 爬取库文档
    pub async fn crawl_library_documentation(&self, library_name: &str, language: &str, base_urls: Vec<String>) -> Result<LibraryDocumentation> {
        info!("🕷️ 开始爬取库文档: {} ({})", library_name, language);

        // 检查缓存
        let cache_key = format!("{}:{}", language, library_name);
        if let Some(cached) = self.get_cached_doc(&cache_key).await {
            if cached.quality_score >= self.config.content_quality_threshold {
                info!("🎯 使用高质量缓存文档");
                return Ok(cached.content);
            }
        }

        // 发现相关URL
        let mut discovered_urls = Vec::new();
        for base_url in &base_urls {
            // 由于Arc的限制，我们使用简化的URL发现逻辑
            let urls = self.discover_simple_urls(base_url, language).await?;
            discovered_urls.extend(urls);
        }

        // 智能URL扩展
        let expanded_urls = self.discover_library_specific_urls(library_name, language, &base_urls).await?;
        discovered_urls.extend(expanded_urls);

        // 爬取和分析内容
        let mut documentation = LibraryDocumentation {
            library_name: library_name.to_string(),
            language: language.to_string(),
            version: None,
            description: String::new(),
            features: Vec::new(),
            api_documentation: Vec::new(),
            tutorials: Vec::new(),
            examples: Vec::new(),
            installation: None,
            dependencies: Vec::new(),
            metadata: DocMetadata {
                homepage: None,
                repository: None,
                documentation_url: None,
                license: None,
                maintainers: Vec::new(),
                last_updated: Utc::now(),
                source_urls: discovered_urls.clone(),
                quality_score: 0.0,
                completeness_score: 0.0,
            },
        };

        // 并发爬取内容
        let mut tasks = Vec::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.concurrent_limit));
        
        for url in discovered_urls.iter().take(self.config.max_pages_per_library) {
            let sem = semaphore.clone();
            let scraper = self.scraper.clone();
            let analyzer = self.analyzer.clone();
            let url = url.clone();
            let language = language.to_string();

            tasks.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                Self::crawl_single_page(scraper, analyzer, &url, &language).await
            }));
        }

        // 收集结果
        let mut page_results = Vec::new();
        for task in tasks {
            if let Ok(Ok(result)) = task.await {
                page_results.push(result);
            }
        }

        // 分析和整合内容
        self.analyze_and_integrate_content(&mut documentation, page_results).await?;

        // 计算质量分数
        self.calculate_quality_scores(&mut documentation).await;

        // 缓存结果
        self.cache_documentation(&cache_key, &documentation).await;

        info!("✅ 完成库文档爬取，质量分数: {:.2}", documentation.metadata.quality_score);
        Ok(documentation)
    }

    /// 爬取单个页面
    async fn crawl_single_page(
        scraper: Arc<IntelligentScraper>,
        _analyzer: Arc<ChangelogAnalyzer>,
        url: &str,
        _language: &str,
    ) -> Result<PageAnalysisResult> {
        debug!("🔍 爬取页面: {}", url);

        let scrape_result = scraper.scrape_intelligent(url, &[]).await?;
        let content_type = scraper.detect_content_type(&scrape_result.content).await;

        Ok(PageAnalysisResult {
            url: url.to_string(),
            scrape_result,
            content_type,
        })
    }

    /// 发现库特定URL
    async fn discover_library_specific_urls(&self, library_name: &str, language: &str, base_urls: &[String]) -> Result<Vec<String>> {
        let mut urls = Vec::new();

        for base_url in base_urls {
            if let Ok(parsed_url) = Url::parse(base_url) {
                if let Some(host) = parsed_url.host_str() {
                    let base_scheme = parsed_url.scheme();
                    let base = format!("{}://{}", base_scheme, host);

                    // 生成库特定路径
                    let library_paths = vec![
                        format!("/{}", library_name),
                        format!("/{}/docs", library_name),
                        format!("/{}/documentation", library_name),
                        format!("/{}/api", library_name),
                        format!("/{}/guide", library_name),
                        format!("/{}/tutorial", library_name),
                        format!("/{}/examples", library_name),
                        format!("/docs/{}", library_name),
                        format!("/documentation/{}", library_name),
                        format!("/api/{}", library_name),
                        format!("/reference/{}", library_name),
                    ];

                    for path in library_paths {
                        let potential_url = format!("{}{}", base, path);
                        if self.url_exists(&potential_url).await {
                            urls.push(potential_url);
                        }
                    }

                    // 包管理器特定URL
                    urls.extend(self.generate_package_manager_urls(library_name, language).await);
                }
            }
        }

        Ok(urls)
    }

    /// 生成包管理器URL
    async fn generate_package_manager_urls(&self, library_name: &str, language: &str) -> Vec<String> {
        let mut urls = Vec::new();

        match language {
            "javascript" | "typescript" => {
                urls.push(format!("https://www.npmjs.com/package/{}", library_name));
                urls.push(format!("https://unpkg.com/{}", library_name));
            }
            "python" => {
                urls.push(format!("https://pypi.org/project/{}", library_name));
            }
            "rust" => {
                urls.push(format!("https://crates.io/crates/{}", library_name));
                urls.push(format!("https://docs.rs/{}", library_name));
            }
            "java" => {
                // Maven Central
                urls.push(format!("https://search.maven.org/search?q=a:{}", library_name));
            }
            "go" => {
                urls.push(format!("https://pkg.go.dev/{}", library_name));
            }
            _ => {}
        }

        urls
    }

    /// 分析和整合内容
    async fn analyze_and_integrate_content(&self, documentation: &mut LibraryDocumentation, page_results: Vec<PageAnalysisResult>) -> Result<()> {
        for page_result in page_results {
            match page_result.content_type {
                ContentType::Documentation => {
                    self.extract_api_documentation(&page_result, documentation).await?;
                }
                ContentType::BlogPost => {
                    self.extract_tutorials(&page_result, documentation).await?;
                }
                _ => {
                    self.extract_general_content(&page_result, documentation).await?;
                }
            }
        }

        Ok(())
    }

    /// 提取API文档
    async fn extract_api_documentation(&self, page_result: &PageAnalysisResult, documentation: &mut LibraryDocumentation) -> Result<()> {
        debug!("📝 提取API文档: {}", page_result.url);
        
        let content = &page_result.scrape_result.content;
        let mut api_doc = ApiDoc {
            module_name: self.extract_module_name(&page_result.url, content),
            functions: Vec::new(),
            classes: Vec::new(),
            types: Vec::new(),
            constants: Vec::new(),
        };

        // 提取函数文档
        api_doc.functions = self.extract_functions_from_content(content, &page_result.url);
        
        // 提取类/结构体文档
        api_doc.classes = self.extract_classes_from_content(content, &page_result.url);
        
        // 提取类型定义
        api_doc.types = self.extract_types_from_content(content);
        
        // 提取常量
        api_doc.constants = self.extract_constants_from_content(content);
        
        documentation.api_documentation.push(api_doc);
        Ok(())
    }

    /// 从URL和内容中提取模块名称
    fn extract_module_name(&self, url: &str, content: &str) -> String {
        // 尝试从URL路径提取模块名
        if let Ok(parsed_url) = Url::parse(url) {
            let path_segments: Vec<&str> = parsed_url.path().split('/').filter(|s| !s.is_empty()).collect();
            if let Some(last_segment) = path_segments.last() {
                if !last_segment.contains('.') {
                    return last_segment.to_string();
                }
            }
        }
        
        // 尝试从内容中提取模块/包名
        let module_patterns = [
            r"module\s+([a-zA-Z_][a-zA-Z0-9_]*)",      // Go, Rust等
            r"package\s+([a-zA-Z_][a-zA-Z0-9_\.]*)",    // Java, Go
            r"class\s+([A-Z][a-zA-Z0-9_]*)",           // Python, Java等
            r"namespace\s+([a-zA-Z_][a-zA-Z0-9_]*)",   // C#, C++
        ];
        
        for pattern in &module_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(content) {
                    if let Some(module_name) = captures.get(1) {
                        return module_name.as_str().to_string();
                    }
                }
            }
        }
        
        "main".to_string()
    }

    /// 从内容中提取函数文档
    fn extract_functions_from_content(&self, content: &str, source_url: &str) -> Vec<FunctionDoc> {
        let mut functions = Vec::new();
        
        // 匹配各种语言的函数定义模式
        let function_patterns = [
            // Python: def function_name(params):
            r"def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*(?:->([^:]*))?\s*:",
            // JavaScript/TypeScript: function name(params) 或 name(params) =>
            r"(?:function\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*(?::\s*([^{=]+))?\s*[{=]",
            // Go: func name(params) returnType
            r"func\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*([^{]*)\s*{",
            // Rust: fn name(params) -> ReturnType
            r"fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*(?:->\s*([^{]+))?\s*{",
            // Java: public/private returnType name(params)
            r"(?:public|private|protected)?\s*(?:static)?\s*([a-zA-Z_][a-zA-Z0-9_<>]*)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)",
        ];

        for pattern in &function_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for captures in re.captures_iter(content) {
                    let name = captures.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let params_str = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                    let return_type = captures.get(3).map(|m| m.as_str().trim().to_string());
                    
                    if !name.is_empty() {
                        let parameters = self.parse_parameters(params_str);
                        let signature = if let Some(ret_type) = &return_type {
                            format!("{}({}) -> {}", name, params_str, ret_type)
                        } else {
                            format!("{}({})", name, params_str)
                        };
                        
                        functions.push(FunctionDoc {
                            name,
                            signature,
                            description: self.extract_function_description(content, &captures.get(0).unwrap().as_str()),
                            parameters,
                            return_type,
                            examples: Vec::new(),
                            source_url: Some(source_url.to_string()),
                        });
                    }
                }
            }
        }
        
        functions
    }

    /// 解析参数列表
    fn parse_parameters(&self, params_str: &str) -> Vec<ParameterDoc> {
        if params_str.trim().is_empty() {
            return Vec::new();
        }
        
        params_str.split(',')
            .map(|param| {
                let param = param.trim();
                let parts: Vec<&str> = param.split(':').collect();
                
                if parts.len() >= 2 {
                    // TypeScript/Python style: name: type
                    ParameterDoc {
                        name: parts[0].trim().to_string(),
                        param_type: parts[1].trim().to_string(),
                        description: String::new(),
                        optional: param.contains('?') || param.contains("Optional"),
                        default_value: None,
                    }
                } else {
                    // Simple parameter
                    ParameterDoc {
                        name: param.to_string(),
                        param_type: "unknown".to_string(),
                        description: String::new(),
                        optional: false,
                        default_value: None,
                    }
                }
            })
            .collect()
    }

    /// 提取函数描述
    fn extract_function_description(&self, content: &str, function_match: &str) -> String {
        // 尝试找到函数前的注释或文档字符串
        if let Some(func_pos) = content.find(function_match) {
            let before_func = &content[..func_pos];
            let lines: Vec<&str> = before_func.lines().rev().take(10).collect();
            
            for line in lines {
                let trimmed = line.trim();
                if trimmed.starts_with("///") || trimmed.starts_with("/**") || 
                   trimmed.starts_with("\"\"\"") || trimmed.starts_with("#") {
                    return trimmed.trim_start_matches(&['/', '*', '"', '#'][..]).trim().to_string();
                }
            }
        }
        
        String::new()
    }

    /// 从内容中提取类文档
    fn extract_classes_from_content(&self, content: &str, source_url: &str) -> Vec<ClassDoc> {
        let mut classes = Vec::new();
        
        // 匹配类定义模式
        let class_patterns = [
            r"class\s+([A-Z][a-zA-Z0-9_]*)\s*(?:\(([^)]*)\))?\s*:",  // Python
            r"class\s+([A-Z][a-zA-Z0-9_]*)\s*(?:extends\s+([a-zA-Z0-9_]+))?\s*{",  // JavaScript
            r"struct\s+([A-Z][a-zA-Z0-9_]*)\s*{",  // Rust, Go
            r"(?:public|private)?\s*class\s+([A-Z][a-zA-Z0-9_]*)\s*(?:extends\s+([a-zA-Z0-9_]+))?\s*{",  // Java
        ];

        for pattern in &class_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for captures in re.captures_iter(content) {
                    let name = captures.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let inheritance = captures.get(2).map(|m| vec![m.as_str().to_string()]).unwrap_or_default();
                    
                    if !name.is_empty() {
                        classes.push(ClassDoc {
                            name: name.clone(),
                            description: self.extract_class_description(content, &name),
                            methods: Vec::new(),  // 可以进一步实现方法提取
                            properties: Vec::new(),  // 可以进一步实现属性提取
                            inheritance,
                            source_url: Some(source_url.to_string()),
                        });
                    }
                }
            }
        }
        
        classes
    }

    /// 提取类描述
    fn extract_class_description(&self, content: &str, class_name: &str) -> String {
        if let Some(class_pos) = content.find(&format!("class {}", class_name)) {
            let before_class = &content[..class_pos];
            let lines: Vec<&str> = before_class.lines().rev().take(5).collect();
            
            for line in lines {
                let trimmed = line.trim();
                if trimmed.starts_with("///") || trimmed.starts_with("/**") || 
                   trimmed.starts_with("\"\"\"") || trimmed.starts_with("#") {
                    return trimmed.trim_start_matches(&['/', '*', '"', '#'][..]).trim().to_string();
                }
            }
        }
        
        String::new()
    }

    /// 从内容中提取类型定义
    fn extract_types_from_content(&self, content: &str) -> Vec<TypeDoc> {
        let mut types = Vec::new();
        
        // 匹配类型定义模式
        let type_patterns = [
            r"type\s+([A-Z][a-zA-Z0-9_]*)\s*=\s*([^;\n]+)",  // TypeScript, Go
            r"typedef\s+([^;]+)\s+([a-zA-Z_][a-zA-Z0-9_]*);",  // C/C++
            r"type\s+([A-Z][a-zA-Z0-9_]*)\s*=\s*([^;]+);",  // Rust
        ];

        for pattern in &type_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for captures in re.captures_iter(content) {
                    let name = captures.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let definition = captures.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
                    
                    if !name.is_empty() {
                        types.push(TypeDoc {
                            name,
                            description: String::new(),
                            type_definition: definition,
                            usage_examples: Vec::new(),
                        });
                    }
                }
            }
        }
        
        types
    }

    /// 从内容中提取常量
    fn extract_constants_from_content(&self, content: &str) -> Vec<ConstantDoc> {
        let mut constants = Vec::new();
        
        // 匹配常量定义模式
        let const_patterns = [
            r"const\s+([A-Z_][A-Z0-9_]*)\s*=\s*([^;\n]+)",  // JavaScript, Go
            r"#define\s+([A-Z_][A-Z0-9_]*)\s+([^\n]+)",    // C/C++
            r"([A-Z_][A-Z0-9_]*)\s*=\s*([^#\n]+)",         // Python
        ];

        for pattern in &const_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for captures in re.captures_iter(content) {
                    let name = captures.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let value = captures.get(2).map(|m| m.as_str().trim()).unwrap_or("").to_string();
                    
                    if !name.is_empty() && !value.is_empty() {
                        constants.push(ConstantDoc {
                            name,
                            value: value.clone(),
                            description: String::new(),
                            const_type: self.infer_constant_type(&value),
                        });
                    }
                }
            }
        }
        
        constants
    }

    /// 推断常量类型
    fn infer_constant_type(&self, value: &str) -> String {
        let value = value.trim();
        
        if value.starts_with('"') && value.ends_with('"') {
            "string".to_string()
        } else if value.parse::<i64>().is_ok() {
            "integer".to_string()
        } else if value.parse::<f64>().is_ok() {
            "float".to_string()
        } else if value == "true" || value == "false" {
            "boolean".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// 提取教程
    async fn extract_tutorials(&self, page_result: &PageAnalysisResult, documentation: &mut LibraryDocumentation) -> Result<()> {
        debug!("📚 提取教程: {}", page_result.url);
        
        let tutorial = Tutorial {
            title: page_result.scrape_result.title.clone(),
            difficulty: TutorialDifficulty::Beginner,
            description: page_result.scrape_result.content.chars().take(200).collect(),
            content: page_result.scrape_result.content.clone(),
            code_examples: Vec::new(),
            duration_minutes: None,
            source_url: page_result.url.clone(),
        };
        
        documentation.tutorials.push(tutorial);
        Ok(())
    }

    /// 提取通用内容
    async fn extract_general_content(&self, page_result: &PageAnalysisResult, documentation: &mut LibraryDocumentation) -> Result<()> {
        debug!("🔍 提取通用内容: {}", page_result.url);
        
        // 更新基本信息
        if documentation.description.is_empty() && !page_result.scrape_result.content.is_empty() {
            documentation.description = page_result.scrape_result.content.chars().take(500).collect();
        }
        
        Ok(())
    }

    /// 计算质量分数
    async fn calculate_quality_scores(&self, documentation: &mut LibraryDocumentation) {
        let mut quality_score = 0.0;
        let mut completeness_score = 0.0;

        // 基于内容丰富度计算质量分数
        if !documentation.description.is_empty() {
            quality_score += 0.2;
            completeness_score += 0.1;
        }

        if !documentation.api_documentation.is_empty() {
            quality_score += 0.3;
            completeness_score += 0.3;
        }

        if !documentation.tutorials.is_empty() {
            quality_score += 0.2;
            completeness_score += 0.2;
        }

        if !documentation.examples.is_empty() {
            quality_score += 0.2;
            completeness_score += 0.2;
        }

        if documentation.installation.is_some() {
            quality_score += 0.1;
            completeness_score += 0.2;
        }

        documentation.metadata.quality_score = quality_score;
        documentation.metadata.completeness_score = completeness_score;
    }

    /// 检查URL是否存在
    async fn url_exists(&self, url: &str) -> bool {
        match self.http_client.head(url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// 获取缓存文档
    async fn get_cached_doc(&self, key: &str) -> Option<CachedDocContent> {
        let cache = self.doc_cache.read().await;
        if let Some(cached) = cache.get(key) {
            let age = Utc::now().signed_duration_since(cached.timestamp);
            if age.num_hours() < self.config.cache_ttl_hours as i64 {
                return Some(cached.clone());
            }
        }
        None
    }

    /// 缓存文档
    async fn cache_documentation(&self, key: &str, documentation: &LibraryDocumentation) {
        let mut cache = self.doc_cache.write().await;
        cache.insert(key.to_string(), CachedDocContent {
            content: documentation.clone(),
            timestamp: Utc::now(),
            quality_score: documentation.metadata.quality_score,
        });
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> DocCacheStats {
        let cache = self.doc_cache.read().await;
        DocCacheStats {
            cached_libraries: cache.len(),
            total_cache_size: cache.values().map(|doc| doc.content.metadata.source_urls.len()).sum(),
            average_quality_score: if cache.is_empty() { 0.0 } else {
                cache.values().map(|doc| doc.quality_score).sum::<f32>() / cache.len() as f32
            },
        }
    }

    /// 智能URL发现
    async fn discover_simple_urls(&self, base_url: &str, language: &str) -> Result<Vec<String>> {
        let mut urls = vec![base_url.to_string()];
        
        if let Ok(parsed_url) = Url::parse(base_url) {
            if let Some(host) = parsed_url.host_str() {
                let base_scheme = parsed_url.scheme();
                let base = format!("{}://{}", base_scheme, host);
                
                // 根据语言添加特定的文档路径
                let language_specific_paths = self.get_language_specific_doc_paths(language);
                let common_doc_paths = vec![
                    "/docs".to_string(), "/documentation".to_string(), "/api".to_string(), "/reference".to_string(), 
                    "/guide".to_string(), "/tutorial".to_string(), "/examples".to_string(), "/readme".to_string(),
                    "/manual".to_string(), "/help".to_string(), "/wiki".to_string(), "/getting-started".to_string()
                ];
                
                // 合并所有可能的路径
                let mut all_paths = language_specific_paths.clone();
                all_paths.extend(common_doc_paths.clone());
                
                // 检查URL存在性（改为顺序检查避免生命周期问题）
                for path in &all_paths {
                    let potential_url = format!("{}{}", base, path);
                    if self.check_url_with_timeout(&potential_url).await {
                        urls.push(potential_url);
                    }
                }
                
                // 尝试发现子目录
                if let Ok(subpaths) = self.discover_subdirectories(&base, language).await {
                    urls.extend(subpaths);
                }
                
                // 尝试从robots.txt或sitemap.xml发现更多路径
                if let Ok(discovered_urls) = self.discover_from_robots_and_sitemap(&base).await {
                    urls.extend(discovered_urls);
                }
            }
        }
        
        // 去重并限制数量
        urls.sort();
        urls.dedup();
        urls.truncate(20); // 限制最多20个URL
        
        Ok(urls)
    }
    
    /// 获取特定语言的文档路径
    fn get_language_specific_doc_paths(&self, language: &str) -> Vec<String> {
        match language.to_lowercase().as_str() {
            "rust" => vec![
                "/rustdoc".to_string(),
                "/docs/rust".to_string(),
                "/doc".to_string(),
                "/book".to_string(),
            ],
            "python" => vec![
                "/docs/python".to_string(),
                "/py-modindex.html".to_string(),
                "/sphinx".to_string(),
                "/pydoc".to_string(),
            ],
            "javascript" | "typescript" => vec![
                "/docs/js".to_string(),
                "/jsdoc".to_string(),
                "/typedoc".to_string(),
                "/api/js".to_string(),
            ],
            "java" => vec![
                "/javadoc".to_string(),
                "/docs/java".to_string(),
                "/apidocs".to_string(),
            ],
            "go" => vec![
                "/godoc".to_string(),
                "/docs/go".to_string(),
                "/pkg".to_string(),
            ],
            "cpp" | "c++" => vec![
                "/doxygen".to_string(),
                "/docs/cpp".to_string(),
                "/reference".to_string(),
            ],
            _ => vec![]
        }
    }
    
    /// 带超时的URL检查
    async fn check_url_with_timeout(&self, url: &str) -> bool {
        let timeout = tokio::time::Duration::from_secs(5);
        
        match tokio::time::timeout(timeout, self.url_exists(url)).await {
            Ok(exists) => exists,
            Err(_) => false, // 超时视为不存在
        }
    }
    
    /// 发现子目录
    async fn discover_subdirectories(&self, base_url: &str, language: &str) -> Result<Vec<String>> {
        let mut discovered_urls = Vec::new();
        
        // 尝试常见的版本子目录
        let version_patterns = vec!["v1", "v2", "latest", "stable", "current"];
        for pattern in version_patterns {
            let url = format!("{}/docs/{}", base_url, pattern);
            if self.check_url_with_timeout(&url).await {
                discovered_urls.push(url);
            }
        }
        
        // 尝试语言特定的子目录
        let language_lower = language.to_lowercase();
        let lang_dirs = vec![language, &language_lower];
        for lang in lang_dirs {
            let url = format!("{}/docs/{}", base_url, lang);
            if self.check_url_with_timeout(&url).await {
                discovered_urls.push(url);
            }
        }
        
        Ok(discovered_urls)
    }
    
    /// 从robots.txt和sitemap.xml发现URL
    async fn discover_from_robots_and_sitemap(&self, base_url: &str) -> Result<Vec<String>> {
        let mut discovered_urls = Vec::new();
        
        // 尝试robots.txt
        if let Ok(robots_urls) = self.parse_robots_txt(base_url).await {
            discovered_urls.extend(robots_urls);
        }
        
        // 尝试sitemap.xml
        if let Ok(sitemap_urls) = self.parse_sitemap_xml(base_url).await {
            discovered_urls.extend(sitemap_urls);
        }
        
        Ok(discovered_urls)
    }
    
    /// 解析robots.txt文件
    async fn parse_robots_txt(&self, base_url: &str) -> Result<Vec<String>> {
        let robots_url = format!("{}/robots.txt", base_url);
        
        match self.http_client.get(&robots_url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(content) = response.text().await {
                    let mut urls = Vec::new();
                    
                    // 查找Sitemap指令
                    for line in content.lines() {
                        if line.to_lowercase().starts_with("sitemap:") {
                            if let Some(sitemap_url) = line.split(':').nth(1) {
                                let sitemap_url = sitemap_url.trim();
                                if sitemap_url.starts_with("http") {
                                    urls.push(sitemap_url.to_string());
                                }
                            }
                        }
                        // 查找Allow指令中的文档路径
                        else if line.to_lowercase().starts_with("allow:") {
                            if let Some(path) = line.split(':').nth(1) {
                                let path = path.trim();
                                if path.contains("doc") || path.contains("api") || path.contains("help") {
                                    urls.push(format!("{}{}", base_url, path));
                                }
                            }
                        }
                    }
                    
                    Ok(urls)
                } else {
                    Ok(Vec::new())
                }
            }
            _ => Ok(Vec::new())
        }
    }
    
    /// 解析sitemap.xml文件
    async fn parse_sitemap_xml(&self, base_url: &str) -> Result<Vec<String>> {
        let sitemap_url = format!("{}/sitemap.xml", base_url);
        
        match self.http_client.get(&sitemap_url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(content) = response.text().await {
                    let mut urls = Vec::new();
                    
                    // 简单的XML解析来提取<loc>标签
                    let loc_pattern = regex::Regex::new(r"<loc>\s*(.*?)\s*</loc>").unwrap();
                    
                    for captures in loc_pattern.captures_iter(&content) {
                        if let Some(url_match) = captures.get(1) {
                            let url = url_match.as_str();
                            // 只收集包含文档关键词的URL
                            if url.contains("doc") || url.contains("api") || 
                               url.contains("guide") || url.contains("tutorial") ||
                               url.contains("reference") || url.contains("help") {
                                urls.push(url.to_string());
                            }
                        }
                    }
                    
                    // 限制数量
                    urls.truncate(10);
                    Ok(urls)
                } else {
                    Ok(Vec::new())
                }
            }
            _ => Ok(Vec::new())
        }
    }
}

/// 页面分析结果
#[derive(Debug, Clone)]
struct PageAnalysisResult {
    url: String,
    scrape_result: ScrapeResult,
    content_type: ContentType,
}

/// 文档缓存统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocCacheStats {
    pub cached_libraries: usize,
    pub total_cache_size: usize,
    pub average_quality_score: f32,
} 