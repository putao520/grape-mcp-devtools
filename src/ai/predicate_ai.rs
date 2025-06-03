use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, debug, warn};
use std::collections::HashMap;

use super::ai_service::{AIService, AIRequest};
use super::prompt_templates::PredicatePrompts;
use crate::tools::environment_detector::LanguageInfo;

/// AI增强的自定义谓词处理器
pub struct PredicateAI {
    ai_service: AIService,
    prompts: PredicatePrompts,
    /// 缓存已解析的谓词
    predicate_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, PredicateResult>>>,
}

/// 谓词评估结果
#[derive(Debug, Clone)]
pub struct PredicateResult {
    /// 评估结果
    pub result: bool,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 解释说明
    pub explanation: String,
    /// 使用的条件
    pub conditions: Vec<EvaluatedCondition>,
    /// 推理过程
    pub reasoning_steps: Vec<String>,
}

/// 已评估的条件
#[derive(Debug, Clone)]
pub struct EvaluatedCondition {
    /// 条件描述
    pub condition: String,
    /// 评估结果
    pub result: bool,
    /// 置信度
    pub confidence: f32,
    /// 相关证据
    pub evidence: Vec<String>,
}

/// 自然语言谓词
#[derive(Debug, Clone)]
pub struct NaturalLanguagePredicate {
    /// 原始文本
    pub text: String,
    /// 解析后的条件
    pub parsed_conditions: Vec<ParsedCondition>,
    /// 逻辑关系
    pub logic_operator: LogicOperator,
}

/// 解析后的条件
#[derive(Debug, Clone)]
pub struct ParsedCondition {
    /// 条件类型
    pub condition_type: ConditionType,
    /// 参数
    pub parameters: HashMap<String, String>,
    /// 期望值
    pub expected_value: String,
    /// 比较操作符
    pub operator: ComparisonOperator,
}

/// 条件类型
#[derive(Debug, Clone)]
pub enum ConditionType {
    /// 项目包含特定文件
    HasFile,
    /// 项目使用特定框架
    UsesFramework,
    /// 项目有特定依赖
    HasDependency,
    /// 代码质量分数
    CodeQuality,
    /// 项目规模
    ProjectSize,
    /// 语言版本
    LanguageVersion,
    /// 自定义条件
    Custom(String),
}

/// 比较操作符
#[derive(Debug, Clone)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Contains,
    NotContains,
    Matches,
}

/// 逻辑操作符
#[derive(Debug, Clone)]
pub enum LogicOperator {
    And,
    Or,
    Not,
    Xor,
}

