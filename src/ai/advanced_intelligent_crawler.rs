use anyhow::Result;
use tracing::{info, debug, warn, error};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json;
use regex;
use url;

use crate::ai::ai_service::{AIService, AIRequest, AIServiceConfig};
use crate::ai::intelligent_web_analyzer::{CrawlTask, ContentType, PageRelevanceAnalysis, ExtractedLink};
use crate::ai::smart_url_crawler::{CrawlerConfig, TaskResult as BasicTaskResult}; // Renaming to avoid conflict

// --- 新的核心结构体定义 ---

/// 高级智能爬虫，用于深度内容提取和知识构建
pub struct AdvancedIntelligentCrawler {
    ai_service: Arc<AIService>,
    config: Arc<CrawlerConfig>,
    // 未来可以添加更多配置，如特定于高级爬虫的参数
}

/// URL发现代理，负责智能地发现和优先处理URL
struct UrlDiscoveryAgent {
    ai_service: Arc<AIService>,
    task: Arc<CrawlTask>,
    visited_urls: Arc<RwLock<HashSet<String>>>,
    pending_urls: Arc<RwLock<VecDeque<PrioritizedUrl>>>,
    config: Arc<CrawlerConfig>,
}

/// 内容提取代理，负责从单个页面提取结构化信息
struct ContentExtractionAgent {
    ai_service: Arc<AIService>,
    task: Arc<CrawlTask>,
    config: Arc<CrawlerConfig>,
}

/// 知识聚合器，负责将来自多个页面的片段聚合成综合文档
struct KnowledgeAggregator {
    ai_service: Arc<AIService>,
    task: Arc<CrawlTask>,
    collected_fragments: Arc<RwLock<Vec<ContentFragment>>>,
}

/// 带有优先级的URL结构
#[derive(Debug, Clone)]
struct PrioritizedUrl {
    url: String,
    priority: f32, // 0.0 - 1.0, AI评估的优先级
    depth: u32,
    source_page_url: Option<String>, // 从哪个页面发现的此URL
}

/// 从页面提取的内容片段
#[derive(Debug, Clone)]
pub struct ContentFragment {
    pub source_url: String,
    pub fragment_type: ContentType, // 复用现有的ContentType或定义新的
    pub title: Option<String>,
    pub content: String,
    pub relevance_score: f32, // AI评估的片段与任务的相关性
    // 可以添加更多元数据，如代码语言、API签名等
}

/// 高级爬虫执行的最终结果
#[derive(Debug, Clone)]
pub struct AdvancedTaskResult {
    pub task: CrawlTask,
    pub aggregated_document: String, // 最终生成的综合文档
    pub source_fragments: Vec<ContentFragment>, // 用于生成文档的所有片段
    pub visited_urls_count: usize,
    // 可以添加更多统计信息
}

/// 发现的链接信息
#[derive(Debug, Clone)]
struct DiscoveredLink {
    url: String,
    priority: f32,
    link_text: String,
    context: String,
}


impl AdvancedIntelligentCrawler {
    pub async fn new(ai_service_config: AIServiceConfig, crawler_config: CrawlerConfig) -> Result<Self> {
        let ai_service = Arc::new(AIService::new(ai_service_config)?);
        let config = Arc::new(crawler_config);
        info!("🚀 高级智能爬虫初始化完成");
        Ok(Self { ai_service, config })
    }

