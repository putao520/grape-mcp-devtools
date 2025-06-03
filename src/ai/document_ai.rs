use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, debug};
use std::collections::HashMap;
use regex;

use super::ai_service::{AIService, AIRequest};
use super::prompt_templates::DocumentPrompts;

/// AI增强的文档处理器
#[derive(Clone)]
pub struct DocumentAI {
    ai_service: AIService,
    prompts: DocumentPrompts,
}

/// 智能提取结果
#[derive(Debug, Clone)]
pub struct IntelligentExtractionResult {
    /// 提取的标题
    pub title: String,
    /// 主要内容
    pub main_content: String,
    /// 代码示例
    pub code_examples: Vec<CodeExample>,
    /// API文档
    pub api_documentation: Vec<ApiDocumentation>,
    /// 教程步骤
    pub tutorial_steps: Vec<TutorialStep>,
    /// 相关链接
    pub related_links: Vec<RelatedLink>,
    /// 内容质量分数 (0.0-1.0)
    pub quality_score: f32,
    /// 相关性分数 (0.0-1.0)
    pub relevance_score: f32,
    /// 内容类型
    pub content_type: ContentType,
    /// 语言检测结果
    pub detected_language: Option<String>,
    /// 提取置信度
    pub confidence: f32,
}

/// 代码示例
#[derive(Debug, Clone)]
pub struct CodeExample {
    /// 编程语言
    pub language: Option<String>,
    /// 代码内容
    pub code: String,
    /// 描述
    pub description: Option<String>,
    /// 是否可运行
    pub is_runnable: bool,
}

/// API文档
#[derive(Debug, Clone)]
pub struct ApiDocumentation {
    /// API名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 参数
    pub parameters: Vec<ApiParameter>,
    /// 返回值
    pub return_type: Option<String>,
    /// 示例用法
    pub examples: Vec<String>,
}

/// API参数
#[derive(Debug, Clone)]
pub struct ApiParameter {
    /// 参数名
    pub name: String,
    /// 类型
    pub param_type: String,
    /// 描述
    pub description: String,
    /// 是否必需
    pub required: bool,
}

/// 教程步骤
#[derive(Debug, Clone)]
pub struct TutorialStep {
    /// 步骤编号
    pub step_number: usize,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 代码示例
    pub code_example: Option<String>,
    /// 预期结果
    pub expected_result: Option<String>,
}

/// 相关链接
#[derive(Debug, Clone)]
pub struct RelatedLink {
    /// 链接文本
    pub text: String,
    /// URL
    pub url: String,
    /// 链接类型
    pub link_type: LinkType,
    /// 相关性分数
    pub relevance_score: f32,
}

/// 链接类型
#[derive(Debug, Clone)]
pub enum LinkType {
    Documentation,
    Tutorial,
    Example,
    Reference,
    Download,
    Other,
}

/// 内容类型
#[derive(Debug, Clone)]
pub enum ContentType {
    Documentation,
    Tutorial,
    ApiReference,
    Example,
    Changelog,
    BlogPost,
    Forum,
    Other,
}

/// 语义分析结果
#[derive(Debug, Clone)]
pub struct SemanticAnalysisResult {
    /// 主题标签
    pub topics: Vec<String>,
    /// 关键概念
    pub key_concepts: Vec<String>,
    /// 难度级别 (1-5)
    pub difficulty_level: u8,
    /// 目标受众
    pub target_audience: Vec<String>,
    /// 内容摘要
    pub summary: String,
    /// 语义相似度（与查询的相关性）
    pub semantic_similarity: f32,
}

/// 质量评估结果
#[derive(Debug, Clone)]
pub struct QualityAssessmentResult {
    /// 整体质量分数 (0.0-1.0)
    pub overall_score: f32,
    /// 内容完整性
    pub completeness_score: f32,
    /// 准确性
    pub accuracy_score: f32,
    /// 可读性
    pub readability_score: f32,
    /// 实用性
    pub usefulness_score: f32,
    /// 时效性
    pub freshness_score: f32,
    /// 质量问题
    pub quality_issues: Vec<QualityIssue>,
    /// 改进建议
    pub improvement_suggestions: Vec<String>,
}

