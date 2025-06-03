use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{info, debug, warn};
use lazy_static::lazy_static;
use regex;

use crate::tools::base::{MCPTool, Schema, SchemaObject, SchemaString};
use crate::errors::MCPError;

lazy_static! {
    static ref FLUTTER_DOCS_SCHEMA: Schema = Schema::Object(SchemaObject {
        properties: {
            let mut props = std::collections::HashMap::new();
            props.insert("widget_name".to_string(), Schema::String(SchemaString {
                description: Some("Flutter Widget名称 (如: Container, Text, ListView)".to_string()),
                enum_values: None,
            }));
            props.insert("package".to_string(), Schema::String(SchemaString {
                description: Some("pub.dev包名称".to_string()),
                enum_values: None,
            }));
            props.insert("flutter_version".to_string(), Schema::String(SchemaString {
                description: Some("Flutter版本 (可选，默认为latest)".to_string()),
                enum_values: None,
            }));
            props.insert("include_samples".to_string(), Schema::String(SchemaString {
                description: Some("是否包含示例代码 (true/false)".to_string()),
                enum_values: Some(vec!["true".to_string(), "false".to_string()]),
            }));
            props
        },
        required: vec![],
        description: Some("Flutter/Dart文档工具参数".to_string()),
    });
}

/// Flutter文档工具 - 专门处理Flutter/Dart语言的文档生成和搜索
pub struct FlutterDocsTool {
    /// 缓存已生成的文档
    cache: Arc<tokio::sync::RwLock<HashMap<String, Value>>>,
}

