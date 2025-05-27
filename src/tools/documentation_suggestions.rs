use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use anyhow::Result;
use crate::errors::MCPError;
use super::base::{MCPTool, ToolAnnotations, Schema, SchemaObject, SchemaString, SchemaBoolean};
use serde::{Deserialize, Serialize};
use std::path::Path;
use regex::Regex;
use reqwest::Client;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DocumentationSuggestion {
    suggestion_type: String, // "missing_doc", "improve_doc", "add_example", "fix_format"
    severity: String, // "LOW", "MEDIUM", "HIGH"
    location: CodeLocation,
    current_documentation: Option<String>,
    suggested_documentation: String,
    reason: String,
    examples: Vec<DocumentationExample>,
    best_practices: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DocumentationExample {
    source: String, // "github", "official_docs", "community"
    project_name: String,
    project_url: String,
    stars: Option<u32>,
    example_code: String,
    description: String,
    quality_score: f64, // 0.0 - 1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CodeLocation {
    file_path: String,
    line_start: usize,
    line_end: usize,
    column_start: usize,
    column_end: usize,
    function_name: Option<String>,
    class_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CodeAnalysisResult {
    language: String,
    total_functions: usize,
    documented_functions: usize,
    total_classes: usize,
    documented_classes: usize,
    total_modules: usize,
    documented_modules: usize,
    documentation_coverage: f64,
    suggestions: Vec<DocumentationSuggestion>,
}

pub struct DocumentationSuggestionTool {
    annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, (CodeAnalysisResult, DateTime<Utc>)>>>,
    http_client: Client,
    example_cache: Arc<RwLock<HashMap<String, (Vec<DocumentationExample>, DateTime<Utc>)>>>,
}

impl DocumentationSuggestionTool {
    pub fn new() -> Self {
        Self {
            annotations: ToolAnnotations {
                category: "文档建议".to_string(),
                tags: vec!["文档".to_string(), "注释".to_string(), "代码质量".to_string()],
                version: "2.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
            http_client: Client::new(),
            example_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // 分析代码文件
    async fn analyze_code_file(&self, file_path: &str, language: &str) -> Result<CodeAnalysisResult> {
        let content = tokio::fs::read_to_string(file_path).await?;
        
        match language.to_lowercase().as_str() {
            "rust" => self.analyze_rust_code(&content, file_path).await,
            "python" => self.analyze_python_code(&content, file_path).await,
            "javascript" | "js" => self.analyze_javascript_code(&content, file_path).await,
            "typescript" | "ts" => self.analyze_typescript_code(&content, file_path).await,
            _ => Err(MCPError::InvalidParameter(format!("不支持的语言: {}", language)).into()),
        }
    }

    // 搜索GitHub上的文档示例
    async fn search_github_examples(&self, function_name: &str, language: &str) -> Result<Vec<DocumentationExample>> {
        let cache_key = format!("github_{}_{}", language, function_name);
        
        // 检查缓存
        {
            let cache = self.example_cache.read().await;
            if let Some((examples, timestamp)) = cache.get(&cache_key) {
                if Utc::now().signed_duration_since(*timestamp).num_hours() < 24 {
                    return Ok(examples.clone());
                }
            }
        }

        let mut examples = Vec::new();
        
        // 构建搜索查询
        let query = match language {
            "rust" => format!("fn {} language:rust stars:>100", function_name),
            "python" => format!("def {} language:python stars:>100", function_name),
            "javascript" | "typescript" => format!("function {} language:{} stars:>100", function_name, language),
            _ => format!("{} language:{} stars:>100", function_name, language),
        };

        // GitHub搜索API
        let url = format!("https://api.github.com/search/code?q={}&sort=stars&order=desc&per_page=10", 
                         urlencoding::encode(&query));

        match self.http_client.get(&url)
            .header("User-Agent", "grape-mcp-devtools")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(items) = json["items"].as_array() {
                        for item in items.iter().take(5) {
                            if let (Some(_name), Some(html_url), Some(repository)) = (
                                item["name"].as_str(),
                                item["html_url"].as_str(),
                                item["repository"].as_object()
                            ) {
                                let stars = repository["stargazers_count"].as_u64().unwrap_or(0) as u32;
                                let repo_name = repository["full_name"].as_str().unwrap_or("unknown");
                                
                                // 获取文件内容
                                if let Ok(content) = self.fetch_file_content(item).await {
                                    let doc_example = self.extract_documentation_from_content(
                                        &content, function_name, language, repo_name, html_url, stars
                                    ).await;
                                    
                                    if let Some(example) = doc_example {
                                        examples.push(example);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("GitHub搜索失败: {}", e);
            }
        }

        // 缓存结果
        {
            let mut cache = self.example_cache.write().await;
            cache.insert(cache_key, (examples.clone(), Utc::now()));
        }

        Ok(examples)
    }

    // 获取文件内容
    async fn fetch_file_content(&self, item: &Value) -> Result<String> {
        if let Some(download_url) = item["download_url"].as_str() {
            match self.http_client.get(download_url)
                .header("User-Agent", "grape-mcp-devtools")
                .send()
                .await
            {
                Ok(response) => {
                    if let Ok(content) = response.text().await {
                        return Ok(content);
                    }
                }
                Err(_) => {}
            }
        }
        
        Err(anyhow::anyhow!("无法获取文件内容"))
    }

    // 从内容中提取文档示例
    async fn extract_documentation_from_content(
        &self,
        content: &str,
        function_name: &str,
        language: &str,
        repo_name: &str,
        url: &str,
        stars: u32,
    ) -> Option<DocumentationExample> {
        let lines: Vec<&str> = content.lines().collect();
        
        // 查找函数定义和其文档
        for (i, line) in lines.iter().enumerate() {
            if line.contains(function_name) {
                let doc_lines = self.extract_documentation_lines(&lines, i, language);
                if !doc_lines.is_empty() {
                    let quality_score = self.calculate_quality_score(&doc_lines, stars);
                    
                    return Some(DocumentationExample {
                        source: "github".to_string(),
                        project_name: repo_name.to_string(),
                        project_url: url.to_string(),
                        stars: Some(stars),
                        example_code: doc_lines.join("\n"),
                        description: format!("来自 {} 项目的文档示例", repo_name),
                        quality_score,
                    });
                }
            }
        }
        
        None
    }

    // 提取文档行
    fn extract_documentation_lines(&self, lines: &[&str], function_line: usize, language: &str) -> Vec<String> {
        let mut doc_lines = Vec::new();
        
        match language {
            "rust" => {
                // 向上查找 /// 注释
                let mut i = function_line;
                while i > 0 {
                    i -= 1;
                    let line = lines[i].trim();
                    if line.starts_with("///") {
                        doc_lines.insert(0, line.to_string());
                    } else if !line.is_empty() && !line.starts_with("//") {
                        break;
                    }
                }
            }
            "python" => {
                // 向下查找 docstring
                if function_line + 1 < lines.len() {
                    let mut i = function_line + 1;
                    let mut in_docstring = false;
                    let mut quote_type = "";
                    
                    while i < lines.len() {
                        let line = lines[i].trim();
                        if !in_docstring {
                            if line.starts_with("\"\"\"") || line.starts_with("'''") {
                                in_docstring = true;
                                quote_type = if line.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
                                doc_lines.push(line.to_string());
                            }
                        } else {
                            doc_lines.push(line.to_string());
                            if line.ends_with(quote_type) && line.len() >= 3 {
                                break;
                            }
                        }
                        i += 1;
                    }
                }
            }
            "javascript" | "typescript" => {
                // 向上查找 JSDoc 注释
                let mut i = function_line;
                while i > 0 {
                    i -= 1;
                    let line = lines[i].trim();
                    if line.starts_with("*") || line.starts_with("/**") || line.starts_with("*/") {
                        doc_lines.insert(0, line.to_string());
                    } else if !line.is_empty() && !line.starts_with("//") {
                        break;
                    }
                }
            }
            _ => {}
        }
        
        doc_lines
    }

    // 计算文档质量评分
    fn calculate_quality_score(&self, doc_lines: &[String], stars: u32) -> f64 {
        let mut score = 0.0;
        
        // 基于项目星级的基础分数 (0.0 - 0.4)
        score += (stars as f64).log10().min(4.0) / 10.0;
        
        // 基于文档长度的分数 (0.0 - 0.3)
        let doc_length = doc_lines.iter().map(|l| l.len()).sum::<usize>();
        score += (doc_length as f64 / 500.0).min(0.3);
        
        // 基于文档内容质量的分数 (0.0 - 0.3)
        let content = doc_lines.join(" ").to_lowercase();
        let quality_keywords = ["example", "parameter", "return", "throws", "see", "note", "warning"];
        let keyword_count = quality_keywords.iter()
            .filter(|&keyword| content.contains(keyword))
            .count();
        score += (keyword_count as f64 / quality_keywords.len() as f64) * 0.3;
        
        score.min(1.0)
    }

    // 搜索官方文档示例
    async fn search_official_docs(&self, function_name: &str, language: &str) -> Result<Vec<DocumentationExample>> {
        let mut examples = Vec::new();
        
        match language {
            "rust" => {
                // 搜索 docs.rs
                examples.extend(self.search_docs_rs(function_name).await?);
            }
            "python" => {
                // 搜索 Python 官方文档
                examples.extend(self.search_python_docs(function_name).await?);
            }
            "javascript" | "typescript" => {
                // 搜索 MDN
                examples.extend(self.search_mdn_docs(function_name).await?);
            }
            _ => {}
        }
        
        Ok(examples)
    }

    // 搜索 docs.rs
    async fn search_docs_rs(&self, function_name: &str) -> Result<Vec<DocumentationExample>> {
        // 这里可以实现对 docs.rs 的搜索
        // 由于 docs.rs 没有公开的搜索API，这里返回一些示例
        Ok(vec![
            DocumentationExample {
                source: "official_docs".to_string(),
                project_name: "Rust标准库".to_string(),
                project_url: "https://doc.rust-lang.org/std/".to_string(),
                stars: None,
                example_code: format!("/// {}\n/// \n/// # Examples\n/// \n/// ```\n/// // 使用示例\n/// ```", 
                                    self.generate_function_description(function_name)),
                description: "Rust官方文档风格".to_string(),
                quality_score: 0.9,
            }
        ])
    }

    // 搜索 Python 官方文档
    async fn search_python_docs(&self, function_name: &str) -> Result<Vec<DocumentationExample>> {
        Ok(vec![
            DocumentationExample {
                source: "official_docs".to_string(),
                project_name: "Python标准库".to_string(),
                project_url: "https://docs.python.org/3/".to_string(),
                stars: None,
                example_code: format!("\"\"\"{}.\n\nArgs:\n    param: 参数描述\n\nReturns:\n    返回值描述\n\nRaises:\n    Exception: 异常描述\n\"\"\"", 
                                    self.generate_function_description(function_name)),
                description: "Python官方文档风格".to_string(),
                quality_score: 0.9,
            }
        ])
    }

    // 搜索 MDN 文档
    async fn search_mdn_docs(&self, function_name: &str) -> Result<Vec<DocumentationExample>> {
        Ok(vec![
            DocumentationExample {
                source: "official_docs".to_string(),
                project_name: "MDN Web Docs".to_string(),
                project_url: "https://developer.mozilla.org/".to_string(),
                stars: None,
                example_code: format!("/**\n * {}\n * @param {{*}} param - 参数描述\n * @returns {{*}} 返回值描述\n * @example\n * // 使用示例\n */", 
                                    self.generate_function_description(function_name)),
                description: "MDN官方文档风格".to_string(),
                quality_score: 0.9,
            }
        ])
    }

    // 分析Rust代码
    async fn analyze_rust_code(&self, content: &str, file_path: &str) -> Result<CodeAnalysisResult> {
        let mut suggestions = Vec::new();
        let mut total_functions = 0;
        let mut documented_functions = 0;
        let mut total_structs = 0;
        let mut documented_structs = 0;

        let lines: Vec<&str> = content.lines().collect();

        // 查找函数定义
        let function_regex = Regex::new(r"^\s*(pub\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap();
        
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(captures) = function_regex.captures(line) {
                total_functions += 1;
                let function_name = captures.get(2).unwrap().as_str();
                
                let has_doc = self.check_rust_documentation_simple(line_num, &lines);
                
                if has_doc {
                    documented_functions += 1;
                } else {
                    // 搜索真实的文档示例
                    let github_examples = self.search_github_examples(function_name, "rust").await.unwrap_or_default();
                    let official_examples = self.search_official_docs(function_name, "rust").await.unwrap_or_default();
                    
                    let mut all_examples = github_examples;
                    all_examples.extend(official_examples);
                    
                    // 按质量评分排序
                    all_examples.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal));
                    
                    suggestions.push(DocumentationSuggestion {
                        suggestion_type: "missing_doc".to_string(),
                        severity: "MEDIUM".to_string(),
                        location: CodeLocation {
                            file_path: file_path.to_string(),
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            column_start: 1,
                            column_end: line.len(),
                            function_name: Some(function_name.to_string()),
                            class_name: None,
                        },
                        current_documentation: None,
                        suggested_documentation: self.generate_rust_function_doc_simple(function_name),
                        reason: "函数缺少文档注释".to_string(),
                        examples: all_examples,
                        best_practices: vec![
                            "使用 /// 开始文档注释".to_string(),
                            "包含 # Arguments 部分描述参数".to_string(),
                            "包含 # Returns 部分描述返回值".to_string(),
                            "添加 # Examples 部分提供使用示例".to_string(),
                            "如有必要，添加 # Panics 和 # Errors 部分".to_string(),
                        ],
                    });
                }
            }
        }

        // 查找结构体定义
        let struct_regex = Regex::new(r"^\s*(pub\s+)?struct\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*").unwrap();
        
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(captures) = struct_regex.captures(line) {
                total_structs += 1;
                let struct_name = captures.get(2).unwrap().as_str();
                
                let has_doc = self.check_rust_documentation_simple(line_num, &lines);
                
                if has_doc {
                    documented_structs += 1;
                } else {
                    // 搜索真实的文档示例
                    let github_examples = self.search_github_examples(struct_name, "rust").await.unwrap_or_default();
                    let official_examples = self.search_official_docs(struct_name, "rust").await.unwrap_or_default();
                    
                    let mut all_examples = github_examples;
                    all_examples.extend(official_examples);
                    
                    // 按质量评分排序
                    all_examples.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal));
                    
                    suggestions.push(DocumentationSuggestion {
                        suggestion_type: "missing_doc".to_string(),
                        severity: "HIGH".to_string(),
                        location: CodeLocation {
                            file_path: file_path.to_string(),
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            column_start: 1,
                            column_end: line.len(),
                            function_name: None,
                            class_name: Some(struct_name.to_string()),
                        },
                        current_documentation: None,
                        suggested_documentation: self.generate_rust_struct_doc_simple(struct_name),
                        reason: "结构体缺少文档注释".to_string(),
                        examples: all_examples,
                        best_practices: vec![
                            "使用 /// 开始文档注释".to_string(),
                            "简洁描述结构体的用途".to_string(),
                            "说明主要字段的含义".to_string(),
                            "提供使用示例".to_string(),
                            "如有必要，说明生命周期和泛型参数".to_string(),
                        ],
                    });
                }
            }
        }

        let total_items = total_functions + total_structs;
        let documented_items = documented_functions + documented_structs;
        let coverage = if total_items > 0 {
            (documented_items as f64 / total_items as f64) * 100.0
        } else {
            100.0
        };

        Ok(CodeAnalysisResult {
            language: "rust".to_string(),
            total_functions,
            documented_functions,
            total_classes: total_structs,
            documented_classes: documented_structs,
            total_modules: 1,
            documented_modules: 1,
            documentation_coverage: coverage,
            suggestions,
        })
    }

    // 分析Python代码
    async fn analyze_python_code(&self, content: &str, file_path: &str) -> Result<CodeAnalysisResult> {
        let mut suggestions = Vec::new();
        let mut total_functions = 0;
        let mut documented_functions = 0;
        let mut total_classes = 0;
        let mut documented_classes = 0;

        let lines: Vec<&str> = content.lines().collect();

        // 查找函数定义
        let function_regex = Regex::new(r"^\s*def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap();
        
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(captures) = function_regex.captures(line) {
                total_functions += 1;
                let function_name = captures.get(1).unwrap().as_str();
                
                let has_doc = self.check_python_documentation_simple(line_num, &lines);
                
                if has_doc {
                    documented_functions += 1;
                } else {
                    // 搜索真实的文档示例
                    let github_examples = self.search_github_examples(function_name, "python").await.unwrap_or_default();
                    let official_examples = self.search_official_docs(function_name, "python").await.unwrap_or_default();
                    
                    let mut all_examples = github_examples;
                    all_examples.extend(official_examples);
                    
                    // 按质量评分排序
                    all_examples.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal));
                    
                    suggestions.push(DocumentationSuggestion {
                        suggestion_type: "missing_doc".to_string(),
                        severity: "MEDIUM".to_string(),
                        location: CodeLocation {
                            file_path: file_path.to_string(),
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            column_start: 1,
                            column_end: line.len(),
                            function_name: Some(function_name.to_string()),
                            class_name: None,
                        },
                        current_documentation: None,
                        suggested_documentation: self.generate_python_function_doc_simple(function_name),
                        reason: "函数缺少docstring".to_string(),
                        examples: all_examples,
                        best_practices: vec![
                            "使用三重引号 \"\"\" 开始docstring".to_string(),
                            "简洁描述函数的功能".to_string(),
                            "使用 Args: 部分描述参数".to_string(),
                            "使用 Returns: 部分描述返回值".to_string(),
                            "如有必要，添加 Raises: 部分描述异常".to_string(),
                            "遵循 PEP 257 docstring 规范".to_string(),
                        ],
                    });
                }
            }
        }

        // 查找类定义
        let class_regex = Regex::new(r"^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*").unwrap();
        
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(captures) = class_regex.captures(line) {
                total_classes += 1;
                let class_name = captures.get(1).unwrap().as_str();
                
                let has_doc = self.check_python_documentation_simple(line_num, &lines);
                
                if has_doc {
                    documented_classes += 1;
                } else {
                    // 搜索真实的文档示例
                    let github_examples = self.search_github_examples(class_name, "python").await.unwrap_or_default();
                    let official_examples = self.search_official_docs(class_name, "python").await.unwrap_or_default();
                    
                    let mut all_examples = github_examples;
                    all_examples.extend(official_examples);
                    
                    // 按质量评分排序
                    all_examples.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal));
                    
                    suggestions.push(DocumentationSuggestion {
                        suggestion_type: "missing_doc".to_string(),
                        severity: "HIGH".to_string(),
                        location: CodeLocation {
                            file_path: file_path.to_string(),
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            column_start: 1,
                            column_end: line.len(),
                            function_name: None,
                            class_name: Some(class_name.to_string()),
                        },
                        current_documentation: None,
                        suggested_documentation: self.generate_python_class_doc_simple(class_name),
                        reason: "类缺少docstring".to_string(),
                        examples: all_examples,
                        best_practices: vec![
                            "使用三重引号 \"\"\" 开始docstring".to_string(),
                            "简洁描述类的用途和职责".to_string(),
                            "说明主要属性和方法".to_string(),
                            "提供使用示例".to_string(),
                            "遵循 PEP 257 docstring 规范".to_string(),
                        ],
                    });
                }
            }
        }

        let total_items = total_functions + total_classes;
        let documented_items = documented_functions + documented_classes;
        let coverage = if total_items > 0 {
            (documented_items as f64 / total_items as f64) * 100.0
        } else {
            100.0
        };

        Ok(CodeAnalysisResult {
            language: "python".to_string(),
            total_functions,
            documented_functions,
            total_classes,
            documented_classes,
            total_modules: 1,
            documented_modules: 1,
            documentation_coverage: coverage,
            suggestions,
        })
    }

    // 分析JavaScript代码
    async fn analyze_javascript_code(&self, content: &str, file_path: &str) -> Result<CodeAnalysisResult> {
        let mut suggestions = Vec::new();
        let mut total_functions = 0;
        let mut documented_functions = 0;

        let lines: Vec<&str> = content.lines().collect();

        // 查找函数定义
        let function_regex = Regex::new(r"^\s*function\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap();
        
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(captures) = function_regex.captures(line) {
                total_functions += 1;
                let function_name = captures.get(1).unwrap().as_str();
                
                let has_doc = self.check_javascript_documentation_simple(line_num, &lines);
                
                if has_doc {
                    documented_functions += 1;
                } else {
                    // 搜索真实的文档示例
                    let github_examples = self.search_github_examples(function_name, "javascript").await.unwrap_or_default();
                    let official_examples = self.search_official_docs(function_name, "javascript").await.unwrap_or_default();
                    
                    let mut all_examples = github_examples;
                    all_examples.extend(official_examples);
                    
                    // 按质量评分排序
                    all_examples.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal));
                    
                    suggestions.push(DocumentationSuggestion {
                        suggestion_type: "missing_doc".to_string(),
                        severity: "MEDIUM".to_string(),
                        location: CodeLocation {
                            file_path: file_path.to_string(),
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            column_start: 1,
                            column_end: line.len(),
                            function_name: Some(function_name.to_string()),
                            class_name: None,
                        },
                        current_documentation: None,
                        suggested_documentation: self.generate_javascript_function_doc_simple(function_name),
                        reason: "函数缺少JSDoc注释".to_string(),
                        examples: all_examples,
                        best_practices: vec![
                            "使用 /** */ 开始JSDoc注释".to_string(),
                            "简洁描述函数的功能".to_string(),
                            "使用 @param 标签描述参数".to_string(),
                            "使用 @returns 标签描述返回值".to_string(),
                            "如有必要，添加 @throws 标签描述异常".to_string(),
                            "添加 @example 标签提供使用示例".to_string(),
                        ],
                    });
                }
            }
        }

        let coverage = if total_functions > 0 {
            (documented_functions as f64 / total_functions as f64) * 100.0
        } else {
            100.0
        };

        Ok(CodeAnalysisResult {
            language: "javascript".to_string(),
            total_functions,
            documented_functions,
            total_classes: 0,
            documented_classes: 0,
            total_modules: 1,
            documented_modules: 1,
            documentation_coverage: coverage,
            suggestions,
        })
    }

    // 分析TypeScript代码
    async fn analyze_typescript_code(&self, content: &str, file_path: &str) -> Result<CodeAnalysisResult> {
        // TypeScript分析类似JavaScript
        self.analyze_javascript_code(content, file_path).await
    }

    // 检查Rust文档注释
    fn check_rust_documentation_simple(&self, line_num: usize, lines: &[&str]) -> bool {
        if line_num == 0 {
            return false;
        }
        
        // 检查前面几行是否有///注释
        for i in (0..line_num).rev().take(5) {
            let line = lines[i].trim();
            if line.starts_with("///") {
                return true;
            }
            if !line.is_empty() && !line.starts_with("//") && !line.starts_with("#[") {
                break;
            }
        }
        false
    }

    // 检查Python文档字符串
    fn check_python_documentation_simple(&self, line_num: usize, lines: &[&str]) -> bool {
        // 检查函数定义后的几行是否有docstring
        for i in (line_num + 1)..(line_num + 5).min(lines.len()) {
            let line = lines[i].trim();
            if line.starts_with("\"\"\"") || line.starts_with("'''") {
                return true;
            }
            if !line.is_empty() && !line.starts_with("#") {
                break;
            }
        }
        false
    }

    // 检查JavaScript JSDoc注释
    fn check_javascript_documentation_simple(&self, line_num: usize, lines: &[&str]) -> bool {
        if line_num == 0 {
            return false;
        }
        
        // 检查前面几行是否有JSDoc注释
        for i in (0..line_num).rev().take(5) {
            let line = lines[i].trim();
            if line.starts_with("/**") {
                return true;
            }
            if !line.is_empty() && !line.starts_with("//") && !line.starts_with("*") {
                break;
            }
        }
        false
    }

    // 生成Rust函数文档
    fn generate_rust_function_doc_simple(&self, function_name: &str) -> String {
        format!(
            "/// {}\n/// \n/// # Arguments\n/// \n/// * `param` - 参数描述\n/// \n/// # Returns\n/// \n/// 返回值描述",
            self.generate_function_description(function_name)
        )
    }

    // 生成Rust结构体文档
    fn generate_rust_struct_doc_simple(&self, struct_name: &str) -> String {
        format!(
            "/// {}\n/// \n/// 结构体的详细描述",
            self.generate_type_description(struct_name)
        )
    }

    // 生成Python函数文档
    fn generate_python_function_doc_simple(&self, function_name: &str) -> String {
        format!(
            "\"\"\"\n{}\n\nArgs:\n    param: 参数描述\n\nReturns:\n    返回值描述\n\"\"\"",
            self.generate_function_description(function_name)
        )
    }

    // 生成Python类文档
    fn generate_python_class_doc_simple(&self, class_name: &str) -> String {
        format!(
            "\"\"\"\n{}\n\n类的详细描述\n\"\"\"",
            self.generate_type_description(class_name)
        )
    }

    // 生成JavaScript函数文档
    fn generate_javascript_function_doc_simple(&self, function_name: &str) -> String {
        format!(
            "/**\n * {}\n * @param {{*}} param - 参数描述\n * @returns {{*}} 返回值描述\n */",
            self.generate_function_description(function_name)
        )
    }

    // 生成函数描述
    fn generate_function_description(&self, function_name: &str) -> String {
        if function_name.starts_with("get_") {
            format!("获取{}", &function_name[4..])
        } else if function_name.starts_with("set_") {
            format!("设置{}", &function_name[4..])
        } else if function_name.starts_with("is_") {
            format!("检查是否{}", &function_name[3..])
        } else if function_name.starts_with("has_") {
            format!("检查是否有{}", &function_name[4..])
        } else if function_name.starts_with("create_") {
            format!("创建{}", &function_name[7..])
        } else if function_name.starts_with("delete_") {
            format!("删除{}", &function_name[7..])
        } else if function_name.starts_with("update_") {
            format!("更新{}", &function_name[7..])
        } else {
            format!("{}函数的描述", function_name)
        }
    }

    // 生成类型描述
    fn generate_type_description(&self, type_name: &str) -> String {
        format!("{}类型的描述", type_name)
    }
}

