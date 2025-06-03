//! # Grape MCP DevTools
//!
//! 一个基于 MCP (Model Context Protocol) 的多语言文档服务，专为 LLM 提供文档查询和版本检查功能。
//! 
//! ## 特性
//! 
//! - 🔍 **文档搜索** - 搜索各种编程语言的包信息、API文档和使用指南
//! - 📦 **版本检查** - 获取包的最新版本、版本历史和兼容性信息
//! - 📚 **API文档** - 获取编程语言API的详细文档信息
//! - 🚀 **MCP协议** - 基于标准MCP协议，支持stdio模式通信
//! 
//! ## 快速开始
//! 
//! ```rust
//! use grape_mcp_devtools::*;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), MCPError> {
//!     // 创建MCP服务器
//!     let server = mcp::create_server().await?;
//!     
//!     // 启动服务器
//!     server.run().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod errors;
pub mod mcp;
pub mod tools;
pub mod versioning;
pub mod cli;
pub mod language_features;
pub mod ai;
pub mod config;
pub mod types;
pub mod storage;
pub mod index;
pub mod query;
pub mod metrics;
pub mod embeddings;

// 新增：智能MCP服务器模块（同进程多Agent架构）
// pub mod intelligent_mcp_server;

pub use errors::{MCPError, MCPResult};
pub use config::*;
pub use types::*;
pub use storage::*;
pub use index::HnswIndex;
pub use query::{QueryEngine, IndexStats as QueryIndexStats};
pub use metrics::*;
pub use embeddings::*;
pub use errors::*;

// 明确指定SearchResult类型，避免冲突
pub use types::SearchResult;

use std::path::PathBuf;
use std::sync::Arc;

/// 向量数据库主结构
pub struct VectorDatabase {
    storage: Box<dyn VectorStore>,
    query_engine: QueryEngine,
    metrics: Arc<MetricsCollector>,
    config: VectorDbConfig,
}

impl VectorDatabase {
    /// 创建新的向量数据库实例
    pub async fn new(data_dir: PathBuf, config: VectorDbConfig) -> Result<Self> {
        let metrics = Arc::new(MetricsCollector::new());
        
        // 创建存储层
        let storage = Box::new(SledVectorStore::new(data_dir.clone(), &config).await?);
        
        // 创建查询引擎
        let query_engine = QueryEngine::new(&config, metrics.clone())?;

        Ok(Self {
            storage,
            query_engine,
            metrics,
            config,
        })
    }

    /// 使用OpenAI兼容API创建向量数据库
    pub async fn with_openai_compatible(
        data_dir: PathBuf,
        endpoint: String,
        api_key: String,
        model: String,
    ) -> Result<Self> {
        let config = VectorDbConfig::with_openai_compatible(endpoint, api_key, model);
        Self::new(data_dir, config).await
    }

    /// 使用Azure OpenAI创建向量数据库
    pub async fn with_azure_openai(
        data_dir: PathBuf,
        endpoint: String,
        api_key: String,
        deployment_name: String,
        api_version: String,
    ) -> Result<Self> {
        let config = VectorDbConfig::with_azure_openai(endpoint, api_key, deployment_name, api_version);
        Self::new(data_dir, config).await
    }

    /// 使用Ollama创建向量数据库
    pub async fn with_ollama(
        data_dir: PathBuf,
        endpoint: String,
        model: String,
    ) -> Result<Self> {
        let config = VectorDbConfig::with_ollama(endpoint, model);
        Self::new(data_dir, config).await
    }

    /// 使用自定义配置创建向量数据库
    pub async fn with_config(data_dir: PathBuf, config: VectorDbConfig) -> Result<Self> {
        Self::new(data_dir, config).await
    }

