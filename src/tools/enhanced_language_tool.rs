use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value, json};
use tracing::{info, debug, warn};
use tokio::process::Command as AsyncCommand;
use reqwest::Client;
use crate::errors::MCPError;
use crate::tools::base::{
    MCPTool, Schema, SchemaObject, SchemaString,
};
use crate::tools::docs::openai_vectorizer::OpenAIVectorizer;
// use crate::tools::docs::{DocumentReranker, RerankerConfig, RerankResult};

/// CLI优先、HTTP后备的语言工具策略
#[derive(Debug, Clone)]
pub enum DocumentStrategy {
    /// 优先使用CLI工具，失败时回退到HTTP
    CLIPrimary,
    /// 只使用HTTP查询
    HTTPOnly,
    /// 只使用CLI工具
    CLIOnly,
}

/// 增强的语言工具基类
pub struct EnhancedLanguageTool {
    pub language: String,
    pub strategy: DocumentStrategy,
    pub http_client: Client,
    /// 缓存的工具名称
    tool_name: Box<str>,
    /// 向量化器（可选）
    vectorizer: Option<Arc<OpenAIVectorizer>>,
    // 重排器（可选）
    // reranker: Option<DocumentReranker>,
}

impl EnhancedLanguageTool {
    pub async fn new(language: String, strategy: DocumentStrategy) -> Result<Self> {
        let tool_name = match language.as_str() {
            "rust" => "enhanced_rust_docs".to_string(),
            "python" => "enhanced_python_docs".to_string(),
            "go" => "enhanced_go_docs".to_string(),
            "javascript" => "enhanced_javascript_docs".to_string(),
            "java" => "enhanced_java_docs".to_string(),
            _ => "enhanced_docs".to_string(),
        }.into_boxed_str();
        
        // 尝试初始化向量化器（如果环境变量可用）
        let vectorizer = match OpenAIVectorizer::from_env() {
            Ok(v) => {
                info!("✅ 向量化器初始化成功 for {}", language);
                Some(Arc::new(v))
            },
            Err(e) => {
                debug!("⚠️ 向量化器初始化失败 for {}: {}，将禁用向量化功能", language, e);
                None
            }
        };
        
        // 尝试初始化重排器（如果环境变量可用）
        // let reranker = match DocumentReranker::from_env() {
        //     Ok(r) => {
        //         info!("✅ 重排器初始化成功 for {}", language);
        //         Some(r)
        //     },
        //     Err(e) => {
        //         debug!("⚠️ 重排器初始化失败 for {}: {}", language, e);
        //         None
        //     }
        // };
        
        Ok(Self {
            language,
            strategy,
            http_client: Client::new(),
            tool_name,
            vectorizer,
            // reranker,
        })
    }

    /// 创建Schema的静态方法
    fn create_schema() -> Schema {
        let mut properties = HashMap::new();
        
        properties.insert(
            "package_name".to_string(),
            Schema::String(SchemaString {
                description: Some("包或库名称".to_string()),
                enum_values: None,
            }),
        );
        
        properties.insert(
            "version".to_string(),
            Schema::String(SchemaString {
                description: Some("包版本（可选）".to_string()),
                enum_values: None,
            }),
        );
        
        properties.insert(
            "query".to_string(),
            Schema::String(SchemaString {
                description: Some("搜索查询（可选）".to_string()),
                enum_values: None,
            }),
        );

        properties.insert(
            "enable_vectorization".to_string(),
            Schema::String(SchemaString {
                description: Some("是否启用向量化搜索（可选，true/false）".to_string()),
                enum_values: Some(vec!["true".to_string(), "false".to_string()]),
            }),
        );
        
        Schema::Object(SchemaObject {
            properties,
            required: vec!["package_name".to_string()],
            description: Some("增强语言工具参数".to_string()),
        })
    }

