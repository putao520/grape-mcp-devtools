use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Value, json};
use reqwest::Client;
use tracing::{info, warn, debug};
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;
use std::sync::OnceLock;

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    data: Value,
    timestamp: u64,
    ttl: u64, // 生存时间（秒）
}

impl CacheEntry {
    fn new(data: Value, ttl: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self { data, timestamp, ttl }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.timestamp > self.ttl
    }
}

/// API文档获取器trait
#[async_trait]
pub trait DocsFetcher: Send + Sync {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value>;
    fn language(&self) -> &str;
    fn supports_symbol_search(&self) -> bool { false }
}

/// Rust文档获取器
pub struct RustDocsFetcher {
    client: Client,
}

impl RustDocsFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Grape-MCP-DevTools/1.0")
            .build()
            .unwrap();
        Self { client }
    }

    async fn fetch_crate_info(&self, package: &str) -> Result<Value> {
        let url = format!("https://crates.io/api/v1/crates/{}", package);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Rust crate not found: {}", package)).into());
        }

        let data: Value = response.json().await?;
        Ok(data)
    }

    async fn fetch_docs_rs_content(&self, package: &str, version: Option<&str>) -> Result<Value> {
        let url = match version {
            Some(v) => format!("https://docs.rs/{}/{}/", package, v),
            None => format!("https://docs.rs/{}/latest/", package),
        };

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("docs.rs documentation not found: {}", package)).into());
        }

        let html = response.text().await?;
        let cleaned_content = self.extract_documentation_content(&html);
        
        Ok(json!({
            "url": url,
            "content": cleaned_content,
            "source": "docs.rs"
        }))
    }

    fn extract_documentation_content(&self, html: &str) -> Value {
        // 使用简单的文本提取，提取主要文档内容
        let title = if let Some(start) = html.find("<title>") {
            if let Some(end) = html[start..].find("</title>") {
                html[start + 7..start + end].to_string()
            } else {
                "Rust Documentation".to_string()
            }
        } else {
            "Rust Documentation".to_string()
        };

        // 提取描述
        let description = if let Some(start) = html.find(r#"<meta name="description" content=""#) {
            if let Some(end) = html[start..].find(r#"">"#) {
                html[start + 34..start + end].to_string()
            } else {
                "Rust crate documentation".to_string()
            }
        } else {
            "Rust crate documentation".to_string()
        };

        json!({
            "title": title,
            "description": description,
            "format": "html",
            "length": html.len()
        })
    }
}

#[async_trait]
impl DocsFetcher for RustDocsFetcher {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value> {
        debug!("🦀 获取Rust文档: {} :: {}", package, symbol);
        
        // 获取crate基本信息
        let crate_info = self.fetch_crate_info(package).await?;
        
        // 获取docs.rs文档内容
        let docs_content = self.fetch_docs_rs_content(package, version).await?;
        
        // 构建完整响应
        Ok(json!({
            "package": package,
            "symbol": symbol,
            "version": version.unwrap_or("latest"),
            "language": "rust",
            "status": "success",
            "documentation": {
                "api_docs": docs_content,
                "crate_info": crate_info["crate"],
                "symbol_search": if symbol != "*" {
                    format!("Search for '{}' in {}", symbol, package)
                } else {
                    "General documentation".to_string()
                }
            },
            "links": {
                "docs_rs": format!("https://docs.rs/{}", package),
                "crates_io": format!("https://crates.io/crates/{}", package),
                "repository": crate_info["crate"]["repository"].as_str().unwrap_or("")
            },
            "metadata": {
                "downloads": crate_info["crate"]["downloads"].as_u64().unwrap_or(0),
                "recent_downloads": crate_info["crate"]["recent_downloads"].as_u64().unwrap_or(0),
                "description": crate_info["crate"]["description"].as_str().unwrap_or(""),
                "max_stable_version": crate_info["crate"]["max_stable_version"].as_str().unwrap_or("")
            }
        }))
    }

    fn language(&self) -> &str {
        "rust"
    }

    fn supports_symbol_search(&self) -> bool {
        true
    }
}

/// Python文档获取器
pub struct PythonDocsFetcher {
    client: Client,
}

impl PythonDocsFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Grape-MCP-DevTools/1.0")
            .build()
            .unwrap();
        Self { client }
    }

    async fn fetch_pypi_info(&self, package: &str) -> Result<Value> {
        let url = format!("https://pypi.org/pypi/{}/json", package);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Python package not found: {}", package)).into());
        }

        let data: Value = response.json().await?;
        Ok(data)
    }
}

