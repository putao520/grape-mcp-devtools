use std::collections::HashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tracing::{debug, info, warn, error};
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankRequest {
    pub model: String,
    pub query: RerankQuery,
    pub passages: Vec<RerankPassage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankQuery {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankPassage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub results: Vec<RerankResult>,
    pub meta: Option<RerankMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f64,
    pub document: Option<RerankDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankDocument {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankMeta {
    pub api_version: Option<String>,
    pub billed_units: Option<HashMap<String, i32>>,
}

#[derive(Debug, Clone)]
pub struct RerankerConfig {
    pub api_key: String,
    pub api_base_url: String,
    pub model_name: String,
    pub max_passages: usize,
    pub score_threshold: f64,
    pub timeout_seconds: u64,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("RERANK_API_KEY").unwrap_or_default(),
            api_base_url: std::env::var("RERANK_API_BASE_URL")
                .unwrap_or_else(|_| "https://ai.api.nvidia.com/v1/retrieval".to_string()),
            model_name: "nvidia/nv-rerankqa-mistral-4b-v3".to_string(),
            max_passages: 10,
            score_threshold: 0.5,
            timeout_seconds: 30,
        }
    }
}

pub struct DocumentReranker {
    client: Client,
    config: RerankerConfig,
}

impl DocumentReranker {
    pub fn new(config: RerankerConfig) -> Self {
        let client = Client::new();
        Self { client, config }
    }

    pub fn from_env() -> Result<Self> {
        let config = RerankerConfig::default();
        
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("RERANK_API_KEY environment variable is required"));
        }
        
        info!("🔄 初始化文档重排器: {}", config.model_name);
        Ok(Self::new(config))
    }

    /// 对检索到的文档片段进行重排
    pub async fn rerank_documents(
        &self,
        query: &str,
        documents: Vec<String>,
        top_k: Option<usize>,
    ) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let effective_top_k = top_k.unwrap_or(self.config.max_passages.min(documents.len()));
        
        info!("🔍 开始重排 {} 个文档，查询: {}", documents.len(), query);
        debug!("重排参数: top_k={}, score_threshold={}", effective_top_k, self.config.score_threshold);

        // 准备请求
        let rerank_request = RerankRequest {
            query: RerankQuery { text: query.to_string() },
            passages: documents.into_iter().map(|text| RerankPassage { text }).collect(),
            model: self.config.model_name.clone(),
        };

        // 发送重排请求
        let response = self.send_rerank_request(&rerank_request).await?;
        
        // 过滤低分结果
        let filtered_results: Vec<RerankResult> = response.results
            .into_iter()
            .filter(|result| result.relevance_score >= self.config.score_threshold)
            .collect();

        info!("✅ 重排完成: {} -> {} 个高质量结果", documents.len(), filtered_results.len());
        
        // 打印重排结果概览
        for (i, result) in filtered_results.iter().enumerate().take(3) {
            debug!("排名 {}: 评分 {:.3}, 索引 {}", i + 1, result.relevance_score, result.index);
        }

        Ok(filtered_results)
    }

    async fn send_rerank_request(&self, request: &RerankRequest) -> Result<RerankResponse> {
        let url = format!("{}/{}/reranking", self.config.api_base_url, self.config.model_name);

        debug!("发送重排请求到: {}", url);
        
        let request_future = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(request)
            .send();

        let response = timeout(Duration::from_secs(self.config.timeout_seconds), request_future)
            .await
            .map_err(|_| anyhow::anyhow!("重排请求超时"))?
            .map_err(|e| anyhow::anyhow!("重排请求失败: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("重排API错误 {}: {}", status, error_text));
        }

        let rerank_response: RerankResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("解析重排响应失败: {}", e))?;

        debug!("收到重排响应: {} 个结果", rerank_response.results.len());
        Ok(rerank_response)
    }

    /// 获取最佳匹配的文档
    pub async fn get_best_match(
        &self,
        query: &str,
        documents: Vec<String>,
    ) -> Result<Option<String>> {
        let results = self.rerank_documents(query, documents, Some(1)).await?;
        
        if let Some(best_result) = results.first() {
            if let Some(doc) = &best_result.document {
                info!("🎯 选择最佳匹配文档 (评分: {:.3})", best_result.relevance_score);
                return Ok(Some(doc.text.clone()));
            }
        }
        
        warn!("⚠️ 未找到符合条件的匹配文档");
        Ok(None)
    }

    /// 批量重排多个查询
    pub async fn batch_rerank(
        &self,
        queries_and_docs: Vec<(String, Vec<String>)>,
        top_k: Option<usize>,
    ) -> Result<Vec<Vec<RerankResult>>> {
        let mut results = Vec::new();
        
        for (query, documents) in queries_and_docs {
            let reranked = self.rerank_documents(&query, documents, top_k).await?;
            results.push(reranked);
        }
        
        Ok(results)
    }

    /// 获取重排器配置信息
    pub fn get_config_info(&self) -> HashMap<String, serde_json::Value> {
        let mut info = HashMap::new();
        
        info.insert("model_name".to_string(), 
                   serde_json::Value::String(self.config.model_name.clone()));
        info.insert("max_passages".to_string(), 
                   serde_json::Value::Number(self.config.max_passages.into()));
        info.insert("score_threshold".to_string(), 
                   serde_json::Value::Number(serde_json::Number::from_f64(self.config.score_threshold).unwrap()));
        info.insert("timeout_seconds".to_string(), 
                   serde_json::Value::Number(self.config.timeout_seconds.into()));
        
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reranker_creation() {
        let config = RerankerConfig {
            api_key: "test_key".to_string(),
            api_base_url: "https://test.api.com".to_string(),
            model_name: "test-model".to_string(),
            max_passages: 5,
            score_threshold: 0.7,
            timeout_seconds: 10,
        };
        
        let reranker = DocumentReranker::new(config);
        assert_eq!(reranker.config.model_name, "test-model");
        assert_eq!(reranker.config.max_passages, 5);
    }
    
    #[test]
    fn test_rerank_request_serialization() {
        let request = RerankRequest {
            query: RerankQuery { text: "test query".to_string() },
            passages: vec![RerankPassage { text: "doc1".to_string() }, RerankPassage { text: "doc2".to_string() }],
            model: "test-model".to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("doc1"));
    }

    #[test]
    fn test_rerank_request_matches_nvidia_api() {
        let request = RerankRequest {
            model: "nvidia/nv-rerankqa-mistral-4b-v3".to_string(),
            query: RerankQuery {
                text: "What is the GPU memory bandwidth of H100 SXM?".to_string(),
            },
            passages: vec![
                RerankPassage {
                    text: "The Hopper GPU is paired with the Grace CPU using NVIDIA's ultra-fast chip-to-chip interconnect, delivering 900GB/s of bandwidth, 7X faster than PCIe Gen5.".to_string(),
                },
                RerankPassage {
                    text: "A100 provides up to 20X higher performance over the prior generation and can be partitioned into seven GPU instances to dynamically adjust to shifting demands.".to_string(),
                },
                RerankPassage {
                    text: "Accelerated servers with H100 deliver the compute power—along with 3 terabytes per second (TB/s) of memory bandwidth per GPU and scalability with NVLink and NVSwitch™.".to_string(),
                },
            ],
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        // 验证顶层结构
        assert!(parsed.get("model").is_some());
        assert!(parsed.get("query").is_some());
        assert!(parsed.get("passages").is_some());
        
        // 验证query结构
        let query = parsed.get("query").unwrap();
        assert!(query.get("text").is_some());
        
        // 验证passages结构
        let passages = parsed.get("passages").unwrap().as_array().unwrap();
        assert_eq!(passages.len(), 3);
        for passage in passages {
            assert!(passage.get("text").is_some());
        }
        
        println!("✅ Rust格式与NVIDIA API示例完全匹配");
    }
} 