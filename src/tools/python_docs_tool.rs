use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{info, warn, debug};

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;

/// Python文档工具 - 专门处理Python语言的文档生成和搜索
pub struct PythonDocsTool {
    /// 缓存已生成的文档
    cache: Arc<tokio::sync::RwLock<HashMap<String, Value>>>,
}

impl PythonDocsTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 生成Python包的文档
    async fn generate_python_docs(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        let cache_key = format!("{}:{}", package_name, version.unwrap_or("latest"));
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_docs) = cache.get(&cache_key) {
                debug!("从缓存返回Python文档: {}", cache_key);
                return Ok(cached_docs.clone());
            }
        }

        info!("生成Python包文档: {}", package_name);

        // 尝试从多个源获取Python文档
        let docs = self.fetch_python_docs_from_sources(package_name, version).await?;

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, docs.clone());
        }

        Ok(docs)
    }

    /// 从多个源获取Python文档
    async fn fetch_python_docs_from_sources(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        // 1. 尝试从PyPI获取包信息
        if let Ok(pypi_docs) = self.fetch_from_pypi(package_name, version).await {
            return Ok(pypi_docs);
        }

        // 2. 尝试从Read the Docs获取
        if let Ok(rtd_docs) = self.fetch_from_readthedocs(package_name).await {
            return Ok(rtd_docs);
        }

        // 3. 尝试从GitHub获取README
        if let Ok(github_docs) = self.fetch_from_github(package_name).await {
            return Ok(github_docs);
        }

        // 4. 生成基础文档结构
        Ok(self.generate_basic_python_docs(package_name, version))
    }

    /// 从PyPI获取包信息
    async fn fetch_from_pypi(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = if let Some(v) = version {
            format!("https://pypi.org/pypi/{}/{}/json", package_name, v)
        } else {
            format!("https://pypi.org/pypi/{}/json", package_name)
        };

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("PyPI包不存在: {}", package_name)).into());
        }

        let pypi_data: Value = response.json().await?;
        Ok(self.parse_pypi_response(&pypi_data, package_name))
    }

    /// 解析PyPI响应
    fn parse_pypi_response(&self, pypi_data: &Value, package_name: &str) -> Value {
        let info = pypi_data.get("info").unwrap_or(&Value::Null);
        let description = info.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let summary = info.get("summary").and_then(|s| s.as_str()).unwrap_or("");
        let version = info.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        let author = info.get("author").and_then(|a| a.as_str()).unwrap_or("unknown");
        let home_page = info.get("home_page").and_then(|h| h.as_str()).unwrap_or("");

        json!({
            "package_name": package_name,
            "version": version,
            "language": "python",
            "source": "pypi",
            "summary": summary,
            "description": description,
            "author": author,
            "home_page": home_page,
            "documentation": {
                "type": "package_info",
                "content": description,
                "sections": self.extract_sections_from_description(description)
            },
            "api_reference": self.generate_python_api_reference(package_name, info),
            "examples": self.extract_examples_from_description(description),
            "installation": {
                "pip": format!("pip install {}", package_name),
                "conda": format!("conda install {}", package_name)
            }
        })
    }

    /// 从Read the Docs获取文档
    async fn fetch_from_readthedocs(&self, package_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = format!("https://{}.readthedocs.io/en/latest/", package_name);

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Read the Docs文档不存在: {}", package_name)).into());
        }

        let html_content = response.text().await?;
        Ok(self.parse_readthedocs_html(&html_content, package_name))
    }

    /// 解析Read the Docs HTML内容
    fn parse_readthedocs_html(&self, html_content: &str, package_name: &str) -> Value {
        use scraper::{Html, Selector};

        let document = Html::parse_document(html_content);
        let title_selector = Selector::parse("title").unwrap();
        let content_selector = Selector::parse(".document .body").unwrap();

        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| format!("{} Documentation", package_name));

        let content = document
            .select(&content_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        json!({
            "package_name": package_name,
            "language": "python",
            "source": "readthedocs",
            "title": title,
            "documentation": {
                "type": "html_docs",
                "content": content,
                "url": format!("https://{}.readthedocs.io/", package_name)
            }
        })
    }

    /// 从GitHub获取README
    async fn fetch_from_github(&self, package_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        // 尝试常见的GitHub仓库命名模式
        let possible_repos = vec![
            format!("https://api.github.com/repos/{}/{}", package_name, package_name),
            format!("https://api.github.com/repos/{}/python-{}", package_name, package_name),
            format!("https://api.github.com/repos/python-{}/{}", package_name, package_name),
        ];

        for repo_url in possible_repos {
            if let Ok(response) = client.get(&repo_url).send().await {
                if response.status().is_success() {
                    let repo_data: Value = response.json().await?;
                    return Ok(self.parse_github_repo(&repo_data, package_name));
                }
            }
        }

        Err(MCPError::NotFound(format!("GitHub仓库不存在: {}", package_name)).into())
    }

    /// 解析GitHub仓库信息
    fn parse_github_repo(&self, repo_data: &Value, package_name: &str) -> Value {
        let description = repo_data.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let html_url = repo_data.get("html_url").and_then(|u| u.as_str()).unwrap_or("");
        let language = repo_data.get("language").and_then(|l| l.as_str()).unwrap_or("Python");

        json!({
            "package_name": package_name,
            "language": "python",
            "source": "github",
            "description": description,
            "repository_url": html_url,
            "primary_language": language,
            "documentation": {
                "type": "repository_info",
                "content": description,
                "url": html_url
            }
        })
    }

    /// 生成基础Python文档结构
    fn generate_basic_python_docs(&self, package_name: &str, version: Option<&str>) -> Value {
        json!({
            "package_name": package_name,
            "version": version.unwrap_or("unknown"),
            "language": "python",
            "source": "generated",
            "documentation": {
                "type": "basic_info",
                "content": format!("Python包: {}", package_name),
                "sections": [
                    {
                        "title": "安装",
                        "content": format!("pip install {}", package_name)
                    },
                    {
                        "title": "导入",
                        "content": format!("import {}", package_name)
                    }
                ]
            },
            "installation": {
                "pip": format!("pip install {}", package_name),
                "conda": format!("conda install {}", package_name)
            }
        })
    }

    /// 从描述中提取章节
    fn extract_sections_from_description(&self, description: &str) -> Vec<Value> {
        let mut sections = Vec::new();
        
        // 简单的章节提取逻辑
        let lines: Vec<&str> = description.lines().collect();
        let mut current_section = String::new();
        let mut current_title = "Overview".to_string();

        for line in lines {
            if line.starts_with('#') || line.starts_with("==") || line.starts_with("--") {
                if !current_section.is_empty() {
                    sections.push(json!({
                        "title": current_title,
                        "content": current_section.trim()
                    }));
                }
                current_title = line.trim_start_matches('#').trim().to_string();
                current_section.clear();
            } else {
                current_section.push_str(line);
                current_section.push('\n');
            }
        }

        if !current_section.is_empty() {
            sections.push(json!({
                "title": current_title,
                "content": current_section.trim()
            }));
        }

        sections
    }

    /// 生成Python API参考
    fn generate_python_api_reference(&self, package_name: &str, info: &Value) -> Value {
        json!({
            "package": package_name,
            "modules": [],
            "classes": [],
            "functions": [],
            "note": "API参考需要通过代码分析生成，当前显示基础信息"
        })
    }

    /// 从描述中提取示例
    fn extract_examples_from_description(&self, description: &str) -> Vec<Value> {
        let mut examples = Vec::new();
        let lines: Vec<&str> = description.lines().collect();
        let mut in_code_block = false;
        let mut current_example = String::new();

        for line in lines {
            if line.trim().starts_with("```python") || line.trim().starts_with(".. code-block:: python") {
                in_code_block = true;
                current_example.clear();
                continue;
            }
            
            if line.trim() == "```" && in_code_block {
                in_code_block = false;
                if !current_example.trim().is_empty() {
                    examples.push(json!({
                        "title": "代码示例",
                        "code": current_example.trim(),
                        "language": "python"
                    }));
                }
                current_example.clear();
                continue;
            }

            if in_code_block {
                current_example.push_str(line);
                current_example.push('\n');
            }
        }

        examples
    }
}

