use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use regex::Regex;
use async_openai::{Client, config::OpenAIConfig};
use async_openai::types::{CreateEmbeddingRequest, EmbeddingInput};

use crate::tools::base::{
    FileDocumentFragment, DocumentVector, FileVectorMetadata, FileVectorizer,
};

/// 简化的嵌入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub api_base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub dimensions: Option<usize>,
    pub timeout_secs: u64,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            api_base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            api_key: std::env::var("EMBEDDING_API_KEY")
                .unwrap_or_else(|_| "nvapi-demo-key".to_string()),
            model_name: "nvidia/nv-embedcode-7b-v1".to_string(),
            dimensions: Some(768),
            timeout_secs: 30,
        }
    }
}

impl EmbeddingConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_base_url: std::env::var("EMBEDDING_API_BASE_URL")
                .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".to_string()),
            api_key: std::env::var("EMBEDDING_API_KEY")
                .map_err(|_| anyhow!("EMBEDDING_API_KEY 环境变量未设置"))?,
            model_name: std::env::var("EMBEDDING_MODEL_NAME")
                .unwrap_or_else(|_| "nvidia/nv-embedcode-7b-v1".to_string()),
            dimensions: std::env::var("EMBEDDING_DIMENSIONS")
                .ok()
                .and_then(|s| s.parse().ok()),
            timeout_secs: std::env::var("EMBEDDING_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        })
    }
}

/// 文件级向量化器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizationConfig {
    /// 向量维度
    pub vector_dimension: usize,
    /// 最大文件大小（字节，超过则分块）
    pub max_file_size: usize,
    /// 分块大小
    pub chunk_size: usize,
    /// 分块重叠
    pub chunk_overlap: usize,
    /// 最大并发文件数
    pub max_concurrent_files: usize,
    /// 请求超时时间
    pub timeout_secs: u64,
}

impl Default for VectorizationConfig {
    fn default() -> Self {
        Self {
            vector_dimension: 768,
            max_file_size: 1048576,  // 1MB
            chunk_size: 8192,       // 8KB
            chunk_overlap: 512,     // 512字节
            max_concurrent_files: 10,
            timeout_secs: 30,
        }
    }
}

impl VectorizationConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            vector_dimension: std::env::var("VECTOR_DIMENSION")
                .unwrap_or_else(|_| "768".to_string())
                .parse()
                .unwrap_or(768),
            max_file_size: std::env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "1048576".to_string())
                .parse()
                .unwrap_or(1048576),
            chunk_size: std::env::var("CHUNK_SIZE")
                .unwrap_or_else(|_| "8192".to_string())
                .parse()
                .unwrap_or(8192),
            chunk_overlap: std::env::var("CHUNK_OVERLAP")
                .unwrap_or_else(|_| "512".to_string())
                .parse()
                .unwrap_or(512),
            max_concurrent_files: std::env::var("MAX_CONCURRENT_FILES")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            timeout_secs: std::env::var("VECTORIZATION_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        })
    }
}

/// 文件级向量化器实现 - 直接使用 async-openai
pub struct FileVectorizerImpl {
    /// async-openai 客户端，支持自定义端点
    client: Client<OpenAIConfig>,
    embedding_config: EmbeddingConfig,
    config: VectorizationConfig,
}

impl FileVectorizerImpl {
    /// 创建新的文件向量化器
    pub async fn new(embedding_config: EmbeddingConfig, vectorization_config: VectorizationConfig) -> Result<Self> {
        // 直接使用 async-openai 配置，支持自定义端点
        let openai_config = OpenAIConfig::new()
            .with_api_key(&embedding_config.api_key)
            .with_api_base(&embedding_config.api_base_url);
            
        let client = Client::with_config(openai_config);
        
        Ok(Self {
            client,
            embedding_config,
            config: vectorization_config,
        })
    }
    
    /// 从环境变量创建
    pub async fn from_env() -> Result<Self> {
        let embedding_config = EmbeddingConfig::from_env()?;
        let vectorization_config = VectorizationConfig::from_env()?;
        
        Self::new(embedding_config, vectorization_config).await
    }
    
    /// 构建用于向量化的文本
    fn build_vectorization_text(&self, fragment: &FileDocumentFragment) -> Result<String> {
        let text = format!(
            "Package: {}\nVersion: {}\nLanguage: {}\nFile: {}\n\n{}",
            fragment.package_name,
            fragment.version,
            fragment.language,
            fragment.file_path,
            fragment.content
        );
        
        Ok(text)
    }
    
