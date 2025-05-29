use anyhow::Result;
use tracing::info;

use super::{
    doc_traits::{DocumentStore, DocumentVectorizer},
    file_store::FileDocumentStore,
    openai_vectorizer::OpenAIVectorizer,
};

/// 文档存储类型
#[derive(Debug, Clone)]
pub enum StoreType {
    /// 使用文件系统存储（真正的嵌入式存储，无需外部服务）
    FileEmbedded { storage_path: String },
}

/// 文档存储工厂
pub struct DocumentStoreFactory;

impl DocumentStoreFactory {
    /// 创建文档存储实例
    /// 
    /// # 参数
    /// - `store_type`: 存储类型
    /// - `collection_name`: 集合/存储名称
    /// - `vector_dimension`: 向量维度
    /// - `vectorizer`: 向量生成器
    pub async fn create_store(
        store_type: StoreType,
        collection_name: String,
        vector_dimension: usize,
        vectorizer: Box<dyn DocumentVectorizer>,
    ) -> Result<Box<dyn DocumentStore>> {
        match store_type {
            StoreType::FileEmbedded { storage_path } => {
                info!("创建嵌入式文件存储: {}", storage_path);
                let store = FileDocumentStore::new(
                    &storage_path,
                    vectorizer,
                ).await?;
                Ok(Box::new(store))
            }
        }
    }

    /// 创建带智能回退的存储实例
    /// 
    /// 直接使用文件存储
    pub async fn create_with_fallback(
        collection_name: String,
        vector_dimension: usize,
        vectorizer: Box<dyn DocumentVectorizer>,
        fallback_path: &str,
    ) -> Result<Box<dyn DocumentStore>> {
        info!("使用嵌入式文件存储");
        
        let store = Self::create_store(
            StoreType::FileEmbedded { 
                storage_path: fallback_path.to_string() 
            },
            collection_name,
            vector_dimension,
            vectorizer,
        ).await?;
        
        info!("✅ 使用文件存储");
        Ok(store)
    }

    /// 获取推荐的存储配置
    pub fn get_recommended_config() -> StoreType {
        StoreType::FileEmbedded {
            storage_path: "./data/docs".to_string(),
        }
    }

    /// 获取存储类型的描述
    pub fn get_store_description(store_type: &StoreType) -> &'static str {
        match store_type {
            StoreType::FileEmbedded { .. } => "嵌入式文件存储（简单易用，无需外部服务）",
        }
    }
} 