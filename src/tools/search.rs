use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use crate::errors::MCPError;
use super::base::{MCPTool, ToolAnnotations, Schema, SchemaObject, SchemaString, SchemaNumber};

pub struct SearchDocsTools {
    annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, Value>>>,
}

impl SearchDocsTools {
    pub fn new() -> Self {
        Self {            annotations: ToolAnnotations {
                category: "文档搜索".to_string(),
                tags: vec!["文档".to_string(), "搜索".to_string()],
                version: "1.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn search_or_get_cached(&self, query: &str, language: &str) -> Result<Value> {
        let cache_key = format!("{}:{}", language, query);
        
        // 尝试从缓存获取
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }
        
        // 执行实际搜索
        let results = self.perform_search(query, language).await?;
        
        // 存入缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, results.clone());
        }
        
        Ok(results)
    }
    
    async fn perform_search(&self, query: &str, language: &str) -> Result<Value> {
        // 实现实际的文档搜索逻辑
        // 根据语言类型搜索不同的文档源
        let results = match language.to_lowercase().as_str() {
            "rust" => self.search_rust_docs(query).await?,
            "python" => self.search_python_docs(query).await?,
            "javascript" | "js" => self.search_js_docs(query).await?,
            "go" => self.search_go_docs(query).await?,
            "java" => self.search_java_docs(query).await?,
            _ => self.search_generic_docs(query, language).await?,
        };
        
        Ok(results)
    }
    
    async fn search_rust_docs(&self, query: &str) -> Result<Value> {
        // 搜索 Rust 官方文档
        let results = vec![
            json!({
                "title": format!("Rust std::{}", query),
                "content": format!("Rust 标准库中的 {} 模块或函数", query),
                "relevance": 0.9,
                "source": "rust_std",
                "url": format!("https://doc.rust-lang.org/std/?search={}", query)
            }),
            json!({
                "title": format!("The Rust Book: {}", query),
                "content": format!("Rust 编程语言书籍中关于 {} 的章节", query),
                "relevance": 0.8,
                "source": "rust_book",
                "url": format!("https://doc.rust-lang.org/book/?search={}", query)
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len()
        }))
    }
    
    async fn search_python_docs(&self, query: &str) -> Result<Value> {
        // 搜索 Python 官方文档
        let results = vec![
            json!({
                "title": format!("Python {}", query),
                "content": format!("Python 标准库中的 {} 模块或函数", query),
                "relevance": 0.9,
                "source": "python_docs",
                "url": format!("https://docs.python.org/3/search.html?q={}", query)
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len()
        }))
    }
    
    async fn search_js_docs(&self, query: &str) -> Result<Value> {
        // 搜索 JavaScript MDN 文档
        let results = vec![
            json!({
                "title": format!("MDN: {}", query),
                "content": format!("Mozilla Developer Network 中关于 {} 的文档", query),
                "relevance": 0.9,
                "source": "mdn",
                "url": format!("https://developer.mozilla.org/en-US/search?q={}", query)
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len()
        }))
    }
    
    async fn search_go_docs(&self, query: &str) -> Result<Value> {
        // 搜索 Go 官方文档
        let results = vec![
            json!({
                "title": format!("Go pkg: {}", query),
                "content": format!("Go 标准库中的 {} 包或函数", query),
                "relevance": 0.9,
                "source": "go_pkg",
                "url": format!("https://pkg.go.dev/search?q={}", query)
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len()
        }))
    }
    
    async fn search_java_docs(&self, query: &str) -> Result<Value> {
        // 搜索 Java 官方文档
        let results = vec![
            json!({
                "title": format!("Java API: {}", query),
                "content": format!("Java API 文档中的 {} 类或方法", query),
                "relevance": 0.9,
                "source": "java_api",
                "url": format!("https://docs.oracle.com/en/java/javase/17/docs/api/search.html?q={}", query)
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len()
        }))
    }
    
    async fn search_generic_docs(&self, query: &str, language: &str) -> Result<Value> {
        // 通用搜索逻辑
        let results = vec![
            json!({
                "title": format!("{} Documentation: {}", language, query),
                "content": format!("关于 {} 在 {} 中的用法说明", query, language),
                "relevance": 0.7,
                "source": "generic",
                "url": format!("https://www.google.com/search?q={}+{}", language, query)
            })
        ];
        
        Ok(json!({
            "results": results,
            "total_hits": results.len()
        }))
    }
}

#[async_trait]
impl MCPTool for SearchDocsTools {
    fn name(&self) -> &str {
        "search_docs"
    }
    
    fn description(&self) -> &str {
        "搜索编程语言文档以获取API参考、教程和最佳实践"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {            Schema::Object(SchemaObject {
                required: vec!["query".to_string(), "language".to_string()],
                properties: {
                    let mut map = HashMap::new();map.insert("query".to_string(), Schema::String(SchemaString {
                        description: Some("搜索查询".to_string()),
                        enum_values: None,
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("编程语言".to_string()),
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
        // 验证参数
        self.validate_params(&params)?;
        
        // 提取参数
        let query = params["query"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("query 参数无效".into()))?;
            
        let language = params["language"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("language 参数无效".into()))?;
            
        let max_results = params["max_results"]
            .as_u64()
            .unwrap_or(10) as usize;
            
        // 搜索或从缓存获取
        let mut results = self.search_or_get_cached(query, language).await?;
        
        // 处理最大结果数限制
        if let Some(results_array) = results["results"].as_array_mut() {
            if results_array.len() > max_results {
                *results_array = results_array[0..max_results].to_vec();
                results["total_hits"] = json!(max_results);
            }
        }
        
        Ok(results)
    }
}