    /// 大文件分块策略
    fn chunk_large_file(&self, content: &str, fragment: &FileDocumentFragment) -> Result<Vec<String>> {
        let chunk_size = self.config.chunk_size;
        let overlap = self.config.chunk_overlap;
        
        let mut chunks = Vec::new();
        let mut start = 0;
        
        while start < content.len() {
            let end = std::cmp::min(start + chunk_size, content.len());
            let chunk = &content[start..end];
            
            // 为每个分块添加上下文信息
            let chunk_with_context = format!(
                "Package: {} | File: {} | Chunk: {}\n\n{}",
                fragment.package_name,
                fragment.file_path,
                chunks.len() + 1,
                chunk
            );
            
            chunks.push(chunk_with_context);
            
            if end >= content.len() {
                break;
            }
            
            // 处理重叠
            start = end - overlap;
        }
        
        Ok(chunks)
    }
    
    /// 合并多个分块的向量
    fn merge_chunk_vectors(&self, vectors: Vec<Vec<f32>>) -> Result<Vec<f32>> {
        if vectors.is_empty() {
            return Err(anyhow!("无法合并空向量列表"));
        }
        
        let dimension = vectors[0].len();
        let mut merged = vec![0.0; dimension];
        
        // 简单平均合并
        for vector in &vectors {
            for (i, &value) in vector.iter().enumerate() {
                merged[i] += value;
            }
        }
        
        // 归一化
        let count = vectors.len() as f32;
        for value in &mut merged {
            *value /= count;
        }
        
        Ok(merged)
    }
    
    /// 从文件内容中提取关键词
    fn extract_keywords(&self, fragment: &FileDocumentFragment) -> Vec<String> {
        let mut keywords = Vec::new();
        
        // 添加文件路径部分作为关键词
        keywords.extend(fragment.hierarchy_path.iter().cloned());
        
        // 从文件名提取
        if let Some(filename) = fragment.file_path.split('/').last() {
            if let Some(name_without_ext) = filename.split('.').next() {
                keywords.push(name_without_ext.to_string());
            }
        }
        
        // 简单的代码关键词提取
        match fragment.language.as_str() {
            "go" => keywords.extend(self.extract_go_keywords(&fragment.content)),
            "rust" => keywords.extend(self.extract_rust_keywords(&fragment.content)),
            "python" => keywords.extend(self.extract_python_keywords(&fragment.content)),
            "javascript" | "typescript" => keywords.extend(self.extract_js_keywords(&fragment.content)),
            _ => {}
        }
        
        // 去重并限制数量
        keywords.sort();
        keywords.dedup();
        keywords.truncate(20);
        
        keywords
    }
    
    /// 提取Go语言关键词
    fn extract_go_keywords(&self, content: &str) -> Vec<String> {
        let mut keywords = Vec::new();
        
        // 创建正则表达式
        let func_re = Regex::new(r"func\s+(\w+)").unwrap();
        let type_re = Regex::new(r"type\s+(\w+)").unwrap();
        let interface_re = Regex::new(r"type\s+(\w+)\s+interface").unwrap();
        
        // 提取函数名
        for cap in func_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        // 提取类型名
        for cap in type_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        // 提取接口名
        for cap in interface_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        keywords
    }
    
    /// 提取Rust关键词
    fn extract_rust_keywords(&self, content: &str) -> Vec<String> {
        let mut keywords = Vec::new();
        
        let fn_re = Regex::new(r"\bfn\s+(\w+)").unwrap();
        let struct_re = Regex::new(r"\bstruct\s+(\w+)").unwrap();
        let enum_re = Regex::new(r"\benum\s+(\w+)").unwrap();
        let trait_re = Regex::new(r"\btrait\s+(\w+)").unwrap();
        
        for cap in fn_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        for cap in struct_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        for cap in enum_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        for cap in trait_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        keywords
    }
    
    /// 提取Python关键词
    fn extract_python_keywords(&self, content: &str) -> Vec<String> {
        let mut keywords = Vec::new();
        
        let def_re = Regex::new(r"\bdef\s+(\w+)").unwrap();
        let class_re = Regex::new(r"\bclass\s+(\w+)").unwrap();
        
        for cap in def_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        for cap in class_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        keywords
    }
    
