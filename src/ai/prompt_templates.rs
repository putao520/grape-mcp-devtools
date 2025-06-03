use std::collections::HashMap;
use crate::tools::environment_detector::LanguageInfo;
use super::predicate_ai::NaturalLanguagePredicate;
use super::url_ai::SemanticUrlResult;

/// 文档处理AI提示词模板
#[derive(Clone)]
pub struct DocumentPrompts;

impl DocumentPrompts {
    pub fn new() -> Self {
        Self
    }

    /// 获取内容提取系统提示词
    pub fn get_extraction_system_prompt(&self) -> String {
        r#"你是一个专业的文档内容提取和分析专家。你的任务是从HTML内容中智能提取结构化信息。

你需要：
1. 提取文档标题和主要内容
2. 识别代码示例和编程语言
3. 提取API文档信息
4. 识别教程步骤
5. 评估内容质量和相关性

请以JSON格式返回结果，包含以下字段：
- title: 文档标题
- main_content: 主要内容（清理后的文本）
- code_examples: 代码示例数组，每个包含language、code、description、is_runnable
- api_documentation: API文档数组，每个包含name、description、return_type
- quality_score: 内容质量分数(0.0-1.0)
- relevance_score: 相关性分数(0.0-1.0)
- confidence: 提取置信度(0.0-1.0)

保持专业、准确、简洁。"#.to_string()
    }