/// 质量问题
#[derive(Debug, Clone)]
pub struct QualityIssue {
    /// 问题类型
    pub issue_type: QualityIssueType,
    /// 问题描述
    pub description: String,
    /// 严重程度 (1-5)
    pub severity: u8,
}

/// 质量问题类型
#[derive(Debug, Clone)]
pub enum QualityIssueType {
    IncompleteInformation,
    OutdatedContent,
    PoorFormatting,
    MissingExamples,
    BrokenLinks,
    IncorrectCode,
    Other,
}

impl DocumentAI {
    /// 创建新的文档AI实例
    pub async fn new(ai_service: AIService) -> Result<Self> {
        let prompts = DocumentPrompts::new();
        
        info!("🤖 文档AI初始化完成");
        Ok(Self {
            ai_service,
            prompts,
        })
    }

    /// 智能内容提取
    pub async fn intelligent_extract(&self, html_content: &str, target_language: &str, query: &str) -> Result<IntelligentExtractionResult> {
        info!("🔍 开始智能内容提取");

        // 预处理HTML内容
        let clean_content = self.preprocess_html(html_content)?;

        // 构建AI请求
        let system_prompt = self.prompts.get_extraction_system_prompt();
        let user_message = self.prompts.get_extraction_user_prompt(&clean_content, target_language, query);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.2),
            max_tokens: Some(4000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        // 解析AI响应
        self.parse_extracted_info(&ai_response.content)
    }

    /// 语义分析
    pub async fn semantic_analysis(&self, content: &str, target_language: &str, query: &str) -> Result<SemanticAnalysisResult> {
        info!("🧠 开始语义分析");

        let system_prompt = self.prompts.get_semantic_analysis_system_prompt();
        let user_message = self.prompts.get_semantic_analysis_user_prompt(content, target_language, query);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(3000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_semantic_analysis_response(&ai_response.content).await
    }

    /// 质量评估
    pub async fn quality_assessment(&self, content: &str, content_type: &str) -> Result<QualityAssessmentResult> {
        info!("📊 开始质量评估");

        let system_prompt = self.prompts.get_quality_assessment_system_prompt();
        let user_message = self.prompts.get_quality_assessment_user_prompt(content, content_type);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.1),
            max_tokens: Some(3000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        self.parse_quality_assessment_response(&ai_response.content).await
    }

    /// 内容翻译
    pub async fn translate_content(&self, content: &str, target_language: &str) -> Result<String> {
        info!("🌐 开始内容翻译");

        let system_prompt = self.prompts.get_translation_system_prompt();
        let user_message = self.prompts.get_translation_user_prompt(content, target_language);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.2),
            max_tokens: Some(4000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        Ok(ai_response.content)
    }

    /// 生成摘要
    pub async fn generate_summary(&self, content: &str, max_length: usize) -> Result<String> {
        info!("📝 开始生成摘要");

        let system_prompt = self.prompts.get_summary_system_prompt();
        let user_message = self.prompts.get_summary_user_prompt(content, max_length);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(2000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        
        Ok(ai_response.content)
    }

    /// 预处理HTML内容
    fn preprocess_html(&self, html_content: &str) -> Result<String> {
        // 移除脚本和样式标签
        let script_re = regex::Regex::new(r"(?s)<script[^>]*>.*?</script>").unwrap();
        let style_re = regex::Regex::new(r"(?s)<style[^>]*>.*?</style>").unwrap();
        let mut cleaned = script_re.replace_all(html_content, "").to_string();
        cleaned = style_re.replace_all(&cleaned, "").to_string();
        
        // 移除HTML注释
        let comment_re = regex::Regex::new(r"(?s)<!--.*?-->").unwrap();
        cleaned = comment_re.replace_all(&cleaned, "").to_string();
        
        // 移除所有HTML标签但保留内容
        let tag_re = regex::Regex::new(r"<[^>]*>").unwrap();
        cleaned = tag_re.replace_all(&cleaned, " ").to_string();
        
        // 清理多余的空白字符
        let space_re = regex::Regex::new(r"\s+").unwrap();
        cleaned = space_re.replace_all(&cleaned, " ").to_string();
        
        Ok(cleaned.trim().to_string())
    }

    /// 解析提取信息
    fn parse_extracted_info(&self, content: &str) -> Result<IntelligentExtractionResult> {
        // 尝试解析JSON响应
        if let Ok(json_value) = serde_json::from_str::<Value>(content) {
            let title = json_value.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("未提取到标题")
                .to_string();

            let main_content = json_value.get("main_content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let code_examples = json_value.get("code_examples")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|example| {
                    Some(CodeExample {
                        language: example.get("language").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        code: example.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        description: example.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        is_runnable: example.get("is_runnable").and_then(|v| v.as_bool()).unwrap_or(false),
                    })
                }).collect())
                .unwrap_or_default();

            let api_documentation = json_value.get("api_documentation")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|api| {
                    let parameters = api.get("parameters")
                        .and_then(|v| v.as_array())
                        .map(|params| params.iter().filter_map(|param| {
                            Some(ApiParameter {
                                name: param.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                param_type: param.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                description: param.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                required: param.get("required").and_then(|v| v.as_bool()).unwrap_or(false),
                            })
                        }).collect())
                        .unwrap_or_default();

                    let examples = api.get("examples")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();

                    Some(ApiDocumentation {
                        name: api.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        description: api.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        parameters,
                        return_type: api.get("return_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        examples,
                    })
                }).collect())
                .unwrap_or_default();

            let tutorial_steps = json_value.get("tutorial_steps")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().enumerate().filter_map(|(i, step)| {
                    Some(TutorialStep {
                        step_number: i + 1,
                        title: step.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        content: step.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        code_example: step.get("code_example").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        expected_result: step.get("expected_result").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    })
                }).collect())
                .unwrap_or_default();

