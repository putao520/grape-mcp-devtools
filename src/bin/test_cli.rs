use std::env;
use anyhow::Result;
use tracing::{info, warn};
use serde_json::json;
use grape_mcp_devtools::{
    mcp::server::MCPServer,
    tools::{
        base::MCPTool,
        SearchDocsTool,
        versioning::CheckVersionTool,
        api_docs::GetApiDocsTool,
        enhanced_language_tool::{EnhancedLanguageTool, DocumentStrategy},
        FlutterDocsTool,
        VectorDocsTool,
        EnhancedDocumentProcessor,
    },
};
use std::sync::Arc;
use dotenv;

/// 测试CLI - 验证MCP工具功能
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 加载环境变量
    dotenv::dotenv().ok();
    
    info!("🚀 启动 Grape MCP DevTools 测试CLI");
    info!("🔧 LLM API配置: {}", env::var("LLM_API_BASE_URL").unwrap_or_else(|_| "未配置".to_string()));
    info!("🤖 LLM模型: {}", env::var("LLM_MODEL_NAME").unwrap_or_else(|_| "未配置".to_string()));
    info!("🔍 Embedding模型: {}", env::var("EMBEDDING_MODEL_NAME").unwrap_or_else(|_| "未配置".to_string()));
    
    println!("\n{}", "=".repeat(60));
    println!("🧪 Grape MCP DevTools 功能测试");
    println!("{}", "=".repeat(60));
    
    // 测试1: 基础MCP服务器创建
    println!("\n📋 测试1: MCP服务器创建");
    println!("{}", "-".repeat(40));
    test_mcp_server_creation().await?;
    
    // 测试2: 版本检查工具
    println!("\n📋 测试2: 版本检查工具");
    println!("{}", "-".repeat(40));
    test_version_check_tool().await?;
    
    // 测试3: API文档工具
    println!("\n📋 测试3: API文档工具");
    println!("{}", "-".repeat(40));
    test_api_docs_tool().await?;
    
    // 测试4: 文档搜索工具
    println!("\n📋 测试4: 文档搜索工具");
    println!("{}", "-".repeat(40));
    test_search_docs_tool().await?;
    
    // 测试5: 增强语言工具（CLI优先）
    println!("\n📋 测试5: 增强语言工具（CLI优先）");
    println!("{}", "-".repeat(40));
    test_enhanced_language_tools().await?;
    
    // 测试6: Flutter专用工具
    println!("\n📋 测试6: Flutter专用工具");
    println!("{}", "-".repeat(40));
    test_flutter_docs_tool().await?;
    
    // 测试7: CLI工具可用性检测
    println!("\n📋 测试7: CLI工具可用性检测");
    println!("{}", "-".repeat(40));
    test_cli_tools_availability().await?;
    
    // 测试8: HTTP后备测试
    println!("\n📋 测试8: HTTP后备功能");
    println!("{}", "-".repeat(40));
    test_http_fallback().await?;
    
    println!("\n{}", "=".repeat(60));
    println!("🎉 所有测试完成！");
    println!("{}", "=".repeat(60));
    
    Ok(())
}

/// 测试MCP服务器创建
async fn test_mcp_server_creation() -> Result<()> {
    let mcp_server = MCPServer::new();
    
    // 注册工具
    let search_tool = SearchDocsTool::new();
    mcp_server.register_tool(Box::new(search_tool)).await?;
    
    let version_tool = CheckVersionTool::new();
    mcp_server.register_tool(Box::new(version_tool)).await?;
    
    let api_docs_tool = GetApiDocsTool::new();
    mcp_server.register_tool(Box::new(api_docs_tool)).await?;
    
    let tool_count = mcp_server.get_tool_count().await?;
    println!("✅ MCP服务器创建成功，已注册 {} 个工具", tool_count);
    
    Ok(())
}

