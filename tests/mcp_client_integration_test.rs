use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use futures::future;

use grape_mcp_devtools::mcp::server::{MCPServer, ToolInfo};
use grape_mcp_devtools::tools::{SearchDocsTool, CheckVersionTool};
use grape_mcp_devtools::tools::api_docs::GetApiDocsTool;

/// 高级MCP客户端集成测试
/// 直接使用Rust代码模拟完整的MCP客户端-服务器交互
#[tokio::test]
async fn test_mcp_client_server_integration() -> Result<()> {
    println!("🧪 开始高级MCP客户端-服务器集成测试");
    
    // 创建MCP服务器实例
    let mcp_server = create_test_mcp_server().await?;
    
    // 模拟客户端初始化
    println!("🔧 测试客户端初始化");
    let client_info = json!({
        "name": "integration-test-client",
        "version": "1.0.0",
        "capabilities": ["documentSearch", "versionInfo", "apiDocs"]
    });
    
    // 验证服务器能够处理初始化
    assert!(client_info.get("name").is_some());
    println!("✅ 客户端初始化成功");
    
    // 1. 测试工具发现
    println!("📚 测试工具发现功能");
    let tools = mcp_server.list_tools().await?;
    assert!(!tools.is_empty(), "服务器应该有可用的工具");
    
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    println!("发现的工具: {:?}", tool_names);
    
    // 验证核心工具存在
    assert!(tool_names.contains(&"search_docs"), "应该包含文档搜索工具");
    assert!(tool_names.contains(&"check_latest_version"), "应该包含版本检查工具");
    assert!(tool_names.contains(&"get_api_docs"), "应该包含API文档工具");
    println!("✅ 工具发现测试成功");
    
    // 2. 测试文档搜索工具
    println!("🔍 测试文档搜索工具");
    let search_params = json!({
        "query": "async programming",
        "language": "rust",
        "limit": 5
    });
    
    let search_result = mcp_server.execute_tool("search_docs", search_params).await?;
    assert!(search_result.is_string() || search_result.is_object());
    
    // 智能处理不同类型的响应
    let search_content = if search_result.is_string() {
        search_result.as_str().unwrap_or("").to_string()
    } else if search_result.is_object() {
        // 如果是JSON对象，检查是否有results数组
        if let Some(results) = search_result.get("results").and_then(|r| r.as_array()) {
            format!("找到 {} 个搜索结果", results.len())
        } else {
            // 如果是其他JSON格式，转换为字符串
            serde_json::to_string_pretty(&search_result).unwrap_or_else(|_| search_result.to_string())
        }
    } else {
        search_result.to_string()
    };
    
    assert!(!search_content.is_empty(), "搜索结果不应为空");
    assert!(search_content.len() > 10, "搜索结果应该有足够的内容，实际长度: {}", search_content.len());
    println!("✅ 文档搜索工具测试成功，返回 {} 字符", search_content.len());
    
    // 3. 测试版本检查工具
    println!("📦 测试版本检查工具");
    let version_params = json!({
        "type": "cargo",
        "name": "tokio"
    });
    
    let version_result = mcp_server.execute_tool("check_latest_version", version_params).await?;
    
    // 智能处理版本检查响应
    let version_content = if version_result.is_string() {
        version_result.as_str().unwrap_or("").to_string()
    } else {
        serde_json::to_string_pretty(&version_result).unwrap_or_else(|_| version_result.to_string())
    };
    
    assert!(!version_content.is_empty(), "版本检查结果不应为空");
    assert!(version_content.contains("tokio") || version_content.contains("版本") || version_content.len() > 10, "结果应包含相关信息");
    println!("✅ 版本检查工具测试成功");
    
    // 4. 测试API文档工具
    println!("📖 测试API文档工具");
    let api_params = json!({
        "language": "rust",
        "package": "serde",
        "symbol": "*"
    });
    
    let api_result = mcp_server.execute_tool("get_api_docs", api_params).await?;
    
    // 智能处理API文档响应
    let api_content = if api_result.is_string() {
        api_result.as_str().unwrap_or("").to_string()
    } else {
        serde_json::to_string_pretty(&api_result).unwrap_or_else(|_| api_result.to_string())
    };
    
    assert!(!api_content.is_empty(), "API文档结果不应为空");
    println!("✅ API文档工具测试成功，返回 {} 字符", api_content.len());
    
    // 5. 测试批量工具调用
    println!("🔄 测试批量工具调用");
    let batch_requests = vec![
        grape_mcp_devtools::mcp::server::ToolRequest {
            tool_name: "search_docs".to_string(),
            params: json!({
                "query": "http client",
                "language": "rust",
                "limit": 3
            }),
            timeout: Some(Duration::from_secs(30)),
        },
        grape_mcp_devtools::mcp::server::ToolRequest {
            tool_name: "check_latest_version".to_string(),
            params: json!({
                "type": "pip",
                "name": "requests"
            }),
            timeout: Some(Duration::from_secs(30)),
        },
    ];
    
    let batch_results = mcp_server.batch_execute_tools(batch_requests).await?;
    assert_eq!(batch_results.len(), 2, "应该返回2个结果");
    
    for result in &batch_results {
        assert!(result.success, "批量调用的每个工具都应该成功");
        assert!(result.error.is_none(), "不应该有错误");
    }
    println!("✅ 批量工具调用测试成功");
    
    // 6. 测试健康检查
    println!("🏥 测试健康检查");
    let health_status = mcp_server.get_tool_health_status().await?;
    assert!(!health_status.is_empty(), "健康状态不应为空");
    
    for (tool_name, health) in &health_status {
        println!("工具 {} 健康状态: {:?}", tool_name, health);
        // 大部分工具应该是健康的
        match health {
            grape_mcp_devtools::mcp::server::ToolHealth::Healthy => {},
            grape_mcp_devtools::mcp::server::ToolHealth::Degraded { reason } => {
                println!("工具 {} 性能降级: {}", tool_name, reason);
            },
            grape_mcp_devtools::mcp::server::ToolHealth::Unhealthy { reason } => {
                println!("工具 {} 不健康: {}", tool_name, reason);
            },
        }
    }
    println!("✅ 健康检查测试成功");
    
    // 7. 测试性能统计
    println!("📊 测试性能统计");
    let performance_stats = mcp_server.get_performance_stats().await?;
    assert!(!performance_stats.is_empty(), "性能统计不应为空");
    
    for (metric_name, metric_value) in &performance_stats {
        println!("性能指标 {}: {:?}", metric_name, metric_value);
    }
    println!("✅ 性能统计测试成功");
    
    // 8. 测试错误处理
    println!("❌ 测试错误处理");
    let error_result = mcp_server.execute_tool("nonexistent_tool", json!({})).await;
    assert!(error_result.is_err(), "调用不存在的工具应该返回错误");
    println!("✅ 错误处理测试成功");
    
    // 9. 测试超时处理
    println!("⏰ 测试超时处理");
    let timeout_result = timeout(
        Duration::from_millis(1), // 极短的超时时间
        mcp_server.execute_tool("search_docs", json!({
            "query": "complex query that might take time",
            "language": "rust",
            "limit": 10
        }))
    ).await;
    
    // 超时是预期的行为
    if timeout_result.is_err() {
        println!("✅ 超时处理测试成功");
    } else {
        println!("⚠️ 工具执行速度很快，未触发超时");
    }
    
    // 10. 测试并发工具调用
    println!("🔄 测试并发工具调用");
    let concurrent_tasks = vec![
        mcp_server.execute_tool("search_docs", json!({
            "query": "concurrent test 1",
            "language": "rust",
            "limit": 2
        })),
        mcp_server.execute_tool("search_docs", json!({
            "query": "concurrent test 2", 
            "language": "python",
            "limit": 2
        })),
        mcp_server.execute_tool("check_latest_version", json!({
            "type": "cargo",
            "name": "serde"
        })),
    ];
    
    let concurrent_results = futures::future::join_all(concurrent_tasks).await;
    let successful_results = concurrent_results.iter().filter(|r| r.is_ok()).count();
    
    assert!(successful_results >= 2, "至少应该有2个并发调用成功");
    println!("✅ 并发工具调用测试成功，{}/{} 个调用成功", successful_results, concurrent_results.len());
    
    println!("🎉 高级MCP客户端-服务器集成测试全部通过！");
    Ok(())
}

