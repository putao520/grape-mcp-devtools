use anyhow::Result;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// MCP集成测试套件
/// 测试完整的MCP服务器-客户端通信流程
#[tokio::test]
async fn test_mcp_server_client_integration() -> Result<()> {
    println!("🧪 开始MCP服务器-客户端集成测试");
    
    // 启动MCP服务器
    let mut server_process = Command::new("cargo")
        .args(&["run", "--bin", "grape-mcp-devtools"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let mut stdin = server_process.stdin.take().unwrap();
    let stdout = server_process.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    
    // 等待服务器启动
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 1. 初始化测试
    println!("🔧 测试MCP初始化");
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "init-1",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });
    
    send_mcp_request(&mut stdin, &init_request)?;
    let init_response = read_mcp_response(&mut reader).await?;
    
    assert!(init_response.contains("result"));
    println!("✅ MCP初始化成功");
    
    // 2. 工具列表测试
    println!("📚 测试工具列表获取");
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": "tools-1",
        "method": "tools/list",
        "params": {}
    });
    
    send_mcp_request(&mut stdin, &tools_request)?;
    let tools_response = read_mcp_response(&mut reader).await?;
    
    assert!(tools_response.contains("tools"));
    println!("✅ 工具列表获取成功");
    
    // 3. 文档搜索工具测试
    println!("🔍 测试文档搜索工具");
    let search_request = json!({
        "jsonrpc": "2.0",
        "id": "search-1",
        "method": "tools/call",
        "params": {
            "name": "search_docs",
            "arguments": {
                "query": "rust async",
                "language": "rust",
                "limit": 5
            }
        }
    });
    
    send_mcp_request(&mut stdin, &search_request)?;
    let search_response = read_mcp_response(&mut reader).await?;
    
    assert!(search_response.contains("content") || search_response.contains("result"));
    println!("✅ 文档搜索工具测试成功");
    
    // 4. 版本检查工具测试
    println!("📦 测试版本检查工具");
    let version_request = json!({
        "jsonrpc": "2.0",
        "id": "version-1",
        "method": "tools/call",
        "params": {
            "name": "check_latest_version",
            "arguments": {
                "language": "rust",
                "package_name": "tokio"
            }
        }
    });
    
    send_mcp_request(&mut stdin, &version_request)?;
    let version_response = read_mcp_response(&mut reader).await?;
    
    assert!(version_response.contains("content") || version_response.contains("result"));
    println!("✅ 版本检查工具测试成功");
    
    // 5. API文档工具测试
    println!("📖 测试API文档工具");
    let api_request = json!({
        "jsonrpc": "2.0",
        "id": "api-1",
        "method": "tools/call",
        "params": {
            "name": "get_api_docs",
            "arguments": {
                "language": "rust",
                "package_name": "serde",
                "item_type": "overview"
            }
        }
    });
    
    send_mcp_request(&mut stdin, &api_request)?;
    let api_response = read_mcp_response(&mut reader).await?;
    
    assert!(api_response.contains("content") || api_response.contains("result"));
    println!("✅ API文档工具测试成功");
    
    // 6. 错误处理测试
    println!("❌ 测试错误处理");
    let error_request = json!({
        "jsonrpc": "2.0",
        "id": "error-1",
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",
            "arguments": {}
        }
    });
    
    send_mcp_request(&mut stdin, &error_request)?;
    let error_response = read_mcp_response(&mut reader).await?;
    
    assert!(error_response.contains("error"));
    println!("✅ 错误处理测试成功");
    
    // 7. 批量工具调用测试
    println!("🔄 测试批量工具调用");
    let batch_request = json!({
        "jsonrpc": "2.0",
        "id": "batch-1",
        "method": "tools/batch_call",
        "params": {
            "requests": [
                {
                    "name": "search_docs",
                    "arguments": {
                        "query": "http client",
                        "language": "rust",
                        "limit": 3
                    }
                },
                {
                    "name": "check_latest_version",
                    "arguments": {
                        "language": "python",
                        "package_name": "requests"
                    }
                }
            ]
        }
    });
    
    send_mcp_request(&mut stdin, &batch_request)?;
    let batch_response = read_mcp_response(&mut reader).await?;
    
    assert!(batch_response.contains("results") || batch_response.contains("content"));
    println!("✅ 批量工具调用测试成功");
    
    // 8. 健康检查测试
    println!("🏥 测试健康检查");
    let health_request = json!({
        "jsonrpc": "2.0",
        "id": "health-1",
        "method": "health_check",
        "params": {}
    });
    
    send_mcp_request(&mut stdin, &health_request)?;
    let health_response = read_mcp_response(&mut reader).await?;
    
    assert!(health_response.contains("overall_status") || health_response.contains("result"));
    println!("✅ 健康检查测试成功");
    
    // 9. 性能统计测试
    println!("📊 测试性能统计");
    let stats_request = json!({
        "jsonrpc": "2.0",
        "id": "stats-1",
        "method": "get_stats",
        "params": {}
    });
    
    send_mcp_request(&mut stdin, &stats_request)?;
    let stats_response = read_mcp_response(&mut reader).await?;
    
    assert!(stats_response.contains("tool_count") || stats_response.contains("result"));
    println!("✅ 性能统计测试成功");
    
    // 清理：终止服务器进程
    server_process.kill()?;
    println!("🎉 MCP集成测试全部通过！");
    
    Ok(())
}

