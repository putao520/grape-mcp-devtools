use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{info, debug};

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;

/// TypeScript文档工具 - 专门处理TypeScript语言的文档生成和搜索
pub struct TypeScriptDocsTool {
    /// 缓存已生成的文档
    cache: Arc<tokio::sync::RwLock<HashMap<String, Value>>>,
}

impl TypeScriptDocsTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 生成TypeScript包的文档
    async fn generate_typescript_docs(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        let cache_key = format!("{}:{}", package_name, version.unwrap_or("latest"));
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_docs) = cache.get(&cache_key) {
                debug!("从缓存返回TypeScript文档: {}", cache_key);
                return Ok(cached_docs.clone());
            }
        }

        info!("生成TypeScript包文档: {}", package_name);

        // 尝试从多个源获取TypeScript文档
        let docs = self.fetch_typescript_docs_from_sources(package_name, version).await?;

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, docs.clone());
        }

        Ok(docs)
    }

    /// 从多个源获取TypeScript文档
    async fn fetch_typescript_docs_from_sources(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        // 1. 尝试从NPM获取TypeScript包信息
        if let Ok(npm_docs) = self.fetch_from_npm_with_types(package_name, version).await {
            return Ok(npm_docs);
        }

        // 2. 尝试从DefinitelyTyped获取类型定义
        if let Ok(dt_docs) = self.fetch_from_definitely_typed(package_name).await {
            return Ok(dt_docs);
        }

        // 3. 尝试从TypeScript官方文档获取
        if let Ok(ts_docs) = self.fetch_from_typescript_handbook(package_name).await {
            return Ok(ts_docs);
        }

        // 4. 尝试从GitHub获取TypeScript项目
        if let Ok(github_docs) = self.fetch_from_github_typescript(package_name).await {
            return Ok(github_docs);
        }

        // 5. 生成基础TypeScript文档结构
        Ok(self.generate_basic_typescript_docs(package_name, version))
    }

    /// 从NPM获取TypeScript包信息（包含类型信息）
    async fn fetch_from_npm_with_types(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
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
        Ok(self.parse_npm_typescript_response(&npm_data, package_name))
    }

    /// 解析NPM TypeScript响应
    fn parse_npm_typescript_response(&self, npm_data: &Value, package_name: &str) -> Value {
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

        // TypeScript特有信息
        let types = npm_data.get("types").and_then(|t| t.as_str());
        let typings = npm_data.get("typings").and_then(|t| t.as_str());
        let typescript_version = npm_data.get("devDependencies")
            .and_then(|deps| deps.get("typescript"))
            .and_then(|ts| ts.as_str());

        // 检测TypeScript配置
        let has_tsconfig = npm_data.get("files")
            .and_then(|files| files.as_array())
            .map(|files| files.iter().any(|f| f.as_str().unwrap_or("").contains("tsconfig")))
            .unwrap_or(false);

        json!({
            "package_name": package_name,
            "version": version,
            "language": "typescript",
            "source": "npm_typescript",
            "description": description,
            "author": author,
            "homepage": homepage,
            "repository": repository,
            "typescript_info": {
                "types_entry": types.or(typings),
                "typescript_version": typescript_version,
                "has_tsconfig": has_tsconfig,
                "is_typescript_native": types.is_some() || typings.is_some()
            },
            "documentation": {
                "type": "typescript_package",
                "content": description,
                "sections": self.extract_typescript_sections_from_description(description)
            },
            "api_reference": self.generate_typescript_api_reference(package_name, npm_data),
            "type_definitions": self.extract_type_definitions(npm_data),
            "examples": self.extract_typescript_examples_from_description(description),
            "installation": {
                "npm": format!("npm install {}", package_name),
                "yarn": format!("yarn add {}", package_name),
                "pnpm": format!("pnpm add {}", package_name),
                "types": if types.is_none() && typings.is_none() {
                    Some(format!("npm install --save-dev @types/{}", package_name.trim_start_matches('@')))
                } else {
                    None
                }
            }
        })
    }

    /// 从DefinitelyTyped获取类型定义
    async fn fetch_from_definitely_typed(&self, package_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        let clean_package_name = package_name.trim_start_matches('@').replace('/', "__");
        let types_package = format!("@types/{}", clean_package_name);
        
        let url = format!("https://registry.npmjs.org/{}", types_package);
        let response = client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("DefinitelyTyped类型定义不存在: {}", types_package)).into());
        }

        let types_data: Value = response.json().await?;
        Ok(self.parse_definitely_typed_response(&types_data, package_name, &types_package))
    }

    /// 解析DefinitelyTyped响应
    fn parse_definitely_typed_response(&self, types_data: &Value, original_package: &str, types_package: &str) -> Value {
        let description = types_data.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let version = types_data.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        let homepage = types_data.get("homepage").and_then(|h| h.as_str()).unwrap_or("");

        json!({
            "package_name": original_package,
            "types_package": types_package,
            "version": version,
            "language": "typescript",
            "source": "definitely_typed",
            "description": description,
            "homepage": homepage,
            "typescript_info": {
                "is_types_package": true,
                "original_package": original_package,
                "types_version": version
            },
            "documentation": {
                "type": "type_definitions",
                "content": description,
                "url": format!("https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/{}", 
                    original_package.trim_start_matches('@').replace('/', "__"))
            },
            "installation": {
                "npm": format!("npm install --save-dev {}", types_package),
                "yarn": format!("yarn add --dev {}", types_package),
                "pnpm": format!("pnpm add --save-dev {}", types_package)
            }
        })
    }

    /// 从TypeScript官方文档获取
    async fn fetch_from_typescript_handbook(&self, package_name: &str) -> Result<Value> {
        // 检查是否为TypeScript内置类型或官方包
        let builtin_types = vec![
            "typescript", "ts-node", "@typescript-eslint/parser", "@typescript-eslint/eslint-plugin",
            "tslib", "typedoc", "ts-loader", "awesome-typescript-loader"
        ];

        if builtin_types.contains(&package_name) {
            return Ok(json!({
                "package_name": package_name,
                "language": "typescript",
                "source": "typescript_official",
                "description": format!("TypeScript官方包: {}", package_name),
                "typescript_info": {
                    "is_official": true,
                    "category": self.categorize_typescript_package(package_name)
                },
                "documentation": {
                    "type": "official_docs",
                    "content": format!("这是TypeScript官方维护的包: {}", package_name),
                    "url": "https://www.typescriptlang.org/docs/"
                }
            }));
        }

        Err(MCPError::NotFound(format!("TypeScript官方文档中未找到: {}", package_name)).into())
    }

    /// 从GitHub获取TypeScript项目
    async fn fetch_from_github_typescript(&self, package_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        // 尝试常见的GitHub仓库命名模式
        let possible_repos = vec![
            format!("https://api.github.com/repos/{}/{}", package_name, package_name),
            format!("https://api.github.com/repos/{}/typescript-{}", package_name, package_name),
            format!("https://api.github.com/repos/typescript-{}/{}", package_name, package_name),
            // 处理scoped包
            format!("https://api.github.com/repos/{}", package_name.trim_start_matches('@').replace('/', "/")),
        ];

        for repo_url in possible_repos {
            if let Ok(response) = client.get(&repo_url).send().await {
                if response.status().is_success() {
                    let repo_data: Value = response.json().await?;
                    if self.is_typescript_repo(&repo_data) {
                        return Ok(self.parse_github_typescript_repo(&repo_data, package_name));
                    }
                }
            }
        }

        Err(MCPError::NotFound(format!("GitHub TypeScript仓库不存在: {}", package_name)).into())
    }

    /// 检查是否为TypeScript仓库
    fn is_typescript_repo(&self, repo_data: &Value) -> bool {
        let language = repo_data.get("language").and_then(|l| l.as_str()).unwrap_or("");
        let description = repo_data.get("description").and_then(|d| d.as_str()).unwrap_or("");
        
        language == "TypeScript" || 
        description.to_lowercase().contains("typescript") ||
        description.to_lowercase().contains("ts")
    }

    /// 解析GitHub TypeScript仓库信息
    fn parse_github_typescript_repo(&self, repo_data: &Value, package_name: &str) -> Value {
        let description = repo_data.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let html_url = repo_data.get("html_url").and_then(|u| u.as_str()).unwrap_or("");
        let language = repo_data.get("language").and_then(|l| l.as_str()).unwrap_or("TypeScript");
        let stars = repo_data.get("stargazers_count").and_then(|s| s.as_u64()).unwrap_or(0);
        let forks = repo_data.get("forks_count").and_then(|f| f.as_u64()).unwrap_or(0);

        json!({
            "package_name": package_name,
            "language": "typescript",
            "source": "github_typescript",
            "description": description,
            "repository_url": html_url,
            "primary_language": language,
            "stats": {
                "stars": stars,
                "forks": forks
            },
            "typescript_info": {
                "is_typescript_repo": true,
                "github_language": language
            },
            "documentation": {
                "type": "repository_info",
                "content": description,
                "url": html_url
            }
        })
    }

    /// 生成基础TypeScript文档结构
    fn generate_basic_typescript_docs(&self, package_name: &str, version: Option<&str>) -> Value {
        json!({
            "package_name": package_name,
            "version": version.unwrap_or("unknown"),
            "language": "typescript",
            "source": "generated",
            "description": format!("TypeScript包: {}", package_name),
            "typescript_info": {
                "is_generated": true,
                "needs_types": true
            },
            "documentation": {
                "type": "basic_info",
                "content": format!("基础TypeScript包信息: {}", package_name),
                "suggestions": [
                    "检查是否有对应的@types包",
                    "查看包的README文件",
                    "检查TypeScript兼容性"
                ]
            },
            "installation": {
                "npm": format!("npm install {}", package_name),
                "types_suggestion": format!("npm install --save-dev @types/{}", 
                    package_name.trim_start_matches('@').replace('/', "__"))
            }
        })
    }

    /// 从描述中提取TypeScript特有的章节
    fn extract_typescript_sections_from_description(&self, description: &str) -> Vec<Value> {
        let mut sections = Vec::new();
        
        // TypeScript特有关键词
        let typescript_keywords = vec![
            "interface", "type", "generic", "decorator", "namespace",
            "module", "enum", "class", "abstract", "implements", "extends"
        ];
        
        for keyword in typescript_keywords {
            if description.to_lowercase().contains(keyword) {
                sections.push(json!({
                    "title": format!("TypeScript {}", keyword),
                    "content": format!("包含{}相关功能", keyword),
                    "type": "typescript_feature"
                }));
            }
        }
        
        sections
    }

    /// 生成TypeScript API参考
    fn generate_typescript_api_reference(&self, package_name: &str, npm_data: &Value) -> Value {
        let mut api_ref = json!({
            "package": package_name,
            "type": "typescript_api"
        });

        // 检查是否有类型定义入口
        if let Some(types) = npm_data.get("types").and_then(|t| t.as_str()) {
            api_ref["types_entry"] = json!(types);
        }

        // 检查导出信息
        if let Some(main) = npm_data.get("main").and_then(|m| m.as_str()) {
            api_ref["main_entry"] = json!(main);
        }

        if let Some(module) = npm_data.get("module").and_then(|m| m.as_str()) {
            api_ref["module_entry"] = json!(module);
        }

        api_ref
    }

    /// 提取类型定义信息
    fn extract_type_definitions(&self, npm_data: &Value) -> Value {
        let mut type_info = json!({});

        if let Some(types) = npm_data.get("types").and_then(|t| t.as_str()) {
            type_info["types_file"] = json!(types);
            type_info["has_builtin_types"] = json!(true);
        } else {
            type_info["has_builtin_types"] = json!(false);
            type_info["suggestion"] = json!("可能需要安装对应的@types包");
        }

        // 检查TypeScript版本要求
        if let Some(peer_deps) = npm_data.get("peerDependencies") {
            if let Some(ts_version) = peer_deps.get("typescript").and_then(|v| v.as_str()) {
                type_info["typescript_version_requirement"] = json!(ts_version);
            }
        }

        type_info
    }

    /// 从描述中提取TypeScript示例
    fn extract_typescript_examples_from_description(&self, description: &str) -> Vec<Value> {
        let mut examples = Vec::new();
        
        // 查找TypeScript代码块
        if description.contains("```typescript") || description.contains("```ts") {
            examples.push(json!({
                "title": "TypeScript示例",
                "content": "在描述中找到TypeScript代码示例",
                "type": "typescript_code"
            }));
        }

        // 查找接口定义
        if description.contains("interface") {
            examples.push(json!({
                "title": "接口定义",
                "content": "包含TypeScript接口定义",
                "type": "interface_example"
            }));
        }

        examples
    }

    /// 分类TypeScript包
    fn categorize_typescript_package(&self, package_name: &str) -> String {
        match package_name {
            "typescript" => "compiler".to_string(),
            "ts-node" => "runtime".to_string(),
            name if name.starts_with("@typescript-eslint") => "linting".to_string(),
            "tslib" => "runtime_library".to_string(),
            "typedoc" => "documentation".to_string(),
            name if name.contains("loader") => "bundler_integration".to_string(),
            _ => "other".to_string(),
        }
    }
}

