use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, debug, warn};
use std::sync::Arc;
use std::collections::HashMap;

// 从AI模块导入所需类型
use crate::ai::ai_service::{AIService, AIResponse, AIServiceConfig};
use crate::ai::document_ai::{DocumentAI, CodeExample};
use crate::ai::predicate_ai::{PredicateAI, EvaluatedCondition};
use crate::ai::url_ai::{UrlAI, UrlType, ContentCategory, QualityIndicator};
use crate::ai::task_oriented_crawler::TaskOrientedCrawler;
use crate::ai::smart_url_crawler::{SmartUrlCrawler, CrawlerConfig};
use crate::ai::intelligent_web_analyzer::{IntelligentWebAnalyzer, CrawlTask, ContentType};
use crate::ai::prompt_templates::{DocumentPrompts, PredicatePrompts, UrlPrompts};
use crate::tools::environment_detector::{LanguageInfo, ToolInfo};
use crate::ai::ml_content_analyzer::MLContentAnalyzer;

use chrono::Utc;
use uuid::Uuid;

/// 创建真实的AI服务实例
fn create_real_ai_service() -> Result<AIService> {
    let ai_service = AIService::new(AIServiceConfig {
        api_base: std::env::var("LLM_API_BASE_URL")
            .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".to_string()),
        api_key: std::env::var("LLM_API_KEY").unwrap_or_else(|_| "test-key".to_string()),
        default_model: std::env::var("LLM_MODEL_NAME")
            .unwrap_or_else(|_| "nvidia/llama-3.1-nemotron-70b-instruct".to_string()),
        timeout_secs: 30,
        max_retries: 2,
        enable_cache: false, // 测试中禁用缓存
        cache_ttl_secs: 0,
    })?;
    Ok(ai_service)
}

#[tokio::test]
async fn test_ml_content_analysis() -> Result<()> {
    println!("🧪 测试ML内容分析");
    
    let _ai_service = create_real_ai_service()?;
    let analyzer = MLContentAnalyzer::new();
    
    let test_content = r#"
    # Rust异步编程指南
    
    这是一个关于Rust异步编程的详细教程。
    
    ```rust
    async fn hello_world() {
        println!("Hello, async world!");
    }
    ```
    
    ## 主要特性
    - 高性能异步运行时
    - 零成本抽象
    - 内存安全
    "#;
    
    let result = analyzer.analyze_content(test_content, Some("rust programming")).await?;
    
    assert!(result.quality_score > 0.0);
    assert!(result.relevance_score > 0.0);
    assert!(!result.topics.is_empty());
    assert!(result.language.is_some());
    assert!(!result.recommendations.is_empty());
    
    println!("✅ ML内容分析测试通过");
    println!("   质量分数: {:.2}", result.quality_score);
    println!("   相关性分数: {:.2}", result.relevance_score);
    println!("   检测到的语言: {:?}", result.language);
    println!("   主题: {:?}", result.topics);
    
    Ok(())
} 