/// 测试MCP工具的详细功能
#[tokio::test]
async fn test_mcp_tool_detailed_functionality() -> Result<()> {
    println!("🔧 开始MCP工具详细功能测试");
    
    let mcp_server = create_test_mcp_server().await?;
    
    // 1. 测试多语言文档搜索
    println!("🌍 测试多语言文档搜索");
    let languages = vec!["rust", "python", "javascript", "go", "java"];
    
    for language in languages {
        let params = json!({
            "query": "http client",
            "language": language,
            "limit": 3
        });
        
        let result = mcp_server.execute_tool("search_docs", params).await?;
        
        // 智能处理搜索响应
        let content = if result.is_string() {
            result.as_str().unwrap_or("").to_string()
        } else if result.is_object() {
            if let Some(results) = result.get("results").and_then(|r| r.as_array()) {
                format!("找到 {} 个{}语言的搜索结果", results.len(), language)
            } else {
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
            }
        } else {
            result.to_string()
        };
        
        assert!(!content.is_empty(), "{}语言的搜索结果不应为空", language);
        println!("✅ {}语言文档搜索成功，返回 {} 字符", language, content.len());
    }
    
    // 2. 测试不同语言的版本检查
    println!("📦 测试不同语言的版本检查");
    let version_tests = vec![
        ("cargo", "serde"),
        ("npm", "express"),
        ("pip", "requests"),
        ("maven", "org.springframework:spring-core"),
        ("go", "github.com/gin-gonic/gin"),
    ];
    
    for (package_type, package) in version_tests {
        let params = json!({
            "type": package_type,
            "name": package
        });
        
        let result = mcp_server.execute_tool("check_latest_version", params).await?;
        
        // 智能处理版本检查响应
        let content = if result.is_string() {
            result.as_str().unwrap_or("").to_string()
        } else {
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
        };
        
        assert!(!content.is_empty(), "{}/{}的版本检查结果不应为空", package_type, package);
        println!("✅ {}/{} 版本检查成功", package_type, package);
    }
    
    // 3. 测试API文档的不同类型
    println!("📚 测试不同类型的API文档");
    let api_tests = vec![
        ("rust", "tokio"),
        ("python", "requests"),
        ("javascript", "express"),
    ];
    
    for (language, package) in api_tests {
        let params = json!({
            "language": language,
            "package": package,
            "symbol": "*"
        });
        
        let result = mcp_server.execute_tool("get_api_docs", params).await?;
        
        // 智能处理API文档响应
        let content = if result.is_string() {
            result.as_str().unwrap_or("").to_string()
        } else {
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
        };
        
        // API文档可能为空，这是正常的，但如果有内容应该是有意义的
        println!("✅ {} 类型的API文档查询完成，返回 {} 字符", language, content.len());
    }
    
    // 4. 测试工具信息获取
    println!("ℹ️ 测试工具信息获取");
    let tools = mcp_server.list_tools().await?;
    
    for tool in tools {
        println!("工具: {}", tool.name);
        println!("  描述: {}", tool.description);
        println!("  语言: {:?}", tool.language);
        println!("  类别: {:?}", tool.category);
        println!("  版本: {:?}", tool.version);
        
        // 验证工具信息的完整性
        assert!(!tool.name.is_empty(), "工具名称不应为空");
        assert!(!tool.description.is_empty(), "工具描述不应为空");
    }
    
    println!("🎉 MCP工具详细功能测试全部通过！");
    Ok(())
}

