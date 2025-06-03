use anyhow::Result;
use grape_mcp_devtools::tools::{
    api_docs::GetApiDocsTool,
    base::MCPTool,
};
use serde_json::json;
use tracing::{info, warn, Level};
use tracing_subscriber;
use tokio::time::{timeout, Duration};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 开始测试增强的API文档获取工具");

    // 创建API文档工具
    let api_docs_tool = GetApiDocsTool::new();

    // 测试各种语言的API文档获取
    test_rust_docs(&api_docs_tool).await?;
    test_python_docs(&api_docs_tool).await?;
    test_javascript_docs(&api_docs_tool).await?;
    test_java_docs(&api_docs_tool).await?;
    test_go_docs(&api_docs_tool).await?;
    
    // 测试缓存功能
    test_cache_functionality(&api_docs_tool).await?;
    
    // 测试错误处理
    test_error_handling(&api_docs_tool).await?;

    info!("✅ 所有API文档工具测试完成");
    Ok(())
}

async fn test_rust_docs(tool: &GetApiDocsTool) -> Result<()> {
    info!("🦀 测试Rust文档获取");
    
    let test_cases = vec![
        ("tokio", None, "异步运行时"),
        ("serde", None, "序列化库"),
        ("clap", Some("4.0.0"), "命令行解析"),
    ];

    for (package, version, description) in test_cases {
        info!("  测试包: {} ({})", package, description);
        
        let params = json!({
            "language": "rust",
            "package": package,
            "symbol": "*",
            "version": version
        });

        let start_time = Instant::now();
        match timeout(Duration::from_secs(20), tool.execute(params)).await {
            Ok(Ok(result)) => {
                let duration = start_time.elapsed();
                info!("    ✅ 成功，耗时: {:?}", duration);
                
                // 验证结果结构
                assert_eq!(result["language"], "rust");
                assert_eq!(result["package"], package);
                assert_eq!(result["status"], "success");
                assert!(result["documentation"].is_object());
                assert!(result["links"].is_object());
                assert!(result["metadata"].is_object());
                
                // 验证关键链接
                let links = &result["links"];
                assert!(links["docs_rs"].as_str().unwrap().contains("docs.rs"));
                assert!(links["crates_io"].as_str().unwrap().contains("crates.io"));
                
                // 显示一些关键信息
                if let Some(metadata) = result["metadata"].as_object() {
                    info!("      描述: {}", metadata.get("description").and_then(|v| v.as_str()).unwrap_or("N/A"));
                    info!("      最新版本: {}", metadata.get("max_stable_version").and_then(|v| v.as_str()).unwrap_or("N/A"));
                    info!("      下载量: {}", metadata.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0));
                }
            }
            Ok(Err(e)) => {
                warn!("    ❌ 失败: {}", e);
            }
            Err(_) => {
                warn!("    ⏰ 超时");
            }
        }
    }

    Ok(())
}

async fn test_python_docs(tool: &GetApiDocsTool) -> Result<()> {
    info!("🐍 测试Python文档获取");
    
    let test_cases = vec![
        ("requests", None, "HTTP库"),
        ("django", None, "Web框架"),
        ("numpy", Some("1.24.0"), "数值计算"),
    ];

    for (package, version, description) in test_cases {
        info!("  测试包: {} ({})", package, description);
        
        let params = json!({
            "language": "python",
            "package": package,
            "symbol": "*",
            "version": version
        });

        let start_time = Instant::now();
        match timeout(Duration::from_secs(20), tool.execute(params)).await {
            Ok(Ok(result)) => {
                let duration = start_time.elapsed();
                info!("    ✅ 成功，耗时: {:?}", duration);
                
                // 验证结果结构
                assert_eq!(result["language"], "python");
                assert_eq!(result["package"], package);
                assert_eq!(result["status"], "success");
                
                // 验证关键链接
                let links = &result["links"];
                assert!(links["pypi"].as_str().unwrap().contains("pypi.org"));
                
                // 显示一些关键信息
                if let Some(docs) = result["documentation"].as_object() {
                    info!("      摘要: {}", docs.get("summary").and_then(|v| v.as_str()).unwrap_or("N/A"));
                }
                if let Some(metadata) = result["metadata"].as_object() {
                    info!("      作者: {}", metadata.get("author").and_then(|v| v.as_str()).unwrap_or("N/A"));
                    info!("      许可证: {}", metadata.get("license").and_then(|v| v.as_str()).unwrap_or("N/A"));
                }
            }
            Ok(Err(e)) => {
                warn!("    ❌ 失败: {}", e);
            }
            Err(_) => {
                warn!("    ⏰ 超时");
            }
        }
    }

    Ok(())
}

