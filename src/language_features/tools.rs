use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde_json::{json, Value};
use async_trait::async_trait;
use tracing::{info, warn};

use crate::errors::MCPError;
use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use super::services::{LanguageVersionService, VersionComparisonService};
use super::data_models::FeatureCategory;
use super::doc_crawler::{DocCrawlerEngine, DocCrawlerConfig, LibraryDocumentation};
use super::ai_collector::AICollectorEngine;
use super::intelligent_scraper::IntelligentScraper;
use super::content_analyzer::ChangelogAnalyzer;
use super::url_discovery::URLDiscoveryEngine;

/// 语言特性查询工具
pub struct LanguageFeaturesTool {
    version_service: Arc<LanguageVersionService>,
    comparison_service: Arc<VersionComparisonService>,
}

impl LanguageFeaturesTool {
    pub async fn new() -> Result<Self> {
        let version_service = Arc::new(LanguageVersionService::new().await?);
        let comparison_service = Arc::new(VersionComparisonService::new(version_service.clone()));
        
        Ok(Self {
            version_service,
            comparison_service,
        })
    }
    
    /// 创建Schema
    fn create_schema() -> Schema {
        let mut properties = HashMap::new();
        
        properties.insert(
            "action".to_string(),
            Schema::String(SchemaString {
                description: Some("操作类型".to_string()),
                enum_values: Some(vec![
                    "list_languages".to_string(),
                    "list_versions".to_string(),
                    "get_version".to_string(),
                    "get_latest".to_string(),
                    "search_features".to_string(),
                    "get_syntax_changes".to_string(),
                    "get_breaking_changes".to_string(),
                    "compare_versions".to_string(),
                    "get_timeline".to_string(),
                ]),
            }),
        );
        
        properties.insert(
            "language".to_string(),
            Schema::String(SchemaString {
                description: Some("编程语言名称".to_string()),
                enum_values: None,
            }),
        );
        
        properties.insert(
            "version".to_string(),
            Schema::String(SchemaString {
                description: Some("版本号".to_string()),
                enum_values: None,
            }),
        );
        
        properties.insert(
            "query".to_string(),
            Schema::String(SchemaString {
                description: Some("搜索查询".to_string()),
                enum_values: None,
            }),
        );
        
        properties.insert(
            "category".to_string(),
            Schema::String(SchemaString {
                description: Some("特性类别".to_string()),
                enum_values: Some(vec![
                    "Syntax".to_string(),
                    "StandardLibrary".to_string(),
                    "TypeSystem".to_string(),
                    "Async".to_string(),
                    "Memory".to_string(),
                    "ErrorHandling".to_string(),
                    "Modules".to_string(),
                    "Macros".to_string(),
                    "Toolchain".to_string(),
                    "Performance".to_string(),
                    "Security".to_string(),
                ]),
            }),
        );
        
        properties.insert(
            "from_version".to_string(),
            Schema::String(SchemaString {
                description: Some("起始版本（用于比较）".to_string()),
                enum_values: None,
            }),
        );
        
        properties.insert(
            "to_version".to_string(),
            Schema::String(SchemaString {
                description: Some("目标版本（用于比较）".to_string()),
                enum_values: None,
            }),
        );
        
        properties.insert(
            "since_version".to_string(),
            Schema::String(SchemaString {
                description: Some("起始版本（用于时间线）".to_string()),
                enum_values: None,
            }),
        );
        
        Schema::Object(SchemaObject {
            properties,
            required: vec!["action".to_string()],
            description: Some("语言特性查询工具参数".to_string()),
        })
    }
    
    async fn handle_list_languages(&self) -> Result<Value> {
        let languages = self.version_service.get_supported_languages();
        Ok(json!({
            "action": "list_languages",
            "supported_languages": languages,
            "count": languages.len()
        }))
    }
    
    async fn handle_list_versions(&self, language: &str) -> Result<Value> {
        let versions = self.version_service.get_language_versions(language).await?;
        Ok(json!({
            "action": "list_versions",
            "language": language,
            "versions": versions,
            "count": versions.len()
        }))
    }
    
    async fn handle_get_version(&self, language: &str, version: &str) -> Result<Value> {
        let version_details = self.version_service.get_version_details(language, version).await?;
        Ok(json!({
            "action": "get_version",
            "language": language,
            "version": version,
            "details": version_details
        }))
    }
    
    async fn handle_get_latest(&self, language: &str) -> Result<Value> {
        let latest_version = self.version_service.get_latest_version(language).await?;
        Ok(json!({
            "action": "get_latest",
            "language": language,
            "latest_version": latest_version
        }))
    }
    