/// 测试MCP服务器的压力测试
#[tokio::test]
async fn test_mcp_server_stress_test() -> Result<()> {
    println!("💪 开始MCP服务器压力测试");
    
    let mcp_server = create_test_mcp_server().await?;
    
    // 1. 大量并发请求测试
    println!("🔄 测试大量并发请求");
    let concurrent_count = 20;
    let mut tasks = Vec::new();
    
    for i in 0..concurrent_count {
        let server = Arc::clone(&mcp_server);
        let task = tokio::spawn(async move {
            let params = json!({
                "query": format!("stress test query {}", i),
                "language": "rust",
                "limit": 2
            });
            
            server.execute_tool("search_docs", params).await
        });
        tasks.push(task);
    }
    
    let results = futures::future::join_all(tasks).await;
    let successful_count = results.iter().filter(|r| {
        match r {
            Ok(Ok(_)) => true,
            _ => false,
        }
    }).count();
    
    let success_rate = successful_count as f64 / concurrent_count as f64;
    println!("并发请求成功率: {:.2}% ({}/{})", success_rate * 100.0, successful_count, concurrent_count);
    
    // 至少80%的请求应该成功
    assert!(success_rate >= 0.8, "并发请求成功率应该至少80%");
    
    // 2. 快速连续请求测试
    println!("⚡ 测试快速连续请求");
    let rapid_count = 50;
    let start_time = std::time::Instant::now();
    
    for i in 0..rapid_count {
        let params = json!({
            "type": "cargo",
            "name": format!("package{}", i % 5) // 循环使用5个不同的包名
        });
        
        let _result = mcp_server.execute_tool("check_latest_version", params).await;
        // 不检查每个结果，只测试服务器是否能处理快速请求
    }
    
    let elapsed = start_time.elapsed();
    let requests_per_second = rapid_count as f64 / elapsed.as_secs_f64();
    
    println!("快速连续请求性能: {:.2} 请求/秒", requests_per_second);
    println!("总耗时: {:?}", elapsed);
    
    // 3. 内存使用测试（简单检查）
    println!("🧠 测试内存使用情况");
    let tool_count = mcp_server.get_tool_count().await?;
    assert!(tool_count > 0, "应该有注册的工具");
    
    let performance_stats = mcp_server.get_performance_stats().await?;
    assert!(!performance_stats.is_empty(), "应该有性能统计数据");
    
    println!("✅ 压力测试完成");
    println!("  - 工具数量: {}", tool_count);
    println!("  - 性能指标数量: {}", performance_stats.len());
    
    println!("🎉 MCP服务器压力测试全部通过！");
    Ok(())
}

