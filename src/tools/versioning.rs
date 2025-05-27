use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use anyhow::Result;
use crate::errors::MCPError;
use super::base::{MCPTool, ToolAnnotations, Schema, SchemaObject, SchemaString, SchemaBoolean};

#[derive(Clone)]
struct VersionInfo {
    latest_stable: String,
    latest_preview: Option<String>,
    release_date: DateTime<Utc>,
    eol_date: Option<DateTime<Utc>>,
    download_url: Option<String>,
    package_type: String,      // 新增: 包类型(npm, cargo, pip等)
    available_versions: Vec<String>, // 新增: 可用版本列表
    dependencies: Option<Value>, // 新增: 依赖信息
    repository_url: Option<String>, // 新增: 代码仓库地址
}

// Registry定义
#[derive(Clone)]
enum Registry {
    CratesIo,
    NpmJs,
    PyPI,
    MavenCentral,
    GoProxy,
    PubDev,
}

impl Registry {
    fn base_url(&self) -> &str {
        match self {
            Registry::CratesIo => "https://crates.io/api/v1",
            Registry::NpmJs => "https://registry.npmjs.org",
            Registry::PyPI => "https://pypi.org/pypi",
            Registry::MavenCentral => "https://search.maven.org/solrsearch/select",
            Registry::GoProxy => "https://proxy.golang.org",
            Registry::PubDev => "https://pub.dev/api",
        }
    }
}

pub struct CheckVersionTool {
    annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, (VersionInfo, DateTime<Utc>)>>>,
    client: reqwest::Client,
}

