use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use anyhow::Result;
use crate::errors::MCPError;
use super::base::{MCPTool, ToolAnnotations, Schema, SchemaObject, SchemaString, SchemaBoolean};
use regex;

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
    FlutterSdk,  // 新增: Flutter SDK
    DartSdk,     // 新增: Dart SDK
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
            Registry::FlutterSdk => "https://docs.flutter.dev",
            Registry::DartSdk => "https://api.github.com/repos/dart-lang/sdk",
        }
    }
}

pub struct CheckVersionTool {
    _annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, (VersionInfo, DateTime<Utc>)>>>,
    client: reqwest::Client,
}

impl CheckVersionTool {
    pub fn new() -> Self {
        // 创建带有User-Agent的HTTP客户端
        let client = reqwest::Client::builder()
            .user_agent("grape-mcp-devtools/2.0.0 (https://github.com/grape-mcp-devtools)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
            
        Self {
            _annotations: ToolAnnotations {
                category: "版本检查".to_string(),
                tags: vec!["版本".to_string(), "检查".to_string()],
                version: "1.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
            client,
        }
    }

    async fn fetch_version_info(&self, type_: &str, name: &str) -> Result<VersionInfo> {
        match type_ {
            "cargo" => self.fetch_crates_io(name).await,
            "npm" => self.fetch_npm(name).await,
            "pip" => self.fetch_pypi(name).await,
            "maven" => self.fetch_maven_central(name).await,
            "go" => self.fetch_go_proxy(name).await,
            "pub" => {
                // 特殊处理Flutter和Dart
                match name {
                    "flutter" => self.fetch_flutter_sdk().await,
                    "dart" => self.fetch_dart_sdk().await,
                    _ => self.fetch_pub_dev(name).await,
                }
            },
            "flutter" => self.fetch_flutter_sdk().await,  // 新增: 直接支持flutter类型
            "dart" => self.fetch_dart_sdk().await,        // 新增: 直接支持dart类型
            _ => Err(MCPError::NotFound(format!(
                "不支持的包类型: {}", type_
            )).into()),
        }
    }

    async fn fetch_flutter_sdk(&self) -> Result<VersionInfo> {
        // 从GitHub API获取Flutter SDK的最新版本
        let url = "https://api.github.com/repos/flutter/flutter/releases/latest";
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(MCPError::NotFound("无法获取Flutter SDK版本信息".to_string()).into());
        }
        
        let data: Value = response.json().await?;
        
        let tag_name = data["tag_name"]
            .as_str()
            .ok_or_else(|| MCPError::CacheError("无效的Flutter SDK响应".to_string()))?;
            
        let published_at = data["published_at"]
            .as_str()
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
            
        // 获取所有版本列表
        let all_releases_url = "https://api.github.com/repos/flutter/flutter/releases?per_page=50";
        let all_releases_response = self.client.get(all_releases_url).send().await?;
        let all_releases: Value = all_releases_response.json().await?;
        
        let available_versions = all_releases
            .as_array()
            .map(|releases| {
                releases.iter()
                    .filter_map(|release| release["tag_name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
            
        Ok(VersionInfo {
            latest_stable: tag_name.to_string(),
            latest_preview: None,
            release_date: published_at,
            eol_date: None,
            download_url: Some("https://docs.flutter.dev/get-started/install".to_string()),
            package_type: "flutter".to_string(),
            available_versions,
            dependencies: None,
            repository_url: Some("https://github.com/flutter/flutter".to_string()),
        })
    }
    
    async fn fetch_dart_sdk(&self) -> Result<VersionInfo> {
        // 从GitHub Tags API获取Dart SDK的版本信息
        let url = "https://api.github.com/repos/dart-lang/sdk/tags?per_page=100";
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(MCPError::NotFound("无法获取Dart SDK版本信息".to_string()).into());
        }
        
        let data: Value = response.json().await?;
        let tags = data.as_array()
            .ok_or_else(|| MCPError::CacheError("无效的Dart SDK响应".to_string()))?;
            
        // 过滤出Dart SDK版本标签（格式通常是数字.数字.数字）
        let mut dart_versions: Vec<String> = tags.iter()
            .filter_map(|tag| tag["name"].as_str())
            .filter(|name| {
                // 过滤出符合版本格式的标签，例如 "3.2.0", "2.19.6" 等
                let version_regex = regex::Regex::new(r"^\d+\.\d+\.\d+(-.*)?$").unwrap();
                version_regex.is_match(name)
            })
            .map(String::from)
            .collect();
            
        if dart_versions.is_empty() {
            return Err(MCPError::NotFound("未找到有效的Dart SDK版本".to_string()).into());
        }
        
        // 按版本号排序，获取最新版本
        dart_versions.sort_by(|a, b| {
            // 简单的版本比较，按字符串排序（对于大多数情况足够）
            b.cmp(a)
        });
        
        let latest_version = dart_versions.first()
            .ok_or_else(|| MCPError::CacheError("无法确定最新版本".to_string()))?;
            
        // 获取该版本的详细信息
        let tag_info_url = format!("https://api.github.com/repos/dart-lang/sdk/git/refs/tags/{}", latest_version);
        let tag_response = self.client.get(&tag_info_url).send().await;
        
        let release_date = if let Ok(tag_resp) = tag_response {
            if let Ok(tag_data) = tag_resp.json::<Value>().await {
                // 尝试从tag信息中获取准确的提交日期
                tag_data["object"]["url"].as_str()
                    .and_then(|_commit_url| {
                        // 可以进一步调用GitHub API获取commit的具体日期
                        // 这里返回None以使用当前时间作为fallback
                        None
                    })
                    .unwrap_or_else(Utc::now)
            } else {
                Utc::now()
            }
        } else {
            Utc::now()
        };
            
        Ok(VersionInfo {
            latest_stable: latest_version.clone(),
            latest_preview: None,
            release_date,
            eol_date: None,
            download_url: Some("https://dart.dev/get-dart".to_string()),
            package_type: "dart".to_string(),
            available_versions: dart_versions,
            dependencies: None,
            repository_url: Some("https://github.com/dart-lang/sdk".to_string()),
        })
    }

    async fn fetch_crates_io(&self, name: &str) -> Result<VersionInfo> {
        let url = format!("{}/crates/{}", Registry::CratesIo.base_url(), name);
        let response = self.client.get(&url).send().await?;
        
        // 检查响应状态
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("未找到Rust包: {}", name)).into());
        }
        
        let data: Value = response.json().await?;

        // 修复：使用正确的字段名
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

        // 修复：使用正确的字段名获取最新版本
        let latest_version = crate_data["newest_version"]
            .as_str()
            .or_else(|| crate_data["max_version"].as_str())
            .unwrap_or("0.0.0");

        Ok(VersionInfo {
            latest_stable: latest_version.to_string(),
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
        // 对于 "org.springframework:spring-core" 格式，需要分离groupId和artifactId
        let (group_id, artifact_id) = if name.contains(':') {
            let parts: Vec<&str> = name.split(':').collect();
            if parts.len() >= 2 {
                (parts[0], parts[1])
            } else {
                ("", name)
            }
        } else {
            ("", name)
        };
        
        let url = if !group_id.is_empty() {
            format!(
                "{}?q=g:\"{}\" AND a:\"{}\"&core=gav&rows=20&wt=json",
                Registry::MavenCentral.base_url(),
                group_id,
                artifact_id
            )
        } else {
            format!(
                "{}?q=a:\"{}\"&core=gav&rows=20&wt=json",
                Registry::MavenCentral.base_url(),
                name
            )
        };
        
        let response = self.client.get(&url).send().await?;
        
        // 检查响应状态
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("未找到Maven包: {}", name)).into());
        }
        
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
                latest["g"].as_str().unwrap_or(group_id),
                artifact_id
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
                            description: Some("包所属的包管理器类型(cargo/npm/pip/maven/go/pub/flutter/dart)，其中flutter和dart为SDK版本检查".to_string()),
                            ..Default::default()
                        }),
                    );
                    map.insert(
                        "name".to_string(),
                        Schema::String(SchemaString {
                            description: Some("要查询版本信息的包名称，对于flutter和dart类型，name参数会被忽略".to_string()),
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
