use crate::{config::EmbeddingConfig, errors::{Result, VectorDbError}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use reqwest::Client;
use async_trait::async_trait;

/// 嵌入提供商trait
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;
    async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}

/// OpenAI API响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
    usage: OpenAIUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    total_tokens: usize,
}

/// OpenAI兼容的嵌入提供商
pub struct OpenAICompatibleProvider {
    client: Client,
    config: EmbeddingConfig,
}

impl OpenAICompatibleProvider {
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        
        // 添加认证头
        if let Some(api_key) = &config.api_key {
            let auth_value = if config.provider == "azure" {
                reqwest::header::HeaderValue::from_str(api_key)
                    .map_err(|e| VectorDbError::config_error(format!("无效的API密钥: {}", e)))?
            } else {
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| VectorDbError::config_error(format!("无效的API密钥: {}", e)))?
            };
            
            let header_name = if config.provider == "azure" {
                "api-key"
            } else {
                "authorization"
            };
            
            headers.insert(header_name, auth_value);
        }

        // 添加自定义头部
        for (key, value) in &config.headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| VectorDbError::config_error(format!("无效的头部名称 {}: {}", key, e)))?;
            let header_value = reqwest::header::HeaderValue::from_str(value)
                .map_err(|e| VectorDbError::config_error(format!("无效的头部值 {}: {}", value, e)))?;
            headers.insert(header_name, header_value);
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| VectorDbError::config_error(format!("创建HTTP客户端失败: {}", e)))?;

        Ok(Self { client, config })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAICompatibleProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.generate_embeddings(&[text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let endpoint = self.config.endpoint.as_ref()
            .ok_or_else(|| VectorDbError::config_error("缺少端点配置".to_string()))?;

        let mut request_body = serde_json::json!({
            "input": texts,
            "model": self.config.model
        });

        // Azure特殊处理
        if self.config.provider == "azure" {
            if let Some(api_version) = &self.config.api_version {
                let url = format!("{}?api-version={}", endpoint, api_version);
                return self.make_request(&url, &request_body).await;
            }
        }

        // 添加维度参数（如果指定）
        if let Some(dimensions) = self.config.dimension {
            request_body["dimensions"] = serde_json::Value::from(dimensions);
        }

        self.make_request(endpoint, &request_body).await
    }

    fn dimensions(&self) -> usize {
        self.config.dimension.unwrap_or(1536) // 默认OpenAI嵌入维度
    }
}

impl OpenAICompatibleProvider {
    async fn make_request(&self, url: &str, body: &serde_json::Value) -> Result<Vec<Vec<f32>>> {
        let mut retry_count = 0;
        let max_retries = self.config.retry_attempts;

        while retry_count < max_retries {
            match self.client.post(url)
                .json(body)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let embedding_response: OpenAIEmbeddingResponse = response.json().await
                            .map_err(|e| VectorDbError::embedding_error(format!("解析响应失败: {}", e)))?;

                        let embeddings: Vec<Vec<f32>> = embedding_response.data
                            .into_iter()
                            .map(|item| item.embedding)
                            .collect();

                        return Ok(embeddings);
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        if retry_count < max_retries - 1 {
                            tracing::warn!("嵌入请求失败，重试中... ({}/{}): {}", retry_count + 1, max_retries, error_text);
                            retry_count += 1;
                            tokio::time::sleep(std::time::Duration::from_millis(1000 * retry_count)).await;
                            continue;
                        } else {
                            return Err(VectorDbError::embedding_error(format!("嵌入请求失败: {}", error_text)));
                        }
                    }
                },
                Err(e) => {
                    if retry_count < max_retries - 1 {
                        tracing::warn!("网络请求失败，重试中... ({}/{}): {}", retry_count + 1, max_retries, e);
                        retry_count += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(1000 * retry_count)).await;
                        continue;
                    } else {
                        return Err(VectorDbError::embedding_error(format!("网络请求失败: {}", e)));
                    }
                }
            }
        }

        Err(VectorDbError::embedding_error("达到最大重试次数".to_string()))
    }
}

/// Mock嵌入提供商（用于测试）
pub struct MockProvider {
    dimension: usize,
}

impl MockProvider {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl EmbeddingProvider for MockProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // 生成基于文本内容的确定性向量
        let mut hash = 0u64;
        for byte in text.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }

        let mut vector = Vec::with_capacity(self.dimension);
        for i in 0..self.dimension {
            let val = ((hash.wrapping_add(i as u64)) % 1000) as f32 / 1000.0;
            vector.push(val);
        }

        // 归一化向量
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut vector {
                *val /= norm;
            }
        }

        Ok(vector)
    }

    async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.generate_embedding(text).await?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.dimension
    }
}

/// 创建嵌入提供商工厂函数
pub fn create_embedding_provider(config: &EmbeddingConfig) -> Result<Box<dyn EmbeddingProvider>> {
    match config.provider.as_str() {
        "openai" | "azure" | "ollama" | "nvidia" | "huggingface" => {
            Ok(Box::new(OpenAICompatibleProvider::new(config.clone())?))
        },
        "mock" => {
            let dimension = config.dimension.unwrap_or(1536);
            Ok(Box::new(MockProvider::new(dimension)))
        },
        _ => Err(VectorDbError::config_error(format!("不支持的嵌入提供商: {}", config.provider)))
    }
} 