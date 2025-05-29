use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{info, debug};

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;

/// Rust文档工具 - 专门处理Rust语言的文档生成和搜索
pub struct RustDocsTool {
    /// 缓存已生成的文档
    cache: Arc<tokio::sync::RwLock<HashMap<String, Value>>>,
}

impl RustDocsTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 生成Rust crate的文档
    async fn generate_rust_docs(&self, crate_name: &str, version: Option<&str>) -> Result<Value> {
        let cache_key = format!("{}:{}", crate_name, version.unwrap_or("latest"));
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_docs) = cache.get(&cache_key) {
                debug!("从缓存返回Rust文档: {}", cache_key);
                return Ok(cached_docs.clone());
            }
        }

        info!("生成Rust crate文档: {}", crate_name);

        // 尝试从多个源获取Rust文档
        let docs = self.fetch_rust_docs_from_sources(crate_name, version).await?;

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, docs.clone());
        }

        Ok(docs)
    }

    /// 从多个源获取Rust文档
    async fn fetch_rust_docs_from_sources(&self, crate_name: &str, version: Option<&str>) -> Result<Value> {
        // 1. 尝试从crates.io获取crate信息
        if let Ok(crates_docs) = self.fetch_from_crates_io(crate_name, version).await {
            return Ok(crates_docs);
        }

        // 2. 尝试从docs.rs获取文档
        if let Ok(docs_rs) = self.fetch_from_docs_rs(crate_name, version).await {
            return Ok(docs_rs);
        }

        // 3. 尝试从GitHub获取README
        if let Ok(github_docs) = self.fetch_from_github(crate_name).await {
            return Ok(github_docs);
        }

        // 4. 生成基础文档结构
        Ok(self.generate_basic_rust_docs(crate_name, version))
    }

    /// 从crates.io获取crate信息
    async fn fetch_from_crates_io(&self, crate_name: &str, version: Option<&str>) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = format!("https://crates.io/api/v1/crates/{}", crate_name);

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Crate不存在: {}", crate_name)).into());
        }

        let crates_data: Value = response.json().await?;
        
        // 获取版本信息
        let versions_url = format!("https://crates.io/api/v1/crates/{}/versions", crate_name);
        let versions_response = client.get(&versions_url).send().await?;
        let versions_data: Value = versions_response.json().await?;

        Ok(self.parse_crates_io_response(&crates_data, &versions_data, crate_name, version))
    }

    /// 解析crates.io响应
    fn parse_crates_io_response(&self, crates_data: &Value, versions_data: &Value, crate_name: &str, version: Option<&str>) -> Value {
        let crate_info = crates_data.get("crate").unwrap_or(&Value::Null);
        let description = crate_info.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let documentation = crate_info.get("documentation").and_then(|d| d.as_str()).unwrap_or("");
        let homepage = crate_info.get("homepage").and_then(|h| h.as_str()).unwrap_or("");
        let repository = crate_info.get("repository").and_then(|r| r.as_str()).unwrap_or("");
        let max_version = crate_info.get("max_version").and_then(|v| v.as_str()).unwrap_or("unknown");

        let versions = versions_data.get("versions")
            .and_then(|v| v.as_array())
            .map(|versions| {
                versions.iter()
                    .filter_map(|v| v.get("num").and_then(|n| n.as_str()))
                    .take(10)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        json!({
            "crate_name": crate_name,
            "version": version.unwrap_or(max_version),
            "language": "rust",
            "source": "crates.io",
            "description": description,
            "homepage": homepage,
            "repository": repository,
            "documentation_url": documentation,
            "available_versions": versions,
            "documentation": {
                "type": "crate_info",
                "content": description,
                "sections": self.extract_sections_from_description(description)
            },
            "api_reference": self.generate_rust_api_reference(crate_name, crate_info),
            "examples": self.extract_examples_from_description(description),
            "installation": {
                "cargo": format!("cargo add {}", crate_name),
                "cargo_toml": format!("{} = \"{}\"", crate_name, max_version)
            },
            "links": {
                "crates_io": format!("https://crates.io/crates/{}", crate_name),
                "docs_rs": format!("https://docs.rs/{}", crate_name),
                "repository": repository
            }
        })
    }

    /// 从docs.rs获取文档
    async fn fetch_from_docs_rs(&self, crate_name: &str, version: Option<&str>) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = if let Some(v) = version {
            format!("https://docs.rs/{}/{}/", crate_name, v)
        } else {
            format!("https://docs.rs/{}/", crate_name)
        };

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("docs.rs文档不存在: {}", crate_name)).into());
        }

        let html_content = response.text().await?;
        Ok(self.parse_docs_rs_html(&html_content, crate_name, version))
    }

    /// 解析docs.rs HTML内容
    fn parse_docs_rs_html(&self, html_content: &str, crate_name: &str, version: Option<&str>) -> Value {
        // 简化的HTML解析，提取主要信息
        let title = if html_content.contains(&format!("{} ", crate_name)) {
            format!("{} - Rust Documentation", crate_name)
        } else {
            format!("{} Documentation", crate_name)
        };

        // 提取简单的描述信息
        let description = if let Some(start) = html_content.find("<meta name=\"description\" content=\"") {
            let start = start + 34;
            if let Some(end) = html_content[start..].find("\"") {
                html_content[start..start + end].to_string()
            } else {
                format!("Rust crate: {}", crate_name)
            }
        } else {
            format!("Rust crate: {}", crate_name)
        };

        json!({
            "crate_name": crate_name,
            "version": version.unwrap_or("latest"),
            "language": "rust",
            "source": "docs.rs",
            "title": title,
            "description": description,
            "documentation": {
                "type": "api_docs",
                "url": format!("https://docs.rs/{}/", crate_name),
                "content": description
            },
            "installation": {
                "cargo": format!("cargo add {}", crate_name),
                "cargo_toml": format!("{} = \"{}\"", crate_name, version.unwrap_or("*"))
            },
            "links": {
                "crates_io": format!("https://crates.io/crates/{}", crate_name),
                "docs_rs": format!("https://docs.rs/{}", crate_name)
            }
        })
    }

    /// 从GitHub获取README
    async fn fetch_from_github(&self, crate_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        // 尝试常见的GitHub仓库命名模式
        let possible_repos = vec![
            format!("https://api.github.com/repos/{}/{}", crate_name, crate_name),
            format!("https://api.github.com/search/repositories?q={}&language:rust", crate_name),
        ];

        for repo_url in possible_repos {
            if let Ok(response) = client.get(&repo_url).send().await {
                if response.status().is_success() {
                    let repo_data: Value = response.json().await?;
                    if repo_url.contains("search") {
                        if let Some(items) = repo_data.get("items").and_then(|i| i.as_array()) {
                            if let Some(first_repo) = items.first() {
                                return Ok(self.parse_github_repo(first_repo, crate_name));
                            }
                        }
                    } else {
                        return Ok(self.parse_github_repo(&repo_data, crate_name));
                    }
                }
            }
        }

        Err(MCPError::NotFound(format!("GitHub仓库不存在: {}", crate_name)).into())
    }

    /// 解析GitHub仓库信息
    fn parse_github_repo(&self, repo_data: &Value, crate_name: &str) -> Value {
        let description = repo_data.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let html_url = repo_data.get("html_url").and_then(|u| u.as_str()).unwrap_or("");
        let language = repo_data.get("language").and_then(|l| l.as_str()).unwrap_or("Rust");
        let stars = repo_data.get("stargazers_count").and_then(|s| s.as_u64()).unwrap_or(0);

        json!({
            "crate_name": crate_name,
            "language": "rust",
            "source": "github",
            "description": description,
            "repository_url": html_url,
            "programming_language": language,
            "stars": stars,
            "documentation": {
                "type": "repository_readme",
                "content": description,
                "url": html_url
            },
            "installation": {
                "cargo": format!("cargo add {}", crate_name),
                "cargo_toml": format!("{} = \"*\"", crate_name)
            },
            "links": {
                "crates_io": format!("https://crates.io/crates/{}", crate_name),
                "docs_rs": format!("https://docs.rs/{}", crate_name),
                "github": html_url
            }
        })
    }

    /// 生成基础Rust文档
    fn generate_basic_rust_docs(&self, crate_name: &str, version: Option<&str>) -> Value {
        json!({
            "crate_name": crate_name,
            "version": version.unwrap_or("latest"),
            "language": "rust",
            "source": "generated",
            "description": format!("Rust crate: {}", crate_name),
            "documentation": {
                "type": "basic_template",
                "content": format!("这是 {} crate 的基础文档。", crate_name),
                "sections": [
                    {
                        "title": "简介",
                        "content": format!("{} 是一个 Rust crate。", crate_name)
                    },
                    {
                        "title": "安装",
                        "content": format!("在 Cargo.toml 中添加：\n{} = \"*\"", crate_name)
                    },
                    {
                        "title": "使用方法",
                        "content": format!("use {};", crate_name)
                    }
                ]
            },
            "installation": {
                "cargo": format!("cargo add {}", crate_name),
                "cargo_toml": format!("{} = \"*\"", crate_name)
            },
            "links": {
                "crates_io": format!("https://crates.io/crates/{}", crate_name),
                "docs_rs": format!("https://docs.rs/{}", crate_name)
            }
        })
    }

    /// 从描述中提取章节
    fn extract_sections_from_description(&self, description: &str) -> Vec<Value> {
        let mut sections = Vec::new();
        
        if description.contains("##") {
            let parts: Vec<&str> = description.split("##").collect();
            for part in parts.iter().skip(1) {
                if let Some(first_line) = part.lines().next() {
                    let title = first_line.trim();
                    let content = part.lines().skip(1).collect::<Vec<_>>().join("\n");
                    sections.push(json!({
                        "title": title,
                        "content": content.trim()
                    }));
                }
            }
        } else if description.len() > 100 {
            // 如果描述较长，按段落分割
            let paragraphs: Vec<&str> = description.split("\n\n").collect();
            for (i, paragraph) in paragraphs.iter().enumerate() {
                if !paragraph.trim().is_empty() {
                    sections.push(json!({
                        "title": format!("Section {}", i + 1),
                        "content": paragraph.trim()
                    }));
                }
            }
        }
        
        sections
    }

    /// 生成Rust API参考
    fn generate_rust_api_reference(&self, _crate_name: &str, _crate_info: &Value) -> Value {
        json!({
            "crate": _crate_name,
            "modules": [],
            "structs": [],
            "functions": [],
            "traits": [],
            "note": "API参考需要通过代码分析生成，当前显示基础信息"
        })
    }

    /// 从描述中提取示例
    fn extract_examples_from_description(&self, description: &str) -> Vec<Value> {
        let mut examples = Vec::new();
        
        // 查找代码块
        if description.contains("```rust") {
            let parts: Vec<&str> = description.split("```rust").collect();
            for part in parts.iter().skip(1) {
                if let Some(end_pos) = part.find("```") {
                    let code = &part[..end_pos];
                    examples.push(json!({
                        "title": "Rust示例",
                        "language": "rust",
                        "code": code.trim()
                    }));
                }
            }
        } else if description.contains("```") {
            let parts: Vec<&str> = description.split("```").collect();
            for (i, part) in parts.iter().enumerate() {
                if i % 2 == 1 { // 奇数索引是代码块
                    examples.push(json!({
                        "title": "代码示例",
                        "language": "rust",
                        "code": part.trim()
                    }));
                }
            }
        }
        
        examples
    }

    #[allow(dead_code)]
    fn generate_rust_doc_entry(&self, _crate_name: &str, _crate_info: &Value) -> Value {
        // Implementation of generate_rust_doc_entry method
        // This method is currently empty and should be implemented
        json!({})
    }
}

