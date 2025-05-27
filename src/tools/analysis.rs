use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::collections::HashMap;

use super::base::{MCPTool, Schema, SchemaObject, SchemaString};
// use super::docs::doc_traits::{DocumentStore, DocumentType};

// 暂时注释掉文档处理工具，专注于代码分析工具
/*
/// 文档处理工具 - 用于处理和索引第三方库文档
pub struct DocsProcessorTool {
    doc_store: Box<dyn DocumentStore>,
}

impl DocsProcessorTool {
    pub fn new(doc_store: Box<dyn DocumentStore>) -> Self {
        Self { doc_store }
    }
}

#[async_trait]
impl MCPTool for DocsProcessorTool {
    fn name(&self) -> &'static str {
        "process_library_docs"
    }
    
    fn description(&self) -> &'static str {
        "处理并索引第三方库文档"
    }
      fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["package_name".to_string(), "language".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("package_name".to_string(), Schema::String(SchemaString {
                        description: Some("要处理的包名称".to_string()),
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
                            "csharp".to_string(),
                            "php".to_string()
                        ]),
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("可选的包版本，默认使用最新版本".to_string()),
                        enum_values: None,
                    }));
                    map
                },
                description: Some("处理第三方库文档的参数".to_string()),
            })
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        let package_name = params.get("package_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少包名称"))?;
            
        let language = params.get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少语言类型"))?;
            
        let version = params.get("version").and_then(|v| v.as_str());
          // 根据语言类型调用相应的文档处理方法
        let result = match language {
            "rust" => process_rust_docs(&*self.doc_store, package_name, version).await?,
            "python" => process_python_docs(&*self.doc_store, package_name, version).await?,
            "javascript" | "typescript" => process_js_docs(&*self.doc_store, package_name, version).await?,
            "java" => process_java_docs(&*self.doc_store, package_name, version).await?,
            "go" => process_go_docs(&*self.doc_store, package_name, version).await?,
            "csharp" => process_csharp_docs(&*self.doc_store, package_name, version).await?,
            "php" => process_php_docs(&*self.doc_store, package_name, version).await?,
            _ => return Err(anyhow::anyhow!("不支持的语言类型: {}", language))
        };
        
        Ok(result)
    }
}

/// 处理 Rust 第三方库文档
async fn process_rust_docs(doc_store: &dyn DocumentStore, package_name: &str, version: Option<&str>) -> Result<Value> {
    // 1. 使用 cargo doc 生成文档
    let output = tokio::process::Command::new("cargo")
        .args(["doc", "--package", package_name, "--no-deps"])
        .output()
        .await?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "生成文档失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    // 2. 解析生成的文档（位于 target/doc 目录）
    let doc_dir = Path::new("target/doc")
        .join(package_name.replace('-', "_"));
        
    // 3. 解析文档内容（HTML格式）
    let fragments = parse_rust_html_docs(&doc_dir)?;
    
    // 4. 存入文档存储
    let doc_count = fragments.len();
    for fragment in fragments {
        doc_store.store(&fragment).await?;
    }
    
    Ok(json!({
        "status": "success",
        "package": package_name,
        "version": version.unwrap_or("latest"),
        "doc_fragments": doc_count,
        "message": format!("成功处理 {} 的文档，共索引 {} 个文档片段", package_name, doc_count)
    }))
}

/// 处理 Python 第三方库文档
async fn process_python_docs(doc_store: &dyn DocumentStore, package_name: &str, version: Option<&str>) -> Result<Value> {
    // 1. 使用 pydoc 生成文档
    let module_path = if let Some(ver) = version {
        format!("{}=={}", package_name, ver)
    } else {
        package_name.to_string()
    };
    
    let output = tokio::process::Command::new("python")
        .args(["-m", "pydoc", "-w", &module_path])
        .output()
        .await?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "生成文档失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    // 2. 解析生成的HTML文档
    let fragments = parse_python_html_docs(&format!("{}.html", package_name))?;
    
    // 3. 存入文档存储
    let doc_count = fragments.len();
    for fragment in fragments {
        doc_store.store(&fragment).await?;
    }
    
    Ok(json!({
        "status": "success",
        "package": package_name,
        "version": version.unwrap_or("latest"),
        "doc_fragments": doc_count,
        "message": format!("成功处理 {} 的文档，共索引 {} 个文档片段", package_name, doc_count)
    }))
}

/// 处理 JavaScript/TypeScript 第三方库文档
async fn process_js_docs(doc_store: &dyn DocumentStore, package_name: &str, version: Option<&str>) -> Result<Value> {
    // 1. 使用 JSDoc/TypeDoc 生成文档
    let package_spec = if let Some(ver) = version {
        format!("{}@{}", package_name, ver)
    } else {
        package_name.to_string()
    };
    
    let output = tokio::process::Command::new("npx")
        .args(["typedoc", "--json", "docs.json", &package_spec])
        .output()
        .await?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "生成文档失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    // 2. 解析生成的JSON文档
    let fragments = parse_js_json_docs("docs.json")?;
    
    // 3. 存入文档存储
    let doc_count = fragments.len();
    for fragment in fragments {
        doc_store.store(&fragment).await?;
    }
    
    Ok(json!({
        "status": "success",
        "package": package_name,
        "version": version.unwrap_or("latest"),
        "doc_fragments": doc_count,
        "message": format!("成功处理 {} 的文档，共索引 {} 个文档片段", package_name, doc_count)
    }))
}

/// 处理 Java 第三方库文档
async fn process_java_docs(doc_store: &dyn DocumentStore, package_name: &str, version: Option<&str>) -> Result<Value> {
    // 1. 使用 Maven 和 Javadoc 生成文档
    let pom_content = format!(r#"
        <project>
            <modelVersion>4.0.0</modelVersion>
            <groupId>temp</groupId>
            <artifactId>temp</artifactId>
            <version>1.0-SNAPSHOT</version>
            <dependencies>
                <dependency>
                    <groupId>{}</groupId>
                    <artifactId>{}</artifactId>
                    <version>{}</version>
                </dependency>
            </dependencies>
        </project>
    "#, 
        package_name.split(":").nth(0).unwrap_or(package_name),
        package_name.split(":").nth(1).unwrap_or(package_name),
        version.unwrap_or("LATEST")
    );

    // 创建临时 pom.xml
    let temp_dir = tempfile::tempdir()?;
    let pom_path = temp_dir.path().join("pom.xml");
    std::fs::write(&pom_path, pom_content)?;

    // 生成 Javadoc
    let output = tokio::process::Command::new("mvn")
        .args(["javadoc:javadoc"])
        .current_dir(&temp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "生成Java文档失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // 2. 解析生成的Javadoc
    let doc_dir = temp_dir.path().join("target/site/apidocs");
    let fragments = parse_java_html_docs(&doc_dir)?;

    // 3. 存入文档存储
    let doc_count = fragments.len();
    for fragment in fragments {
        doc_store.store(&fragment).await?;
    }

    Ok(json!({
        "status": "success",
        "package": package_name,
        "version": version.unwrap_or("latest"),
        "doc_fragments": doc_count,
        "message": format!("成功处理 {} 的文档，共索引 {} 个文档片段", package_name, doc_count)
    }))
}

/// 处理 Go 第三方库文档
async fn process_go_docs(doc_store: &dyn DocumentStore, package_name: &str, version: Option<&str>) -> Result<Value> {
    // 1. 下载并生成Go文档
    let go_get_args = if let Some(ver) = version {
        vec!["get", &format!("{}@{}", package_name, ver)]
    } else {
        vec!["get", package_name]
    };

    let output = tokio::process::Command::new("go")
        .args(&go_get_args)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "获取Go包失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // 使用 godoc 生成文档
    let output = tokio::process::Command::new("godoc")
        .args(["-html", package_name])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "生成Go文档失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // 2. 解析生成的HTML文档
    let doc_content = String::from_utf8_lossy(&output.stdout);
    let fragments = parse_go_html_docs(&doc_content)?;

    // 3. 存入文档存储
    let doc_count = fragments.len();
    for fragment in fragments {
        doc_store.store(&fragment).await?;
    }

    Ok(json!({
        "status": "success",
        "package": package_name,
        "version": version.unwrap_or("latest"),
        "doc_fragments": doc_count,
        "message": format!("成功处理 {} 的文档，共索引 {} 个文档片段", package_name, doc_count)
    }))
}

/// 处理 C# 第三方库文档
async fn process_csharp_docs(doc_store: &dyn DocumentStore, package_name: &str, version: Option<&str>) -> Result<Value> {
    // 1. 使用 NuGet 和 DocFX 生成文档
    let temp_dir = tempfile::tempdir()?;
    
    // 创建临时项目
    let output = tokio::process::Command::new("dotnet")
        .args(["new", "classlib"])
        .current_dir(&temp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("创建C#项目失败"));
    }

    // 添加包引用
    let package_ref = if let Some(ver) = version {
        format!("{}:{}", package_name, ver)
    } else {
        package_name.to_string()
    };

    let output = tokio::process::Command::new("dotnet")
        .args(["add", "package", &package_ref])
        .current_dir(&temp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("添加NuGet包引用失败"));
    }

    // 使用 DocFX 生成文档
    let output = tokio::process::Command::new("docfx")
        .args(["init", "-q"])
        .current_dir(&temp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("初始化DocFX失败"));
    }

    let output = tokio::process::Command::new("docfx")
        .args(["build"])
        .current_dir(&temp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("生成DocFX文档失败"));
    }

    // 2. 解析生成的文档
    let doc_dir = temp_dir.path().join("_site");
    let fragments = parse_csharp_html_docs(&doc_dir)?;

    // 3. 存入文档存储
    let doc_count = fragments.len();
    for fragment in fragments {
        doc_store.store(&fragment).await?;
    }

    Ok(json!({
        "status": "success",
        "package": package_name,
        "version": version.unwrap_or("latest"),
        "doc_fragments": doc_count,
        "message": format!("成功处理 {} 的文档，共索引 {} 个文档片段", package_name, doc_count)
    }))
}

/// 处理 PHP 第三方库文档
async fn process_php_docs(doc_store: &dyn DocumentStore, package_name: &str, version: Option<&str>) -> Result<Value> {
    // 1. 使用 Composer 和 phpDocumentor 生成文档
    let temp_dir = tempfile::tempdir()?;
    
    // 创建composer.json
    let composer_json = json!({
        "require": {
            package_name: version.unwrap_or("*")
        }
    });

    std::fs::write(
        temp_dir.path().join("composer.json"),
        serde_json::to_string_pretty(&composer_json)?
    )?;

    // 安装依赖
    let output = tokio::process::Command::new("composer")
        .args(["install"])
        .current_dir(&temp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("安装PHP依赖失败"));
    }

    // 使用phpDocumentor生成文档
    let output = tokio::process::Command::new("phpDocumentor")
        .args([
            "--directory=vendor/",
            &format!("--target=docs"),
            "--template=clean"
        ])
        .current_dir(&temp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("生成PHP文档失败"));
    }

    // 2. 解析生成的文档
    let doc_dir = temp_dir.path().join("docs");
    let fragments = parse_php_html_docs(&doc_dir)?;

    // 3. 存入文档存储
    let doc_count = fragments.len();
    for fragment in fragments {
        doc_store.store(&fragment).await?;
    }

    Ok(json!({
        "status": "success",
        "package": package_name,
        "version": version.unwrap_or("latest"),
        "doc_fragments": doc_count,
        "message": format!("成功处理 {} 的文档，共索引 {} 个文档片段", package_name, doc_count)
    }))
}

// 文档解析函数实现
fn parse_rust_html_docs(doc_dir: &Path) -> Result<Vec<DocumentFragment>> {
    use crate::tools::docs::rust_processor::RustDocumentProcessor;
    use crate::tools::docs::doc_traits::DocumentProcessor;
    
    let mut fragments = Vec::new();
    
    // 遍历文档目录，查找HTML文件
    if doc_dir.exists() {
        for entry in std::fs::read_dir(doc_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "html") {
                let content = std::fs::read_to_string(&path)?;
                let processor = RustDocumentProcessor::new(Default::default());
                
                // 由于这是异步函数，我们需要在运行时执行
                let parsed_fragments = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        processor.extract_structure(&content).await
                    })
                })?;
                
                // 转换为DocumentFragment格式
                for heading in parsed_fragments.headings {
                    fragments.push(DocumentFragment {
                        id: format!("rust-{}", heading.text.replace(' ', "-")),
                        title: heading.text,
                        kind: DocElementKind::Function,
                        full_name: None,
                        description: "Rust documentation".to_string(),
                        source_type: DocSourceType::ApiDoc,
                        code_context: None,
                        examples: Vec::new(),
                        api_info: None,
                        references: Vec::new(),
                        metadata: DocMetadata {
                            package_name: "rust".to_string(),
                            version: None,
                            language: "rust".to_string(),
                            source_url: None,
                            deprecated: false,
                            since_version: None,
                            visibility: Visibility::Public,
                        },
                        changelog_info: None,
                    });
                }
            }
        }
    }
    
    Ok(fragments)
}

fn parse_python_html_docs(doc_file: &str) -> Result<Vec<DocumentFragment>> {
    use crate::tools::docs::python_processor::PythonDocumentProcessor;
    use crate::tools::docs::doc_traits::DocumentProcessor;
    
    let content = std::fs::read_to_string(doc_file)?;
    let processor = PythonDocumentProcessor::new(Default::default());
    
    // 由于这是异步函数，我们需要在运行时执行
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            processor.extract_structure(&content).await
        })
    }).map(|structure| {
        structure.headings.into_iter().map(|heading| {
            DocumentFragment {
                id: format!("python-{}", heading.text.replace(' ', "-")),
                title: heading.text,
                kind: DocElementKind::Function,
                full_name: None,
                description: "Python documentation".to_string(),
                source_type: DocSourceType::ApiDoc,
                code_context: None,
                examples: Vec::new(),
                api_info: None,
                references: Vec::new(),
                metadata: DocMetadata {
                    package_name: "python".to_string(),
                    version: None,
                    language: "python".to_string(),
                    source_url: None,
                    deprecated: false,
                    since_version: None,
                    visibility: Visibility::Public,
                },
                changelog_info: None,
            }
        }).collect()
    })
}

fn parse_js_json_docs(json_file: &str) -> Result<Vec<DocumentFragment>> {
    let content = std::fs::read_to_string(json_file)?;
    let json_data: serde_json::Value = serde_json::from_str(&content)?;
    
    let mut fragments = Vec::new();
    
    // 解析JSON格式的文档
    if let Some(items) = json_data.as_array() {
        for item in items {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                fragments.push(DocumentFragment {
                    id: format!("js-{}", name.replace(' ', "-")),
                    title: name.to_string(),
                    kind: DocElementKind::Function,
                    full_name: None,
                    description: item.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("JavaScript documentation")
                        .to_string(),
                    source_type: DocSourceType::ApiDoc,
                    code_context: None,
                    examples: Vec::new(),
                    api_info: None,
                    references: Vec::new(),
                    metadata: DocMetadata {
                        package_name: "javascript".to_string(),
                        version: None,
                        language: "javascript".to_string(),
                        source_url: None,
                        deprecated: false,
                        since_version: None,
                        visibility: Visibility::Public,
                    },
                    changelog_info: None,
                });
            }
        }
    }
    
    Ok(fragments)
}

fn parse_java_html_docs(doc_dir: &Path) -> Result<Vec<DocumentFragment>> {
    use crate::tools::docs::java_processor::JavaDocumentProcessor;
    use crate::tools::docs::doc_traits::DocumentProcessor;
    
    let mut fragments = Vec::new();
    
    // 遍历Javadoc目录
    if doc_dir.exists() {
        for entry in std::fs::read_dir(doc_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "html") {
                let content = std::fs::read_to_string(&path)?;
                let processor = JavaDocumentProcessor::new(Default::default());
                
                // 由于这是异步函数，我们需要在运行时执行
                let parsed_fragments = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        processor.extract_structure(&content).await
                    })
                })?;
                
                // 转换为DocumentFragment格式
                for heading in parsed_fragments.headings {
                    fragments.push(DocumentFragment {
                        id: format!("java-{}", heading.text.replace(' ', "-")),
                        title: heading.text,
                        kind: DocElementKind::Function,
                        full_name: None,
                        description: "Java documentation".to_string(),
                        source_type: DocSourceType::ApiDoc,
                        code_context: None,
                        examples: Vec::new(),
                        api_info: None,
                        references: Vec::new(),
                        metadata: DocMetadata {
                            package_name: "java".to_string(),
                            version: None,
                            language: "java".to_string(),
                            source_url: None,
                            deprecated: false,
                            since_version: None,
                            visibility: Visibility::Public,
                        },
                        changelog_info: None,
                    });
                }
            }
        }
    }
    
    Ok(fragments)
}

fn parse_csharp_html_docs(doc_dir: &Path) -> Result<Vec<DocumentFragment>> {
    let mut fragments = Vec::new();
    
    // 遍历DocFX生成的HTML文档
    if doc_dir.exists() {
        for entry in std::fs::read_dir(doc_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "html") {
                let content = std::fs::read_to_string(&path)?;
                
                // 简单的HTML解析，提取标题
                if let Some(title_start) = content.find("<title>") {
                    if let Some(title_end) = content[title_start..].find("</title>") {
                        let title = &content[title_start + 7..title_start + title_end];
                        
                        fragments.push(DocumentFragment {
                            id: format!("csharp-{}", title.replace(' ', "-")),
                            title: title.to_string(),
                            kind: DocElementKind::Function,
                            full_name: None,
                            description: "C# documentation".to_string(),
                            source_type: DocSourceType::ApiDoc,
                            code_context: None,
                            examples: Vec::new(),
                            api_info: None,
                            references: Vec::new(),
                            metadata: DocMetadata {
                                package_name: "csharp".to_string(),
                                version: None,
                                language: "csharp".to_string(),
                                source_url: None,
                                deprecated: false,
                                since_version: None,
                                visibility: Visibility::Public,
                            },
                            changelog_info: None,
                        });
                    }
                }
            }
        }
    }
    
    Ok(fragments)
}

fn parse_go_html_docs(content: &str) -> Result<Vec<DocumentFragment>> {
    // 使用 Go 文档处理器来解析 HTML 内容
    use crate::tools::docs::go_processor::GoDocProcessorImpl;
    use crate::tools::docs::doc_traits::GoDocProcessor;
    
    let processor = GoDocProcessorImpl::new();
    
    // 由于这是异步函数，我们需要在运行时执行
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            processor.process_godoc(content).await
        })
    })
}

fn parse_php_html_docs(doc_dir: &Path) -> Result<Vec<DocumentFragment>> {
    let mut fragments = Vec::new();
    
    // 遍历phpDocumentor生成的HTML文档
    if doc_dir.exists() {
        for entry in std::fs::read_dir(doc_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "html") {
                let content = std::fs::read_to_string(&path)?;
                
                // 简单的HTML解析，提取标题
                if let Some(title_start) = content.find("<title>") {
                    if let Some(title_end) = content[title_start..].find("</title>") {
                        let title = &content[title_start + 7..title_start + title_end];
                        
                        fragments.push(DocumentFragment {
                            id: format!("php-{}", title.replace(' ', "-")),
                            title: title.to_string(),
                            kind: DocElementKind::Function,
                            full_name: None,
                            description: "PHP documentation".to_string(),
                            source_type: DocSourceType::ApiDoc,
                            code_context: None,
                            examples: Vec::new(),
                            api_info: None,
                            references: Vec::new(),
                            metadata: DocMetadata {
                                package_name: "php".to_string(),
                                version: None,
                                language: "php".to_string(),
                                source_url: None,
                                deprecated: false,
                                since_version: None,
                                visibility: Visibility::Public,
                            },
                            changelog_info: None,
                        });
                    }
                }
            }
        }
    }
    
    Ok(fragments)
}
*/