async fn test_javascript_docs(tool: &GetApiDocsTool) -> Result<()> {
    info!("📦 测试JavaScript文档获取");
    
    let test_cases = vec![
        ("express", None, "Web框架"),
        ("lodash", None, "工具库"),
        ("react", Some("18.0.0"), "UI库"),
    ];

    for (package, version, description) in test_cases {
        info!("  测试包: {} ({})", package, description);
        
        let params = json!({
            "language": "javascript",
            "package": package,
            "symbol": "*",
            "version": version
        });

        let start_time = Instant::now();
        match timeout(Duration::from_secs(20), tool.execute(params)).await {
            Ok(Ok(result)) => {
                let duration = start_time.elapsed();
                info!("    ✅ 成功，耗时: {:?}", duration);
                
                // 验证结果结构
                assert_eq!(result["language"], "javascript");
                assert_eq!(result["package"], package);
                assert_eq!(result["status"], "success");
                
                // 验证关键链接
                let links = &result["links"];
                assert!(links["npm"].as_str().unwrap().contains("npmjs.com"));
                
                // 显示一些关键信息
                if let Some(docs) = result["documentation"].as_object() {
                    info!("      描述: {}", docs.get("description").and_then(|v| v.as_str()).unwrap_or("N/A"));
                }
                if let Some(metadata) = result["metadata"].as_object() {
                    info!("      主文件: {}", metadata.get("main").and_then(|v| v.as_str()).unwrap_or("N/A"));
                    info!("      许可证: {}", metadata.get("license").and_then(|v| v.as_str()).unwrap_or("N/A"));
                }
            }
            Ok(Err(e)) => {
                warn!("    ❌ 失败: {}", e);
            }
            Err(_) => {
                warn!("    ⏰ 超时");
            }
        }
    }

    Ok(())
}

async fn test_java_docs(tool: &GetApiDocsTool) -> Result<()> {
    info!("☕ 测试Java文档获取");
    
    let test_cases = vec![
        ("com.google.guava:guava", None, "Google工具库"),
        ("org.apache.commons:commons-lang3", None, "Apache工具库"),
        ("junit:junit", Some("4.13.2"), "测试框架"),
    ];

    for (package, version, description) in test_cases {
        info!("  测试包: {} ({})", package, description);
        
        let params = json!({
            "language": "java",
            "package": package,
            "symbol": "*",
            "version": version
        });

        let start_time = Instant::now();
        match timeout(Duration::from_secs(20), tool.execute(params)).await {
            Ok(Ok(result)) => {
                let duration = start_time.elapsed();
                info!("    ✅ 成功，耗时: {:?}", duration);
                
                // 验证结果结构
                assert_eq!(result["language"], "java");
                assert_eq!(result["package"], package);
                assert_eq!(result["status"], "success");
                
                // 验证关键链接
                let links = &result["links"];
                assert!(links["maven_central"].as_str().unwrap().contains("search.maven.org"));
                assert!(links["javadoc"].as_str().unwrap().contains("javadoc.io"));
                
                // 显示一些关键信息
                if let Some(docs) = result["documentation"].as_object() {
                    info!("      Group ID: {}", docs.get("group_id").and_then(|v| v.as_str()).unwrap_or("N/A"));
                    info!("      Artifact ID: {}", docs.get("artifact_id").and_then(|v| v.as_str()).unwrap_or("N/A"));
                }
                if let Some(metadata) = result["metadata"].as_object() {
                    info!("      最新版本: {}", metadata.get("latest_version").and_then(|v| v.as_str()).unwrap_or("N/A"));
                    info!("      版本数: {}", metadata.get("version_count").and_then(|v| v.as_u64()).unwrap_or(0));
                }
            }
            Ok(Err(e)) => {
                warn!("    ❌ 失败: {}", e);
            }
            Err(_) => {
                warn!("    ⏰ 超时");
            }
        }
    }

    Ok(())
}

