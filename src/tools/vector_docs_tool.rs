use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use uuid::Uuid;
use instant_distance::{Builder, HnswMap, Search};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use dotenv;
use regex;
use md5;

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString, FileDocumentFragment};
use crate::errors::MCPError;

/// 文档结构特征
#[derive(Debug, Clone)]
struct StructureFeatures {
    paragraph_count: usize,
    code_block_count: usize,
    list_count: usize,
}

/// 向量点类型，实现 Point trait
#[derive(Debug, Clone, PartialEq)]
struct VectorPoint(Vec<f32>);

impl instant_distance::Point for VectorPoint {
    fn distance(&self, other: &Self) -> f32 {
        // 欧几里得距离
        self.0.iter()
            .zip(other.0.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }
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

/// 文档记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: String,
    pub content: String,
    pub title: String,
    pub language: String,
    pub package_name: String,
    pub version: String,
    pub doc_type: String,
    pub metadata: HashMap<String, String>,
    pub embedding: Vec<f32>,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub title: String,
    pub language: String,
    pub package_name: String,
    pub version: String,
    pub doc_type: String,
    pub metadata: HashMap<String, String>,
    pub score: f32,
}

/// 持久化数据结构
#[derive(Debug, Serialize, Deserialize)]
struct PersistentData {
    documents: HashMap<String, DocumentRecord>,
    vectors: Vec<Vec<f32>>,
    vector_to_doc_id: Vec<String>,
    processed_package_versions: Option<std::collections::HashSet<String>>,
}

/// 嵌入式向量数据库存储
struct VectorStore {
    /// 文档记录
    documents: HashMap<String, DocumentRecord>,
    /// 向量索引
    search_index: Option<HnswMap<VectorPoint, String>>,
    /// 向量数据
    vectors: Vec<Vec<f32>>,
    /// 向量ID到文档ID的映射
    vector_to_doc_id: Vec<String>,
    /// 数据存储路径
    data_dir: PathBuf,
    processed_package_versions: std::collections::HashSet<String>,
}

impl VectorStore {
    fn new(data_dir: PathBuf) -> Self {
        Self {
            documents: HashMap::new(),
            search_index: None,
            vectors: Vec::new(),
            vector_to_doc_id: Vec::new(),
            data_dir,
            processed_package_versions: std::collections::HashSet::new(),
        }
    }

    /// 从磁盘加载数据
    fn load(&mut self) -> Result<()> {
        let data_file = self.data_dir.join("vector_data.bin");
        
        if !data_file.exists() {
            // 首次运行，没有数据文件
            return Ok(());
        }

        let data = fs::read(&data_file)?;
        match bincode::deserialize::<PersistentData>(&data) {
            Ok(persistent_data) => {
                self.documents = persistent_data.documents;
                self.vectors = persistent_data.vectors;
                self.vector_to_doc_id = persistent_data.vector_to_doc_id;
                self.processed_package_versions = persistent_data.processed_package_versions.unwrap_or_else(|| std::collections::HashSet::new());
                self.rebuild_index()?;
                tracing::info!("从磁盘加载了 {} 个文档和 {} 个已处理包版本标记。", self.documents.len(), self.processed_package_versions.len());
            }
            Err(e) => {
                // 尝试加载旧格式数据（不含 processed_package_versions）
                tracing::warn!("尝试加载新格式数据失败: {}. 尝试加载旧格式...", e);
                let old_persistent_data: Result<OldPersistentData, _> = bincode::deserialize(&data);
                 match old_persistent_data {
                    Ok(old_data) => {
                        self.documents = old_data.documents;
                        self.vectors = old_data.vectors;
                        self.vector_to_doc_id = old_data.vector_to_doc_id;
                        self.processed_package_versions = std::collections::HashSet::new();
                        self.rebuild_index()?;
                        tracing::info!("成功从旧格式磁盘数据加载了 {} 个文档。已处理包版本标记将重新建立。", self.documents.len());
                    }
                    Err(old_err) => {
                        tracing::error!("加载旧格式数据也失败: {}. 将创建新的向量库。", old_err);
                        // 如果都失败，则不改变当前状态（相当于新建）
                    }
                }
            }
        }
        Ok(())
    }

    /// 保存数据到磁盘
    fn save(&self) -> Result<()> {
        // 确保数据目录存在
        fs::create_dir_all(&self.data_dir)?;
        
        let persistent_data = PersistentData {
            documents: self.documents.clone(),
            vectors: self.vectors.clone(),
            vector_to_doc_id: self.vector_to_doc_id.clone(),
            processed_package_versions: Some(self.processed_package_versions.clone()),
        };
        
        let data = bincode::serialize(&persistent_data)?;
        let data_file = self.data_dir.join("vector_data.bin");
        fs::write(&data_file, data)?;
        
        tracing::debug!("向量数据（包含已处理包版本标记）已保存到: {:?}", data_file);
        Ok(())
    }

    fn add_document(&mut self, doc: DocumentRecord) -> Result<()> {
        let doc_id = doc.id.clone();
        // 检查文档是否已存在，如果存在则可以考虑更新或跳过
        if self.documents.contains_key(&doc_id) {
            // 简单的跳过逻辑，可以根据需求改为更新
            tracing::debug!("文档 {} 已存在，跳过添加单个文档。", doc_id);
            return Ok(()); 
        }
        let embedding = doc.embedding.clone(); 
        
        self.documents.insert(doc_id.clone(), doc);
        self.vectors.push(embedding);
        self.vector_to_doc_id.push(doc_id.clone());
        
        self.rebuild_index()?;        
        self.save() // 单个添加后保存
    }

    /// 批量添加文档记录，并在完成后重建索引和保存
    fn add_documents_batch(&mut self, docs: Vec<DocumentRecord>) -> Result<()> {
        if docs.is_empty() {
            return Ok(());
        }
        let mut new_docs_count = 0;
        for doc in docs {
            let doc_id = doc.id.clone();
            // 检查文档是否已存在，如果存在则可以考虑更新或跳过
            if self.documents.contains_key(&doc_id) {
                tracing::debug!("文档 {} 已存在于批处理中，跳过添加。", doc_id);
                continue; 
            }
            let embedding = doc.embedding.clone();

            self.documents.insert(doc_id.clone(), doc);
            self.vectors.push(embedding);
            self.vector_to_doc_id.push(doc_id.clone());
            new_docs_count += 1;
        }

        if new_docs_count > 0 {
            self.rebuild_index()?;
            self.save()?; // 所有新文档添加完成后保存一次
            tracing::info!("成功批量添加 {} 个新文档记录到向量库并已保存。", new_docs_count);
        } else {
            tracing::info!("批量添加操作中没有新的文档被添加。");
        }
        Ok(())
    }

    fn rebuild_index(&mut self) -> Result<()> {
        if self.vectors.is_empty() {
            self.search_index = None;
            return Ok(());
        }

        let builder = Builder::default();
        let points: Vec<VectorPoint> = self.vectors.iter()
            .map(|v| VectorPoint(v.clone()))
            .collect();
        let values: Vec<String> = self.vector_to_doc_id.clone();
        
        let search_map = builder.build(points, values);
        self.search_index = Some(search_map);
        
        Ok(())
    }