    /// 添加文档
    pub async fn add_document(&mut self, document: Document) -> Result<String> {
        let _timer = QueryTimer::new(self.metrics.clone());

        // 生成嵌入向量
        let embedding_provider = create_embedding_provider(&self.config.embedding)?;
        let embedding = embedding_provider.generate_embedding(&document.content).await?;
        
        // 创建文档记录
        let record = DocumentRecord {
            id: document.id.clone(),
            title: document.title.unwrap_or_else(|| "无标题".to_string()),
            content: document.content.clone(),
            embedding,
            package_name: document.package_name.unwrap_or_else(|| "unknown".to_string()),
            doc_type: document.doc_type.unwrap_or_else(|| "unknown".to_string()),
            language: document.language.unwrap_or_else(|| "unknown".to_string()),
            version: document.version.unwrap_or_else(|| "1.0".to_string()),
            metadata: document.metadata.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // 保存到存储
        self.storage.add_document(record.clone()).await?;
        
        // 添加到索引
        self.query_engine.add_document(&record).await?;

        // 更新指标
        let stats = self.storage.stats();
        self.metrics.update_document_count(stats.document_count as u64);

        Ok(document.id)
    }

    /// 获取文档
    pub async fn get_document(&self, id: &str) -> Result<Option<Document>> {
        let _timer = QueryTimer::new(self.metrics.clone());

        if let Some(record) = self.storage.get_document(id).await? {
            self.metrics.record_cache_hit();
            Ok(Some(Document {
                id: record.id,
                title: Some(record.title),
                content: record.content,
                package_name: Some(record.package_name),
                doc_type: Some(record.doc_type),
                language: Some(record.language),
                version: Some(record.version),
                metadata: record.metadata,
            }))
        } else {
            self.metrics.record_cache_miss();
            Ok(None)
        }
    }

    /// 删除文档
    pub async fn delete_document(&mut self, id: &str) -> Result<bool> {
        let _timer = QueryTimer::new(self.metrics.clone());

        // 从存储删除
        let deleted_from_storage = self.storage.delete_document(id).await?;
        
        // 从索引删除
        let deleted_from_index = self.query_engine.remove_document(id).await?;

        if deleted_from_storage || deleted_from_index {
            // 更新指标
            let stats = self.storage.stats();
            self.metrics.update_document_count(stats.document_count as u64);
        }

        Ok(deleted_from_storage || deleted_from_index)
    }

    /// 更新文档
    pub async fn update_document(&mut self, document: Document) -> Result<()> {
        let _timer = QueryTimer::new(self.metrics.clone());

        // 生成新的嵌入向量
        let embedding_provider = create_embedding_provider(&self.config.embedding)?;
        let embedding = embedding_provider.generate_embedding(&document.content).await?;
        
        // 创建更新的文档记录
        let record = DocumentRecord {
            id: document.id.clone(),
            title: document.title.unwrap_or_else(|| "无标题".to_string()),
            content: document.content.clone(),
            embedding,
            package_name: document.package_name.unwrap_or_else(|| "unknown".to_string()),
            doc_type: document.doc_type.unwrap_or_else(|| "unknown".to_string()),
            language: document.language.unwrap_or_else(|| "unknown".to_string()),
            version: document.version.unwrap_or_else(|| "1.0".to_string()),
            metadata: document.metadata.clone(),
            created_at: chrono::Utc::now(), // 这里应该保留原始创建时间，但简化实现
            updated_at: chrono::Utc::now(),
        };

        // 更新存储
        self.storage.update_document(record.clone()).await?;
        
        // 更新索引（先删除再添加）
        self.query_engine.remove_document(&document.id).await?;
        self.query_engine.add_document(&record).await?;

        Ok(())
    }

    /// 向量搜索
    pub async fn vector_search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        self.query_engine.vector_search(&*self.storage, query_vector, limit).await
    }

