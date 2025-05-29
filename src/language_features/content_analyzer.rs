use anyhow::{anyhow, Result};
use async_openai::{Client as OpenAIClient, config::OpenAIConfig};
use async_openai::types::{CreateChatCompletionRequestArgs, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs};
use serde_json::{json, Value};
use tracing::{info, warn, debug};
use regex::Regex;

use super::data_models::*;
use super::ai_collector::ChangelogAnalysisResult;

/// AI驱动的变更日志内容分析器
pub struct ChangelogAnalyzer {
    openai_client: Option<OpenAIClient<OpenAIConfig>>,
    model_name: String,
    max_tokens: u16,
    temperature: f32,
    fallback_patterns: FallbackPatterns,
}

/// 备用模式匹配规则
#[derive(Debug, Clone)]
struct FallbackPatterns {
    version_patterns: Vec<Regex>,
    feature_patterns: Vec<Regex>,
    breaking_change_patterns: Vec<Regex>,
    deprecation_patterns: Vec<Regex>,
    syntax_change_patterns: Vec<Regex>,
}

impl ChangelogAnalyzer {
    pub async fn new(openai_api_key: Option<String>) -> Result<Self> {
        let openai_client = if let Some(api_key) = openai_api_key {
            let config = OpenAIConfig::new().with_api_key(api_key);
            Some(OpenAIClient::with_config(config))
        } else {
            warn!("⚠️ OpenAI API密钥未提供，将使用备用模式分析");
            None
        };

        Ok(Self {
            openai_client,
            model_name: "gpt-4".to_string(),
            max_tokens: 2048,
            temperature: 0.1, // 低温度确保一致性
            fallback_patterns: Self::init_fallback_patterns(),
        })
    }

    /// 初始化备用模式匹配规则
    fn init_fallback_patterns() -> FallbackPatterns {
        FallbackPatterns {
            version_patterns: vec![
                Regex::new(r"(?i)version\s+(\d+\.\d+(?:\.\d+)?)").unwrap(),
                Regex::new(r"(?i)v?(\d+\.\d+(?:\.\d+)?)").unwrap(),
                Regex::new(r"(?i)release\s+(\d+\.\d+(?:\.\d+)?)").unwrap(),
            ],
            feature_patterns: vec![
                Regex::new(r"(?i)added?\s+(.+)").unwrap(),
                Regex::new(r"(?i)new\s+(.+)").unwrap(),
                Regex::new(r"(?i)introduced?\s+(.+)").unwrap(),
                Regex::new(r"(?i)implemented?\s+(.+)").unwrap(),
                Regex::new(r"(?i)stabilized?\s+(.+)").unwrap(),
            ],
            breaking_change_patterns: vec![
                Regex::new(r"(?i)breaking\s+change").unwrap(),
                Regex::new(r"(?i)backward.incompatible").unwrap(),
                Regex::new(r"(?i)removed?\s+(.+)").unwrap(),
                Regex::new(r"(?i)changed?\s+(.+)").unwrap(),
            ],
            deprecation_patterns: vec![
                Regex::new(r"(?i)deprecated?\s+(.+)").unwrap(),
                Regex::new(r"(?i)obsolete").unwrap(),
                Regex::new(r"(?i)will\s+be\s+removed").unwrap(),
            ],
            syntax_change_patterns: vec![
                Regex::new(r"(?i)syntax\s+(.+)").unwrap(),
                Regex::new(r"(?i)grammar\s+(.+)").unwrap(),
                Regex::new(r"(?i)parser\s+(.+)").unwrap(),
            ],
        }
    }

    /// 分析变更日志内容
    pub async fn analyze_changelog_content(&self, content: &str, language: &str) -> Result<Value> {
        info!("🤖 开始AI分析变更日志内容，语言: {}", language);

        if let Some(ref openai_client) = self.openai_client {
            match self.analyze_with_ai(content, language, openai_client).await {
                Ok(result) => {
                    info!("✅ AI分析成功");
                    return Ok(result);
                }
                Err(e) => {
                    warn!("⚠️ AI分析失败，使用备用方法: {}", e);
                }
            }
        }

        // 使用备用模式分析
        self.analyze_with_patterns(content, language).await
    }