    /// 检查CLI工具是否可用
    async fn is_cli_available(&self) -> bool {
        let cli_command = match self.language.as_str() {
            "rust" => "cargo",
            "python" => "pip",
            "go" => "go",
            "javascript" => "npm",
            "java" => "mvn",
            _ => return false,
        };

        match AsyncCommand::new(cli_command)
            .arg("--version")
            .output()
            .await
        {
            Ok(output) => {
                let available = output.status.success();
                if available {
                    debug!("✅ CLI工具可用: {}", cli_command);
                } else {
                    debug!("❌ CLI工具不可用: {}", cli_command);
                }
                available
            }
            Err(_) => {
                debug!("❌ CLI工具检测失败: {}", cli_command);
                false
            }
        }
    }

    /// 使用CLI获取包文档
    async fn get_docs_with_cli(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        match self.language.as_str() {
            "rust" => self.get_rust_docs_cli(package_name, version).await,
            "python" => self.get_python_docs_cli(package_name, version).await,
            "go" => self.get_go_docs_cli(package_name, version).await,
            "javascript" => self.get_javascript_docs_cli(package_name, version).await,
            "java" => self.get_java_docs_cli(package_name, version).await,
            _ => Err(anyhow::anyhow!("不支持的语言: {}", self.language)),
        }
    }

    /// Rust CLI文档获取
    async fn get_rust_docs_cli(&self, package_name: &str, _version: Option<&str>) -> Result<Value> {
        // 1. 尝试添加包到临时项目
        let add_output = AsyncCommand::new("cargo")
            .args(&["add", package_name])
            .output()
            .await?;

        if !add_output.status.success() {
            return Err(anyhow::anyhow!("无法添加包: {}", package_name));
        }

        // 2. 生成文档
        let doc_output = AsyncCommand::new("cargo")
            .args(&["doc", "--no-deps"])
            .output()
            .await?;

        if !doc_output.status.success() {
            return Err(anyhow::anyhow!("文档生成失败"));
        }

        Ok(json!({
            "source": "cargo_doc",
            "package": package_name,
            "language": "rust",
            "version": "latest",
            "installation": format!("cargo add {}", package_name),
            "docs_path": format!("target/doc/{}/index.html", package_name),
            "documentation": {
                "type": "local_generated",
                "generated_by": "cargo doc",
                "content": ""
            }
        }))
    }

    /// Python CLI文档获取
    async fn get_python_docs_cli(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        let package_spec = if let Some(v) = version {
            format!("{}=={}", package_name, v)
        } else {
            package_name.to_string()
        };

        // 1. 获取包信息
        let show_output = AsyncCommand::new("pip")
            .args(&["show", package_name])
            .output()
            .await?;

        if !show_output.status.success() {
            return Err(anyhow::anyhow!("包不存在或未安装: {}", package_name));
        }

        let show_info = String::from_utf8_lossy(&show_output.stdout);

        // 2. 尝试获取文档
        let help_output = AsyncCommand::new("python")
            .args(&["-c", &format!("import {}; help({})", package_name, package_name)])
            .output()
            .await;

        let documentation = if let Ok(output) = help_output {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            "文档获取失败，请查看官方文档".to_string()
        };

        Ok(json!({
            "source": "pip_show",
            "package": package_name,
            "language": "python",
            "version": version.unwrap_or("latest"),
            "installation": format!("pip install {}", package_spec),
            "package_info": show_info,
            "documentation": {
                "type": "cli_generated",
                "generated_by": "pip show + python help",
                "content": documentation
            }
        }))
    }

    /// Go CLI文档获取
    async fn get_go_docs_cli(&self, package_name: &str, _version: Option<&str>) -> Result<Value> {
        // 1. 获取包文档
        let doc_output = AsyncCommand::new("go")
            .args(&["doc", package_name])
            .output()
            .await?;

        if !doc_output.status.success() {
            return Err(anyhow::anyhow!("无法获取Go包文档: {}", package_name));
        }

        let documentation = String::from_utf8_lossy(&doc_output.stdout);

        Ok(json!({
            "source": "go_doc",
            "package": package_name,
            "language": "go",
            "version": "latest",
            "installation": format!("go get {}", package_name),
            "documentation": {
                "type": "cli_generated",
                "generated_by": "go doc",
                "content": documentation.to_string()
            }
        }))
    }