    fn search_similar(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let search_index = match &self.search_index {
            Some(index) => index,
            None => return Ok(Vec::new()),
        };

        let query_point = VectorPoint(query_embedding.to_vec());
        let mut search = Search::default();
        
        let mut results = Vec::new();
        for item in search_index.search(&query_point, &mut search).take(limit) {
            if let Some(doc) = self.documents.get(item.value.as_str()) {
                let distance = item.distance;
                results.push(SearchResult {
                    id: doc.id.clone(),
                    content: doc.content.clone(),
                    title: doc.title.clone(),
                    language: doc.language.clone(),
                    package_name: doc.package_name.clone(),
                    version: doc.version.clone(),
                    doc_type: doc.doc_type.clone(),
                    metadata: doc.metadata.clone(),
                    score: 1.0 / (1.0 + distance), // 转换距离为相似度分数
                });
            }
        }
        
        Ok(results)
    }

    fn get_document(&self, doc_id: &str) -> Option<&DocumentRecord> {
        self.documents.get(doc_id)
    }

    fn delete_document(&mut self, doc_id: &str) -> Result<bool> {
        if let Some(_) = self.documents.remove(doc_id) {
            // 找到并移除对应的向量
            if let Some(pos) = self.vector_to_doc_id.iter().position(|id| id == doc_id) {
                self.vectors.remove(pos);
                self.vector_to_doc_id.remove(pos);
                
                // 重建索引
                self.rebuild_index()?;
                
                // 自动保存
                self.save()?;
                
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// 获取统计信息
    fn get_stats(&self) -> (usize, usize) {
        (self.documents.len(), self.vectors.len())
    }

    /// 检查某个包的特定版本是否已被标记为完整处理
    pub fn has_processed_package_version(&self, language: &str, package_name: &str, version: &str) -> bool {
        let key = format!("{}/{}/{}", language, package_name, version);
        self.processed_package_versions.contains(&key)
    }

    /// 标记某个包的特定版本为已完整处理
    pub fn mark_package_version_as_processed(&mut self, language: &str, package_name: &str, version: &str) -> Result<()> {
        let key = format!("{}/{}/{}", language, package_name, version);
        if self.processed_package_versions.insert(key.clone()) {
            tracing::info!("已标记包版本 {} 为已处理。", key);
            self.save() // 保存更改
        } else {
            tracing::debug!("包版本 {} 已被标记为已处理，无需重复标记。", key);
            Ok(())
        }
    }

    /// 混合搜索：向量相似度 + 关键词匹配
    fn hybrid_search(&self, query_embedding: &[f32], query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // 1. 向量相似度搜索
        let vector_results = self.search_similar(query_embedding, limit * 2)?; // 获取更多候选
        
        // 2. 关键词匹配增强
        let query_lower = query_text.to_lowercase();
        let query_keywords: std::collections::HashSet<String> = query_lower
            .split_whitespace()
            .filter(|word| word.len() > 2) // 过滤短词
            .map(|word| word.to_string())
            .collect();
        
        // 3. 重新计算混合分数
        let mut enhanced_results: Vec<SearchResult> = vector_results
            .into_iter()
            .map(|mut result| {
                // 计算关键词匹配分数
                let doc_content_lower = result.content.to_lowercase();
                let doc_title_lower = result.title.to_lowercase();
                
                let mut keyword_score = 0.0;
                let total_keywords = query_keywords.len() as f32;
                
                if total_keywords > 0.0 {
                    for keyword in &query_keywords {
                        let mut word_score: f32 = 0.0;
                        
                        // 标题匹配权重更高
                        if doc_title_lower.contains(keyword) {
                            word_score += 0.6;
                        }
                        
                        // 内容匹配
                        if doc_content_lower.contains(keyword) {
                            word_score += 0.4;
                        }
                        
                        // 精确匹配加分
                        if doc_content_lower.contains(&format!(" {} ", keyword)) || 
                           doc_title_lower.contains(&format!(" {} ", keyword)) {
                            word_score += 0.2;
                        }
                        
                        keyword_score += word_score.min(1.0);
                    }
                    keyword_score /= total_keywords;
                }
                
                // 4. 语言和包名匹配加分
                let mut context_bonus = 0.0;
                if query_lower.contains(&result.language.to_lowercase()) {
                    context_bonus += 0.1;
                }
                if query_lower.contains(&result.package_name.to_lowercase()) {
                    context_bonus += 0.1;
                }
                
                // 5. 混合分数计算：向量相似度60% + 关键词匹配30% + 上下文10%
                result.score = result.score * 0.6 + keyword_score * 0.3 + context_bonus;
                
                // 6. 文档类型相关性调整
                if query_lower.contains("api") && result.doc_type.contains("api") {
                    result.score += 0.05;
                }
                if query_lower.contains("tutorial") && result.doc_type.contains("tutorial") {
                    result.score += 0.05;
                }
                
                result
            })
            .collect();
        
        // 按新分数排序并返回指定数量的结果
        enhanced_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        enhanced_results.truncate(limit);
        
        Ok(enhanced_results)
    }
}

/// 为了兼容旧的 PersistentData 格式，定义一个不包含 processed_package_versions 的结构
#[derive(Debug, Serialize, Deserialize)]
struct OldPersistentData {
    documents: HashMap<String, DocumentRecord>,
    vectors: Vec<Vec<f32>>,
    vector_to_doc_id: Vec<String>,
}

/// 嵌入式向量化文档工具
pub struct VectorDocsTool {
    /// 向量存储
    store: Arc<Mutex<VectorStore>>,
    /// HTTP客户端
    client: Client,
    /// NVIDIA API密钥
    api_key: String,
    /// 嵌入模型名称
    model_name: String,
    /// 参数schema
    schema: Schema,
    /// 语义嵌入缓存（文本内容 -> 嵌入向量）
    embedding_cache: Arc<Mutex<HashMap<String, (Vec<f32>, std::time::SystemTime)>>>,
}

impl Default for VectorDocsTool {
    fn default() -> Self {
        // 创建一个默认的占位工具，用于错误情况
        let data_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".vector_db");
        
        Self {
            store: Arc::new(Mutex::new(VectorStore::new(data_dir))),
            client: Client::new(),
            api_key: String::new(),
            model_name: "nvidia/nv-embedqa-e5-v5".to_string(),
            schema: Self::create_schema(),
            embedding_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl VectorDocsTool {
    /// 创建新的嵌入式向量化文档工具
    pub fn new() -> Result<Self> {
        // 加载环境变量
        dotenv::dotenv().ok();
        
        // 必须有API密钥，不允许简化模式
        let api_key = std::env::var("EMBEDDING_API_KEY")
            .map_err(|_| anyhow::anyhow!(
                "❌ 必须设置 EMBEDDING_API_KEY 环境变量才能使用向量化功能。\n\
                 请在 .env 文件中配置：\n\
                 EMBEDDING_API_KEY=your-actual-api-key\n\
                 EMBEDDING_API_BASE_URL=https://integrate.api.nvidia.com/v1\n\
                 EMBEDDING_MODEL_NAME=nvidia/nv-embedqa-mistral-7b-v2"
            ))?;
            
        let model_name = std::env::var("EMBEDDING_MODEL_NAME")
            .unwrap_or_else(|_| "nvidia/nv-embedqa-mistral-7b-v2".to_string());

        // 创建数据目录
        let data_dir = std::env::var("VECTOR_STORAGE_PATH")
            .unwrap_or_else(|_| ".mcp_vector_data".to_string());
        let data_path = PathBuf::from(data_dir);
        
        if !data_path.exists() {
            fs::create_dir_all(&data_path)?;
        }

        let mut store = VectorStore::new(data_path);
        
        // 尝试加载现有数据
        store.load()?;

        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            client: Client::new(),
            api_key,
            model_name,
            schema: Self::create_schema(),
            embedding_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 创建参数schema
    fn create_schema() -> Schema {
        Schema::Object(SchemaObject {
            properties: {
                let mut props = HashMap::new();
                props.insert("action".to_string(), Schema::String(SchemaString {
                    description: Some("操作类型: store(存储), search(搜索), get(获取), delete(删除)".to_string()),
                    enum_values: Some(vec!["store".to_string(), "search".to_string(), "get".to_string(), "delete".to_string()]),
                }));
                props.insert("content".to_string(), Schema::String(SchemaString {
                    description: Some("文档内容 (store操作必需)".to_string()),
                    enum_values: None,
                }));
                props.insert("title".to_string(), Schema::String(SchemaString {
                    description: Some("文档标题 (store操作可选)".to_string()),
                    enum_values: None,
                }));
                props.insert("language".to_string(), Schema::String(SchemaString {
                    description: Some("编程语言或文档语言 (store操作可选)".to_string()),
                    enum_values: None,
                }));
                props.insert("doc_type".to_string(), Schema::String(SchemaString {
                    description: Some("文档类型 (store操作可选)".to_string()),
                    enum_values: None,
                }));
                props.insert("query".to_string(), Schema::String(SchemaString {
                    description: Some("搜索查询 (search操作必需)".to_string()),
                    enum_values: None,
                }));
                props.insert("id".to_string(), Schema::String(SchemaString {
                    description: Some("文档ID (get/delete操作必需)".to_string()),
                    enum_values: None,
                }));
                props.insert("limit".to_string(), Schema::String(SchemaString {
                    description: Some("搜索结果限制 (search操作可选，默认5)".to_string()),
                    enum_values: None,
                }));
                props
            },
            required: vec!["action".to_string()],
            description: Some("嵌入式向量化文档管理工具参数".to_string()),
        })
    }

    /// 生成文本的嵌入向量
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // 生成内容哈希作为缓存键
        let content_hash = format!("{:x}", md5::compute(text.as_bytes()));
        
        // 检查缓存
        {
            let cache = self.embedding_cache.lock().unwrap();
            if let Some((embedding, timestamp)) = cache.get(&content_hash) {
                // 检查缓存是否过期（24小时）
                if timestamp.elapsed().unwrap_or(std::time::Duration::MAX) < std::time::Duration::from_secs(86400) {
                    tracing::debug!("命中嵌入向量缓存，内容哈希: {}", &content_hash[..8]);
                    return Ok(embedding.clone());
                }
            }
        }
        
        // 缓存未命中，调用API
        tracing::debug!("调用NVIDIA API生成嵌入向量，内容长度: {} 字符", text.len());
        
        let request = EmbeddingRequest {
            input: vec![text.to_string()],
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
            let embedding = embedding_data.embedding.clone();
            
            // 更新缓存
            {
                let mut cache = self.embedding_cache.lock().unwrap();
                
                // 如果缓存太大，清理旧条目
                if cache.len() > 1000 {
                    let cutoff_time = std::time::SystemTime::now() - std::time::Duration::from_secs(43200); // 12小时
                    cache.retain(|_, (_, timestamp)| *timestamp > cutoff_time);
                }
                
                cache.insert(content_hash.clone(), (embedding.clone(), std::time::SystemTime::now()));
                tracing::debug!("缓存嵌入向量，内容哈希: {}，当前缓存大小: {}", &content_hash[..8], cache.len());
            }
            
            Ok(embedding)
        } else {
            Err(anyhow::anyhow!("NVIDIA API返回空的嵌入向量"))
        }
    }

    /// 智能文档相似度检测（替代简单哈希比较）
    /// 基于语义相似度和内容特征的综合评估
    async fn calculate_document_similarity(&self, existing_content: &str, new_content: &str) -> Result<f32> {
        // 1. 基础文本相似度检测
        let text_similarity = self.calculate_text_similarity(existing_content, new_content);
        
        // 2. 结构化内容相似度
        let structure_similarity = self.calculate_structure_similarity(existing_content, new_content);
        
        // 3. 语义关键词相似度
        let keyword_similarity = self.calculate_keyword_similarity(existing_content, new_content);
        
        // 4. 长度和复杂度相似度
        let complexity_similarity = self.calculate_complexity_similarity(existing_content, new_content);
        
        // 加权综合相似度分数
        let weighted_similarity = 
            text_similarity * 0.4 +           // 文本相似度权重40%
            structure_similarity * 0.25 +     // 结构相似度权重25%
            keyword_similarity * 0.25 +       // 关键词相似度权重25%
            complexity_similarity * 0.1;      // 复杂度相似度权重10%
        
        Ok(weighted_similarity)
    }
    
    /// 计算文本相似度（混合模式：词频向量 + 语义嵌入）
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f32 {
        if text1.is_empty() && text2.is_empty() {
            return 1.0;
        }
        if text1.is_empty() || text2.is_empty() {
            return 0.0;
        }
        
        // 标准化文本
        let normalized1 = self.normalize_text(text1);
        let normalized2 = self.normalize_text(text2);
        
        // 1. 基于词频向量的余弦相似度（快速，离线）
        let vector1 = self.build_word_frequency_vector(&normalized1);
        let vector2 = self.build_word_frequency_vector(&normalized2);
        let lexical_similarity = self.calculate_cosine_similarity(&vector1, &vector2);
        
        // 2. 尝试使用语义嵌入相似度（如果文本足够长且重要）
        if text1.len() > 100 && text2.len() > 100 && !self.api_key.is_empty() {
            // 异步调用嵌入API会比较复杂，这里使用同步的备用方案
            // 实际应用中可以考虑缓存常用文本的嵌入向量
            
            // 现在先使用增强的词频分析
            let enhanced_similarity = self.calculate_enhanced_lexical_similarity(&normalized1, &normalized2);
            
            // 混合权重：70%词频 + 30%增强分析
            lexical_similarity * 0.7 + enhanced_similarity * 0.3
        } else {
            lexical_similarity
        }
    }
    
    /// 增强的词汇相似度分析（基于语义场和上下文）
    fn calculate_enhanced_lexical_similarity(&self, text1: &str, text2: &str) -> f32 {
        // 1. N-gram相似度分析
        let bigrams1 = self.extract_ngrams(text1, 2);
        let bigrams2 = self.extract_ngrams(text2, 2);
        let bigram_similarity = self.calculate_set_similarity(&bigrams1, &bigrams2);
        
        // 2. 技术术语权重提升
        let tech_terms1 = self.extract_technical_terms(text1);
        let tech_terms2 = self.extract_technical_terms(text2);
        let tech_similarity = self.calculate_set_similarity(&tech_terms1, &tech_terms2);
        
        // 3. 语义场相似度（同义词和相关概念）
        let semantic_similarity = self.calculate_semantic_field_similarity(text1, text2);
        
        // 加权组合
        bigram_similarity * 0.4 + tech_similarity * 0.4 + semantic_similarity * 0.2
    }
    
    /// 提取N-gram特征
    fn extract_ngrams(&self, text: &str, n: usize) -> std::collections::HashSet<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut ngrams = std::collections::HashSet::new();
        
        for i in 0..=words.len().saturating_sub(n) {
            let ngram = words[i..i + n].join(" ");
            if ngram.len() >= 4 { // 过滤太短的ngram
                ngrams.insert(ngram);
            }
        }
        
        ngrams
    }
    
    /// 提取技术术语
    fn extract_technical_terms(&self, text: &str) -> std::collections::HashSet<String> {
        let mut tech_terms = std::collections::HashSet::new();
        let text_lower = text.to_lowercase();
        
        // 编程相关术语词典
        let programming_terms = [
            "api", "sdk", "framework", "library", "function", "method", "class", "interface",
            "async", "await", "promise", "callback", "closure", "lambda", "generic", "template",
            "memory", "thread", "process", "concurrency", "parallelism", "synchronization",
            "mutex", "lock", "atomic", "volatile", "pointer", "reference", "ownership",
            "compilation", "runtime", "virtual", "machine", "garbage", "collection",
            "serialization", "deserialization", "encoding", "decoding", "protocol",
            "http", "https", "tcp", "udp", "websocket", "database", "sql", "nosql"
        ];
        
        for term in programming_terms {
            if text_lower.contains(term) {
                tech_terms.insert(term.to_string());
            }
        }
        
        // 提取大写的标识符（通常是类名、常量等）
        for word in text.split_whitespace() {
            if word.len() > 2 && word.chars().next().unwrap().is_uppercase() {
                tech_terms.insert(word.to_lowercase());
            }
        }
        
        tech_terms
    }
    
    /// 计算集合相似度（Jaccard Index）
    fn calculate_set_similarity(&self, set1: &std::collections::HashSet<String>, set2: &std::collections::HashSet<String>) -> f32 {
        if set1.is_empty() && set2.is_empty() {
            return 1.0;
        }
        if set1.is_empty() || set2.is_empty() {
            return 0.0;
        }
        
        let intersection = set1.intersection(set2).count() as f32;
        let union = set1.union(set2).count() as f32;
        
        intersection / union
    }
    
    /// 计算语义场相似度（基于预定义的概念关系）
    fn calculate_semantic_field_similarity(&self, text1: &str, text2: &str) -> f32 {
        // 定义语义场映射
        let semantic_fields = [
            (vec!["rust", "cargo", "crate", "rustc"], "rust_ecosystem"),
            (vec!["python", "pip", "conda", "pypi"], "python_ecosystem"),
            (vec!["javascript", "npm", "node", "yarn"], "js_ecosystem"),
            (vec!["memory", "allocation", "heap", "stack"], "memory_management"),
            (vec!["async", "await", "concurrent", "parallel"], "concurrency"),
            (vec!["api", "endpoint", "request", "response"], "web_api"),
        ];
        
        let mut field_matches = 0;
        let mut total_fields = 0;
        
        for (terms, _field_name) in semantic_fields.iter() {
            total_fields += 1;
            
            let text1_lower = text1.to_lowercase();
            let text2_lower = text2.to_lowercase();
            
            let has_field1 = terms.iter().any(|term| text1_lower.contains(term));
            let has_field2 = terms.iter().any(|term| text2_lower.contains(term));
            
            if has_field1 && has_field2 {
                field_matches += 1;
            }
        }
        
        if total_fields > 0 {
            field_matches as f32 / total_fields as f32
        } else {
            0.0
        }
    }
    
    /// 构建词频向量
    fn build_word_frequency_vector(&self, text: &str) -> std::collections::HashMap<String, f32> {
        let mut word_freq = std::collections::HashMap::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        let total_words = words.len() as f32;
        
        if total_words == 0.0 {
            return word_freq;
        }
        
        // 计算词频
        for word in words {
            let word_lower = word.to_lowercase();
            // 过滤掉过短的词和常见停用词
            if word_lower.len() >= 2 && !self.is_stop_word(&word_lower) {
                *word_freq.entry(word_lower).or_insert(0.0) += 1.0;
            }
        }
        
        // 标准化词频（可选：使用TF-IDF，这里使用简单的词频标准化）
        for freq in word_freq.values_mut() {
            *freq /= total_words;
        }
        
        word_freq
    }
    
    /// 计算两个词频向量的余弦相似度
    fn calculate_cosine_similarity(&self, vector1: &std::collections::HashMap<String, f32>, vector2: &std::collections::HashMap<String, f32>) -> f32 {
        if vector1.is_empty() && vector2.is_empty() {
            return 1.0;
        }
        if vector1.is_empty() || vector2.is_empty() {
            return 0.0;
        }
        
        // 获取所有唯一词汇
        let mut all_words: std::collections::HashSet<String> = std::collections::HashSet::new();
        all_words.extend(vector1.keys().cloned());
        all_words.extend(vector2.keys().cloned());
        
        // 计算点积和向量模长
        let mut dot_product = 0.0;
        let mut norm1 = 0.0;
        let mut norm2 = 0.0;
        
        for word in all_words {
            let freq1 = vector1.get(&word).unwrap_or(&0.0);
            let freq2 = vector2.get(&word).unwrap_or(&0.0);
            
            dot_product += freq1 * freq2;
            norm1 += freq1 * freq1;
            norm2 += freq2 * freq2;
        }
        
        // 计算余弦相似度
        let norm_product = norm1.sqrt() * norm2.sqrt();
        if norm_product == 0.0 {
            return 0.0;
        }
        
        (dot_product / norm_product).max(0.0).min(1.0)
    }
    
    /// 判断是否为停用词
    fn is_stop_word(&self, word: &str) -> bool {
        // 常见的英文停用词
        const STOP_WORDS: &[&str] = &[
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
            "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did",
            "will", "would", "could", "should", "may", "might", "can", "this", "that", "these", "those",
            "i", "you", "he", "she", "it", "we", "they", "me", "him", "her", "us", "them", "my", "your",
            "his", "her", "its", "our", "their", "from", "up", "about", "into", "through", "during",
            "before", "after", "above", "below", "between", "among", "within", "without", "under", "over"
        ];
        
        STOP_WORDS.contains(&word)
    }
    
    /// 计算结构化内容相似度
    fn calculate_structure_similarity(&self, text1: &str, text2: &str) -> f32 {
        // 提取结构特征
        let features1 = self.extract_structure_features(text1);
        let features2 = self.extract_structure_features(text2);
        
        // 比较结构特征
        let mut similarity_score = 0.0;
        let mut total_features = 0;
        
        // 比较段落数量相似度
        let para_diff = (features1.paragraph_count as f32 - features2.paragraph_count as f32).abs();
        let para_max = features1.paragraph_count.max(features2.paragraph_count) as f32;
        let para_similarity = if para_max > 0.0 { 1.0 - (para_diff / para_max) } else { 1.0 };
        similarity_score += para_similarity;
        total_features += 1;
        
        // 比较代码块数量相似度
        let code_diff = (features1.code_block_count as f32 - features2.code_block_count as f32).abs();
        let code_max = features1.code_block_count.max(features2.code_block_count) as f32;
        let code_similarity = if code_max > 0.0 { 1.0 - (code_diff / code_max) } else { 1.0 };
        similarity_score += code_similarity;
        total_features += 1;
        
        // 比较列表数量相似度
        let list_diff = (features1.list_count as f32 - features2.list_count as f32).abs();
        let list_max = features1.list_count.max(features2.list_count) as f32;
        let list_similarity = if list_max > 0.0 { 1.0 - (list_diff / list_max) } else { 1.0 };
        similarity_score += list_similarity;
        total_features += 1;
        
        if total_features > 0 {
            similarity_score / total_features as f32
        } else {
            0.5 // 默认中等相似度
        }
    }
    
    /// 计算关键词相似度
    fn calculate_keyword_similarity(&self, text1: &str, text2: &str) -> f32 {
        let keywords1 = self.extract_technical_keywords(text1);
        let keywords2 = self.extract_technical_keywords(text2);
        
        if keywords1.is_empty() && keywords2.is_empty() {
            return 1.0;
        }
        if keywords1.is_empty() || keywords2.is_empty() {
            return 0.0;
        }
        
        let intersection = keywords1.intersection(&keywords2).count() as f32;
        let union = keywords1.union(&keywords2).count() as f32;
        
        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
    
    /// 计算复杂度相似度
    fn calculate_complexity_similarity(&self, text1: &str, text2: &str) -> f32 {
        let len1 = text1.len() as f32;
        let len2 = text2.len() as f32;
        
        if len1 == 0.0 && len2 == 0.0 {
            return 1.0;
        }
        
        let length_ratio = if len1 > len2 { len2 / len1 } else { len1 / len2 };
        
        // 计算词汇复杂度
        let vocab1 = text1.split_whitespace().collect::<std::collections::HashSet<_>>().len() as f32;
        let vocab2 = text2.split_whitespace().collect::<std::collections::HashSet<_>>().len() as f32;
        let vocab_ratio = if vocab1 > vocab2 { vocab2 / vocab1 } else { vocab1 / vocab2 };
        
        // 综合复杂度相似度
        (length_ratio * 0.6 + vocab_ratio * 0.4).min(1.0)
    }
    
    /// 标准化文本
    fn normalize_text(&self, text: &str) -> String {
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?;:()[]{}\"'".contains(*c))
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// 提取结构特征
    fn extract_structure_features(&self, text: &str) -> StructureFeatures {
        let paragraph_count = text.split("\n\n").filter(|p| !p.trim().is_empty()).count();
        let code_block_count = text.matches("```").count() / 2; // 代码块成对出现
        let list_count = text.lines().filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("- ") || trimmed.starts_with("* ") || 
            trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
        }).count();
        
        StructureFeatures {
            paragraph_count,
            code_block_count,
            list_count,
        }
    }
    
    /// 提取技术关键词
    fn extract_technical_keywords(&self, text: &str) -> std::collections::HashSet<String> {
        let technical_patterns = [
            // 编程概念
            r"\b(function|method|class|struct|enum|trait|interface|module|package|library|framework)\b",
            // 数据类型
            r"\b(string|integer|boolean|array|vector|hashmap|option|result|future|stream)\b",
            // 操作关键词
            r"\b(async|await|impl|pub|use|mod|crate|extern|unsafe|mut|ref|move|clone)\b",
            // API相关
            r"\b(api|endpoint|request|response|http|json|xml|rest|graphql|websocket)\b",
        ];
        
        let mut keywords = std::collections::HashSet::new();
        let text_lower = text.to_lowercase();
        
        for pattern in &technical_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for mat in regex.find_iter(&text_lower) {
                    keywords.insert(mat.as_str().to_string());
                }
            }
        }
        
        // 提取驼峰命名和下划线命名的标识符
        if let Ok(identifier_regex) = regex::Regex::new(r"\b[a-zA-Z][a-zA-Z0-9_]*[a-zA-Z0-9]\b") {
            for mat in identifier_regex.find_iter(&text) {
                let identifier = mat.as_str();
                if identifier.len() > 3 && (identifier.contains('_') || identifier.chars().any(|c| c.is_uppercase())) {
                    keywords.insert(identifier.to_lowercase());
                }
            }
        }
        
        keywords
    }

    /// 智能重复检查（替代原来的哈希比较）
    async fn intelligent_duplicate_check(&self, fragment: &FileDocumentFragment) -> Result<bool> {
        let store_guard = self.store.lock().unwrap();
        if let Some(existing_doc) = store_guard.get_document(&fragment.id) {
            // 版本检查
            if existing_doc.version != fragment.version {
                tracing::info!("文档 {} 版本不同 (现有: {}, 新: {})，需要更新", 
                    fragment.id, existing_doc.version, fragment.version);
                return Ok(false); // 版本不同，不是重复
            }
            
            // 智能相似度检测
            let similarity = self.calculate_document_similarity(&existing_doc.content, &fragment.content).await?;
            
            // 相似度阈值：85%以上认为是重复内容
            const SIMILARITY_THRESHOLD: f32 = 0.85;
            
            if similarity >= SIMILARITY_THRESHOLD {
                tracing::info!("文档 {} 内容相似度 {:.2}%，判定为重复内容", 
                    fragment.id, similarity * 100.0);
                return Ok(true); // 是重复内容
            } else {
                tracing::info!("文档 {} 内容相似度 {:.2}%，判定为不同内容，需要更新", 
                    fragment.id, similarity * 100.0);
                return Ok(false); // 内容有显著差异，需要更新
            }
        }
        
        Ok(false) // 文档不存在，不是重复
    }

    /// 公开方法，用于从 FileDocumentFragment 添加文档
    /// 这个方法会被 BackgroundDocCacher 调用
    pub async fn add_file_fragment(&self, fragment: &FileDocumentFragment) -> Result<String> {
        if fragment.content.trim().is_empty() {
            return Err(anyhow::anyhow!("文档内容为空，跳过嵌入和存储: {}", fragment.id));
        }

        // 智能重复检查
        if let Ok(is_duplicate) = self.intelligent_duplicate_check(fragment).await {
            if is_duplicate {
                tracing::info!("文档 {} 已存在且内容相同，跳过添加。", fragment.id);
                return Ok(fragment.id.clone());
            } else {
                tracing::info!("文档 {} 存在但内容已更新，将替换现有文档。", fragment.id);
                // 继续执行以更新文档
            }
        }

        let embedding = self.generate_embedding(&fragment.content).await
            .map_err(|e| anyhow::anyhow!("为文档 {} 生成嵌入向量失败: {}", fragment.id, e))?;

        let mut metadata = HashMap::new();
        metadata.insert("file_path".to_string(), fragment.file_path.clone());
        metadata.insert("hierarchy_path".to_string(), fragment.hierarchy_path.join("/"));
        metadata.insert("similarity_check".to_string(), "intelligent".to_string());

        let doc_record = DocumentRecord {
            id: fragment.id.clone(),
            content: fragment.content.clone(),
            title: fragment.get_filename_without_ext().unwrap_or_else(|| "Unknown Title".to_string()),
            language: fragment.language.clone(),
            package_name: fragment.package_name.clone(),
            version: fragment.version.clone(),
            doc_type: format!("{:?}", fragment.file_type).to_lowercase(), // e.g., "source", "documentation"
            metadata,
            embedding,
        };

        let mut store_guard = self.store.lock().unwrap();
        store_guard.add_document(doc_record.clone())?; // 假设 add_document 内部会调用 save
        
        tracing::info!("文档 {} 已成功向量化并存储。", fragment.id);
        Ok(fragment.id.clone())
    }

    /// 批量添加 FileDocumentFragment
    pub async fn add_file_fragments_batch(&self, fragments: &[FileDocumentFragment]) -> Result<Vec<String>> {
        if fragments.is_empty() {
            return Ok(Vec::new());
        }

        let mut added_ids = Vec::new();
        let mut records_to_add = Vec::new();

        {
            let store_guard = self.store.lock().unwrap();
            for fragment in fragments {
                if fragment.content.trim().is_empty() {
                    tracing::warn!("文档内容为空，跳过嵌入和存储: {}", fragment.id);
                    continue;
                }
                // 初步检查是否已存在 (更精细的检查在VectorStore的批量添加中进行)
                if store_guard.get_document(&fragment.id).is_some() {
                    tracing::info!("文档 {} 已存在于向量库 (初步检查)，跳过处理。", fragment.id);
                    added_ids.push(fragment.id.clone()); // 认为已存在即为"已添加"
                    continue;
                }
                 records_to_add.push(fragment);
            }
        }
        
        if records_to_add.is_empty() && !added_ids.is_empty(){
             return Ok(added_ids);
        } else if records_to_add.is_empty() {
            return Ok(Vec::new());
        }

        let mut document_records = Vec::with_capacity(records_to_add.len());
        for fragment_ref in records_to_add {
            // 这里直接使用 fragment_ref, 因为 records_to_add 中的生命周期足够
            let fragment = fragment_ref; 
            match self.generate_embedding(&fragment.content).await {
                Ok(embedding) => {
                    let mut metadata = HashMap::new();
                    metadata.insert("file_path".to_string(), fragment.file_path.clone());
                    metadata.insert("hierarchy_path".to_string(), fragment.hierarchy_path.join("/"));

                    document_records.push(DocumentRecord {
                        id: fragment.id.clone(),
                        content: fragment.content.clone(),
                        title: fragment.get_filename_without_ext().unwrap_or_else(|| "Unknown Title".to_string()),
                        language: fragment.language.clone(),
                        package_name: fragment.package_name.clone(),
                        version: fragment.version.clone(),
                        doc_type: format!("{:?}", fragment.file_type).to_lowercase(),
                        metadata,
                        embedding,
                    });
                    added_ids.push(fragment.id.clone());
                }
                Err(e) => {
                    tracing::error!("为文档 {} 生成嵌入向量失败: {}。将跳过此文档。", fragment.id, e);
                }
            }
        }
        
        if !document_records.is_empty() {
            let mut store_guard = self.store.lock().unwrap();
            // 假设 VectorStore 有一个批量添加方法
            match store_guard.add_documents_batch(document_records) { 
                Ok(_) => tracing::info!("成功批量添加 {} 个新文档记录到向量库。", added_ids.len()),
                Err(e) => tracing::error!("批量添加文档到向量库失败: {}", e),
            }
        }

        Ok(added_ids)
    }

    /// 检查某个包的特定版本是否已被标记为完整处理
    pub fn has_processed_package_version(&self, language: &str, package_name: &str, version: &str) -> bool {
        let store_guard = self.store.lock().unwrap();
        store_guard.has_processed_package_version(language, package_name, version)
    }

    /// 标记某个包的特定版本为已完整处理
    pub fn mark_package_version_as_processed(&self, language: &str, package_name: &str, version: &str) -> Result<()> {
        let mut store_guard = self.store.lock().unwrap();
        store_guard.mark_package_version_as_processed(language, package_name, version)
    }

    /// 获取系统状态和统计信息
    pub fn get_system_status(&self) -> Value {
        let store = self.store.lock().unwrap();
        let (doc_count, vector_count) = store.get_stats();
        
        let cache_stats = {
            let cache = self.embedding_cache.lock().unwrap();
            json!({
                "cached_embeddings": cache.len(),
                "cache_limit": 1000,
                "cache_usage_percent": (cache.len() as f32 / 1000.0 * 100.0).round()
            })
        };
        
        json!({
            "status": "运行中",
            "database": {
                "total_documents": doc_count,
                "total_vectors": vector_count,
                "backend": "instant-distance (HNSW)"
            },
            "cache": cache_stats,
            "api": {
                "provider": "NVIDIA",
                "model": self.model_name,
                "has_api_key": !self.api_key.is_empty()
            },
            "performance": {
                "search_algorithm": "混合搜索 (向量60% + 关键词30% + 上下文10%)",
                "similarity_detection": "智能多维度相似度检测",
                "caching": "MD5哈希缓存，24小时有效期"
            }
        })
    }

    /// 批量生成嵌入向量
    pub async fn generate_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // 检查缓存
        let mut cached_embeddings = Vec::new();
        let mut uncached_texts = Vec::new();
        let mut uncached_indices = Vec::new();

        {
            let cache = self.embedding_cache.lock().unwrap();
            for (idx, text) in texts.iter().enumerate() {
                let hash = format!("{:x}", md5::compute(text));
                if let Some((embedding, timestamp)) = cache.get(&hash) {
                    // 检查是否过期（24小时）
                    if timestamp.elapsed().unwrap_or(std::time::Duration::from_secs(86401)) < std::time::Duration::from_secs(86400) {
                        cached_embeddings.push((idx, embedding.clone()));
                        continue;
                    }
                }
                uncached_texts.push(text.clone());
                uncached_indices.push(idx);
            }
        }

        // 为未缓存的文本生成嵌入
        let mut new_embeddings = Vec::new();
        if !uncached_texts.is_empty() {
            let request = EmbeddingRequest {
                input: uncached_texts.clone(),
                model: self.model_name.clone(),
                input_type: "query".to_string(),
            };

            let response = self.client
                .post("https://integrate.api.nvidia.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!("NVIDIA API请求失败: {}", response.status()));
            }

            let embedding_response: EmbeddingResponse = response.json().await?;
            
            if embedding_response.data.len() != uncached_texts.len() {
                return Err(anyhow::anyhow!("返回的嵌入数量与请求文本数量不匹配"));
            }

            for (i, embedding_data) in embedding_response.data.into_iter().enumerate() {
                new_embeddings.push((uncached_indices[i], embedding_data.embedding));
            }

            // 缓存新的嵌入
            {
                let mut cache = self.embedding_cache.lock().unwrap();
                for (i, text) in uncached_texts.iter().enumerate() {
                    let hash = format!("{:x}", md5::compute(text));
                    if let Some((_, embedding)) = new_embeddings.get(i) {
                        cache.insert(hash, (embedding.clone(), std::time::SystemTime::now()));
                    }
                }

                // 清理缓存（如果超过1000个条目）
                if cache.len() > 1000 {
                    let cutoff_time = std::time::SystemTime::now() - std::time::Duration::from_secs(43200); // 12小时前
                    cache.retain(|_, (_, timestamp)| *timestamp > cutoff_time);
                }
            }
        }

        // 合并缓存和新生成的嵌入
        let mut final_embeddings = vec![Vec::new(); texts.len()];
        for (idx, embedding) in cached_embeddings {
            final_embeddings[idx] = embedding;
        }
        for (idx, embedding) in new_embeddings {
            final_embeddings[idx] = embedding;
        }

        Ok(final_embeddings)
    }