    async fn handle_search_features(
        &self,
        language: &str,
        version: Option<&str>,
        query: &str,
        category: Option<&str>,
    ) -> Result<Value> {
        let feature_category = if let Some(cat_str) = category {
            match cat_str {
                "Syntax" => Some(FeatureCategory::Syntax),
                "StandardLibrary" => Some(FeatureCategory::StandardLibrary),
                "TypeSystem" => Some(FeatureCategory::TypeSystem),
                "Async" => Some(FeatureCategory::Async),
                "Memory" => Some(FeatureCategory::Memory),
                "ErrorHandling" => Some(FeatureCategory::ErrorHandling),
                "Modules" => Some(FeatureCategory::Modules),
                "Macros" => Some(FeatureCategory::Macros),
                "Toolchain" => Some(FeatureCategory::Toolchain),
                "Performance" => Some(FeatureCategory::Performance),
                "Security" => Some(FeatureCategory::Security),
                _ => None,
            }
        } else {
            None
        };
        
        let features = self.version_service
            .search_features(language, version, query, feature_category)
            .await?;
            
        Ok(json!({
            "action": "search_features",
            "language": language,
            "version": version,
            "query": query,
            "category": category,
            "features": features,
            "count": features.len()
        }))
    }
    
    async fn handle_get_syntax_changes(&self, language: &str, version: &str) -> Result<Value> {
        let syntax_changes = self.version_service.get_syntax_changes(language, version).await?;
        Ok(json!({
            "action": "get_syntax_changes",
            "language": language,
            "version": version,
            "syntax_changes": syntax_changes,
            "count": syntax_changes.len()
        }))
    }
    
    async fn handle_get_breaking_changes(&self, language: &str, version: &str) -> Result<Value> {
        let breaking_changes = self.version_service.get_breaking_changes(language, version).await?;
        Ok(json!({
            "action": "get_breaking_changes",
            "language": language,
            "version": version,
            "breaking_changes": breaking_changes,
            "count": breaking_changes.len()
        }))
    }
    
    async fn handle_compare_versions(
        &self,
        language: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<Value> {
        let comparison = self.comparison_service
            .compare_versions(language, from_version, to_version)
            .await?;
            
        Ok(json!({
            "action": "compare_versions",
            "comparison": comparison
        }))
    }
    
    async fn handle_get_timeline(
        &self,
        language: &str,
        since_version: Option<&str>,
    ) -> Result<Value> {
        let timeline = self.comparison_service
            .get_version_timeline(language, since_version)
            .await?;
            
        Ok(json!({
            "action": "get_timeline",
            "language": language,
            "since_version": since_version,
            "timeline": timeline,
            "count": timeline.len()
        }))
    }
}

#[async_trait]
impl MCPTool for LanguageFeaturesTool {
    fn name(&self) -> &str {
        "language_features"
    }
    
    fn description(&self) -> &str {
        "查询编程语言版本特性、语法变化和版本比较的工具"
    }
    
    fn parameters_schema(&self) -> &Schema {
        Box::leak(Box::new(Self::create_schema()))
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::InvalidParameter("缺少action参数".to_string()))?;
            
        info!("🔍 执行语言特性查询: {}", action);
        
        match action {
            "list_languages" => {
                self.handle_list_languages().await
            }
            
            "list_versions" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                self.handle_list_versions(language).await
            }
            
            "get_version" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                let version = params.get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少version参数".to_string()))?;
                self.handle_get_version(language, version).await
            }
            
            "get_latest" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                self.handle_get_latest(language).await
            }
            
            "search_features" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                let version = params.get("version").and_then(|v| v.as_str());
                let query = params.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少query参数".to_string()))?;
                let category = params.get("category").and_then(|v| v.as_str());
                self.handle_search_features(language, version, query, category).await
            }
            
            "get_syntax_changes" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                let version = params.get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少version参数".to_string()))?;
                self.handle_get_syntax_changes(language, version).await
            }
            
            "get_breaking_changes" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                let version = params.get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少version参数".to_string()))?;
                self.handle_get_breaking_changes(language, version).await
            }
            
            "compare_versions" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                let from_version = params.get("from_version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少from_version参数".to_string()))?;
                let to_version = params.get("to_version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少to_version参数".to_string()))?;
                self.handle_compare_versions(language, from_version, to_version).await
            }
            
            "get_timeline" => {
                let language = params.get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;
                let since_version = params.get("since_version").and_then(|v| v.as_str());
                self.handle_get_timeline(language, since_version).await
            }
            
            _ => Err(anyhow::anyhow!("不支持的操作: {}", action))
        }
        .map(|mut result| {
            result["status"] = json!("success");
            result["timestamp"] = json!(chrono::Utc::now().to_rfc3339());
            result
        })
    }
}

