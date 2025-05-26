use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::SystemTime;

// 直接使用 qdrant-client 的现代 API
use qdrant_client::{
    Qdrant,
    qdrant::{
        vectors_config::Config as VectorsConfig,
        Distance, VectorParams, PointStruct, Vectors,
        CreateCollection, UpsertPoints, SearchPoints, GetPoints, DeletePoints, ScrollPoints,
        Filter, Condition, FieldCondition, Match,
        condition::ConditionOneOf,
        r#match::MatchValue,
        PointsSelector, points_selector::PointsSelectorOneOf,
        PointsIdsList, WithPayloadSelector,
    },
    Payload,
};

use crate::tools::base::{
    DocumentVector, FileDocumentFragment, FileSearchResult, HierarchyFilter,
};
use crate::storage::traits::{
    DocumentVectorStore, VectorStore, VectorStoreInfo, StorageStats,
    LanguageStats, PackageStats,
};

/// 简化的 Qdrant 配置
#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub collection_prefix: String,
    pub vector_dimension: u64,
    pub distance: Distance,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:6334".to_string(),
            api_key: None,
            collection_prefix: "mcp_".to_string(),
            vector_dimension: 768,
            distance: Distance::Cosine,
        }
    }
}

impl QdrantConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6334".to_string()),
            api_key: std::env::var("QDRANT_API_KEY").ok(),
            collection_prefix: std::env::var("VECTOR_DB_COLLECTION_PREFIX")
                .unwrap_or_else(|_| "mcp_".to_string()),
            vector_dimension: std::env::var("VECTOR_DIMENSION")
                .unwrap_or_else(|_| "768".to_string())
                .parse()
                .unwrap_or(768),
            distance: match std::env::var("VECTOR_DB_DISTANCE")
                .unwrap_or_else(|_| "Cosine".to_string())
                .to_lowercase()
                .as_str()
            {
                "dot" => Distance::Dot,
                "euclid" | "euclidean" => Distance::Euclid,
                "manhattan" => Distance::Manhattan,
                _ => Distance::Cosine,
            },
        })
    }
}

/// 简化的 Qdrant 文件存储 - 直接使用原生客户端
pub struct QdrantFileStore {
    client: Qdrant,
    config: QdrantConfig,
}

impl QdrantFileStore {
    /// 创建新实例 - 使用现代 qdrant-client API
    pub async fn new(config: QdrantConfig) -> Result<Self> {
        let mut client_config = qdrant_client::config::QdrantConfig::from_url(&config.url);
        
        if let Some(api_key) = &config.api_key {
            client_config = client_config.api_key(api_key.clone());
        }
        
        let client = Qdrant::new(client_config)?;
        
        Ok(Self { client, config })
    }

    pub async fn from_env() -> Result<Self> {
        let config = QdrantConfig::from_env()?;
        Self::new(config).await
    }

    fn collection_name(&self, language: &str) -> String {
        format!("{}{}", self.config.collection_prefix, language)
    }

    /// 确保集合存在 - 使用原生 API
    async fn ensure_collection(&self, language: &str) -> Result<()> {
        let collection_name = self.collection_name(language);
        
        // 检查集合是否存在
        if self.client.collection_exists(&collection_name).await? {
            return Ok(());
        }

        // 创建集合
        let vectors_config = VectorsConfig::Params(VectorParams {
            size: self.config.vector_dimension,
            distance: self.config.distance.into(),
            hnsw_config: None,
            quantization_config: None,
            on_disk: Some(true),
            datatype: None,
            multivector_config: None,
        });

        let create_collection = CreateCollection {
            collection_name: collection_name.clone(),
            vectors_config: Some(vectors_config.into()),
            ..Default::default()
        };

        self.client.create_collection(create_collection).await?;
        tracing::info!("✅ 创建 Qdrant 集合: {}", collection_name);
        
        Ok(())
    }

    /// 构建点 ID
    fn build_point_id(&self, fragment: &FileDocumentFragment) -> String {
        format!(
            "{}:{}:{}:{}",
            fragment.language,
            fragment.package_name,
            fragment.version,
            fragment.file_path
        )
    }