    /// JavaScript CLI文档获取
    async fn get_javascript_docs_cli(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        let package_spec = if let Some(v) = version {
            format!("{}@{}", package_name, v)
        } else {
            package_name.to_string()
        };

        // 1. 获取包信息
        let info_output = AsyncCommand::new("npm")
            .args(&["info", &package_spec, "--json"])
            .output()
            .await?;

        if !info_output.status.success() {
            return Err(anyhow::anyhow!("无法获取npm包信息: {}", package_name));
        }

        let package_info = String::from_utf8_lossy(&info_output.stdout);

        Ok(json!({
            "source": "npm_info",
            "package": package_name,
            "language": "javascript",
            "version": version.unwrap_or("latest"),
            "installation": format!("npm install {}", package_spec),
            "documentation": {
                "type": "cli_generated",
                "generated_by": "npm info",
                "content": package_info.to_string()
            }
        }))
    }

    /// Java CLI文档获取
    async fn get_java_docs_cli(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        // 解析groupId:artifactId格式
        let (group_id, artifact_id) = if package_name.contains(':') {
            let parts: Vec<&str> = package_name.split(':').collect();
            (parts[0], parts[1])
        } else {
            ("", package_name)
        };

        let version_spec = version.unwrap_or("LATEST");

        // 使用Maven获取依赖信息
        let mvn_output = AsyncCommand::new("mvn")
            .args(&[
                "help:describe",
                &format!("-DgroupId={}", group_id),
                &format!("-DartifactId={}", artifact_id),
                &format!("-Dversion={}", version_spec),
            ])
            .output()
            .await?;

        if !mvn_output.status.success() {
            return Err(anyhow::anyhow!("无法获取Maven依赖信息: {}", package_name));
        }

        let dependency_info = String::from_utf8_lossy(&mvn_output.stdout);

        Ok(json!({
            "source": "mvn_describe",
            "package": package_name,
            "language": "java",
            "version": version_spec,
            "installation": format!("Maven: {}:{}:{}", group_id, artifact_id, version_spec),
            "documentation": {
                "type": "cli_generated",
                "generated_by": "mvn help:describe",
                "content": dependency_info.to_string()
            }
        }))
    }

    /// 使用HTTP API获取包文档（回退方案）
    async fn get_docs_with_http(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        match self.language.as_str() {
            "rust" => self.get_rust_docs_http(package_name, version).await,
            "python" => self.get_python_docs_http(package_name, version).await,
            "go" => self.get_go_docs_http(package_name, version).await,
            "javascript" => self.get_javascript_docs_http(package_name, version).await,
            "java" => self.get_java_docs_http(package_name, version).await,
            _ => Err(anyhow::anyhow!("不支持的语言: {}", self.language)),
        }
    }

