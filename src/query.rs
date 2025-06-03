use crate::{
    types::*, 
    config::VectorDbConfig, 
    storage::VectorStore, 
    index::HnswIndex,
    metrics::{MetricsCollector, QueryTimer},
    errors::{Result, VectorDbError}
};
use std::sync::Arc;
use std::collections::HashMap;

/// 查询引擎
pub struct QueryEngine {
    config: VectorDbConfig,
    hnsw_index: Arc<HnswIndex>,
    metrics: Arc<MetricsCollector>,
}

impl QueryEngine {
    pub fn new(config: &VectorDbConfig, metrics: Arc<MetricsCollector>) -> Result<Self> {
        // 创建HNSW索引
        let hnsw_index = Arc::new(HnswIndex::new(
            config.hnsw.clone(),
            config.vector_dimension,
        ));

        Ok(Self {
            config: config.clone(),
            hnsw_index,
            metrics,
        })
    }
} 