use anyhow::Result;
use async_trait::async_trait;

use crate::tools::base::{
    DocumentVector, FileDocumentFragment, FileSearchResult, HierarchyFilter,
};

/// 向量存储接口
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// 初始化存储
    async fn initialize(&self) -> Result<()>;
    
    /// 检查存储是否健康
    async fn health_check(&self) -> Result<bool>;
    
    /// 获取存储信息
    async fn get_info(&self) -> Result<VectorStoreInfo>;
}

/// 文档向量存储接口
#[async_trait]
pub trait DocumentVectorStore: VectorStore {
    /// 存储单个文件向量
    async fn store_file_vector(
        &self,
        vector: &DocumentVector,
        fragment: &FileDocumentFragment,
    ) -> Result<()>;
    
    /// 批量存储文件向量
    async fn store_file_vectors_batch(
        &self,
        vectors: &[(DocumentVector, FileDocumentFragment)],
    ) -> Result<()>;
    
    /// 搜索相似向量
    async fn search_similar(
        &self,
        query_vector: Vec<f32>,
        limit: Option<u64>,
        threshold: Option<f32>,
    ) -> Result<Vec<FileSearchResult>>;
    
    /// 层次化搜索
    async fn search_with_hierarchy(
        &self,
        query_vector: Vec<f32>,
        filter: &HierarchyFilter,
    ) -> Result<Vec<FileSearchResult>>;
    
    /// 检查文档是否存在
    async fn file_exists(
        &self,
        language: &str,
        package: &str,
        version: &str,
        file_path: &str,
    ) -> Result<bool>;
    
    /// 获取文档
    async fn get_file_document(
        &self,
        language: &str,
        package: &str,
        version: &str,
        file_path: &str,
    ) -> Result<Option<FileDocumentFragment>>;
    
    /// 删除包的所有文档
    async fn delete_package_docs(
        &self,
        language: &str,
        package: &str,
        version: &str,
    ) -> Result<()>;
    
    /// 删除单个文档
    async fn delete_file_document(
        &self,
        language: &str,
        package: &str,
        version: &str,
        file_path: &str,
    ) -> Result<()>;
    
    /// 获取包的所有文件
    async fn list_package_files(
        &self,
        language: &str,
        package: &str,
        version: &str,
    ) -> Result<Vec<String>>;
    
    /// 获取存储统计信息
    async fn get_storage_stats(&self) -> Result<StorageStats>;
}

/// 向量存储信息
#[derive(Debug, Clone)]
pub struct VectorStoreInfo {
    pub store_type: String,
    pub version: String,
    pub total_collections: usize,
    pub total_vectors: usize,
    pub memory_usage: Option<u64>,
    pub disk_usage: Option<u64>,
}

/// 存储统计信息
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// 总文档数
    pub total_documents: usize,
    /// 总向量数
    pub total_vectors: usize,
    /// 按语言分组的统计
    pub by_language: std::collections::HashMap<String, LanguageStats>,
    /// 按包分组的统计
    pub by_package: std::collections::HashMap<String, PackageStats>,
    /// 存储大小（字节）
    pub storage_size_bytes: u64,
    /// 最后更新时间
    pub last_updated: std::time::SystemTime,
}

/// 语言级统计信息
#[derive(Debug, Clone)]
pub struct LanguageStats {
    pub language: String,
    pub document_count: usize,
    pub package_count: usize,
    pub total_size_bytes: u64,
}

/// 包级统计信息
#[derive(Debug, Clone)]
pub struct PackageStats {
    pub package_name: String,
    pub language: String,
    pub version_count: usize,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub latest_version: Option<String>,
} 