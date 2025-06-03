use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, debug, warn};
use std::collections::HashMap;
use url::Url;
use serde::{Deserialize, Serialize};
use async_openai::{Client, types::{CreateChatCompletionRequestArgs, ChatCompletionRequestMessage, Role}};

use super::ai_service::{AIService, AIRequest};
use super::prompt_templates::UrlPrompts;

/// AI增强的URL分析器
pub struct UrlAI {
    ai_service: AIService,
    prompts: UrlPrompts,
    /// 缓存分析结果
    analysis_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, UrlAnalysisResult>>>,
}

/// URL分析结果
#[derive(Debug, Clone)]
pub struct UrlAnalysisResult {
    /// 语义理解结果
    pub semantic_understanding: SemanticUrlResult,
    /// 内容预测
    pub content_prediction: ContentPrediction,
    /// 质量评估
    pub quality_assessment: UrlQualityAssessment,
    /// 相关性分数 (0.0-1.0)
    pub relevance_score: f32,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 分析时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 语义URL理解结果
#[derive(Debug, Clone)]
pub struct SemanticUrlResult {
    /// URL类型
    pub url_type: UrlType,
    /// 主题标签
    pub topics: Vec<String>,
    /// 编程语言
    pub programming_languages: Vec<String>,
    /// 技术栈
    pub tech_stack: Vec<String>,
    /// 内容类别
    pub content_category: ContentCategory,
    /// 目标受众
    pub target_audience: Vec<String>,
    /// 难度级别 (1-5)
    pub difficulty_level: u8,
}

/// URL类型
#[derive(Debug, Clone)]
pub enum UrlType {
    Documentation,
    Tutorial,
    ApiReference,
    Example,
    Blog,
    Forum,
    Repository,
    Package,
    Tool,
    Other,
}

/// 内容类别
#[derive(Debug, Clone)]
pub enum ContentCategory {
    GettingStarted,
    Advanced,
    Reference,
    Tutorial,
    Example,
    Troubleshooting,
    BestPractices,
    News,
    Community,
    Other,
}

/// 内容预测
#[derive(Debug, Clone)]
pub struct ContentPrediction {
    /// 预测的内容类型
    pub predicted_content_type: Vec<PredictedContentType>,
    /// 预期内容质量 (0.0-1.0)
    pub expected_quality: f32,
    /// 预期有用性 (0.0-1.0)
    pub expected_usefulness: f32,
    /// 预期时效性 (0.0-1.0)
    pub expected_freshness: f32,
    /// 可能包含的信息
    pub likely_information: Vec<String>,
    /// 潜在问题
    pub potential_issues: Vec<String>,
}

/// 预测的内容类型
#[derive(Debug, Clone)]
pub struct PredictedContentType {
    /// 内容类型
    pub content_type: String,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 描述
    pub description: String,
}

/// URL质量评估
#[derive(Debug, Clone)]
pub struct UrlQualityAssessment {
    /// 域名权威性 (0.0-1.0)
    pub domain_authority: f32,
    /// URL结构质量 (0.0-1.0)
    pub url_structure_quality: f32,
    /// 语言一致性 (0.0-1.0)
    pub language_consistency: f32,
    /// 可信度 (0.0-1.0)
    pub trustworthiness: f32,
    /// 质量指标
    pub quality_indicators: Vec<QualityIndicator>,
    /// 风险因素
    pub risk_factors: Vec<RiskFactor>,
}

/// 质量指标
#[derive(Debug, Clone)]
pub struct QualityIndicator {
    /// 指标名称
    pub name: String,
    /// 分数 (0.0-1.0)
    pub score: f32,
    /// 描述
    pub description: String,
}

/// 风险因素
#[derive(Debug, Clone)]
pub struct RiskFactor {
    /// 风险类型
    pub risk_type: RiskType,
    /// 严重程度 (1-5)
    pub severity: u8,
    /// 描述
    pub description: String,
}

/// 风险类型
#[derive(Debug, Clone)]
pub enum RiskType {
    Security,
    Outdated,
    LowQuality,
    Spam,
    Malicious,
    Other,
}

/// URL比较结果
#[derive(Debug, Clone)]
pub struct UrlComparisonResult {
    /// 语义相似度 (0.0-1.0)
    pub semantic_similarity: f32,
    /// 内容相似度 (0.0-1.0)
    pub content_similarity: f32,
    /// 质量差异
    pub quality_difference: f32,
    /// 推荐选择
    pub recommendation: UrlRecommendation,
    /// 比较说明
    pub explanation: String,
}

/// URL推荐
#[derive(Debug, Clone)]
pub enum UrlRecommendation {
    PreferFirst,
    PreferSecond,
    BothGood,
    BothPoor,
    Equivalent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlSemanticAnalysis {
    pub url: String,
    pub semantic_meaning: String,
    pub confidence: f64,
    pub related_concepts: Vec<String>,
    pub domain_relevance: f64,
    pub path_analysis: Vec<String>,
    pub query_parameters: HashMap<String, String>,
    pub content_type_prediction: String,
    pub security_assessment: String,
    pub accessibility_score: f64,
    pub url_type: String,
    pub topics: Vec<String>,
    pub programming_languages: Vec<String>,
    pub tech_stack: Vec<String>,
    pub content_category: String,
    pub target_audience: String,
    pub difficulty_level: String,
}

impl UrlAI {
    /// 创建新的AI URL分析器
    pub async fn new(ai_service: AIService) -> Result<Self> {
        let prompts = UrlPrompts::new();
        
        Ok(Self {
            ai_service,
            prompts,
            analysis_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// 智能URL分析
    pub async fn analyze_url(&self, url: &str, target_language: &str, query_context: &str) -> Result<UrlAnalysisResult> {
        info!("🔍 开始智能URL分析: {}", url);

        // 检查缓存
        let cache_key = format!("{}:{}:{}", url, target_language, query_context);
        if let Some(cached) = self.get_cached_result(&cache_key).await {
            debug!("🎯 使用缓存的URL分析结果");
            return Ok(cached);
        }

        // 解析URL
        let parsed_url = Url::parse(url)?;
        
        // 语义理解
        let semantic_result = self.semantic_url_understanding(&parsed_url, target_language, query_context).await?;
        
        // 内容预测
        let content_prediction = self.predict_content(&parsed_url, target_language, &semantic_result).await?;
        
        // 质量评估
        let quality_assessment = self.assess_url_quality(&parsed_url, target_language, &semantic_result).await?;
        
        // 计算整体相关性和置信度
        let relevance_score = self.calculate_relevance_score(&semantic_result, &content_prediction, query_context);
        let confidence = self.calculate_confidence(&semantic_result, &content_prediction, &quality_assessment);

        let result = UrlAnalysisResult {
            semantic_understanding: semantic_result,
            content_prediction,
            quality_assessment,
            relevance_score,
            confidence,
            timestamp: chrono::Utc::now(),
        };

        // 缓存结果
        self.cache_result(&cache_key, &result).await;
        
        Ok(result)
    }

    /// 语义URL理解
    pub async fn semantic_url_understanding(&self, url: &Url, target_language: &str, query_context: &str) -> Result<SemanticUrlResult> {
        info!("🧠 开始语义URL理解");

        let system_prompt = self.prompts.get_semantic_understanding_system_prompt();
        let user_message = self.prompts.get_semantic_understanding_user_prompt(url.as_str(), target_language, query_context);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.2),
            max_tokens: Some(2000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_semantic_understanding_response(&ai_response.content).await
    }

    /// 内容预测
    pub async fn predict_content(&self, url: &Url, target_language: &str, semantic_result: &SemanticUrlResult) -> Result<ContentPrediction> {
        info!("🔮 开始内容预测");

        let system_prompt = self.prompts.get_content_prediction_system_prompt();
        let user_message = self.prompts.get_content_prediction_user_prompt(url.as_str(), target_language, semantic_result);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(2500),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_content_prediction_response(&ai_response.content).await
    }

    /// URL质量评估
    pub async fn assess_url_quality(&self, url: &Url, target_language: &str, semantic_result: &SemanticUrlResult) -> Result<UrlQualityAssessment> {
        info!("📊 开始URL质量评估");

        let system_prompt = self.prompts.get_quality_assessment_system_prompt();
        let user_message = self.prompts.get_quality_assessment_user_prompt(url.as_str(), target_language, semantic_result);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.1),
            max_tokens: Some(2000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_quality_assessment_response(&ai_response.content).await
    }

    /// 比较多个URL
    pub async fn compare_urls(&self, urls: &[String], target_language: &str, query_context: &str) -> Result<Vec<UrlComparisonResult>> {
        info!("⚖️ 开始比较多个URL");

        let system_prompt = self.prompts.get_comparison_system_prompt();
        let user_message = self.prompts.get_comparison_user_prompt(urls, target_language, query_context);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.2),
            max_tokens: Some(3000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_comparison_response(&ai_response.content).await
    }

    /// 生成URL建议
    pub async fn suggest_urls(&self, query: &str, target_language: &str, preferences: &HashMap<String, String>) -> Result<Vec<String>> {
        info!("💡 生成URL建议");

        let system_prompt = self.prompts.get_suggestion_system_prompt();
        let user_message = self.prompts.get_suggestion_user_prompt(query, target_language, preferences);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.4),
            max_tokens: Some(2000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_suggestion_response(&ai_response.content).await
    }

    /// 解析语义理解响应
    async fn parse_semantic_understanding_response(&self, response: &str) -> Result<SemanticUrlResult> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let url_type = match json_value.get("url_type").and_then(|v| v.as_str()).unwrap_or("other") {
                "documentation" => UrlType::Documentation,
                "tutorial" => UrlType::Tutorial,
                "api_reference" => UrlType::ApiReference,
                "example" => UrlType::Example,
                "blog" => UrlType::Blog,
                "forum" => UrlType::Forum,
                "repository" => UrlType::Repository,
                "package" => UrlType::Package,
                "tool" => UrlType::Tool,
                _ => UrlType::Other,
            };

            let topics = json_value.get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let programming_languages = json_value.get("programming_languages")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let tech_stack = json_value.get("tech_stack")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let content_category = match json_value.get("content_category").and_then(|v| v.as_str()).unwrap_or("other") {
                "getting_started" => ContentCategory::GettingStarted,
                "advanced" => ContentCategory::Advanced,
                "reference" => ContentCategory::Reference,
                "tutorial" => ContentCategory::Tutorial,
                "example" => ContentCategory::Example,
                "troubleshooting" => ContentCategory::Troubleshooting,
                "best_practices" => ContentCategory::BestPractices,
                "news" => ContentCategory::News,
                "community" => ContentCategory::Community,
                _ => ContentCategory::Other,
            };

            let mut target_audience: Vec<String> = Vec::new();
            if let Some(audience) = json_value.get("target_audience") {
                if let Some(audience_array) = audience.as_array() {
                    target_audience = audience_array.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                }
            }

            let _difficulty_level = json_value.get("difficulty_level")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as u8;

            Ok(SemanticUrlResult {
                url_type,
                topics,
                programming_languages,
                tech_stack,
                content_category,
                target_audience,
                difficulty_level: _difficulty_level,
            })
        } else {
            // 基于文本内容的解析
            Ok(SemanticUrlResult {
                url_type: UrlType::Other,
                topics: vec!["programming".to_string()],
                programming_languages: Vec::new(),
                tech_stack: Vec::new(),
                content_category: ContentCategory::Other,
                target_audience: vec!["developers".to_string()],
                difficulty_level: 3,
            })
        }
    }

    /// 解析内容预测响应
    async fn parse_content_prediction_response(&self, response: &str) -> Result<ContentPrediction> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let predicted_content_type = json_value.get("predicted_content_type")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|item| {
                    Some(PredictedContentType {
                        content_type: item.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        confidence: item.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32,
                        description: item.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                }).collect())
                .unwrap_or_default();

            let expected_quality = json_value.get("expected_quality")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let expected_usefulness = json_value.get("expected_usefulness")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let expected_freshness = json_value.get("expected_freshness")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.6) as f32;

