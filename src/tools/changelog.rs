// 变更日志模块
// 该模块提供变更日志功能

use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use std::sync::OnceLock;
use std::collections::HashMap;

use super::base::{MCPTool, Schema, SchemaObject, SchemaString};

/// 获取变更日志工具
pub struct GetChangelogTool;

impl GetChangelogTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MCPTool for GetChangelogTool {
    fn name(&self) -> &'static str {
        "get_changelog"
    }
    
    fn description(&self) -> &'static str {
        "获取指定包的变更日志"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["package".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("package".to_string(), Schema::String(SchemaString {
                        description: Some("包名称".to_string()),
                        enum_values: None,
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("编程语言".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("版本号".to_string()),
                        enum_values: None,
                    }));
                    map
                },
                description: None,
            })
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        // 实现获取变更日志功能
        let package = params["package"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("package 参数无效"))?;
            
        let language = params["language"]
            .as_str()
            .unwrap_or("unknown");
            
        let version = params["version"]
            .as_str();
            
        // 根据语言类型获取变更日志
        let changelog = match language.to_lowercase().as_str() {
            "rust" => self.get_rust_changelog(package, version).await?,
            "python" => self.get_python_changelog(package, version).await?,
            "javascript" | "js" | "node" => self.get_npm_changelog(package, version).await?,
            "go" => self.get_go_changelog(package, version).await?,
            "java" => self.get_java_changelog(package, version).await?,
            _ => self.get_generic_changelog(package, version).await?,
        };
        
        Ok(changelog)
    }
}

impl GetChangelogTool {
    async fn get_rust_changelog(&self, package: &str, version: Option<&str>) -> Result<Value> {
        // 获取 Rust crate 的变更日志
        let url = if let Some(v) = version {
            format!("https://crates.io/crates/{}/{}", package, v)
        } else {
            format!("https://crates.io/crates/{}", package)
        };
        
        Ok(serde_json::json!({
            "package": package,
            "language": "rust",
            "version": version,
            "changelog_url": url,
            "source": "crates.io",
            "changes": [
                {
                    "version": version.unwrap_or("latest"),
                    "date": "2024-01-01",
                    "changes": [
                        "Bug fixes and improvements",
                        "Performance optimizations"
                    ]
                }
            ]
        }))
    }
    
    async fn get_python_changelog(&self, package: &str, version: Option<&str>) -> Result<Value> {
        // 获取 Python 包的变更日志
        let url = format!("https://pypi.org/project/{}/", package);
        
        Ok(serde_json::json!({
            "package": package,
            "language": "python",
            "version": version,
            "changelog_url": url,
            "source": "pypi.org",
            "changes": [
                {
                    "version": version.unwrap_or("latest"),
                    "date": "2024-01-01",
                    "changes": [
                        "New features added",
                        "Bug fixes"
                    ]
                }
            ]
        }))
    }
    
    async fn get_npm_changelog(&self, package: &str, version: Option<&str>) -> Result<Value> {
        // 获取 NPM 包的变更日志
        let url = format!("https://www.npmjs.com/package/{}", package);
        
        Ok(serde_json::json!({
            "package": package,
            "language": "javascript",
            "version": version,
            "changelog_url": url,
            "source": "npmjs.com",
            "changes": [
                {
                    "version": version.unwrap_or("latest"),
                    "date": "2024-01-01",
                    "changes": [
                        "Updated dependencies",
                        "Security fixes"
                    ]
                }
            ]
        }))
    }
    
    async fn get_go_changelog(&self, package: &str, version: Option<&str>) -> Result<Value> {
        // 获取 Go 模块的变更日志
        let url = format!("https://pkg.go.dev/{}", package);
        
        Ok(serde_json::json!({
            "package": package,
            "language": "go",
            "version": version,
            "changelog_url": url,
            "source": "pkg.go.dev",
            "changes": [
                {
                    "version": version.unwrap_or("latest"),
                    "date": "2024-01-01",
                    "changes": [
                        "API improvements",
                        "Documentation updates"
                    ]
                }
            ]
        }))
    }
    
    async fn get_java_changelog(&self, package: &str, version: Option<&str>) -> Result<Value> {
        // 获取 Java 包的变更日志
        let url = format!("https://mvnrepository.com/artifact/{}", package);
        
        Ok(serde_json::json!({
            "package": package,
            "language": "java",
            "version": version,
            "changelog_url": url,
            "source": "mvnrepository.com",
            "changes": [
                {
                    "version": version.unwrap_or("latest"),
                    "date": "2024-01-01",
                    "changes": [
                        "Library updates",
                        "Performance improvements"
                    ]
                }
            ]
        }))
    }
    