    /// Rust HTTP文档获取
    async fn get_rust_docs_http(&self, package_name: &str, _version: Option<&str>) -> Result<Value> {
        let url = format!("https://crates.io/api/v1/crates/{}", package_name);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("无法获取crate信息: {}", package_name));
        }

        let crate_info: Value = response.json().await?;
        
        Ok(json!({
            "source": "crates.io",
            "package": package_name,
            "language": "rust",
            "crate_info": crate_info,
            "docs_url": format!("https://docs.rs/{}", package_name),
            "documentation": {
                "type": "http_api",
                "generated_by": "crates.io API",
                "content": crate_info.to_string()
            }
        }))
    }

    /// Python HTTP文档获取
    async fn get_python_docs_http(&self, package_name: &str, _version: Option<&str>) -> Result<Value> {
        let url = format!("https://pypi.org/pypi/{}/json", package_name);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("无法获取PyPI包信息: {}", package_name));
        }

        let package_info: Value = response.json().await?;
        
        Ok(json!({
            "source": "pypi.org",
            "package": package_name,
            "language": "python",
            "package_info": package_info,
            "documentation": {
                "type": "http_api",
                "generated_by": "PyPI API",
                "content": package_info.to_string()
            }
        }))
    }

    /// Go HTTP文档获取
    async fn get_go_docs_http(&self, package_name: &str, _version: Option<&str>) -> Result<Value> {
        let url = format!("https://pkg.go.dev/{}", package_name);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("无法获取Go包信息: {}", package_name));
        }

        let content = response.text().await?;
        
        Ok(json!({
            "source": "pkg.go.dev",
            "package": package_name,
            "language": "go",
            "docs_url": url,
            "documentation": {
                "type": "http_scraping",
                "generated_by": "pkg.go.dev",
                "content": content
            }
        }))
    }

    /// JavaScript HTTP文档获取
    async fn get_javascript_docs_http(&self, package_name: &str, _version: Option<&str>) -> Result<Value> {
        let url = format!("https://registry.npmjs.org/{}", package_name);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("无法获取npm包信息: {}", package_name));
        }

        let package_info: Value = response.json().await?;
        
        Ok(json!({
            "source": "npmjs.org",
            "package": package_name,
            "language": "javascript",
            "package_info": package_info,
            "documentation": {
                "type": "http_api",
                "generated_by": "npm registry API",
                "content": package_info.to_string()
            }
        }))
    }

    /// Java HTTP文档获取
    async fn get_java_docs_http(&self, package_name: &str, version: Option<&str>) -> Result<Value> {
        // 解析groupId:artifactId格式
        let (group_id, artifact_id) = if package_name.contains(':') {
            let parts: Vec<&str> = package_name.split(':').collect();
            (parts[0], parts[1])
        } else {
            return Err(anyhow::anyhow!("Java包名必须是groupId:artifactId格式"));
        };

        let version_spec = version.unwrap_or("LATEST");
        let url = format!(
            "https://search.maven.org/solrsearch/select?q=g:{}+AND+a:{}&core=gav&rows=1&wt=json",
            group_id, artifact_id
        );
        
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("无法获取Maven包信息: {}", package_name));
        }

        let search_result: Value = response.json().await?;
        
        Ok(json!({
            "source": "search.maven.org",
            "package": package_name,
            "language": "java",
            "version": version_spec,
            "search_result": search_result,
            "documentation": {
                "type": "http_api",
                "generated_by": "Maven Central API",
                "content": search_result.to_string()
            }
        }))
    }

    /// 向量化文档内容
    async fn vectorize_content(&self, content: &str, package_name: &str) -> Result<Option<Value>> {
        if let Some(vectorizer) = &self.vectorizer {
            match vectorizer.vectorize(content).await {
                Ok(vector) => {
                    info!("✅ 文档向量化成功: {} (维度: {})", package_name, vector.len());
                    Ok(Some(json!({
                        "vectorized": true,
                        "vector_dimension": vector.len(),
                        "similarity_search_enabled": true
                    })))
                },
                Err(e) => {
                    warn!("⚠️ 文档向量化失败: {}: {}", package_name, e);
                    Ok(Some(json!({
                        "vectorized": false,
                        "error": e.to_string()
                    })))
                }
            }
        } else {
            Ok(None)
        }
    }

    /// 主要的包文档获取方法
    pub async fn get_package_docs(
        &self,
        package_name: &str,
        version: Option<&str>,
        query: Option<&str>,
    ) -> Result<Value> {
        let mut result = match self.strategy {
            DocumentStrategy::CLIPrimary => {
                // CLI优先策略
                if self.is_cli_available().await {
                    match self.get_docs_with_cli(package_name, version).await {
                        Ok(docs) => {
                            info!("✅ 使用CLI成功获取 {} 包文档: {}", self.language, package_name);
                            docs
                        }
                        Err(e) => {
                            warn!("⚠️ CLI获取失败，回退到HTTP: {}", e);
                            self.get_docs_with_http(package_name, version).await?
                        }
                    }
                } else {
                    warn!("⚠️ CLI不可用，使用HTTP");
                    self.get_docs_with_http(package_name, version).await?
                }
            }
            DocumentStrategy::HTTPOnly => {
                self.get_docs_with_http(package_name, version).await?
            }
            DocumentStrategy::CLIOnly => {
                self.get_docs_with_cli(package_name, version).await?
            }
        };

        // 添加查询信息
        if let Some(q) = query {
            result["query"] = json!(q);
        }

        Ok(result)
    }

    /// 增强的文档检索方法，集成向量化搜索和重排功能
    pub async fn enhanced_search(
        &self,
        package_name: &str,
        query: &str,
        version: Option<&str>,
    ) -> Result<Value> {
        info!("🚀 启动增强搜索: 包={}, 查询={}", package_name, query);

        // 1. 获取基础文档
        let base_docs = self.get_package_docs(package_name, version, Some(query)).await?;
        
        // 2. 提取文档片段用于重排
        let document_chunks = self.extract_searchable_content(&base_docs)?;
        
        if document_chunks.is_empty() {
            info!("⚠️ 未找到可搜索的文档内容");
            return Ok(base_docs);
        }

        // 3. 如果有重排器，则进行重排
        // if let Some(reranker) = &self.reranker {
        //     info!("🔄 使用重排器优化搜索结果...");
        //     
        //     match reranker.rerank_documents(query, document_chunks.clone(), Some(3)).await {
        //         Ok(rerank_results) => {
        //             info!("✅ 重排完成，返回 {} 个优化结果", rerank_results.len());
        //             
        //             // 构建重排后的结果
        //             let mut enhanced_result = base_docs;
        //             enhanced_result["reranked_results"] = json!(rerank_results.iter().map(|r| {
        //                 json!({
        //                     "relevance_score": r.relevance_score,
        //                     "content": r.document.as_ref().map(|d| &d.text).unwrap_or(&document_chunks[r.index]),
        //                     "original_index": r.index
        //                 })
        //             }).collect::<Vec<_>>());
        //             
        //             // 添加最佳匹配
        //             if let Some(best_result) = rerank_results.first() {
        //                 enhanced_result["best_match"] = json!({
        //                     "score": best_result.relevance_score,
        //                     "content": best_result.document.as_ref().map(|d| &d.text).unwrap_or(&document_chunks[best_result.index])
        //                 });
        //             }
        //             
        //             enhanced_result["search_enhanced"] = json!(true);
        //             enhanced_result["rerank_method"] = json!("nv-rerankqa-mistral-4b-v3");
        //             
        //             return Ok(enhanced_result);
        //         }
        //         Err(e) => {
        //             warn!("⚠️ 重排失败，返回基础结果: {}", e);
        //         }
        //     }
        // }

        // 4. 如果有向量化器但没有重排器，使用向量化搜索
        if let Some(_vectorizer) = &self.vectorizer {
            info!("🔍 使用向量化搜索...");
            // 这里可以添加向量化搜索逻辑
        }

        // 5. 返回基础文档（标记为未增强）
        let mut result = base_docs;
        result["search_enhanced"] = json!(false);
        Ok(result)
    }

    /// 从文档中提取可搜索的内容片段
    fn extract_searchable_content(&self, docs: &Value) -> Result<Vec<String>> {
        let mut chunks = Vec::new();
        
        // 提取不同类型的文档内容
        if let Some(content) = docs.get("documentation").and_then(|d| d.get("content")) {
            if let Some(content_str) = content.as_str() {
                if !content_str.is_empty() {
                    chunks.push(content_str.to_string());
                }
            }
        }
        
        // 提取包信息作为另一个片段
        if let Some(description) = docs.get("description") {
            if let Some(desc_str) = description.as_str() {
                if !desc_str.is_empty() {
                    chunks.push(desc_str.to_string());
                }
            }
        }
        
        // 提取安装说明
        if let Some(installation) = docs.get("installation") {
            if let Some(install_str) = installation.as_str() {
                chunks.push(format!("Installation: {}", install_str));
            }
        }
        
        // 如果内容太长，进行分块
        let mut final_chunks = Vec::new();
        for chunk in chunks {
            if chunk.len() > 1000 {
                // 按段落分块
                let paragraphs: Vec<&str> = chunk.split("\n\n").collect();
                for paragraph in paragraphs {
                    if !paragraph.trim().is_empty() {
                        final_chunks.push(paragraph.trim().to_string());
                    }
                }
            } else {
                final_chunks.push(chunk);
            }
        }
        
        debug!("📄 提取到 {} 个文档片段", final_chunks.len());
        Ok(final_chunks)
    }
}