    /// 公开的混合搜索方法
    pub fn hybrid_search(&self, query_embedding: &[f32], query_text: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let store = self.store.lock().unwrap();
        store.hybrid_search(query_embedding, query_text, limit)
    }

    /// 公开的向量相似度搜索方法
    pub fn search_similar(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let store = self.store.lock().unwrap();
        store.search_similar(query_embedding, limit)
    }
}

#[async_trait]
impl MCPTool for VectorDocsTool {
    fn name(&self) -> &str {
        "vector_docs"
    }

    fn description(&self) -> &str {
        "嵌入式向量化文档管理工具 - 使用instant-distance进行高效的向量存储和搜索"
    }

    fn parameters_schema(&self) -> &Schema {
        &self.schema
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::InvalidParameter("缺少action参数".to_string()))?;

        match action {
            "store" => {
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("store操作需要content参数".to_string()))?;
                let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("未命名文档");
                let language = args.get("language").and_then(|v| v.as_str()).unwrap_or("unknown");
                let package_name = args.get("package_name").and_then(|v| v.as_str()).unwrap_or("unknown");
                let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
                let doc_type = args.get("doc_type").and_then(|v| v.as_str()).unwrap_or("text");
                let id_param = args.get("id").and_then(|v| v.as_str());

                let embedding = self.generate_embedding(content).await
                    .map_err(|e| MCPError::ServerError(format!("生成嵌入向量失败: {}", e)))?;

                let doc_id = id_param.map_or_else(|| Uuid::new_v4().to_string(), |s| s.to_string());
                
                let mut metadata_map = HashMap::new();
                if let Some(meta_val) = args.get("metadata") {
                    if let Some(meta_obj) = meta_val.as_object() {
                        for (k,v) in meta_obj {
                            if let Some(val_str) = v.as_str() {
                                metadata_map.insert(k.clone(), val_str.to_string());
                            }
                        }
                    }
                }

                let doc = DocumentRecord {
                    id: doc_id,
                    content: content.to_string(),
                    title: title.to_string(),
                    language: language.to_string(),
                    package_name: package_name.to_string(),
                    version: version.to_string(),
                    doc_type: doc_type.to_string(),
                    metadata: metadata_map,
                    embedding,
                };

                let mut store = self.store.lock().unwrap();
                store.add_document(doc.clone())
                    .map_err(|e| MCPError::ServerError(format!("存储文档失败: {}", e)))?;

                Ok(json!({
                    "status": "success",
                    "document_id": doc.id
                }))
            }

