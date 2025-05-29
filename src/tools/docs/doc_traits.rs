use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use reqwest::Client;

/// 文档片段 - 简化的文档结构，用于向量化存储
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentFragment {
    /// 唯一标识符
    pub id: String,
    /// 标题
    pub title: String,
    /// 文档内容
    pub content: String,
    /// 文档类型
    pub doc_type: DocElementKind,
    /// 语言
    pub language: String,
    /// 包名/模块名
    pub package_name: String,
    /// 版本
    pub version: Option<String>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 文档元素类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocElementKind {
    /// 函数
    Function,
    /// 类
    Class,
    /// 模块
    Module,
    /// 接口
    Interface,
    /// 结构体
    Struct,
    /// 枚举
    Enum,
    /// 常量
    Constant,
    /// 变量
    Variable,
    /// 类型
    Type,
    /// 其他
    Other,
}

/// 文档向量
pub type DocumentVector = Vec<f32>;

/// 搜索过滤器
#[derive(Debug, Clone)]
pub struct SearchFilter {
    /// 语言过滤
    pub languages: Option<Vec<String>>,
    /// 文档类型过滤
    pub doc_types: Option<Vec<DocElementKind>>,
    /// 结果数量限制
    pub limit: Option<usize>,
    /// 相似度阈值
    pub similarity_threshold: Option<f32>,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 文档片段
    pub fragment: DocumentFragment,
    /// 相关度分数
    pub score: f32,
}

/// 文档存储 trait - 简单的 CRUD 接口
#[async_trait]
pub trait DocumentStore: Send + Sync {
    /// 存储文档片段
    async fn store(&self, fragment: &DocumentFragment) -> Result<()>;
    
    /// 获取文档片段
    async fn get(&self, id: &str) -> Result<Option<DocumentFragment>>;
    
    /// 删除文档片段
    async fn delete(&self, id: &str) -> Result<()>;
    
    /// 搜索文档
    async fn search(&self, query: &str, filter: &SearchFilter) -> Result<Vec<SearchResult>>;
}

/// 文档向量化器 trait - 使用真实的NVIDIA API
#[async_trait]
pub trait DocumentVectorizer: Send + Sync {
    /// 向量化文档内容
    async fn vectorize(&self, content: &str) -> Result<DocumentVector>;
    
    /// 计算向量相似度
    fn calculate_similarity(&self, vec1: &DocumentVector, vec2: &DocumentVector) -> f32;
}

/// NVIDIA API 嵌入响应
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// NVIDIA API 嵌入请求
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
    input_type: String,
}

/// 真实的NVIDIA API文档向量化器实现
pub struct NvidiaDocumentVectorizer {
    client: Client,
    api_key: String,
    model_name: String,
}

impl NvidiaDocumentVectorizer {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("EMBEDDING_API_KEY")
            .map_err(|_| anyhow::anyhow!("未设置 EMBEDDING_API_KEY 环境变量"))?;
        
        Ok(Self {
            client: Client::new(),
            api_key,
            model_name: "nvidia/nv-embedqa-e5-v5".to_string(),
        })
    }
    
    pub fn from_env() -> Result<Self> {
        Self::new()
    }
}

#[async_trait]
impl DocumentVectorizer for NvidiaDocumentVectorizer {
    async fn vectorize(&self, content: &str) -> Result<DocumentVector> {
        let request = EmbeddingRequest {
            input: vec![content.to_string()],
            model: self.model_name.clone(),
            input_type: "passage".to_string(),
        };
        
        let response = self.client
            .post("https://ai.api.nvidia.com/v1/retrieval/nvidia/nv-embedqa-e5-v5/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!("NVIDIA API请求失败: {}", error_text));
        }
        
        let embedding_response: EmbeddingResponse = response.json().await?;
        
        if let Some(embedding_data) = embedding_response.data.first() {
            Ok(embedding_data.embedding.clone())
        } else {
            Err(anyhow::anyhow!("NVIDIA API返回空的嵌入向量"))
        }
    }
    
    fn calculate_similarity(&self, vec1: &DocumentVector, vec2: &DocumentVector) -> f32 {
        if vec1.len() != vec2.len() {
            return 0.0;
        }
        
        // 计算余弦相似度
        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm1 == 0.0 || norm2 == 0.0 {
            0.0
        } else {
            dot_product / (norm1 * norm2)
        }
    }
} 