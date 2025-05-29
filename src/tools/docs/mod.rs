/// 文档处理核心 trait 和数据结构
pub mod doc_traits;

/// 基于 OpenAI 兼容 API 的向量化器
pub mod openai_vectorizer;

/// 基于文件系统的持久化存储
pub mod file_store;

/// 文档存储工厂
pub mod store_factory;

/// 重排器模块
pub mod reranker;

// 重新导出核心类型
pub use doc_traits::*;
pub use openai_vectorizer::OpenAIVectorizer;
pub use file_store::FileDocumentStore;
pub use store_factory::{DocumentStoreFactory, StoreType};
pub use reranker::{DocumentReranker, RerankerConfig, RerankResult}; 