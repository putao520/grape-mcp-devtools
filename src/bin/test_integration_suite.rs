use std::time::Instant;
use serde_json::json;
use anyhow::Result;
use tracing::{info, error};

use grape_mcp_devtools::{
    mcp::server::MCPServer,
    tools::{
        dynamic_registry::{DynamicToolRegistry, RegistrationPolicy},
        versioning::CheckVersionTool,
        api_docs::GetApiDocsTool,
        doc_processor::DocumentProcessor,
        enhanced_doc_processor::EnhancedDocumentProcessor,
        vector_docs_tool::VectorDocsTool,
        base::MCPTool,
    },
};

/// 集成测试套件
struct IntegrationTestSuite {
    test_results: Vec<TestResult>,
    start_time: Instant,
}

#[derive(Debug, Clone)]
struct TestResult {
    test_name: String,
    success: bool,
    duration_ms: u64,
    details: String,
    error_message: Option<String>,
}

impl IntegrationTestSuite {
    fn new() -> Self {
        Self {
            test_results: Vec::new(),
            start_time: Instant::now(),
        }
    }

    async fn run_test<F, Fut>(&mut self, test_name: &str, test_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let start = Instant::now();
        info!("🧪 开始测试: {}", test_name);
        
        match test_fn().await {
            Ok(details) => {
                let duration = start.elapsed().as_millis() as u64;
                info!("✅ 测试通过: {} ({}ms)", test_name, duration);
                
                self.test_results.push(TestResult {
                    test_name: test_name.to_string(),
                    success: true,
                    duration_ms: duration,
                    details,
                    error_message: None,
                });
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                error!("❌ 测试失败: {} - {}", test_name, e);
                
                self.test_results.push(TestResult {
                    test_name: test_name.to_string(),
                    success: false,
                    duration_ms: duration,
                    details: "测试失败".to_string(),
                    error_message: Some(e.to_string()),
                });
            }
        }
        
        Ok(())
    }

