use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{info, debug};

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;

/// Java文档工具 - 专门处理Java语言的文档生成和搜索
pub struct JavaDocsTool {
    /// 缓存已生成的文档
    cache: Arc<tokio::sync::RwLock<HashMap<String, Value>>>,
}

impl JavaDocsTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 生成Java库的文档
    async fn generate_java_docs(&self, artifact_name: &str, version: Option<&str>) -> Result<Value> {
        let cache_key = format!("{}:{}", artifact_name, version.unwrap_or("latest"));
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_docs) = cache.get(&cache_key) {
                debug!("从缓存返回Java文档: {}", cache_key);
                return Ok(cached_docs.clone());
            }
        }

        info!("生成Java库文档: {}", artifact_name);

        // 尝试从多个源获取Java文档
        let docs = self.fetch_java_docs_from_sources(artifact_name, version).await?;

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, docs.clone());
        }

        Ok(docs)
    }

    /// 从多个源获取Java文档
    async fn fetch_java_docs_from_sources(&self, artifact_name: &str, version: Option<&str>) -> Result<Value> {
        // 1. 尝试从Maven Central获取包信息
        if let Ok(maven_docs) = self.fetch_from_maven_central(artifact_name, version).await {
            return Ok(maven_docs);
        }

        // 2. 尝试从Javadoc.io获取文档
        if let Ok(javadoc_docs) = self.fetch_from_javadoc_io(artifact_name, version).await {
            return Ok(javadoc_docs);
        }

        // 3. 尝试从GitHub获取README
        if let Ok(github_docs) = self.fetch_from_github(artifact_name).await {
            return Ok(github_docs);
        }

        // 4. 生成基础文档结构
        Ok(self.generate_basic_java_docs(artifact_name, version))
    }

    /// 从Maven Central获取包信息
    async fn fetch_from_maven_central(&self, artifact_name: &str, version: Option<&str>) -> Result<Value> {
        let client = reqwest::Client::new();
        
        // 尝试解析 groupId:artifactId 格式
        let (group_id, artifact_id) = if artifact_name.contains(':') {
            let parts: Vec<&str> = artifact_name.split(':').collect();
            if parts.len() >= 2 {
                (parts[0], parts[1])
            } else {
                return Err(MCPError::InvalidParameter("无效的Maven坐标格式".into()).into());
            }
        } else {
            // 如果没有groupId，尝试搜索
            return self.search_maven_central(artifact_name).await;
        };

        let url = format!(
            "https://search.maven.org/solrsearch/select?q=g:\"{}\"AND+a:\"{}\"&rows=20&wt=json",
            group_id, artifact_id
        );

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Maven库不存在: {}", artifact_name)).into());
        }

        let maven_data: Value = response.json().await?;
        Ok(self.parse_maven_central_response(&maven_data, artifact_name, version))
    }

    /// 搜索Maven Central
    async fn search_maven_central(&self, artifact_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://search.maven.org/solrsearch/select?q=a:\"{}\"&rows=20&wt=json",
            artifact_name
        );

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Maven库不存在: {}", artifact_name)).into());
        }

        let maven_data: Value = response.json().await?;
        Ok(self.parse_maven_search_response(&maven_data, artifact_name))
    }

    /// 解析Maven Central响应
    fn parse_maven_central_response(&self, maven_data: &Value, artifact_name: &str, version: Option<&str>) -> Value {
        if let Some(docs) = maven_data["response"]["docs"].as_array() {
            if let Some(first_doc) = docs.first() {
                let group_id = first_doc["g"].as_str().unwrap_or("unknown");
                let artifact_id = first_doc["a"].as_str().unwrap_or(artifact_name);
                let latest_version = first_doc["latestVersion"].as_str().unwrap_or("unknown");
                let version_count = first_doc["versionCount"].as_u64().unwrap_or(0);

                return json!({
                    "artifact_name": artifact_name,
                    "group_id": group_id,
                    "artifact_id": artifact_id,
                    "version": version.unwrap_or(latest_version),
                    "latest_version": latest_version,
                    "language": "java",
                    "source": "maven_central",
                    "version_count": version_count,
                    "documentation": {
                        "type": "maven_info",
                        "content": format!("Java库: {}:{}", group_id, artifact_id),
                        "maven_coordinate": format!("{}:{}:{}", group_id, artifact_id, latest_version)
                    },
                    "installation": {
                        "maven": format!("<dependency>\n  <groupId>{}</groupId>\n  <artifactId>{}</artifactId>\n  <version>{}</version>\n</dependency>", group_id, artifact_id, latest_version),
                        "gradle": format!("implementation '{}:{}:{}'", group_id, artifact_id, latest_version),
                        "sbt": format!("libraryDependencies += \"{}\" % \"{}\" % \"{}\"", group_id, artifact_id, latest_version)
                    },
                    "links": {
                        "maven_central": format!("https://search.maven.org/artifact/{}/{}", group_id, artifact_id),
                        "mvn_repository": format!("https://mvnrepository.com/artifact/{}/{}", group_id, artifact_id)
                    }
                });
            }
        }

        self.generate_basic_java_docs(artifact_name, version)
    }

    /// 解析Maven搜索响应
    fn parse_maven_search_response(&self, maven_data: &Value, artifact_name: &str) -> Value {
        let mut results = Vec::new();

        if let Some(docs) = maven_data["response"]["docs"].as_array() {
            for doc in docs.iter().take(5) {
                let group_id = doc["g"].as_str().unwrap_or("unknown");
                let artifact_id = doc["a"].as_str().unwrap_or("unknown");
                let latest_version = doc["latestVersion"].as_str().unwrap_or("unknown");

                results.push(json!({
                    "group_id": group_id,
                    "artifact_id": artifact_id,
                    "latest_version": latest_version,
                    "maven_coordinate": format!("{}:{}:{}", group_id, artifact_id, latest_version)
                }));
            }
        }

        json!({
            "artifact_name": artifact_name,
            "language": "java",
            "source": "maven_search",
            "search_results": results,
            "documentation": {
                "type": "search_results",
                "content": format!("找到 {} 个相关的Java库", results.len())
            }
        })
    }

    /// 从Javadoc.io获取文档
    async fn fetch_from_javadoc_io(&self, artifact_name: &str, version: Option<&str>) -> Result<Value> {
        // 解析Maven坐标
        let (group_id, artifact_id) = if artifact_name.contains(':') {
            let parts: Vec<&str> = artifact_name.split(':').collect();
            if parts.len() >= 2 {
                (parts[0], parts[1])
            } else {
                return Err(MCPError::InvalidParameter("无效的Maven坐标格式".into()).into());
            }
        } else {
            return Err(MCPError::InvalidParameter("需要完整的Maven坐标 (groupId:artifactId)".into()).into());
        };

        let client = reqwest::Client::new();
        let url = if let Some(v) = version {
            format!("https://javadoc.io/doc/{}/{}/{}/", group_id, artifact_id, v)
        } else {
            format!("https://javadoc.io/doc/{}/{}/", group_id, artifact_id)
        };

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Javadoc.io文档不存在: {}", artifact_name)).into());
        }

        Ok(json!({
            "artifact_name": artifact_name,
            "group_id": group_id,
            "artifact_id": artifact_id,
            "version": version.unwrap_or("latest"),
            "language": "java",
            "source": "javadoc.io",
            "documentation": {
                "type": "javadoc",
                "url": url,
                "content": format!("{}:{} 的完整Javadoc文档", group_id, artifact_id)
            },
            "installation": {
                "maven": format!("<dependency>\n  <groupId>{}</groupId>\n  <artifactId>{}</artifactId>\n  <version>{}</version>\n</dependency>", group_id, artifact_id, version.unwrap_or("latest")),
                "gradle": format!("implementation '{}:{}:{}'", group_id, artifact_id, version.unwrap_or("latest"))
            }
        }))
    }

    /// 从GitHub获取README
    async fn fetch_from_github(&self, artifact_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        
        // 提取artifact_id作为搜索关键词
        let search_term = if artifact_name.contains(':') {
            artifact_name.split(':').nth(1).unwrap_or(artifact_name)
        } else {
            artifact_name
        };

        let search_url = format!(
            "https://api.github.com/search/repositories?q={}&language:java&sort=stars",
            search_term
        );

        let response = client.get(&search_url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("GitHub仓库不存在: {}", artifact_name)).into());
        }

        let search_data: Value = response.json().await?;
        if let Some(items) = search_data["items"].as_array() {
            if let Some(first_repo) = items.first() {
                return Ok(self.parse_github_repo(first_repo, artifact_name));
            }
        }

        Err(MCPError::NotFound(format!("GitHub仓库不存在: {}", artifact_name)).into())
    }

    /// 解析GitHub仓库信息
    fn parse_github_repo(&self, repo_data: &Value, artifact_name: &str) -> Value {
        let description = repo_data["description"].as_str().unwrap_or("");
        let html_url = repo_data["html_url"].as_str().unwrap_or("");
        let language = repo_data["language"].as_str().unwrap_or("Java");
        let stars = repo_data["stargazers_count"].as_u64().unwrap_or(0);
        let forks = repo_data["forks_count"].as_u64().unwrap_or(0);

        json!({
            "artifact_name": artifact_name,
            "language": "java",
            "source": "github",
            "description": description,
            "repository_url": html_url,
            "programming_language": language,
            "stars": stars,
            "forks": forks,
            "documentation": {
                "type": "repository_readme",
                "content": description,
                "url": html_url
            },
            "installation": {
                "maven": format!("<!-- 在 pom.xml 中添加 -->\n<dependency>\n  <groupId>GROUP_ID</groupId>\n  <artifactId>{}</artifactId>\n  <version>VERSION</version>\n</dependency>", 
                    if artifact_name.contains(':') { artifact_name.split(':').nth(1).unwrap_or(artifact_name) } else { artifact_name }),
                "gradle": format!("implementation 'GROUP_ID:{}:VERSION'", 
                    if artifact_name.contains(':') { artifact_name.split(':').nth(1).unwrap_or(artifact_name) } else { artifact_name })
            }
        })
    }

    /// 生成基础Java文档
    fn generate_basic_java_docs(&self, artifact_name: &str, version: Option<&str>) -> Value {
        json!({
            "artifact_name": artifact_name,
            "version": version.unwrap_or("latest"),
            "language": "java",
            "source": "generated",
            "description": format!("Java库: {}", artifact_name),
            "documentation": {
                "type": "basic_template",
                "content": format!("这是 {} 的基础文档。", artifact_name),
                "sections": [
                    {
                        "title": "简介",
                        "content": format!("{} 是一个 Java 库。", artifact_name)
                    },
                    {
                        "title": "Maven安装",
                        "content": format!("<!-- 在 pom.xml 中添加 -->\n<dependency>\n  <groupId>GROUP_ID</groupId>\n  <artifactId>{}</artifactId>\n  <version>VERSION</version>\n</dependency>", 
                            if artifact_name.contains(':') { artifact_name.split(':').nth(1).unwrap_or(artifact_name) } else { artifact_name })
                    },
                    {
                        "title": "Gradle安装",
                        "content": format!("implementation 'GROUP_ID:{}:VERSION'", 
                            if artifact_name.contains(':') { artifact_name.split(':').nth(1).unwrap_or(artifact_name) } else { artifact_name })
                    },
                    {
                        "title": "使用方法",
                        "content": format!("import GROUP_ID.{};", 
                            if artifact_name.contains(':') { artifact_name.split(':').nth(1).unwrap_or(artifact_name) } else { artifact_name })
                    }
                ]
            },
            "installation": {
                "maven": format!("<!-- 在 pom.xml 中添加 -->\n<dependency>\n  <groupId>GROUP_ID</groupId>\n  <artifactId>{}</artifactId>\n  <version>VERSION</version>\n</dependency>", 
                    if artifact_name.contains(':') { artifact_name.split(':').nth(1).unwrap_or(artifact_name) } else { artifact_name }),
                "gradle": format!("implementation 'GROUP_ID:{}:VERSION'", 
                    if artifact_name.contains(':') { artifact_name.split(':').nth(1).unwrap_or(artifact_name) } else { artifact_name })
            },
            "links": {
                "maven_central": format!("https://search.maven.org/search?q={}", artifact_name),
                "mvn_repository": format!("https://mvnrepository.com/search?q={}", artifact_name)
            }
        })
    }
}