    /// 分析release notes
    pub async fn analyze_release_notes(&self, content: &str, language: &str) -> Result<ChangelogAnalysisResult> {
        info!("📝 分析release notes，语言: {}", language);

        let analysis_value = self.analyze_changelog_content(content, language).await?;
        self.convert_to_analysis_result(analysis_value, language).await
    }

    /// 使用AI进行分析
    async fn analyze_with_ai(&self, content: &str, language: &str, client: &OpenAIClient<OpenAIConfig>) -> Result<Value> {
        debug!("🤖 开始AI分析内容");
        
        let system_prompt = self.create_system_prompt(language);
        let user_prompt = self.create_user_prompt(content);
        
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .messages([
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(&system_prompt)
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(user_prompt.clone())
                    .build()?
                    .into(),
            ])
            .max_tokens(self.max_tokens)
            .temperature(self.temperature)
            .build()?;

        let response = client.chat().create(request).await
            .map_err(|e| anyhow!("AI分析API调用失败: {}", e))?;

        let content = response.choices.first()
            .and_then(|choice| choice.message.content.as_ref())
            .ok_or_else(|| anyhow!("AI响应为空"))?;

        // 解析JSON响应
        serde_json::from_str(content)
            .map_err(|e| anyhow!("解析AI响应JSON失败: {}", e))
    }