#[async_trait]
impl MCPTool for PythonDocsTool {
    fn name(&self) -> &'static str {
        "python_docs_tool"
    }

    fn description(&self) -> &'static str {
        "当LLM需要了解Python包的功能、安装方法、使用示例或API说明时，使用此工具获取指定Python包的详细信息，包括pip安装命令、导入方式、主要功能和代码示例。"
    }

    fn parameters_schema(&self) -> &Schema {
        use std::sync::OnceLock;
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["package_name".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("package_name".to_string(), Schema::String(SchemaString {
                        description: Some("要查询的Python包名称".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("要查询的包版本，不指定则查询最新版本".to_string()),
                        enum_values: None,
                    }));
                    map.insert("include_examples".to_string(), Schema::Boolean(crate::tools::base::SchemaBoolean {
                        description: Some("是否包含代码示例".to_string()),
                    }));
                    map
                },
                description: Some("Python文档工具参数".to_string()),
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let package_name = params.get("package_name")
            .and_then(|p| p.as_str())
            .ok_or_else(|| MCPError::InvalidParameter("缺少package_name参数".to_string()))?;

        let version = params.get("version").and_then(|v| v.as_str());
        let include_examples = params.get("include_examples")
            .and_then(|e| e.as_bool())
            .unwrap_or(true);

        info!("执行Python文档工具: package={}, version={:?}", package_name, version);

        let docs = self.generate_python_docs(package_name, version).await?;

        let mut result = json!({
            "status": "success",
            "tool": "python_docs_tool",
            "package_name": package_name,
            "documentation": docs
        });

        if !include_examples {
            if let Some(doc_obj) = result.get_mut("documentation") {
                if let Some(doc_map) = doc_obj.as_object_mut() {
                    doc_map.remove("examples");
                }
            }
        }

        Ok(result)
    }
}

impl Default for PythonDocsTool {
    fn default() -> Self {
        Self::new()
    }
} 