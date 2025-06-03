use std::sync::Arc;
use async_trait::async_trait;
use anyhow::{anyhow, Result};
use regex::Regex;

use crate::tools::base::{
    MCPTool, FileDocumentFragment,
    Schema, SchemaObject, SchemaString, SchemaBoolean,
};

use crate::tools::doc_processor::DocumentProcessor;

/// 文件级Go文档工具
pub struct FileGoDocsTool {
    /// 文档处理器
    doc_processor: Arc<DocumentProcessor>,
    client: reqwest::Client,
}

impl FileGoDocsTool {
    /// 创建新的Go文档工具
    pub fn new() -> Result<Self> {
        let doc_processor = Arc::new(DocumentProcessor::new());
        
        Ok(Self {
            doc_processor,
            client: reqwest::Client::new(),
        })
    }

    /// 获取Go包的文档
    pub async fn get_go_package_docs(&self, package: &str, version: Option<&str>) -> Result<Vec<FileDocumentFragment>> {
        // 使用doc_processor获取Go包文档
        self.doc_processor.generate_go_docs(package, version).await
    }

    /// 提取代码示例
    fn extract_code_examples(&self, content: &str) -> Vec<String> {
        let mut examples = Vec::new();
        
        // 提取Go代码块
        let code_block_re = Regex::new(r"```(?:go|golang)\n([\s\S]*?)\n```").unwrap();
        
        for cap in code_block_re.captures_iter(content) {
            if let Some(code) = cap.get(1) {
                let code_content = code.as_str().trim();
                if !code_content.is_empty() {
                    examples.push(code_content.to_string());
                }
            }
        }
        
        // 也提取没有语言标记的代码块
        if examples.is_empty() {
            let generic_code_re = Regex::new(r"```\n([\s\S]*?)\n```").unwrap();
            for cap in generic_code_re.captures_iter(content) {
                if let Some(code) = cap.get(1) {
                    let code_content = code.as_str().trim();
                    if !code_content.is_empty() && self.looks_like_go_code(code_content) {
                        examples.push(code_content.to_string());
                    }
                }
            }
        }
        
        examples
    }
    
    /// 检查代码是否看起来像Go代码
    fn looks_like_go_code(&self, code: &str) -> bool {
        let go_keywords = ["package", "import", "func", "type", "var", "const", "if", "for", "range"];
        let go_patterns = ["fmt.", "http.", "context.", "errors.", "log."];
        
        // 检查Go关键词
        for keyword in &go_keywords {
            if code.contains(keyword) {
                return true;
            }
        }
        
        // 检查Go标准库模式
        for pattern in &go_patterns {
            if code.contains(pattern) {
                return true;
            }
        }
        
        false
    }
}

#[async_trait]
impl MCPTool for FileGoDocsTool {
    fn name(&self) -> &str {
        "get_go_docs"
    }

    fn description(&self) -> &str {
        "获取Go包文档和API参考信息"
    }

    fn parameters_schema(&self) -> &Schema {
        use std::collections::HashMap;
        use std::sync::OnceLock;
        
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            
            properties.insert("package".to_string(), Schema::String(SchemaString {
                description: Some("Go包名称，例如: fmt, net/http".to_string()),
                enum_values: None,
            }));
            
            properties.insert("version".to_string(), Schema::String(SchemaString {
                description: Some("包版本 (可选)".to_string()),
                enum_values: None,
            }));
            
            properties.insert("include_examples".to_string(), Schema::Boolean(SchemaBoolean {
                description: Some("是否包含代码示例".to_string()),
            }));
            
            Schema::Object(SchemaObject {
                properties,
                required: vec!["package".to_string()],
                description: Some("Go文档工具参数".to_string()),
            })
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let package = args.get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少必需参数: package"))?;

        let version = args.get("version")
            .and_then(|v| v.as_str());

        let include_examples = args.get("include_examples")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 使用doc_processor获取Go包文档
        let docs = self.get_go_package_docs(package, version).await?;

        let mut response = serde_json::json!({
            "package": package,
            "version": version.unwrap_or("latest"),
            "documents": docs.len(),
            "success": true
        });

        // 格式化文档内容
        let mut formatted_docs = Vec::new();
        for doc in docs {
            let mut doc_json = serde_json::json!({
                "file_path": doc.file_path,
                "content": doc.content,
                "language": doc.language,
                "package": doc.package_name,
                "version": doc.version,
            });

            // 如果需要示例，提取代码示例
            if include_examples {
                let examples = self.extract_code_examples(&doc.content);
                if !examples.is_empty() {
                    doc_json["examples"] = serde_json::json!(examples);
                }
            }

            formatted_docs.push(doc_json);
        }

        response["documents"] = serde_json::json!(formatted_docs);

        Ok(response)
    }
} 