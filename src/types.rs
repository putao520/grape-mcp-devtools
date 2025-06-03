use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// 文档结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub package_name: Option<String>,
    pub doc_type: Option<String>,
    pub language: Option<String>,
    pub version: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: None,
            content: String::new(),
            package_name: None,
            doc_type: None,
            language: None,
            version: None,
            metadata: HashMap::new(),
        }
    }
}

/// 文档记录（包含嵌入向量）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: String,
    pub title: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub package_name: String,
    pub doc_type: String,
    pub language: String,
    pub version: String,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document_id: String,
    pub title: String,
    pub content_snippet: String,
    pub similarity_score: f32,
    pub package_name: String,
    pub doc_type: String,
    pub metadata: HashMap<String, String>,
}

/// 向量点
#[derive(Debug, Clone)]
pub struct VectorPoint {
    pub vector: Vec<f32>,
    pub document_id: String,
}

/// 数据库统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub document_count: usize,
    pub vector_count: usize,
    pub total_size_mb: f64,
    pub memory_usage_mb: f64,
    pub index_size_mb: f64,
    pub last_updated: DateTime<Utc>,
}

impl Default for DatabaseStats {
    fn default() -> Self {
        Self {
            document_count: 0,
            vector_count: 0,
            total_size_mb: 0.0,
            memory_usage_mb: 0.0,
            index_size_mb: 0.0,
            last_updated: Utc::now(),
        }
    }
} 