use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::errors::MCPError;
use super::base::{MCPTool, ToolAnnotations, Schema, SchemaObject, SchemaString, SchemaNumber};

pub struct SearchDocsTools {
    _annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, (Value, DateTime<Utc>)>>>,
    client: reqwest::Client,
}

impl SearchDocsTools {
    pub fn new() -> Self {
        Self {            
            _annotations: ToolAnnotations {
                category: "文档搜索".to_string(),
                tags: vec!["文档".to_string(), "搜索".to_string()],
                version: "1.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::new(),
        }
    }
    
    fn validate_params(&self, params: &Value) -> Result<()> {
        if params["query"].as_str().is_none() {
            return Err(MCPError::InvalidParameter("缺少query参数".to_string()).into());
        }
        
        if params["language"].as_str().is_none() {
            return Err(MCPError::InvalidParameter("缺少language参数".to_string()).into());
        }
        
        Ok(())
    }
    
    async fn search_or_get_cached(&self, query: &str, language: &str) -> Result<Value> {
        let cache_key = format!("{}:{}", language, query);
        let cache_ttl = chrono::Duration::hours(1);
        
        {
            let cache = self.cache.read().await;
            if let Some((cached_result, timestamp)) = cache.get(&cache_key) {
                if Utc::now() - *timestamp < cache_ttl {
                    tracing::debug!("从缓存返回搜索结果: {}", cache_key);
                    return Ok(cached_result.clone());
                }
            }
        }
        
        let results = self.perform_search(query, language).await?;
        
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, (results.clone(), Utc::now()));
        }
        
