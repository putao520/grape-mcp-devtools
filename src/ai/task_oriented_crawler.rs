use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, debug, warn, error};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::ai_service::AIService;
use super::intelligent_web_analyzer::{
    IntelligentWebAnalyzer, CrawlTask, ContentType, PageRelevanceAnalysis
};
use super::smart_url_crawler::{SmartUrlCrawler, TaskResult, CrawlStatistics, CrawlerConfig};

/// 任务导向的爬虫管理器
/// 整合智能网页分析和URL遍历，提供完整的目标导向爬虫解决方案
pub struct TaskOrientedCrawler {
    ai_service: AIService,
    smart_crawler: SmartUrlCrawler,
    task_templates: HashMap<String, CrawlTaskTemplate>,
}

/// 爬虫任务模板
/// 预定义常见的爬虫场景配置
#[derive(Debug, Clone)]
pub struct CrawlTaskTemplate {
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 编程语言
    pub programming_language: String,
    /// 期望的内容类型
    pub expected_content_types: Vec<ContentType>,
    /// 推荐配置
    pub recommended_config: CrawlerConfig,
    /// 示例起始URL模式
    pub url_patterns: Vec<String>,
}

/// 任务执行结果
#[derive(Debug, Clone)]
pub struct TaskExecutionResult {
    /// 任务信息
    pub task: CrawlTask,
    /// 爬虫配置
    pub config: CrawlerConfig,
    /// 任务结果
    pub results: Vec<TaskResult>,
    /// 统计信息
    pub statistics: CrawlStatistics,
    /// 智能摘要
    pub intelligent_summary: String,
    /// 关键发现
    pub key_findings: Vec<KeyFinding>,
    /// 推荐的后续行动
    pub recommended_actions: Vec<String>,
}

/// 关键发现
#[derive(Debug, Clone)]
pub struct KeyFinding {
    /// 发现类型
    pub finding_type: FindingType,
    /// 标题
    pub title: String,
    /// 描述
    pub description: String,
    /// 相关URL
    pub urls: Vec<String>,
    /// 重要性分数
    pub importance_score: f32,
}

/// 发现类型
#[derive(Debug, Clone)]
pub enum FindingType {
    Documentation,
    Tutorial,
    ApiReference,
    Example,
    Installation,
    Troubleshooting,
    Community,
    HighQualityContent,
    OfficialResource,
}

impl TaskOrientedCrawler {
    /// 创建新的任务导向爬虫
    pub async fn new(ai_service: AIService, config: CrawlerConfig) -> Result<Self> {
        let smart_crawler = SmartUrlCrawler::new(ai_service.clone(), config).await?;
        
        let mut task_templates = HashMap::new();
        Self::init_default_templates(&mut task_templates);

        info!("🎯 任务导向爬虫初始化完成");
        info!("📚 已加载 {} 个任务模板", task_templates.len());

        Ok(Self {
            ai_service,
            smart_crawler,
            task_templates,
        })
    }

    /// 为特定库创建文档搜集任务
    pub async fn create_library_documentation_task(
        &self,
        library_name: &str,
        programming_language: &str,
        start_url: &str,
        custom_description: Option<String>,
    ) -> Result<CrawlTask> {
        let task_id = Uuid::new_v4().to_string();
        
        let target_description = custom_description.unwrap_or_else(|| {
            format!(
                "为{}库收集完整的文档、API参考、教程和代码示例，重点关注使用指南和最佳实践",
                library_name
            )
        });

        let task = CrawlTask {
            task_id,
            target_description,
            start_url: start_url.to_string(),
            library_name: library_name.to_string(),
            programming_language: programming_language.to_string(),
            expected_content_types: vec![
                ContentType::Documentation,
                ContentType::ApiReference,
                ContentType::Tutorial,
                ContentType::Examples,
                ContentType::GettingStarted,
                ContentType::Installation,
            ],
            max_depth: 4,
            max_pages: 50,
            created_at: Utc::now(),
        };

        info!("📋 创建文档搜集任务: {} ({})", library_name, programming_language);
        Ok(task)
    }