#[async_trait]
impl DocsFetcher for PythonDocsFetcher {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value> {
        debug!("🐍 获取Python文档: {} :: {}", package, symbol);
        
        let pypi_info = self.fetch_pypi_info(package).await?;
        
        let info = &pypi_info["info"];
        let latest_version = info["version"].as_str().unwrap_or("unknown");
        let target_version = version.unwrap_or(latest_version);
        
        Ok(json!({
            "package": package,
            "symbol": symbol,
            "version": target_version,
            "language": "python",
            "status": "success",
            "documentation": {
                "description": info["description"].as_str().unwrap_or(""),
                "summary": info["summary"].as_str().unwrap_or(""),
                "symbol_search": if symbol != "*" {
                    format!("Search for '{}' in {}", symbol, package)
                } else {
                    "Package documentation".to_string()
                },
                "project_urls": info["project_urls"]
            },
            "links": {
                "pypi": format!("https://pypi.org/project/{}/", package),
                "home_page": info["home_page"].as_str().unwrap_or(""),
                "documentation": info["project_urls"]["Documentation"].as_str().unwrap_or(""),
                "repository": info["project_urls"]["Repository"].as_str().unwrap_or("")
            },
            "metadata": {
                "author": info["author"].as_str().unwrap_or(""),
                "license": info["license"].as_str().unwrap_or(""),
                "keywords": info["keywords"].as_str().unwrap_or(""),
                "classifiers": info["classifiers"]
            }
        }))
    }

    fn language(&self) -> &str {
        "python"
    }

    fn supports_symbol_search(&self) -> bool {
        false
    }
}

/// JavaScript/Node.js文档获取器
pub struct JavaScriptDocsFetcher {
    client: Client,
}

impl JavaScriptDocsFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Grape-MCP-DevTools/1.0")
            .build()
            .unwrap();
        Self { client }
    }

    async fn fetch_npm_info(&self, package: &str) -> Result<Value> {
        let url = format!("https://registry.npmjs.org/{}", package);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("npm package not found: {}", package)).into());
        }

        let data: Value = response.json().await?;
        Ok(data)
    }
}

#[async_trait]
impl DocsFetcher for JavaScriptDocsFetcher {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value> {
        debug!("📦 获取JavaScript文档: {} :: {}", package, symbol);
        
        let npm_info = self.fetch_npm_info(package).await?;
        
        let latest_version = npm_info["dist-tags"]["latest"].as_str().unwrap_or("unknown");
        let target_version = version.unwrap_or(latest_version);
        
        let version_info = &npm_info["versions"][target_version];
        
        Ok(json!({
            "package": package,
            "symbol": symbol,
            "version": target_version,
            "language": "javascript",
            "status": "success",
            "documentation": {
                "description": npm_info["description"].as_str().unwrap_or(""),
                "readme": npm_info["readme"].as_str().unwrap_or("").chars().take(500).collect::<String>(),
                "symbol_search": if symbol != "*" {
                    format!("Search for '{}' in {}", symbol, package)
                } else {
                    "Package documentation".to_string()
                }
            },
            "links": {
                "npm": format!("https://www.npmjs.com/package/{}", package),
                "homepage": npm_info["homepage"].as_str().unwrap_or(""),
                "repository": npm_info["repository"]["url"].as_str().unwrap_or(""),
                "bugs": npm_info["bugs"]["url"].as_str().unwrap_or("")
            },
            "metadata": {
                "author": npm_info["author"].as_str().unwrap_or(""),
                "license": npm_info["license"].as_str().unwrap_or(""),
                "keywords": npm_info["keywords"],
                "main": version_info["main"].as_str().unwrap_or(""),
                "dependencies": version_info["dependencies"]
            }
        }))
    }

    fn language(&self) -> &str {
        "javascript"
    }
}

/// Java文档获取器
pub struct JavaDocsFetcher {
    client: Client,
}

impl JavaDocsFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Grape-MCP-DevTools/1.0")
            .build()
            .unwrap();
        Self { client }
    }

    async fn fetch_maven_info(&self, group_id: &str, artifact_id: &str) -> Result<Value> {
        let url = format!("https://search.maven.org/solrsearch/select?q=g:\"{}\" AND a:\"{}\"&core=gav&rows=1&wt=json", group_id, artifact_id);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Maven artifact not found: {}:{}", group_id, artifact_id)).into());
        }

        let data: Value = response.json().await?;
        Ok(data)
    }
}