/// 测试版本检查工具
async fn test_version_check_tool() -> Result<()> {
    let tool = CheckVersionTool::new();
    
    // 测试Rust包版本检查
    let test_params = json!({
        "package": "serde",
        "language": "rust"
    });
    
    match tool.execute(test_params).await {
        Ok(result) => {
            println!("✅ 版本检查成功");
            if let Some(version) = result.get("latest_version") {
                println!("   📦 serde 最新版本: {}", version);
            }
            if let Some(url) = result.get("docs_url") {
                println!("   📚 文档链接: {}", url);
            }
        }
        Err(e) => {
            warn!("⚠️ 版本检查失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试API文档工具
async fn test_api_docs_tool() -> Result<()> {
    let tool = GetApiDocsTool::new();
    
    // 测试获取Rust标准库文档
    let test_params = json!({
        "language": "rust",
        "package": "std",
        "query": "Vec"
    });
    
    match tool.execute(test_params).await {
        Ok(result) => {
            println!("✅ API文档获取成功");
            if let Some(docs) = result.get("documentation") {
                let docs_str = docs.to_string();
                let preview = if docs_str.len() > 100 {
                    format!("{}...", &docs_str[..100])
                } else {
                    docs_str
                };
                println!("   📄 文档预览: {}", preview);
            }
        }
        Err(e) => {
            warn!("⚠️ API文档获取失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试文档搜索工具
async fn test_search_docs_tool() -> Result<()> {
    let tool = SearchDocsTool::new();
    
    // 测试搜索Rust文档
    let test_params = json!({
        "query": "vector operations",
        "language": "rust",
        "limit": 3
    });
    
    match tool.execute(test_params).await {
        Ok(result) => {
            println!("✅ 文档搜索成功");
            if let Some(results) = result.get("results") {
                if let Some(results_array) = results.as_array() {
                    println!("   🔍 找到 {} 个结果", results_array.len());
                }
            }
        }
        Err(e) => {
            warn!("⚠️ 文档搜索失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试增强语言工具
async fn test_enhanced_language_tools() -> Result<()> {
    // 首先创建必需的依赖
    let vector_tool = Arc::new(VectorDocsTool::new()?);
    let doc_processor = Arc::new(EnhancedDocumentProcessor::new(vector_tool).await?);
    
    let languages = vec![
        ("rust", "serde"),
        ("go", "github.com/gin-gonic/gin"),
        ("python", "requests"),
        ("javascript", "express"),
        ("java", "com.fasterxml.jackson.core:jackson-core"),
    ];
    
    for (language, package) in languages {
        println!("  🔧 测试 {} 语言工具 - 包: {}", language, package);
        
        match test_single_language_tool(language, package, Arc::clone(&doc_processor)).await {
            Ok(_) => {
                println!("    ✅ {} 工具测试成功", language);
            }
            Err(e) => {
                warn!("    ⚠️ {} 工具测试失败: {}", language, e);
            }
        }
    }
    
    Ok(())
}

/// 测试单个语言工具
async fn test_single_language_tool(language: &str, package: &str, doc_processor: Arc<EnhancedDocumentProcessor>) -> Result<()> {
    // 使用新的构造函数签名
    let tool = EnhancedLanguageTool::new(language, doc_processor).await?;
    
    let docs = tool.get_package_docs(package, None, Some("documentation")).await?;
    
    if let Some(source) = docs.get("source") {
        println!("      📚 文档源: {}", source);
    }
    
    if let Some(installation) = docs.get("installation") {
        println!("      📦 安装命令: {}", installation);
    }
    
    Ok(())
}

/// 测试Flutter专用工具
async fn test_flutter_docs_tool() -> Result<()> {
    let tool = FlutterDocsTool::new();
    
    // 测试1: Widget文档
    println!("  🔧 测试Widget文档获取");
    let widget_params = json!({
        "widget_name": "Container",
        "include_samples": true
    });
    
    match tool.execute(widget_params).await {
        Ok(result) => {
            println!("    ✅ Container Widget文档获取成功");
            if let Some(examples) = result.get("examples") {
                if let Some(examples_array) = examples.as_array() {
                    println!("      📝 包含 {} 个示例", examples_array.len());
                }
            }
            if let Some(tips) = result.get("performance_tips") {
                if let Some(tips_array) = tips.as_array() {
                    println!("      💡 包含 {} 个性能提示", tips_array.len());
                }
            }
        }
        Err(e) => {
            warn!("    ⚠️ Widget文档获取失败: {}", e);
        }
    }
    
    // 测试2: 包文档
    println!("  🔧 测试pub.dev包文档获取");
    let package_params = json!({
        "package": "http",
        "include_samples": true
    });
    
    match tool.execute(package_params).await {
        Ok(result) => {
            println!("    ✅ http包文档获取成功");
            if let Some(source) = result.get("source") {
                println!("      📚 文档源: {}", source);
            }
        }
        Err(e) => {
            warn!("    ⚠️ 包文档获取失败: {}", e);
        }
    }
    
    // 测试3: 基础Flutter文档
    println!("  🔧 测试基础Flutter文档");
    let basic_params = json!({});
    
    match tool.execute(basic_params).await {
        Ok(result) => {
            println!("    ✅ 基础Flutter文档获取成功");
            if let Some(platform_support) = result.get("platform_support") {
                println!("      🌐 平台支持信息已获取");
            }
        }
        Err(e) => {
            warn!("    ⚠️ 基础文档获取失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试CLI工具可用性
async fn test_cli_tools_availability() -> Result<()> {
    let cli_tools = vec![
        ("go", "Go语言工具"),
        ("cargo", "Rust工具"),
        ("npm", "Node.js包管理器"),
        ("pip", "Python包管理器"),
        ("mvn", "Maven构建工具"),
        ("gradle", "Gradle构建工具"),
        ("yarn", "Yarn包管理器"),
        ("pnpm", "pnpm包管理器"),
        ("poetry", "Poetry Python包管理器"),
        ("conda", "Conda包管理器"),
        ("flutter", "Flutter开发工具"),
        ("dart", "Dart语言工具"),
        ("pub", "Dart包管理器"),
    ];
    
    println!("  🔍 检测本地CLI工具...");
    
    for (tool_name, description) in cli_tools {
        match check_cli_tool(tool_name).await {
            Ok(version) => {
                println!("    ✅ {} ({}): {}", description, tool_name, version.trim());
            }
            Err(_) => {
                println!("    ❌ {} ({}) 不可用", description, tool_name);
            }
        }
    }
    
    Ok(())
}

/// 测试HTTP后备功能
async fn test_http_fallback() -> Result<()> {
    // 创建必需的依赖
    let vector_tool = Arc::new(VectorDocsTool::new()?);
    let doc_processor = Arc::new(EnhancedDocumentProcessor::new(vector_tool).await?);
    
    // 使用新的构造函数签名
    let tool = EnhancedLanguageTool::new("rust", doc_processor).await?;
    
    match tool.get_package_docs("serde", None, Some("serialization")).await {
        Ok(docs) => {
            println!("✅ HTTP后备功能正常");
            if let Some(source) = docs.get("source") {
                println!("   📚 文档源: {}", source);
            }
        }
        Err(e) => {
            warn!("⚠️ HTTP后备功能失败: {}", e);
        }
    }
    
    Ok(())
}

/// 检查CLI工具是否可用
async fn check_cli_tool(tool_name: &str) -> Result<String> {
    use tokio::process::Command;
    
    let output = Command::new(tool_name)
        .arg("--version")
        .output()
        .await?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        let first_line = version.lines().next().unwrap_or("unknown version");
        Ok(first_line.to_string())
    } else {
        Err(anyhow::anyhow!("工具不可用: {}", tool_name))
    }
} 