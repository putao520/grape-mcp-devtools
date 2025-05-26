use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use anyhow::Result;
use crate::errors::MCPError;
use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use regex::Regex;

// 简单的Schema构建助手
struct SchemaBuilder {
    schema: SchemaObject
}

impl SchemaBuilder {
    fn new() -> Self {
        Self {
            schema: SchemaObject::default()
        }
    }
    
    fn add_string_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut str_schema = SchemaString::default();
        str_schema.description = Some(description.to_string());
        
        self.schema.properties.insert(
            name.to_string(), 
            Schema::String(str_schema)
        );
        
        if required {
            self.schema.required.push(name.to_string());
        }
        
        self
    }
    
    fn build(self) -> Schema {
        Schema::Object(self.schema)
    }
}

/// 定义文档获取器trait
#[async_trait]
pub trait DocsFetcher: Send + Sync {
    /// 获取指定符号的API文档
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value>;
}

/// Rust文档获取器
pub struct RustDocsFetcher {
    client: reqwest::Client,
    base_url: String,
}

impl RustDocsFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://docs.rs".to_string(),
        }
    }

    async fn fetch_crate_data(&self, package: &str, version: Option<&str>) -> Result<Value> {
        let url = match version {
            Some(v) => format!("{}/api/{}/{}/", self.base_url, package, v),
            None => format!("{}/api/{}/latest/", self.base_url, package),
        };

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| MCPError::ServerError(format!("获取Rust包文档失败: {}", e)))?;

        if !response.status().is_success() {                return Err(MCPError::NotFound(format!(
                    "未找到包 {} {} 的文档",
                    package,
                    version.unwrap_or("最新版本")
                )).into());
        }

        let text = response.text().await
            .map_err(|e| MCPError::ServerError(format!("解析文档响应失败: {}", e)))?;

        Ok(json!({ "raw_doc": text }))
    }

    fn parse_rust_doc(&self, raw_doc: &str, symbol: &str) -> Result<Value> {
        // 实现Rust文档解析逻辑,支持:
        // - 函数文档
        // - 结构体文档  
        // - trait文档
        // - 模块文档
        
        let mut doc_type = "unknown";
        let mut signature = String::new();
        let mut description = String::new();
        let mut parameters = Vec::new();
        let mut returns = json!({"type": "()", "description": ""});
        let mut examples = Vec::new();
        
        // 解析函数
        let fn_regex = Regex::new(r"pub\s+fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^{;]+))?").unwrap();
        if let Some(captures) = fn_regex.captures(raw_doc) {
            if &captures[1] == symbol {
                doc_type = "function";
                signature = captures[0].trim().to_string();
                
                // 解析参数
                let params_str = &captures[2];
                if !params_str.trim().is_empty() {
                    for param in params_str.split(',') {
                        let param = param.trim();
                        if let Some(colon_pos) = param.find(':') {
                            let name = param[..colon_pos].trim();
                            let type_str = param[colon_pos + 1..].trim();
                            parameters.push(json!({
                                "name": name,
                                "type": type_str,
                                "description": ""
                            }));
                        }
                    }
                }
                
                // 解析返回类型
                if let Some(return_type) = captures.get(3) {
                    returns = json!({
                        "type": return_type.as_str().trim(),
                        "description": ""
                    });
                }
            }
        }
        
        // 解析结构体
        let struct_regex = Regex::new(r"pub\s+struct\s+(\w+)").unwrap();
        if let Some(captures) = struct_regex.captures(raw_doc) {
            if &captures[1] == symbol {
                doc_type = "struct";
                signature = captures[0].trim().to_string();
            }
        }
        
        // 解析trait
        let trait_regex = Regex::new(r"pub\s+trait\s+(\w+)").unwrap();
        if let Some(captures) = trait_regex.captures(raw_doc) {
            if &captures[1] == symbol {
                doc_type = "trait";
                signature = captures[0].trim().to_string();
            }
        }
        
        // 解析模块
        let mod_regex = Regex::new(r"pub\s+mod\s+(\w+)").unwrap();
        if let Some(captures) = mod_regex.captures(raw_doc) {
            if &captures[1] == symbol {
                doc_type = "module";
                signature = captures[0].trim().to_string();
            }
        }
        
        // 提取文档注释
        let doc_comment_regex = Regex::new(r"///\s*(.+)").unwrap();
        let mut doc_lines = Vec::new();
        let mut in_example = false;
        let mut current_example = String::new();
        
        for line in raw_doc.lines() {
            if let Some(captures) = doc_comment_regex.captures(line) {
                let comment = captures[1].trim().to_string();
                
                if comment.starts_with("```") {
                    if in_example {
                        // 结束示例
                        if !current_example.trim().is_empty() {
                            examples.push(json!({
                                "code": current_example.trim(),
                                "language": "rust"
                            }));
                        }
                        current_example.clear();
                        in_example = false;
                    } else {
                        // 开始示例
                        in_example = true;
                    }
                } else if in_example {
                    current_example.push_str(&comment);
                    current_example.push('\n');
                } else if comment.starts_with("# ") {
                    // 示例标题，忽略
                } else {
                    doc_lines.push(comment);
                }
            }
        }
        
        description = doc_lines.join(" ");
        
        Ok(json!({
            "name": symbol,
            "type": doc_type,
            "signature": signature,
            "description": description,
            "parameters": parameters,
            "returns": returns,
            "examples": examples
        }))
    }
}

