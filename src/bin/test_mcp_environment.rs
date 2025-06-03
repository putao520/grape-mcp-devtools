use grape_mcp_devtools::mcp::server::MCPServer;
use grape_mcp_devtools::tools::{EnvironmentDetectionTool, SearchDocsTool, CheckVersionTool};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🚀 测试MCP服务器环境检测工具集成");
    println!("=====================================");

    // 创建MCP服务器
    let mcp_server = MCPServer::new();
    
    // 注册环境检测工具
    let env_tool = EnvironmentDetectionTool::new();
    mcp_server.register_tool(Box::new(env_tool)).await?;
    println!("✅ 环境检测工具已注册");
    
    // 注册其他基础工具
    let search_tool = SearchDocsTool::new();
    mcp_server.register_tool(Box::new(search_tool)).await?;
    println!("✅ 文档搜索工具已注册");
    
    let version_tool = CheckVersionTool::new();
    mcp_server.register_tool(Box::new(version_tool)).await?;
    println!("✅ 版本检查工具已注册");
    
    // 获取工具列表
    println!("\n📋 已注册的工具列表:");
    match mcp_server.list_tools().await {
        Ok(tools) => {
            for (i, tool) in tools.iter().enumerate() {
                println!("  {}. 🔧 {} - {}", i + 1, tool.name, tool.description);
                if let Some(lang) = &tool.language {
                    println!("     🗣️ 语言: {}", lang);
                }
            }
        }
        Err(e) => {
            println!("❌ 获取工具列表失败: {}", e);
        }
    }
    
    // 测试环境检测工具
    println!("\n🧪 测试环境检测工具执行:");
    println!("------------------------");
    
    let params = json!({
        "path": ".",
        "depth": 2,
        "include_dependencies": false
    });
    
    match mcp_server.execute_tool("detect_environment", params).await {
        Ok(result) => {
            println!("✅ 环境检测执行成功!");
            
            // 解析结果
            if let Some(env) = result.get("environment") {
                if let Some(primary) = env.get("primary_language").and_then(|v| v.as_str()) {
                    println!("🎯 主要语言: {}", primary);
                }
                
                if let Some(languages) = env.get("languages").and_then(|v| v.as_array()) {
                    println!("🗣️ 检测到的语言:");
                    for lang in languages.iter().take(3) {
                        if let (Some(name), Some(weight)) = (
                            lang.get("name").and_then(|v| v.as_str()),
                            lang.get("weight").and_then(|v| v.as_f64())
                        ) {
                            println!("   - {}: {:.1}%", name, weight * 100.0);
                        }
                    }
                }
                
                if let Some(project_type) = env.get("project_type") {
                    if let Some(category) = project_type.get("category").and_then(|v| v.as_str()) {
                        println!("📂 项目类型: {}", category);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ 环境检测执行失败: {}", e);
        }
    }
    
    // 测试批量工具执行
    println!("\n🧪 测试批量工具执行:");
    println!("-------------------");
    
    let requests = vec![
        grape_mcp_devtools::mcp::server::ToolRequest {
            tool_name: "detect_environment".to_string(),
            params: json!({
                "path": ".",
                "depth": 1,
                "include_dependencies": false
            }),
            timeout: None,
        },
        grape_mcp_devtools::mcp::server::ToolRequest {
            tool_name: "check_version".to_string(),
            params: json!({
                "type": "rust",
                "name": "tokio"
            }),
            timeout: None,
        },
    ];
    
    match mcp_server.batch_execute_tools(requests).await {
        Ok(results) => {
            println!("✅ 批量执行成功! 执行了 {} 个工具", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("  {}. {} - {}", 
                    i + 1, 
                    result.tool_name, 
                    if result.success { "成功" } else { "失败" }
                );
                if let Some(error) = &result.error {
                    println!("     ❌ 错误: {}", error);
                }
            }
        }
        Err(e) => {
            println!("❌ 批量执行失败: {}", e);
        }
    }
    
    // 测试工具健康状态
    println!("\n🏥 工具健康状态检查:");
    println!("-------------------");
    
    match mcp_server.get_tool_health_status().await {
        Ok(health_status) => {
            for (tool_name, health) in health_status {
                match health {
                    grape_mcp_devtools::mcp::server::ToolHealth::Healthy => {
                        println!("✅ {}: 健康", tool_name);
                    }
                    grape_mcp_devtools::mcp::server::ToolHealth::Degraded { reason } => {
                        println!("⚠️ {}: 降级 - {}", tool_name, reason);
                    }
                    grape_mcp_devtools::mcp::server::ToolHealth::Unhealthy { reason } => {
                        println!("❌ {}: 不健康 - {}", tool_name, reason);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ 健康检查失败: {}", e);
        }
    }
    
    // 获取性能统计
    println!("\n📊 性能统计:");
    println!("------------");
    
    match mcp_server.get_performance_stats().await {
        Ok(stats) => {
            for (key, value) in stats {
                println!("📈 {}: {}", key, value);
            }
        }
        Err(e) => {
            println!("❌ 获取性能统计失败: {}", e);
        }
    }
    
    println!("\n🎉 MCP环境检测工具集成测试完成!");
    println!("===================================");
    
    Ok(())
} 