#[async_trait]
impl DocsFetcher for JavaDocsFetcher {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value> {
        debug!("☕ 获取Java文档: {} :: {}", package, symbol);
        
        // 解析Maven坐标 (groupId:artifactId)
        let parts: Vec<&str> = package.split(':').collect();
        if parts.len() != 2 {
            return Err(MCPError::InvalidParameter(
                format!("Java包名格式应为 'groupId:artifactId'，得到: {}", package)
            ).into());
        }
        
        let (group_id, artifact_id) = (parts[0], parts[1]);
        let maven_info = self.fetch_maven_info(group_id, artifact_id).await?;
        
        let docs = &maven_info["response"]["docs"];
        if docs.as_array().map(|arr| arr.is_empty()).unwrap_or(true) {
            return Err(MCPError::NotFound(format!("Maven artifact not found: {}", package)).into());
        }
        
        let doc = &docs[0];
        let latest_version = doc["latestVersion"].as_str().unwrap_or("unknown");
        let target_version = version.unwrap_or(latest_version);
        
        Ok(json!({
            "package": package,
            "symbol": symbol,
            "version": target_version,
            "language": "java",
            "status": "success",
            "documentation": {
                "group_id": group_id,
                "artifact_id": artifact_id,
                "symbol_search": if symbol != "*" {
                    format!("Search for '{}' in {}", symbol, package)
                } else {
                    "Java documentation".to_string()
                }
            },
            "links": {
                "maven_central": format!("https://search.maven.org/artifact/{}/{}/{}/jar", group_id, artifact_id, target_version),
                "mvnrepository": format!("https://mvnrepository.com/artifact/{}/{}", group_id, artifact_id),
                "javadoc": format!("https://javadoc.io/doc/{}/{}/{}", group_id, artifact_id, target_version)
            },
            "metadata": {
                "latest_version": latest_version,
                "timestamp": doc["timestamp"].as_u64().unwrap_or(0),
                "version_count": doc["versionCount"].as_u64().unwrap_or(0)
            }
        }))
    }

    fn language(&self) -> &str {
        "java"
    }
}

/// Go文档获取器
pub struct GoDocsFetcher {
    client: Client,
}

impl GoDocsFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Grape-MCP-DevTools/1.0")
            .build()
            .unwrap();
        Self { client }
    }

    async fn fetch_pkg_go_dev_info(&self, package: &str) -> Result<Value> {
        let url = format!("https://api.pkg.go.dev/v1/badge/{}", package);
        let response = self.client.get(&url).send().await;
        
        // pkg.go.dev API比较简单，我们主要检查包是否存在
        let exists = response.is_ok() && response.unwrap().status().is_success();
        
        Ok(json!({
            "exists": exists,
            "package": package
        }))
    }
}

#[async_trait]
impl DocsFetcher for GoDocsFetcher {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value> {
        debug!("🐹 获取Go文档: {} :: {}", package, symbol);
        
        let pkg_info = self.fetch_pkg_go_dev_info(package).await?;
        
        if !pkg_info["exists"].as_bool().unwrap_or(false) {
            return Err(MCPError::NotFound(format!("Go package not found: {}", package)).into());
        }
        
        Ok(json!({
            "package": package,
            "symbol": symbol,
            "version": version.unwrap_or("latest"),
            "language": "go",
            "status": "success",
            "documentation": {
                "symbol_search": if symbol != "*" {
                    format!("Search for '{}' in {}", symbol, package)
                } else {
                    "Go package documentation".to_string()
                }
            },
            "links": {
                "pkg_go_dev": format!("https://pkg.go.dev/{}", package),
                "godoc": format!("https://godoc.org/{}", package)
            },
            "metadata": {
                "import_path": package
            }
        }))
    }

    fn language(&self) -> &str {
        "go"
    }
}

/// 文档获取器工厂
pub struct DocsFetcherFactory;