#[async_trait]
impl MCPTool for DocumentationSuggestionTool {
    fn name(&self) -> &str {
        "suggest_documentation"
    }

    fn description(&self) -> &str {
        "在需要改进代码文档质量、添加缺失注释或优化文档格式时，分析代码并提供文档改进建议，包括缺失的函数注释、类文档和格式优化建议。"
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["file_path".to_string(), "language".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert(
                        "file_path".to_string(),
                        Schema::String(SchemaString {
                            description: Some("要分析的代码文件路径".to_string()),
                            ..Default::default()
                        }),
                    );
                    map.insert(
                        "language".to_string(),
                        Schema::String(SchemaString {
                            description: Some("代码文件的编程语言".to_string()),
                            enum_values: Some(vec![
                                "rust".to_string(),
                                "python".to_string(),
                                "javascript".to_string(),
                                "typescript".to_string(),
                            ]),
                        }),
                    );
                    map.insert(
                        "severity_filter".to_string(),
                        Schema::String(SchemaString {
                            description: Some("过滤建议的严重程度（LOW/MEDIUM/HIGH）".to_string()),
                            enum_values: Some(vec![
                                "LOW".to_string(),
                                "MEDIUM".to_string(),
                                "HIGH".to_string(),
                            ]),
                        }),
                    );
                    map.insert(
                        "include_examples".to_string(),
                        Schema::Boolean(SchemaBoolean {
                            description: Some("是否包含文档示例".to_string()),
                        }),
                    );
                    map
                },
                ..Default::default()
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("缺少file_path参数".to_string()))?;

