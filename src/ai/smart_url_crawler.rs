use anyhow::Result;
use tracing::{info, debug, warn, error};
use std::collections::{HashMap, HashSet, VecDeque};
use url::Url;
use chrono::{DateTime, Utc, Duration};
use tokio::time::sleep;
use std::sync::Arc;

use super::ai_service::AIService;
use super::intelligent_web_analyzer::{
    IntelligentWebAnalyzer, CrawlTask, PageRelevanceAnalysis, 
    ContentRegionAnalysis, ExtractedLink, RecommendedAction
};

/// 智能URL爬虫
/// 任务导向的防循环爬虫系统
pub struct SmartUrlCrawler {
    web_analyzer: IntelligentWebAnalyzer,
    http_client: reqwest::Client,
    crawl_state: Arc<tokio::sync::RwLock<CrawlState>>,
}

/// 爬虫状态
#[derive(Debug)]
struct CrawlState {
    /// 已访问的URL
    visited_urls: HashSet<String>,
    /// 待处理的URL队列
    pending_urls: VecDeque<PendingUrl>,
    /// 当前任务
    current_task: Option<CrawlTask>,
    /// 任务结果
    task_results: Vec<TaskResult>,
    /// 循环检测记录
    loop_detection: HashMap<String, LoopDetectionInfo>,
    /// 爬虫统计
    statistics: CrawlStatistics,
}

/// 待处理的URL
#[derive(Debug, Clone)]
struct PendingUrl {
    /// URL地址
    url: String,
    /// 优先级 (1-5, 5最高)
    priority: u8,
    /// 深度
    depth: u32,
    /// 父URL
    parent_url: Option<String>,
    /// 发现时间
    discovered_at: DateTime<Utc>,
    /// 预期内容类型
    expected_content_type: Option<String>,
}

/// 任务结果
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// 任务ID
    pub task_id: String,
    /// 处理的URL
    pub url: String,
    /// 相关性分析
    pub relevance_analysis: PageRelevanceAnalysis,
    /// 内容区域
    pub content_regions: ContentRegionAnalysis,
    /// 提取的内容摘要
    pub content_summary: String,
    /// 发现的相关链接数量
    pub discovered_links_count: usize,
    /// 处理时间
    pub processed_at: DateTime<Utc>,
    /// 处理耗时（毫秒）
    pub processing_time_ms: u64,
}

/// 循环检测信息
#[derive(Debug, Clone)]
struct LoopDetectionInfo {
    /// 访问次数
    visit_count: u32,
    /// 首次访问时间
    first_visit: DateTime<Utc>,
    /// 最后访问时间
    last_visit: DateTime<Utc>,
    /// 访问路径
    visit_path: Vec<String>,
}

/// 爬虫统计
#[derive(Debug, Clone)]
pub struct CrawlStatistics {
    /// 总访问页面数
    pub total_pages_visited: u32,
    /// 相关页面数
    pub relevant_pages_count: u32,
    /// 跳过页面数
    pub skipped_pages_count: u32,
    /// 循环检测次数
    pub loop_detections: u32,
    /// 总处理时间（毫秒）
    pub total_processing_time_ms: u64,
    /// 平均相关性分数
    pub average_relevance_score: f32,
    /// 开始时间
    pub start_time: DateTime<Utc>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
}

/// 爬虫配置
#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    /// 并发度
    pub concurrency: u32,
    /// 请求间隔（毫秒）
    pub delay_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 循环检测阈值
    pub loop_detection_threshold: u32,
    /// 最小相关性分数
    pub min_relevance_score: f32,
    /// 用户代理
    pub user_agent: String,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            concurrency: 3,
            delay_ms: 1000,
            max_retries: 3,
            timeout_secs: 30,
            loop_detection_threshold: 3,
            min_relevance_score: 0.5,
            user_agent: "GrapeMCPDevtools/2.0 (Intelligent Web Crawler)".to_string(),
        }
    }
}