/// HTTP库文档识别与爬取工具
pub struct HttpDocCrawlTool {
    engine: DocCrawlerEngine,
    ai_collector: AICollectorEngine,
}

impl HttpDocCrawlTool {
    /// 创建新的HTTP文档爬取工具
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let http_client = reqwest::Client::new();
        
        // 创建核心组件
        let scraper = Arc::new(IntelligentScraper::new(http_client.clone(), true).await?);
        let analyzer = Arc::new(ChangelogAnalyzer::new(std::env::var("OPENAI_API_KEY").ok()).await?);
        let _url_discovery = Arc::new(URLDiscoveryEngine::new(http_client.clone()).await?);
        
        // 创建AI采集引擎
        let ai_collector = AICollectorEngine::new(Default::default()).await?;
        
        // 创建文档爬取引擎
        let doc_config = DocCrawlerConfig {
            max_crawl_depth: 3,
            max_pages_per_library: 100,
            concurrent_limit: 8,
            cache_ttl_hours: 48,
            enable_ai_analysis: true,
            content_quality_threshold: 0.6,
        };
        
        let engine = DocCrawlerEngine::new(
            http_client.clone(),
            scraper.clone(),
            analyzer,
            doc_config,
        ).await?;
        
        Ok(Self {
            engine,
            ai_collector,
        })
    }

    /// 智能爬取库文档
    pub async fn crawl_library(&self, library_name: &str, language: &str, base_urls: Option<Vec<String>>) -> Result<LibraryDocumentation, Box<dyn std::error::Error>> {
        // 如果没有提供base_urls，尝试自动发现
        let urls = if let Some(urls) = base_urls {
            urls
        } else {
            self.discover_library_urls(library_name, language).await?
        };
        
        info!("🔍 开始智能爬取库文档: {} ({})", library_name, language);
        info!("📍 使用URL: {:?}", urls);
        
        let documentation = self.engine.crawl_library_documentation(library_name, language, urls).await?;
        
        info!("✅ 文档爬取完成，质量分数: {:.2}", documentation.metadata.quality_score);
        Ok(documentation)
    }

    /// 自动发现库URL
    async fn discover_library_urls(&self, library_name: &str, language: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut urls = Vec::new();
        
        // 1. 包管理器URL
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
                urls.push(format!("https://search.maven.org/search?q=a:{}", library_name));
            }
            "go" => {
                urls.push(format!("https://pkg.go.dev/{}", library_name));
            }
            _ => {}
        }
        
        // 2. GitHub搜索
        urls.push(format!("https://github.com/search?q={}&type=repositories", library_name));
        
        // 3. 文档站点猜测
        let common_docs_patterns = vec![
            format!("https://{}.readthedocs.io", library_name),
            format!("https://{}.github.io", library_name),
            format!("https://docs.{}.org", library_name),
            format!("https://{}.org", library_name),
        ];
        urls.extend(common_docs_patterns);
        
        Ok(urls)
    }

    /// 批量爬取多个库
    pub async fn crawl_multiple_libraries(&self, libraries: Vec<(String, String, Option<Vec<String>>)>) -> Result<Vec<LibraryDocumentation>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        for (library_name, language, base_urls) in libraries {
            match self.crawl_library(&library_name, &language, base_urls).await {
                Ok(doc) => results.push(doc),
                Err(e) => {
                    warn!("⚠️ 爬取库 {} 失败: {}", library_name, e);
                }
            }
        }
        
        Ok(results)
    }

    /// 获取爬取统计信息
    pub async fn get_crawl_stats(&self) -> serde_json::Value {
        let doc_stats = self.engine.get_cache_stats().await;
        let ai_stats = self.ai_collector.get_collection_stats().await;
        
        json!({
            "doc_crawler": {
                "cached_libraries": doc_stats.cached_libraries,
                "total_cache_size": doc_stats.total_cache_size,
                "average_quality_score": doc_stats.average_quality_score
            },
            "ai_collector": {
                "supported_languages": ai_stats.supported_languages,
                "cached_results": ai_stats.cached_results,
                "total_data_sources": ai_stats.total_data_sources
            }
        })
    }
} 