    /// 提取JavaScript/TypeScript关键词
    fn extract_js_keywords(&self, content: &str) -> Vec<String> {
        let mut keywords = Vec::new();
        
        let function_re = Regex::new(r"\bfunction\s+(\w+)").unwrap();
        let class_re = Regex::new(r"\bclass\s+(\w+)").unwrap();
        let const_re = Regex::new(r"\bconst\s+(\w+)").unwrap();
        let interface_re = Regex::new(r"\binterface\s+(\w+)").unwrap();
        
        for cap in function_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        for cap in class_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        for cap in const_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        for cap in interface_re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                keywords.push(name.as_str().to_string());
            }
        }
        
        keywords
    }
    
    /// 调用 async-openai 的 embeddings API
    async fn create_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let request = CreateEmbeddingRequest {
            model: self.embedding_config.model_name.clone(),
            input: EmbeddingInput::StringArray(texts.to_vec()),
            encoding_format: Some(async_openai::types::EncodingFormat::Float),
            dimensions: self.embedding_config.dimensions.map(|d| d as u32),
            user: None,
        };
        
        let timeout_duration = Duration::from_secs(self.embedding_config.timeout_secs);
        
        let response = timeout(timeout_duration, self.client.embeddings().create(request))
            .await
            .map_err(|_| anyhow!("嵌入API请求超时"))?
            .map_err(|e| anyhow!("嵌入API请求失败: {}", e))?;
        
        let embeddings: Vec<Vec<f32>> = response
            .data
            .into_iter()
            .map(|embedding| embedding.embedding)
            .collect();
        
        if embeddings.len() != texts.len() {
            return Err(anyhow!(
                "嵌入向量数量不匹配：期望 {}，获得 {}",
                texts.len(),
                embeddings.len()
            ));
        }
        
        Ok(embeddings)
    }
}

#[async_trait]
impl FileVectorizer for FileVectorizerImpl {
    /// 向量化单个文件
    async fn vectorize_file(&self, fragment: &FileDocumentFragment) -> Result<DocumentVector> {
        // 1. 构建向量化文本
        let vectorization_text = self.build_vectorization_text(fragment)?;
        
        // 2. 文件分块（如果需要）
        let chunks = if vectorization_text.len() > self.config.max_file_size {
            self.chunk_large_file(&vectorization_text, fragment)?
        } else {
            vec![vectorization_text]
        };
        
        // 3. 调用向量化API
        let embeddings = self.create_embeddings(&chunks).await?;
        
        // 4. 合并向量（如果有多个分块）
        let final_vector = if embeddings.len() == 1 {
            embeddings.into_iter().next().unwrap()
        } else {
            self.merge_chunk_vectors(embeddings)?
        };
        
        // 5. 提取关键词
        let keywords = self.extract_keywords(fragment);
        
        // 6. 构建最终向量对象
        Ok(DocumentVector {
            data: final_vector.clone(),
            dimension: final_vector.len(),
            metadata: FileVectorMetadata::from_fragment(fragment, keywords),
        })
    }
    
    /// 批量向量化多个文件
    async fn vectorize_files_batch(&self, fragments: &[FileDocumentFragment]) -> Result<Vec<DocumentVector>> {
        // 构建所有向量化文本
        let texts: Vec<String> = fragments
            .iter()
            .map(|f| self.build_vectorization_text(f))
            .collect::<Result<Vec<_>>>()?;
            
        // 批量调用embedding API
        let embeddings = self.create_embeddings(&texts).await?;
        
        // 构建向量对象
        let mut vectors = Vec::new();
        for (fragment, embedding) in fragments.iter().zip(embeddings.iter()) {
            let keywords = self.extract_keywords(fragment);
            vectors.push(DocumentVector {
                data: embedding.clone(),
                dimension: embedding.len(),
                metadata: FileVectorMetadata::from_fragment(fragment, keywords),
            });
        }
        
        Ok(vectors)
    }
    
    /// 向量化查询文本
    async fn vectorize_query(&self, query: &str) -> Result<Vec<f32>> {
        let embeddings = self.create_embeddings(&[query.to_string()]).await?;
        embeddings.into_iter().next()
            .ok_or_else(|| anyhow!("未获取到查询向量"))
    }
} 