    /// 创建系统提示词
    fn create_system_prompt(&self, language: &str) -> String {
        format!(r#"你是一个专业的编程语言版本变更分析专家，专门分析{}编程语言的changelog和release notes。

请分析提供的内容，并以JSON格式返回结构化的分析结果，包含以下字段：

{{
  "versions": [
    {{
      "version": "版本号",
      "release_date": "发布日期（如果可用）",
      "is_stable": true/false,
      "is_lts": true/false
    }}
  ],
  "features": [
    {{
      "name": "特性名称",
      "description": "特性描述",
      "category": "特性类别",
      "impact": "影响程度",
      "examples": ["代码示例"]
    }}
  ],
  "syntax_changes": [
    {{
      "description": "语法变化描述",
      "change_type": "变化类型",
      "old_syntax": "旧语法（如果可用）",
      "new_syntax": "新语法（如果可用）"
    }}
  ],
  "breaking_changes": [
    {{
      "description": "破坏性变更描述",
      "affected_features": ["影响的功能"],
      "migration_guide": "迁移指南"
    }}
  ],
  "deprecations": [
    {{
      "feature_name": "弃用功能名称",
      "reason": "弃用原因",
      "replacement": "替代方案",
      "removal_version": "移除版本"
    }}
  ],
  "performance_improvements": [
    {{
      "description": "性能改进描述",
      "improvement_percentage": 数值或null,
      "affected_operations": ["影响的操作"]
    }}
  ]
}}

特性类别应该是以下之一：
- Syntax: 语法特性
- StandardLibrary: 标准库
- TypeSystem: 类型系统
- Async: 异步编程
- Memory: 内存管理
- ErrorHandling: 错误处理
- Modules: 模块系统
- Macros: 宏系统
- Toolchain: 工具链
- Performance: 性能优化
- Security: 安全特性

影响程度应该是：High, Medium, Low, Internal 之一
变化类型应该是：Addition, Modification, Removal, Enhancement 之一"#, language)
    }

    /// 创建用户提示词
    fn create_user_prompt(&self, content: &str) -> String {
        format!(r#"请分析以下changelog或release notes内容，提取版本信息、新特性、语法变化、破坏性变更、弃用信息和性能改进：

```
{}
```

请返回标准的JSON格式分析结果。"#, content)
    }

    /// 使用模式匹配进行备用分析
    async fn analyze_with_patterns(&self, content: &str, language: &str) -> Result<Value> {
        debug!("🔍 使用模式匹配分析内容");

        let mut analysis = json!({
            "analysis_type": "pattern_matching",
            "language": language,
            "versions": [],
            "features": [],
            "syntax_changes": [],
            "breaking_changes": [],
            "deprecations": [],
            "performance_improvements": []
        });

        // 提取版本信息
        let versions = self.extract_versions_with_patterns(content);
        analysis["versions"] = json!(versions);

        // 提取特性
        let features = self.extract_features_with_patterns(content, language);
        analysis["features"] = json!(features);

        // 提取破坏性变更
        let breaking_changes = self.extract_breaking_changes_with_patterns(content);
        analysis["breaking_changes"] = json!(breaking_changes);

        // 提取弃用信息
        let deprecations = self.extract_deprecations_with_patterns(content);
        analysis["deprecations"] = json!(deprecations);

        // 提取语法变化
        let syntax_changes = self.extract_syntax_changes_with_patterns(content);
        analysis["syntax_changes"] = json!(syntax_changes);

        Ok(analysis)
    }

    /// 使用模式提取版本信息
    fn extract_versions_with_patterns(&self, content: &str) -> Vec<Value> {
        let mut versions = Vec::new();
        
        for pattern in &self.fallback_patterns.version_patterns {
            for capture in pattern.captures_iter(content) {
                if let Some(version_match) = capture.get(1) {
                    let version = version_match.as_str();
                    versions.push(json!({
                        "version": version,
                        "release_date": null,
                        "is_stable": true,
                        "is_lts": false,
                        "source": "pattern_extraction"
                    }));
                }
            }
        }

        // 去重
        let mut seen = std::collections::HashSet::new();
        versions.retain(|v| {
            if let Some(version) = v.get("version").and_then(|v| v.as_str()) {
                seen.insert(version.to_string())
            } else {
                false
            }
        });

        versions
    }

    /// 使用模式提取特性
    fn extract_features_with_patterns(&self, content: &str, language: &str) -> Vec<Value> {
        let mut features = Vec::new();
        
        for pattern in &self.fallback_patterns.feature_patterns {
            for capture in pattern.captures_iter(content) {
                if let Some(feature_match) = capture.get(1) {
                    let description = feature_match.as_str().trim();
                    if description.len() > 5 { // 过滤太短的描述
                        features.push(json!({
                            "name": self.extract_feature_name(description),
                            "description": description,
                            "category": self.categorize_feature_pattern(description, language),
                            "impact": "Medium",
                            "examples": [],
                            "source": "pattern_extraction"
                        }));
                    }
                }
            }
        }

        features
    }

    /// 使用模式提取破坏性变更
    fn extract_breaking_changes_with_patterns(&self, content: &str) -> Vec<Value> {
        let mut breaking_changes = Vec::new();
        
        for pattern in &self.fallback_patterns.breaking_change_patterns {
            for capture in pattern.captures_iter(content) {
                let description = if let Some(desc_match) = capture.get(1) {
                    desc_match.as_str().trim()
                } else {
                    capture.get(0).unwrap().as_str().trim()
                };
                
                if description.len() > 5 {
                    breaking_changes.push(json!({
                        "description": description,
                        "affected_features": [],
                        "migration_guide": "",
                        "automation_available": false,
                        "source": "pattern_extraction"
                    }));
                }
            }
        }

        breaking_changes
    }

    /// 使用模式提取弃用信息
    fn extract_deprecations_with_patterns(&self, content: &str) -> Vec<Value> {
        let mut deprecations = Vec::new();
        
        for pattern in &self.fallback_patterns.deprecation_patterns {
            for capture in pattern.captures_iter(content) {
                let description = if let Some(desc_match) = capture.get(1) {
                    desc_match.as_str().trim()
                } else {
                    capture.get(0).unwrap().as_str().trim()
                };
                
                if description.len() > 5 {
                    deprecations.push(json!({
                        "feature_name": self.extract_feature_name(description),
                        "reason": description,
                        "replacement": null,
                        "removal_version": null,
                        "warning_level": "Hard",
                        "source": "pattern_extraction"
                    }));
                }
            }
        }

        deprecations
    }

    /// 使用模式提取语法变化
    fn extract_syntax_changes_with_patterns(&self, content: &str) -> Vec<Value> {
        let mut syntax_changes = Vec::new();
        
        for pattern in &self.fallback_patterns.syntax_change_patterns {
            for capture in pattern.captures_iter(content) {
                let description = if let Some(desc_match) = capture.get(1) {
                    desc_match.as_str().trim()
                } else {
                    capture.get(0).unwrap().as_str().trim()
                };
                
                if description.len() > 5 {
                    syntax_changes.push(json!({
                        "change_type": "Modification",
                        "description": description,
                        "old_syntax": null,
                        "new_syntax": null,
                        "migration_guide": null,
                        "source": "pattern_extraction"
                    }));
                }
            }
        }

        syntax_changes
    }

    /// 提取特性名称
    fn extract_feature_name(&self, description: &str) -> String {
        // 简单的特性名称提取逻辑
        description.split(':').next()
            .unwrap_or(description)
            .split('(').next()
            .unwrap_or(description)
            .split('[').next()
            .unwrap_or(description)
            .trim()
            .to_string()
    }

    /// 根据模式分类特性
    fn categorize_feature_pattern(&self, description: &str, language: &str) -> String {
        let desc_lower = description.to_lowercase();
        
        // 语言特定的分类规则
        match language {
            "rust" => {
                if desc_lower.contains("async") || desc_lower.contains("await") {
                    "Async".to_string()
                } else if desc_lower.contains("trait") || desc_lower.contains("type") {
                    "TypeSystem".to_string()
                } else if desc_lower.contains("macro") {
                    "Macros".to_string()
                } else if desc_lower.contains("std") || desc_lower.contains("library") {
                    "StandardLibrary".to_string()
                } else if desc_lower.contains("cargo") {
                    "Toolchain".to_string()
                } else {
                    "Syntax".to_string()
                }
            }
            "python" => {
                if desc_lower.contains("async") || desc_lower.contains("await") {
                    "Async".to_string()
                } else if desc_lower.contains("typing") || desc_lower.contains("type") {
                    "TypeSystem".to_string()
                } else if desc_lower.contains("import") || desc_lower.contains("module") {
                    "Modules".to_string()
                } else {
                    "StandardLibrary".to_string()
                }
            }
            "javascript" => {
                if desc_lower.contains("async") || desc_lower.contains("promise") {
                    "Async".to_string()
                } else if desc_lower.contains("class") || desc_lower.contains("prototype") {
                    "Syntax".to_string()
                } else if desc_lower.contains("module") || desc_lower.contains("import") {
                    "Modules".to_string()
                } else {
                    "StandardLibrary".to_string()
                }
            }
            _ => "Other".to_string(),
        }
    }

    /// 转换分析结果为ChangelogAnalysisResult
    async fn convert_to_analysis_result(&self, analysis: Value, language: &str) -> Result<ChangelogAnalysisResult> {
        let mut result = ChangelogAnalysisResult::default();

        // 转换特性
        if let Some(features_array) = analysis.get("features").and_then(|v| v.as_array()) {
            for feature_value in features_array {
                if let Ok(feature) = self.parse_language_feature(feature_value, language) {
                    result.features.push(feature);
                }
            }
        }

        // 转换语法变化
        if let Some(syntax_changes_array) = analysis.get("syntax_changes").and_then(|v| v.as_array()) {
            for change_value in syntax_changes_array {
                if let Ok(syntax_change) = self.parse_syntax_change(change_value) {
                    result.syntax_changes.push(syntax_change);
                }
            }
        }

        // 转换弃用信息
        if let Some(deprecations_array) = analysis.get("deprecations").and_then(|v| v.as_array()) {
            for dep_value in deprecations_array {
                if let Ok(deprecation) = self.parse_deprecation(dep_value) {
                    result.deprecations.push(deprecation);
                }
            }
        }

        // 转换破坏性变更
        if let Some(breaking_changes_array) = analysis.get("breaking_changes").and_then(|v| v.as_array()) {
            for change_value in breaking_changes_array {
                if let Ok(breaking_change) = self.parse_breaking_change(change_value) {
                    result.breaking_changes.push(breaking_change);
                }
            }
        }

        Ok(result)
    }

    /// 解析语言特性
    fn parse_language_feature(&self, value: &Value, language: &str) -> Result<LanguageFeature> {
        let examples = self.parse_feature_examples(value, language);
        
        Ok(LanguageFeature {
            name: value.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            description: value.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            category: self.parse_feature_category(value.get("category").and_then(|v| v.as_str()).unwrap_or("Other")),
            examples,
            proposal_link: value.get("proposal_link").and_then(|v| v.as_str()).map(|s| s.to_string()),
            documentation_link: value.get("documentation_link").and_then(|v| v.as_str()).map(|s| s.to_string()),
            stability: FeatureStability::Stable,
            tags: self.parse_feature_tags(value),
            impact: self.parse_impact_level(value.get("impact").and_then(|v| v.as_str()).unwrap_or("Medium")),
        })
    }

    /// 解析特性示例
    fn parse_feature_examples(&self, value: &Value, _language: &str) -> Vec<CodeExample> {
        // 从 examples 数组中解析
        if let Some(examples_array) = value.get("examples").and_then(|v| v.as_array()) {
            return examples_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| CodeExample {
                    title: "示例".to_string(),
                    code: s.to_string(),
                    description: None,
                    requirements: None,
                }))
                .collect();
        }
        
        // 从描述中提取代码块
        if let Some(description) = value.get("description").and_then(|v| v.as_str()) {
            return self.extract_code_examples_from_text(description);
        }
        
        Vec::new()
    }

