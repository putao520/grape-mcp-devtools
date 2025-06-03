use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use tracing::{info, warn, debug};
use tokio::time::{timeout, Duration};

/// AI服务配置
#[derive(Debug, Clone)]
pub struct AIServiceConfig {
    /// API基础URL
    pub api_base: String,
    /// API密钥
    pub api_key: String,
    /// 默认模型
    pub default_model: String,
    /// 请求超时时间（秒）
    pub timeout_secs: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 缓存TTL（秒）
    pub cache_ttl_secs: u64,
}

impl Default for AIServiceConfig {
    fn default() -> Self {
        // 加载环境变量
        dotenv::dotenv().ok();
        
        Self {
            api_base: env::var("LLM_API_BASE_URL")
                .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".to_string()),
            api_key: env::var("LLM_API_KEY")
                .expect("LLM_API_KEY environment variable is required"),
            default_model: env::var("LLM_MODEL_NAME")
                .unwrap_or_else(|_| "nvidia/llama-3.1-nemotron-70b-instruct".to_string()),
            timeout_secs: env::var("AI_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse().unwrap_or(30),
            max_retries: env::var("AI_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse().unwrap_or(3),
            enable_cache: env::var("AI_ENABLE_CACHE")
                .unwrap_or_else(|_| "true".to_string())
                .parse().unwrap_or(true),
            cache_ttl_secs: env::var("AI_CACHE_TTL_SECS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse().unwrap_or(3600),
        }
    }
}

/// AI请求参数
#[derive(Debug, Clone)]
pub struct AIRequest {
    /// 模型名称
    pub model: Option<String>,
    /// 系统提示
    pub system_prompt: Option<String>,
    /// 用户消息
    pub user_message: String,
    /// 温度参数
    pub temperature: Option<f32>,
    /// 最大token数
    pub max_tokens: Option<u32>,
    /// 是否流式响应
    pub stream: bool,
}

/// AI响应结果
#[derive(Debug, Clone)]
pub struct AIResponse {
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
    /// 消耗的token数
    pub tokens_used: Option<u32>,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 是否来自缓存
    pub from_cache: bool,
}

/// AI服务核心实现
#[derive(Clone)]
pub struct AIService {
    config: AIServiceConfig,
    client: Client,
    cache: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, CachedResponse>>>,
}

/// 缓存响应
#[derive(Debug, Clone)]
struct CachedResponse {
    response: AIResponse,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl AIService {
    /// 创建新的AI服务实例
    pub fn new(config: AIServiceConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;

        info!("🤖 初始化AI服务");
        info!("API Base: {}", config.api_base);
        info!("默认模型: {}", config.default_model);
        info!("缓存启用: {}", config.enable_cache);

        Ok(Self {
            config,
            client,
            cache: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// 从环境变量创建AI服务
    pub fn from_env() -> Result<Self> {
        let config = AIServiceConfig::default();
        Self::new(config)
    }

    /// 发送AI请求
    pub async fn request(&self, request: AIRequest) -> Result<AIResponse> {
        let start_time = std::time::Instant::now();
        
        // 检查缓存
        if self.config.enable_cache {
            let cache_key = self.generate_cache_key(&request);
            if let Some(cached) = self.get_cached_response(&cache_key).await {
                debug!("🎯 使用缓存的AI响应");
                return Ok(cached);
            }
        }

        // 发送请求
        let response = self.send_request_with_retry(&request).await?;
        
        // 缓存响应
        if self.config.enable_cache {
            let cache_key = self.generate_cache_key(&request);
            self.cache_response(&cache_key, &response).await;
        }

        let elapsed = start_time.elapsed().as_millis() as u64;
        debug!("🤖 AI请求完成，耗时: {}ms", elapsed);

        Ok(AIResponse {
            response_time_ms: elapsed,
            from_cache: false,
            ..response
        })
    }

    /// 带重试的请求发送
    async fn send_request_with_retry(&self, request: &AIRequest) -> Result<AIResponse> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            match self.send_single_request(request).await {
                Ok(response) => {
                    if attempt > 1 {
                        info!("✅ AI请求重试成功 (第{}次尝试)", attempt);
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        warn!("⚠️ AI请求失败，将重试 (第{}次尝试): {}", attempt, last_error.as_ref().unwrap());
                        tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }

    /// 发送单次请求
    async fn send_single_request(&self, request: &AIRequest) -> Result<AIResponse> {
        let model = request.model.as_ref()
            .unwrap_or(&self.config.default_model);

        let mut messages = Vec::new();
        
        // 添加系统消息
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }
        
        // 添加用户消息
        messages.push(json!({
            "role": "user",
            "content": request.user_message
        }));

        let mut request_body = json!({
            "model": model,
            "messages": messages,
            "stream": request.stream
        });

        // 添加可选参数
        if let Some(temperature) = request.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(max_tokens) = request.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }

        let response = timeout(
            Duration::from_secs(self.config.timeout_secs),
            self.client
                .post(&format!("{}/chat/completions", self.config.api_base))
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        ).await??;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("AI API调用失败: {} - {}", status, error_text));
        }

        let response_json: Value = response.json().await?;
        
        // 解析响应
        let content = response_json
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| anyhow::anyhow!("无效的AI响应格式"))?
            .to_string();

        let tokens_used = response_json
            .get("usage")
            .and_then(|usage| usage.get("total_tokens"))
            .and_then(|tokens| tokens.as_u64())
            .map(|t| t as u32);

        Ok(AIResponse {
            content,
            model: model.clone(),
            tokens_used,
            response_time_ms: 0, // 将在上层设置
            from_cache: false,
        })
    }

    /// 生成缓存键
    fn generate_cache_key(&self, request: &AIRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.user_message.hash(&mut hasher);
        request.system_prompt.hash(&mut hasher);
        request.model.hash(&mut hasher);
        request.temperature.map(|t| (t * 1000.0) as u32).hash(&mut hasher);
        request.max_tokens.hash(&mut hasher);
        
        format!("ai_cache_{:x}", hasher.finish())
    }

    /// 获取缓存响应
    async fn get_cached_response(&self, cache_key: &str) -> Option<AIResponse> {
        let cache = self.cache.read().await;
        
        if let Some(cached) = cache.get(cache_key) {
            let age = chrono::Utc::now().signed_duration_since(cached.timestamp);
            if age.num_seconds() < self.config.cache_ttl_secs as i64 {
                let mut response = cached.response.clone();
                response.from_cache = true;
                return Some(response);
            }
        }
        
        None
    }

    /// 缓存响应
    async fn cache_response(&self, cache_key: &str, response: &AIResponse) {
        let mut cache = self.cache.write().await;
        
        cache.insert(cache_key.to_string(), CachedResponse {
            response: response.clone(),
            timestamp: chrono::Utc::now(),
        });
        
        // 清理过期缓存
        let now = chrono::Utc::now();
        cache.retain(|_, cached| {
            let age = now.signed_duration_since(cached.timestamp);
            age.num_seconds() < self.config.cache_ttl_secs as i64
        });
    }

    /// 清理缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("🧹 AI服务缓存已清理");
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let now = chrono::Utc::now();
        let valid = cache.values().filter(|cached| {
            let age = now.signed_duration_since(cached.timestamp);
            age.num_seconds() < self.config.cache_ttl_secs as i64
        }).count();
        
        (total, valid)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool> {
        let test_request = AIRequest {
            model: None,
            system_prompt: Some("You are a helpful assistant.".to_string()),
            user_message: "Hello, this is a health check. Please respond with 'OK'.".to_string(),
            temperature: Some(0.1),
            max_tokens: Some(10),
            stream: false,
        };

        match timeout(Duration::from_secs(10), self.send_single_request(&test_request)).await {
            Ok(Ok(response)) => {
                debug!("🏥 AI服务健康检查通过: {}", response.content);
                Ok(true)
            }
            Ok(Err(e)) => {
                warn!("🏥 AI服务健康检查失败: {}", e);
                Ok(false)
            }
            Err(_) => {
                warn!("🏥 AI服务健康检查超时");
                Ok(false)
            }
        }
    }
} 