    /// 文本搜索
    pub async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.query_engine.text_search(&*self.storage, query, limit).await
    }

    /// 混合搜索（向量 + 文本）
    pub async fn hybrid_search(
        &self,
        query_text: &str,
        limit: usize,
        vector_weight: f32,
        text_weight: f32,
    ) -> Result<Vec<SearchResult>> {
        // 生成查询向量
        let embedding_provider = create_embedding_provider(&self.config.embedding)?;
        let query_vector = embedding_provider.generate_embedding(query_text).await?;

        self.query_engine.search(
            &*self.storage,
            Some(&query_vector),
            Some(query_text),
            limit,
            vector_weight,
            text_weight,
        ).await
    }

    /// 语义搜索（基于文本生成向量）
    pub async fn semantic_search(&self, query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let embedding_provider = create_embedding_provider(&self.config.embedding)?;
        let query_vector = embedding_provider.generate_embedding(query_text).await?;
        
        self.vector_search(&query_vector, limit).await
    }

    /// 简化的搜索方法（主要用于测试）
    pub async fn search(&self, query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.semantic_search(query_text, limit).await
    }

    /// 列出文档
    pub async fn list_documents(&self, offset: usize, limit: usize) -> Result<Vec<Document>> {
        let _timer = QueryTimer::new(self.metrics.clone());

        let records = self.storage.list_documents(offset, limit).await?;
        let documents = records.into_iter().map(|record| Document {
            id: record.id,
            title: Some(record.title),
            content: record.content,
            package_name: Some(record.package_name),
            doc_type: Some(record.doc_type),
            language: Some(record.language),
            version: Some(record.version),
            metadata: record.metadata,
        }).collect();

        Ok(documents)
    }

    /// 重建索引
    pub async fn rebuild_index(&self) -> Result<()> {
        self.query_engine.rebuild_index().await
    }

    /// 保存数据库
    pub async fn save(&self) -> Result<()> {
        self.storage.save().await
    }

    /// 压缩数据库
    pub async fn compact(&self) -> Result<()> {
        self.storage.compact().await
    }

    /// 获取数据库统计信息
    pub fn get_stats(&self) -> DatabaseStats {
        self.storage.stats()
    }

    /// 获取性能指标
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.get_metrics()
    }

    /// 获取索引统计信息
    pub fn get_index_stats(&self) -> QueryIndexStats {
        self.query_engine.get_index_stats()
    }

    /// 重置指标
    pub fn reset_metrics(&self) {
        self.metrics.reset();
    }

    /// 获取配置
    pub fn get_config(&self) -> &VectorDbConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_vector_database() {
        let temp_dir = TempDir::new().unwrap();
        let config = VectorDbConfig::default();
        
        let mut db = VectorDatabase::new(temp_dir.path().to_path_buf(), config).await.unwrap();

        // 添加文档
        let doc = Document {
            id: "test1".to_string(),
            title: Some("测试文档".to_string()),
            content: "这是一个测试文档的内容".to_string(),
            package_name: Some("test_package".to_string()),
            doc_type: Some("test".to_string()),
            ..Default::default()
        };

        let doc_id = db.add_document(doc.clone()).await.unwrap();
        assert_eq!(doc_id, "test1");

        // 获取文档
        let retrieved = db.get_document("test1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, Some("测试文档".to_string()));

        // 搜索
        let results = db.text_search("测试", 5).await.unwrap();
        assert!(!results.is_empty());

        // 获取统计信息
        let stats = db.get_stats();
        assert_eq!(stats.document_count, 1);

        // 删除文档
        let deleted = db.delete_document("test1").await.unwrap();
        assert!(deleted);

        let stats = db.get_stats();
        assert_eq!(stats.document_count, 0);
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let temp_dir = TempDir::new().unwrap();
        let config = VectorDbConfig::default();
        
        let mut db = VectorDatabase::new(temp_dir.path().to_path_buf(), config).await.unwrap();

        // 添加一些测试文档
        let docs = vec![
            Document {
                id: "doc1".to_string(),
                title: Some("Rust编程语言".to_string()),
                content: "Rust是一种系统编程语言，注重安全性和性能".to_string(),
                package_name: Some("rust".to_string()),
                doc_type: Some("tutorial".to_string()),
                ..Default::default()
            },
            Document {
                id: "doc2".to_string(),
                title: Some("Python数据科学".to_string()),
                content: "Python是数据科学和机器学习的热门语言".to_string(),
                package_name: Some("python".to_string()),
                doc_type: Some("guide".to_string()),
                ..Default::default()
            },
        ];

        for doc in docs {
            db.add_document(doc).await.unwrap();
        }

        // 重建索引
        db.rebuild_index().await.unwrap();

        // 语义搜索
        let results = db.semantic_search("编程语言", 5).await.unwrap();
        assert!(!results.is_empty());
        
        // 混合搜索
        let results = db.hybrid_search("编程", 5, 0.7, 0.3).await.unwrap();
        assert!(!results.is_empty());
    }
}

// Re-export commonly used types
pub use async_trait::async_trait;
pub use serde_json::{json, Value}; 