    pub async fn execute_task(&self, task: CrawlTask) -> Result<AdvancedTaskResult> {
        info!("🎯 开始执行高级爬虫任务: {}", task.target_description);
        let task_arc = Arc::new(task.clone());

        let visited_urls = Arc::new(RwLock::new(HashSet::new()));
        let pending_urls = Arc::new(RwLock::new(VecDeque::new()));
        let collected_fragments = Arc::new(RwLock::new(Vec::new()));

        // 1. 初始化URL发现代理并添加起始URL
        let url_discoverer = UrlDiscoveryAgent::new(
            self.ai_service.clone(),
            task_arc.clone(),
            visited_urls.clone(),
            pending_urls.clone(),
            self.config.clone(),
        );
        url_discoverer.add_start_url().await;

        // 2. 初始化内容提取代理和知识聚合器
        let content_extractor = ContentExtractionAgent::new(
            self.ai_service.clone(),
            task_arc.clone(),
            self.config.clone(),
        );
        let knowledge_aggregator = KnowledgeAggregator::new(
            self.ai_service.clone(),
            task_arc.clone(),
            collected_fragments.clone(),
        );
        
        // 3. 主爬取循环
        let mut pages_processed = 0;
        while let Some(current_url) = url_discoverer.get_next_url().await {
            if pages_processed >= task_arc.max_pages {
                info!("达到最大页面处理限制 ({})，停止爬取。", task_arc.max_pages);
                break;
            }
            if current_url.depth >= task_arc.max_depth {
                info!("达到最大深度 ({})，跳过 URL: {}", task_arc.max_depth, current_url.url);
                continue;
            }

            debug!("处理URL: {} (深度: {}, 优先级: {:.2})", current_url.url, current_url.depth, current_url.priority);
            
            // 获取并提取内容
            match content_extractor.fetch_and_extract(&current_url.url).await {
                Ok(fragments) => {
                    if !fragments.is_empty() {
                        info!("📄 从 {} 提取了 {} 个内容片段", current_url.url, fragments.len());
                        for fragment in fragments {
                            knowledge_aggregator.add_fragment(fragment).await;
                        }
                        
                        // 🔥 关键改进：从当前页面发现新的相关链接
                        if current_url.depth < task_arc.max_depth - 1 {
                            // 重新获取页面内容用于链接发现
                            match content_extractor.fetch_page_content(&current_url.url).await {
                                Ok(page_content) => {
                                    if let Err(e) = url_discoverer.discover_links_from_content(
                                        &page_content, 
                                        &current_url.url, 
                                        current_url.depth
                                    ).await {
                                        warn!("链接发现失败 {}: {}", current_url.url, e);
                                    }
                                }
                                Err(e) => {
                                    warn!("重新获取页面内容失败 {}: {}", current_url.url, e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("提取内容失败 {}: {}", current_url.url, e);
                }
            }
            
            // 标记已访问（即使提取失败也标记，避免重试无法访问的页面）
            url_discoverer.mark_as_visited(&current_url.url).await;
            pages_processed += 1;

            // 短暂延迟，避免过于频繁请求
            tokio::time::sleep(std::time::Duration::from_millis(self.config.delay_ms)).await;
        }

        // 4. 聚合知识
        let aggregated_document = knowledge_aggregator.aggregate_knowledge().await?;
        
        // 提前获取值以避免借用检查器问题
        let source_fragments = collected_fragments.read().await.clone();
        let visited_urls_count = visited_urls.read().await.len();
        
        info!("✅ 高级爬虫任务完成: {}，生成了 {} 字符的文档，访问了 {} 个URL", 
            task_arc.target_description, aggregated_document.len(), visited_urls_count);

        Ok(AdvancedTaskResult {
            task: task_arc.as_ref().clone(),
            aggregated_document,
            source_fragments,
            visited_urls_count,
        })
    }
}

impl UrlDiscoveryAgent {
    fn new(
        ai_service: Arc<AIService>,
        task: Arc<CrawlTask>,
        visited_urls: Arc<RwLock<HashSet<String>>>,
        pending_urls: Arc<RwLock<VecDeque<PrioritizedUrl>>>,
        config: Arc<CrawlerConfig>,
    ) -> Self {
        Self { ai_service, task, visited_urls, pending_urls, config }
    }

    async fn add_start_url(&self) {
        let mut queue = self.pending_urls.write().await;
        queue.push_back(PrioritizedUrl {
            url: self.task.start_url.clone(),
            priority: 1.0, // 起始URL最高优先级
            depth: 0,
            source_page_url: None,
        });
        info!("种子URL已添加: {}", self.task.start_url);
    }

    async fn get_next_url(&self) -> Option<PrioritizedUrl> {
        let mut queue = self.pending_urls.write().await;
        // 简单地从队列前端获取，未来可以实现更复杂的优先级调度
        queue.pop_front()
    }

    async fn mark_as_visited(&self, url: &str) {
        let mut visited = self.visited_urls.write().await;
        visited.insert(url.to_string());
    }
    
    /// 智能链接发现：从页面内容中识别相关链接
    async fn discover_links_from_content(&self, page_content: &str, current_url: &str, current_depth: u32) -> Result<()> {
        info!("🔍 URL发现代理: 开始从 {} 发现相关链接", current_url);
        
        // 1. 构建AI请求，要求智能提取和评估链接
        let system_prompt = self.get_link_discovery_system_prompt();
        let user_message = self.get_link_discovery_user_message(page_content, current_url);
        
        let request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3), // 较低温度确保准确性
            max_tokens: Some(2000),
            stream: false,
        };
        
        let response = self.ai_service.request(request).await?;
        
        // 2. 解析AI响应，提取链接信息
        let discovered_links = self.parse_link_discovery_response(&response.content, current_url).await?;
        
        // 3. 处理发现的链接
        let mut queue = self.pending_urls.write().await;
        let visited = self.visited_urls.read().await;
        
        for link in discovered_links {
            // 规范化URL
            if let Ok(normalized_url) = self.normalize_url(&link.url, current_url) {
                // 检查是否已访问或已在队列中
                if !visited.contains(&normalized_url) && 
                   !queue.iter().any(|p| p.url == normalized_url) {
                    
                    let prioritized_url = PrioritizedUrl {
                        url: normalized_url.clone(),
                        priority: link.priority,
                        depth: current_depth + 1,
                        source_page_url: Some(current_url.to_string()),
                    };
                    
                    // 按优先级插入队列
                    self.insert_by_priority(&mut queue, prioritized_url);
                    debug!("🔗 发现新链接: {} (优先级: {:.2})", normalized_url, link.priority);
                }
            }
        }
        
        info!("✅ 链接发现完成，队列中现有 {} 个待处理URL", queue.len());
        Ok(())
    }
    
    fn get_link_discovery_system_prompt(&self) -> String {
        r#"你是一个专业的网页链接分析专家。你的任务是从HTML内容中智能识别和评估与特定技术任务相关的链接。

你需要：
1. 分析HTML中的所有链接（<a>标签、导航菜单、相关链接等）
2. 根据任务目标评估每个链接的相关性和价值
3. 为每个相关链接分配优先级分数（0.0-1.0）
4. 返回结构化的JSON结果

评估标准：
- 官方文档链接：优先级 0.9-1.0
- API参考和教程：优先级 0.7-0.9
- 代码示例和用例：优先级 0.6-0.8
- 社区讨论和博客：优先级 0.4-0.6
- 相关但非核心内容：优先级 0.2-0.4
- 无关内容：优先级 0.0-0.2

返回格式：JSON数组，每个对象包含：
{
  "url": "链接URL",
  "priority": 0.85,
  "link_text": "链接文本",
  "context": "链接上下文描述",
  "reasoning": "选择此链接的原因"
}"#.to_string()
    }
    
    fn get_link_discovery_user_message(&self, page_content: &str, current_url: &str) -> String {
        format!(
            r#"任务目标：为 {} 语言的 {} 库收集相关文档
具体查询：{}
当前页面：{}

HTML内容（前8000字符）：
{}

请分析此页面中的所有链接，识别与任务目标相关的链接，并按相关性排序。
只返回优先级 >= 0.3 的链接。
确保URL是完整的绝对路径。"#,
            self.task.programming_language,
            self.task.library_name,
            self.task.target_description,
            current_url,
            page_content.chars().take(8000).collect::<String>()
        )
    }
    
    async fn parse_link_discovery_response(&self, response: &str, current_url: &str) -> Result<Vec<DiscoveredLink>> {
        debug!("解析链接发现响应: {}", response.chars().take(200).collect::<String>());
        
        // 尝试解析JSON响应
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(response) {
            if let Some(links_array) = json_value.as_array() {
                let mut discovered_links = Vec::new();
                
                for link_obj in links_array {
                    if let (Some(url), Some(priority)) = (
                        link_obj.get("url").and_then(|v| v.as_str()),
                        link_obj.get("priority").and_then(|v| v.as_f64())
                    ) {
                        let link_text = link_obj.get("link_text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let context = link_obj.get("context")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        discovered_links.push(DiscoveredLink {
                            url: url.to_string(),
                            priority: priority as f32,
                            link_text,
                            context,
                        });
                    }
                }
                
                return Ok(discovered_links);
            }
        }
        
        // 如果JSON解析失败，尝试从文本中提取链接
        warn!("JSON解析失败，尝试从文本中提取链接");
        Ok(self.extract_links_from_text(response, current_url).await)
    }
    
    async fn extract_links_from_text(&self, text: &str, _current_url: &str) -> Vec<DiscoveredLink> {
        // 简单的文本链接提取作为备用方案
        let mut links = Vec::new();
        
        // 查找HTTP/HTTPS链接
        let url_regex = regex::Regex::new(r"https?://[^\s<>]+").unwrap();
        for mat in url_regex.find_iter(text) {
            let url = mat.as_str().to_string();
            // 给文本提取的链接一个中等优先级
            links.push(DiscoveredLink {
                url,
                priority: 0.5,
                link_text: "从文本提取".to_string(),
                context: "备用链接提取".to_string(),
            });
        }
        
        links
    }
    
    fn normalize_url(&self, url: &str, base_url: &str) -> Result<String> {
        use url::Url;
        
        // 如果已经是绝对URL，直接返回
        if url.starts_with("http://") || url.starts_with("https://") {
            return Ok(url.to_string());
        }
        
        // 否则基于base_url构建绝对URL
        let base = Url::parse(base_url)?;
        let absolute = base.join(url)?;
        Ok(absolute.to_string())
    }
    
    fn insert_by_priority(&self, queue: &mut VecDeque<PrioritizedUrl>, new_url: PrioritizedUrl) {
        // 按优先级降序插入
        let mut insert_index = queue.len();
        
        for (i, existing) in queue.iter().enumerate() {
            if new_url.priority > existing.priority {
                insert_index = i;
                break;
            }
        }
        
        queue.insert(insert_index, new_url);
    }
}

impl ContentExtractionAgent {
    fn new(ai_service: Arc<AIService>, task: Arc<CrawlTask>, config: Arc<CrawlerConfig>) -> Self {
        Self { ai_service, task, config }
    }

    /// 内容提取：获取页面内容并使用AI提取结构化信息片段
    async fn fetch_and_extract(&self, url: &str) -> Result<Vec<ContentFragment>> {
        info!("内容提取代理: 开始从 {} 提取内容 (部分实现)", url);
        // 1. HTTP GET请求获取页面HTML
        let html_content = self.fetch_page_content(url).await?;

        // 2. 构建AI请求，要求从HTML中提取与任务相关的结构化信息片段
        //    - Prompt应包含任务描述 (目标语言、库、具体查询)
        //    - 要求AI识别不同类型的内容（代码、API定义、文本解释、列表等）
        //    - 要求AI评估每个片段的相关性
        //    - AI响应应是结构化的，例如JSON数组，每个对象是一个内容片段
        let request = AIRequest {
            model: None, // 使用默认模型
            system_prompt: Some(self.get_extraction_system_prompt()),
            user_message: self.get_extraction_user_message(&html_content, url),
            temperature: Some(0.2), // 低温以获取更精确的提取
            max_tokens: Some(3000), // 根据预期内容调整
            stream: false,
        };
        
        let response = self.ai_service.request(request).await?;
        
        // 3. 解析AI响应到 Vec<ContentFragment>
        let fragments = self.parse_extraction_response(&response.content, url).await?;
        
        Ok(fragments)
    }

    async fn fetch_page_content(&self, url: &str) -> Result<String> {
        debug!("📥 获取页面内容: {}", url);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .user_agent(self.config.user_agent.clone())
            .build()?;

        let mut attempts = 0;
        while attempts < self.config.max_retries {
            match client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let content = response.text().await?;
                        debug!("✅ 成功获取页面内容，长度: {} 字符", content.len());
                        return Ok(content);
                    } else {
                        warn!("🚫 HTTP错误: {} - {}", response.status(), url);
                    }
                }
                Err(e) => {
                    warn!("🌐 网络请求失败 (尝试 {}/{}): {} for URL {}", attempts + 1, self.config.max_retries, e, url);
                }
            }
            attempts += 1;
            if attempts < self.config.max_retries {
                tokio::time::sleep(std::time::Duration::from_millis(1000 * attempts as u64)).await;
            }
        }
        Err(anyhow::anyhow!("获取页面 {} 内容失败，已重试 {} 次", url, self.config.max_retries))
    }
    
    fn get_extraction_system_prompt(&self) -> String {
        r#"你是一个专业的技术内容提取专家。你的任务是从HTML网页中提取与特定编程任务相关的结构化信息。

你需要：
1. 分析HTML内容，识别与任务目标相关的技术信息
2. 提取代码示例、API文档、教程步骤、配置说明等
3. 为每个内容片段分配相关性分数（0.0-1.0）
4. 返回结构化的JSON结果

内容类型分类：
- Documentation: 官方文档和说明
- Tutorial: 教程和指南
- ApiReference: API参考文档
- Examples: 代码示例和用例
- GettingStarted: 入门指南
- Installation: 安装说明
- Configuration: 配置文档
- Troubleshooting: 故障排除

返回格式：JSON数组，每个对象包含：
{
  "fragment_type": "Documentation",
  "title": "片段标题",
  "content": "提取的内容文本",
  "relevance_score": 0.85,
  "code_language": "rust" // 如果包含代码
}"#.to_string()
    }

    fn get_extraction_user_message(&self, html_content: &str, url: &str) -> String {
        format!(
            r#"任务目标：为 {} 语言的 {} 库提取相关技术内容
具体查询：{}
当前页面：{}

HTML内容（前10000字符）：
{}

请从此页面提取与任务目标相关的所有技术内容片段。
重点关注：
1. 与 {} 库相关的代码示例
2. API使用说明和参数描述
3. 配置和安装指南
4. 常见问题和解决方案
5. 最佳实践和使用建议

只返回相关性分数 >= 0.4 的内容片段。
确保提取的内容完整且有意义。"#,
            self.task.programming_language,
            self.task.library_name,
            self.task.target_description,
            url,
            html_content.chars().take(10000).collect::<String>(),
            self.task.library_name
        )
    }

    async fn parse_extraction_response(&self, ai_response_content: &str, source_url: &str) -> Result<Vec<ContentFragment>> {
        debug!("解析内容提取响应: {}", ai_response_content.chars().take(200).collect::<String>());
        
        // 尝试解析JSON响应
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(ai_response_content) {
            if let Some(fragments_array) = json_value.as_array() {
                let mut content_fragments = Vec::new();
                
                for fragment_obj in fragments_array {
                    if let (Some(content), Some(relevance_score)) = (
                        fragment_obj.get("content").and_then(|v| v.as_str()),
                        fragment_obj.get("relevance_score").and_then(|v| v.as_f64())
                    ) {
                        let fragment_type = fragment_obj.get("fragment_type")
                            .and_then(|v| v.as_str())
                            .and_then(|s| self.parse_content_type(s))
                            .unwrap_or(ContentType::Documentation);
                        
                        let title = fragment_obj.get("title")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        
                        content_fragments.push(ContentFragment {
                            source_url: source_url.to_string(),
                            fragment_type,
                            title,
                            content: content.to_string(),
                            relevance_score: relevance_score as f32,
                        });
                    }
                }
                
                info!("✅ 成功解析 {} 个内容片段", content_fragments.len());
                return Ok(content_fragments);
            }
        }
        
        // 如果JSON解析失败，创建一个包含AI响应的基本片段
        warn!("JSON解析失败，创建基本内容片段");
        Ok(vec![ContentFragment {
            source_url: source_url.to_string(),
            fragment_type: ContentType::Documentation,
            title: Some("AI提取的内容".to_string()),
            content: ai_response_content.chars().take(1000).collect(),
            relevance_score: 0.6,
        }])
    }
    
    fn parse_content_type(&self, type_str: &str) -> Option<ContentType> {
        match type_str {
            "Documentation" => Some(ContentType::Documentation),
            "Tutorial" => Some(ContentType::Tutorial),
            "ApiReference" => Some(ContentType::ApiReference),
            "Examples" => Some(ContentType::Examples),
            "GettingStarted" => Some(ContentType::GettingStarted),
            "Installation" => Some(ContentType::Installation),
            "Configuration" => Some(ContentType::Configuration),
            "Troubleshooting" => Some(ContentType::Troubleshooting),
            _ => None,
        }
    }
}