    /// 从文本中提取代码示例
    fn extract_code_examples_from_text(&self, text: &str) -> Vec<CodeExample> {
        let mut examples = Vec::new();
        
        // 提取markdown代码块
        let code_block_regex = regex::Regex::new(r"```[\w]*\n(.*?)\n```").unwrap();
        for cap in code_block_regex.captures_iter(text) {
            if let Some(code) = cap.get(1) {
                examples.push(CodeExample {
                    title: "代码块示例".to_string(),
                    code: code.as_str().trim().to_string(),
                    description: None,
                    requirements: None,
                });
            }
        }
        
        // 提取内联代码
        let inline_code_regex = regex::Regex::new(r"`([^`]+)`").unwrap();
        for cap in inline_code_regex.captures_iter(text) {
            if let Some(code) = cap.get(1) {
                let code_str = code.as_str().trim();
                // 过滤掉太短的内联代码（可能是变量名）
                if code_str.len() > 10 && (code_str.contains("(") || code_str.contains("=") || code_str.contains(":")) {
                    examples.push(CodeExample {
                        title: "内联代码示例".to_string(),
                        code: code_str.to_string(),
                        description: None,
                        requirements: None,
                    });
                }
            }
        }
        
        examples
    }

    /// 解析特性标签
    fn parse_feature_tags(&self, value: &Value) -> Vec<String> {
        if let Some(tags_array) = value.get("tags").and_then(|v| v.as_array()) {
            return tags_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        
        // 从其他字段推断标签
        let mut tags = Vec::new();
        
        if let Some(category) = value.get("category").and_then(|v| v.as_str()) {
            tags.push(category.to_lowercase());
        }
        
        if let Some(impact) = value.get("impact").and_then(|v| v.as_str()) {
            tags.push(format!("impact-{}", impact.to_lowercase()));
        }
        
        tags
    }

    /// 解析特性类别
    fn parse_feature_category(&self, category_str: &str) -> FeatureCategory {
        match category_str {
            "Syntax" => FeatureCategory::Syntax,
            "StandardLibrary" => FeatureCategory::StandardLibrary,
            "TypeSystem" => FeatureCategory::TypeSystem,
            "Async" => FeatureCategory::Async,
            "Memory" => FeatureCategory::Memory,
            "ErrorHandling" => FeatureCategory::ErrorHandling,
            "Modules" => FeatureCategory::Modules,
            "Macros" => FeatureCategory::Macros,
            "Toolchain" => FeatureCategory::Toolchain,
            "Performance" => FeatureCategory::Performance,
            "Security" => FeatureCategory::Security,
            _ => FeatureCategory::Other(category_str.to_string()),
        }
    }

    /// 解析影响级别
    fn parse_impact_level(&self, impact_str: &str) -> ImpactLevel {
        match impact_str {
            "High" => ImpactLevel::High,
            "Medium" => ImpactLevel::Medium,
            "Low" => ImpactLevel::Low,
            "Internal" => ImpactLevel::Internal,
            _ => ImpactLevel::Medium,
        }
    }

    /// 解析语法变化
    fn parse_syntax_change(&self, value: &Value) -> Result<SyntaxChange> {
        Ok(SyntaxChange {
            change_type: self.parse_syntax_change_type(
                value.get("change_type").and_then(|v| v.as_str()).unwrap_or("Modification")
            ),
            description: value.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            old_syntax: value.get("old_syntax").and_then(|v| v.as_str()).map(|s| s.to_string()),
            new_syntax: value.get("new_syntax").and_then(|v| v.as_str()).map(|s| s.to_string()),
            migration_guide: value.get("migration_guide").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    /// 解析语法变化类型
    fn parse_syntax_change_type(&self, type_str: &str) -> SyntaxChangeType {
        match type_str {
            "Addition" => SyntaxChangeType::Addition,
            "Modification" => SyntaxChangeType::Modification,
            "Removal" => SyntaxChangeType::Removal,
            "Enhancement" => SyntaxChangeType::Enhancement,
            _ => SyntaxChangeType::Modification,
        }
    }

    /// 解析弃用信息
    fn parse_deprecation(&self, value: &Value) -> Result<Deprecation> {
        Ok(Deprecation {
            feature_name: value.get("feature_name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            reason: value.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            replacement: value.get("replacement").and_then(|v| v.as_str()).map(|s| s.to_string()),
            removal_version: value.get("removal_version").and_then(|v| v.as_str()).map(|s| s.to_string()),
            warning_level: DeprecationLevel::Hard, // 默认值
        })
    }

    /// 解析破坏性变更
    fn parse_breaking_change(&self, value: &Value) -> Result<BreakingChange> {
        Ok(BreakingChange {
            description: value.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            affected_features: value.get("affected_features")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            migration_guide: value.get("migration_guide").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            automation_available: value.get("automation_available").and_then(|v| v.as_bool()).unwrap_or(false),
        })
    }

    /// 获取分析器统计信息
    pub fn get_analyzer_stats(&self) -> AnalyzerStats {
        AnalyzerStats {
            has_ai_capability: self.openai_client.is_some(),
            model_name: self.model_name.clone(),
            fallback_patterns_count: self.fallback_patterns.feature_patterns.len(),
            supported_languages: vec!["rust", "python", "javascript", "java", "go"].iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// 分析器统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalyzerStats {
    pub has_ai_capability: bool,
    pub model_name: String,
    pub fallback_patterns_count: usize,
    pub supported_languages: Vec<String>,
} 