    /// 构建 payload
    fn build_payload(&self, fragment: &FileDocumentFragment, keywords: &[String]) -> Payload {
        let mut payload = Payload::new();
        
        payload.insert("package_name", fragment.package_name.clone());
        payload.insert("version", fragment.version.clone());
        payload.insert("file_path", fragment.file_path.clone());
        payload.insert("language", fragment.language.clone());
        payload.insert("hierarchy_path", fragment.hierarchy_path.clone());
        payload.insert("content", fragment.content.clone());
        payload.insert("keywords", keywords.to_vec());
        payload.insert("created_at", chrono::Utc::now().timestamp());
        
        payload
    }

    /// 构建搜索过滤器 - 使用正确的 API
    fn build_filter(&self, filter: &HierarchyFilter) -> Option<Filter> {
        let mut conditions = Vec::new();

        if let Some(package) = &filter.package_name {
            conditions.push(Condition {
                condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                    key: "package_name".to_string(),
                    r#match: Some(Match {
                        match_value: Some(MatchValue::Keyword(package.clone())),
                    }),
                    ..Default::default()
                })),
            });
        }

        if let Some(version) = &filter.version {
            conditions.push(Condition {
                condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                    key: "version".to_string(),
                    r#match: Some(Match {
                        match_value: Some(MatchValue::Keyword(version.clone())),
                    }),
                    ..Default::default()
                })),
            });
        }

        if conditions.is_empty() {
            None
        } else {
            Some(Filter {
                must: conditions,
                ..Default::default()
            })
        }
    }

    /// 转换搜索结果
    fn convert_search_result(&self, point: &qdrant_client::qdrant::ScoredPoint) -> Result<FileSearchResult> {
        let payload = &point.payload;
        
        let language = payload.get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少language字段"))?;
        let package_name = payload.get("package_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少package_name字段"))?;
        let version = payload.get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少version字段"))?;
        let file_path = payload.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少file_path字段"))?;
        let content = payload.get("content")
            .and_then(|v| v.as_str())
            .map_or("内容未找到", |v| v);

        let fragment = FileDocumentFragment::new(
            language.to_string(),
            package_name.to_string(),
            version.to_string(),
            file_path.to_string(),
            content.to_string(),
        );

        let mut result = FileSearchResult::new(fragment, point.score);

        // 提取关键词
        if let Some(keywords) = payload.get("keywords") {
            if let Some(keywords_vec) = keywords.as_list() {
                result.matched_keywords = keywords_vec.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl VectorStore for QdrantFileStore {
    async fn initialize(&self) -> Result<()> {
        tracing::info!("初始化 Qdrant 客户端，连接到: {}", self.config.url);
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool> {
        match self.client.health_check().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    async fn get_info(&self) -> Result<VectorStoreInfo> {
        let collections = self.client.list_collections().await?;
        let total_collections = collections.collections.len();
        
        let mut total_vectors = 0;
        for collection in &collections.collections {
            if let Ok(info) = self.client.collection_info(&collection.name).await {
                if let Some(result) = &info.result {
                    total_vectors += result.vectors_count.unwrap_or(0) as usize;
                }
            }
        }

        Ok(VectorStoreInfo {
            store_type: "Qdrant".to_string(),
            version: "1.14.0".to_string(),
            total_collections,
            total_vectors,
            memory_usage: None,
            disk_usage: None,
        })
    }
}

#[async_trait]
impl DocumentVectorStore for QdrantFileStore {
    async fn store_file_vector(&self, vector: &DocumentVector, fragment: &FileDocumentFragment) -> Result<()> {
        let collection_name = self.collection_name(&fragment.language);
        self.ensure_collection(&fragment.language).await?;
        
        let point_id = self.build_point_id(fragment);
        let payload = self.build_payload(fragment, &vector.metadata.keywords);
        
        let point = PointStruct::new(
            point_id,
            Vectors::from(vector.data.clone()),
            payload,
        );
        
        let upsert_points = UpsertPoints {
            collection_name,
            points: vec![point],
            ..Default::default()
        };
        
        self.client.upsert_points(upsert_points).await?;
        Ok(())
    }

    async fn store_file_vectors_batch(&self, vector_fragment_pairs: &[(DocumentVector, FileDocumentFragment)]) -> Result<()> {
        // 按语言分组批量处理
        let mut language_groups: HashMap<String, Vec<PointStruct>> = HashMap::new();
        
        for (vector, fragment) in vector_fragment_pairs {
            let point_id = self.build_point_id(fragment);
            let payload = self.build_payload(fragment, &vector.metadata.keywords);
            let point = PointStruct::new(point_id, Vectors::from(vector.data.clone()), payload);
            
            language_groups.entry(fragment.language.clone())
                .or_default()
                .push(point);
        }
        
        for (language, points) in language_groups {
            let collection_name = self.collection_name(&language);
            self.ensure_collection(&language).await?;
            
            let upsert_points = UpsertPoints {
                collection_name,
                points,
                ..Default::default()
            };
            
            self.client.upsert_points(upsert_points).await?;
        }
        
        Ok(())
    }

    async fn search_similar(
        &self,
        query_vector: Vec<f32>,
        limit: Option<u64>,
        threshold: Option<f32>,
    ) -> Result<Vec<FileSearchResult>> {
        // 搜索所有语言集合
        let collections = self.client.list_collections().await?;
        let mut all_results = Vec::new();
        
        for collection in collections.collections {
            if !collection.name.starts_with(&self.config.collection_prefix) {
                continue;
            }
            
            let search_points = SearchPoints {
                collection_name: collection.name,
                vector: query_vector.clone(),
                limit: limit.unwrap_or(10),
                score_threshold: threshold,
                with_payload: Some(WithPayloadSelector::from(true)),
                ..Default::default()
            };
            
            if let Ok(response) = self.client.search_points(search_points).await {
                for point in response.result {
                    if let Ok(result) = self.convert_search_result(&point) {
                        all_results.push(result);
                    }
                }
            }
        }
        
        // 按分数排序
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        all_results.truncate(limit.unwrap_or(10) as usize);
        
        Ok(all_results)
    }

    async fn search_with_hierarchy(&self, query_vector: Vec<f32>, filter: &HierarchyFilter) -> Result<Vec<FileSearchResult>> {
        let language = filter.language.as_ref()
            .ok_or_else(|| anyhow!("搜索需要指定语言"))?;
        
        let collection_name = self.collection_name(language);
        self.ensure_collection(language).await?;
        
        let search_points = SearchPoints {
            collection_name,
            vector: query_vector,
            limit: filter.limit.unwrap_or(10) as u64,
            filter: self.build_filter(filter),
            with_payload: Some(WithPayloadSelector::from(true)),
            ..Default::default()
        };
        
        let response = self.client.search_points(search_points).await?;
        let mut results = Vec::new();
        
        for point in response.result {
            if let Ok(result) = self.convert_search_result(&point) {
                results.push(result);
            }
        }
        
        Ok(results)
    }

    async fn get_file_document(
        &self,
        language: &str,
        package: &str,
        version: &str,
        file_path: &str,
    ) -> Result<Option<FileDocumentFragment>> {
        let collection_name = self.collection_name(language);
        self.ensure_collection(language).await?;
        
        let point_id = format!("{}:{}:{}:{}", language, package, version, file_path);
        
        let get_points = GetPoints {
            collection_name,
            ids: vec![point_id.into()],
            with_payload: Some(WithPayloadSelector::from(true)),
            ..Default::default()
        };
        
        let response = self.client.get_points(get_points).await?;
        
        if let Some(point) = response.result.first() {
            let payload = &point.payload;
            let content = payload.get("content")
                .and_then(|v| v.as_str())
                .map_or("", |v| v);
            
            Ok(Some(FileDocumentFragment::new(
                language.to_string(),
                package.to_string(),
                version.to_string(),
                file_path.to_string(),
                content.to_string(),
            )))
        } else {
            Ok(None)
        }
    }

    async fn file_exists(&self, language: &str, package: &str, version: &str, file_path: &str) -> Result<bool> {
        let result = self.get_file_document(language, package, version, file_path).await?;
        Ok(result.is_some())
    }

    async fn delete_package_docs(&self, language: &str, package: &str, version: &str) -> Result<()> {
        let collection_name = self.collection_name(language);
        self.ensure_collection(language).await?;
        
        let filter = Filter {
            must: vec![
                Condition {
                    condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                        key: "package_name".to_string(),
                        r#match: Some(Match {
                            match_value: Some(MatchValue::Keyword(package.to_string())),
                        }),
                        ..Default::default()
                    })),
                },
                Condition {
                    condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                        key: "version".to_string(),
                        r#match: Some(Match {
                            match_value: Some(MatchValue::Keyword(version.to_string())),
                        }),
                        ..Default::default()
                    })),
                },
            ],
            ..Default::default()
        };

        let delete_points = DeletePoints {
            collection_name,
            points: Some(PointsSelector {
                points_selector_one_of: Some(PointsSelectorOneOf::Filter(filter)),
            }),
            ..Default::default()
        };

        self.client.delete_points(delete_points).await?;
        Ok(())
    }

    async fn delete_file_document(&self, language: &str, package: &str, version: &str, file_path: &str) -> Result<()> {
        let collection_name = self.collection_name(language);
        self.ensure_collection(language).await?;
        
        let point_id = format!("{}:{}:{}:{}", language, package, version, file_path);
        
        let delete_points = DeletePoints {
            collection_name,
            points: Some(PointsSelector {
                points_selector_one_of: Some(PointsSelectorOneOf::Points(PointsIdsList {
                    ids: vec![point_id.into()],
                })),
            }),
            ..Default::default()
        };

        self.client.delete_points(delete_points).await?;
        Ok(())
    }

    async fn list_package_files(&self, language: &str, package: &str, version: &str) -> Result<Vec<String>> {
        let collection_name = self.collection_name(language);
        self.ensure_collection(language).await?;
        
        let filter = Filter {
            must: vec![
                Condition {
                    condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                        key: "package_name".to_string(),
                        r#match: Some(Match {
                            match_value: Some(MatchValue::Keyword(package.to_string())),
                        }),
                        ..Default::default()
                    })),
                },
                Condition {
                    condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                        key: "version".to_string(),
                        r#match: Some(Match {
                            match_value: Some(MatchValue::Keyword(version.to_string())),
                        }),
                        ..Default::default()
                    })),
                },
            ],
            ..Default::default()
        };

        let scroll_points = ScrollPoints {
            collection_name,
            filter: Some(filter),
            limit: Some(1000),
            with_payload: Some(WithPayloadSelector::from(true)),
            ..Default::default()
        };

        let response = self.client.scroll(scroll_points).await?;
        let mut files = Vec::new();
        
        for point in response.result {
            if let Some(file_path) = point.payload.get("file_path").and_then(|v| v.as_str()) {
                files.push(file_path.to_string());
            }
        }

        files.sort();
        Ok(files)
    }

    async fn get_storage_stats(&self) -> Result<StorageStats> {
        let mut by_language = HashMap::new();
        let mut by_package = HashMap::new();
        let mut total_documents = 0;
        let mut total_vectors = 0;
        
        let collections = self.client.list_collections().await?;
        
        for collection in collections.collections {
            if let Ok(info) = self.client.collection_info(&collection.name).await {
                if let Some(result) = &info.result {
                    let vectors_count = result.vectors_count.unwrap_or(0) as usize;
                    total_vectors += vectors_count;
                    
                    // 使用 scroll 获取统计信息
                    let scroll_points = ScrollPoints {
                        collection_name: collection.name.clone(),
                        limit: Some(1000),
                        with_payload: Some(WithPayloadSelector::from(true)),
                        ..Default::default()
                    };
                    
                    if let Ok(scroll_result) = self.client.scroll(scroll_points).await {
                        for point in scroll_result.result {
                            total_documents += 1;
                            
                            // 统计语言信息
                            if let Some(language) = point.payload.get("language").and_then(|v| v.as_str()) {
                                let lang_stats = by_language.entry(language.to_string())
                                    .or_insert_with(|| LanguageStats {
                                        language: language.to_string(),
                                        document_count: 0,
                                        package_count: 0,
                                        total_size_bytes: 0,
                                    });
                                
                                lang_stats.document_count += 1;
                            }
                            
                            // 统计包信息
                            if let (Some(package), Some(language)) = (
                                point.payload.get("package_name").and_then(|v| v.as_str()),
                                point.payload.get("language").and_then(|v| v.as_str())
                            ) {
                                let pkg_stats = by_package.entry(package.to_string())
                                    .or_insert_with(|| PackageStats {
                                        package_name: package.to_string(),
                                        language: language.to_string(),
                                        version_count: 0,
                                        file_count: 0,
                                        total_size_bytes: 0,
                                        latest_version: None,
                                    });
                                
                                pkg_stats.file_count += 1;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(StorageStats {
            total_documents,
            total_vectors,
            by_language,
            by_package,
            storage_size_bytes: 0,
            last_updated: SystemTime::now(),
        })
    }
} 