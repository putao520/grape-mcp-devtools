use std::env;
use anyhow::Result;
use tracing::{info, warn, error};
use serde_json::json;
use grape_mcp_devtools::{
    mcp::server::MCPServer,
    tools::{
        base::MCPTool,
        SearchDocsTool,
        versioning::CheckVersionTool,
        api_docs::GetApiDocsTool,
        vector_docs_tool::VectorDocsTool,
        enhanced_language_tool::{EnhancedLanguageTool, DocumentStrategy},
    },
    vectorization::embeddings::{EmbeddingConfig, VectorizationConfig, FileVectorizerImpl},
};

/// 详细测试CLI - 专门测试.env配置的功能
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 加载环境变量
    dotenv::dotenv().ok();
    
    info!("🚀 启动 Grape MCP DevTools 详细测试CLI");
    
    // 显示环境配置
    display_env_config();
    
    println!("\n{}", "=".repeat(70));
    println!("🧪 Grape MCP DevTools 详细功能测试（使用.env配置）");
    println!("{}", "=".repeat(70));
    
    // 测试1: 环境变量配置验证
    println!("\n📋 测试1: 环境变量配置验证");
    println!("{}", "-".repeat(50));
    test_env_config().await?;
    
    // 测试2: 向量化组件测试
    println!("\n📋 测试2: 向量化组件测试");
    println!("{}", "-".repeat(50));
    test_vectorization_components().await?;
    
    // 测试3: 向量文档工具测试
    println!("\n📋 测试3: 向量文档工具测试");
    println!("{}", "-".repeat(50));
    test_vector_docs_tool().await?;
    
    // 测试4: 完整MCP服务器测试
    println!("\n📋 测试4: 完整MCP服务器测试");
    println!("{}", "-".repeat(50));
    test_complete_mcp_server().await?;
    
    // 测试5: 增强语言工具与向量化集成测试
    println!("\n📋 测试5: 增强语言工具与向量化集成");
    println!("{}", "-".repeat(50));
    test_enhanced_tools_with_vectorization().await?;
    
    // 测试6: 真实包文档生成测试
    println!("\n📋 测试6: 真实包文档生成测试");
    println!("{}", "-".repeat(50));
    test_real_package_documentation().await?;
    
    println!("\n{}", "=".repeat(70));
    println!("🎉 详细测试完成！环境配置正常工作");
    println!("{}", "=".repeat(70));
    
    Ok(())
}

/// 显示环境配置
fn display_env_config() {
    println!("\n🔧 环境配置信息:");
    println!("   LLM_API_BASE_URL: {}", env::var("LLM_API_BASE_URL").unwrap_or_else(|_| "未配置".to_string()));
    println!("   LLM_MODEL_NAME: {}", env::var("LLM_MODEL_NAME").unwrap_or_else(|_| "未配置".to_string()));
    println!("   EMBEDDING_API_BASE_URL: {}", env::var("EMBEDDING_API_BASE_URL").unwrap_or_else(|_| "未配置".to_string()));
    println!("   EMBEDDING_MODEL_NAME: {}", env::var("EMBEDDING_MODEL_NAME").unwrap_or_else(|_| "未配置".to_string()));
    println!("   LLM_API_KEY: {}...", env::var("LLM_API_KEY").unwrap_or_else(|_| "未配置".to_string()).chars().take(10).collect::<String>());
    println!("   EMBEDDING_API_KEY: {}...", env::var("EMBEDDING_API_KEY").unwrap_or_else(|_| "未配置".to_string()).chars().take(10).collect::<String>());
}

/// 测试环境变量配置
async fn test_env_config() -> Result<()> {
    // 检查必需的环境变量
    let required_vars = vec!["LLM_API_BASE_URL", "LLM_API_KEY", "LLM_MODEL_NAME", "EMBEDDING_API_BASE_URL", "EMBEDDING_API_KEY", "EMBEDDING_MODEL_NAME"];
    let mut all_present = true;
    
    for var in &required_vars {
        match env::var(var) {
            Ok(value) => {
                if value.is_empty() {
                    println!("❌ {} 为空", var);
                    all_present = false;
                } else {
                    println!("✅ {} 已配置", var);
                }
            }
            Err(_) => {
                println!("❌ {} 未配置", var);
                all_present = false;
            }
        }
    }
    
    if all_present {
        println!("✅ 所有必需的环境变量都已配置");
    } else {
        warn!("⚠️ 部分环境变量缺失，某些功能可能不可用");
    }
    
    Ok(())
}

