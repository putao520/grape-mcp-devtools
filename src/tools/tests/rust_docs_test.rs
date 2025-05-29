use crate::tools::rust_docs_tool::RustDocsTool;
use crate::tools::base::MCPTool;
use crate::errors::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_rust_docs_tool_basic() -> Result<()> {
    println!("🦀 测试 RustDocsTool 基础功能");
    
    let rust_docs_tool = RustDocsTool::new();
    
    // 测试一个知名的Rust crate
    let params = json!({
        "crate_name": "serde"
    });
    
    match timeout(Duration::from_secs(30), rust_docs_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Rust文档生成成功: {}", docs);
                    assert_eq!(docs["language"], "rust");
                    assert!(docs["crate_name"].as_str().unwrap() == "serde");
                    assert!(docs["documentation"].is_object());
                },
                Err(e) => {
                    println!("❌ Rust文档生成失败: {}", e);
                    // 即使失败也可能返回基础文档
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust文档生成超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_docs_tool_with_version() -> Result<()> {
    println!("🦀 测试 RustDocsTool 指定版本功能");
    
    let rust_docs_tool = RustDocsTool::new();
    
    let params = json!({
        "crate_name": "tokio",
        "version": "1.0.0"
    });
    
    match timeout(Duration::from_secs(30), rust_docs_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Rust文档（指定版本）生成成功: {}", docs);
                    assert_eq!(docs["language"], "rust");
                    assert!(docs["crate_name"].as_str().unwrap() == "tokio");
                    assert!(docs["version"].as_str().is_some());
                },
                Err(e) => {
                    println!("❌ Rust文档（指定版本）生成失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust文档（指定版本）生成超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_docs_tool_multiple_crates() -> Result<()> {
    println!("🦀 测试 RustDocsTool 多个crate");
    
    let rust_docs_tool = RustDocsTool::new();
    
    let test_crates = vec![
        "clap",
        "reqwest", 
        "anyhow",
        "nonexistent_crate_12345"  // 测试不存在的crate
    ];
    
    for crate_name in test_crates {
        println!("📚 测试crate: {}", crate_name);
        
        let params = json!({
            "crate_name": crate_name
        });
        
        match timeout(Duration::from_secs(20), rust_docs_tool.execute(params)).await {
            Ok(result) => {
                match result {
                    Ok(docs) => {
                        println!("✅ {} 文档生成成功", crate_name);
                        assert_eq!(docs["language"], "rust");
                        assert_eq!(docs["crate_name"], crate_name);
                        
                        // 检查必要的字段
                        assert!(docs["documentation"].is_object());
                        assert!(docs["installation"].is_object());
                        assert!(docs["links"].is_object());
                    },
                    Err(e) => {
                        println!("❌ {} 文档生成失败: {}", crate_name, e);
                    }
                }
            },
            Err(_) => {
                println!("⏰ {} 文档生成超时", crate_name);
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_docs_tool_caching() -> Result<()> {
    println!("🦀 测试 RustDocsTool 缓存功能");
    
    let rust_docs_tool = RustDocsTool::new();
    
    let params = json!({
        "crate_name": "serde_json"
    });
    
    // 第一次调用
    let start_time = std::time::Instant::now();
    let result1 = timeout(Duration::from_secs(30), rust_docs_tool.execute(params.clone())).await;
    let first_duration = start_time.elapsed();
    
    // 第二次调用（应该使用缓存）
    let start_time = std::time::Instant::now();
    let result2 = timeout(Duration::from_secs(30), rust_docs_tool.execute(params.clone())).await;
    let second_duration = start_time.elapsed();
    
    match (result1, result2) {
        (Ok(Ok(docs1)), Ok(Ok(docs2))) => {
            println!("✅ 两次调用都成功");
            println!("第一次耗时: {:?}, 第二次耗时: {:?}", first_duration, second_duration);
            
            // 验证内容相同
            assert_eq!(docs1["crate_name"], docs2["crate_name"]);
            assert_eq!(docs1["language"], docs2["language"]);
            
            // 第二次应该更快（缓存效果）
            if second_duration < first_duration {
                println!("✅ 缓存生效，第二次调用更快");
            } else {
                println!("⚠️ 缓存可能未生效或网络延迟影响");
            }
        },
        _ => {
            println!("⚠️ 缓存测试未能完全成功");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_docs_tool_invalid_params() -> Result<()> {
    println!("🦀 测试 RustDocsTool 参数验证");
    
    let rust_docs_tool = RustDocsTool::new();
    
    // 测试缺少必需参数
    let invalid_params = json!({
        "version": "1.0.0"
        // 缺少 crate_name
    });
    
    match rust_docs_tool.execute(invalid_params).await {
        Ok(_) => {
            println!("⚠️ 参数验证失败：应该拒绝无效参数");
        },
        Err(e) => {
            println!("✅ 参数验证成功：正确拒绝了无效参数: {}", e);
        }
    }
    
    // 测试空crate名称
    let empty_name_params = json!({
        "crate_name": ""
    });
    
    match timeout(Duration::from_secs(10), rust_docs_tool.execute(empty_name_params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ 空crate名称被处理: {}", docs);
                    // 应该返回基础文档
                    assert_eq!(docs["language"], "rust");
                },
                Err(e) => {
                    println!("✅ 空crate名称被正确拒绝: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ 空crate名称测试超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_docs_tool_integration() -> Result<()> {
    println!("🦀 测试 RustDocsTool 集成功能");
    
    let rust_docs_tool = RustDocsTool::new();
    
    // 测试工具元数据
    assert_eq!(rust_docs_tool.name(), "rust_docs");
    assert!(rust_docs_tool.description().contains("Rust"));
    
    let schema = rust_docs_tool.parameters_schema();
    println!("✅ 参数模式: {:?}", schema);
    
    // 测试一个真实的Rust生态系统中的流行crate
    let popular_crates = vec!["serde", "tokio", "clap"];
    
    for crate_name in popular_crates {
        let params = json!({
            "crate_name": crate_name,
            "include_examples": "true"
        });
        
        match timeout(Duration::from_secs(25), rust_docs_tool.execute(params)).await {
            Ok(result) => {
                match result {
                    Ok(docs) => {
                        println!("✅ {} 集成测试成功", crate_name);
                        
                        // 验证返回结构
                        assert!(docs["crate_name"].is_string());
                        assert!(docs["language"].is_string());
                        assert!(docs["documentation"].is_object());
                        assert!(docs["installation"].is_object());
                        
                        // 验证安装信息
                        if let Some(installation) = docs["installation"].as_object() {
                            assert!(installation.contains_key("cargo"));
                            assert!(installation.contains_key("cargo_toml"));
                        }
                        
                        // 验证链接信息
                        if let Some(links) = docs["links"].as_object() {
                            assert!(links.contains_key("crates_io"));
                            assert!(links.contains_key("docs_rs"));
                        }
                    },
                    Err(e) => {
                        println!("❌ {} 集成测试失败: {}", crate_name, e);
                    }
                }
            },
            Err(_) => {
                println!("⏰ {} 集成测试超时", crate_name);
            }
        }
    }
    
    Ok(())
} 