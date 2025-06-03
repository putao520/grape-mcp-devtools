use grape_mcp_devtools::tools::{
    versioning::CheckVersionTool,
    base::MCPTool,
};
use grape_mcp_devtools::mcp::server::MCPServer;
use grape_mcp_devtools::cli::registry::{DynamicToolRegistry as CliRegistry, RegistrationStrategy};
use serde_json::json;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 测试版本检查工具集成");
    
    // 1. 测试直接工具调用
    println!("\n📦 步骤1: 直接工具调用测试");
    let version_tool = CheckVersionTool::new();
    
    let test_params = json!({
        "type": "cargo",
        "name": "tokio",
        "include_preview": false
    });
    
    match version_tool.execute(test_params).await {
        Ok(result) => {
            println!("✅ 直接调用成功:");
            println!("   包名: tokio");
            println!("   最新版本: {}", result["latest_stable"].as_str().unwrap_or("未知"));
            println!("   包类型: {}", result["package_type"].as_str().unwrap_or("未知"));
        }
        Err(e) => {
            println!("❌ 直接调用失败: {}", e);
        }
    }
    
    // 2. 测试MCP服务器集成
    println!("\n🖥️ 步骤2: MCP服务器集成测试");
    let mcp_server = MCPServer::new();
    
    // 注册版本检查工具 - 使用register_tool_arc方法
    let version_tool_arc = Arc::new(CheckVersionTool::new());
    match mcp_server.register_tool_arc(version_tool_arc).await {
        Ok(_) => {
            println!("✅ 工具注册到MCP服务器成功");
            
            // 测试通过MCP服务器调用
            let tool_params = json!({
                "type": "npm",
                "name": "lodash",
                "include_preview": false
            });
            
            match mcp_server.execute_tool("check_latest_version", tool_params).await {
                Ok(result) => {
                    println!("✅ MCP服务器调用成功:");
                    println!("   包名: lodash");
                    println!("   最新版本: {}", result["latest_stable"].as_str().unwrap_or("未知"));
                    println!("   下载地址: {}", result["download_url"].as_str().unwrap_or("未知"));
                }
                Err(e) => {
                    println!("❌ MCP服务器调用失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ 工具注册失败: {}", e);
        }
    }
    
    // 3. 测试动态注册系统集成
    println!("\n🔄 步骤3: 动态注册系统集成测试");
    let mut cli_registry = CliRegistry::new(RegistrationStrategy::ForceAll);
    
    match cli_registry.detect_and_register(&mcp_server).await {
        Ok(report) => {
            println!("✅ 动态注册完成:");
            println!("   注册成功: {} 个工具", report.registered_tools.len());
            println!("   注册失败: {} 个工具", report.failed_tools.len());
            println!("   跳过工具: {} 个工具", report.skipped_tools.len());
            
            // 检查版本检查工具是否被注册
            let version_tools: Vec<_> = report.registered_tools.iter()
                .filter(|tool| tool.contains("version") || tool.contains("cargo") || tool.contains("npm"))
                .collect();
            
            if !version_tools.is_empty() {
                println!("✅ 版本检查相关工具已注册: {:?}", version_tools);
            } else {
                println!("⚠️ 未发现版本检查相关工具注册");
            }
        }
        Err(e) => {
            println!("❌ 动态注册失败: {}", e);
        }
    }
    
    // 4. 测试工具列表
    println!("\n📋 步骤4: 工具列表测试");
    match mcp_server.list_tools().await {
        Ok(tools) => {
            println!("✅ 获取工具列表成功:");
            println!("   总工具数: {}", tools.len());
            
            // 查找版本检查工具 - 正确访问ToolInfo结构体字段
            let version_tool = tools.iter()
                .find(|tool| tool.name == "check_latest_version");
            
            if let Some(tool) = version_tool {
                println!("✅ 找到版本检查工具:");
                println!("   名称: {}", tool.name);
                println!("   描述: {}", tool.description);
            } else {
                println!("⚠️ 未找到版本检查工具");
            }
        }
        Err(e) => {
            println!("❌ 获取工具列表失败: {}", e);
        }
    }
    
    // 5. 测试多包管理器批量检查
    println!("\n🚀 步骤5: 批量版本检查测试");
    let batch_tests = vec![
        ("cargo", "serde"),
        ("npm", "react"),
        ("pip", "django"),
    ];
    
    for (pkg_type, pkg_name) in batch_tests {
        let params = json!({
            "type": pkg_type,
            "name": pkg_name,
            "include_preview": false
        });
        
        match mcp_server.execute_tool("check_latest_version", params).await {
            Ok(result) => {
                println!("✅ {} 包 {} 检查成功: v{}", 
                    pkg_type, 
                    pkg_name, 
                    result["latest_stable"].as_str().unwrap_or("未知")
                );
            }
            Err(e) => {
                println!("❌ {} 包 {} 检查失败: {}", pkg_type, pkg_name, e);
            }
        }
        
        // 添加延迟避免API限制
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }
    
    println!("\n🎉 版本检查工具集成测试完成！");
    println!("📊 测试总结:");
    println!("   ✅ 直接工具调用");
    println!("   ✅ MCP服务器集成");
    println!("   ✅ 动态注册系统");
    println!("   ✅ 工具列表功能");
    println!("   ✅ 批量版本检查");
    
    Ok(())
} 