        Ok(results)
    }
    
    async fn perform_search(&self, query: &str, language: &str) -> Result<Value> {
        tracing::info!("执行文档搜索: {} (语言: {})", query, language);
        
        let results = match language.to_lowercase().as_str() {
            "rust" => self.search_rust_docs(query).await?,
            "python" => self.search_python_docs(query).await?,
            "javascript" | "js" | "typescript" | "ts" => self.search_js_docs(query).await?,
            "go" => self.search_go_docs(query).await?,
            "java" => self.search_java_docs(query).await?,
            _ => self.search_generic_docs(query, language).await?,
        };
        
        Ok(results)
    }
    
    async fn search_rust_docs(&self, query: &str) -> Result<Value> {
        let mut results = Vec::new();
        
        if let Ok(docs_rs_results) = self.search_docs_rs(query).await {
            results.extend(docs_rs_results);
        }
        
        if let Ok(std_results) = self.search_rust_std(query).await {
            results.extend(std_results);
        }
        
        if results.is_empty() {
            results.push(json!({
                "title": format!("Rust 官方文档: {}", query),
                "content": format!("在 Rust 官方文档中搜索 '{}'，包括标准库、语言特性和最佳实践", query),
                "relevance": 0.7,
                "source": "rust_docs",
                "url": format!("https://doc.rust-lang.org/std/?search={}", urlencoding::encode(query))
            }));
        }
        
        results.push(json!({
            "title": format!("Rust GitHub 代码示例: {}", query),
            "content": format!("在 GitHub 上搜索关于 '{}' 的 Rust 代码示例和实际应用", query),
            "relevance": 0.8,
            "source": "github",
            "url": format!("https://github.com/search?q={}&l=rust", urlencoding::encode(query))
        }));
        
        results.push(json!({
            "title": format!("Rust by Example: {}", query),
            "content": format!("Rust by Example 中关于 '{}' 的教程和代码示例", query),
            "relevance": 0.9,
            "source": "rust_by_example",
            "url": format!("https://doc.rust-lang.org/rust-by-example/?search={}", urlencoding::encode(query))
        }));
        
        results.push(json!({
            "title": format!("The Rust Programming Language: {}", query),
            "content": format!("The Rust Book 中关于 '{}' 的详细说明和概念解释", query),
            "relevance": 0.85,
            "source": "rust_book",
            "url": format!("https://doc.rust-lang.org/book/?search={}", urlencoding::encode(query))
        }));
        
        Ok(json!({
            "results": results,
            "total_hits": results.len(),
            "language": "rust",
            "search_quality": "comprehensive",
            "summary": format!("找到 {} 个关于 '{}' 的 Rust 相关文档和资源", results.len(), query)
        }))
    }
    
    async fn search_docs_rs(&self, query: &str) -> Result<Vec<Value>> {
        let url = format!("https://docs.rs/releases/search?query={}", urlencoding::encode(query));
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(data) = response.json::<Value>().await {
                    let mut results = Vec::new();
                    
                    if let Some(releases) = data.as_array() {
                        for release in releases.iter().take(5) {
                            if let (Some(name), Some(description)) = (
                                release["name"].as_str(),
                                release["description"].as_str()
                            ) {
                                results.push(json!({
                                    "title": format!("Rust crate: {}", name),
                                    "content": description,
                                    "relevance": 0.9,
                                    "source": "docs.rs",
                                    "url": format!("https://docs.rs/{}", name)
                                }));
                            }
                        }
                    }
                    
                    Ok(results)
                } else {
                    Ok(Vec::new())
                }
            },
            _ => Ok(Vec::new())
        }
    }
    
    async fn search_rust_std(&self, query: &str) -> Result<Vec<Value>> {
        Ok(vec![
            json!({
                "title": format!("Rust std::{}", query),
                "content": format!("Rust 标准库中的 {} 相关功能", query),
                "relevance": 0.8,
                "source": "rust_std",
                "url": format!("https://doc.rust-lang.org/std/?search={}", urlencoding::encode(query))
            })
        ])
    }
    
    async fn search_python_docs(&self, query: &str) -> Result<Value> {
        let mut results = Vec::new();
        
        if let Ok(pypi_results) = self.search_pypi(query).await {
            results.extend(pypi_results);
        }
        
        results.push(json!({
            "title": format!("Python 官方文档: {}", query),
            "content": format!("Python 官方文档中关于 '{}' 的完整说明，包括语法、用法和示例", query),
            "relevance": 0.8,
            "source": "python_docs",
            "url": format!("https://docs.python.org/3/search.html?q={}", urlencoding::encode(query))
        }));
        
        results.push(json!({
            "title": format!("PyPI 包搜索: {}", query),
            "content": format!("在 Python Package Index 中搜索与 '{}' 相关的包和库", query),
            "relevance": 0.9,
            "source": "pypi_search",
            "url": format!("https://pypi.org/search/?q={}", urlencoding::encode(query))
        }));
        
        results.push(json!({
            "title": format!("Python 教程: {}", query),
            "content": format!("Real Python 和官方教程中关于 '{}' 的学习资源", query),
            "relevance": 0.85,
            "source": "python_tutorial",
            "url": format!("https://realpython.com/search/?q={}", urlencoding::encode(query))
        }));
        
        Ok(json!({
            "results": results,
            "total_hits": results.len(),
            "language": "python",
            "search_quality": "comprehensive",
            "summary": format!("找到 {} 个关于 '{}' 的 Python 相关文档和资源", results.len(), query)
        }))
    }
    
    async fn search_pypi(&self, query: &str) -> Result<Vec<Value>> {
        let url = format!("https://pypi.org/search/?q={}", urlencoding::encode(query));
        
        Ok(vec![
            json!({
                "title": format!("PyPI 包搜索: {}", query),
                "content": format!("在 PyPI 中搜索与 {} 相关的 Python 包", query),
                "relevance": 0.9,
                "source": "pypi",
                "url": url
            })
        ])
    }
    
    async fn search_js_docs(&self, query: &str) -> Result<Value> {
        let mut results = Vec::new();
        
        if let Ok(npm_results) = self.search_npm(query).await {
            results.extend(npm_results);
        }
        
        results.push(json!({
            "title": format!("MDN Web Docs: {}", query),
            "content": format!("MDN Web Docs 中关于 '{}' 的权威文档，包括API参考和使用指南", query),
            "relevance": 0.9,
            "source": "mdn",
            "url": format!("https://developer.mozilla.org/en-US/search?q={}", urlencoding::encode(query))
        }));
        
        results.push(json!({
            "title": format!("NPM 包搜索: {}", query),
            "content": format!("在 NPM 注册表中搜索与 '{}' 相关的 JavaScript/Node.js 包", query),
            "relevance": 0.85,
            "source": "npm_search",
            "url": format!("https://www.npmjs.com/search?q={}", urlencoding::encode(query))
        }));
        
        results.push(json!({
            "title": format!("Node.js 官方文档: {}", query),
            "content": format!("Node.js 官方文档中关于 '{}' 的API参考和使用说明", query),
            "relevance": 0.8,
            "source": "nodejs_docs",
            "url": format!("https://nodejs.org/api/?search={}", urlencoding::encode(query))
        }));
        
        Ok(json!({
            "results": results,
            "total_hits": results.len(),
            "language": "javascript",
            "search_quality": "comprehensive",
            "summary": format!("找到 {} 个关于 '{}' 的 JavaScript 相关文档和资源", results.len(), query)
        }))
    }
    
    async fn search_npm(&self, query: &str) -> Result<Vec<Value>> {
        let url = format!("https://registry.npmjs.org/-/v1/search?text={}&size=5", urlencoding::encode(query));
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(data) = response.json::<Value>().await {
                    let mut results = Vec::new();
                    
                    if let Some(objects) = data["objects"].as_array() {
                        for obj in objects {
                            if let Some(package) = obj["package"].as_object() {
                                if let (Some(name), Some(description)) = (
                                    package["name"].as_str(),
                                    package["description"].as_str()
                                ) {
                                    results.push(json!({
                                        "title": format!("NPM 包: {}", name),
                                        "content": description,
                                        "relevance": 0.9,
                                        "source": "npm",
                                        "url": format!("https://www.npmjs.com/package/{}", name)
                                    }));
                                }
                            }
                        }
                    }
                    
                    Ok(results)
                } else {
                    Ok(Vec::new())
                }
            },
            _ => Ok(Vec::new())
        }
    }
    
    async fn search_go_docs(&self, query: &str) -> Result<Value> {
        let results = vec![
            json!({
                "title": format!("Go 包搜索: {}", query),
                "content": format!("在 pkg.go.dev 中搜索与 {} 相关的 Go 包", query),
                "relevance": 0.9,
                "source": "go_pkg",
                "url": format!("https://pkg.go.dev/search?q={}", urlencoding::encode(query))
            }),
            json!({
                "title": format!("Go 官方文档: {}", query),
                "content": format!("Go 官方文档中关于 {} 的内容", query),
                "relevance": 0.8,
                "source": "go_docs",
                "url": format!("https://golang.org/search?q={}", urlencoding::encode(query))
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len(),
            "language": "go"
        }))
    }
    
    async fn search_java_docs(&self, query: &str) -> Result<Value> {
        let results = vec![
            json!({
                "title": format!("Maven Central: {}", query),
                "content": format!("在 Maven Central 中搜索与 {} 相关的 Java 库", query),
                "relevance": 0.9,
                "source": "maven_central",
                "url": format!("https://search.maven.org/search?q={}", urlencoding::encode(query))
            }),
            json!({
                "title": format!("Java API 文档: {}", query),
                "content": format!("Java API 文档中关于 {} 的内容", query),
                "relevance": 0.8,
                "source": "java_api",
                "url": format!("https://docs.oracle.com/en/java/javase/17/docs/api/search.html?q={}", urlencoding::encode(query))
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len(),
            "language": "java"
        }))
    }
    
    async fn search_generic_docs(&self, query: &str, language: &str) -> Result<Value> {
        let results = vec![
            json!({
                "title": format!("{} 文档搜索: {}", language, query),
                "content": format!("在 {} 相关文档中搜索 {}", language, query),
                "relevance": 0.7,
                "source": "google",
                "url": format!("https://www.google.com/search?q={}+{}+documentation", 
                    urlencoding::encode(language), urlencoding::encode(query))
            }),
            json!({
                "title": format!("GitHub 代码搜索: {}", query),
                "content": format!("在 GitHub 中搜索 {} 相关的 {} 代码", language, query),
                "relevance": 0.8,
                "source": "github",
                "url": format!("https://github.com/search?q={}+language:{}", 
                    urlencoding::encode(query), urlencoding::encode(language))
            }),
            json!({
                "title": format!("Stack Overflow: {}", query),
                "content": format!("Stack Overflow 上关于 {} 和 {} 的问答", language, query),
                "relevance": 0.6,
                "source": "stackoverflow",
                "url": format!("https://stackoverflow.com/search?q={}+{}", 
                    urlencoding::encode(language), urlencoding::encode(query))
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len(),
            "language": language
        }))
    }
}

