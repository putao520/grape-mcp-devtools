use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::collections::HashMap;

use super::base::{MCPTool, Schema, SchemaObject, SchemaString};

/// 代码分析工具
pub struct AnalyzeCodeTool;

#[async_trait]
impl MCPTool for AnalyzeCodeTool {
    fn name(&self) -> &'static str {
        "analyze_code"
    }
    
    fn description(&self) -> &'static str {
        "在需要评估代码质量、识别潜在bug、性能问题或进行代码审查时，对指定的代码片段进行全面的质量检查，包括复杂度计算、代码建议和最佳实践检查。"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["code".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("code".to_string(), Schema::String(SchemaString {
                        description: Some("要分析的代码".to_string()),
                        enum_values: None,
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("编程语言".to_string()),
                        enum_values: Some(vec![
                            "rust".to_string(),
                            "python".to_string(),
                            "javascript".to_string(),
                            "typescript".to_string(),
                            "java".to_string(),
                            "go".to_string(),
                        ]),
                    }));
                    map
                },
                description: Some("代码分析参数".to_string()),
            })
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        let code = params.get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少代码参数"))?;
            
        let language = params.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        // 简单的代码分析
        let lines = code.lines().count();
        let chars = code.chars().count();
        let complexity_score = calculate_complexity(code, language);
        let suggestions = generate_suggestions(code, language);
        
        Ok(json!({
            "analysis": {
                "lines": lines,
                "characters": chars,
                "complexity_score": complexity_score,
                "language": language
            },
            "suggestions": suggestions,
            "message": "代码分析完成"
        }))
    }
}

/// 重构建议工具
pub struct SuggestRefactoringTool;

#[async_trait]
impl MCPTool for SuggestRefactoringTool {
    fn name(&self) -> &'static str {
        "suggest_refactoring"
    }
    
    fn description(&self) -> &'static str {
        "在需要改进代码结构、提升代码可维护性或优化代码设计时，为指定的代码片段提供详细的重构建议，包括结构优化、性能改进和最佳实践推荐。"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["code".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("code".to_string(), Schema::String(SchemaString {
                        description: Some("要重构的代码".to_string()),
                        enum_values: None,
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("编程语言".to_string()),
                        enum_values: Some(vec![
                            "rust".to_string(),
                            "python".to_string(),
                            "javascript".to_string(),
                            "typescript".to_string(),
                            "java".to_string(),
                            "go".to_string(),
                        ]),
                    }));
                    map
                },
                description: Some("重构建议参数".to_string()),
            })
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        let code = params.get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少代码参数"))?;
            
        let language = params.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let refactoring_suggestions = generate_refactoring_suggestions(code, language);
        
        Ok(json!({
            "refactoring_suggestions": refactoring_suggestions,
            "language": language,
            "message": "重构建议生成完成"
        }))
    }
}

/// 计算代码复杂度
fn calculate_complexity(code: &str, language: &str) -> u32 {
    let mut complexity = 1; // 基础复杂度
    
    // 统计控制流语句
    let control_keywords = match language {
        "rust" => vec!["if", "else", "match", "while", "for", "loop"],
        "python" => vec!["if", "elif", "else", "while", "for", "try", "except"],
        "javascript" | "typescript" => vec!["if", "else", "while", "for", "switch", "try", "catch"],
        "java" => vec!["if", "else", "while", "for", "switch", "try", "catch"],
        "go" => vec!["if", "else", "for", "switch", "select"],
        _ => vec!["if", "else", "while", "for"],
    };
    
    for keyword in control_keywords {
        complexity += code.matches(&format!(" {} ", keyword)).count() as u32;
        complexity += code.matches(&format!("{} ", keyword)).count() as u32;
    }
    
    complexity
}

/// 生成代码建议
fn generate_suggestions(code: &str, language: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    // 检查代码长度
    if code.lines().count() > 50 {
        suggestions.push("函数过长，建议拆分为更小的函数".to_string());
    }
    
    // 检查复杂度
    let complexity = calculate_complexity(code, language);
    if complexity > 10 {
        suggestions.push("代码复杂度较高，建议简化逻辑".to_string());
    }
    
    // 语言特定建议
    match language {
        "rust" => {
            if code.contains("unwrap()") {
                suggestions.push("避免使用 unwrap()，考虑使用 ? 操作符或 match".to_string());
            }
            if code.contains("clone()") && code.matches("clone()").count() > 3 {
                suggestions.push("频繁使用 clone()，考虑使用引用或重新设计数据结构".to_string());
            }
        }
        "python" => {
            if code.contains("except:") {
                suggestions.push("避免使用裸露的 except，指定具体的异常类型".to_string());
            }
        }
        "javascript" | "typescript" => {
            if code.contains("var ") {
                suggestions.push("使用 let 或 const 替代 var".to_string());
            }
        }
        _ => {}
    }
    
    if suggestions.is_empty() {
        suggestions.push("代码质量良好，暂无建议".to_string());
    }
    
    suggestions
}

/// 生成重构建议
fn generate_refactoring_suggestions(code: &str, language: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    // 通用重构建议
    if code.lines().count() > 30 {
        suggestions.push("函数较长，建议拆分为多个小函数".to_string());
    }
    
    // 检查重复代码
    let lines: Vec<&str> = code.lines().collect();
    let mut line_counts = HashMap::new();
    for line in &lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() && trimmed.len() > 10 {
            *line_counts.entry(trimmed).or_insert(0) += 1;
        }
    }
    
    for (line, count) in line_counts {
        if count > 2 {
            suggestions.push(format!("发现重复代码: \"{}\"，建议提取为函数", 
                if line.len() > 30 { &line[..30] } else { line }));
            break; // 只报告第一个重复
        }
    }
    
    // 语言特定重构建议
    match language {
        "rust" => {
            if code.contains("if let Some(") && code.matches("if let Some(").count() > 2 {
                suggestions.push("多个 if let Some 模式，考虑使用 match 或提取函数".to_string());
            }
        }
        "python" => {
            if code.contains("if __name__ == '__main__'") {
                suggestions.push("考虑将主逻辑提取到单独的函数中".to_string());
            }
        }
        _ => {}
    }
    
    if suggestions.is_empty() {
        suggestions.push("代码结构良好，暂无重构建议".to_string());
    }
    
    suggestions
}