impl SmartUrlCrawler {
    /// 创建新的智能爬虫
    pub async fn new(ai_service: AIService, config: CrawlerConfig) -> Result<Self> {
        let web_analyzer = IntelligentWebAnalyzer::new(ai_service).await?;
        
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .user_agent(config.user_agent.clone())
            .build()?;

        let crawl_state = Arc::new(tokio::sync::RwLock::new(CrawlState {
            visited_urls: HashSet::new(),
            pending_urls: VecDeque::new(),
            current_task: None,
            task_results: Vec::new(),
            loop_detection: HashMap::new(),
            statistics: CrawlStatistics {
                total_pages_visited: 0,
                relevant_pages_count: 0,
                skipped_pages_count: 0,
                loop_detections: 0,
                total_processing_time_ms: 0,
                average_relevance_score: 0.0,
                start_time: Utc::now(),
                end_time: None,
            },
        }));

        info!("🚀 智能URL爬虫初始化完成");
        info!("⚙️ 配置 - 并发度: {}, 延迟: {}ms, 最小相关性: {}", 
              config.concurrency, config.delay_ms, config.min_relevance_score);

        Ok(Self {
            web_analyzer,
            http_client,
            crawl_state,
        })
    }

    /// 开始执行爬虫任务
    pub async fn execute_task(&self, task: CrawlTask, config: CrawlerConfig) -> Result<Vec<TaskResult>> {
        info!("🎯 开始执行爬虫任务: {}", task.target_description);
        info!("📍 起始URL: {}", task.start_url);

        // 初始化任务
        {
            let mut state = self.crawl_state.write().await;
            state.current_task = Some(task.clone());
            state.visited_urls.clear();
            state.pending_urls.clear();
            state.task_results.clear();
            state.loop_detection.clear();
            state.statistics = CrawlStatistics {
                total_pages_visited: 0,
                relevant_pages_count: 0,
                skipped_pages_count: 0,
                loop_detections: 0,
                total_processing_time_ms: 0,
                average_relevance_score: 0.0,
                start_time: Utc::now(),
                end_time: None,
            };

            // 添加起始URL到待处理队列
            state.pending_urls.push_back(PendingUrl {
                url: task.start_url.clone(),
                priority: 5, // 起始URL最高优先级
                depth: 0,
                parent_url: None,
                discovered_at: Utc::now(),
                expected_content_type: None,
            });
        }

        // 执行爬虫循环
        while let Some(pending_url) = self.get_next_url().await {
            if self.should_stop_crawling(&task).await {
                info!("⏹️ 达到爬虫停止条件");
                break;
            }

            // 循环检测
            if self.detect_loop(&pending_url.url).await {
                warn!("🔄 检测到循环，跳过URL: {}", pending_url.url);
                self.increment_loop_detection().await;
                continue;
            }

            // 处理URL
            match self.process_url(&pending_url, &task, &config).await {
                Ok(result) => {
                    if let Some(task_result) = result {
                        self.add_task_result(task_result).await;
                    }
                }
                Err(e) => {
                    error!("❌ 处理URL失败 {}: {}", pending_url.url, e);
                }
            }

            // 延迟
            if config.delay_ms > 0 {
                sleep(std::time::Duration::from_millis(config.delay_ms)).await;
            }
        }

        // 完成任务
        let results = {
            let mut state = self.crawl_state.write().await;
            state.statistics.end_time = Some(Utc::now());
            state.task_results.clone()
        };

        info!("✅ 爬虫任务完成");
        self.print_statistics().await;

        Ok(results)
    }