impl KnowledgeAggregator {
    fn new(ai_service: Arc<AIService>, task: Arc<CrawlTask>, collected_fragments: Arc<RwLock<Vec<ContentFragment>>>) -> Self {
        Self { ai_service, task, collected_fragments }
    }

    async fn add_fragment(&self, fragment: ContentFragment) {
        let mut fragments = self.collected_fragments.write().await;
        fragments.push(fragment);
    }

    /// 知识聚合：将收集的内容片段聚合成连贯文档
    async fn aggregate_knowledge(&self) -> Result<String> {
        info!("知识聚合器: 开始聚合已收集的片段 (部分实现)");
        let fragments = self.collected_fragments.read().await;
        if fragments.is_empty() {
            return Ok("未收集到任何内容片段进行聚合。".to_string());
        }

        // 1. 预处理和筛选片段 (例如，按相关性、去重等)
        let mut content_to_aggregate = String::new();
        for (i, fragment) in fragments.iter().enumerate() {
            content_to_aggregate.push_str(&format!(
                "--- Fragment {} from {} (Relevance: {:.2}) ---\nTitle: {}\nType: {:?}\nContent:\n{}\n\n",
                i + 1,
                fragment.source_url,
                fragment.relevance_score,
                fragment.title.as_deref().unwrap_or("N/A"),
                fragment.fragment_type,
                fragment.content
            ));
        }
        
        // 2. 构建AI请求，要求将这些片段聚合成一份结构良好、信息全面的文档
        //    - Prompt应包含原始任务目标
        //    - 指示AI组织内容、消除冗余、确保流畅性和准确性
        let request = AIRequest {
            model: None,
            system_prompt: Some(self.get_aggregation_system_prompt()),
            user_message: self.get_aggregation_user_message(&content_to_aggregate),
            temperature: Some(0.5), // 适中温度以平衡创造性和准确性
            max_tokens: Some(4000), // 允许生成较长的文档
            stream: false,
        };

        let response = self.ai_service.request(request).await?;
        
        // 3. 返回AI生成的聚合文档
        Ok(response.content)
    }
    
