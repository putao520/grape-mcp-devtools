use std::sync::Arc;
use std::path::PathBuf;
use async_trait::async_trait;
use serde_json::json;
use anyhow::{anyhow, Result};
use tokio::time::{timeout, Duration};
use regex::Regex;

use crate::tools::base::{
    MCPTool, FileDocumentFragment, DocumentVector,
    FileVectorizer, HierarchyFilter, FileSearchResult,
    Schema, SchemaObject, SchemaString, SchemaBoolean,
};

use crate::storage::traits::DocumentVectorStore;

/// 升级后的文件级Go文档工具
pub struct FileGoDocsTool {
    /// 文档向量化器
    vectorizer: Arc<dyn FileVectorizer>,
    /// 文档存储
    storage: Arc<dyn DocumentVectorStore>,
    /// HTTP客户端
    client: reqwest::Client,
    /// 工作目录
    work_dir: PathBuf,
}

impl FileGoDocsTool {
    /// 创建新的Go文档工具
    pub async fn new(
        vectorizer: Arc<dyn FileVectorizer>,
        storage: Arc<dyn DocumentVectorStore>,
    ) -> Result<Self> {
        let work_dir = std::env::temp_dir().join("grape-mcp-go-docs");
        tokio::fs::create_dir_all(&work_dir).await?;
        
        Ok(Self {
            vectorizer,
            storage,
            client: reqwest::Client::new(),
            work_dir,
        })
    }
    