    /// 处理单个URL
    async fn process_url(&self, pending_url: &PendingUrl, task: &CrawlTask, config: &CrawlerConfig) -> Result<Option<TaskResult>> {
        let start_time = std::time::Instant::now();
        
        info!("🔍 处理URL: {} (深度: {}, 优先级: {})", 
              pending_url.url, pending_url.depth, pending_url.priority);

        // 检查是否已访问
        if self.is_visited(&pending_url.url).await {
            debug!("⏭️ URL已访问，跳过: {}", pending_url.url);
            return Ok(None);
        }

        // 检查深度限制
        if pending_url.depth >= task.max_depth {
            debug!("📏 达到最大深度，跳过: {}", pending_url.url);
            return Ok(None);
        }

        // 标记为已访问
        self.mark_as_visited(&pending_url.url).await;

        // 获取页面内容
        let html_content = match self.fetch_page_content(&pending_url.url, config).await {
            Ok(content) => content,
            Err(e) => {
                warn!("📄 无法获取页面内容 {}: {}", pending_url.url, e);
                return Ok(None);
            }
        };

        // 综合分析页面
        let (relevance_analysis, content_regions, extracted_links) = self.web_analyzer
            .comprehensive_page_analysis(&html_content, &pending_url.url, task)
            .await?;

        // 检查相关性
        if relevance_analysis.relevance_score < config.min_relevance_score {
            info!("📉 相关性分数过低 ({:.2})，跳过页面: {}", 
                  relevance_analysis.relevance_score, pending_url.url);
            self.increment_skipped_pages().await;
            return Ok(None);
        }

        // 生成内容摘要
        let content_summary = self.web_analyzer
            .generate_task_focused_summary(&content_regions, task)
            .await?;

        // 处理提取的链接
        self.process_extracted_links(&extracted_links, &pending_url.url, pending_url.depth + 1).await;

        let processing_time = start_time.elapsed().as_millis() as u64;

        // 更新统计
        self.update_statistics(relevance_analysis.relevance_score, processing_time).await;

        info!("✅ URL处理完成，相关性: {:.2}, 发现链接: {}", 
              relevance_analysis.relevance_score, extracted_links.len());

        Ok(Some(TaskResult {
            task_id: task.task_id.clone(),
            url: pending_url.url.clone(),
            relevance_analysis,
            content_regions,
            content_summary,
            discovered_links_count: extracted_links.len(),
            processed_at: Utc::now(),
            processing_time_ms: processing_time,
        }))
    }

    /// 获取页面内容
    async fn fetch_page_content(&self, url: &str, config: &CrawlerConfig) -> Result<String> {
        debug!("📥 获取页面内容: {}", url);

        let mut attempts = 0;
        while attempts < config.max_retries {
            match self.http_client.get(url).send().await {
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
                    warn!("🌐 网络请求失败 (尝试 {}/{}): {}", attempts + 1, config.max_retries, e);
                }
            }
            
            attempts += 1;
            if attempts < config.max_retries {
                sleep(std::time::Duration::from_millis(1000 * attempts as u64)).await;
            }
        }