    /// 获取内容提取用户提示词
    pub fn get_extraction_user_prompt(&self, content: &str, target_language: &str, query: &str) -> String {
        format!(r#"请分析以下HTML内容，目标编程语言是{}，查询上下文是"{}"：

内容：
{}

请提取结构化信息并评估质量。"#, target_language, query, content)
    }

    /// 获取语义分析系统提示词
    pub fn get_semantic_analysis_system_prompt(&self) -> String {
        r#"你是一个语义分析专家，专门分析技术文档的语义内容。

你需要：
1. 识别主题标签和关键概念
2. 评估难度级别
3. 确定目标受众
4. 生成内容摘要
5. 计算与查询的语义相似度

请以JSON格式返回结果，包含：
- topics: 主题标签数组
- key_concepts: 关键概念数组
- difficulty_level: 难度级别(1-5)
- target_audience: 目标受众数组
- summary: 内容摘要
- semantic_similarity: 语义相似度(0.0-1.0)"#.to_string()
    }

    /// 获取语义分析用户提示词
    pub fn get_semantic_analysis_user_prompt(&self, content: &str, target_language: &str, query: &str) -> String {
        format!(r#"请对以下{}技术内容进行语义分析，查询上下文是"{}"：

内容：
{}

请分析语义信息。"#, target_language, query, content)
    }

    /// 获取质量评估系统提示词
    pub fn get_quality_assessment_system_prompt(&self) -> String {
        r#"你是一个内容质量评估专家，专门评估技术文档的质量。

评估维度：
1. 完整性 - 信息是否完整
2. 准确性 - 内容是否准确
3. 可读性 - 是否易于理解
4. 实用性 - 是否有实际价值
5. 时效性 - 内容是否最新

请以JSON格式返回结果，包含：
- overall_score: 整体质量分数(0.0-1.0)
- completeness_score: 完整性分数
- accuracy_score: 准确性分数
- readability_score: 可读性分数
- usefulness_score: 实用性分数
- freshness_score: 时效性分数
- improvement_suggestions: 改进建议数组"#.to_string()
    }

    /// 获取质量评估用户提示词
    pub fn get_quality_assessment_user_prompt(&self, content: &str, content_type: &str) -> String {
        format!(r#"请评估以下{}类型内容的质量：

内容：
{}

请提供详细的质量评估。"#, content_type, content)
    }

    /// 获取翻译系统提示词
    pub fn get_translation_system_prompt(&self) -> String {
        r#"你是一个专业的技术文档翻译专家。请保持技术术语的准确性，确保翻译的专业性和可读性。

翻译要求：
1. 保持技术术语的原文或使用标准译名
2. 保持代码示例不变
3. 保持文档结构
4. 确保翻译的自然流畅"#.to_string()
    }

    /// 获取翻译用户提示词
    pub fn get_translation_user_prompt(&self, content: &str, target_language: &str) -> String {
        format!(r#"请将以下技术内容翻译成{}：

内容：
{}

请提供专业的翻译。"#, target_language, content)
    }

    /// 获取摘要系统提示词
    pub fn get_summary_system_prompt(&self) -> String {
        r#"你是一个专业的技术文档摘要专家。请生成简洁、准确、有用的摘要。

摘要要求：
1. 突出关键信息
2. 保持技术准确性
3. 控制长度
4. 易于理解"#.to_string()
    }

    /// 获取摘要用户提示词
    pub fn get_summary_user_prompt(&self, content: &str, max_length: usize) -> String {
        format!(r#"请为以下技术内容生成摘要，最大长度{}字符：

内容：
{}

请生成简洁的摘要。"#, max_length, content)
    }
}

/// 谓词处理AI提示词模板
pub struct PredicatePrompts;

impl PredicatePrompts {
    pub fn new() -> Self {
        Self
    }

    /// 获取谓词解析系统提示词
    pub fn get_parsing_system_prompt(&self) -> String {
        r#"你是一个自然语言谓词解析专家。你需要将自然语言描述的条件转换为结构化的谓词。

支持的条件类型：
- has_file: 项目包含特定文件
- uses_framework: 项目使用特定框架
- has_dependency: 项目有特定依赖
- code_quality: 代码质量分数
- project_size: 项目规模
- language_version: 语言版本
- custom: 自定义条件

支持的操作符：
- equal, not_equal, greater_than, less_than, greater_or_equal, less_or_equal
- contains, not_contains, matches

支持的逻辑操作符：
- and, or, not, xor

请以JSON格式返回解析结果，包含：
- conditions: 条件数组，每个包含type、parameters、expected_value、operator
- logic_operator: 逻辑操作符"#.to_string()
    }

    /// 获取谓词解析用户提示词
    pub fn get_parsing_user_prompt(&self, predicate_text: &str) -> String {
        format!(r#"请解析以下自然语言谓词：

"{}"

请将其转换为结构化的条件表达式。"#, predicate_text)
    }

    /// 获取谓词评估系统提示词
    pub fn get_evaluation_system_prompt(&self) -> String {
        r#"你是一个谓词评估专家。根据项目信息和解析后的谓词条件，评估谓词是否为真。

评估过程：
1. 分析每个条件
2. 查找相关证据
3. 应用逻辑操作符
4. 计算置信度
5. 提供解释

请以JSON格式返回结果，包含：
- result: 评估结果(true/false)
- confidence: 置信度(0.0-1.0)
- explanation: 解释说明
- conditions: 条件评估数组，每个包含condition、result、confidence、evidence
- reasoning_steps: 推理步骤数组"#.to_string()
    }

    /// 获取谓词评估用户提示词
    pub fn get_evaluation_user_prompt(&self, predicate: &NaturalLanguagePredicate, language: &str, info: &LanguageInfo) -> String {
        format!(r#"请评估以下谓词条件，项目语言是{}：

谓词：{}

项目信息：
- 项目文件：{:?}
- 检测到的特性：{:?}
- CLI工具：{:?}

请进行详细评估。"#, 
            language, 
            predicate.text,
            info.project_files.iter().take(10).collect::<Vec<_>>(),
            info.detected_features.iter().take(10).collect::<Vec<_>>(),
            info.cli_tools.iter().take(5).map(|t| &t.name).collect::<Vec<_>>()
        )
    }

    /// 获取推理系统提示词
    pub fn get_reasoning_system_prompt(&self) -> String {
        r#"你是一个智能推理专家。基于给定的条件和上下文，进行逻辑推理并得出结论。

推理要求：
1. 逻辑清晰
2. 证据充分
3. 结论可靠
4. 解释详细

请以JSON格式返回推理结果。"#.to_string()
    }

    /// 获取推理用户提示词
    pub fn get_reasoning_user_prompt(&self, conditions: &[String], context: &str, language: &str, info: &LanguageInfo) -> String {
        format!(r#"请对以下条件进行智能推理：

条件：{:?}
上下文：{}
语言：{}

项目信息：
- 文件数量：{}
- 特性数量：{}
- 工具数量：{}

请进行推理分析。"#, 
            conditions, context, language,
            info.project_files.len(),
            info.detected_features.len(),
            info.cli_tools.len()
        )
    }

    /// 获取建议系统提示词
    pub fn get_suggestion_system_prompt(&self) -> String {
        r#"你是一个谓词建议专家。基于项目信息和使用场景，建议合适的谓词条件。

建议要求：
1. 实用性强
2. 针对性好
3. 易于理解
4. 覆盖常见场景

请以JSON格式返回建议，包含suggestions数组。"#.to_string()
    }

    /// 获取建议用户提示词
    pub fn get_suggestion_user_prompt(&self, language: &str, info: &LanguageInfo, use_case: &str) -> String {
        format!(r#"请为{}项目建议合适的谓词条件，使用场景是"{}"：

项目特征：
- 主要文件类型：{:?}
- 检测到的框架：{:?}
- 可用工具：{:?}

请提供实用的谓词建议。"#, 
            language, use_case,
            info.project_files.iter().take(5).collect::<Vec<_>>(),
            info.detected_features.iter().take(5).collect::<Vec<_>>(),
            info.cli_tools.iter().take(3).map(|t| &t.name).collect::<Vec<_>>()
        )
    }
}

/// URL分析AI提示词模板
pub struct UrlPrompts;

impl UrlPrompts {
    pub fn new() -> Self {
        Self
    }

    /// 获取语义理解系统提示词
    pub fn get_semantic_understanding_system_prompt(&self) -> String {
        r#"你是一个URL语义理解专家。分析URL的语义含义，识别内容类型、主题和技术栈。

分析维度：
1. URL类型（文档、教程、API参考等）
2. 主题标签
3. 编程语言
4. 技术栈
5. 内容类别
6. 目标受众
7. 难度级别

请以JSON格式返回结果，包含：
- url_type: URL类型
- topics: 主题标签数组
- programming_languages: 编程语言数组
- tech_stack: 技术栈数组
- content_category: 内容类别
- target_audience: 目标受众数组
- difficulty_level: 难度级别(1-5)"#.to_string()
    }

    /// 获取语义理解用户提示词
    pub fn get_semantic_understanding_user_prompt(&self, url: &str, target_language: &str, query_context: &str) -> String {
        format!(r#"请分析以下URL的语义含义：

URL: {}
目标语言: {}
查询上下文: {}

请提供详细的语义分析。"#, url, target_language, query_context)
    }

    /// 获取内容预测系统提示词
    pub fn get_content_prediction_system_prompt(&self) -> String {
        r#"你是一个内容预测专家。基于URL和语义分析结果，预测页面可能包含的内容。

预测维度：
1. 内容类型和置信度
2. 预期质量
3. 预期有用性
4. 预期时效性
5. 可能包含的信息
6. 潜在问题

请以JSON格式返回预测结果。"#.to_string()
    }

    /// 获取内容预测用户提示词
    pub fn get_content_prediction_user_prompt(&self, url: &str, target_language: &str, semantic_result: &SemanticUrlResult) -> String {
        format!(r#"请预测以下URL的内容：

URL: {}
目标语言: {}
语义分析结果: {:?}

请提供内容预测。"#, url, target_language, semantic_result)
    }

    /// 获取质量评估系统提示词
    pub fn get_quality_assessment_system_prompt(&self) -> String {
        r#"你是一个URL质量评估专家。评估URL的质量和可信度。

评估维度：
1. 域名权威性
2. URL结构质量
3. 语言一致性
4. 可信度
5. 质量指标
6. 风险因素

请以JSON格式返回评估结果。"#.to_string()
    }

    /// 获取质量评估用户提示词
    pub fn get_quality_assessment_user_prompt(&self, url: &str, target_language: &str, semantic_result: &SemanticUrlResult) -> String {
        format!(r#"请评估以下URL的质量：

URL: {}
目标语言: {}
语义分析: {:?}

请提供质量评估。"#, url, target_language, semantic_result)
    }

    /// 获取比较系统提示词
    pub fn get_comparison_system_prompt(&self) -> String {
        r#"你是一个URL比较专家。比较多个URL的质量、相关性和实用性。

比较维度：
1. 语义相似度
2. 内容相似度
3. 质量差异
4. 推荐选择
5. 详细解释

请以JSON格式返回比较结果。"#.to_string()
    }

    /// 获取比较用户提示词
    pub fn get_comparison_user_prompt(&self, urls: &[String], target_language: &str, query_context: &str) -> String {
        format!(r#"请比较以下URL：

URLs: {:?}
目标语言: {}
查询上下文: {}

请提供详细比较。"#, urls, target_language, query_context)
    }

    /// 获取建议系统提示词
    pub fn get_suggestion_system_prompt(&self) -> String {
        r#"你是一个URL建议专家。基于查询和偏好，建议相关的URL。

建议要求：
1. 高度相关
2. 质量可靠
3. 实用性强
4. 多样性好

请以JSON格式返回建议，包含suggestions数组。"#.to_string()
    }

    /// 获取建议用户提示词
    pub fn get_suggestion_user_prompt(&self, query: &str, target_language: &str, preferences: &HashMap<String, String>) -> String {
        format!(r#"请为以下查询建议相关URL：

查询: {}
目标语言: {}
偏好设置: {:?}

请提供URL建议。"#, query, target_language, preferences)
    }
} 