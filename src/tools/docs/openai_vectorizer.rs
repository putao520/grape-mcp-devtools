use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use tracing::info;

use super::doc_traits::*;

/// OpenAI 兼容的文档向量化器
pub struct OpenAIVectorizer {
    /// HTTP 客户端
    client: Client,
    /// API 基础 URL
    api_base: String,
    /// API 密钥
    api_key: String,
    /// 嵌入模型名称
    model: String,
    /// 向量维度
    dimension: usize,
}

impl OpenAIVectorizer {
    /// 创建新的 OpenAI 文档向量化器
    pub fn new(api_key: String) -> Self {
        // 加载 .env 文件
        dotenv::dotenv().ok();
        
        let api_base = env::var("EMBEDDING_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
            
        let model = env::var("EMBEDDING_MODEL_NAME")
            .unwrap_or_else(|_| "text-embedding-ada-002".to_string());
            
        let dimension = env::var("VECTOR_DIMENSION")
            .unwrap_or_else(|_| "768".to_string())
            .parse::<usize>()
            .unwrap_or(768);

        info!("初始化 OpenAI 向量化器");
        info!("API Base: {}", api_base);
        info!("模型: {}", model);
        info!("向量维度: {}", dimension);

        Self {
            client: Client::new(),
            api_base,
            api_key,
            model,
            dimension,
        }
    }

    /// 从环境变量创建向量化器
    pub fn from_env() -> Result<Self> {
        // 加载 .env 文件
        dotenv::dotenv().ok();
        
        let api_key = env::var("EMBEDDING_API_KEY")
            .map_err(|_| anyhow::anyhow!("未找到 EMBEDDING_API_KEY 环境变量"))?;
            
        Ok(Self::new(api_key))
    }

    /// 获取向量维度
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// 向量化文本
    pub async fn vectorize(&self, text: &str) -> Result<Vec<f32>> {
        // 预处理文本：限制长度并清理
        let processed_text = self.preprocess_text(text);
        
        let request_body = json!({
            "input": processed_text,
            "model": self.model,
            "encoding_format": "float",
            "input_type": "passage"
        });

        let response = self.client
            .post(&format!("{}/embeddings", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("嵌入 API 调用失败: {} - {}", status, error_text));
        }

        let response_json: Value = response.json().await?;
        
        if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
            if let Some(first_embedding) = data.first() {
                if let Some(embedding) = first_embedding.get("embedding").and_then(|e| e.as_array()) {
                    let vector: Result<Vec<f32>, _> = embedding
                        .iter()
                        .map(|v| v.as_f64().map(|f| f as f32).ok_or_else(|| anyhow::anyhow!("无效的嵌入值")))
                        .collect();
                    return vector;
                }
            }
        }

        Err(anyhow::anyhow!("无效的 API 响应格式"))
    }

    /// 预处理文本：限制长度并清理格式
    fn preprocess_text(&self, text: &str) -> String {
        // 移除多余的空白字符
        let cleaned = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        
        // 限制长度（大约512个token，按3个字符=1个token估算）
        const MAX_CHARS: usize = 1500; // 保守估计，留一些余量
        
        if cleaned.len() <= MAX_CHARS {
            cleaned
        } else {
            // 截断并添加省略号
            let truncated = &cleaned[..MAX_CHARS];
            // 尝试在单词边界截断
            if let Some(last_space) = truncated.rfind(' ') {
                format!("{}...", &truncated[..last_space])
            } else {
                format!("{}...", truncated)
            }
        }
    }
}

#[async_trait]
impl DocumentVectorizer for OpenAIVectorizer {
    async fn vectorize(&self, content: &str) -> Result<DocumentVector> {
        let processed_content = self.preprocess_text(content);
        let embedding = self.vectorize(&processed_content).await?;
        
        Ok(embedding)
    }

    fn calculate_similarity(&self, vec1: &DocumentVector, vec2: &DocumentVector) -> f32 {
        if vec1.len() != vec2.len() {
            return 0.0;
        }

        // 计算余弦相似度
        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 == 0.0 || norm2 == 0.0 {
            0.0
        } else {
            dot_product / (norm1 * norm2)
        }
    }
}

impl Default for OpenAIVectorizer {
    fn default() -> Self {
        Self::new("".to_string())
    }
} 