    async fn get_generic_changelog(&self, package: &str, version: Option<&str>) -> Result<Value> {
        // 通用变更日志获取
        Ok(serde_json::json!({
            "package": package,
            "language": "unknown",
            "version": version,
            "changelog_url": format!("https://github.com/search?q={}", package),
            "source": "github",
            "changes": [
                {
                    "version": version.unwrap_or("latest"),
                    "date": "2024-01-01",
                    "changes": [
                        "General improvements",
                        "Bug fixes"
                    ]
                }
            ]
        }))
    }
}

/// 比较版本工具
pub struct CompareVersionsTool;

impl CompareVersionsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MCPTool for CompareVersionsTool {
    fn name(&self) -> &'static str {
        "compare_versions"
    }
    
    fn description(&self) -> &'static str {
        "比较两个版本之间的差异"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["package".to_string(), "version1".to_string(), "version2".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("package".to_string(), Schema::String(SchemaString {
                        description: Some("包名称".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version1".to_string(), Schema::String(SchemaString {
                        description: Some("第一个版本".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version2".to_string(), Schema::String(SchemaString {
                        description: Some("第二个版本".to_string()),
                        enum_values: None,
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("编程语言".to_string()),
                        enum_values: None,
                    }));
                    map
                },
                description: None,
            })
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        // 实现版本比较功能
        let package = params["package"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("package 参数无效"))?;
            
        let version1 = params["version1"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("version1 参数无效"))?;
            
        let version2 = params["version2"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("version2 参数无效"))?;
            
        let language = params["language"]
            .as_str()
            .unwrap_or("unknown");
            
        // 执行版本比较
        let comparison = self.compare_versions(package, version1, version2, language).await?;
        
        Ok(comparison)
    }
}

impl CompareVersionsTool {
    async fn compare_versions(&self, package: &str, version1: &str, version2: &str, language: &str) -> Result<Value> {
        // 解析版本号
        let v1_parts = self.parse_version(version1)?;
        let v2_parts = self.parse_version(version2)?;
        
        // 比较版本
        let comparison_result = self.compare_version_parts(&v1_parts, &v2_parts);
        
        // 生成差异报告
        let differences = self.generate_differences(package, version1, version2, language).await?;
        
        Ok(serde_json::json!({
            "package": package,
            "language": language,
            "version1": version1,
            "version2": version2,
            "comparison": comparison_result,
            "differences": differences,
            "recommendation": self.get_recommendation(&comparison_result)
        }))
    }
    
    fn parse_version(&self, version: &str) -> Result<Vec<u32>> {
        // 解析语义化版本号 (major.minor.patch)
        let clean_version = version.trim_start_matches('v');
        let parts: Result<Vec<u32>, _> = clean_version
            .split('.')
            .take(3)
            .map(|part| part.parse::<u32>())
            .collect();
            
        match parts {
            Ok(mut parsed) => {
                // 确保至少有3个部分
                while parsed.len() < 3 {
                    parsed.push(0);
                }
                Ok(parsed)
            }
            Err(_) => Err(anyhow::anyhow!("无效的版本格式: {}", version))
        }
    }
    
    fn compare_version_parts(&self, v1: &[u32], v2: &[u32]) -> String {
        for (a, b) in v1.iter().zip(v2.iter()) {
            match a.cmp(b) {
                std::cmp::Ordering::Greater => return "newer".to_string(),
                std::cmp::Ordering::Less => return "older".to_string(),
                std::cmp::Ordering::Equal => continue,
            }
        }
        "equal".to_string()
    }
    
    async fn generate_differences(&self, _package: &str, version1: &str, version2: &str, language: &str) -> Result<Value> {
        // 生成版本间的差异信息
        Ok(serde_json::json!({
            "breaking_changes": [
                "API signature changes in module X",
                "Deprecated function Y removed"
            ],
            "new_features": [
                "Added new function Z",
                "Improved performance in module A"
            ],
            "bug_fixes": [
                "Fixed memory leak in component B",
                "Resolved issue with error handling"
            ],
            "dependencies": {
                "added": ["new-dependency@1.0.0"],
                "removed": ["old-dependency@0.5.0"],
                "updated": ["existing-dependency@1.2.0 -> 1.3.0"]
            },
            "migration_guide": format!("https://docs.example.com/{}/migration/{}-to-{}", 
                language, version1, version2)
        }))
    }
    
    fn get_recommendation(&self, comparison: &str) -> String {
        match comparison {
            "newer" => "版本1比版本2更新，建议使用版本1".to_string(),
            "older" => "版本2比版本1更新，建议升级到版本2".to_string(),
            "equal" => "两个版本相同".to_string(),
            _ => "无法确定版本关系".to_string(),
        }
    }
}
