/// 搜索最相似的向量
pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
    if query.len() != self.dimension {
        return Err(VectorDbError::InvalidVectorDimension {
            expected: self.dimension,
            actual: query.len(),
        });
    }

    // 确保索引已构建
    if self.index.read().is_none() {
        drop(self.index.read()); // 释放读锁
        self.build_index()?;
    }

    let index_guard = self.index.read();
    let index = index_guard.as_ref().ok_or_else(|| {
        VectorDbError::index_error("索引未构建".to_string())
    })?;

    let query_point = HnswPoint {
        vector: query.to_vec(),
        document_id: "query".to_string(),
    };

    // 创建搜索对象
    let mut search = instant_distance::Search::default();
    let search_results = index.search(&query_point, &mut search);
    
    let mut results = Vec::new();
    for item in search_results.take(k) {
        let point = item.point;
        results.push(SearchResult {
            document_id: point.document_id.clone(),
            distance: item.distance,
            similarity: 1.0 / (1.0 + item.distance), // 转换为相似度分数
        });
    }

    Ok(results)
} 