    fn generate_report(&self) -> String {
        let total_tests = self.test_results.len();
        let passed_tests = self.test_results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;
        let total_duration = self.start_time.elapsed().as_millis();
        
        let mut report = format!(
            "\n🎯 集成测试报告\n{}\n",
            "=".repeat(50)
        );
        
        report.push_str(&format!(
            "📊 总体统计:\n  • 总测试数: {}\n  • 通过: {} ✅\n  • 失败: {} ❌\n  • 总耗时: {}ms\n\n",
            total_tests, passed_tests, failed_tests, total_duration
        ));
        
        report.push_str("📋 详细结果:\n");
        for result in &self.test_results {
            let status = if result.success { "✅" } else { "❌" };
            report.push_str(&format!(
                "  {} {} ({}ms)\n",
                status, result.test_name, result.duration_ms
            ));
            
            if !result.success {
                if let Some(error) = &result.error_message {
                    report.push_str(&format!("     错误: {}\n", error));
                }
            }
        }
        
        report.push_str(&format!("\n{}\n", "=".repeat(50)));
        report
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    info!("🚀 启动Grape MCP DevTools集成测试套件");
    
    let mut test_suite = IntegrationTestSuite::new();
    
    // 运行所有集成测试
    test_suite.run_test("MCP服务器基础功能", || async {
        test_mcp_server_basic().await
    }).await?;
    
    test_suite.run_test("动态工具注册集成", || async {
        test_dynamic_registry_integration().await
    }).await?;
    
    test_suite.run_test("版本检查工具集成", || async {
        test_version_check_integration().await
    }).await?;
    
    test_suite.run_test("API文档工具集成", || async {
        test_api_docs_integration().await
    }).await?;
    
    test_suite.run_test("文档搜索工具集成", || async {
        test_search_docs_integration().await
    }).await?;
    
    test_suite.run_test("增强文档处理器集成", || async {
        test_enhanced_doc_processor_integration().await
    }).await?;
    
    test_suite.run_test("向量文档工具集成", || async {
        test_vector_docs_integration().await
    }).await?;
    
    test_suite.run_test("多工具协作工作流", || async {
        test_multi_tool_workflow().await
    }).await?;
    
    test_suite.run_test("性能和并发测试", || async {
        test_performance_concurrency().await
    }).await?;
    
    test_suite.run_test("错误恢复和容错", || async {
        test_error_recovery().await
    }).await?;
    
    // 生成并打印测试报告
    let report = test_suite.generate_report();
    println!("{}", report);
    
    // 检查是否所有测试都通过
    let all_passed = test_suite.test_results.iter().all(|r| r.success);
    if all_passed {
        info!("🎉 所有集成测试通过！");
        Ok(())
    } else {
        error!("💥 部分集成测试失败");
        std::process::exit(1);
    }
}

// 测试1: MCP服务器基础功能
async fn test_mcp_server_basic() -> Result<String> {
    info!("测试MCP服务器基础功能...");
    
    let server = MCPServer::new();
    
    // 测试工具列表
    let tools = server.list_tools().await?;
    
    Ok(format!("服务器状态正常，初始工具数: {}", tools.len()))
}

// 测试2: 动态工具注册集成
async fn test_dynamic_registry_integration() -> Result<String> {
    info!("测试动态工具注册集成...");
    
    let mut registry = DynamicToolRegistry::new();
    registry.set_policy(RegistrationPolicy::Adaptive { score_threshold: 0.3 });
    
    let report = registry.auto_register().await?;
    
    Ok(format!(
        "注册了 {} 个工具，失败 {} 个，耗时 {}ms",
        report.registered_tools.len(),
        report.failed_registrations.len(),
        report.registration_duration_ms
    ))
}

// 测试3: 版本检查工具集成
async fn test_version_check_integration() -> Result<String> {
    info!("测试版本检查工具集成...");
    
    let tool = CheckVersionTool::new();
    
    let params = json!({
        "type": "cargo",
        "name": "serde",
        "include_preview": false
    });
    
    let result = tool.execute(params).await?;
    
    Ok(format!("版本检查完成: {}", result))
}

// 测试4: API文档工具集成
async fn test_api_docs_integration() -> Result<String> {
    info!("测试API文档工具集成...");
    
    let tool = GetApiDocsTool::new();
    
    let params = json!({
        "language": "rust",
        "package": "tokio",
        "version": "*"
    });
    
    let result = tool.execute(params).await?;
    
    Ok(format!("API文档获取完成: {}", result.to_string().len()))
}

// 测试5: 文档搜索工具集成
async fn test_search_docs_integration() -> Result<String> {
    info!("测试文档搜索工具集成...");
    
    let processor = DocumentProcessor::new().await?;
    
    let result = processor.process_documentation_request(
        "rust",
        "tokio",
        Some("1.0"),
        "async runtime"
    ).await?;
    
    Ok(format!("文档搜索完成，结果长度: {}", result.len()))
}

// 测试6: 增强文档处理器集成
async fn test_enhanced_doc_processor_integration() -> Result<String> {
    info!("测试增强文档处理器集成...");
    
    let processor = EnhancedDocumentProcessor::new().await?;
    
    let language = "rust";
    let package = "serde";
    let version = "1.0";
    
    // 使用正确的方法名
    match processor.process_documentation_request_enhanced(language, package, Some(version), "serialization").await {
        Ok(result) => {
            let summary = if result.is_empty() {
                "无结果".to_string()
            } else {
                format!("处理了 {} 个结果", result.len())
            };
            Ok(format!("增强文档处理完成，结果: {}", summary))
        }
        Err(e) => {
            Err(anyhow::anyhow!("增强文档处理失败: {}", e))
        }
    }
}

// 测试7: 向量文档工具集成
async fn test_vector_docs_integration() -> Result<String> {
    info!("测试向量文档工具集成...");
    
    // 修复：处理Result类型
    let vector_tool = VectorDocsTool::new()?;
    
    // 添加文档
    let add_params = json!({
        "action": "add",
        "content": "这是一个测试文档",
        "metadata": {
            "title": "测试文档",
            "language": "rust"
        }
    });
    
    let _add_result = vector_tool.execute(add_params).await;
    
    // 搜索文档
    let search_params = json!({
        "action": "search",
        "query": "测试",
        "limit": 5
    });
    
    let search_result = vector_tool.execute(search_params).await?;
    
    Ok(format!("向量文档操作完成: {}", search_result))
}

// 测试8: 多工具协作工作流
async fn test_multi_tool_workflow() -> Result<String> {
    info!("测试多工具协作工作流...");
    
    // 1. 版本检查
    let version_tool = CheckVersionTool::new();
    let version_params = json!({
        "type": "cargo",
        "name": "tokio",
        "include_preview": false
    });
    let _version_result = version_tool.execute(version_params).await?;
    
    // 2. API文档获取
    let api_tool = GetApiDocsTool::new();
    let api_params = json!({
        "language": "rust",
        "package": "tokio",
        "version": "*"
    });
    let _api_result = api_tool.execute(api_params).await?;
    
    // 3. 文档处理
    let doc_processor = DocumentProcessor::new().await?;
    let _doc_result = doc_processor.process_documentation_request(
        "rust",
        "tokio",
        Some("1.0"),
        "async runtime"
    ).await?;
    
    Ok("多工具协作工作流完成".to_string())
}

// 测试9: 性能和并发测试
async fn test_performance_concurrency() -> Result<String> {
    info!("测试性能和并发...");
    
    let start = Instant::now();
    let mut handles = Vec::new();
    
    // 创建5个并发任务
    for i in 0..5 {
        let handle = tokio::spawn(async move {
            let tool = CheckVersionTool::new();
            let params = json!({
                "type": "cargo",
                "name": "serde",
                "include_preview": false
            });
            
            let start = Instant::now();
            let result: Result<serde_json::Value> = tool.execute(params).await;
            let duration = start.elapsed();
            
            (i, result, duration)
        });
        handles.push(handle);
    }
    
    // 等待所有任务完成
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await?);
    }
    
    let total_duration = start.elapsed();
    let successful_tasks = results.iter().filter(|(_, result, _)| result.is_ok()).count();
    
    Ok(format!(
        "并发测试完成: {}/{} 任务成功，总耗时: {:?}",
        successful_tasks, results.len(), total_duration
    ))
}

// 测试10: 错误恢复和容错
async fn test_error_recovery() -> Result<String> {
    info!("测试错误恢复和容错...");
    
    let tool = GetApiDocsTool::new();
    
    // 测试无效参数
    let invalid_params = json!({
        "language": "invalid_language",
        "package": "nonexistent_package"
    });
    
    match tool.execute(invalid_params).await {
        Ok(_) => return Err(anyhow::anyhow!("应该返回错误但却成功了")),
        Err(_) => {
            // 预期的错误，继续测试
        }
    }
    
    // 测试有效参数
    let valid_params = json!({
        "language": "rust",
        "package": "serde",
        "version": "*"
    });
    
    let _result = tool.execute(valid_params).await?;
    
    Ok("错误恢复和容错测试通过".to_string())
} 