#[async_trait]
impl DocsFetcher for RustDocsFetcher {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value> {
        let crate_data = self.fetch_crate_data(package, version).await?;
        let raw_doc = crate_data["raw_doc"].as_str()
            .ok_or_else(|| MCPError::ServerError("无效的文档数据格式".to_string()))?;
            
        self.parse_rust_doc(raw_doc, symbol)
    }
}

/// Python文档获取器
pub struct PythonDocsFetcher {
    client: reqwest::Client,
    pypi_url: String,
    rtd_url: String,
}

impl PythonDocsFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            pypi_url: "https://pypi.org/pypi".to_string(),
            rtd_url: "https://readthedocs.org/api/v3".to_string(),
        }
    }

    async fn fetch_package_info(&self, package: &str, version: Option<&str>) -> Result<Value> {
        let url = match version {
            Some(v) => format!("{}/{}/{}/json", self.pypi_url, package, v),
            None => format!("{}/{}/json", self.pypi_url, package),
        };

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| MCPError::ServerError(format!("获取Python包信息失败: {}", e)))?;

        if !response.status().is_success() {                return Err(MCPError::NotFound(format!(
                    "未找到包 {} {} 的信息",
                    package,
                    version.unwrap_or("最新版本")
                )).into());
        }        response.json()
            .await
            .map_err(|e| MCPError::ServerError(format!("解析包信息失败: {}", e)).into())
    }

    async fn fetch_rtd_docs(&self, package: &str, _version: Option<&str>) -> Result<Value> {
        let url = format!("{}/projects/{}", self.rtd_url, package);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| MCPError::ServerError(format!("获取readthedocs文档失败: {}", e)))?;

        if !response.status().is_success() {                return Err(MCPError::NotFound(format!(
                    "未找到包 {} 的readthedocs文档",
                    package
                )).into());
        }        response.json()
            .await
            .map_err(|e| MCPError::ServerError(format!("解析readthedocs响应失败: {}", e)).into())
    }

    fn parse_python_doc(&self, _package_info: Value, _rtd_info: Option<Value>, symbol: &str) -> Result<Value> {
        use regex::Regex;
        
        // 实现Python文档解析逻辑,支持:
        // - 函数文档
        // - 类文档
        // - 模块文档
        // - docstring解析
        
        // 注意：由于这是从包信息解析，我们主要从描述中提取信息
        // 在实际实现中，需要访问实际的Python源码或生成的文档
        
        let mut doc_type = "function";
        let mut signature = format!("def {}():", symbol);
        let mut description = String::new();
        let mut parameters = Vec::new();
        let mut returns = json!({"type": "None", "description": ""});
        let mut examples = Vec::new();
        
        // 从包信息中提取描述
        if let Some(info) = _package_info.get("info") {
            if let Some(desc) = info.get("description").and_then(|d| d.as_str()) {
                description = desc.to_string();
                
                // 解析docstring格式的内容
                let lines: Vec<&str> = desc.lines().collect();
                let mut current_section = None;
                let mut current_content = String::new();
                
                for line in lines {
                    let trimmed = line.trim();
                    
                    // 检测各种section
                    if trimmed.starts_with("Args:") || trimmed.starts_with("Parameters:") {
                        current_section = Some("args");
                        current_content.clear();
                    } else if trimmed.starts_with("Returns:") || trimmed.starts_with("Return:") {
                        current_section = Some("returns");
                        current_content.clear();
                    } else if trimmed.starts_with("Examples:") || trimmed.starts_with("Example:") {
                        current_section = Some("examples");
                        current_content.clear();
                    } else if trimmed.starts_with("Raises:") || trimmed.starts_with("Exceptions:") {
                        current_section = Some("raises");
                        current_content.clear();
                    } else if !trimmed.is_empty() {
                        match current_section {
                            Some("args") => {
                                // 解析参数格式: param_name (type): description
                                let param_regex = Regex::new(r"(\w+)\s*\(([^)]+)\)\s*:\s*(.+)").unwrap();
                                if let Some(captures) = param_regex.captures(trimmed) {
                                    parameters.push(json!({
                                        "name": &captures[1],
                                        "type": &captures[2],
                                        "description": &captures[3]
                                    }));
                                }
                            }
                            Some("returns") => {
                                // 解析返回值格式: type: description
                                if let Some(colon_pos) = trimmed.find(':') {
                                    let return_type = trimmed[..colon_pos].trim();
                                    let return_desc = trimmed[colon_pos + 1..].trim();
                                    returns = json!({
                                        "type": return_type,
                                        "description": return_desc
                                    });
                                } else {
                                    returns = json!({
                                        "type": "Unknown",
                                        "description": trimmed
                                    });
                                }
                            }
                            Some("examples") => {
                                current_content.push_str(trimmed);
                                current_content.push('\n');
                                
                                // 如果有代码块，提取示例
                                if trimmed.starts_with(">>>") || trimmed.starts_with("```") {
                                    examples.push(json!({
                                        "code": trimmed,
                                        "language": "python"
                                    }));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // 检测类型
        if symbol.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            doc_type = "class";
            signature = format!("class {}:", symbol);
        } else if symbol.contains("module") || description.contains("module") {
            doc_type = "module";
            signature = symbol.to_string();
        }
        
        // 生成更智能的函数签名
        if doc_type == "function" && !parameters.is_empty() {
            let param_names: Vec<String> = parameters.iter()
                .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
                .map(|s| s.to_string())
                .collect();
            signature = format!("def {}({}):", symbol, param_names.join(", "));
        }
        
        Ok(json!({
            "name": symbol,
            "type": doc_type,
            "signature": signature,
            "description": description,
            "parameters": parameters,
            "returns": returns,
            "examples": examples
        }))
    }
}

#[async_trait]
impl DocsFetcher for PythonDocsFetcher {
    async fn fetch_docs(&self, package: &str, symbol: &str, version: Option<&str>) -> Result<Value> {
        let package_info = self.fetch_package_info(package, version).await?;
        let rtd_info = self.fetch_rtd_docs(package, version).await.ok();
        
        self.parse_python_doc(package_info, rtd_info, symbol)
    }
}

/// 文档获取器工厂
pub struct DocsFetcherFactory;

impl DocsFetcherFactory {
    pub fn create(language: &str) -> Result<Box<dyn DocsFetcher>> {
        match language.to_lowercase().as_str() {
            "rust" => Ok(Box::new(RustDocsFetcher {
                client: reqwest::Client::new(),
                base_url: "https://docs.rs".to_string(),
            })),
            "python" => Ok(Box::new(PythonDocsFetcher {
                client: reqwest::Client::new(),
                pypi_url: "https://pypi.org/pypi".to_string(),
                rtd_url: "https://readthedocs.org".to_string(),
            })),
            _ => Err(MCPError::InvalidParameter(format!("不支持的语言: {}", language)).into())
        }
    }
}

/// API文档缓存项
#[derive(Debug, Clone)]
struct ApiDocsCache {
    docs: Value,
    timestamp: SystemTime,
}

/// GetApiDocsTool 实现了获取API文档的功能
pub struct GetApiDocsTool {
    cache: Arc<RwLock<HashMap<String, ApiDocsCache>>>,
    cache_ttl: Duration,
    parameters_schema: Schema,
}

#[derive(Debug, Clone, Default)]
pub struct GetApiDocsConfig {
    /// 缓存过期时间(秒)
    pub cache_ttl_secs: u64,
    /// 最大缓存条目数
    pub max_cache_entries: usize,
}

impl GetApiDocsTool {
    pub fn new(config: Option<GetApiDocsConfig>) -> Self {
        let config = config.unwrap_or_default();
        let cache_ttl = Duration::from_secs(config.cache_ttl_secs.max(60));

        // 创建参数schema
        let parameters_schema = SchemaBuilder::new()
            .add_string_property("language", "编程语言", true)
            .add_string_property("package", "包名称", true)
            .add_string_property("symbol", "API符号", true)
            .add_string_property("version", "API版本", false)
            .build();

        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(
                config.max_cache_entries.max(100)
            ))),
            cache_ttl,
            parameters_schema,
        }
    }

    /// 生成缓存键
    fn cache_key(&self, language: &str, package: &str, symbol: &str, version: Option<&str>) -> String {
        match version {
            Some(v) => format!("{}:{}:{}:{}", language, package, symbol, v),
            None => format!("{}:{}:{}", language, package, symbol),
        }
    }

    /// 从缓存获取文档
    fn get_from_cache(&self, key: &str) -> Option<Value> {
        let cache = self.cache.read();
        if let Some(cached) = cache.get(key) {
            if cached.timestamp.elapsed().unwrap() < self.cache_ttl {
                return Some(cached.docs.clone());
            }
        }
        None
    }

    /// 更新缓存
    fn update_cache(&self, key: String, docs: Value) {
        let mut cache = self.cache.write();
        // 如果缓存已满,移除最旧的条目
        if cache.len() >= 1000 {              if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, v)| v.timestamp)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(key, ApiDocsCache {
            docs,
            timestamp: SystemTime::now(),
        });
    }

    /// 获取指定语言和符号的API文档
    async fn fetch_api_docs(&self, 
        language: &str, 
        package: &str, 
        symbol: &str, 
        version: Option<&str>
    ) -> Result<Value> {
        // 获取对应语言的文档获取器
        let fetcher = DocsFetcherFactory::create(language)?;
        
        // 尝试获取文档,最多重试3次
        let mut last_error = None;
        for _ in 0..3 {
            match fetcher.fetch_docs(package, symbol, version).await {
                Ok(docs) => return Ok(docs),
                Err(e) => last_error = Some(e),
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // 如果所有重试都失败了,返回最后一个错误
        Err(last_error.unwrap_or_else(|| 
            MCPError::ServerError("无法获取API文档".to_string()).into()
        ))
    }
}

#[async_trait]
impl MCPTool for GetApiDocsTool {
    fn name(&self) -> &str {
        "get_api_docs"
    }

    fn description(&self) -> &str {
        "获取编程语言API的详细文档信息"
    }

    fn parameters_schema(&self) -> &Schema {
        &self.parameters_schema
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        // 验证参数
        self.validate_params(&params)?;

        let language = params["language"].as_str()
            .ok_or_else(|| MCPError::InvalidParameter("language 参数缺失".to_string()))?;
        let package = params["package"].as_str()
            .ok_or_else(|| MCPError::InvalidParameter("package 参数缺失".to_string()))?;
        let symbol = params["symbol"].as_str()
            .ok_or_else(|| MCPError::InvalidParameter("symbol 参数缺失".to_string()))?;
        let version = params["version"].as_str();

        // 尝试从缓存获取
        let cache_key = self.cache_key(language, package, symbol, version);
        if let Some(cached) = self.get_from_cache(&cache_key) {
            return Ok(cached);
        }        // 获取新的文档
        let docs = match self.fetch_api_docs(language, package, symbol, version).await {
            Ok(docs) => docs,            Err(_) => {
                // 简化错误处理
                return Err(MCPError::NotFound(format!(
                    "未找到 {} 包中的 {} 符号文档。请检查包名和符号名是否正确,或尝试不同的版本。",
                    package, symbol
                )).into());
            }
        };
        
        // 更新缓存
        self.update_cache(cache_key, docs.clone());

        Ok(docs)
    }
    
    fn validate_params(&self, params: &Value) -> Result<()> {
        // 使用Schema验证参数
        match self.parameters_schema.validate(params) {
            Ok(()) => Ok(()),
            Err(e) => Err(MCPError::InvalidParameter(
                format!("参数验证失败: {}", e)
            ).into())
        }
    }
}
