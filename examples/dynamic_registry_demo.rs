use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use grape_mcp_devtools::{
    cli::{DynamicToolRegistry, RegistrationStrategy},
    mcp::server::MCPServer,
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("🎯 动态MCP工具注册系统演示");
    println!("{}", "=".repeat(50));

    // 创建MCP服务器
    let mcp_server = MCPServer::new();
    
    // 演示不同的注册策略
    demo_only_available_strategy(&mcp_server).await?;
    demo_force_all_strategy(&mcp_server).await?;
    demo_feature_based_strategy(&mcp_server).await?;

    Ok(())
}

/// 演示仅注册可用工具策略
async fn demo_only_available_strategy(mcp_server: &MCPServer) -> Result<()> {
    println!("\n📦 策略 1: OnlyAvailable - 仅注册检测到的可用工具");
    println!("{}", "-".repeat(50));
    
    let strategy = RegistrationStrategy::OnlyAvailable;
    let mut registry = DynamicToolRegistry::new(strategy);
    
    let report = registry.detect_and_register(mcp_server).await?;
    
    println!("📊 检测报告:");
    println!("{}", registry.get_detection_report());
    
    println!("📋 注册报告:");
    println!("{}", report.generate_report());
    
    let (success, failed, skipped) = report.get_stats();
    println!("✨ 总结: {} 成功, {} 失败, {} 跳过", success, failed, skipped);
    
    Ok(())
}

/// 演示强制注册所有工具策略
async fn demo_force_all_strategy(mcp_server: &MCPServer) -> Result<()> {
    println!("\n🔧 策略 2: ForceAll - 强制注册所有工具");
    println!("{}", "-".repeat(50));
    
    let strategy = RegistrationStrategy::ForceAll;
    let mut registry = DynamicToolRegistry::new(strategy);
    
    let report = registry.detect_and_register(mcp_server).await?;
    
    println!("📋 注册报告:");
    println!("{}", report.generate_report());
    
    let (success, failed, skipped) = report.get_stats();
    println!("✨ 总结: {} 成功, {} 失败, {} 跳过", success, failed, skipped);
    
    Ok(())
}

/// 演示基于特性的注册策略
async fn demo_feature_based_strategy(mcp_server: &MCPServer) -> Result<()> {
    println!("\n🎯 策略 3: FeatureBased - 仅注册构建工具");
    println!("{}", "-".repeat(50));
    
    let features = vec!["build-tool".to_string(), "package-manager".to_string()];
    let strategy = RegistrationStrategy::FeatureBased(features);
    let mut registry = DynamicToolRegistry::new(strategy);
    
    let report = registry.detect_and_register(mcp_server).await?;
    
    println!("📋 注册报告:");
    println!("{}", report.generate_report());
    
    let (success, failed, skipped) = report.get_stats();
    println!("✨ 总结: {} 成功, {} 失败, {} 跳过", success, failed, skipped);
    
    // 显示特定特性的工具
    let build_tools = registry.get_available_tools()
        .into_iter()
        .filter(|tool| tool.features.contains(&"build-tool".to_string()))
        .collect::<Vec<_>>();
    
    println!("\n🔨 检测到的构建工具:");
    for tool in build_tools {
        let version_str = tool.version.as_ref()
            .map(|v| format!(" ({})", v))
            .unwrap_or_default();
        println!("  • {}{}", tool.name, version_str);
    }
    
    Ok(())
} 