    fn get_aggregation_system_prompt(&self) -> String {
        r#"你是一个专业的技术文档编写专家。你的任务是将来自多个网页的技术内容片段整合成一份连贯、全面、高质量的技术文档。

你需要：
1. 分析所有内容片段，理解它们之间的关系和层次
2. 去除重复信息，整合相关内容
3. 按逻辑顺序组织内容（概述→安装→基础用法→高级特性→示例→故障排除）
4. 确保技术信息的准确性和完整性
5. 添加必要的过渡和解释文本
6. 生成结构化的Markdown文档

文档结构要求：
- 使用清晰的标题层次（#, ##, ###）
- 代码块使用正确的语言标识
- 包含目录和章节导航
- 突出重要信息和最佳实践
- 提供实用的代码示例

输出格式：完整的Markdown文档，包含：
1. 文档标题和简介
2. 目录
3. 主要内容章节
4. 代码示例和用法
5. 参考链接和来源"#.to_string()
    }

    fn get_aggregation_user_message(&self, all_fragments_text: &str) -> String {
        format!(
            r#"任务目标：为 {} 语言的 {} 库创建综合技术文档
用户查询：{}

收集到的内容片段：
{}

请将这些片段整合成一份专业的技术文档。文档应该：

1. **结构清晰**：按逻辑顺序组织内容
2. **内容全面**：涵盖安装、配置、使用、示例等方面
3. **实用性强**：包含可直接使用的代码示例
4. **易于理解**：适合不同技能水平的开发者

重点关注：
- {} 库的核心功能和特性
- 实际使用场景和最佳实践
- 常见问题和解决方案
- 完整的代码示例

请生成一份高质量的Markdown技术文档。"#,
            self.task.programming_language,
            self.task.library_name,
            self.task.target_description,
            all_fragments_text.chars().take(12000).collect::<String>(), // 增加输入长度
            self.task.library_name
        )
    }
}

// --- 辅助函数和结构 (如果需要) --- 