            let related_links = json_value.get("related_links")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|link| {
                    let link_type = match link.get("type").and_then(|v| v.as_str()).unwrap_or("other") {
                        "documentation" => LinkType::Documentation,
                        "tutorial" => LinkType::Tutorial,
                        "example" => LinkType::Example,
                        "reference" => LinkType::Reference,
                        "download" => LinkType::Download,
                        _ => LinkType::Other,
                    };

                    Some(RelatedLink {
                        text: link.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        url: link.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        link_type,
                        relevance_score: link.get("relevance_score").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32,
                    })
                }).collect())
                .unwrap_or_default();

            let quality_score = json_value.get("quality_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let relevance_score = json_value.get("relevance_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let content_type = match json_value.get("content_type").and_then(|v| v.as_str()).unwrap_or("other") {
                "documentation" => ContentType::Documentation,
                "tutorial" => ContentType::Tutorial,
                "api_reference" => ContentType::ApiReference,
                "example" => ContentType::Example,
                "changelog" => ContentType::Changelog,
                "blog_post" => ContentType::BlogPost,
                "forum" => ContentType::Forum,
                _ => ContentType::Other,
            };

            let detected_language = json_value.get("detected_language")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let confidence = json_value.get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8) as f32;

            Ok(IntelligentExtractionResult {
                title,
                main_content,
                code_examples,
                api_documentation,
                tutorial_steps,
                related_links,
                quality_score,
                relevance_score,
                content_type,
                detected_language,
                confidence,
            })
        } else {
            // 如果JSON解析失败，使用备用解析方法
            self.parse_html_content_fallback(content)
        }
    }

    /// 备用HTML内容解析
    fn parse_html_content_fallback(&self, content: &str) -> Result<IntelligentExtractionResult> {
        // 简单的文本处理作为备用方案
        let lines: Vec<&str> = content.lines().collect();
        
        let title = lines.first()
            .map(|line| line.trim().to_string())
            .unwrap_or_else(|| "未提取到标题".to_string());

        let main_content = content.chars().take(1000).collect::<String>();

        // 寻找代码块
        let mut code_examples = Vec::new();
        let mut in_code_block = false;
        let mut current_code = String::new();
        let mut current_language: Option<String> = None;

        for line in lines {
            if line.trim().starts_with("```") {
                if in_code_block {
                    // 结束代码块
                    if !current_code.trim().is_empty() {
                        code_examples.push(CodeExample {
                            language: current_language.clone(),
                            code: current_code.trim().to_string(),
                            description: None,
                            is_runnable: false,
                        });
                    }
                    current_code.clear();
                    current_language = None;
                    in_code_block = false;
                } else {
                    // 开始代码块
                    in_code_block = true;
                    let lang = line.trim().strip_prefix("```").unwrap_or("").trim();
                    if !lang.is_empty() {
                        current_language = Some(lang.to_string());
                    }
                }
            } else if in_code_block {
                current_code.push_str(line);
                current_code.push('\n');
            }
        }

        Ok(IntelligentExtractionResult {
            title,
            main_content,
            code_examples,
            api_documentation: Vec::new(),
            tutorial_steps: Vec::new(),
            related_links: Vec::new(),
            quality_score: 0.6,
            relevance_score: 0.5,
            content_type: ContentType::Other,
            detected_language: Some("Text".to_string()),
            confidence: 0.4,
        })
    }

    /// 解析语义分析响应
    async fn parse_semantic_analysis_response(&self, response: &str) -> Result<SemanticAnalysisResult> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let topics = json_value.get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let key_concepts = json_value.get("key_concepts")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let difficulty_level = json_value.get("difficulty_level")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as u8;

            let target_audience = json_value.get("target_audience")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let summary = json_value.get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let semantic_similarity = json_value.get("semantic_similarity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            Ok(SemanticAnalysisResult {
                topics,
                key_concepts,
                difficulty_level,
                target_audience,
                summary,
                semantic_similarity,
            })
        } else {
            // 基于文本内容的解析：按行分割
            Ok(SemanticAnalysisResult {
                topics: vec!["programming".to_string()],
                key_concepts: vec!["development".to_string()],
                difficulty_level: 3,
                target_audience: vec!["developers".to_string()],
                summary: response.chars().take(200).collect(),
                semantic_similarity: 0.5,
            })
        }
    }

    /// 解析质量评估响应
    async fn parse_quality_assessment_response(&self, response: &str) -> Result<QualityAssessmentResult> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let overall_score = json_value.get("overall_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let completeness_score = json_value.get("completeness_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let accuracy_score = json_value.get("accuracy_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8) as f32;

            let readability_score = json_value.get("readability_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let usefulness_score = json_value.get("usefulness_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            let freshness_score = json_value.get("freshness_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.6) as f32;

            let quality_issues = json_value.get("quality_issues")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|issue| {
                    let issue_type = match issue.get("type").and_then(|v| v.as_str()).unwrap_or("other") {
                        "incomplete_information" => QualityIssueType::IncompleteInformation,
                        "outdated_content" => QualityIssueType::OutdatedContent,
                        "poor_formatting" => QualityIssueType::PoorFormatting,
                        "missing_examples" => QualityIssueType::MissingExamples,
                        "broken_links" => QualityIssueType::BrokenLinks,
                        "incorrect_code" => QualityIssueType::IncorrectCode,
                        _ => QualityIssueType::Other,
                    };

                    Some(QualityIssue {
                        issue_type,
                        description: issue.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        severity: issue.get("severity").and_then(|v| v.as_u64()).unwrap_or(3) as u8,
                    })
                }).collect())
                .unwrap_or_default();

            let improvement_suggestions = json_value.get("improvement_suggestions")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            Ok(QualityAssessmentResult {
                overall_score,
                completeness_score,
                accuracy_score,
                readability_score,
                usefulness_score,
                freshness_score,
                quality_issues,
                improvement_suggestions,
            })
        } else {
            // 基于文本内容的解析
            Ok(QualityAssessmentResult {
                overall_score: 0.7,
                completeness_score: 0.7,
                accuracy_score: 0.7,
                readability_score: 0.7,
                usefulness_score: 0.7,
                freshness_score: 0.7,
                quality_issues: Vec::new(),
                improvement_suggestions: vec!["建议改进内容质量".to_string()],
            })
        }
    }
} 