    /// 为特定技术创建学习路径任务
    pub async fn create_learning_path_task(
        &self,
        technology: &str,
        learning_level: &str, // "beginner", "intermediate", "advanced"
        start_url: &str,
    ) -> Result<CrawlTask> {
        let task_id = Uuid::new_v4().to_string();
        
        let target_description = format!(
            "为{}技术创建{}级别的学习路径，收集教程、示例、最佳实践和进阶指南",
            technology, learning_level
        );

        let expected_types = match learning_level {
            "beginner" => vec![
                ContentType::GettingStarted,
                ContentType::Tutorial,
                ContentType::Installation,
                ContentType::Examples,
            ],
            "intermediate" => vec![
                ContentType::Tutorial,
                ContentType::Examples,
                ContentType::Documentation,
                ContentType::ApiReference,
            ],
            "advanced" => vec![
                ContentType::ApiReference,
                ContentType::Examples,
                ContentType::Configuration,
                ContentType::Troubleshooting,
            ],
            _ => vec![ContentType::Documentation, ContentType::Tutorial],
        };

        let task = CrawlTask {
            task_id,
            target_description,
            start_url: start_url.to_string(),
            library_name: technology.to_string(),
            programming_language: "general".to_string(),
            expected_content_types: expected_types,
            max_depth: 5,
            max_pages: 75,
            created_at: Utc::now(),
        };

        info!("🎓 创建学习路径任务: {} ({}级别)", technology, learning_level);
        Ok(task)
    }

    /// 为问题解决创建故障排除任务
    pub async fn create_troubleshooting_task(
        &self,
        technology: &str,
        problem_description: &str,
        start_url: &str,
    ) -> Result<CrawlTask> {
        let task_id = Uuid::new_v4().to_string();
        
        let target_description = format!(
            "为{}的问题'{}' 收集故障排除指南、解决方案和相关讨论",
            technology, problem_description
        );

        let task = CrawlTask {
            task_id,
            target_description,
            start_url: start_url.to_string(),
            library_name: technology.to_string(),
            programming_language: "general".to_string(),
            expected_content_types: vec![
                ContentType::Troubleshooting,
                ContentType::Community,
                ContentType::Examples,
                ContentType::Documentation,
            ],
            max_depth: 3,
            max_pages: 30,
            created_at: Utc::now(),
        };

        info!("🔧 创建故障排除任务: {}", problem_description);
        Ok(task)
    }

    /// 执行任务并生成智能结果
    pub async fn execute_task_with_intelligence(
        &self,
        task: CrawlTask,
        config: Option<CrawlerConfig>,
    ) -> Result<TaskExecutionResult> {
        let crawler_config = config.unwrap_or_else(|| self.get_optimal_config_for_task(&task));
        
        info!("🚀 开始执行智能任务: {}", task.target_description);
        info!("⚙️ 使用配置: 深度={}, 页面数={}, 最小相关性={}", 
              task.max_depth, task.max_pages, crawler_config.min_relevance_score);

        // 执行爬虫任务
        let results = self.smart_crawler.execute_task(task.clone(), crawler_config.clone()).await?;
        let statistics = self.smart_crawler.get_statistics().await;

        info!("📊 爬虫完成，处理了{}个页面，发现{}个相关页面", 
              statistics.total_pages_visited, statistics.relevant_pages_count);

        // 生成智能分析
        let intelligent_summary = self.generate_intelligent_summary(&task, &results).await?;
        let key_findings = self.extract_key_findings(&task, &results).await?;
        let recommended_actions = self.generate_recommended_actions(&task, &results, &statistics).await?;

        info!("🧠 智能分析完成，发现{}个关键点", key_findings.len());

        Ok(TaskExecutionResult {
            task,
            config: crawler_config,
            results,
            statistics,
            intelligent_summary,
            key_findings,
            recommended_actions,
        })
    }

    /// 生成智能摘要
    async fn generate_intelligent_summary(&self, task: &CrawlTask, results: &[TaskResult]) -> Result<String> {
        info!("📝 生成智能任务摘要");

        // 筛选高质量结果
        let high_quality_results: Vec<_> = results
            .iter()
            .filter(|r| r.relevance_analysis.relevance_score > 0.7)
            .collect();

        if high_quality_results.is_empty() {
            return Ok("未找到足够相关的内容来生成摘要。".to_string());
        }

        // 构建摘要内容
        let content_summaries: Vec<String> = high_quality_results
            .iter()
            .map(|r| format!("URL: {}\n摘要: {}\n", r.url, r.content_summary))
            .collect();

        let combined_content = content_summaries.join("\n---\n");

        // 使用AI生成智能摘要
        let system_prompt = self.get_intelligent_summary_prompt();
        let user_message = format!(
            r#"任务目标：{}
目标库：{}
编程语言：{}

收集到的内容摘要：
{}

请生成一个专业的、结构化的智能摘要，突出关键发现和价值信息。"#,
            task.target_description,
            task.library_name,
            task.programming_language,
            combined_content
        );

        let ai_request = super::ai_service::AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(2000),
            stream: false,
        };