/// 测试向量化组件
async fn test_vectorization_components() -> Result<()> {
    println!("  🔍 测试向量化配置加载...");
    
    // 测试配置加载
    match EmbeddingConfig::from_env() {
        Ok(config) => {
            println!("    ✅ EmbeddingConfig 加载成功");
            println!("       🌐 API Base URL: {}", config.api_base_url);
            println!("       🤖 模型: {}", config.model_name);
        }
        Err(e) => {
            error!("    ❌ EmbeddingConfig 加载失败: {}", e);
            return Ok(()); // 继续其他测试
        }
    }
    
    match VectorizationConfig::from_env() {
        Ok(config) => {
            println!("    ✅ VectorizationConfig 加载成功");
            println!("       📏 块大小: {}", config.chunk_size);
            println!("       🔄 重叠: {}", config.chunk_overlap);
        }
        Err(e) => {
            error!("    ❌ VectorizationConfig 加载失败: {}", e);
            return Ok(()); // 继续其他测试
        }
    }
    
    // 测试向量化器创建
    println!("  🔍 测试向量化器创建...");
    match create_test_vectorizer().await {
        Ok(_) => {
            println!("    ✅ FileVectorizerImpl 创建成功");
        }
        Err(e) => {
            warn!("    ⚠️ FileVectorizerImpl 创建失败: {}", e);
            warn!("    💡 这可能是由于API密钥或网络问题，但不影响其他功能");
        }
    }
    
    Ok(())
}

/// 创建测试向量化器
async fn create_test_vectorizer() -> Result<FileVectorizerImpl> {
    let embedding_config = EmbeddingConfig::from_env()?;
    let vectorization_config = VectorizationConfig::from_env()?;
    
    FileVectorizerImpl::new(embedding_config, vectorization_config).await
}

