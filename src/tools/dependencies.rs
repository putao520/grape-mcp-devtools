use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::path::Path;
use async_trait::async_trait;
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use anyhow::Result;
use crate::errors::MCPError;
use super::base::{MCPTool, ToolAnnotations, Schema, SchemaObject, SchemaString, SchemaBoolean, SchemaArray};

#[derive(Clone)]
struct DependencyInfo {
    name: String,
    current_version: String,
    latest_version: String,
    release_date: DateTime<Utc>,
    security_alerts: Vec<String>,
}

pub struct AnalyzeDependenciesTool {
    annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, (Vec<DependencyInfo>, DateTime<Utc>)>>>,
    security_db: Arc<RwLock<HashMap<String, Vec<String>>>>, // 模拟安全漏洞数据库
}

impl AnalyzeDependenciesTool {
    pub fn new() -> Self {
        Self {            annotations: ToolAnnotations {
                category: "依赖分析".to_string(),
                tags: vec!["依赖".to_string(), "分析".to_string()],
                version: "1.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
            security_db: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // 解析不同类型的依赖文件
    async fn parse_dependency_file(&self, language: &str, file_path: &str) -> Result<Vec<(String, String)>> {
        // 验证文件是否存在
        if !Path::new(file_path).exists() {
            return Err(MCPError::NotFound(format!("文件不存在: {}", file_path)).into());
        }

        // 根据文件类型解析依赖
        match language {
            "rust" => self.parse_cargo_toml(file_path).await,
            "python" => self.parse_requirements_txt(file_path).await,
            "javascript" | "typescript" => self.parse_package_json(file_path).await,
            _ => Err(MCPError::InvalidParameter(format!(
                "不支持的编程语言: {}", language
            )).into()),
        }
    }

    async fn parse_cargo_toml(&self, _file_path: &str) -> Result<Vec<(String, String)>> {
        // 实际实现中应该使用 toml 解析器
        // 这里仅作为示例返回一些模拟数据
        Ok(vec![
            ("tokio".to_string(), "1.36.0".to_string()),
            ("serde".to_string(), "1.0.197".to_string()),
        ])
    }

    async fn parse_requirements_txt(&self, _file_path: &str) -> Result<Vec<(String, String)>> {
        Ok(vec![
            ("requests".to_string(), "2.31.0".to_string()),
            ("flask".to_string(), "3.0.2".to_string()),
        ])
    }

    async fn parse_package_json(&self, _file_path: &str) -> Result<Vec<(String, String)>> {
        Ok(vec![
            ("express".to_string(), "4.18.3".to_string()),
            ("typescript".to_string(), "5.4.2".to_string()),
        ])
    }

    // 检查依赖项的最新版本和安全警告
    async fn check_dependency(&self, name: &str, current_version: &str) -> Result<DependencyInfo> {
        // 在实际实现中，这里应该查询包管理器的 API
        // 这里使用模拟数据
        let latest_version = match name {
            "tokio" => "1.36.0",
            "serde" => "1.0.197",
            "requests" => "2.31.0",
            "flask" => "3.0.2",
            "express" => "4.18.3",
            "typescript" => "5.4.2",
            _ => current_version,
        };

        // 检查安全警告
        let security_alerts = {
            let db = self.security_db.read().await;
            db.get(name)
                .cloned()
                .unwrap_or_default()
        };

        Ok(DependencyInfo {
            name: name.to_string(),
            current_version: current_version.to_string(),
            latest_version: latest_version.to_string(),
            release_date: Utc::now(), // 实际实现中应该从包管理器获取
            security_alerts,
        })
    }
}

#[async_trait]
impl MCPTool for AnalyzeDependenciesTool {
    fn name(&self) -> &str {
        "analyze_dependencies"
    }

    fn description(&self) -> &str {
        "分析项目依赖，识别过时的包、安全漏洞和版本兼容性问题"
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();

        SCHEMA.get_or_init(|| {            Schema::Object(SchemaObject {
                required: vec!["language".to_string(), "files".to_string()],
                properties: {
                    let mut map = HashMap::new();map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("编程语言".to_string()),
                        enum_values: Some(vec!["rust".to_string(), "python".to_string(), "javascript".to_string()]),
                    }));
                    map.insert("files".to_string(), Schema::Array(SchemaArray {
                        description: Some("依赖文件路径列表".to_string()),
                        items: Box::new(Schema::String(SchemaString::default())),
                    }));
                    map.insert("check_updates".to_string(), Schema::Boolean(SchemaBoolean {
                        description: Some("是否检查更新".to_string()),
                    }));
                    map
                },
                ..Default::default()
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        // 验证参数
        self.validate_params(&params)?;

        // 提取参数
        let language = params["language"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("language 参数无效".into()))?;

        let files = params["files"]
            .as_array()
            .ok_or_else(|| MCPError::InvalidParameter("files 参数必须是数组".into()))?;

        let _check_updates = params["check_updates"]
            .as_bool()
            .unwrap_or(true);

        let mut all_deps = Vec::new();

        // 处理每个依赖文件
        for file in files {
            let file_path = file.as_str()
                .ok_or_else(|| MCPError::InvalidParameter("file 路径必须是字符串".into()))?;

            // 解析依赖文件
            let deps = self.parse_dependency_file(language, file_path).await?;

            // 检查每个依赖的信息
            for (name, current_version) in deps {
                let info = self.check_dependency(&name, &current_version).await?;

                all_deps.push(json!({
                    "name": info.name,
                    "current_version": info.current_version,
                    "latest_version": info.latest_version,
                    "is_outdated": info.current_version != info.latest_version,
                    "security_alerts": info.security_alerts
                }));
            }
        }

        Ok(json!({
            "dependencies": all_deps
        }))
    }
}