        let language = params["language"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("缺少language参数".to_string()))?;

        let severity_filter = params["severity_filter"].as_str();
        let include_examples = params["include_examples"].as_bool().unwrap_or(true);

        // 检查文件是否存在
        if !Path::new(file_path).exists() {
            return Err(MCPError::NotFound(format!("文件不存在: {}", file_path)).into());
        }

        // 检查缓存
        let cache_key = format!("{}:{}", file_path, language);
        let cache_ttl = chrono::Duration::minutes(30);

        {
            let cache = self.cache.read().await;
            if let Some((result, timestamp)) = cache.get(&cache_key) {
                if Utc::now() - *timestamp < cache_ttl {
                    let mut filtered_result = result.clone();
                    
                    if let Some(filter) = severity_filter {
                        filtered_result.suggestions.retain(|s| s.severity == filter);
                    }
                    
                    if !include_examples {
                        for suggestion in &mut filtered_result.suggestions {
                            suggestion.examples.clear();
                        }
                    }
                    
                    return Ok(json!(filtered_result));
                }
            }
        }

        // 分析代码
        let mut result = self.analyze_code_file(file_path, language).await?;

        // 应用过滤器
        if let Some(filter) = severity_filter {
            result.suggestions.retain(|s| s.severity == filter);
        }

        if !include_examples {
            for suggestion in &mut result.suggestions {
                suggestion.examples.clear();
            }
        }

        // 更新缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, (result.clone(), Utc::now()));
        }

        Ok(json!(result))
    }
} 