impl DocsFetcherFactory {
    pub fn create(language: &str) -> Result<Box<dyn DocsFetcher>> {
        match language.to_lowercase().as_str() {
            "rust" => Ok(Box::new(RustDocsFetcher::new())),
            "python" => Ok(Box::new(PythonDocsFetcher::new())),
            "javascript" | "js" | "typescript" | "ts" => Ok(Box::new(JavaScriptDocsFetcher::new())),
            "java" => Ok(Box::new(JavaDocsFetcher::new())),
            "go" | "golang" => Ok(Box::new(GoDocsFetcher::new())),
            _ => Err(MCPError::InvalidParameter(format!("不支持的语言: {}", language)).into())
        }
    }

    pub fn supported_languages() -> Vec<&'static str> {
        vec!["rust", "python", "javascript", "typescript", "java", "go"]
    }
}

/// 增强的API文档获取工具
pub struct GetApiDocsTool {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl GetApiDocsTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_cached_or_fetch(&self, cache_key: &str, fetch_fn: impl std::future::Future<Output = Result<Value>>) -> Result<Value> {
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(cache_key) {
                if !entry.is_expired() {
                    debug!("🎯 从缓存返回API文档: {}", cache_key);
                    return Ok(entry.data.clone());
                }
            }
        }

        // 获取新数据
        let result = fetch_fn.await?;

        // 更新缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key.to_string(), CacheEntry::new(result.clone(), 3600)); // 1小时缓存
        }

        Ok(result)
    }

    async fn fetch_api_docs(
        &self,
        language: &str,
        package: &str,
        symbol: &str,
        version: Option<&str>
    ) -> Result<Value> {
        let cache_key = format!("{}:{}:{}:{}", language, package, symbol, version.unwrap_or("latest"));
        
        self.get_cached_or_fetch(&cache_key, async {
            info!("📚 获取API文档: {} {} :: {}", language, package, symbol);
            
            let fetcher = DocsFetcherFactory::create(language)?;
            fetcher.fetch_docs(package, symbol, version).await
        }).await
    }

    /// 清理过期缓存
    pub async fn cleanup_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
        debug!("🧹 清理了过期的API文档缓存");
    }

    /// 获取缓存统计信息
    pub async fn cache_stats(&self) -> Value {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|entry| entry.is_expired()).count();
        
        json!({
            "total_entries": total_entries,
            "expired_entries": expired_entries,
            "active_entries": total_entries - expired_entries
        })
    }
}

#[async_trait]
impl MCPTool for GetApiDocsTool {
    fn name(&self) -> &str {
        "get_api_docs"
    }

    fn description(&self) -> &str {
        "在需要查找特定编程语言包的API文档、函数说明或符号定义时，获取来自官方源的详细API文档信息，支持Rust、Python、JavaScript、Java、Go等多种语言。"
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["language".to_string(), "package".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("编程语言 (rust, python, javascript, java, go)".to_string()),
                        enum_values: Some(DocsFetcherFactory::supported_languages().iter().map(|s| s.to_string()).collect()),
                    }));
                    map.insert("package".to_string(), Schema::String(SchemaString {
                        description: Some("包名称或完整标识符 (如Java的groupId:artifactId)".to_string()),
                        enum_values: None,
                    }));
                    map.insert("symbol".to_string(), Schema::String(SchemaString {
                        description: Some("API符号名称或函数名 (可选，默认为 '*' 表示包级文档)".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("版本号 (可选，默认为最新版本)".to_string()),
                        enum_values: None,
                    }));
                    map
                },
                description: Some("获取API文档的参数".to_string()),
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let language = params["language"].as_str()
            .ok_or_else(|| MCPError::InvalidParameter("language 参数缺失".to_string()))?;
        let package = params["package"].as_str()
            .ok_or_else(|| MCPError::InvalidParameter("package 参数缺失".to_string()))?;
        let symbol = params["symbol"].as_str().unwrap_or("*");
        let version = params["version"].as_str();

        // 验证语言支持
        if !DocsFetcherFactory::supported_languages().contains(&language.to_lowercase().as_str()) {
            return Err(MCPError::InvalidParameter(
                format!("不支持的语言: {}。支持的语言: {:?}", language, DocsFetcherFactory::supported_languages())
            ).into());
        }

        match self.fetch_api_docs(language, package, symbol, version).await {
            Ok(result) => {
                info!("✅ API文档获取成功: {} {}", language, package);
                Ok(result)
            }
            Err(e) => {
                warn!("❌ API文档获取失败: {} {} - {}", language, package, e);
                Err(e)
            }
        }
    }
}
