use anyhow::{anyhow, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, debug};
use regex::Regex;
use boa_engine::{Context, Source};

/// 智能爬虫，支持JavaScript渲染和内容识别
pub struct IntelligentScraper {
    http_client: Client,
    enable_js_rendering: bool,
    user_agents: Vec<String>,
    retry_config: RetryConfig,
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

/// 抓取结果
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub extracted_data: HashMap<String, Value>,
    pub links: Vec<String>,
    pub metadata: ScrapeMetadata,
}

/// 抓取元数据
#[derive(Debug, Clone)]
pub struct ScrapeMetadata {
    pub status_code: u16,
    pub content_type: String,
    pub content_length: Option<u64>,
    pub last_modified: Option<String>,
    pub server: Option<String>,
    pub encoding: String,
}

impl IntelligentScraper {
    pub async fn new(http_client: Client, enable_js_rendering: bool) -> Result<Self> {
        Ok(Self {
            http_client,
            enable_js_rendering,
            user_agents: Self::init_user_agents(),
            retry_config: RetryConfig::default(),
        })
    }

    /// 初始化用户代理列表
    fn init_user_agents() -> Vec<String> {
        vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Grape-MCP-DevTools/2.0 (Intelligent Scraper)".to_string(),
        ]
    }

    /// 智能抓取页面内容
    pub async fn scrape_intelligent(&self, url: &str, selectors: &[String]) -> Result<ScrapeResult> {
        info!("🕷️ 开始智能抓取: {}", url);

        let mut result = if self.enable_js_rendering {
            self.scrape_with_js_rendering(url).await?
        } else {
            self.scrape_static_html(url).await?
        };

        // 使用选择器提取特定内容
        if !selectors.is_empty() {
            result.extracted_data = self.extract_content_by_selectors(&result.content, selectors)?;
        }

        // 提取链接
        result.links = self.extract_links(&result.content, url)?;

        info!("✅ 智能抓取完成，内容长度: {} 字符", result.content.len());
        Ok(result)
    }

    /// 使用JavaScript渲染抓取页面
    async fn scrape_with_js_rendering(&self, url: &str) -> Result<ScrapeResult> {
        debug!("🌐 启用JavaScript执行抓取: {}", url);

        // 先获取静态HTML
        let mut result = self.scrape_static_html(url).await?;
        
        // 使用JavaScript引擎处理动态内容
        match self.execute_javascript_on_content(&result.content).await {
            Ok(enhanced_content) => {
                result.content = enhanced_content;
                info!("✅ JavaScript执行成功增强内容");
            }
            Err(e) => {
                warn!("⚠️ JavaScript执行失败，使用静态内容: {}", e);
            }
        }
        
        Ok(result)
    }

    /// 使用JavaScript引擎处理内容
    async fn execute_javascript_on_content(&self, html_content: &str) -> Result<String> {
        let mut context = Context::default();
        
        // 设置全局变量
        context.global_object()
            .set("htmlContent", html_content, false, &mut context)
            .map_err(|e| anyhow!("设置全局变量失败: {}", e))?;
        
        // 创建简化的DOM操作函数
        let js_code = r#"
            // 简化的文档解析器
            function parseDocument(html) {
                // 提取主要内容区域
                var mainContentRegex = /<main[^>]*>([\s\S]*?)<\/main>/i;
                var articleRegex = /<article[^>]*>([\s\S]*?)<\/article>/i;
                var contentRegex = /<div[^>]*class="[^"]*content[^"]*"[^>]*>([\s\S]*?)<\/div>/i;
                
                var content = html;
                
                // 尝试提取主要内容
                var match = content.match(mainContentRegex) || 
                           content.match(articleRegex) || 
                           content.match(contentRegex);
                
                if (match && match[1]) {
                    content = match[1];
                }
                
                // 清理HTML标签，保留文本内容
                content = content.replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '');
                content = content.replace(/<style[^>]*>[\s\S]*?<\/style>/gi, '');
                content = content.replace(/<[^>]+>/g, ' ');
                content = content.replace(/\s+/g, ' ');
                
                // 简单的trim操作
                while (content.charAt(0) === ' ') content = content.substring(1);
                while (content.charAt(content.length - 1) === ' ') content = content.substring(0, content.length - 1);
                
                return content;
            }
            
            // 处理主要逻辑
            var processedContent = parseDocument(htmlContent);
            processedContent;
        "#;
        
        // 执行JavaScript代码
        let source = Source::from_bytes(js_code);
        let result = context.eval(source)
            .map_err(|e| anyhow!("JavaScript执行失败: {}", e))?;
        
        // 获取字符串结果
        let processed_content = result.to_string(&mut context)
            .map_err(|e| anyhow!("JavaScript结果转换失败: {}", e))?;
        
        Ok(processed_content.to_std_string_escaped())
    }

    /// 静态HTML抓取
    async fn scrape_static_html(&self, url: &str) -> Result<ScrapeResult> {
        debug!("📄 静态HTML抓取: {}", url);

        let mut last_error = None;
        
        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                let delay = self.calculate_retry_delay(attempt);
                debug!("⏱️ 等待 {}ms 后重试 (第{}次)", delay, attempt);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            // 随机选择User-Agent
            let user_agent = &self.user_agents[attempt as usize % self.user_agents.len()];

            match self.fetch_page_content(url, user_agent).await {
                Ok(result) => {
                    info!("✅ 成功抓取页面 (第{}次尝试)", attempt + 1);
                    return Ok(result);
                }
                Err(e) => {
                    warn!("⚠️ 抓取失败 (第{}次尝试): {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("所有重试都失败了")))
    }

    /// 获取页面内容
    async fn fetch_page_content(&self, url: &str, user_agent: &str) -> Result<ScrapeResult> {
        let response = self.http_client
            .get(url)
            .header("User-Agent", user_agent)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        let status_code = response.status().as_u16();
        let headers = response.headers().clone();
        
        if !response.status().is_success() {
            return Err(anyhow!("HTTP错误: {}", status_code));
        }

        let content_type = headers.get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let content_length = headers.get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());

        let last_modified = headers.get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let server = headers.get("server")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let html_content = response.text().await?;
        let document = Html::parse_document(&html_content);

        // 提取页面标题
        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|element| element.text().collect::<String>())
            .unwrap_or_else(|| url.to_string());

        // 提取主要内容
        let content = self.extract_main_content(&document);

        Ok(ScrapeResult {
            url: url.to_string(),
            title: title.trim().to_string(),
            content,
            extracted_data: HashMap::new(),
            links: Vec::new(),
            metadata: ScrapeMetadata {
                status_code,
                content_type,
                content_length,
                last_modified,
                server,
                encoding: "utf-8".to_string(),
            },
        })
    }

    /// 提取页面主要内容
    fn extract_main_content(&self, document: &Html) -> String {
        // 尝试多种内容选择器
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
            "body",
        ];

        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if text.len() > 100 { // 确保内容足够长
                        debug!("✅ 使用选择器提取内容: {}", selector_str);
                        return self.clean_text(&text);
                    }
                }
            }
        }

        // 如果没有找到合适的内容，使用body
        let body_text = document
            .select(&Selector::parse("body").unwrap())
            .next()
            .map(|element| element.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();

        self.clean_text(&body_text)
    }

    /// 使用选择器提取特定内容
    fn extract_content_by_selectors(&self, html: &str, selectors: &[String]) -> Result<HashMap<String, Value>> {
        let document = Html::parse_document(html);
        let mut extracted = HashMap::new();

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                let elements: Vec<String> = document
                    .select(&selector)
                    .map(|element| {
                        let text = element.text().collect::<Vec<_>>().join(" ");
                        self.clean_text(&text)
                    })
                    .filter(|text| !text.is_empty())
                    .collect();

                if !elements.is_empty() {
                    extracted.insert(selector_str.clone(), json!(elements));
                }
            }
        }

        Ok(extracted)
    }

    /// 提取页面链接
    fn extract_links(&self, html: &str, base_url: &str) -> Result<Vec<String>> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a[href]").unwrap();
        
        let mut links = Vec::new();
        
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(absolute_url) = self.resolve_url(href, base_url) {
                    links.push(absolute_url);
                }
            }
        }

        // 去重
        links.sort();
        links.dedup();

        Ok(links)
    }

    /// 解析相对URL为绝对URL
    fn resolve_url(&self, href: &str, base_url: &str) -> Result<String> {
        if href.starts_with("http://") || href.starts_with("https://") {
            return Ok(href.to_string());
        }

        if href.starts_with("//") {
            let protocol = if base_url.starts_with("https") { "https:" } else { "http:" };
            return Ok(format!("{}{}", protocol, href));
        }

        if href.starts_with('/') {
            if let Ok(base) = url::Url::parse(base_url) {
                if let Some(host) = base.host_str() {
                    let scheme = base.scheme();
                    let port = base.port_or_known_default();
                    
                    let port_str = match port {
                        Some(80) | Some(443) => String::new(),
                        Some(p) => format!(":{}", p),
                        None => String::new(),
                    };
                    
                    return Ok(format!("{}://{}{}{}", scheme, host, port_str, href));
                }
            }
        }

        // 相对路径
        if let Ok(base) = url::Url::parse(base_url) {
            if let Ok(resolved) = base.join(href) {
                return Ok(resolved.to_string());
            }
        }

        Err(anyhow!("无法解析URL: {}", href))
    }

    /// 清理文本内容
    fn clean_text(&self, text: &str) -> String {
        // 移除多余的空白字符
        let re_whitespace = Regex::new(r"\s+").unwrap();
        let cleaned = re_whitespace.replace_all(text.trim(), " ");
        
        // 移除HTML实体
        let re_entity = Regex::new(r"&[a-zA-Z0-9#]+;").unwrap();
        let cleaned = re_entity.replace_all(&cleaned, " ");
        
        cleaned.trim().to_string()
    }

    /// 计算重试延迟
    fn calculate_retry_delay(&self, attempt: u32) -> u64 {
        let delay = self.retry_config.base_delay_ms as f64 
            * self.retry_config.backoff_multiplier.powi(attempt as i32 - 1);
        
        delay.min(self.retry_config.max_delay_ms as f64) as u64
    }

    /// 智能内容检测
    pub async fn detect_content_type(&self, content: &str) -> ContentType {
        // 检测changelog模式
        if self.is_changelog_content(content) {
            return ContentType::Changelog;
        }

        // 检测版本发布页面
        if self.is_release_page(content) {
            return ContentType::ReleasePage;
        }

        // 检测文档页面
        if self.is_documentation(content) {
            return ContentType::Documentation;
        }

        // 检测博客文章
        if self.is_blog_post(content) {
            return ContentType::BlogPost;
        }

        ContentType::Unknown
    }

    /// 检测是否为changelog内容
    fn is_changelog_content(&self, content: &str) -> bool {
        let changelog_indicators = [
            r"changelog",
            r"change log",
            r"version \d+\.\d+",
            r"release notes",
            r"what's new",
            r"breaking changes",
            r"## \[\d+\.\d+\.\d+\]",
            r"# Version \d+\.\d+",
        ];

        let content_lower = content.to_lowercase();
        changelog_indicators.iter().any(|pattern| {
            if let Ok(re) = Regex::new(pattern) {
                re.is_match(&content_lower)
            } else {
                content_lower.contains(&pattern.to_lowercase())
            }
        })
    }

    /// 检测是否为版本发布页面
    fn is_release_page(&self, content: &str) -> bool {
        let release_indicators = [
            r"release",
            r"download",
            r"latest version",
            r"current version",
            r"stable release",
        ];

        let content_lower = content.to_lowercase();
        release_indicators.iter()
            .filter(|&pattern| content_lower.contains(pattern))
            .count() >= 2
    }

    /// 检测是否为文档页面
    fn is_documentation(&self, content: &str) -> bool {
        let doc_indicators = [
            r"documentation",
            r"api reference",
            r"user guide",
            r"developer guide",
            r"getting started",
        ];

        let content_lower = content.to_lowercase();
        doc_indicators.iter().any(|&pattern| content_lower.contains(pattern))
    }

    /// 检测是否为博客文章
    fn is_blog_post(&self, content: &str) -> bool {
        let blog_indicators = [
            r"posted on",
            r"published",
            r"author:",
            r"by [a-zA-Z]+ [a-zA-Z]+",
            r"read more",
        ];

        let content_lower = content.to_lowercase();
        blog_indicators.iter().any(|pattern| {
            if let Ok(re) = Regex::new(pattern) {
                re.is_match(&content_lower)
            } else {
                content_lower.contains(&pattern.to_lowercase())
            }
        })
    }

    /// 批量抓取URL
    pub async fn scrape_batch(&self, urls: Vec<String>, selectors: &[String]) -> Vec<Result<ScrapeResult>> {
        let mut results = Vec::new();
        
        // 使用信号量限制并发数
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(5));
        let mut handles = Vec::new();

        for url in urls {
            let semaphore = semaphore.clone();
            let selectors = selectors.to_vec();
            let scraper = self.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                scraper.scrape_intelligent(&url, &selectors).await
            });
            
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(anyhow!("任务执行失败: {}", e))),
            }
        }

        results
    }
}

// 实现Clone以支持多线程使用
impl Clone for IntelligentScraper {
    fn clone(&self) -> Self {
        Self {
            http_client: self.http_client.clone(),
            enable_js_rendering: self.enable_js_rendering,
            user_agents: self.user_agents.clone(),
            retry_config: self.retry_config.clone(),
        }
    }
}

/// 内容类型
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Changelog,
    ReleasePage,
    Documentation,
    BlogPost,
    Unknown,
} 