    /// 获取包的最新版本
    async fn get_latest_version(&self, package: &str) -> Result<String> {
        let url = format!("https://proxy.golang.org/{}/list", package);
        
        let response = timeout(Duration::from_secs(10), self.client.get(&url).send())
            .await
            .map_err(|_| anyhow!("获取版本信息超时"))?
            .map_err(|e| anyhow!("获取版本列表失败: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("包 {} 不存在", package));
        }

        let versions = response.text().await
            .map_err(|e| anyhow!("解析版本列表失败: {}", e))?;

        let latest = versions.lines()
            .filter(|line| !line.is_empty())
            .last()
            .ok_or_else(|| anyhow!("包 {} 没有可用版本", package))?;

        Ok(latest.to_string())
    }
    
    /// 从pkg.go.dev抓取文档文件
    async fn fetch_package_docs(&self, package: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        // 首先尝试从pkg.go.dev获取文档
        let doc_url = format!("https://pkg.go.dev/{}@{}", package, version);
        
        let response = timeout(Duration::from_secs(30), self.client.get(&doc_url).send())
            .await
            .map_err(|_| anyhow!("获取文档页面超时"))?
            .map_err(|e| anyhow!("获取文档页面失败: {}", e))?;
            
        if !response.status().is_success() {
            return Err(anyhow!("获取文档失败，状态码: {}", response.status()));
        }
        
        let html_content = response.text().await
            .map_err(|e| anyhow!("读取HTML内容失败: {}", e))?;
            
        // 解析HTML，提取文档结构
        self.parse_pkg_go_dev_html(package, version, &html_content).await
    }
    
    /// 解析pkg.go.dev的HTML页面，提取文件结构
    async fn parse_pkg_go_dev_html(
        &self,
        package: &str,
        version: &str,
        html: &str,
    ) -> Result<Vec<FileDocumentFragment>> {
        let mut fragments = Vec::new();
        
        // 提取包的主要文档
        let package_doc = self.extract_package_overview(package, version, html)?;
        fragments.push(package_doc);
        
        // 提取函数文档
        let function_docs = self.extract_function_docs(package, version, html)?;
        fragments.extend(function_docs);
        
        // 提取类型文档
        let type_docs = self.extract_type_docs(package, version, html)?;
        fragments.extend(type_docs);
        
        // 提取变量和常量文档
        let var_docs = self.extract_variable_docs(package, version, html)?;
        fragments.extend(var_docs);
        
        Ok(fragments)
    }
    
    /// 提取包概览文档
    fn extract_package_overview(&self, package: &str, version: &str, html: &str) -> Result<FileDocumentFragment> {
        // 使用正则表达式提取包文档
        let doc_re = Regex::new(r#"<div[^>]*class="[^"]*Documentation[^"]*"[^>]*>(.*?)</div>"#)?;
        let overview_content = doc_re.find(html)
            .map(|m| self.clean_html(m.as_str()))
            .unwrap_or_else(|| format!("Package {} documentation", package));
            
        let content = format!(
            "# Package {}\n\nVersion: {}\n\n## Overview\n\n{}",
            package, version, overview_content
        );
        
        Ok(FileDocumentFragment::new(
            "go".to_string(),
            package.to_string(),
            version.to_string(),
            "package_overview.md".to_string(),
            content,
        ))
    }
    
    /// 提取函数文档
    fn extract_function_docs(&self, package: &str, version: &str, html: &str) -> Result<Vec<FileDocumentFragment>> {
        let mut fragments = Vec::new();
        
        // 查找所有函数定义
        let func_re = Regex::new(r#"<h4[^>]*id="([^"]*)"[^>]*>func\s+([^<(]+)"#)?;
        
        for cap in func_re.captures_iter(html) {
            if let (Some(id), Some(func_name)) = (cap.get(1), cap.get(2)) {
                let func_name = func_name.as_str().trim();
                let func_id = id.as_str();
                
                // 提取函数的详细文档
                let func_doc = self.extract_function_detail(html, func_id, func_name)?;
                
                let content = format!(
                    "# Function: {}\n\nPackage: {}\nVersion: {}\n\n{}",
                    func_name, package, version, func_doc
                );
                
                let fragment = FileDocumentFragment::new(
                    "go".to_string(),
                    package.to_string(),
                    version.to_string(),
                    format!("functions/{}.md", func_name),
                    content,
                );
                
                fragments.push(fragment);
            }
        }
        
        Ok(fragments)
    }
    
    /// 提取类型文档
    fn extract_type_docs(&self, package: &str, version: &str, html: &str) -> Result<Vec<FileDocumentFragment>> {
        let mut fragments = Vec::new();
        
        // 查找所有类型定义
        let type_re = Regex::new(r#"<h4[^>]*id="([^"]*)"[^>]*>type\s+([^<\s]+)"#)?;
        
        for cap in type_re.captures_iter(html) {
            if let (Some(id), Some(type_name)) = (cap.get(1), cap.get(2)) {
                let type_name = type_name.as_str().trim();
                let type_id = id.as_str();
                
                // 提取类型的详细文档
                let type_doc = self.extract_type_detail(html, type_id, type_name)?;
                
                let content = format!(
                    "# Type: {}\n\nPackage: {}\nVersion: {}\n\n{}",
                    type_name, package, version, type_doc
                );
                
                let fragment = FileDocumentFragment::new(
                    "go".to_string(),
                    package.to_string(),
                    version.to_string(),
                    format!("types/{}.md", type_name),
                    content,
                );
                
                fragments.push(fragment);
            }
        }
        
        Ok(fragments)
    }
    
    /// 提取变量和常量文档
    fn extract_variable_docs(&self, package: &str, version: &str, html: &str) -> Result<Vec<FileDocumentFragment>> {
        let mut fragments = Vec::new();
        
        // 查找变量定义
        let var_re = Regex::new(r#"<h4[^>]*id="([^"]*)"[^>]*>var\s+([^<\s]+)"#)?;
        for cap in var_re.captures_iter(html) {
            if let (Some(id), Some(var_name)) = (cap.get(1), cap.get(2)) {
                let var_name = var_name.as_str().trim();
                let var_doc = self.extract_variable_detail(html, id.as_str(), var_name)?;
                
                let content = format!(
                    "# Variable: {}\n\nPackage: {}\nVersion: {}\n\n{}",
                    var_name, package, version, var_doc
                );
                
                let fragment = FileDocumentFragment::new(
                    "go".to_string(),
                    package.to_string(),
                    version.to_string(),
                    format!("variables/{}.md", var_name),
                    content,
                );
                
                fragments.push(fragment);
            }
        }
        
        // 查找常量定义
        let const_re = Regex::new(r#"<h4[^>]*id="([^"]*)"[^>]*>const\s+([^<\s]+)"#)?;
        for cap in const_re.captures_iter(html) {
            if let (Some(id), Some(const_name)) = (cap.get(1), cap.get(2)) {
                let const_name = const_name.as_str().trim();
                let const_doc = self.extract_variable_detail(html, id.as_str(), const_name)?;
                
                let content = format!(
                    "# Constant: {}\n\nPackage: {}\nVersion: {}\n\n{}",
                    const_name, package, version, const_doc
                );
                
                let fragment = FileDocumentFragment::new(
                    "go".to_string(),
                    package.to_string(),
                    version.to_string(),
                    format!("constants/{}.md", const_name),
                    content,
                );
                
                fragments.push(fragment);
            }
        }
        
        Ok(fragments)
    }
    
    /// 提取函数详细信息
    fn extract_function_detail(&self, html: &str, func_id: &str, func_name: &str) -> Result<String> {
        // 构建查找模式
        let pattern = format!(r#"id="{}"[^>]*>(.*?)(?=<h[234]|$)"#, regex::escape(func_id));
        let detail_re = Regex::new(&pattern)?;
        
        if let Some(cap) = detail_re.find(html) {
            let detail = self.clean_html(cap.as_str());
            Ok(format!("## Signature\n\n```go\n{}\n```\n\n## Description\n\n{}", func_name, detail))
        } else {
            Ok(format!("Function: {}", func_name))
        }
    }
    
    /// 提取类型详细信息
    fn extract_type_detail(&self, html: &str, type_id: &str, type_name: &str) -> Result<String> {
        let pattern = format!(r#"id="{}"[^>]*>(.*?)(?=<h[234]|$)"#, regex::escape(type_id));
        let detail_re = Regex::new(&pattern)?;
        
        if let Some(cap) = detail_re.find(html) {
            let detail = self.clean_html(cap.as_str());
            Ok(format!("## Definition\n\n```go\ntype {}\n```\n\n## Description\n\n{}", type_name, detail))
        } else {
            Ok(format!("Type: {}", type_name))
        }
    }
    
    /// 提取变量详细信息
    fn extract_variable_detail(&self, html: &str, var_id: &str, var_name: &str) -> Result<String> {
        let pattern = format!(r#"id="{}"[^>]*>(.*?)(?=<h[234]|$)"#, regex::escape(var_id));
        let detail_re = Regex::new(&pattern)?;
        
        if let Some(cap) = detail_re.find(html) {
            let detail = self.clean_html(cap.as_str());
            Ok(format!("## Declaration\n\n```go\n{}\n```\n\n## Description\n\n{}", var_name, detail))
        } else {
            Ok(format!("Variable: {}", var_name))
        }
    }
    
    /// 清理HTML标签，保留文本内容
    fn clean_html(&self, html: &str) -> String {
        // 简单的HTML标签清理
        let tag_re = Regex::new(r"<[^>]*>").unwrap();
        let cleaned = tag_re.replace_all(html, "");
        
        // 清理多余的空白字符
        let space_re = Regex::new(r"\s+").unwrap();
        space_re.replace_all(&cleaned, " ").trim().to_string()
    }
    
    /// 完整的文档生成和向量化流程
    async fn generate_and_store_docs(&self, package: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        // 1. 抓取文档
        let fragments = self.fetch_package_docs(package, version).await?;
        
        // 2. 批量向量化
        let vectors = self.vectorizer.vectorize_files_batch(&fragments).await?;
        
        // 3. 批量存储
        let vector_fragment_pairs: Vec<(DocumentVector, FileDocumentFragment)> = 
            vectors.into_iter().zip(fragments.iter().cloned()).collect();
        
        self.storage.store_file_vectors_batch(&vector_fragment_pairs).await?;
        
        Ok(fragments)
    }
    
    /// 搜索已存储的文档
    async fn search_stored_docs(&self, package: &str, version: Option<&str>, query: &str) -> Result<Vec<FileSearchResult>> {
        // 向量化查询
        let query_vector = self.vectorizer.vectorize_query(query).await?;
        
        // 构建层次过滤器
        let filter = HierarchyFilter {
            language: Some("go".to_string()),
            package_name: Some(package.to_string()),
            version: version.map(|v| v.to_string()),
            limit: Some(10),
            similarity_threshold: Some(0.7),
            ..Default::default()
        };
        
        // 执行搜索
        self.storage.search_with_hierarchy(query_vector, &filter).await
    }
}

#[async_trait]
impl MCPTool for FileGoDocsTool {
    fn name(&self) -> &str {
        "file_go_docs"
    }

    fn description(&self) -> &str {
        "在需要了解Go包的功能、使用方法、API文档或代码示例时，获取指定Go包的详细信息，包括安装方法、导入方式、函数说明、类型定义和实际使用示例。"
    }

    fn parameters_schema(&self) -> &Schema {
        use std::sync::OnceLock;
        use std::collections::HashMap;
        
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            
            properties.insert("package".to_string(), Schema::String(SchemaString {
                description: Some("要查询的Go包名称，例如'github.com/gin-gonic/gin'".to_string()),
                enum_values: None,
            }));
            
            properties.insert("version".to_string(), Schema::String(SchemaString {
                description: Some("要查询的包版本，不指定则查询最新版本，例如'v1.9.1'".to_string()),
                enum_values: None,
            }));
            
            properties.insert("query".to_string(), Schema::String(SchemaString {
                description: Some("要查询的具体功能或问题，例如'如何创建HTTP服务器'、'Context怎么使用'".to_string()),
                enum_values: None,
            }));
            
            properties.insert("force_regenerate".to_string(), Schema::Boolean(SchemaBoolean {
                description: Some("是否强制重新生成文档".to_string()),
            }));

            Schema::Object(SchemaObject {
                required: vec!["package".to_string()],
                properties,
                description: Some("文件级Go文档工具参数".to_string()),
            })
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let package = args.get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少必需参数: package"))?;
            
        let version = args.get("version")
            .and_then(|v| v.as_str());
            
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
            
        let force_regenerate = args.get("force_regenerate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 确定版本
        let resolved_version = match version {
            Some(v) => v.to_string(),
            None => self.get_latest_version(package).await?,
        };

        // 检查是否需要生成文档
        let need_generate = force_regenerate || 
            !self.storage.file_exists("go", package, &resolved_version, "package_overview.md").await?;

        if need_generate {
            // 生成并存储文档
            let fragments = self.generate_and_store_docs(package, &resolved_version).await?;
            
            if query.is_empty() {
                // 如果没有查询，返回包概览
                let empty_string = "".to_string();
                let overview = fragments.iter()
                    .find(|f| f.file_path == "package_overview.md")
                    .map(|f| &f.content)
                    .unwrap_or(&empty_string);
                    
                return Ok(json!({
                    "success": true,
                    "action": "generated",
                    "package": package,
                    "version": resolved_version,
                    "total_files": fragments.len(),
                    "overview": overview,
                    "files": fragments.iter().map(|f| &f.file_path).collect::<Vec<_>>()
                }));
            }
        }

        // 执行搜索
        if !query.is_empty() {
            let results = self.search_stored_docs(package, Some(&resolved_version), query).await?;
            
            return Ok(json!({
                "success": true,
                "action": "searched",
                "package": package,
                "version": resolved_version,
                "query": query,
                "results": results.iter().map(|r| json!({
                    "file": r.fragment.file_path,
                    "score": r.score,
                    "preview": r.content_preview,
                    "keywords": r.matched_keywords
                })).collect::<Vec<_>>()
            }));
        }
        
        // 如果既不需要生成也没有查询，列出已存储的文件
        let files = self.storage.list_package_files("go", package, &resolved_version).await?;
        
        Ok(json!({
            "success": true,
            "action": "listed",
            "package": package,
            "version": resolved_version,
            "files": files
        }))
    }
} 