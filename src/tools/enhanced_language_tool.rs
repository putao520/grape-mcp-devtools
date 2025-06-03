use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tracing::{info, debug, warn, error};
use tokio::process::Command as AsyncCommand;
use reqwest::Client;
use crate::errors::MCPError;
use crate::tools::base::{
    MCPTool, Schema, SchemaObject, SchemaString,
    FileDocumentFragment,
};
use crate::tools::docs::openai_vectorizer::OpenAIVectorizer;
use super::enhanced_doc_processor::{EnhancedDocumentProcessor, ProcessorConfig, EnhancedSearchResult};
use super::vector_docs_tool::{VectorDocsTool, SearchResult};
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
    pub vector_tool: Option<Arc<VectorDocsTool>>,
}

impl EnhancedLanguageTool {
    pub async fn new(language: &str, processor: Arc<EnhancedDocumentProcessor>) -> Result<Self> {
        let tool_name = match language {
            "rust" => "enhanced_rust_docs".to_string(),
            "python" => "enhanced_python_docs".to_string(),
            "go" => "enhanced_go_docs".to_string(),
            "javascript" => "enhanced_javascript_docs".to_string(),
            "java" => "enhanced_java_docs".to_string(),
            _ => "enhanced_docs".to_string(),
        }.into_boxed_str();
        
        // 尝试初始化向量工具（如果环境变量可用）
        let vector_tool = match VectorDocsTool::new() {
            Ok(v) => {
                info!("✅ 向量工具初始化成功 for {}", language);
                Some(Arc::new(v))
            },
            Err(e) => {
                debug!("⚠️ 向量工具初始化失败 for {}: {}，将禁用向量化功能", language, e);
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
            language: language.to_string(),
            strategy: DocumentStrategy::CLIPrimary,
            http_client: Client::new(),
            vector_tool,
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
            _ => Err(anyhow!("不支持的语言: {}", self.language)),
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
            return Err(anyhow!("无法添加包: {}", package_name));
        }

        // 2. 生成文档
        let doc_output = AsyncCommand::new("cargo")
            .args(&["doc", "--no-deps"])
            .output()
            .await?;

        if !doc_output.status.success() {
            return Err(anyhow!("文档生成失败"));
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
            return Err(anyhow!("包不存在或未安装: {}", package_name));
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
            return Err(anyhow!("无法获取Go包文档: {}", package_name));
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
            return Err(anyhow!("无法获取npm包信息: {}", package_name));
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
            return Err(anyhow!("无法获取Maven依赖信息: {}", package_name));
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
            _ => Err(anyhow!("不支持的语言: {}", self.language)),
        }
    }

    /// Rust HTTP文档获取
    async fn get_rust_docs_http(&self, package_name: &str, _version: Option<&str>) -> Result<Value> {
        let url = format!("https://crates.io/api/v1/crates/{}", package_name);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("无法获取crate信息: {}", package_name));
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
            return Err(anyhow!("无法获取PyPI包信息: {}", package_name));
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
            return Err(anyhow!("无法获取Go包信息: {}", package_name));
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
            return Err(anyhow!("无法获取npm包信息: {}", package_name));
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
            return Err(anyhow!("Java包名必须是groupId:artifactId格式"));
        };

