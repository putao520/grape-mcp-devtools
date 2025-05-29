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

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;

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
}

impl VectorStore {
    fn new(data_dir: PathBuf) -> Self {
        Self {
            documents: HashMap::new(),
            search_index: None,
            vectors: Vec::new(),
            vector_to_doc_id: Vec::new(),
            data_dir,
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
        let persistent_data: PersistentData = bincode::deserialize(&data)?;
        
        self.documents = persistent_data.documents;
        self.vectors = persistent_data.vectors;
        self.vector_to_doc_id = persistent_data.vector_to_doc_id;
        
        // 重建索引
        self.rebuild_index()?;
        
        tracing::info!("从磁盘加载了 {} 个文档", self.documents.len());
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
        };
        
        let data = bincode::serialize(&persistent_data)?;
        let data_file = self.data_dir.join("vector_data.bin");
        fs::write(&data_file, data)?;
        
        tracing::debug!("向量数据已保存到: {:?}", data_file);
        Ok(())
    }

    fn add_document(&mut self, doc: DocumentRecord) -> Result<()> {
        let doc_id = doc.id.clone();
        let embedding = doc.embedding.clone();
        
        // 存储文档
        self.documents.insert(doc_id.clone(), doc);
        
        // 添加向量
        self.vectors.push(embedding);
        self.vector_to_doc_id.push(doc_id.clone());
        
        // 重建索引
        self.rebuild_index()?;
        
        // 自动保存
        self.save()?;
        
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
        }
    }
}

impl VectorDocsTool {
    /// 创建新的嵌入式向量化文档工具
    pub fn new() -> Result<Self> {
        // 加载环境变量
        dotenv::dotenv().ok();
        
        let api_key = std::env::var("EMBEDDING_API_KEY")
            .map_err(|_| anyhow::anyhow!("未设置 EMBEDDING_API_KEY 环境变量"))?;
            
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
        store.load()?;

        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            client: Client::new(),
            api_key,
            model_name,
            schema: Self::create_schema(),
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

    /// 生成文本嵌入向量
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            input: vec![text.to_string()],
            model: self.model_name.clone(),
            input_type: "passage".to_string(),
        };

        let response = self.client
            .post(&format!("{}/embeddings", 
                std::env::var("EMBEDDING_API_BASE_URL")
                    .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".to_string())))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("NVIDIA API错误: {}", error_text));
        }

        let embedding_response: EmbeddingResponse = response.json().await?;
        
        embedding_response.data
            .into_iter()
            .next()
            .map(|data| data.embedding)
            .ok_or_else(|| anyhow::anyhow!("未收到嵌入向量"))
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

                let title = args.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未命名文档");

                let language = args.get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let doc_type = args.get("doc_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("text");

                // 生成嵌入向量
                let embedding = self.generate_embedding(content).await
                    .map_err(|e| MCPError::ServerError(format!("生成嵌入向量失败: {}", e)))?;

                let doc = DocumentRecord {
                    id: Uuid::new_v4().to_string(),
                    content: content.to_string(),
                    title: title.to_string(),
                    language: language.to_string(),
                    doc_type: doc_type.to_string(),
                    metadata: HashMap::new(),
                    embedding,
                };

                let mut store = self.store.lock().unwrap();
                store.add_document(doc.clone())
                    .map_err(|e| MCPError::ServerError(format!("存储文档失败: {}", e)))?;

                Ok(json!({
                    "status": "success",
                    "message": "文档已成功存储到嵌入式向量数据库",
                    "document_id": doc.id,
                    "title": doc.title,
                    "language": doc.language,
                    "doc_type": doc.doc_type,
                    "database": "instant-distance"
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
                let results = store.search_similar(&query_embedding, limit)
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