async fn test_go_docs(tool: &GetApiDocsTool) -> Result<()> {
    info!("🐹 测试Go文档获取");
    
    let test_cases: Vec<(&str, Option<&str>, &str)> = vec![
        ("github.com/gin-gonic/gin", None, "Web框架"),
        ("fmt", None, "标准库"),
        ("net/http", None, "HTTP库"),
    ];

    for (package, version, description) in test_cases {
        info!("  测试包: {} ({})", package, description);
        
        let params = json!({
            "language": "go",
            "package": package,
            "symbol": "*",
            "version": version
        });

        let start_time = Instant::now();
        match timeout(Duration::from_secs(20), tool.execute(params)).await {
            Ok(Ok(result)) => {
                let duration = start_time.elapsed();
                info!("    ✅ 成功，耗时: {:?}", duration);
                
                // 验证结果结构
                assert_eq!(result["language"], "go");
                assert_eq!(result["package"], package);
                assert_eq!(result["status"], "success");
                
                // 验证关键链接
                let links = &result["links"];
                assert!(links["pkg_go_dev"].as_str().unwrap().contains("pkg.go.dev"));
                
                // 显示一些关键信息
                if let Some(metadata) = result["metadata"].as_object() {
                    info!("      导入路径: {}", metadata.get("import_path").and_then(|v| v.as_str()).unwrap_or("N/A"));
                }
            }
            Ok(Err(e)) => {
                warn!("    ❌ 失败: {}", e);
            }
            Err(_) => {
                warn!("    ⏰ 超时");
            }
        }
    }

    Ok(())
}

async fn test_cache_functionality(tool: &GetApiDocsTool) -> Result<()> {
    info!("🎯 测试缓存功能");
    
    // 获取缓存统计
    let stats_before = tool.cache_stats().await;
    info!("  缓存使用前: {:?}", stats_before);
    
    // 执行相同的请求两次
    let params = json!({
        "language": "rust",
        "package": "tokio",
        "symbol": "*"
    });
    
    // 第一次请求
    let start1 = Instant::now();
    let result1 = tool.execute(params.clone()).await?;
    let duration1 = start1.elapsed();
    info!("  第一次请求耗时: {:?}", duration1);
    
    // 第二次请求（应该从缓存返回）
    let start2 = Instant::now();
    let result2 = tool.execute(params).await?;
    let duration2 = start2.elapsed();
    info!("  第二次请求耗时: {:?}", duration2);
    
    // 获取缓存统计
    let stats_after = tool.cache_stats().await;
    info!("  缓存使用后: {:?}", stats_after);
    
    // 验证缓存效果
    assert_eq!(result1, result2, "两次请求结果应该相同");
    assert!(duration2 < duration1, "第二次请求应该更快（缓存命中）");
    
    // 清理缓存
    tool.cleanup_cache().await;
    let stats_cleaned = tool.cache_stats().await;
    info!("  缓存清理后: {:?}", stats_cleaned);
    
    info!("  ✅ 缓存功能正常");
    Ok(())
}

async fn test_error_handling(tool: &GetApiDocsTool) -> Result<()> {
    info!("❌ 测试错误处理");
    
    // 测试不支持的语言
    let invalid_lang_params = json!({
        "language": "invalid_language",
        "package": "some_package",
        "symbol": "*"
    });
    
    match tool.execute(invalid_lang_params).await {
        Ok(_) => {
            tracing::error!("应该返回错误：不支持的语言");
            assert!(false, "应该返回错误：不支持的语言");
        }
        Err(e) => {
            info!("  ✅ 正确处理不支持的语言错误: {}", e);
        }
    }
    
    // 测试不存在的包
    let nonexistent_package_params = json!({
        "language": "rust",
        "package": "definitely_nonexistent_package_12345",
        "symbol": "*"
    });
    
    match timeout(Duration::from_secs(10), tool.execute(nonexistent_package_params)).await {
        Ok(Ok(_)) => warn!("  ⚠️ 未返回预期的错误：包不存在"),
        Ok(Err(e)) => {
            info!("  ✅ 正确处理包不存在错误: {}", e);
        }
        Err(_) => {
            warn!("  ⏰ 超时处理包不存在的情况");
        }
    }
    
    // 测试缺失必需参数
    let missing_params = json!({
        "language": "rust"
        // 缺少 package 参数
    });
    
    match tool.execute(missing_params).await {
        Ok(_) => {
            tracing::error!("应该返回错误：缺失必需参数");
            assert!(false, "应该返回错误：缺失必需参数");
        }
        Err(e) => {
            info!("  ✅ 正确处理缺失参数错误: {}", e);
        }
    }
    
    info!("  ✅ 错误处理功能正常");
    Ok(())
} 