impl FlutterDocsTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 生成Flutter包或Widget的文档
    async fn generate_flutter_docs(&self, widget_name: Option<&str>, package: Option<&str>, flutter_version: Option<&str>) -> Result<Value> {
        let cache_key = format!("{}:{}:{}", 
            widget_name.unwrap_or(""), 
            package.unwrap_or(""), 
            flutter_version.unwrap_or("latest")
        );
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_docs) = cache.get(&cache_key) {
                debug!("从缓存返回Flutter文档: {}", cache_key);
                return Ok(cached_docs.clone());
            }
        }

        info!("生成Flutter文档: widget={:?}, package={:?}", widget_name, package);

        let docs = if let Some(widget) = widget_name {
            self.fetch_widget_docs(widget, flutter_version).await?
        } else if let Some(pkg) = package {
            self.fetch_package_docs(pkg, flutter_version).await?
        } else {
            self.generate_basic_flutter_docs()
        };

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, docs.clone());
        }

        Ok(docs)
    }

    /// 获取Widget文档
    async fn fetch_widget_docs(&self, widget_name: &str, flutter_version: Option<&str>) -> Result<Value> {
        let cached_result = {
            let cache = self.cache.read().await;
            cache.get(&format!("widget_{}_{}", widget_name, flutter_version.unwrap_or("latest"))).cloned()
        };

        if let Some(result) = cached_result {
            return Ok(result);
        }

        let mut result = self.fetch_from_flutter_api(widget_name, flutter_version).await?;
        
        // 添加生成的示例和额外信息
        let widget_description = format!("Flutter {} Widget documentation", widget_name);
        
        result = json!({
            "type": "widget_docs",
            "widget_name": widget_name,
            "description": widget_description,
            "documentation": {
                "type": "widget",
                "url": format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name)
            },
            "examples": self.generate_widget_examples(widget_name).await,
            "performance_tips": self.get_widget_performance_tips(widget_name),
            "platform_compatibility": self.get_platform_compatibility(widget_name),
            "flutter_version": flutter_version.unwrap_or("latest"),
            "documentation_url": format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name),
            "api_reference": format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name)
        });

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.insert(format!("widget_{}_{}", widget_name, flutter_version.unwrap_or("latest")), result.clone());
        }

        Ok(result)
    }

    /// 从Flutter API文档获取Widget信息
    async fn fetch_from_flutter_api(&self, widget_name: &str, flutter_version: Option<&str>) -> Result<Value> {
        let client = reqwest::Client::new();
        let version = flutter_version.unwrap_or("stable");
        let url = format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name);

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("Flutter Widget文档不存在: {}", widget_name)).into());
        }

        let html_content = response.text().await?;
        Ok(self.parse_flutter_api_html(&html_content, widget_name, version).await)
    }

    /// 解析Flutter API HTML内容
    async fn parse_flutter_api_html(&self, html_content: &str, widget_name: &str, version: &str) -> Value {
        // 先完成所有HTML解析工作，避免跨await边界
        let (description, constructors, properties) = {
            use scraper::{Html, Selector};

            let document = Html::parse_document(html_content);
            
            // 提取Widget描述
            let desc_selector = Selector::parse(".desc").unwrap();
            let description = document
                .select(&desc_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            // 提取构造函数信息
            let constructor_selector = Selector::parse(".constructor").unwrap();
            let constructors: Vec<String> = document
                .select(&constructor_selector)
                .map(|el| el.text().collect::<String>())
                .collect();

            // 提取属性信息
            let properties_selector = Selector::parse(".property").unwrap();
            let properties: Vec<String> = document
                .select(&properties_selector)
                .map(|el| el.text().collect::<String>())
                .collect();
                
            (description, constructors, properties)
        };

        // 现在可以安全地使用await
        let examples = self.generate_widget_examples(widget_name).await;

        json!({
            "widget_name": widget_name,
            "language": "dart",
            "framework": "flutter",
            "version": version,
            "source": "flutter_api",
            "description": description,
            "documentation": {
                "type": "widget_docs",
                "constructors": constructors,
                "properties": properties,
                "url": format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name)
            },
            "examples": examples,
            "performance_tips": self.get_widget_performance_tips(widget_name),
            "platform_compatibility": self.get_platform_compatibility(widget_name),
            "flutter_version": version,
            "documentation_url": format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name),
            "api_reference": format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name)
        })
    }

    /// 从pub.dev获取包文档
    async fn fetch_package_docs(&self, package_name: &str, flutter_version: Option<&str>) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = format!("https://pub.dev/api/packages/{}", package_name);

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("pub.dev包不存在: {}", package_name)).into());
        }

        let pub_data: Value = response.json().await?;
        Ok(self.parse_pub_dev_response(&pub_data, package_name, flutter_version))
    }

    /// 从pub.dev搜索相关包
    async fn fetch_from_pub_dev(&self, widget_name: &str) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = format!("https://pub.dev/api/search?q={}", widget_name);

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(MCPError::NotFound(format!("pub.dev搜索失败: {}", widget_name)).into());
        }

        let search_data: Value = response.json().await?;
        Ok(self.parse_pub_search_response(&search_data, widget_name))
    }

    /// 解析pub.dev包响应
    fn parse_pub_dev_response(&self, pub_data: &Value, package_name: &str, flutter_version: Option<&str>) -> Value {
        let latest = pub_data.get("latest").unwrap_or(&Value::Null);
        let version = latest.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        let description = latest.get("pubspec")
            .and_then(|p| p.get("description"))
            .and_then(|d| d.as_str())
            .unwrap_or("");

        json!({
            "package_name": package_name,
            "language": "dart",
            "framework": "flutter",
            "version": version,
            "flutter_version": flutter_version.unwrap_or("any"),
            "source": "pub_dev",
            "description": description,
            "documentation": {
                "type": "package_docs",
                "content": description,
                "url": format!("https://pub.dev/packages/{}", package_name)
            },
            "installation": {
                "pubspec": format!("dependencies:\n  {}: ^{}", package_name, version),
                "flutter_pub_get": "flutter pub get"
            },
            "examples": self.generate_package_examples(package_name),
            "platform_compatibility": self.get_package_platform_compatibility(package_name)
        })
    }

    /// 解析pub.dev搜索响应
    fn parse_pub_search_response(&self, search_data: &Value, widget_name: &str) -> Value {
        let empty_packages = vec![];
        let packages = search_data.get("packages").and_then(|p| p.as_array()).unwrap_or(&empty_packages);
        
        let relevant_packages: Vec<Value> = packages.iter()
            .take(5)
            .map(|pkg| {
                let name = pkg.get("package").and_then(|n| n.as_str()).unwrap_or("");
                let description = pkg.get("description").and_then(|d| d.as_str()).unwrap_or("");
                json!({
                    "name": name,
                    "description": description,
                    "url": format!("https://pub.dev/packages/{}", name)
                })
            })
            .collect();

        json!({
            "search_query": widget_name,
            "language": "dart",
            "framework": "flutter",
            "source": "pub_dev_search",
            "relevant_packages": relevant_packages,
            "documentation": {
                "type": "search_results",
                "packages_found": relevant_packages.len()
            }
        })
    }

    /// 生成基础Widget文档
    async fn generate_basic_widget_docs(&self, widget_name: &str, flutter_version: Option<&str>) -> Value {
        json!({
            "type": "widget_docs",
            "widget_name": widget_name,
            "description": format!("Basic {} widget documentation", widget_name),
            "documentation": {
                "type": "basic_widget",
                "content": format!("{} is a Flutter widget", widget_name),
                "reference_url": "https://flutter.dev/docs/development/ui/widgets"
            },
            "examples": self.generate_widget_examples(widget_name).await,
            "performance_tips": self.get_widget_performance_tips(widget_name),
            "platform_compatibility": self.get_platform_compatibility(widget_name),
            "flutter_version": flutter_version.unwrap_or("latest")
        })
    }

    /// 生成基础Flutter文档
    fn generate_basic_flutter_docs(&self) -> Value {
        json!({
            "language": "dart",
            "framework": "flutter",
            "source": "generated",
            "description": "Flutter framework documentation",
            "documentation": {
                "type": "framework_docs",
                "content": "Flutter is Google's UI toolkit for building beautiful, natively compiled applications",
                "getting_started": "https://flutter.dev/docs/get-started",
                "widget_catalog": "https://flutter.dev/docs/development/ui/widgets",
                "cookbook": "https://flutter.dev/docs/cookbook"
            },
            "installation": {
                "flutter_install": "https://flutter.dev/docs/get-started/install",
                "verify_install": "flutter doctor"
            },
            "platform_support": {
                "android": true,
                "ios": true,
                "web": true,
                "desktop": true,
                "linux": true,
                "macos": true,
                "windows": true
            }
        })
    }

    /// 生成Widget示例代码
    async fn generate_widget_examples(&self, widget_name: &str) -> Vec<Value> {
        // 首先尝试从Flutter官方API获取真实示例
        if let Ok(examples) = self.fetch_real_widget_examples(widget_name).await {
            if !examples.is_empty() {
                return examples;
            }
        }
        
        // 如果无法获取真实示例，生成基于模式的动态示例
        self.generate_dynamic_widget_examples(widget_name)
    }
    
    /// 从Flutter官方API获取真实Widget示例
    async fn fetch_real_widget_examples(&self, widget_name: &str) -> Result<Vec<Value>> {
        let mut examples = Vec::new();
        
        // 尝试多个Flutter文档源
        let api_urls = vec![
            format!("https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name),
            format!("https://master-api.flutter.dev/flutter/widgets/{}.html", widget_name),
        ];
        
        for url in api_urls {
            if let Ok(response) = reqwest::get(&url).await {
                if response.status().is_success() {
                    if let Ok(html_content) = response.text().await {
                        // 解析HTML中的代码示例
                        examples.extend(self.extract_examples_from_flutter_html(&html_content, widget_name));
                        if !examples.is_empty() {
                            info!("✅ 从Flutter API获取到 {} 个真实示例: {}", examples.len(), widget_name);
                            return Ok(examples);
                        }
                    }
                }
            }
        }
        
        // 尝试从Flutter GitHub示例仓库获取
        let github_url = format!("https://api.github.com/search/code?q={}+in:file+repo:flutter/flutter+path:examples", widget_name);
        if let Ok(response) = reqwest::get(&github_url).await {
            if let Ok(search_data) = response.json::<Value>().await {
                if let Some(items) = search_data["items"].as_array() {
                    for item in items.iter().take(3) {
                        if let Some(download_url) = item["download_url"].as_str() {
                            if let Ok(code_response) = reqwest::get(download_url).await {
                                if let Ok(code_content) = code_response.text().await {
                                    if code_content.contains(widget_name) {
                                        examples.push(json!({
                                            "title": format!("Flutter官方示例: {}", widget_name),
                                            "code": self.extract_widget_usage_from_code(&code_content, widget_name),
                                            "source": "flutter/flutter",
                                            "url": download_url
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(examples)
    }
    
    /// 从Flutter HTML文档中提取代码示例
    fn extract_examples_from_flutter_html(&self, html_content: &str, widget_name: &str) -> Vec<Value> {
        let mut examples = Vec::new();
        
        // 寻找代码块 <pre><code>...</code></pre>
        let code_pattern = regex::Regex::new(r"<pre[^>]*><code[^>]*>(.*?)</code></pre>").unwrap();
        
        for (i, capture) in code_pattern.captures_iter(html_content).enumerate() {
            if let Some(code_match) = capture.get(1) {
                let code = code_match.as_str()
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"");
                    
                if code.contains(widget_name) && code.len() > 10 {
                    examples.push(json!({
                        "title": format!("Flutter文档示例 {}", i + 1),
                        "code": code.trim(),
                        "source": "Flutter官方文档"
                    }));
                }
            }
        }
        
        examples
    }
    
    /// 从代码中提取特定Widget的使用方式
    fn extract_widget_usage_from_code(&self, code_content: &str, widget_name: &str) -> String {
        let lines: Vec<&str> = code_content.lines().collect();
        let mut extracted_lines = Vec::new();
        let mut found_widget = false;
        let mut brace_count = 0;
        
        for line in lines {
            if line.contains(widget_name) && line.contains("(") {
                found_widget = true;
                extracted_lines.push(line.to_string());
                brace_count += line.matches('(').count() as i32;
                brace_count -= line.matches(')').count() as i32;
                
                if brace_count <= 0 {
                    break;
                }
            } else if found_widget {
                extracted_lines.push(line.to_string());
                brace_count += line.matches('(').count() as i32;
                brace_count -= line.matches(')').count() as i32;
                
                if brace_count <= 0 {
                    break;
                }
            }
        }
        
        if extracted_lines.is_empty() {
            format!("{}(\n  // Flutter Widget用法\n)", widget_name)
        } else {
            extracted_lines.join("\n")
        }
    }
    
    /// 生成基于模式的动态Widget示例（当无法获取真实示例时）
    fn generate_dynamic_widget_examples(&self, widget_name: &str) -> Vec<Value> {
        // 根据Widget名称的模式生成更智能的示例
        let widget_lower = widget_name.to_lowercase();
        
        let mut examples = Vec::new();
        
        // 基础示例
        examples.push(json!({
            "title": format!("基础 {} 用法", widget_name),
            "code": format!("{}(\n  // TODO: 添加必要的属性\n  // 查看Flutter文档获取具体API\n)", widget_name),
            "note": "这是基于Widget名称生成的模板，实际用法请参考Flutter官方文档"
        }));
        
        // 根据Widget类型添加特定示例
        if widget_lower.contains("button") {
            examples.push(json!({
                "title": format!("{} 带回调", widget_name),
                "code": format!("{}(\n  onPressed: () {{\n    // 按钮点击处理\n  }},\n  child: Text('点击我'),\n)", widget_name)
            }));
        } else if widget_lower.contains("text") {
            examples.push(json!({
                "title": format!("{} 样式示例", widget_name),
                "code": format!("{}(\n  'Hello World',\n  style: TextStyle(\n    fontSize: 16,\n    fontWeight: FontWeight.bold,\n  ),\n)", widget_name)
            }));
        } else if widget_lower.contains("container") || widget_lower.contains("box") {
            examples.push(json!({
                "title": format!("{} 装饰示例", widget_name),
                "code": format!("{}(\n  width: 100,\n  height: 100,\n  decoration: BoxDecoration(\n    color: Colors.blue,\n    borderRadius: BorderRadius.circular(8),\n  ),\n  child: // 子组件\n)", widget_name)
            }));
        }
        
        // 添加文档链接提示
        examples.push(json!({
            "title": "获取完整示例",
            "code": format!("// 完整的 {} API文档和示例：\n// https://api.flutter.dev/flutter/widgets/{}-class.html", widget_name, widget_name),
            "note": "建议查看Flutter官方文档获取最新和完整的API信息"
        }));
        
        examples
    }

    /// 生成包示例代码
    fn generate_package_examples(&self, package_name: &str) -> Vec<Value> {
        vec![
            json!({
                "title": format!("Import {}", package_name),
                "code": format!("import 'package:{}/{}';", package_name, package_name)
            }),
            json!({
                "title": "Basic Usage",
                "code": format!("// Example usage of {}\n// Check package documentation for specific APIs", package_name)
            })
        ]
    }

    /// 获取Widget性能提示
    fn get_widget_performance_tips(&self, widget_name: &str) -> Vec<String> {
        match widget_name.to_lowercase().as_str() {
            "listview" => vec![
                "使用ListView.builder()进行大列表优化".to_string(),
                "考虑使用ListView.separated()添加分隔符".to_string(),
                "避免在ListView中嵌套滚动组件".to_string(),
            ],
            "container" => vec![
                "避免不必要的Container嵌套".to_string(),
                "使用SizedBox代替空Container".to_string(),
                "考虑使用DecoratedBox替代简单装饰".to_string(),
            ],
            "column" | "row" => vec![
                "使用Flex代替嵌套的Column/Row".to_string(),
                "考虑使用Wrap处理溢出".to_string(),
                "避免在滚动视图中使用无限制的Column/Row".to_string(),
            ],
            _ => vec![
                "使用const构造函数提高性能".to_string(),
                "避免在build方法中创建复杂对象".to_string(),
                "考虑使用RepaintBoundary优化重绘".to_string(),
            ]
        }
    }

    /// 获取Widget平台兼容性
    fn get_platform_compatibility(&self, widget_name: &str) -> Value {
        // 大多数Flutter Widget都支持所有平台
        json!({
            "android": true,
            "ios": true,
            "web": true,
            "desktop": true,
            "notes": format!("{} is supported on all Flutter platforms", widget_name)
        })
    }

    /// 获取包平台兼容性
    fn get_package_platform_compatibility(&self, package_name: &str) -> Value {
        // 根据包名推断平台支持
        match package_name {
            name if name.contains("android") => json!({
                "android": true,
                "ios": false,
                "web": false,
                "desktop": false,
                "notes": "Android-specific package"
            }),
            name if name.contains("ios") => json!({
                "android": false,
                "ios": true,
                "web": false,
                "desktop": false,
                "notes": "iOS-specific package"
            }),
            _ => json!({
                "android": true,
                "ios": true,
                "web": true,
                "desktop": true,
                "notes": "Check package documentation for specific platform support"
            })
        }
    }
}

#[async_trait]
impl MCPTool for FlutterDocsTool {
    fn name(&self) -> &'static str {
        "flutter_docs"
    }

    fn description(&self) -> &'static str {
        "Flutter/Dart文档工具 - 获取Flutter Widget文档、包信息、示例代码和性能指南"
    }

    fn parameters_schema(&self) -> &Schema {
        &FLUTTER_DOCS_SCHEMA
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let widget_name = params.get("widget_name").and_then(|v| v.as_str());
        let package = params.get("package").and_then(|v| v.as_str());
        let flutter_version = params.get("flutter_version").and_then(|v| v.as_str());
        let include_samples = params.get("include_samples")
            .and_then(|v| v.as_str())
            .unwrap_or("true") == "true";

        if widget_name.is_none() && package.is_none() {
            warn!("未提供widget_name或package参数，返回基础Flutter文档");
            return Ok(self.generate_basic_flutter_docs());
        }

        let mut result = self.generate_flutter_docs(widget_name, package, flutter_version).await?;

        // 如果不包含示例，移除示例部分
        if !include_samples {
            if let Some(obj) = result.as_object_mut() {
                obj.remove("examples");
            }
        }

        Ok(result)
    }
}

impl Default for FlutterDocsTool {
    fn default() -> Self {
        Self::new()
    }
} 