/// 代码分析工具
pub struct AnalyzeCodeTool;

#[async_trait]
impl MCPTool for AnalyzeCodeTool {
    fn name(&self) -> &'static str {
        "analyze_code"
    }
    
    fn description(&self) -> &'static str {
        "在需要分析代码质量、发现潜在问题或获取代码改进建议时，检查代码中的问题、性能瓶颈和最佳实践建议。"
    }
    
    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["code".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("code".to_string(), Schema::String(SchemaString {
                        description: Some("要分析的代码内容".to_string()),
                        enum_values: None,
                    }));
                    map.insert("language".to_string(), Schema::String(SchemaString {
                        description: Some("代码所使用的编程语言".to_string()),
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
            .ok_or_else(|| anyhow::anyhow!("缺少代码内容"))?;
            
        let language = params.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        // 简单的代码分析逻辑
        let lines = code.lines().count();
        let chars = code.chars().count();
        let words = code.split_whitespace().count();
        
        Ok(json!({
            "status": "success",
            "language": language,
            "metrics": {
                "lines": lines,
                "characters": chars,
                "words": words
            },
            "suggestions": [
                "考虑添加更多注释",
                "检查代码复杂度",
                "确保遵循编码规范"
            ]
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
        "在需要获取代码重构建议、优化方案或改进代码结构时，分析代码并提供具体的重构建议、优化步骤和最佳实践指导。"
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
                        description: Some("代码所使用的编程语言".to_string()),
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
            .ok_or_else(|| anyhow::anyhow!("缺少代码内容"))?;
            
        let language = params.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        // 简单的重构建议逻辑
        let mut suggestions = Vec::new();
        
        if code.lines().count() > 50 {
            suggestions.push("函数过长，考虑拆分为更小的函数");
        }
        
        if code.contains("TODO") || code.contains("FIXME") {
            suggestions.push("存在待办事项，需要完成");
        }
        
        if code.matches("if").count() > 5 {
            suggestions.push("条件语句过多，考虑使用多态或策略模式");
        }
        
        Ok(json!({
            "status": "success",
            "language": language,
            "suggestions": suggestions,
            "refactoring_opportunities": [
                {
                    "type": "extract_method",
                    "description": "提取重复代码为方法",
                    "priority": "medium"
                },
                {
                    "type": "simplify_conditions",
                    "description": "简化复杂条件",
                    "priority": "low"
                }
            ]
        }))
    }
}