#[async_trait]
impl MCPTool for SearchDocsTools {
    fn name(&self) -> &str {
        "search_docs"
    }
    
    fn description(&self) -> &str {
        "在需要查找能实现特定功能的包或库时，搜索相关的包信息、官方文档、API参考和使用指南，帮助找到合适的技术解决方案。"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {            Schema::Object(SchemaObject {
                required: vec!["query".to_string(), "language".to_string()],
                properties: {
                    let mut map = HashMap::new();                    map.insert("query".to_string(), Schema::String(SchemaString {
                        description: Some("要搜索的功能或技术需求".to_string()),
                        enum_values: None,
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("目标编程语言".to_string()),
                        enum_values: None,
                    }));
                    map.insert("scope".to_string(), Schema::String(SchemaString {
                        description: Some("搜索范围: api|tutorial|best_practices".to_string()),
                        enum_values: None,
                    }));
                    map.insert("max_results".to_string(), Schema::Number(SchemaNumber {
                        description: Some("最大结果数".to_string()),
                        minimum: Some(1.0),
                        maximum: Some(100.0),
                    }));
                    map
                },
                ..Default::default()
            })
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        self.validate_params(&params)?;
        
        let query = params["query"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("query 参数无效".into()))?;
            
        let language = params["language"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("language 参数无效".into()))?;
            
        let max_results = params["max_results"]
            .as_u64()
            .unwrap_or(10) as usize;
            
        let mut results = self.search_or_get_cached(query, language).await?;
        
        if let Some(results_array) = results["results"].as_array_mut() {
            if results_array.len() > max_results {
                *results_array = results_array[0..max_results].to_vec();
                results["total_hits"] = json!(max_results);
            }
        }
        
        Ok(results)
    }
}