#[async_trait]
impl MCPTool for RustDocsTool {
    fn name(&self) -> &'static str {
        "rust_docs"
    }

    fn description(&self) -> &'static str {
        "在需要查找Rust crate的详细文档、API参考或使用示例时，获取来自crates.io、docs.rs和GitHub的综合文档信息。"
    }

    fn parameters_schema(&self) -> &Schema {
        use std::sync::OnceLock;
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["crate_name".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("crate_name".to_string(), Schema::String(SchemaString {
                        description: Some("要查询文档的Rust crate名称".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("特定版本号（可选）".to_string()),
                        enum_values: None,
                    }));
                    map.insert("include_examples".to_string(), Schema::String(SchemaString {
                        description: Some("是否包含代码示例".to_string()),
                        enum_values: Some(vec!["true".to_string(), "false".to_string()]),
                    }));
                    map
                },
                ..Default::default()
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let crate_name = params["crate_name"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("crate_name 参数是必需的".into()))?;

        let version = params["version"].as_str();

        match self.generate_rust_docs(crate_name, version).await {
            Ok(docs) => Ok(docs),
            Err(e) => {
                debug!("生成Rust文档失败: {}", e);
                // 返回基础文档而不是错误
                Ok(self.generate_basic_rust_docs(crate_name, version))
            }
        }
    }
}

impl Default for RustDocsTool {
    fn default() -> Self {
        Self::new()
    }
} 