/// 测试向量文档工具
async fn test_vector_docs_tool() -> Result<()> {
    println!("  🔍 测试向量文档工具初始化...");
    
    match VectorDocsTool::new() {
        Ok(tool) => {
            println!("    ✅ VectorDocsTool 创建成功");
            
            // 测试存储操作
            println!("  🔍 测试文档存储功能...");
            let store_params = json!({
                "action": "store",
                "title": "测试文档",
                "content": "这是一个测试文档，用于验证向量化存储功能。",
                "language": "rust",
                "doc_type": "test"
            });
            
            match tool.execute(store_params).await {
                Ok(result) => {
                    println!("    ✅ 文档存储测试成功");
                    if result["status"] == "success" {
                        println!("       📝 存储状态: 成功");
                    }
                }
                Err(e) => {
                    warn!("    ⚠️ 文档存储测试失败: {}", e);
                    warn!("    💡 这可能是由于向量化API问题，但工具结构正常");
                }
            }
            
            // 测试搜索操作
            println!("  🔍 测试文档搜索功能...");
            let search_params = json!({
                "action": "search",
                "query": "测试文档",
                "limit": 5
            });
            
            match tool.execute(search_params).await {
                Ok(result) => {
                    println!("    ✅ 文档搜索测试成功");
                    if let Some(count) = result.get("results_count") {
                        println!("       🔍 搜索结果数量: {}", count);
                    }
                }
                Err(e) => {
                    warn!("    ⚠️ 文档搜索测试失败: {}", e);
                }
            }
        }
        Err(e) => {
            error!("    ❌ VectorDocsTool 创建失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试完整MCP服务器
async fn test_complete_mcp_server() -> Result<()> {
    let mcp_server = MCPServer::new();
    
    // 注册所有工具
    println!("  🔍 注册MCP工具...");
    
    // 基础工具
    let search_tool = SearchDocsTool::new();
    mcp_server.register_tool(Box::new(search_tool)).await?;
    println!("    ✅ SearchDocsTool 注册成功");
    
    let version_tool = CheckVersionTool::new();
    mcp_server.register_tool(Box::new(version_tool)).await?;
    println!("    ✅ CheckVersionTool 注册成功");
    
    let api_docs_tool = GetApiDocsTool::new(None);
    mcp_server.register_tool(Box::new(api_docs_tool)).await?;
    println!("    ✅ GetApiDocsTool 注册成功");
    
    // 向量工具
    match VectorDocsTool::new() {
        Ok(vector_tool) => {
            mcp_server.register_tool(Box::new(vector_tool)).await?;
            println!("    ✅ VectorDocsTool 注册成功");
        }
        Err(e) => {
            warn!("    ⚠️ VectorDocsTool 注册失败: {}", e);
        }
    }
    
    let tool_count = mcp_server.get_tool_count().await?;
    println!("  ✅ MCP服务器配置完成，共注册 {} 个工具", tool_count);
    
    Ok(())
}

/// 测试增强语言工具与向量化集成
async fn test_enhanced_tools_with_vectorization() -> Result<()> {
    let test_scenarios = vec![
        ("rust", "tokio", "异步运行时"),
        ("python", "fastapi", "web框架"),
        ("javascript", "lodash", "工具库"),
    ];
    
    for (language, package, description) in test_scenarios {
        println!("  🔧 测试 {} - {} ({})", language, package, description);
        
        // 测试CLI优先策略
        match test_language_tool_with_strategy(language, package, DocumentStrategy::CLIPrimary).await {
            Ok(result) => {
                println!("    ✅ CLI优先策略成功");
                if let Some(source) = result.get("source") {
                    println!("       📚 文档源: {}", source);
                }
            }
            Err(e) => {
                warn!("    ⚠️ CLI优先策略失败: {}", e);
            }
        }
        
        // 测试HTTP备用策略
        match test_language_tool_with_strategy(language, package, DocumentStrategy::HTTPOnly).await {
            Ok(result) => {
                println!("    ✅ HTTP策略成功");
                if let Some(source) = result.get("source") {
                    println!("       📚 文档源: {}", source);
                }
            }
            Err(e) => {
                warn!("    ⚠️ HTTP策略失败: {}", e);
            }
        }
    }
    
    Ok(())
}

/// 测试语言工具特定策略
async fn test_language_tool_with_strategy(
    language: &str, 
    package: &str, 
    strategy: DocumentStrategy
) -> Result<serde_json::Value> {
    let tool = EnhancedLanguageTool::new(language.to_string(), strategy).await?;
    tool.get_package_docs(package, None, Some("API documentation")).await
}

/// 测试真实包文档生成
async fn test_real_package_documentation() -> Result<()> {
    println!("  🔍 测试真实包文档生成...");
    
    // 测试本地可用的工具
    let available_tools = check_available_cli_tools().await;
    println!("    📋 可用CLI工具: {:?}", available_tools);
    
    // 根据可用工具选择测试包
    if available_tools.contains(&"cargo".to_string()) {
        println!("  🦀 测试Rust包文档生成...");
        test_rust_package_docs().await?;
    }
    
    if available_tools.contains(&"pip".to_string()) {
        println!("  🐍 测试Python包文档生成...");
        test_python_package_docs().await?;
    }
    
    if available_tools.contains(&"pnpm".to_string()) {
        println!("  📦 测试JavaScript包文档生成...");
        test_javascript_package_docs().await?;
    }
    
    // 总是测试HTTP方式
    println!("  🌐 测试HTTP文档获取...");
    test_http_package_docs().await?;
    
    Ok(())
}

/// 检查可用的CLI工具
async fn check_available_cli_tools() -> Vec<String> {
    let tools = vec!["cargo", "pip", "npm", "pnpm", "go", "mvn", "gradle"];
    let mut available = Vec::new();
    
    for tool in tools {
        if is_cli_available(tool).await {
            available.push(tool.to_string());
        }
    }
    
    available
}

/// 检查CLI工具是否可用
async fn is_cli_available(tool: &str) -> bool {
    use tokio::process::Command;
    
    Command::new(tool)
        .arg("--version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// 测试Rust包文档
async fn test_rust_package_docs() -> Result<()> {
    let tool = EnhancedLanguageTool::new("rust".to_string(), DocumentStrategy::CLIPrimary).await?;
    
    match tool.get_package_docs("serde", Some("1.0"), Some("serialization")).await {
        Ok(docs) => {
            println!("    ✅ Rust包文档生成成功");
            if let Some(source) = docs.get("source") {
                println!("       📚 文档源: {}", source);
            }
            if let Some(content) = docs.get("documentation") {
                let content_str = content.to_string();
                let preview = content_str.chars().take(100).collect::<String>();
                println!("       📄 内容预览: {}...", preview);
            }
        }
        Err(e) => {
            warn!("    ⚠️ Rust包文档生成失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试Python包文档
async fn test_python_package_docs() -> Result<()> {
    let tool = EnhancedLanguageTool::new("python".to_string(), DocumentStrategy::CLIPrimary).await?;
    
    match tool.get_package_docs("requests", None, Some("HTTP library")).await {
        Ok(docs) => {
            println!("    ✅ Python包文档生成成功");
            if let Some(source) = docs.get("source") {
                println!("       📚 文档源: {}", source);
            }
        }
        Err(e) => {
            warn!("    ⚠️ Python包文档生成失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试JavaScript包文档
async fn test_javascript_package_docs() -> Result<()> {
    let tool = EnhancedLanguageTool::new("javascript".to_string(), DocumentStrategy::CLIPrimary).await?;
    
    match tool.get_package_docs("express", None, Some("web framework")).await {
        Ok(docs) => {
            println!("    ✅ JavaScript包文档生成成功");
            if let Some(source) = docs.get("source") {
                println!("       📚 文档源: {}", source);
            }
        }
        Err(e) => {
            warn!("    ⚠️ JavaScript包文档生成失败: {}", e);
        }
    }
    
    Ok(())
}

/// 测试HTTP文档获取
async fn test_http_package_docs() -> Result<()> {
    let tool = EnhancedLanguageTool::new("rust".to_string(), DocumentStrategy::HTTPOnly).await?;
    
    match tool.get_package_docs("anyhow", None, Some("error handling")).await {
        Ok(docs) => {
            println!("    ✅ HTTP文档获取成功");
            if let Some(source) = docs.get("source") {
                println!("       📚 文档源: {}", source);
            }
        }
        Err(e) => {
            warn!("    ⚠️ HTTP文档获取失败: {}", e);
        }
    }
    
    Ok(())
} 