impl CheckVersionTool {
    pub fn new() -> Self {
        Self {
            annotations: ToolAnnotations {
                category: "版本检查".to_string(),
                tags: vec!["版本".to_string(), "检查".to_string()],
                version: "1.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::new(),
        }
    }
    
    async fn fetch_version_info(&self, type_: &str, name: &str) -> Result<VersionInfo> {
        match type_ {
            "cargo" => self.fetch_crates_io(name).await,
            "npm" => self.fetch_npm(name).await,
            "pip" => self.fetch_pypi(name).await,
            "maven" => self.fetch_maven_central(name).await,
            "go" => self.fetch_go_proxy(name).await,
            "pub" => self.fetch_pub_dev(name).await,
            _ => Err(MCPError::NotFound(format!(
                "不支持的包类型: {}", type_
            )).into()),
        }
    }

    async fn fetch_crates_io(&self, name: &str) -> Result<VersionInfo> {
        let url = format!("{}/crates/{}", Registry::CratesIo.base_url(), name);
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;

        let crate_data = data["crate"].as_object()
            .ok_or_else(|| MCPError::CacheError("无效的crates.io响应".to_string()))?;

        // 获取版本列表
        let versions_url = format!("{}/crates/{}/versions", Registry::CratesIo.base_url(), name);
        let versions_response = self.client.get(&versions_url).send().await?;
        let versions_data: Value = versions_response.json().await?;
        
        let available_versions = versions_data["versions"]
            .as_array()
            .map(|versions| {
                versions.iter()
                    .filter_map(|v| v["num"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // 获取最新版本的发布日期
        let latest_release_date = versions_data["versions"]
            .as_array()
            .and_then(|versions| versions.first())
            .and_then(|version| version["created_at"].as_str())
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Ok(VersionInfo {
            latest_stable: crate_data["max_stable_version"]
                .as_str()
                .unwrap_or("0.0.0")
                .to_string(),
            latest_preview: None,
            release_date: latest_release_date,
            eol_date: None,
            download_url: Some(format!("https://crates.io/crates/{}", name)),
            package_type: "cargo".to_string(),
            available_versions,
            dependencies: None,
            repository_url: crate_data["repository"]
                .as_str()
                .map(String::from),
        })
    }

    async fn fetch_npm(&self, name: &str) -> Result<VersionInfo> {
        let url = format!("{}/{}", Registry::NpmJs.base_url(), name);
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;

        let latest_version = data["dist-tags"]["latest"]
            .as_str()
            .ok_or_else(|| MCPError::CacheError("无效的npm响应".to_string()))?;

        Ok(VersionInfo {
            latest_stable: latest_version.to_string(),
            latest_preview: None,
            release_date: data["time"][latest_version]
                .as_str()
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            eol_date: None,
            download_url: Some(format!("https://www.npmjs.com/package/{}", name)),
            package_type: "npm".to_string(),
            available_versions: data["versions"]
                .as_object()
                .map(|versions| versions.keys().cloned().collect())
                .unwrap_or_default(),
            dependencies: data["versions"][latest_version]["dependencies"]
                .as_object()
                .map(|deps| json!(deps)),
            repository_url: data["repository"]["url"]
                .as_str()
                .map(String::from),
        })
    }

    async fn fetch_pypi(&self, name: &str) -> Result<VersionInfo> {
        let url = format!("{}/{}/json", Registry::PyPI.base_url(), name);
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;

        let info = data["info"].as_object()
            .ok_or_else(|| MCPError::CacheError("无效的PyPI响应".to_string()))?;

        let version = info["version"].as_str().unwrap_or("0.0.0");

        Ok(VersionInfo {
            latest_stable: version.to_string(),
            latest_preview: None,
            release_date: data["releases"][version]
                .as_array()
                .and_then(|releases| releases.first())
                .and_then(|release| release["upload_time_iso_8601"].as_str())
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            eol_date: None,
            download_url: Some(format!("https://pypi.org/project/{}", name)),
            package_type: "pip".to_string(),
            available_versions: data["releases"]
                .as_object()
                .map(|releases| releases.keys().cloned().collect())
                .unwrap_or_default(),
            dependencies: None,
            repository_url: info["project_urls"]["Source"]
                .as_str()
                .map(String::from),
        })
    }

    async fn fetch_maven_central(&self, name: &str) -> Result<VersionInfo> {
        // Maven Central使用Solr查询API
        let url = format!(
            "{}?q=a:\"{}\"&core=gav&rows=20&wt=json",
            Registry::MavenCentral.base_url(),
            name
        );
        
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;
        
        let docs = data["response"]["docs"].as_array()
            .ok_or_else(|| MCPError::CacheError("无效的Maven Central响应".to_string()))?;
            
        if docs.is_empty() {
            return Err(MCPError::NotFound(format!("未找到Maven包: {}", name)).into());
        }
        
        // 获取最新版本
        let latest = docs.iter()
            .max_by_key(|doc| doc["timestamp"].as_i64().unwrap_or(0))
            .ok_or_else(|| MCPError::CacheError("无法确定最新版本".to_string()))?;
            
        Ok(VersionInfo {
            latest_stable: latest["v"]
                .as_str()
                .unwrap_or("0.0.0")
                .to_string(),
            latest_preview: None,
            release_date: latest["timestamp"]
                .as_i64()
                .and_then(|ts| DateTime::from_timestamp(ts / 1000, 0))
                .unwrap_or_else(Utc::now),
            eol_date: None,
            download_url: Some(format!(
                "https://search.maven.org/artifact/{}/{}", 
                latest["g"].as_str().unwrap_or(""),
                name
            )),
            package_type: "maven".to_string(),
            available_versions: docs.iter()
                .filter_map(|doc| doc["v"].as_str().map(String::from))
                .collect(),
            dependencies: None,
            repository_url: None,
        })
    }

    async fn fetch_go_proxy(&self, name: &str) -> Result<VersionInfo> {
        // Go Proxy API
        let url = format!("{}/{}/@v/list", Registry::GoProxy.base_url(), name);
        let response = self.client.get(&url).send().await?;
        let versions: Vec<String> = response
            .text()
            .await?
            .lines()
            .map(String::from)
            .collect();
            
        if versions.is_empty() {
            return Err(MCPError::NotFound(format!("未找到Go包: {}", name)).into());
        }
        
        // 获取最新版本的详细信息
        let latest = versions.last()
            .ok_or_else(|| MCPError::CacheError("无法获取最新版本".to_string()))?;
            
        let info_url = format!(
            "{}/{}/@v/{}.info",
            Registry::GoProxy.base_url(),
            name,
            latest
        );
        
        let info: Value = self.client.get(&info_url)
            .send()
            .await?
            .json()
            .await?;
            
        Ok(VersionInfo {
            latest_stable: latest.clone(),
            latest_preview: None,
            release_date: info["Time"]
                .as_str()
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            eol_date: None,
            download_url: Some(format!("https://pkg.go.dev/{}", name)),
            package_type: "go".to_string(),
            available_versions: versions,
            dependencies: None,
            repository_url: Some(format!("https://pkg.go.dev/{}", name)),
        })
    }

    async fn fetch_pub_dev(&self, name: &str) -> Result<VersionInfo> {
        // pub.dev API
        let url = format!("{}/packages/{}", Registry::PubDev.base_url(), name);
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;
        
        let latest = data["latest"]
            .as_object()
            .ok_or_else(|| MCPError::CacheError("无效的pub.dev响应".to_string()))?;
            
        let version = latest["version"]
            .as_str()
            .ok_or_else(|| MCPError::CacheError("无法获取版本信息".to_string()))?;
            
        Ok(VersionInfo {
            latest_stable: version.to_string(),
            latest_preview: None,
            release_date: latest["published"]
                .as_str()
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            eol_date: None,
            download_url: Some(format!("https://pub.dev/packages/{}", name)),
            package_type: "pub".to_string(),
            available_versions: data["versions"]
                .as_array()
                .map(|versions| {
                    versions.iter()
                        .filter_map(|v| v["version"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            dependencies: latest["pubspec"]["dependencies"]
                .as_object()
                .map(|deps| json!(deps)),
            repository_url: latest["pubspec"]["repository"]
                .as_str()
                .map(String::from),
        })
    }
    
    async fn get_version_info(&self, type_: &str, name: &str) -> Result<VersionInfo> {
        let cache_key = format!("{}:{}", type_, name);
        let cache_ttl = chrono::Duration::hours(1);
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some((info, timestamp)) = cache.get(&cache_key) {
                if Utc::now() - *timestamp < cache_ttl {
                    return Ok(info.clone());
                }
            }
        }
        
        // 获取新数据
        let info = self.fetch_version_info(type_, name).await?;
        
        // 更新缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, (info.clone(), Utc::now()));
        }
        
        Ok(info)
    }
}

#[async_trait]
impl MCPTool for CheckVersionTool {
    fn name(&self) -> &str {
        "check_latest_version"
    }
    
    fn description(&self) -> &str {
        "在需要了解包的最新版本、版本历史、发布日期或版本兼容性信息时，获取指定包的版本详情，包括最新稳定版、预览版、发布时间和下载地址。"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["type".to_string(), "name".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert(
                        "type".to_string(),
                        Schema::String(SchemaString {
                            description: Some("包所属的包管理器类型(cargo/npm/pip/maven/go/pub)".to_string()),
                            ..Default::default()
                        }),
                    );
                    map.insert(
                        "name".to_string(),
                        Schema::String(SchemaString {
                            description: Some("要查询版本信息的包名称".to_string()),
                            ..Default::default()
                        }),
                    );
                    map.insert(
                        "include_preview".to_string(),
                        Schema::Boolean(SchemaBoolean {
                            description: Some("是否包含预览版本".to_string()),
                        }),
                    );
                    map
                },
                ..Default::default()
            })
        })
    }

    async fn execute(&self, parameters: Value) -> Result<Value> {
        let type_ = parameters["type"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("缺少type参数".to_string()))?;
            
        let name = parameters["name"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("缺少name参数".to_string()))?;
            
        let _include_preview = parameters["include_preview"]
            .as_bool()
            .unwrap_or(false);

        let info = self.get_version_info(type_, name).await?;
        
        Ok(json!({
            "latest_stable": info.latest_stable,
            "latest_preview": info.latest_preview,
            "release_date": info.release_date,
            "eol_date": info.eol_date,
            "download_url": info.download_url,
            "package_type": info.package_type,
            "available_versions": info.available_versions,
            "dependencies": info.dependencies,
            "repository_url": info.repository_url,
        }))
    }
}