/// 测试MCP服务器的并发处理能力
#[tokio::test]
async fn test_mcp_concurrent_requests() -> Result<()> {
    println!("🔄 开始MCP并发请求测试");
    
    let mut server_process = Command::new("cargo")
        .args(&["run", "--bin", "grape-mcp-devtools"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let mut stdin = server_process.stdin.take().unwrap();
    let stdout = server_process.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    
    // 等待服务器启动
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 初始化
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "init-concurrent",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "clientInfo": {
                "name": "concurrent-test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });
    
    send_mcp_request(&mut stdin, &init_request)?;
    let _init_response = read_mcp_response(&mut reader).await?;
    
    // 并发发送多个请求
    let start_time = Instant::now();
    let concurrent_requests = 5;
    
    for i in 0..concurrent_requests {
        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("concurrent-{}", i),
            "method": "tools/call",
            "params": {
                "name": "search_docs",
                "arguments": {
                    "query": format!("test query {}", i),
                    "language": "rust",
                    "limit": 2
                }
            }
        });
        
        send_mcp_request(&mut stdin, &request)?;
        
        // 短暂延迟模拟并发
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    // 读取所有响应
    let mut received_responses = 0;
    for _ in 0..concurrent_requests {
        let response = read_mcp_response(&mut reader).await?;
        if !response.is_empty() {
            received_responses += 1;
        }
    }
    
    let elapsed = start_time.elapsed();
    
    assert_eq!(received_responses, concurrent_requests);
    println!("✅ 并发请求测试成功：{} 个请求在 {:?} 内完成", received_responses, elapsed);
    
    server_process.kill()?;
    Ok(())
}

/// 测试MCP服务器的错误恢复能力
#[tokio::test]
async fn test_mcp_error_recovery() -> Result<()> {
    println!("🛠️ 开始MCP错误恢复测试");
    
    let mut server_process = Command::new("cargo")
        .args(&["run", "--bin", "grape-mcp-devtools"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let mut stdin = server_process.stdin.take().unwrap();
    let stdout = server_process.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 初始化
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "init-error",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "clientInfo": {
                "name": "error-recovery-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });
    
    send_mcp_request(&mut stdin, &init_request)?;
    let _init_response = read_mcp_response(&mut reader).await?;
    
    // 1. 测试格式错误的JSON
    println!("📝 测试格式错误的JSON");
    writeln!(stdin, "{{invalid json}}")?;
    stdin.flush()?;
    
    // 2. 测试缺少必要字段的请求
    println!("🔍 测试缺少必要字段的请求");
    let invalid_request = json!({
        "jsonrpc": "2.0",
        "id": "invalid-1"
        // 缺少method字段
    });
    send_mcp_request(&mut stdin, &invalid_request)?;
    
    // 3. 测试不存在的方法
    println!("🚫 测试不存在的方法");
    let nonexistent_method = json!({
        "jsonrpc": "2.0",
        "id": "nonexistent-1",
        "method": "nonexistent_method",
        "params": {}
    });
    send_mcp_request(&mut stdin, &nonexistent_method)?;
    
    // 4. 测试错误参数的工具调用
    println!("⚠️ 测试错误参数的工具调用");
    let bad_params = json!({
        "jsonrpc": "2.0",
        "id": "bad-params-1",
        "method": "tools/call",
        "params": {
            "name": "search_docs",
            "arguments": {
                // 缺少必要的参数
                "invalid_param": "value"
            }
        }
    });
    send_mcp_request(&mut stdin, &bad_params)?;
    
    // 验证服务器在错误后仍能正常工作
    println!("✅ 验证服务器恢复能力");
    let recovery_request = json!({
        "jsonrpc": "2.0",
        "id": "recovery-1",
        "method": "tools/list",
        "params": {}
    });
    
    send_mcp_request(&mut stdin, &recovery_request)?;
    let recovery_response = read_mcp_response(&mut reader).await?;
    
    assert!(recovery_response.contains("tools") || recovery_response.contains("result"));
    println!("✅ 服务器错误恢复测试成功");
    
    server_process.kill()?;
    Ok(())
}

/// 测试MCP服务器的性能基准
#[tokio::test]
async fn test_mcp_performance_benchmark() -> Result<()> {
    println!("⚡ 开始MCP性能基准测试");
    
    let mut server_process = Command::new("cargo")
        .args(&["run", "--bin", "grape-mcp-devtools", "--release"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let mut stdin = server_process.stdin.take().unwrap();
    let stdout = server_process.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // 初始化
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "init-perf",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "clientInfo": {
                "name": "performance-test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });
    
    send_mcp_request(&mut stdin, &init_request)?;
    let _init_response = read_mcp_response(&mut reader).await?;
    
    // 性能测试：快速连续请求
    let test_iterations = 10;
    let start_time = Instant::now();
    
    for i in 0..test_iterations {
        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("perf-{}", i),
            "method": "tools/list",
            "params": {}
        });
        
        send_mcp_request(&mut stdin, &request)?;
        let response = read_mcp_response(&mut reader).await?;
        assert!(!response.is_empty());
    }
    
    let elapsed = start_time.elapsed();
    let avg_response_time = elapsed / test_iterations;
    
    println!("📊 性能统计:");
    println!("   - 总请求数: {}", test_iterations);
    println!("   - 总耗时: {:?}", elapsed);
    println!("   - 平均响应时间: {:?}", avg_response_time);
    println!("   - 请求/秒: {:.2}", test_iterations as f64 / elapsed.as_secs_f64());
    
    // 性能要求：平均响应时间应该在合理范围内
    assert!(avg_response_time < Duration::from_millis(1000), 
            "平均响应时间过长: {:?}", avg_response_time);
    
    println!("✅ 性能基准测试通过");
    
    server_process.kill()?;
    Ok(())
}

/// 辅助函数：发送MCP请求
fn send_mcp_request(stdin: &mut std::process::ChildStdin, request: &Value) -> Result<()> {
    let request_json = serde_json::to_string(request)?;
    writeln!(stdin, "{}", request_json)?;
    stdin.flush()?;
    Ok(())
}

/// 辅助函数：读取MCP响应
async fn read_mcp_response(reader: &mut BufReader<std::process::ChildStdout>) -> Result<String> {
    // 使用tokio的异步包装来避免阻塞
    let result = timeout(Duration::from_secs(10), async {
        // 创建一个新的字符串来避免借用问题
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => Ok(line.trim().to_string()),
            Err(e) => Err(anyhow::anyhow!("IO错误: {}", e)),
        }
    }).await;
    
    match result {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow::anyhow!("响应超时")),
    }
}

/// 集成测试：完整的工作流程测试
#[tokio::test]
async fn test_mcp_complete_workflow() -> Result<()> {
    println!("🎯 开始MCP完整工作流程测试");
    
    let mut server_process = Command::new("cargo")
        .args(&["run", "--bin", "grape-mcp-devtools"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let mut stdin = server_process.stdin.take().unwrap();
    let stdout = server_process.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 完整工作流程：初始化 -> 发现工具 -> 使用工具 -> 检查健康状态 -> 获取统计
    
    // 1. 初始化
    println!("🔧 步骤 1: 初始化连接");
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-init",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "clientInfo": {
                "name": "workflow-test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });
    
    send_mcp_request(&mut stdin, &init_request)?;
    let init_response = read_mcp_response(&mut reader).await?;
    assert!(init_response.contains("result"));
    
    // 2. 发现可用工具
    println!("🔍 步骤 2: 发现可用工具");
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-tools",
        "method": "tools/list",
        "params": {}
    });
    
    send_mcp_request(&mut stdin, &tools_request)?;
    let tools_response = read_mcp_response(&mut reader).await?;
    assert!(tools_response.contains("tools"));
    
    // 3. 使用核心工具（模拟实际使用场景）
    println!("🛠️ 步骤 3: 使用核心工具");
    
    // 3.1 搜索文档
    let search_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-search",
        "method": "tools/call",
        "params": {
            "name": "search_docs",
            "arguments": {
                "query": "async programming",
                "language": "rust",
                "limit": 3
            }
        }
    });
    
    send_mcp_request(&mut stdin, &search_request)?;
    let search_response = read_mcp_response(&mut reader).await?;
    assert!(search_response.contains("content") || search_response.contains("result"));
    
    // 3.2 检查版本信息
    let version_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-version",
        "method": "tools/call",
        "params": {
            "name": "check_latest_version",
            "arguments": {
                "language": "rust",
                "package_name": "tokio"
            }
        }
    });
    
    send_mcp_request(&mut stdin, &version_request)?;
    let version_response = read_mcp_response(&mut reader).await?;
    assert!(version_response.contains("content") || version_response.contains("result"));
    
    // 4. 检查系统健康状态
    println!("🏥 步骤 4: 检查系统健康状态");
    let health_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-health",
        "method": "health_check",
        "params": {}
    });
    
    send_mcp_request(&mut stdin, &health_request)?;
    let health_response = read_mcp_response(&mut reader).await?;
    assert!(health_response.contains("overall_status") || health_response.contains("result"));
    
    // 5. 获取性能统计
    println!("📊 步骤 5: 获取性能统计");
    let stats_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-stats",
        "method": "get_stats",
        "params": {}
    });
    
    send_mcp_request(&mut stdin, &stats_request)?;
    let stats_response = read_mcp_response(&mut reader).await?;
    assert!(stats_response.contains("tool_count") || stats_response.contains("result"));
    
    server_process.kill()?;
    println!("🎉 完整工作流程测试成功完成！");
    
    Ok(())
} 