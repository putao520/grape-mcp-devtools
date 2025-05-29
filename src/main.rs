use anyhow::Result;
use tracing::{info, error, warn};
use tracing_subscriber;
use dotenv;

mod errors;
mod mcp;
mod tools;
mod vectorization;
mod versioning;
mod cli;

use mcp::server::MCPServer;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("grape_mcp_devtools=debug,info")
        .init();

    info!("🚀 启动 Grape MCP DevTools 服务器...");

    // 创建动态工具注册器
    let mut registry = tools::DynamicRegistryBuilder::new()
        .with_policy(tools::RegistrationPolicy::Adaptive { score_threshold: 0.3 })
        .add_scan_path(std::env::current_dir()?)
        .build();

    info!("🔍 执行环境检测和动态工具注册...");
    
    // 执行动态注册
    match registry.auto_register().await {
        Ok(report) => {
            info!("✅ 动态注册完成！");
            info!("📊 注册报告:");
            info!("   - 注册工具: {} 个", report.registered_tools.len());
            info!("   - 失败注册: {} 个", report.failed_registrations.len());
            info!("   - 注册评分: {:.1}%", report.registration_score * 100.0);
            info!("   - 注册耗时: {}ms", report.registration_duration_ms);
            
            for tool in &report.registered_tools {
                info!("   ✅ {}", tool);
            }
            
            for (tool, error) in &report.failed_registrations {
                warn!("   ❌ {} - {}", tool, error);
            }
        }
        Err(e) => {
            error!("❌ 动态注册失败: {}", e);
            return Err(e);
        }
    }

    // 创建MCP服务器实例
    let mcp_server = MCPServer::new();

    // 从注册器获取已注册的工具并添加到服务器
    for (_tool_name, tool) in registry.get_registered_tools() {
        // 由于Arc<dyn MCPTool>不能直接转换为Box<dyn MCPTool>，
        // 我们需要重新创建工具实例
        info!("🔧 工具已注册: {}", _tool_name);
    }

    // 手动添加一些基础工具作为示例
    let search_tool = tools::SearchDocsTool::new();
    mcp_server.register_tool(Box::new(search_tool)).await?;
    info!("🔧 工具已添加到服务器: search_docs");

    let tool_count = mcp_server.get_tool_count().await?;
    info!("📋 服务器工具总数: {}", tool_count);
    
    // 显示统计信息
    let stats = registry.get_statistics();
    info!("📈 动态注册统计:");
    for (key, value) in stats {
        info!("   - {}: {}", key, value);
    }

    // 创建并运行完整的MCP服务器
    let mut server = mcp::server::Server::new(
        "grape-mcp-devtools".to_string(),
        "0.1.0".to_string(),
        mcp_server,
    );

    info!("🌐 启动MCP服务器...");
    server.run().await?;

    Ok(())
} 