impl PredicateAI {
    /// 创建新的AI谓词处理器
    pub async fn new(ai_service: AIService) -> Result<Self> {
        let prompts = PredicatePrompts::new();
        
        Ok(Self {
            ai_service,
            prompts,
            predicate_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// 评估自然语言谓词
    pub async fn evaluate_natural_language_predicate(
        &self,
        predicate_text: &str,
        language: &str,
        info: &LanguageInfo,
    ) -> Result<PredicateResult> {
        info!("🧠 评估自然语言谓词: {}", predicate_text);

        // 检查缓存
        let cache_key = format!("{}:{}:{}", predicate_text, language, self.generate_info_hash(info));
        if let Some(cached) = self.get_cached_result(&cache_key).await {
            debug!("🎯 使用缓存的谓词结果");
            return Ok(cached);
        }

        // 解析自然语言谓词
        let parsed_predicate = self.parse_natural_language_predicate(predicate_text).await?;
        
        // 评估谓词
        let result = self.evaluate_parsed_predicate(&parsed_predicate, language, info).await?;
        
        // 缓存结果
        self.cache_result(&cache_key, &result).await;
        
        Ok(result)
    }

    /// 解析自然语言谓词
    pub async fn parse_natural_language_predicate(&self, predicate_text: &str) -> Result<NaturalLanguagePredicate> {
        info!("🔍 解析自然语言谓词");

        let system_prompt = self.prompts.get_parsing_system_prompt();
        let user_message = self.prompts.get_parsing_user_prompt(predicate_text);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.1),
            max_tokens: Some(2000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_predicate_response(&ai_response.content, predicate_text).await
    }

    /// 评估解析后的谓词
    pub async fn evaluate_parsed_predicate(
        &self,
        predicate: &NaturalLanguagePredicate,
        language: &str,
        info: &LanguageInfo,
    ) -> Result<PredicateResult> {
        info!("⚖️ 评估解析后的谓词");

        let system_prompt = self.prompts.get_evaluation_system_prompt();
        let user_message = self.prompts.get_evaluation_user_prompt(predicate, language, info);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.2),
            max_tokens: Some(3000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_evaluation_response(&ai_response.content).await
    }

    /// 智能推理条件
    pub async fn intelligent_reasoning(
        &self,
        conditions: &[String],
        context: &str,
        language: &str,
        info: &LanguageInfo,
    ) -> Result<PredicateResult> {
        info!("🤔 开始智能推理");

        let system_prompt = self.prompts.get_reasoning_system_prompt();
        let user_message = self.prompts.get_reasoning_user_prompt(conditions, context, language, info);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(3000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_reasoning_response(&ai_response.content).await
    }

    /// 生成谓词建议
    pub async fn suggest_predicates(
        &self,
        language: &str,
        info: &LanguageInfo,
        use_case: &str,
    ) -> Result<Vec<String>> {
        info!("💡 生成谓词建议");

        let system_prompt = self.prompts.get_suggestion_system_prompt();
        let user_message = self.prompts.get_suggestion_user_prompt(language, info, use_case);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.4),
            max_tokens: Some(2000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_suggestions_response(&ai_response.content).await
    }

    /// 解析谓词响应
    async fn parse_predicate_response(&self, response: &str, original_text: &str) -> Result<NaturalLanguagePredicate> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let conditions = if let Some(conditions_array) = json_value.get("conditions").and_then(|v| v.as_array()) {
                conditions_array.iter().filter_map(|condition| {
                    let condition_type = match condition.get("type").and_then(|v| v.as_str())? {
                        "has_file" => ConditionType::HasFile,
                        "uses_framework" => ConditionType::UsesFramework,
                        "has_dependency" => ConditionType::HasDependency,
                        "code_quality" => ConditionType::CodeQuality,
                        "project_size" => ConditionType::ProjectSize,
                        "language_version" => ConditionType::LanguageVersion,
                        other => ConditionType::Custom(other.to_string()),
                    };

                    let parameters = condition.get("parameters")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                        .unwrap_or_default();

                    let expected_value = condition.get("expected_value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let operator = match condition.get("operator").and_then(|v| v.as_str()).unwrap_or("equal") {
                        "equal" => ComparisonOperator::Equal,
                        "not_equal" => ComparisonOperator::NotEqual,
                        "greater_than" => ComparisonOperator::GreaterThan,
                        "less_than" => ComparisonOperator::LessThan,
                        "greater_or_equal" => ComparisonOperator::GreaterOrEqual,
                        "less_or_equal" => ComparisonOperator::LessOrEqual,
                        "contains" => ComparisonOperator::Contains,
                        "not_contains" => ComparisonOperator::NotContains,
                        "matches" => ComparisonOperator::Matches,
                        _ => ComparisonOperator::Equal,
                    };

                    Some(ParsedCondition {
                        condition_type,
                        parameters,
                        expected_value,
                        operator,
                    })
                }).collect()
            } else {
                Vec::new()
            };

            let logic_operator = match json_value.get("logic_operator").and_then(|v| v.as_str()).unwrap_or("and") {
                "and" => LogicOperator::And,
                "or" => LogicOperator::Or,
                "not" => LogicOperator::Not,
                "xor" => LogicOperator::Xor,
                _ => LogicOperator::And,
            };

            Ok(NaturalLanguagePredicate {
                text: original_text.to_string(),
                parsed_conditions: conditions,
                logic_operator,
            })
        } else {
            // 基于文本内容的解析
            Ok(NaturalLanguagePredicate {
                text: original_text.to_string(),
                parsed_conditions: vec![ParsedCondition {
                    condition_type: ConditionType::Custom(original_text.to_string()),
                    parameters: HashMap::new(),
                    expected_value: "true".to_string(),
                    operator: ComparisonOperator::Equal,
                }],
                logic_operator: LogicOperator::And,
            })
        }
    }

    /// 解析评估响应
    async fn parse_evaluation_response(&self, response: &str) -> Result<PredicateResult> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let result = json_value.get("result")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let confidence = json_value.get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32;

            let explanation = json_value.get("explanation")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let reasoning_steps = json_value.get("reasoning_steps")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let conditions = json_value.get("conditions")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|condition| {
                    Some(EvaluatedCondition {
                        condition: condition.get("condition").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        result: condition.get("result").and_then(|v| v.as_bool()).unwrap_or(false),
                        confidence: condition.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32,
                        evidence: condition.get("evidence")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default(),
                    })
                }).collect())
                .unwrap_or_default();

            Ok(PredicateResult {
                result,
                confidence,
                explanation,
                conditions,
                reasoning_steps,
            })
        } else {
            // 基于文本内容的解析
            Ok(PredicateResult {
                result: false,
                confidence: 0.5,
                explanation: "无法解析AI响应".to_string(),
                conditions: Vec::new(),
                reasoning_steps: Vec::new(),
            })
        }
    }

    /// 解析推理响应
    async fn parse_reasoning_response(&self, response: &str) -> Result<PredicateResult> {
        // 与评估响应解析类似
        self.parse_evaluation_response(response).await
    }

    /// 解析建议响应
    async fn parse_suggestions_response(&self, response: &str) -> Result<Vec<String>> {
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
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string())
                .collect())
        }
    }

    /// 生成信息哈希
    fn generate_info_hash(&self, info: &LanguageInfo) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        info.project_files.hash(&mut hasher);
        info.detected_features.hash(&mut hasher);
        info.cli_tools.len().hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    /// 获取缓存结果
    async fn get_cached_result(&self, cache_key: &str) -> Option<PredicateResult> {
        let cache = self.predicate_cache.read().await;
        cache.get(cache_key).cloned()
    }

    /// 缓存结果
    async fn cache_result(&self, cache_key: &str, result: &PredicateResult) {
        let mut cache = self.predicate_cache.write().await;
        cache.insert(cache_key.to_string(), result.clone());
        
        // 限制缓存大小
        if cache.len() > 1000 {
            // 移除一些旧的条目
            let keys_to_remove: Vec<String> = cache.keys().take(100).cloned().collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }
    }

    /// 清理缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.predicate_cache.write().await;
        cache.clear();
        info!("🧹 谓词缓存已清理");
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> usize {
        let cache = self.predicate_cache.read().await;
        cache.len()
    }
} 