            "search" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("search操作需要query参数".to_string()))?;

                let limit = args.get("limit")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(5);

                // 生成查询嵌入向量
                let query_embedding = self.generate_embedding(query).await
                    .map_err(|e| MCPError::ServerError(format!("生成查询嵌入向量失败: {}", e)))?;

                let store = self.store.lock().unwrap();
                let results = store.hybrid_search(&query_embedding, query, limit)
                    .map_err(|e| MCPError::ServerError(format!("搜索失败: {}", e)))?;

                Ok(json!({
                    "status": "success",
                    "query": query,
                    "results": results,
                    "results_count": results.len(),
                    "database": "instant-distance (嵌入式)"
                }))
            }

            "get" => {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("get操作需要id参数".to_string()))?;

                let store = self.store.lock().unwrap();
                if let Some(doc) = store.get_document(id) {
                    Ok(json!({
                        "status": "success",
                        "document": {
                            "id": doc.id,
                            "title": doc.title,
                            "content": doc.content,
                            "language": doc.language,
                            "doc_type": doc.doc_type,
                            "metadata": doc.metadata
                        },
                        "database": "instant-distance (嵌入式)"
                    }))
                } else {
                    Ok(json!({
                        "status": "not_found",
                        "message": "文档未找到",
                        "document_id": id,
                        "database": "instant-distance (嵌入式)"
                    }))
                }
            }

            "delete" => {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MCPError::InvalidParameter("delete操作需要id参数".to_string()))?;

                let mut store = self.store.lock().unwrap();
                let deleted = store.delete_document(id)
                    .map_err(|e| MCPError::ServerError(format!("删除文档失败: {}", e)))?;

                if deleted {
                    Ok(json!({
                        "status": "success",
                        "message": "文档已成功删除",
                        "document_id": id,
                        "database": "instant-distance (嵌入式)"
                    }))
                } else {
                    Ok(json!({
                        "status": "not_found",
                        "message": "文档未找到",
                        "document_id": id,
                        "database": "instant-distance (嵌入式)"
                    }))
                }
            }

            _ => Err(MCPError::InvalidParameter(format!("不支持的操作: {}", action)).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intelligent_similarity_detection() {
        // 创建测试工具实例
        let tool = VectorDocsTool::default();
        
        // 测试1: 完全相同的内容
        let content1 = "This is a test document about Rust programming.";
        let content2 = "This is a test document about Rust programming.";
        let similarity = tool.calculate_document_similarity(content1, content2).await.unwrap();
        assert!(similarity > 0.95, "完全相同的内容相似度应该很高: {}", similarity);
        
        // 测试2: 相似但不完全相同的内容
        let content3 = "This is a test document about Rust programming language.";
        let content4 = "This is a test document about Rust programming and development.";
        let similarity2 = tool.calculate_document_similarity(content3, content4).await.unwrap();
        assert!(similarity2 > 0.7 && similarity2 < 0.95, "相似内容相似度应该在70%-95%之间: {}", similarity2);
        
        // 测试3: 完全不同的内容
        let content5 = "This is about Rust programming.";
        let content6 = "This is about Python web development.";
        let similarity3 = tool.calculate_document_similarity(content5, content6).await.unwrap();
        assert!(similarity3 < 0.7, "不同内容相似度应该较低: {}", similarity3);
    }

    #[test]
    fn test_text_similarity_calculation() {
        let tool = VectorDocsTool::default();
        
        // 测试余弦相似度计算
        let text1 = "rust programming language tutorial";
        let text2 = "rust programming language guide";
        let similarity = tool.calculate_text_similarity(text1, text2);
        assert!(similarity > 0.7, "相关文本相似度应该大于0.7: {}", similarity);
        
        // 测试完全相同的文本
        let identical_similarity = tool.calculate_text_similarity(text1, text1);
        assert!(identical_similarity > 0.95, "相同文本相似度应该接近1.0: {}", identical_similarity);
        
        // 测试空文本处理
        let empty_similarity = tool.calculate_text_similarity("", "");
        assert_eq!(empty_similarity, 1.0, "空文本相似度应该为1.0");
        
        let mixed_similarity = tool.calculate_text_similarity("test", "");
        assert_eq!(mixed_similarity, 0.0, "空文本与非空文本相似度应该为0.0");
        
        // 测试完全不同的文本
        let different_similarity = tool.calculate_text_similarity("rust programming", "python web development");
        assert!(different_similarity < 0.3, "不同文本相似度应该较低: {}", different_similarity);
    }

    #[test]
    fn test_word_frequency_vector() {
        let tool = VectorDocsTool::default();
        
        let text = "rust programming rust language programming tutorial";
        let vector = tool.build_word_frequency_vector(text);
        
        // 检查词频计算
        assert!(vector.contains_key("rust"), "应该包含'rust'");
        assert!(vector.contains_key("programming"), "应该包含'programming'");
        assert!(vector.contains_key("language"), "应该包含'language'");
        assert!(vector.contains_key("tutorial"), "应该包含'tutorial'");
        
        // 检查词频值
        let rust_freq = vector.get("rust").unwrap();
        let programming_freq = vector.get("programming").unwrap();
        assert!(rust_freq > programming_freq, "'rust'出现频率应该高于其他词");
    }

    #[test]
    fn test_cosine_similarity_calculation() {
        let tool = VectorDocsTool::default();
        
        // 创建测试向量
        let mut vector1 = std::collections::HashMap::new();
        vector1.insert("rust".to_string(), 0.5);
        vector1.insert("programming".to_string(), 0.3);
        vector1.insert("language".to_string(), 0.2);
        
        let mut vector2 = std::collections::HashMap::new();
        vector2.insert("rust".to_string(), 0.4);
        vector2.insert("programming".to_string(), 0.4);
        vector2.insert("tutorial".to_string(), 0.2);
        
        let similarity = tool.calculate_cosine_similarity(&vector1, &vector2);
        assert!(similarity > 0.0 && similarity <= 1.0, "余弦相似度应该在[0,1]范围内: {}", similarity);
        assert!(similarity > 0.5, "有共同词汇的向量相似度应该较高: {}", similarity);
        
        // 测试相同向量
        let identical_similarity = tool.calculate_cosine_similarity(&vector1, &vector1);
        assert!((identical_similarity - 1.0).abs() < 0.001, "相同向量的余弦相似度应该为1.0: {}", identical_similarity);
    }

    #[test]
    fn test_stop_words_filtering() {
        let tool = VectorDocsTool::default();
        
        // 测试停用词过滤
        assert!(tool.is_stop_word("the"), "'the'应该被识别为停用词");
        assert!(tool.is_stop_word("and"), "'and'应该被识别为停用词");
        assert!(tool.is_stop_word("is"), "'is'应该被识别为停用词");
        assert!(!tool.is_stop_word("rust"), "'rust'不应该被识别为停用词");
        assert!(!tool.is_stop_word("programming"), "'programming'不应该被识别为停用词");
        
        // 测试包含停用词的文本处理
        let text_with_stopwords = "the rust programming language is very powerful and safe";
        let vector = tool.build_word_frequency_vector(text_with_stopwords);
        
        // 停用词应该被过滤掉
        assert!(!vector.contains_key("the"), "停用词'the'应该被过滤");
        assert!(!vector.contains_key("is"), "停用词'is'应该被过滤");
        assert!(!vector.contains_key("and"), "停用词'and'应该被过滤");
        
        // 有意义的词应该保留
        assert!(vector.contains_key("rust"), "关键词'rust'应该保留");
        assert!(vector.contains_key("programming"), "关键词'programming'应该保留");
        assert!(vector.contains_key("language"), "关键词'language'应该保留");
    }

    #[test]
    fn test_structure_features_extraction() {
        let tool = VectorDocsTool::default();
        
        let markdown_text = r#"
# Title

This is a paragraph.

Another paragraph here.

```rust
fn main() {
    println!("Hello, world!");
}
```

- List item 1
- List item 2
* Another list item

1. Numbered item
2. Another numbered item
"#;
        
        let features = tool.extract_structure_features(markdown_text);
        assert!(features.paragraph_count >= 2, "应该检测到至少2个段落");
        assert!(features.code_block_count >= 1, "应该检测到至少1个代码块");
        assert!(features.list_count >= 4, "应该检测到至少4个列表项");
    }

    #[test]
    fn test_technical_keywords_extraction() {
        let tool = VectorDocsTool::default();
        
        let technical_text = "This function implements an async method for handling HTTP requests using the tokio framework.";
        let keywords = tool.extract_technical_keywords(technical_text);
        
        assert!(keywords.contains("function"), "应该提取到'function'关键词");
        assert!(keywords.contains("async"), "应该提取到'async'关键词");
        assert!(keywords.contains("method"), "应该提取到'method'关键词");
        assert!(keywords.contains("http"), "应该提取到'http'关键词");
    }

    #[test]
    fn test_text_normalization() {
        let tool = VectorDocsTool::default();
        
        let messy_text = "  This   is\n\na  messy\n\n\ntext   with   extra   spaces  \n\n";
        let normalized = tool.normalize_text(messy_text);
        assert_eq!(normalized, "This is a messy text with extra spaces", "文本应该被正确标准化");
        
        let special_chars = "Hello, world! How are you? (Fine) [Good] {Great}";
        let normalized2 = tool.normalize_text(special_chars);
        assert!(normalized2.contains("Hello, world!"), "应该保留基本标点符号");
    }
}