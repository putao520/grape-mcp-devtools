use std::sync::Arc;
use serde_json::json;
use anyhow::Result;
use tracing::info;

use grape_mcp_devtools::mcp::server::MCPServer;
use grape_mcp_devtools::intelligent_mcp_server::IntelligentMCPServer;
use grape_mcp_devtools::tools::SearchDocsTool;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("grape_mcp_devtools=debug,info")
        .init();

    info!("🚀 启动多Agent架构演示");

    // 1. 创建基础MCP服务器
    let mcp_server = Arc::new(MCPServer::new());
    
    // 2. 注册一些基础工具
    let search_tool = SearchDocsTool::new();
    mcp_server.register_tool(Box::new(search_tool)).await?;
    
    info!("✅ 基础MCP服务器创建完成，已注册工具");

    // 3. 创建智能MCP服务器（集成多Agent架构）
    let intelligent_server = IntelligentMCPServer::new(mcp_server);
    
    // 4. 启动所有Agent
    intelligent_server.start().await?;
    info!("🤖 所有Agent已启动");

    // 5. 演示增强的工具调用
    info!("📞 演示1: 调用search_docs工具");
    let params1 = json!({
        "query": "Rust HTTP client libraries",
        "language": "rust"
    });
    
    let result1 = intelligent_server.handle_enhanced_tool_call("search_docs", params1).await?;
    info!("📋 增强响应1:");
    println!("{}", serde_json::to_string_pretty(&result1)?);

    // 6. 演示会话上下文效果
    info!("📞 演示2: 相关的后续查询");
    let params2 = json!({
        "query": "reqwest examples",
        "language": "rust"
    });
    
    let result2 = intelligent_server.handle_enhanced_tool_call("search_docs", params2).await?;
    info!("📋 增强响应2 (应该包含会话上下文):");
    println!("{}", serde_json::to_string_pretty(&result2)?);

    // 7. 演示不同类型的查询
    info!("📞 演示3: 不同技术栈查询");
    let params3 = json!({
        "query": "JavaScript async await",
        "language": "javascript"
    });
    
    let result3 = intelligent_server.handle_enhanced_tool_call("search_docs", params3).await?;
    info!("📋 增强响应3 (不同技术栈):");
    println!("{}", serde_json::to_string_pretty(&result3)?);

    // 8. 停止服务
    intelligent_server.stop().await?;
    info!("🛑 所有Agent已停止");

    info!("✅ 多Agent架构演示完成");

    Ok(())
}

// 可选：添加性能测试
#[allow(dead_code)]
async fn performance_test(intelligent_server: &IntelligentMCPServer) -> Result<()> {
    use std::time::Instant;
    
    info!("⚡ 开始性能测试");
    
    let test_queries = vec![
        ("Rust", "serde serialization"),
        ("Python", "fastapi async"),
        ("JavaScript", "express middleware"),
        ("TypeScript", "type definitions"),
        ("Java", "spring boot starter"),
    ];
    
    for (language, query) in test_queries {
        let start = Instant::now();
        
        let params = json!({
            "query": query,
            "language": language.to_lowercase()
        });
        
        let _result = intelligent_server.handle_enhanced_tool_call("search_docs", params).await?;
        
        let duration = start.elapsed();
        info!("🏃 {} 查询耗时: {:?}", language, duration);
    }
    
    info!("✅ 性能测试完成");
    
    Ok(())
}

// 可选：并发测试
#[allow(dead_code)]
async fn concurrent_test(intelligent_server: &IntelligentMCPServer) -> Result<()> {
    use tokio::task::JoinSet;
    
    info!("🔀 开始并发测试");
    
    let mut join_set = JoinSet::new();
    
    // 启动10个并发任务
    for i in 0..10 {
        let server = intelligent_server;
        join_set.spawn(async move {
            let params = json!({
                "query": format!("test query {}", i),
                "language": "rust"
            });
            
            server.handle_enhanced_tool_call("search_docs", params).await
        });
    }
    
    // 等待所有任务完成
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(response)) => results.push(response),
            Ok(Err(e)) => info!("❌ 任务失败: {}", e),
            Err(e) => info!("❌ 任务执行错误: {}", e),
        }
    }
    
    info!("✅ 并发测试完成，成功处理 {} 个请求", results.len());
    
    Ok(())
} 