#[async_trait]
impl MCPTool for JavaDocsTool {
    fn name(&self) -> &'static str {
        "java_docs"
    }

    fn description(&self) -> &'static str {
        "在需要查找Java库的详细文档、API参考或使用示例时，获取来自Maven Central、Javadoc.io和GitHub的综合文档信息。"
    }

    fn parameters_schema(&self) -> &Schema {
        use std::sync::OnceLock;
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["artifact_name".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("artifact_name".to_string(), Schema::String(SchemaString {
                        description: Some("要查询文档的Java库名称（支持groupId:artifactId格式）".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("特定版本号（可选）".to_string()),
                        enum_values: None,
                    }));
                    map.insert("include_dependencies".to_string(), Schema::String(SchemaString {
                        description: Some("是否包含依赖信息".to_string()),
                        enum_values: Some(vec!["true".to_string(), "false".to_string()]),
                    }));
                    map
                },
                ..Default::default()
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let artifact_name = params["artifact_name"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("artifact_name 参数是必需的".into()))?;

        let version = params["version"].as_str();

        match self.generate_java_docs(artifact_name, version).await {
            Ok(docs) => Ok(docs),
            Err(e) => {
                debug!("生成Java文档失败: {}", e);
                // 返回基础文档而不是错误
                Ok(self.generate_basic_java_docs(artifact_name, version))
            }
        }
    }
}

impl Default for JavaDocsTool {
    fn default() -> Self {
        Self::new()
    }
} 