        let response = self.ai_service.request(ai_request).await?;
        Ok(response.content)
    }

    /// 提取关键发现
    async fn extract_key_findings(&self, task: &CrawlTask, results: &[TaskResult]) -> Result<Vec<KeyFinding>> {
        info!("🔍 提取关键发现");

        let mut findings = Vec::new();

        // 基于内容类型分组分析
        let mut content_type_groups: HashMap<String, Vec<&TaskResult>> = HashMap::new();
        
        for result in results {
            for content_type in &result.relevance_analysis.detected_content_types {
                let key = format!("{:?}", content_type);
                content_type_groups.entry(key).or_default().push(result);
            }
        }

        // 为每个内容类型生成发现
        for (content_type, group_results) in content_type_groups {
            if group_results.len() >= 2 { // 至少有2个相关结果才算发现
                let urls: Vec<String> = group_results.iter().map(|r| r.url.clone()).collect();
                let avg_score: f32 = group_results.iter()
                    .map(|r| r.relevance_analysis.relevance_score)
                    .sum::<f32>() / group_results.len() as f32;

                let finding_type = self.map_content_type_to_finding(&content_type);
                
                findings.push(KeyFinding {
                    finding_type,
                    title: format!("发现{}个{}相关资源", group_results.len(), content_type),
                    description: format!("在{}个页面中发现了相关的{}内容，平均相关性分数为{:.2}", 
                                       group_results.len(), content_type, avg_score),
                    urls,
                    importance_score: avg_score,
                });
            }
        }

        // 识别高质量官方资源
        let official_results: Vec<_> = results
            .iter()
            .filter(|r| r.relevance_analysis.relevance_score > 0.9 || 
                       r.url.contains("official") || r.url.contains("docs"))
            .collect();

        if !official_results.is_empty() {
            findings.push(KeyFinding {
                finding_type: FindingType::OfficialResource,
                title: format!("发现{}个高质量官方资源", official_results.len()),
                description: "这些资源具有很高的权威性和可信度".to_string(),
                urls: official_results.iter().map(|r| r.url.clone()).collect(),
                importance_score: 0.95,
            });
        }

        // 按重要性排序
        findings.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap());

        info!("✅ 提取了{}个关键发现", findings.len());
        Ok(findings)
    }

    /// 生成推荐行动
    async fn generate_recommended_actions(&self, task: &CrawlTask, results: &[TaskResult], statistics: &CrawlStatistics) -> Result<Vec<String>> {
        let mut actions = Vec::new();

        // 基于统计信息的建议
        if statistics.relevant_pages_count < 5 {
            actions.push("建议扩大搜索范围或调整搜索关键词，当前相关内容较少".to_string());
        }

        if statistics.average_relevance_score < 0.6 {
            actions.push("建议优化搜索策略，当前内容相关性偏低".to_string());
        }

        // 基于内容类型的建议
        let has_docs = results.iter().any(|r| r.relevance_analysis.detected_content_types.contains(&ContentType::Documentation));
        let has_tutorials = results.iter().any(|r| r.relevance_analysis.detected_content_types.contains(&ContentType::Tutorial));
        let has_examples = results.iter().any(|r| r.relevance_analysis.detected_content_types.contains(&ContentType::Examples));

        if !has_docs {
            actions.push(format!("建议专门搜索{}的官方文档", task.library_name));
        }

        if !has_tutorials {
            actions.push(format!("建议寻找{}的教程和入门指南", task.library_name));
        }

        if !has_examples {
            actions.push(format!("建议收集{}的代码示例和用例", task.library_name));
        }

        // 通用建议
        if results.len() > 10 {
            actions.push("建议对收集的内容进行分类整理和优先级排序".to_string());
        }

        if actions.is_empty() {
            actions.push("当前搜索结果良好，建议继续深入研究相关内容".to_string());
        }

        Ok(actions)
    }

    /// 获取任务的最优配置
    pub fn get_optimal_config_for_task(&self, task: &CrawlTask) -> CrawlerConfig {
        let mut config = CrawlerConfig::default();

        // 根据任务类型调整配置
        if task.expected_content_types.contains(&ContentType::Documentation) {
            config.min_relevance_score = 0.6; // 文档要求较高相关性
            config.max_retries = 3;
        }

        if task.expected_content_types.contains(&ContentType::Tutorial) {
            config.delay_ms = 1500; // 教程内容通常需要更多时间加载
        }

        if task.expected_content_types.contains(&ContentType::Troubleshooting) {
            config.min_relevance_score = 0.4; // 故障排除允许较低相关性
            config.concurrency = 2; // 减少并发避免被限制
        }

        config
    }

    /// 映射内容类型到发现类型
    fn map_content_type_to_finding(&self, content_type: &str) -> FindingType {
        match content_type {
            "Documentation" => FindingType::Documentation,
            "Tutorial" => FindingType::Tutorial,
            "ApiReference" => FindingType::ApiReference,
            "Examples" => FindingType::Example,
            "Installation" => FindingType::Installation,
            "Troubleshooting" => FindingType::Troubleshooting,
            "Community" => FindingType::Community,
            _ => FindingType::HighQualityContent,
        }
    }

    /// 获取智能摘要系统提示词
    fn get_intelligent_summary_prompt(&self) -> String {
        r#"你是一个专业的技术内容分析专家。你需要分析爬虫收集的技术内容，并生成一个智能的、结构化的摘要。

摘要要求：
1. 突出最重要和最有价值的发现
2. 按内容类型分类整理（文档、教程、API、示例等）
3. 识别关键的技术特性和使用方法
4. 提供清晰的结构和要点
5. 保持专业性和技术准确性

请生成一个专业的智能摘要，帮助用户快速理解收集到的内容价值。"#.to_string()
    }

    /// 初始化默认任务模板
    fn init_default_templates(templates: &mut HashMap<String, CrawlTaskTemplate>) {
        // Rust库文档模板
        templates.insert("rust_library".to_string(), CrawlTaskTemplate {
            name: "Rust库文档收集".to_string(),
            description: "收集Rust crate的完整文档、示例和使用指南".to_string(),
            programming_language: "rust".to_string(),
            expected_content_types: vec![
                ContentType::Documentation,
                ContentType::ApiReference,
                ContentType::Examples,
                ContentType::GettingStarted,
            ],
            recommended_config: CrawlerConfig {
                min_relevance_score: 0.6,
                max_retries: 3,
                delay_ms: 1000,
                ..Default::default()
            },
            url_patterns: vec![
                "docs.rs/*".to_string(),
                "crates.io/*".to_string(),
                "github.com/*/tree/*/examples".to_string(),
            ],
        });

        // JavaScript/Node.js库模板
        templates.insert("javascript_library".to_string(), CrawlTaskTemplate {
            name: "JavaScript库文档收集".to_string(),
            description: "收集npm包的文档、教程和代码示例".to_string(),
            programming_language: "javascript".to_string(),
            expected_content_types: vec![
                ContentType::Documentation,
                ContentType::Tutorial,
                ContentType::Examples,
                ContentType::Installation,
            ],
            recommended_config: CrawlerConfig {
                min_relevance_score: 0.5,
                delay_ms: 1200,
                ..Default::default()
            },
            url_patterns: vec![
                "npmjs.com/package/*".to_string(),
                "github.com/*/blob/*/README.md".to_string(),
                "*.github.io/*".to_string(),
            ],
        });

        // Python库模板
        templates.insert("python_library".to_string(), CrawlTaskTemplate {
            name: "Python库文档收集".to_string(),
            description: "收集PyPI包的文档、教程和使用示例".to_string(),
            programming_language: "python".to_string(),
            expected_content_types: vec![
                ContentType::Documentation,
                ContentType::Tutorial,
                ContentType::ApiReference,
                ContentType::Examples,
            ],
            recommended_config: CrawlerConfig {
                min_relevance_score: 0.6,
                delay_ms: 1000,
                ..Default::default()
            },
            url_patterns: vec![
                "pypi.org/project/*".to_string(),
                "readthedocs.io/*".to_string(),
                "*.readthedocs.io/*".to_string(),
            ],
        });
    }

    /// 获取可用的任务模板
    pub fn get_available_templates(&self) -> Vec<&CrawlTaskTemplate> {
        self.task_templates.values().collect()
    }

    /// 根据模板创建任务
    pub fn create_task_from_template(
        &self,
        template_name: &str,
        library_name: &str,
        start_url: &str,
        custom_description: Option<String>,
    ) -> Result<CrawlTask> {
        let template = self.task_templates.get(template_name)
            .ok_or_else(|| anyhow::anyhow!("未找到模板: {}", template_name))?;

        let task_id = Uuid::new_v4().to_string();
        
        let target_description = custom_description.unwrap_or_else(|| {
            format!("为{}库{}，{}", library_name, template.description, template.name)
        });

        Ok(CrawlTask {
            task_id,
            target_description,
            start_url: start_url.to_string(),
            library_name: library_name.to_string(),
            programming_language: template.programming_language.clone(),
            expected_content_types: template.expected_content_types.clone(),
            max_depth: 4,
            max_pages: 50,
            created_at: Utc::now(),
        })
    }

    /// 清理所有缓存
    pub async fn clear_all_cache(&self) {
        self.smart_crawler.clear_cache().await;
        info!("🧹 任务导向爬虫所有缓存已清理");
    }
} 