        Err(anyhow::anyhow!("无法获取页面内容，已重试{}次", config.max_retries))
    }

    /// 处理提取的链接
    async fn process_extracted_links(&self, links: &[ExtractedLink], parent_url: &str, depth: u32) {
        let mut state = self.crawl_state.write().await;
        let current_task = state.current_task.as_ref().unwrap();

        for link in links {
            // 验证和规范化URL
            if let Ok(absolute_url) = self.normalize_url(&link.url, parent_url) {
                // 避免重复添加
                if !state.visited_urls.contains(&absolute_url) && 
                   !state.pending_urls.iter().any(|p| p.url == absolute_url) {
                    
                    let pending_url = PendingUrl {
                        url: absolute_url,
                        priority: link.priority,
                        depth,
                        parent_url: Some(parent_url.to_string()),
                        discovered_at: Utc::now(),
                        expected_content_type: Some(format!("{:?}", link.link_type)),
                    };

                    // 按优先级插入队列
                    self.insert_by_priority(&mut state.pending_urls, pending_url);
                }
            }
        }

        debug!("🔗 处理了{}个链接，队列中有{}个待处理URL", 
               links.len(), state.pending_urls.len());
    }

    /// 按优先级插入队列
    fn insert_by_priority(&self, queue: &mut VecDeque<PendingUrl>, new_url: PendingUrl) {
        let mut insert_index = queue.len();
        
        for (i, existing) in queue.iter().enumerate() {
            if new_url.priority > existing.priority || 
               (new_url.priority == existing.priority && new_url.depth < existing.depth) {
                insert_index = i;
                break;
            }
        }
        
        queue.insert(insert_index, new_url);
    }

    /// 规范化URL
    fn normalize_url(&self, url: &str, base_url: &str) -> Result<String> {
        let base = Url::parse(base_url)?;
        let absolute = base.join(url)?;
        Ok(absolute.to_string())
    }

    /// 循环检测
    async fn detect_loop(&self, url: &str) -> bool {
        let mut state = self.crawl_state.write().await;
        
        let info = state.loop_detection.entry(url.to_string()).or_insert_with(|| {
            LoopDetectionInfo {
                visit_count: 0,
                first_visit: Utc::now(),
                last_visit: Utc::now(),
                visit_path: Vec::new(),
            }
        });

        info.visit_count += 1;
        info.last_visit = Utc::now();
        info.visit_path.push(url.to_string());

        // 检查是否超过阈值
        if info.visit_count > 3 {
            warn!("🔄 检测到可能的循环: {} (访问{}次)", url, info.visit_count);
            return true;
        }

        // 检查时间窗口内的频繁访问
        let time_diff = info.last_visit.signed_duration_since(info.first_visit);
        if info.visit_count > 2 && time_diff < Duration::minutes(5) {
            warn!("⏰ 检测到短时间内频繁访问: {}", url);
            return true;
        }

        false
    }

    /// 获取下一个待处理的URL
    async fn get_next_url(&self) -> Option<PendingUrl> {
        let mut state = self.crawl_state.write().await;
        state.pending_urls.pop_front()
    }

    /// 检查是否应该停止爬虫
    async fn should_stop_crawling(&self, task: &CrawlTask) -> bool {
        let state = self.crawl_state.read().await;
        
        // 检查页面数量限制
        if state.statistics.total_pages_visited >= task.max_pages {
            return true;
        }

        // 检查队列是否为空
        if state.pending_urls.is_empty() {
            return true;
        }

        // 检查是否有足够的相关结果
        if state.statistics.relevant_pages_count >= 20 {
            return true;
        }

        false
    }

    /// 标记URL为已访问
    async fn mark_as_visited(&self, url: &str) {
        let mut state = self.crawl_state.write().await;
        state.visited_urls.insert(url.to_string());
    }

    /// 检查URL是否已访问
    async fn is_visited(&self, url: &str) -> bool {
        let state = self.crawl_state.read().await;
        state.visited_urls.contains(url)
    }

    /// 添加任务结果
    async fn add_task_result(&self, result: TaskResult) {
        let mut state = self.crawl_state.write().await;
        state.task_results.push(result);
    }

    /// 更新统计信息
    async fn update_statistics(&self, relevance_score: f32, processing_time: u64) {
        let mut state = self.crawl_state.write().await;
        
        state.statistics.total_pages_visited += 1;
        state.statistics.total_processing_time_ms += processing_time;
        
        if relevance_score >= 0.5 {
            state.statistics.relevant_pages_count += 1;
        }

        // 计算平均相关性分数
        let total_score = state.statistics.average_relevance_score * (state.statistics.total_pages_visited - 1) as f32 + relevance_score;
        state.statistics.average_relevance_score = total_score / state.statistics.total_pages_visited as f32;
    }

    /// 增加跳过页面计数
    async fn increment_skipped_pages(&self) {
        let mut state = self.crawl_state.write().await;
        state.statistics.skipped_pages_count += 1;
    }

    /// 增加循环检测计数
    async fn increment_loop_detection(&self) {
        let mut state = self.crawl_state.write().await;
        state.statistics.loop_detections += 1;
    }

    /// 打印统计信息
    async fn print_statistics(&self) {
        let state = self.crawl_state.read().await;
        let stats = &state.statistics;
        
        info!("📊 爬虫统计信息:");
        info!("   总页面数: {}", stats.total_pages_visited);
        info!("   相关页面数: {}", stats.relevant_pages_count);
        info!("   跳过页面数: {}", stats.skipped_pages_count);
        info!("   循环检测次数: {}", stats.loop_detections);
        info!("   平均相关性分数: {:.2}", stats.average_relevance_score);
        info!("   总处理时间: {}ms", stats.total_processing_time_ms);
        
        if let Some(end_time) = stats.end_time {
            let duration = end_time.signed_duration_since(stats.start_time);
            info!("   总耗时: {}秒", duration.num_seconds());
        }
    }

    /// 获取爬虫统计
    pub async fn get_statistics(&self) -> CrawlStatistics {
        let state = self.crawl_state.read().await;
        state.statistics.clone()
    }

    /// 停止爬虫
    pub async fn stop_crawling(&self) {
        let mut state = self.crawl_state.write().await;
        state.pending_urls.clear();
        state.statistics.end_time = Some(Utc::now());
        info!("⏹️ 爬虫已手动停止");
    }

    /// 获取任务结果
    pub async fn get_task_results(&self) -> Vec<TaskResult> {
        let state = self.crawl_state.read().await;
        state.task_results.clone()
    }

    /// 清理缓存
    pub async fn clear_cache(&self) {
        self.web_analyzer.clear_cache().await;
        info!("🧹 智能爬虫缓存已清理");
    }
} 