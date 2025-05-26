use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use crate::errors::MCPError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use anyhow::Result;



/// JSON Schema 定义
#[derive(Debug, Clone)]
pub enum Schema {
    Object(SchemaObject),
    String(SchemaString),
    Number(SchemaNumber),
    Integer(SchemaInteger),
    Boolean(SchemaBoolean),
    Array(SchemaArray),
}

impl Schema {
    pub fn validate(&self, value: &Value) -> Result<()> {
        match self {
            Schema::Object(obj) => obj.validate(value),
            Schema::String(s) => s.validate(value),
            Schema::Number(n) => n.validate(value),
            Schema::Integer(i) => i.validate(value),
            Schema::Boolean(b) => b.validate(value),
            Schema::Array(a) => a.validate(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchemaObject {
    pub required: Vec<String>,
    pub properties: HashMap<String, Schema>,
    pub description: Option<String>,
}

impl Default for SchemaObject {
    fn default() -> Self {
        Self {
            required: Vec::new(),
            properties: HashMap::new(),
            description: None,
        }
    }
}

impl SchemaObject {
    pub fn validate(&self, value: &Value) -> Result<()> {
        if !value.is_object() {
            return Err(MCPError::InvalidParameter("Expected object".to_string()).into());
        }
        
        for req in &self.required {
            if !value.get(req).is_some() {
                return Err(MCPError::InvalidParameter(format!("Required property {} missing", req)).into());
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SchemaString {
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

impl Default for SchemaString {
    fn default() -> Self {
        Self {
            description: None,
            enum_values: None,
        }
    }
}

impl SchemaString {
    pub fn validate(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(MCPError::InvalidParameter("Expected string".to_string()).into());
        }
        
        if let Some(enum_values) = &self.enum_values {
            let str_val = value.as_str().unwrap();
            if !enum_values.contains(&str_val.to_string()) {
                return Err(MCPError::InvalidParameter(format!(
                    "Value must be one of: {:?}", enum_values
                )).into());
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SchemaNumber {
    pub description: Option<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

impl Default for SchemaNumber {
    fn default() -> Self {
        Self {
            description: None,
            minimum: None,
            maximum: None,
        }
    }
}

impl SchemaNumber {
    pub fn validate(&self, value: &Value) -> Result<()> {
        if !value.is_number() {
            return Err(MCPError::InvalidParameter("Expected number".to_string()).into());
        }
        
        let num = value.as_f64().unwrap();
        
        if let Some(min) = self.minimum {
            if num < min {
                return Err(MCPError::InvalidParameter(format!("Value must be >= {}", min)).into());
            }
        }
        
        if let Some(max) = self.maximum {
            if num > max {
                return Err(MCPError::InvalidParameter(format!("Value must be <= {}", max)).into());
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SchemaInteger {
    pub description: Option<String>,
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
}

impl Default for SchemaInteger {
    fn default() -> Self {
        Self {
            description: None,
            minimum: None,
            maximum: None,
        }
    }
}

impl SchemaInteger {
    pub fn validate(&self, value: &Value) -> Result<()> {
        if !value.is_i64() {
            return Err(MCPError::InvalidParameter("Expected integer".to_string()).into());
        }
        
        let num = value.as_i64().unwrap();
        
        if let Some(min) = self.minimum {
            if num < min {
                return Err(MCPError::InvalidParameter(format!("Value must be >= {}", min)).into());
            }
        }
        
        if let Some(max) = self.maximum {
            if num > max {
                return Err(MCPError::InvalidParameter(format!("Value must be <= {}", max)).into());
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SchemaBoolean {
    pub description: Option<String>,
}

impl Default for SchemaBoolean {
    fn default() -> Self {
        Self {
            description: None,
        }
    }
}

impl SchemaBoolean {
    pub fn validate(&self, value: &Value) -> Result<()> {
        if !value.is_boolean() {
            return Err(MCPError::InvalidParameter("Expected boolean".to_string()).into());
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SchemaArray {
    pub description: Option<String>,
    pub items: Box<Schema>,
}

impl Default for SchemaArray {
    fn default() -> Self {
        Self {
            description: None,
            items: Box::new(Schema::String(SchemaString::default())),
        }
    }
}

impl SchemaArray {
    pub fn validate(&self, value: &Value) -> Result<()> {
        if !value.is_array() {
            return Err(MCPError::InvalidParameter("Expected array".to_string()).into());
        }
        
        for item in value.as_array().unwrap() {
            self.items.validate(item)?;
        }
        
        Ok(())
    }
}

/// 工具元信息
#[derive(Debug, Clone)]
pub struct ToolAnnotations {
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
}

// Tool 的基础 trait 定义
#[async_trait]
pub trait MCPTool: Send + Sync {
    /// 获取工具名称
    fn name(&self) -> &str;

    /// 获取工具描述
    fn description(&self) -> &str;

    /// 获取工具参数Schema
    fn parameters_schema(&self) -> &Schema;

    /// 执行工具
    async fn execute(&self, params: Value) -> Result<Value>;

    /// 验证输入参数
    fn validate_params(&self, params: &Value) -> Result<()> {
        let schema = self.parameters_schema();
        schema.validate(params)
            .map_err(|e| MCPError::InvalidParameter(e.to_string()).into())
    }
}

// 简化版工具接口，适用于自然语言查询
#[async_trait]
pub trait SimpleMCPTool: Send + Sync {
    /// 获取工具名称
    fn name(&self) -> &str;

    /// 获取工具描述
    fn description(&self) -> &str;

    /// 验证文本查询
    fn validate_query(&self, text_query: &str) -> Result<()> {
        if text_query.trim().is_empty() {
            return Err(MCPError::InvalidParameter("查询不能为空".into()).into());
        }
        Ok(())
    }

    /// 执行简化工具
    async fn execute_simple(&self, text_query: &str) -> Result<String> {
        self.validate_query(text_query)?;
        self.process_query(text_query).await
    }

    /// 处理查询的具体实现
    async fn process_query(&self, text_query: &str) -> Result<String>;
}

/// 文件级文档片段 - 新的核心数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDocumentFragment {
    /// 全局唯一标识符：语言/包名/版本/文件路径
    pub id: String,
    
    /// 包基本信息
    pub package_name: String,
    pub version: String,
    pub language: String,
    
    /// 文件信息
    pub file_path: String,
    pub content: String,
    pub hierarchy_path: Vec<String>,
    
    /// 文件类型
    pub file_type: FileType,
    
    /// 创建时间
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// 文件大小（字节）
    pub file_size: usize,
    /// 最后修改时间
    pub last_modified: Option<SystemTime>,
    /// 文件类型（source/test/example/doc）
    pub file_type: FileType,
    /// 编码格式
    pub encoding: String,
    /// 语言特定信息
    pub language_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Source,      // 源代码文件
    Test,        // 测试文件
    Example,     // 示例文件
    Documentation, // 文档文件
    Configuration, // 配置文件
    Other(String), // 其他类型
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            file_size: 0,
            last_modified: None,
            file_type: FileType::Source,
            encoding: "utf-8".to_string(),
            language_metadata: serde_json::Value::Null,
        }
    }
}

impl FileDocumentFragment {
    /// 生成文件级别的唯一ID
    pub fn generate_id(language: &str, package: &str, version: &str, file_path: &str) -> String {
        format!("{}/{}/{}/{}", language, package, version, file_path)
    }
    
    /// 创建新的文件文档片段
    pub fn new(
        language: String,
        package_name: String,
        version: String,
        file_path: String,
        content: String,
    ) -> Self {
        let hierarchy_path = vec![package_name.clone(), version.clone()];
        let file_type = Self::determine_file_type(&file_path);
        let id = format!("{}/{}/{}/{}", language, package_name, version, file_path);
        
        Self {
            id,
            language,
            package_name,
            version,
            file_path,
            content,
            hierarchy_path,
            file_type,
            created_at: SystemTime::now(),
        }
    }
    
    /// 根据文件路径判断文件类型
    fn determine_file_type(file_path: &str) -> FileType {
        let path = PathBuf::from(file_path);
        
        // 检查是否是测试文件
        if file_path.contains("test") || file_path.contains("spec") {
            return FileType::Test;
        }
        
        // 检查是否是示例文件
        if file_path.contains("example") || file_path.contains("demo") {
            return FileType::Example;
        }
        
        // 检查是否是文档文件
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext.to_lowercase().as_str() {
                "md" | "rst" | "txt" | "adoc" => return FileType::Documentation,
                "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" => return FileType::Configuration,
                _ => {}
            }
        }
        
        FileType::Source
    }
    
    /// 提取文件名（不含扩展名）
    pub fn get_filename_without_ext(&self) -> Option<String> {
        PathBuf::from(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }
    
    /// 获取文件扩展名
    pub fn get_file_extension(&self) -> Option<String> {
        PathBuf::from(&self.file_path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }
    
    /// 获取文件目录路径
    pub fn get_directory_path(&self) -> Option<String> {
        PathBuf::from(&self.file_path)
            .parent()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string())
    }
}

/// 文档向量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVector {
    pub data: Vec<f32>,
    pub dimension: usize,
    pub metadata: FileVectorMetadata,
}

/// 文件向量元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVectorMetadata {
    /// 原文档ID
    pub doc_id: String,
    
    /// 层次信息
    pub language: String,
    pub package_name: String,
    pub version: String,
    pub file_path: String,
    pub hierarchy_path: Vec<String>,
    
    /// 内容摘要
    pub keywords: Vec<String>,
    pub content_hash: String,
    pub content_length: usize,
    
    /// 时间戳
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl FileVectorMetadata {
    pub fn from_fragment(fragment: &FileDocumentFragment, keywords: Vec<String>) -> Self {
        Self {
            doc_id: fragment.id.clone(),
            language: fragment.language.clone(),
            package_name: fragment.package_name.clone(),
            version: fragment.version.clone(),
            file_path: fragment.file_path.clone(),
            hierarchy_path: fragment.hierarchy_path.clone(),
            keywords,
            content_hash: Self::calculate_content_hash(&fragment.content),
            content_length: fragment.content.len(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        }
    }
    
    fn calculate_content_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// 文件搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub fragment: FileDocumentFragment,
    pub score: f32,
    pub content_preview: String,
    pub matched_keywords: Vec<String>,
}

impl FileSearchResult {
    pub fn new(fragment: FileDocumentFragment, score: f32) -> Self {
        let content_preview = if fragment.content.len() > 500 {
            format!("{}...", &fragment.content[..500])
        } else {
            fragment.content.clone()
        };
        
        Self {
            fragment,
            score,
            content_preview,
            matched_keywords: Vec::new(),
        }
    }
}

/// 层次化过滤器
#[derive(Debug, Clone, Default)]
pub struct HierarchyFilter {
    pub language: Option<String>,
    pub package_name: Option<String>,
    pub version: Option<String>,
    pub file_path_prefix: Option<String>,
    pub hierarchy_level: Option<usize>,
    pub file_type: Option<FileType>,
    pub limit: Option<u64>,
    pub similarity_threshold: Option<f32>,
}

/// 文档生成器trait（文件级）
#[async_trait::async_trait]
pub trait FileDocumentGenerator: Send + Sync {
    /// 生成指定包的所有文档文件
    async fn generate_package_docs(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<FileDocumentFragment>>;
    
    /// 检查包文档是否存在
    async fn package_docs_exist(&self, package_name: &str, version: &str) -> Result<bool>;
    
    /// 获取包的最新版本
    async fn get_latest_version(&self, package_name: &str) -> Result<String>;
    
    /// 获取支持的语言
    fn supported_language(&self) -> &str;
}

/// 文档向量化器trait
#[async_trait::async_trait]
pub trait FileVectorizer: Send + Sync {
    /// 向量化单个文件
    async fn vectorize_file(&self, fragment: &FileDocumentFragment) -> Result<DocumentVector>;
    
    /// 批量向量化多个文件
    async fn vectorize_files_batch(&self, fragments: &[FileDocumentFragment]) -> Result<Vec<DocumentVector>>;
    
    /// 向量化查询文本
    async fn vectorize_query(&self, query: &str) -> Result<Vec<f32>>;
}

/// 文档存储trait
#[async_trait::async_trait]
pub trait FileDocumentStore: Send + Sync {
    /// 存储文件向量
    async fn store_file_vector(&self, vector: &DocumentVector, fragment: &FileDocumentFragment) -> Result<()>;
    
    /// 批量存储文件向量
    async fn store_file_vectors_batch(&self, vectors: &[(DocumentVector, FileDocumentFragment)]) -> Result<()>;
    
    /// 层次化搜索
    async fn search_with_hierarchy(&self, query_vector: Vec<f32>, filter: &HierarchyFilter) -> Result<Vec<FileSearchResult>>;
    
    /// 检查文档是否存在
    async fn file_exists(&self, language: &str, package: &str, version: &str, file_path: &str) -> Result<bool>;
    
    /// 删除包的所有文档
    async fn delete_package_docs(&self, language: &str, package: &str, version: &str) -> Result<()>;
}