#[async_trait]
impl MCPTool for TypeScriptDocsTool {
    fn name(&self) -> &'static str {
        "typescript_docs"
    }

    fn description(&self) -> &'static str {
        "在需要了解TypeScript包的类型定义、使用方法、配置选项或类型安全实践时，获取指定TypeScript包的详细信息，包括类型声明、安装方法、导入方式和类型使用示例。"
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            
            properties.insert("package_name".to_string(), Schema::String(SchemaString {
                description: Some("要查询的TypeScript包名称".to_string()),
                enum_values: None,
            }));
            
            properties.insert("version".to_string(), Schema::String(SchemaString {
                description: Some("要查询的包版本，不指定则查询最新版本".to_string()),
                enum_values: None,
            }));

            Schema::Object(SchemaObject {
                required: vec!["package_name".to_string()],
                properties,
                description: Some("TypeScript文档查询参数".to_string()),
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let package_name = params.get("package_name")
            .and_then(|p| p.as_str())
            .ok_or_else(|| MCPError::InvalidParameter("缺少package_name参数".to_string()))?;

        let version = params.get("version").and_then(|v| v.as_str());

        let docs = self.generate_typescript_docs(package_name, version).await?;

        Ok(json!({
            "status": "success",
            "data": docs,
            "metadata": {
                "tool": "typescript_docs",
                "package": package_name,
                "version": version.unwrap_or("latest"),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        }))
    }
}

impl Default for TypeScriptDocsTool {
    fn default() -> Self {
        Self::new()
    }
} 