#[async_trait]
impl MCPTool for EnhancedLanguageTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        "增强的语言包文档工具，优先使用CLI工具，支持HTTP回退"
    }

    fn parameters_schema(&self) -> &Schema {
        // 为了避免生命周期问题，这里使用泄漏内存的方式创建静态引用
        // 在实际应用中，工具的Schema是不变的，所以这是可以接受的
        Box::leak(Box::new(Self::create_schema()))
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let package_name = params.get("package_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::InvalidParameter("缺少package_name参数".to_string()))?;

        let version = params.get("version").and_then(|v| v.as_str());
        let query = params.get("query").and_then(|v| v.as_str());
        let enable_vectorization = params.get("enable_vectorization")
            .and_then(|v| v.as_str())
            .unwrap_or("false") == "true";

        // 如果有查询参数且有重排器，使用增强搜索
        let mut result = if let Some(query_str) = query {
            // if self.reranker.is_some() {
            //     info!("🚀 使用增强搜索模式");
            //     self.enhanced_search(package_name, query_str, version).await?
            // } else {
                info!("📖 使用标准搜索模式");
                self.get_package_docs(package_name, version, Some(query_str)).await?
            // }
        } else {
            self.get_package_docs(package_name, version, None).await?
        };

        // 如果用户明确要求向量化，尝试向量化任何可用的文档内容
        if enable_vectorization {
            if let Some(_vectorizer) = &self.vectorizer {
                // 尝试从多个可能的位置获取文档内容，确保内容非空
                let content_to_vectorize = {
                    // 首先尝试documentation.content
                    if let Some(content) = result.get("documentation")
                        .and_then(|doc| doc.get("content"))
                        .and_then(|c| c.as_str()) {
                        if !content.trim().is_empty() {
                            Some(content.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }.or_else(|| {
                    // 然后尝试package_info
                    if let Some(info) = result.get("package_info")
                        .and_then(|info| info.as_str()) {
                        if !info.trim().is_empty() {
                            Some(info.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }).or_else(|| {
                    // 最后尝试其他字段，但避免过长的内容
                    let result_string = result.to_string();
                    if !result_string.trim().is_empty() && result_string.len() < 10000 {
                        Some(result_string)
                    } else {
                        None
                    }
                });

                if let Some(content) = content_to_vectorize {
                    // 确保内容不为空且有意义
                    let trimmed_content = content.trim();
                    if !trimmed_content.is_empty() && trimmed_content.len() > 10 {
                        match self.vectorize_content(trimmed_content, package_name).await {
                            Ok(Some(vector_info)) => {
                                result["vectorization"] = vector_info;
                            },
                            Ok(None) => {
                                result["vectorization"] = json!({
                                    "vectorized": false,
                                    "error": "向量化器不可用"
                                });
                            },
                            Err(e) => {
                                result["vectorization"] = json!({
                                    "vectorized": false,
                                    "error": e.to_string()
                                });
                            }
                        }
                    } else {
                        result["vectorization"] = json!({
                            "vectorized": false,
                            "error": "内容为空或过短，无法向量化"
                        });
                    }
                } else {
                    result["vectorization"] = json!({
                        "vectorized": false,
                        "error": "没有找到可向量化的内容"
                    });
                }
            } else {
                result["vectorization"] = json!({
                    "vectorized": false,
                    "error": "向量化器未初始化（检查EMBEDDING_API_KEY环境变量）"
                });
            }
        }

        // 添加重排器状态信息
        // result["reranker_available"] = json!(self.reranker.is_some());
        // if let Some(reranker) = &self.reranker {
        //     result["reranker_config"] = json!(reranker.get_config_info());
        // }

        Ok(json!({
            "status": "success",
            "language": self.language,
            "strategy": format!("{:?}", self.strategy),
            "package": package_name,
            "query": query,
            "enable_vectorization": enable_vectorization,
            "data": result
        }))
    }
} 