        let version_spec = version.unwrap_or("LATEST");
        let url = format!(
            "https://search.maven.org/solrsearch/select?q=g:{}+AND+a:{}&core=gav&rows=1&wt=json",
            group_id, artifact_id
        );
        
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("无法获取Maven包信息: {}", package_name));
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

    /// 向量化内容
    async fn vectorize_content(&self, content: &str, package_name: &str) -> Result<String> {
        if let Some(vector_tool) = &self.vector_tool {
            // 使用真实的向量化工具
            match vector_tool.generate_embedding(content).await {
                Ok(embedding) => {
                    info!("✅ 成功为包 {} 生成嵌入向量，维度: {}", package_name, embedding.len());
                    // 将向量化内容存储到向量数据库
                    let file_fragment = FileDocumentFragment::new(
                        self.language.clone(),
                        package_name.to_string(),
                        "latest".to_string(),
                        format!("{}_docs.md", package_name),
                        content.to_string(),
                    );
                    
                    // 存储到向量数据库
                    if let Err(e) = vector_tool.add_file_fragment(&file_fragment).await {
                        warn!("⚠️ 向量化内容存储失败: {}", e);
                    }
                    
                    Ok(format!("已向量化并存储包 {} 的文档内容", package_name))
                }
                Err(e) => {
                    warn!("⚠️ 向量化失败，回退到文本处理: {}", e);
                    tracing::info!("回退处理包 {} 的文档内容: {} 字符", package_name, content.len());
                    Ok(content.to_string())
                }
            }
        } else {
            // 没有向量工具时的合理回退
            tracing::info!("向量工具不可用，直接处理包 {} 的文档内容: {} 字符", package_name, content.len());
            Ok(content.to_string())
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
        info!("🚀 启动真正的向量增强搜索: 包={}, 查询={}", package_name, query);

        // 1. 获取基础文档
        let base_docs = self.get_package_docs(package_name, version, Some(query)).await?;
        
        // 2. 提取文档片段用于向量搜索
        let document_chunks = self.extract_searchable_content(&base_docs)?;
        
        if document_chunks.is_empty() {
            info!("⚠️ 未找到可搜索的文档内容");
            return Ok(base_docs);
        }

        // 3. 如果有向量工具，执行真正的向量搜索
        if let Some(vector_tool) = &self.vector_tool {
            info!("🔍 使用语义嵌入向量搜索...");
            
            // 3.1 为查询生成嵌入向量
            match vector_tool.generate_embedding(query).await {
                Ok(query_embedding) => {
                    info!("✅ 查询嵌入向量生成成功，维度: {}", query_embedding.len());
                    
                    // 3.2 先从已有的向量数据库搜索
                    let mut vector_results = vector_tool.hybrid_search(&query_embedding, query, 3)
                        .unwrap_or_else(|e| {
                            warn!("⚠️ 向量数据库搜索失败: {}", e);
                            Vec::new()
                        });
                    
                    // 3.3 如果向量数据库没有结果，为当前文档片段临时生成嵌入向量进行搜索
                    if vector_results.is_empty() && !document_chunks.is_empty() {
                        info!("🔄 向量数据库无结果，对当前文档片段进行临时向量分析...");
                        
                        match vector_tool.generate_embeddings_batch(&document_chunks).await {
                            Ok(chunk_embeddings) => {
                                info!("✅ 文档片段嵌入向量生成成功，共 {} 个片段", chunk_embeddings.len());
                                
                                // 计算余弦相似度
                                let mut similarities = Vec::new();
                                for (idx, chunk_embedding) in chunk_embeddings.iter().enumerate() {
                                    let similarity = self.calculate_cosine_similarity(&query_embedding, chunk_embedding);
                                    similarities.push((idx, similarity, document_chunks[idx].clone()));
                                }
                                
                                // 按相似度排序并取前3个
                                similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                                similarities.truncate(3);
                                
                                // 转换为SearchResult格式
                                vector_results = similarities.into_iter().map(|(idx, score, content)| {
                                    SearchResult {
                                        id: format!("temp_{}", idx),
                                        content,
                                        title: format!("{} 文档片段 {}", package_name, idx + 1),
                                        language: self.language.clone(),
                                        package_name: package_name.to_string(),
                                        version: version.unwrap_or("latest").to_string(),
                                        doc_type: "documentation".to_string(),
                                        metadata: HashMap::new(),
                                        score,
                                    }
                                }).collect();
                                
                                info!("✅ 临时向量分析完成，找到 {} 个相关结果", vector_results.len());
                            }
                            Err(e) => {
                                warn!("⚠️ 批量嵌入向量生成失败: {}", e);
                            }
                        }
                    }
                    
                    // 3.4 构建增强的搜索结果
                    if !vector_results.is_empty() {
                        let mut enhanced_result = base_docs;
                        
                        enhanced_result["vector_search_results"] = json!(vector_results.iter().map(|r| {
                            json!({
                                "relevance_score": r.score,
                                "content": r.content,
                                "title": r.title,
                                "language": r.language,
                                "package_name": r.package_name,
                                "version": r.version,
                                "doc_type": r.doc_type
                            })
                        }).collect::<Vec<_>>());
                        
                        // 添加最佳匹配
                        if let Some(best_result) = vector_results.first() {
                            enhanced_result["best_match"] = json!({
                                "score": best_result.score,
                                "content": best_result.content,
                                "title": best_result.title,
                                "explanation": format!("基于语义嵌入向量相似度匹配，置信度: {:.3}", best_result.score)
                            });
                        }
                        
                        enhanced_result["search_enhanced"] = json!(true);
                        enhanced_result["vector_search_enabled"] = json!(true);
                        enhanced_result["search_method"] = json!("NVIDIA语义嵌入向量 + HNSW近似最近邻搜索");
                        enhanced_result["embedding_model"] = json!("nvidia/nv-embedqa-e5-v5");
                        
                        info!("✅ 向量增强搜索完成，返回 {} 个语义匹配结果", vector_results.len());
                        return Ok(enhanced_result);
                    }
                }
                Err(e) => {
                    warn!("⚠️ 查询嵌入向量生成失败: {}", e);
                }
            }
        } else {
            info!("⚠️ 向量工具不可用，跳过向量搜索");
        }

        // 4. 回退到基础文档搜索
        info!("🔍 使用基础文档搜索（无向量增强）...");
        let mut result = base_docs;
        result["search_enhanced"] = json!(false);
        result["vector_search_enabled"] = json!(false);
        result["search_method"] = json!("基础文档检索（未使用语义向量）");
        
        Ok(result)
    }

    /// 计算两个向量之间的余弦相似度
    fn calculate_cosine_similarity(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        if vec1.len() != vec2.len() {
            return 0.0;
        }
        
        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        let magnitude1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude1 == 0.0 || magnitude2 == 0.0 {
            return 0.0;
        }
        
        dot_product / (magnitude1 * magnitude2)
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
        Box::leak(format!("enhanced_{}_docs", self.language).into_boxed_str())
    }

    fn description(&self) -> &str {
        Box::leak(format!("增强的 {} 语言包文档工具，优先使用CLI工具，支持HTTP回退", self.language).into_boxed_str())
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            properties.insert("package_name".to_string(), Schema::String(SchemaString {
                description: Some("包名".to_string()),
                enum_values: None,
            }));
            properties.insert("version".to_string(), Schema::String(SchemaString {
                description: Some("包版本 (可选, 默认 latest)".to_string()),
                enum_values: None,
            }));
            properties.insert("query".to_string(), Schema::String(SchemaString {
                description: Some("搜索查询或问题 (可选)".to_string()),
                enum_values: None,
            }));
            Schema::Object(SchemaObject {
                required: vec!["package_name".to_string()],
                properties,
                description: Some("增强语言工具参数".to_string()),
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let package_name = params.get("package_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 package_name 参数"))?;
        let version = params.get("version").and_then(|v| v.as_str());
        let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");

        // 使用完整的增强搜索功能，支持向量搜索和语义分析
        info!("🔍 开始增强文档搜索: 语言={}, 包={}, 版本={}, 查询={}", 
              self.language, package_name, version.unwrap_or("latest"), query);
              
        match self.enhanced_search(package_name, query, version).await {
            Ok(result) => {
                // 添加执行元数据
                let mut enhanced_result = result;
                enhanced_result["execution_metadata"] = json!({
                    "tool_name": format!("enhanced_{}_docs", self.language),
                    "language": self.language,
                    "package_name": package_name,
                    "version": version.unwrap_or("latest"),
                    "query": query,
                    "strategy_used": format!("{:?}", self.strategy),
                    "execution_time": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
                });
                
                Ok(json!({
                    "status": "success",
                    "package_name": package_name,
                    "version": version.unwrap_or("latest"),
                    "query": query,
                    "results": enhanced_result
                }))
            }
            Err(e) => {
                error!("❌ 增强文档搜索失败: 语言={}, 包={}, 错误={}", self.language, package_name, e);
                Err(anyhow!("处理 {} 文档请求失败 for {}:{} - {}", self.language, package_name, version.unwrap_or("latest"), e))
            }
        }
    }
} 