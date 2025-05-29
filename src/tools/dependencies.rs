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
use super::security::SecurityCheckTool;
use regex::Regex;
use roxmltree;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DependencyInfo {
    name: String,
    current_version: String,
    latest_version: Option<String>,
    release_date: Option<DateTime<Utc>>,
    security_alerts: Vec<String>,
    dependency_type: String, // "direct", "dev", "peer", etc.
    source: String, // "cargo", "npm", "pip", etc.
}

#[derive(Debug, Serialize, Deserialize)]
struct CargoTomlDependency {
    version: Option<String>,
    path: Option<String>,
    git: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
    features: Option<Vec<String>>,
    optional: Option<bool>,
    default_features: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageJsonDependencies {
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "peerDependencies")]
    peer_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "optionalDependencies")]
    optional_dependencies: Option<HashMap<String, String>>,
}

pub struct AnalyzeDependenciesTool {
    _annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, (Vec<DependencyInfo>, DateTime<Utc>)>>>,
    security_tool: SecurityCheckTool,
    client: reqwest::Client,
}

impl AnalyzeDependenciesTool {
    pub fn new() -> Self {
        Self {
            _annotations: ToolAnnotations {
                category: "依赖分析".to_string(),
                tags: vec!["依赖".to_string(), "分析".to_string()],
                version: "1.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
            security_tool: SecurityCheckTool::new(),
            client: reqwest::Client::new(),
        }
    }

    // 解析不同类型的依赖文件
    async fn parse_dependency_file(&self, language: &str, file_path: &str) -> Result<Vec<DependencyInfo>> {
        // 验证文件是否存在
        if !Path::new(file_path).exists() {
            return Err(MCPError::NotFound(format!("文件不存在: {}", file_path)).into());
        }

        // 根据文件类型解析依赖
        match language.to_lowercase().as_str() {
            "rust" => self.parse_cargo_toml(file_path).await,
            "python" => self.parse_requirements_txt(file_path).await,
            "javascript" | "typescript" | "node" => self.parse_package_json(file_path).await,
            "java" => self.parse_pom_xml(file_path).await,
            "go" => self.parse_go_mod(file_path).await,
            "dart" | "flutter" => self.parse_pubspec_yaml(file_path).await,
            _ => Err(MCPError::InvalidParameter(format!(
                "不支持的编程语言: {}", language
            )).into()),
        }
    }

    async fn parse_cargo_toml(&self, file_path: &str) -> Result<Vec<DependencyInfo>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let parsed: toml::Value = toml::from_str(&content)?;
        
        let mut dependencies = Vec::new();
        
        // 解析 [dependencies]
        if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_table()) {
            for (name, value) in deps {
                let dep_info = self.parse_cargo_dependency(name, value, "direct", "cargo").await?;
                dependencies.push(dep_info);
            }
        }
        