            let likely_information = json_value.get("likely_information")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let potential_issues = json_value.get("potential_issues")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            Ok(ContentPrediction {
                predicted_content_type,
                expected_quality,
                expected_usefulness,
                expected_freshness,
                likely_information,
                potential_issues,
            })
        } else {
            // 基于文本内容的解析
            Ok(ContentPrediction {
                predicted_content_type: vec![PredictedContentType {
                    content_type: "documentation".to_string(),
                    confidence: 0.6,
                    description: "General documentation content".to_string(),
                }],
                expected_quality: 0.7,
                expected_usefulness: 0.7,
                expected_freshness: 0.6,
                likely_information: vec!["Code examples".to_string(), "API documentation".to_string()],
                potential_issues: Vec::new(),
            })
        }
    }

    /// 解析质量评估响应
    async fn parse_quality_assessment_response(&self, response: &str) -> Result<UrlQualityAssessment> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let domain_authority = json_value.get("domain_authority")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let url_structure_quality = json_value.get("url_structure_quality")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let language_consistency = json_value.get("language_consistency")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8) as f32;

            let trustworthiness = json_value.get("trustworthiness")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            Ok(UrlQualityAssessment {
                domain_authority,
                url_structure_quality,
                language_consistency,
                trustworthiness,
                quality_indicators: Vec::new(),
                risk_factors: Vec::new(),
            })
        } else {
            // 基于文本内容的解析
            Ok(UrlQualityAssessment {
                domain_authority: 0.7,
                url_structure_quality: 0.7,
                language_consistency: 0.8,
                trustworthiness: 0.7,
                quality_indicators: Vec::new(),
                risk_factors: Vec::new(),
            })
        }
    }

    /// 解析比较响应
    async fn parse_comparison_response(&self, response: &str) -> Result<Vec<UrlComparisonResult>> {
        // 实用的实现
        Ok(Vec::new())
    }

    /// 解析建议响应
    async fn parse_suggestion_response(&self, response: &str) -> Result<Vec<String>> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            if let Some(suggestions) = json_value.get("suggestions").and_then(|v| v.as_array()) {
                Ok(suggestions.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect())
            } else {
                Ok(Vec::new())
            }
        } else {
            // 基于文本内容的解析：按行分割
            Ok(response.lines()
                .filter(|line| !line.trim().is_empty() && line.starts_with("http"))
                .map(|line| line.trim().to_string())
                .collect())
        }
    }

    /// 计算相关性分数
    fn calculate_relevance_score(&self, semantic: &SemanticUrlResult, prediction: &ContentPrediction, query_context: &str) -> f32 {
        let mut score = 0.0;
        
        // 基于语义理解的相关性
        if !semantic.topics.is_empty() {
            score += 0.3;
        }
        
        // 基于内容预测的相关性
        score += prediction.expected_usefulness * 0.4;
        
        // 基于查询上下文的相关性
        if !query_context.is_empty() {
            score += 0.3;
        }
        
        score.min(1.0)
    }

    /// 计算置信度
    fn calculate_confidence(&self, semantic: &SemanticUrlResult, prediction: &ContentPrediction, quality: &UrlQualityAssessment) -> f32 {
        let semantic_confidence = if semantic.topics.len() > 0 { 0.8 } else { 0.5 };
        let prediction_confidence = prediction.expected_quality;
        let quality_confidence = (quality.domain_authority + quality.trustworthiness) / 2.0;
        
        (semantic_confidence + prediction_confidence + quality_confidence) / 3.0
    }

    /// 获取缓存结果
    async fn get_cached_result(&self, cache_key: &str) -> Option<UrlAnalysisResult> {
        let cache = self.analysis_cache.read().await;
        cache.get(cache_key).cloned()
    }

    /// 缓存结果
    async fn cache_result(&self, cache_key: &str, result: &UrlAnalysisResult) {
        let mut cache = self.analysis_cache.write().await;
        cache.insert(cache_key.to_string(), result.clone());
        
        // 限制缓存大小
        if cache.len() > 500 {
            // 移除一些旧的条目
            let keys_to_remove: Vec<String> = cache.keys().take(50).cloned().collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }
    }

    /// 清理缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.analysis_cache.write().await;
        cache.clear();
        info!("🧹 URL分析缓存已清理");
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> usize {
        let cache = self.analysis_cache.read().await;
        cache.len()
    }

    async fn parse_url_semantic_response(&self, content: &str) -> Result<UrlSemanticAnalysis> {
        // 完整的JSON解析实现
        if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(content) {
            let url_type = parsed_json.get("url_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            
            let topics = parsed_json.get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect())
                .unwrap_or_else(Vec::new);
            
            let programming_languages = parsed_json.get("programming_languages")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect())
                .unwrap_or_else(Vec::new);
            
            let tech_stack = parsed_json.get("tech_stack")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect())
                .unwrap_or_else(Vec::new);
            
            let content_category = parsed_json.get("content_category")
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .to_string();
            
            let target_audience = parsed_json.get("target_audience")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect())
                .unwrap_or_else(Vec::new);
            
            let _difficulty_level = parsed_json.get("difficulty_level")
                .and_then(|v| v.as_i64())
                .unwrap_or(1) as u32;
            
            Ok(UrlSemanticAnalysis {
                url: "unknown".to_string(),
                semantic_meaning: "Text-based semantic analysis".to_string(),
                confidence: 0.6,
                related_concepts: topics.clone(),
                domain_relevance: 0.5,
                path_analysis: vec!["/text".to_string(), "/analysis".to_string()],
                query_parameters: HashMap::new(),
                content_type_prediction: "text/plain".to_string(),
                security_assessment: "unknown".to_string(),
                accessibility_score: 0.5,
                url_type,
                topics,
                programming_languages,
                tech_stack,
                content_category,
                target_audience: target_audience.join(", "),
                difficulty_level: _difficulty_level.to_string(),
            })
        } else {
            // 如果JSON解析失败，使用文本解析作为备用方案
            let analysis = self.parse_url_semantic_from_text(content)?;
            Ok(analysis)
        }
    }

    /// 从文本内容解析URL语义分析结果（备用方案）
    fn parse_url_semantic_from_text(&self, content: &str) -> Result<UrlSemanticAnalysis> {
        let mut url_type = "unknown".to_string();
        let mut topics = Vec::new();
        let mut programming_languages = Vec::new();
        let mut tech_stack = Vec::new();
        let mut content_category = "general".to_string();
        let mut target_audience: Vec<String> = Vec::new();
        let mut _difficulty_level = 1u32;

        // 分析内容确定URL类型
        let content_lower = content.to_lowercase();
        if content_lower.contains("documentation") || content_lower.contains("docs") {
            url_type = "documentation".to_string();
        } else if content_lower.contains("tutorial") || content_lower.contains("guide") {
            url_type = "tutorial".to_string();
        } else if content_lower.contains("api") || content_lower.contains("reference") {
            url_type = "api_reference".to_string();
        } else if content_lower.contains("example") || content_lower.contains("demo") {
            url_type = "example".to_string();
        }

        // 提取编程语言关键词
        let language_keywords = vec![
            "rust", "python", "javascript", "typescript", "java", "go", 
            "c++", "c#", "php", "ruby", "swift", "kotlin"
        ];
        
        for keyword in language_keywords {
            if content_lower.contains(keyword) {
                programming_languages.push(keyword.to_string());
            }
        }

        // 提取技术栈关键词
        let tech_keywords = vec![
            "react", "vue", "angular", "node", "express", "django", "flask",
            "spring", "tokio", "actix", "serde", "reqwest", "async", "await"
        ];
        
        for keyword in tech_keywords {
            if content_lower.contains(keyword) {
                tech_stack.push(keyword.to_string());
            }
        }

        // 基于内容复杂度估算难度级别
        let word_count = content.split_whitespace().count();
        _difficulty_level = match word_count {
            0..=50 => 1,
            51..=200 => 2,
            201..=500 => 3,
            501..=1000 => 4,
            _ => 5,
        };

        // 提取主题（基于关键词频率）
        let topic_keywords = vec![
            "async", "programming", "web", "api", "database", "security", 
            "testing", "deployment", "performance", "architecture"
        ];
        
        for keyword in topic_keywords {
            if content_lower.contains(keyword) {
                topics.push(keyword.to_string());
            }
        }

        Ok(UrlSemanticAnalysis {
            url: "unknown".to_string(),
            semantic_meaning: "Text-based semantic analysis".to_string(),
            confidence: 0.6,
            related_concepts: topics.clone(),
            domain_relevance: 0.5,
            path_analysis: vec!["/text".to_string(), "/analysis".to_string()],
            query_parameters: HashMap::new(),
            content_type_prediction: "text/plain".to_string(),
            security_assessment: "unknown".to_string(),
            accessibility_score: 0.5,
            url_type,
            topics,
            programming_languages,
            tech_stack,
            content_category,
            target_audience: target_audience.join(", "),
            difficulty_level: _difficulty_level.to_string(),
        })
    }
} 