use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{info, warn, debug};

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;

/// JavaScript/TypeScript文档工具 - 专门处理JavaScript和TypeScript的文档生成和搜索
pub struct JavaScriptDocsTool {
    /// 缓存已生成的文档
    cache: Arc<tokio::sync::RwLock<HashMap<String, Value>>>,
}

impl JavaScriptDocsTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 生成JavaScript/TypeScript包的文档
    async fn generate_js_docs(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        let cache_key = format!("{}:{}", package_name, version.unwrap_or("latest"));
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_docs) = cache.get(&cache_key) {
                debug!("从缓存返回JavaScript文档: {}", cache_key);
                return Ok(cached_docs.clone());
            }
        }

        info!("生成JavaScript包文档: {}", package_name);

        // 尝试从多个源获取JavaScript文档
        let docs = self.fetch_js_docs_from_sources(package_name, version).await?;

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, docs.clone());
        }

        Ok(docs)
    }

    /// 从多个源获取JavaScript文档
    async fn fetch_js_docs_from_sources(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        // 1. 尝试从NPM获取包信息
        if let Ok(npm_docs) = self.fetch_from_npm(package_name, version).await {
            return Ok(npm_docs);
        }

        // 2. 尝试从GitHub获取README
        if let Ok(github_docs) = self.fetch_from_github(package_name).await {
            return Ok(github_docs);
        }

        // 3. 尝试从TypeScript官方文档获取
        if let Ok(ts_docs) = self.fetch_from_typescript_docs(package_name).await {
            return Ok(ts_docs);
        }

        // 4. 生成基础文档结构
        Ok(self.generate_basic_js_docs(package_name, version))
    }

    /// 从NPM获取包信息
    async fn fetch_from_npm(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = if let Some(v) = version {
            format!("https://registry.npmjs.org/{}/{}", package_name, v)
        } else {
            format!("https://registry.npmjs.org/{}", package_name)
        };

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("NPM包不存在: {}", package_name)).into());
        }

        let npm_data: Value = response.json().await?;
        Ok(self.parse_npm_response(&npm_data, package_name))
    }

    /// 解析NPM响应
    fn parse_npm_response(&self, npm_data: &Value, package_name: &str) -> Value {
        let description = npm_data.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let version = npm_data.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        let author = npm_data.get("author")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");
        let homepage = npm_data.get("homepage").and_then(|h| h.as_str()).unwrap_or("");
        let repository = npm_data.get("repository")
            .and_then(|r| r.get("url"))
            .and_then(|u| u.as_str())
            .unwrap_or("");

        // 检测是否为TypeScript包
        let has_types = npm_data.get("types").is_some() || 
                       npm_data.get("typings").is_some() ||
                       package_name.starts_with("@types/");

        let language = if has_types { "typescript" } else { "javascript" };

        json!({
            "package_name": package_name,
            "version": version,
            "language": language,
            "source": "npm",
            "description": description,
            "author": author,
            "homepage": homepage,
            "repository": repository,
            "has_types": has_types,
            "documentation": {
                "type": "package_info",
                "content": description,
                "sections": self.extract_sections_from_description(description)
            },
            "api_reference": self.generate_js_api_reference(package_name, npm_data),
            "examples": self.extract_examples_from_description(description),
            "installation": {
                "npm": format!("npm install {}", package_name),
                "yarn": format!("yarn add {}", package_name),
                "pnpm": format!("pnpm add {}", package_name)
            }
        })
    }

    /// 从GitHub获取README
    async fn fetch_from_github(&self, package_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        // 尝试常见的GitHub仓库命名模式
        let possible_repos = vec![
            format!("https://api.github.com/repos/{}/{}", package_name, package_name),
            format!("https://api.github.com/repos/{}/js-{}", package_name, package_name),
            format!("https://api.github.com/repos/js-{}/{}", package_name, package_name),
            // 处理scoped包
            format!("https://api.github.com/repos/{}", package_name.trim_start_matches('@').replace('/', "/")),
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
        let language = repo_data.get("language").and_then(|l| l.as_str()).unwrap_or("JavaScript");

        json!({
            "package_name": package_name,
            "language": language.to_lowercase(),
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

    /// 从TypeScript官方文档获取
    async fn fetch_from_typescript_docs(&self, package_name: &str) -> Result<Value> {
        // 检查是否为@types包
        if package_name.starts_with("@types/") {
            let base_package = package_name.trim_start_matches("@types/");
            return Ok(json!({
                "package_name": package_name,
                "language": "typescript",
                "source": "typescript_types",
                "description": format!("TypeScript类型定义包，为{}提供类型支持", base_package),
                "documentation": {
                    "type": "type_definitions",
                    "content": format!("这是{}的TypeScript类型定义包", base_package),
                    "url": format!("https://www.typescriptlang.org/dt/search?search={}", base_package)
                }
            }));
        }

        Err(MCPError::NotFound(format!("TypeScript文档不存在: {}", package_name)).into())
    }

    /// 生成基础JavaScript文档结构
    fn generate_basic_js_docs(&self, package_name: &str, version: Option<&str>) -> Value {
        let language = if package_name.starts_with("@types/") { "typescript" } else { "javascript" };
        
        json!({
            "package_name": package_name,
            "version": version.unwrap_or("unknown"),
            "language": language,
            "source": "generated",
            "documentation": {
                "type": "basic_info",
                "content": format!("{}包: {}", language, package_name),
                "sections": [
                    {
                        "title": "安装",
                        "content": format!("npm install {}", package_name)
                    },
                    {
                        "title": "导入",
                        "content": if language == "typescript" {
                            format!("import * as {} from '{}';", package_name.replace('-', "_"), package_name)
                        } else {
                            format!("const {} = require('{}');", package_name.replace('-', "_"), package_name)
                        }
                    }
                ]
            },
            "installation": {
                "npm": format!("npm install {}", package_name),
                "yarn": format!("yarn add {}", package_name),
                "pnpm": format!("pnpm add {}", package_name)
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

    /// 生成JavaScript API参考
    fn generate_js_api_reference(&self, package_name: &str, npm_data: &Value) -> Value {
        let main_file = npm_data.get("main").and_then(|m| m.as_str()).unwrap_or("index.js");
        let types_file = npm_data.get("types")
            .or_else(|| npm_data.get("typings"))
            .and_then(|t| t.as_str());

        json!({
            "package": package_name,
            "main_file": main_file,
            "types_file": types_file,
            "exports": [],
            "interfaces": [],
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
        let mut current_language = "javascript";

        for line in lines {
            if line.trim().starts_with("```javascript") || line.trim().starts_with("```js") {
                in_code_block = true;
                current_language = "javascript";
                current_example.clear();
                continue;
            } else if line.trim().starts_with("```typescript") || line.trim().starts_with("```ts") {
                in_code_block = true;
                current_language = "typescript";
                current_example.clear();
                continue;
            }
            
            if line.trim() == "```" && in_code_block {
                in_code_block = false;
                if !current_example.trim().is_empty() {
                    examples.push(json!({
                        "title": "代码示例",
                        "code": current_example.trim(),
                        "language": current_language
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
impl MCPTool for JavaScriptDocsTool {
    fn name(&self) -> &'static str {
        "javascript_docs_tool"
    }

    fn description(&self) -> &'static str {
        "当LLM需要了解JavaScript/Node.js包的功能、安装配置、使用方法或API文档时，使用此工具获取指定包的详细信息，包括npm安装、导入方式、配置选项和使用示例。"
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
                        description: Some("要查询的JavaScript/TypeScript包名称".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("包版本（可选）".to_string()),
                        enum_values: None,
                    }));
                    map.insert("include_examples".to_string(), Schema::Boolean(crate::tools::base::SchemaBoolean {
                        description: Some("是否包含代码示例".to_string()),
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("指定语言类型（javascript或typescript）".to_string()),
                        enum_values: Some(vec!["javascript".to_string(), "typescript".to_string()]),
                    }));
                    map
                },
                description: Some("JavaScript文档工具参数".to_string()),
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

        info!("执行JavaScript文档工具: package={}, version={:?}", package_name, version);

        let docs = self.generate_js_docs(package_name, version).await?;

        let mut result = json!({
            "status": "success",
            "tool": "javascript_docs_tool",
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

impl Default for JavaScriptDocsTool {
    fn default() -> Self {
        Self::new()
    }
} 