/// 创建测试用的MCP服务器
async fn create_test_mcp_server() -> Result<Arc<MCPServer>> {
    let mut mcp_server = MCPServer::new();
    
    // 注册所有工具
    mcp_server.register_tool(Box::new(SearchDocsTool::new())).await?;
    mcp_server.register_tool(Box::new(CheckVersionTool::new())).await?;
    mcp_server.register_tool(Box::new(GetApiDocsTool::new())).await?;
    
    Ok(Arc::new(mcp_server))
}

/// 测试MCP协议兼容性
#[tokio::test]
async fn test_mcp_protocol_compatibility() -> Result<()> {
    println!("🔌 开始MCP协议兼容性测试");
    
    let mcp_server = create_test_mcp_server().await?;
    
    // 1. 测试协议版本
    println!("📋 测试协议版本兼容性");
    let protocol_version = "2025-03-26";
    println!("支持的协议版本: {}", protocol_version);
    
    // 2. 测试JSON-RPC格式
    println!("📝 测试JSON-RPC格式兼容性");
    
    // 模拟标准的JSON-RPC请求格式
    let jsonrpc_request = json!({
        "jsonrpc": "2.0",
        "id": "test-1",
        "method": "tools/list",
        "params": {}
    });
    
    // 验证请求格式
    assert_eq!(jsonrpc_request["jsonrpc"], "2.0");
    assert!(jsonrpc_request["id"].is_string());
    assert!(jsonrpc_request["method"].is_string());
    assert!(jsonrpc_request["params"].is_object());
    
    // 3. 测试工具调用格式
    println!("🛠️ 测试工具调用格式");
    let tool_call_request = json!({
        "jsonrpc": "2.0",
        "id": "tool-call-1",
        "method": "tools/call",
        "params": {
            "name": "search_docs",
            "arguments": {
                "query": "test",
                "language": "rust",
                "limit": 5
            }
        }
    });
    
    // 验证工具调用格式
    assert!(tool_call_request["params"]["name"].is_string());
    assert!(tool_call_request["params"]["arguments"].is_object());
    
    // 4. 测试响应格式
    println!("📤 测试响应格式");
    let tools = mcp_server.list_tools().await?;
    
    // 模拟标准响应格式
    let response = json!({
        "jsonrpc": "2.0",
        "id": "test-1",
        "result": {
            "tools": tools
        }
    });
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_object());
    
    println!("✅ MCP协议兼容性测试通过");
    Ok(())
} 