        // 解析 [dev-dependencies]
        if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|v| v.as_table()) {
            for (name, value) in dev_deps {
                let dep_info = self.parse_cargo_dependency(name, value, "dev", "cargo").await?;
                dependencies.push(dep_info);
            }
        }
        
        // 解析 [build-dependencies]
        if let Some(build_deps) = parsed.get("build-dependencies").and_then(|v| v.as_table()) {
            for (name, value) in build_deps {
                let dep_info = self.parse_cargo_dependency(name, value, "build", "cargo").await?;
                dependencies.push(dep_info);
            }
        }
        
        Ok(dependencies)
    }

    async fn parse_cargo_dependency(&self, name: &str, value: &toml::Value, dep_type: &str, source: &str) -> Result<DependencyInfo> {
        let version = match value {
            toml::Value::String(v) => v.clone(),
            toml::Value::Table(table) => {
                table.get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string()
            },
            _ => "*".to_string(),
        };

        // 获取最新版本信息
        let latest_version = self.fetch_latest_version("cargo", name).await.ok();
        
        // 检查安全漏洞
        let security_alerts = self.check_security_vulnerabilities("cargo", name, &version).await?;

        Ok(DependencyInfo {
            name: name.to_string(),
            current_version: version,
            latest_version,
            release_date: None, // 可以通过API获取
            security_alerts,
            dependency_type: dep_type.to_string(),
            source: source.to_string(),
        })
    }

    async fn parse_package_json(&self, file_path: &str) -> Result<Vec<DependencyInfo>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let parsed: PackageJsonDependencies = serde_json::from_str(&content)?;
        
        let mut dependencies = Vec::new();
        
        // 解析 dependencies
        if let Some(deps) = parsed.dependencies {
            for (name, version) in deps {
                let dep_info = self.parse_npm_dependency(&name, &version, "direct", "npm").await?;
                dependencies.push(dep_info);
            }
        }
        
        // 解析 devDependencies
        if let Some(dev_deps) = parsed.dev_dependencies {
            for (name, version) in dev_deps {
                let dep_info = self.parse_npm_dependency(&name, &version, "dev", "npm").await?;
                dependencies.push(dep_info);
            }
        }
        
        // 解析 peerDependencies
        if let Some(peer_deps) = parsed.peer_dependencies {
            for (name, version) in peer_deps {
                let dep_info = self.parse_npm_dependency(&name, &version, "peer", "npm").await?;
                dependencies.push(dep_info);
            }
        }
        
        // 解析 optionalDependencies
        if let Some(optional_deps) = parsed.optional_dependencies {
            for (name, version) in optional_deps {
                let dep_info = self.parse_npm_dependency(&name, &version, "optional", "npm").await?;
                dependencies.push(dep_info);
            }
        }
        
        Ok(dependencies)
    }

    async fn parse_npm_dependency(&self, name: &str, version: &str, dep_type: &str, source: &str) -> Result<DependencyInfo> {
        // 清理版本号（移除 ^, ~, >= 等前缀）
        let clean_version = version.trim_start_matches(&['^', '~', '>', '=', ' '][..]).to_string();
        
        // 获取最新版本信息
        let latest_version = self.fetch_latest_version("npm", name).await.ok();
        
        // 检查安全漏洞
        let security_alerts = self.check_security_vulnerabilities("npm", name, &clean_version).await?;

        Ok(DependencyInfo {
            name: name.to_string(),
            current_version: clean_version,
            latest_version,
            release_date: None,
            security_alerts,
            dependency_type: dep_type.to_string(),
            source: source.to_string(),
        })
    }

    async fn parse_requirements_txt(&self, file_path: &str) -> Result<Vec<DependencyInfo>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let mut dependencies = Vec::new();
        
        // 正则表达式匹配 package==version, package>=version 等格式
        let re = Regex::new(r"^([a-zA-Z0-9_-]+)([><=!]+)([0-9.]+.*?)(?:\s|$)")?;
        let simple_re = Regex::new(r"^([a-zA-Z0-9_-]+)(?:\s|$)")?;
        
        for line in content.lines() {
            let line = line.trim();
            
            // 跳过注释和空行
            if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
                continue;
            }
            
            if let Some(caps) = re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let version = caps.get(3).unwrap().as_str();
                
                let dep_info = self.parse_pip_dependency(name, version, "direct", "pip").await?;
                dependencies.push(dep_info);
            } else if let Some(caps) = simple_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                
                let dep_info = self.parse_pip_dependency(name, "*", "direct", "pip").await?;
                dependencies.push(dep_info);
            }
        }
        
        Ok(dependencies)
    }

    async fn parse_pip_dependency(&self, name: &str, version: &str, dep_type: &str, source: &str) -> Result<DependencyInfo> {
        // 获取最新版本信息
        let latest_version = self.fetch_latest_version("pip", name).await.ok();
        
        // 检查安全漏洞
        let security_alerts = self.check_security_vulnerabilities("pip", name, version).await?;

        Ok(DependencyInfo {
            name: name.to_string(),
            current_version: version.to_string(),
            latest_version,
            release_date: None,
            security_alerts,
            dependency_type: dep_type.to_string(),
            source: source.to_string(),
        })
    }

    async fn parse_pom_xml(&self, file_path: &str) -> Result<Vec<DependencyInfo>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let doc = roxmltree::Document::parse(&content)?;
        
        let mut dependencies = Vec::new();
        
        // 查找所有 <dependency> 节点
        for node in doc.descendants() {
            if node.tag_name().name() == "dependency" {
                let mut group_id = String::new();
                let mut artifact_id = String::new();
                let mut version = String::new();
                let mut scope = "compile".to_string();
                
                for child in node.children() {
                    match child.tag_name().name() {
                        "groupId" => group_id = child.text().unwrap_or("").to_string(),
                        "artifactId" => artifact_id = child.text().unwrap_or("").to_string(),
                        "version" => version = child.text().unwrap_or("").to_string(),
                        "scope" => scope = child.text().unwrap_or("compile").to_string(),
                        _ => {}
                    }
                }
                
                if !group_id.is_empty() && !artifact_id.is_empty() {
                    let name = format!("{}:{}", group_id, artifact_id);
                    let dep_info = self.parse_maven_dependency(&name, &version, &scope, "maven").await?;
                    dependencies.push(dep_info);
                }
            }
        }
        
        Ok(dependencies)
    }

    async fn parse_maven_dependency(&self, name: &str, version: &str, dep_type: &str, source: &str) -> Result<DependencyInfo> {
        // 获取最新版本信息
        let latest_version = self.fetch_latest_version("maven", name).await.ok();
        
        // 检查安全漏洞
        let security_alerts = self.check_security_vulnerabilities("maven", name, version).await?;

        Ok(DependencyInfo {
            name: name.to_string(),
            current_version: version.to_string(),
            latest_version,
            release_date: None,
            security_alerts,
            dependency_type: dep_type.to_string(),
            source: source.to_string(),
        })
    }

    async fn parse_go_mod(&self, file_path: &str) -> Result<Vec<DependencyInfo>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let mut dependencies = Vec::new();
        
        // 简单的 go.mod 解析
        let re = Regex::new(r"^\s*([^\s]+)\s+v([0-9.]+.*?)(?:\s|$)")?;
        let mut in_require_block = false;
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with("require (") {
                in_require_block = true;
                continue;
            }
            
            if line == ")" && in_require_block {
                in_require_block = false;
                continue;
            }
            
            if line.starts_with("require ") || in_require_block {
                let search_line = if line.starts_with("require ") {
                    &line[8..]
                } else {
                    line
                };
                
                if let Some(caps) = re.captures(search_line) {
                    let name = caps.get(1).unwrap().as_str();
                    let version = caps.get(2).unwrap().as_str();
                    
                    let dep_info = self.parse_go_dependency(name, version, "direct", "go").await?;
                    dependencies.push(dep_info);
                }
            }
        }
        
        Ok(dependencies)
    }

    async fn parse_go_dependency(&self, name: &str, version: &str, dep_type: &str, source: &str) -> Result<DependencyInfo> {
        // 获取最新版本信息
        let latest_version = self.fetch_latest_version("go", name).await.ok();
        
        // 检查安全漏洞
        let security_alerts = self.check_security_vulnerabilities("go", name, version).await?;

        Ok(DependencyInfo {
            name: name.to_string(),
            current_version: version.to_string(),
            latest_version,
            release_date: None,
            security_alerts,
            dependency_type: dep_type.to_string(),
            source: source.to_string(),
        })
    }

    async fn parse_pubspec_yaml(&self, file_path: &str) -> Result<Vec<DependencyInfo>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content)?;
        
        let mut dependencies = Vec::new();
        
        // 解析 dependencies
        if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_mapping()) {
            for (name, version) in deps {
                if let (Some(name_str), Some(version_str)) = (name.as_str(), version.as_str()) {
                    let dep_info = self.parse_dart_dependency(name_str, version_str, "direct", "pub").await?;
                    dependencies.push(dep_info);
                }
            }
        }
        
        // 解析 dev_dependencies
        if let Some(dev_deps) = parsed.get("dev_dependencies").and_then(|v| v.as_mapping()) {
            for (name, version) in dev_deps {
                if let (Some(name_str), Some(version_str)) = (name.as_str(), version.as_str()) {
                    let dep_info = self.parse_dart_dependency(name_str, version_str, "dev", "pub").await?;
                    dependencies.push(dep_info);
                }
            }
        }
        
        Ok(dependencies)
    }

    async fn parse_dart_dependency(&self, name: &str, version: &str, dep_type: &str, source: &str) -> Result<DependencyInfo> {
        // 清理版本号
        let clean_version = version.trim_start_matches(&['^', '~', '>', '=', ' '][..]).to_string();
        
        // 获取最新版本信息
        let latest_version = self.fetch_latest_version("pub", name).await.ok();
        
        // 检查安全漏洞
        let security_alerts = self.check_security_vulnerabilities("pub", name, &clean_version).await?;

        Ok(DependencyInfo {
            name: name.to_string(),
            current_version: clean_version,
            latest_version,
            release_date: None,
            security_alerts,
            dependency_type: dep_type.to_string(),
            source: source.to_string(),
        })
    }

    // 获取最新版本信息
    async fn fetch_latest_version(&self, package_type: &str, name: &str) -> Result<String> {
        let url = match package_type {
            "cargo" => format!("https://crates.io/api/v1/crates/{}", name),
            "npm" => format!("https://registry.npmjs.org/{}", name),
            "pip" => format!("https://pypi.org/pypi/{}/json", name),
            "maven" => format!("https://search.maven.org/solrsearch/select?q=a:\"{}\"&core=gav&rows=1&wt=json", name),
            "go" => format!("https://proxy.golang.org/{}/@v/list", name),
            "pub" => format!("https://pub.dev/api/packages/{}", name),
            _ => return Err(anyhow::anyhow!("不支持的包类型: {}", package_type)),
        };

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("获取版本信息失败: {}", response.status()));
        }

        let version = match package_type {
            "go" => {
                let text = response.text().await?;
                text.lines().last().unwrap_or("unknown").to_string()
            },
            _ => {
                let data: Value = response.json().await?;
                match package_type {
                    "cargo" => data["crate"]["max_stable_version"].as_str().unwrap_or("unknown"),
                    "npm" => data["dist-tags"]["latest"].as_str().unwrap_or("unknown"),
                    "pip" => data["info"]["version"].as_str().unwrap_or("unknown"),
                    "maven" => {
                        data["response"]["docs"]
                            .as_array()
                            .and_then(|docs| docs.first())
                            .and_then(|doc| doc["v"].as_str())
                            .unwrap_or("unknown")
                    },
                    "pub" => data["latest"]["version"].as_str().unwrap_or("unknown"),
                    _ => "unknown",
                }.to_string()
            }
        };

        Ok(version.to_string())
    }

    // 改进的安全漏洞检查 - 使用真实的安全检查工具
    async fn check_security_vulnerabilities(&self, package_type: &str, name: &str, version: &str) -> Result<Vec<String>> {
        // 映射包类型到生态系统名称
        let ecosystem = match package_type {
            "cargo" => "cargo",
            "npm" => "npm", 
            "pip" => "pip",
            "maven" => "maven",
            "go" => "go",
            "pub" => "pub",
            _ => package_type,
        };

        // 构造安全检查参数
        let security_params = json!({
            "ecosystem": ecosystem,
            "package": name,
            "version": version,
            "include_fixed": false // 只关注未修复的漏洞
        });

        // 调用安全检查工具
        match self.security_tool.execute(security_params).await {
            Ok(result) => {
                let empty_vec = vec![];
                let vulnerabilities = result["vulnerabilities"].as_array().unwrap_or(&empty_vec);
                let alerts: Vec<String> = vulnerabilities.iter()
                    .map(|vuln| {
                        let id = vuln["id"].as_str().unwrap_or("unknown");
                        let severity = vuln["severity"].as_str().unwrap_or("unknown");
                        let summary = vuln["summary"].as_str().unwrap_or("无摘要");
                        format!("{} ({}): {}", id, severity, summary)
                    })
                    .collect();
                Ok(alerts)
            },
            Err(e) => {
                tracing::warn!("安全漏洞检查失败 {}/{}: {}", ecosystem, name, e);
                Ok(Vec::new()) // 失败时返回空列表，不阻断依赖分析
            }
        }
    }

    // 添加缓存机制
    async fn get_cached_dependencies(&self, cache_key: &str) -> Option<Vec<DependencyInfo>> {
        let cache = self.cache.read().await;
        if let Some((deps, timestamp)) = cache.get(cache_key) {
            let cache_ttl = chrono::Duration::hours(2); // 依赖信息缓存2小时
            if Utc::now() - *timestamp < cache_ttl {
                return Some(deps.clone());
            }
        }
        None
    }

    async fn cache_dependencies(&self, cache_key: String, dependencies: Vec<DependencyInfo>) {
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, (dependencies, Utc::now()));
    }

    // 添加参数验证方法
    fn validate_params(&self, params: &Value) -> Result<()> {
        if params["language"].as_str().is_none() {
            return Err(MCPError::InvalidParameter("缺少language参数".to_string()).into());
        }
        
        if params["files"].as_array().is_none() {
            return Err(MCPError::InvalidParameter("缺少files参数或格式错误".to_string()).into());
        }
        
        Ok(())
    }

    // 执行依赖分析的内部方法
    async fn execute_internal(&self, params: Value) -> Result<Value> {
        // 验证参数
        self.validate_params(&params)?;

        // 提取参数
        let language = params["language"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("language 参数无效".into()))?;

        let files = params["files"]
            .as_array()
            .ok_or_else(|| MCPError::InvalidParameter("files 参数必须是数组".into()))?;

        let check_updates = params["check_updates"]
            .as_bool()
            .unwrap_or(true);

        // 生成缓存键
        let cache_key = format!("{}:{}", language, files.iter()
            .filter_map(|f| f.as_str())
            .collect::<Vec<_>>()
            .join(","));

        // 检查缓存
        if let Some(cached_deps) = self.get_cached_dependencies(&cache_key).await {
            tracing::debug!("从缓存返回依赖分析结果: {}", cache_key);
            return Ok(self.format_dependency_result(cached_deps, language, check_updates));
        }

        let mut all_deps = Vec::new();
        let mut analysis_errors = Vec::new();

        // 处理每个依赖文件
        for file in files {
            let file_path = file.as_str()
                .ok_or_else(|| MCPError::InvalidParameter("file 路径必须是字符串".into()))?;

            tracing::info!("分析依赖文件: {}", file_path);

            // 解析依赖文件
            match self.parse_dependency_file(language, file_path).await {
                Ok(deps) => {
                    tracing::info!("成功解析 {} 个依赖项从文件: {}", deps.len(), file_path);
                    all_deps.extend(deps);
                },
                Err(e) => {
                    let error_msg = format!("解析文件 {} 失败: {}", file_path, e);
                    tracing::warn!("{}", error_msg);
                    analysis_errors.push(error_msg);
                }
            }
        }

        // 缓存结果
        if !all_deps.is_empty() {
            self.cache_dependencies(cache_key, all_deps.clone()).await;
        }

        Ok(self.format_dependency_result_with_errors(all_deps, language, check_updates, analysis_errors))
    }

    // 格式化依赖分析结果
    fn format_dependency_result(&self, dependencies: Vec<DependencyInfo>, language: &str, check_updates: bool) -> Value {
        self.format_dependency_result_with_errors(dependencies, language, check_updates, Vec::new())
    }

    // 格式化依赖分析结果（包含错误信息）
    fn format_dependency_result_with_errors(&self, dependencies: Vec<DependencyInfo>, language: &str, check_updates: bool, errors: Vec<String>) -> Value {
        let formatted_deps: Vec<Value> = dependencies.iter().map(|dep| {
            let update_needed = if check_updates {
                dep.latest_version.as_ref().map_or(false, |latest| &dep.current_version != latest)
            } else {
                false
            };

            let security_risk_level = if dep.security_alerts.is_empty() {
                "NONE"
            } else if dep.security_alerts.iter().any(|alert| alert.contains("CRITICAL")) {
                "CRITICAL"
            } else if dep.security_alerts.iter().any(|alert| alert.contains("HIGH")) {
                "HIGH"
            } else if dep.security_alerts.iter().any(|alert| alert.contains("MEDIUM")) {
                "MEDIUM"
            } else {
                "LOW"
            };

            json!({
                "name": dep.name,
                "current_version": dep.current_version,
                "latest_version": dep.latest_version,
                "release_date": dep.release_date.map(|d| d.to_rfc3339()),
                "security_alerts": dep.security_alerts,
                "security_risk_level": security_risk_level,
                "dependency_type": dep.dependency_type,
                "source": dep.source,
                "update_needed": update_needed,
                "update_available": dep.latest_version.is_some() && update_needed,
            })
        }).collect();

        // 统计信息
        let total_deps = dependencies.len();
        let security_issues = dependencies.iter().filter(|d| !d.security_alerts.is_empty()).count();
        let updates_available = if check_updates {
            dependencies.iter().filter(|d| {
                d.latest_version.as_ref().map_or(false, |latest| &d.current_version != latest)
            }).count()
        } else {
            0
        };

        let mut result = json!({
            "language": language,
            "dependencies": formatted_deps,
            "summary": {
                "total_dependencies": total_deps,
                "security_issues": security_issues,
                "updates_available": updates_available,
                "analysis_timestamp": Utc::now().to_rfc3339()
            }
        });

        // 如果有错误，添加错误信息
        if !errors.is_empty() {
            result["errors"] = json!(errors);
        }

        result
    }
}

#[async_trait]
impl MCPTool for AnalyzeDependenciesTool {
    fn name(&self) -> &str {
        "analyze_dependencies"
    }

    fn description(&self) -> &str {
        "在需要了解项目的依赖关系、包的兼容性或依赖安全状况时，分析指定项目的依赖信息，包括依赖版本、更新状态、安全漏洞和兼容性检查。"
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();

        SCHEMA.get_or_init(|| {            Schema::Object(SchemaObject {
                required: vec!["language".to_string(), "files".to_string()],
                properties: {
                    let mut map = HashMap::new();                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("项目所使用的编程语言".to_string()),
                        enum_values: Some(vec![
                            "rust".to_string(), 
                            "python".to_string(), 
                            "javascript".to_string(),
                            "typescript".to_string(),
                            "node".to_string(),
                            "java".to_string(),
                            "go".to_string(),
                            "dart".to_string(),
                            "flutter".to_string()
                        ]),
                    }));
                    map.insert("files".to_string(), Schema::Array(SchemaArray {
                        description: Some("要分析的依赖文件路径列表".to_string()),
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
        self.execute_internal(params).await
    }
}
