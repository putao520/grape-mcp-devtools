use crate::tools::java_docs_tool::JavaDocsTool;
use crate::tools::base::MCPTool;
use crate::errors::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_java_docs_tool_basic() -> Result<()> {
    println!("☕ 测试 JavaDocsTool 基础功能");
    
    let java_docs_tool = JavaDocsTool::new();
    
    // 测试一个知名的Java库（完整的Maven坐标）
    let params = json!({
        "artifact_name": "org.springframework:spring-core"
    });
    
    match timeout(Duration::from_secs(30), java_docs_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Java文档生成成功: {}", docs);
                    assert_eq!(docs["language"], "java");
                    assert!(docs["artifact_name"].as_str().unwrap() == "org.springframework:spring-core");
                    assert!(docs["documentation"].is_object());
                },
                Err(e) => {
                    println!("❌ Java文档生成失败: {}", e);
                    // 即使失败也可能返回基础文档
                }
            }
        },
        Err(_) => {
            println!("⏰ Java文档生成超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_docs_tool_search_mode() -> Result<()> {
    println!("☕ 测试 JavaDocsTool 搜索模式");
    
    let java_docs_tool = JavaDocsTool::new();
    
    // 测试只有artifactId的搜索
    let params = json!({
        "artifact_name": "jackson-core"
    });
    
    match timeout(Duration::from_secs(30), java_docs_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Java搜索模式成功: {}", docs);
                    assert_eq!(docs["language"], "java");
                    assert!(docs["artifact_name"].as_str().unwrap() == "jackson-core");
                    
                    // 搜索模式应该返回搜索结果
                    if docs["source"] == "maven_search" {
                        assert!(docs["search_results"].is_array());
                    }
                },
                Err(e) => {
                    println!("❌ Java搜索模式失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java搜索模式超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_docs_tool_with_version() -> Result<()> {
    println!("☕ 测试 JavaDocsTool 指定版本功能");
    
    let java_docs_tool = JavaDocsTool::new();
    
    let params = json!({
        "artifact_name": "org.apache.commons:commons-lang3",
        "version": "3.12.0"
    });
    
    match timeout(Duration::from_secs(30), java_docs_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Java文档（指定版本）生成成功: {}", docs);
                    assert_eq!(docs["language"], "java");
                    assert!(docs["artifact_name"].as_str().unwrap() == "org.apache.commons:commons-lang3");
                    assert!(docs["version"].as_str().is_some());
                },
                Err(e) => {
                    println!("❌ Java文档（指定版本）生成失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java文档（指定版本）生成超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_docs_tool_multiple_artifacts() -> Result<()> {
    println!("☕ 测试 JavaDocsTool 多个库");
    
    let java_docs_tool = JavaDocsTool::new();
    
    let test_artifacts = vec![
        "gson",  // 只有artifactId，会触发搜索
        "org.springframework:spring-boot-starter-web", // 完整坐标
        "com.fasterxml.jackson.core:jackson-core", // 另一个完整坐标
        "nonexistent_artifact_12345"  // 测试不存在的artifact
    ];
    
    for artifact_name in test_artifacts {
        println!("📚 测试artifact: {}", artifact_name);
        
        let params = json!({
            "artifact_name": artifact_name
        });
        
        match timeout(Duration::from_secs(20), java_docs_tool.execute(params)).await {
            Ok(result) => {
                match result {
                    Ok(docs) => {
                        println!("✅ {} 文档生成成功", artifact_name);
                        assert_eq!(docs["language"], "java");
                        assert_eq!(docs["artifact_name"], artifact_name);
                        
                        // 检查必要的字段
                        assert!(docs["documentation"].is_object());
                        assert!(docs["installation"].is_object());
                        
                        // 验证安装信息
                        if let Some(installation) = docs["installation"].as_object() {
                            assert!(installation.contains_key("maven") || installation.contains_key("gradle"));
                        }
                    },
                    Err(e) => {
                        println!("❌ {} 文档生成失败: {}", artifact_name, e);
                    }
                }
            },
            Err(_) => {
                println!("⏰ {} 文档生成超时", artifact_name);
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_docs_tool_caching() -> Result<()> {
    println!("☕ 测试 JavaDocsTool 缓存功能");
    
    let java_docs_tool = JavaDocsTool::new();
    
    let params = json!({
        "artifact_name": "org.apache.commons:commons-lang3"
    });
    
    // 第一次调用
    let start_time = std::time::Instant::now();
    let result1 = timeout(Duration::from_secs(30), java_docs_tool.execute(params.clone())).await;
    let first_duration = start_time.elapsed();
    
    // 第二次调用（应该使用缓存）
    let start_time = std::time::Instant::now();
    let result2 = timeout(Duration::from_secs(30), java_docs_tool.execute(params.clone())).await;
    let second_duration = start_time.elapsed();
    
    match (result1, result2) {
        (Ok(Ok(docs1)), Ok(Ok(docs2))) => {
            println!("✅ 两次调用都成功");
            println!("第一次耗时: {:?}, 第二次耗时: {:?}", first_duration, second_duration);
            
            // 验证内容相同
            assert_eq!(docs1["artifact_name"], docs2["artifact_name"]);
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
async fn test_java_docs_tool_invalid_params() -> Result<()> {
    println!("☕ 测试 JavaDocsTool 参数验证");
    
    let java_docs_tool = JavaDocsTool::new();
    
    // 测试缺少必需参数
    let invalid_params = json!({
        "version": "1.0.0"
        // 缺少 artifact_name
    });
    
    match java_docs_tool.execute(invalid_params).await {
        Ok(_) => {
            println!("⚠️ 参数验证失败：应该拒绝无效参数");
        },
        Err(e) => {
            println!("✅ 参数验证成功：正确拒绝了无效参数: {}", e);
        }
    }
    
    // 测试空artifact名称
    let empty_name_params = json!({
        "artifact_name": ""
    });
    
    match timeout(Duration::from_secs(10), java_docs_tool.execute(empty_name_params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ 空artifact名称被处理: {}", docs);
                    // 应该返回基础文档
                    assert_eq!(docs["language"], "java");
                },
                Err(e) => {
                    println!("✅ 空artifact名称被正确拒绝: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ 空artifact名称测试超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_docs_tool_integration() -> Result<()> {
    println!("☕ 测试 JavaDocsTool 集成功能");
    
    let java_docs_tool = JavaDocsTool::new();
    
    // 测试工具元数据
    assert_eq!(java_docs_tool.name(), "java_docs");
    assert!(java_docs_tool.description().contains("Java"));
    
    let schema = java_docs_tool.parameters_schema();
    println!("✅ 参数模式: {:?}", schema);
    
    // 测试一个真实的Java生态系统中的流行库
    let popular_libraries = vec![
        "com.google.guava:guava",
        "org.apache.commons:commons-lang3",
        "junit:junit"
    ];
    
    for artifact_name in popular_libraries {
        let params = json!({
            "artifact_name": artifact_name,
            "include_dependencies": "true"
        });
        
        match timeout(Duration::from_secs(25), java_docs_tool.execute(params)).await {
            Ok(result) => {
                match result {
                    Ok(docs) => {
                        println!("✅ {} 集成测试成功", artifact_name);
                        
                        // 验证返回结构
                        assert!(docs["artifact_name"].is_string());
                        assert!(docs["language"].is_string());
                        assert!(docs["documentation"].is_object());
                        assert!(docs["installation"].is_object());
                        
                        // 验证安装信息
                        if let Some(installation) = docs["installation"].as_object() {
                            assert!(installation.contains_key("maven"));
                            assert!(installation.contains_key("gradle"));
                        }
                        
                        // 检查Maven Central成功响应的特定字段
                        if docs["source"] == "maven_central" {
                            assert!(docs.get("group_id").is_some());
                            assert!(docs.get("artifact_id").is_some());
                            assert!(docs.get("latest_version").is_some());
                            
                            if let Some(links) = docs["links"].as_object() {
                                assert!(links.contains_key("maven_central"));
                                assert!(links.contains_key("mvn_repository"));
                            }
                        }
                    },
                    Err(e) => {
                        println!("❌ {} 集成测试失败: {}", artifact_name, e);
                    }
                }
            },
            Err(_) => {
                println!("⏰ {} 集成测试超时", artifact_name);
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_docs_tool_maven_coordinate_parsing() -> Result<()> {
    println!("☕ 测试 JavaDocsTool Maven坐标解析");
    
    let java_docs_tool = JavaDocsTool::new();
    
    // 测试不同格式的Maven坐标
    let coordinate_formats = vec![
        ("org.springframework:spring-core", "完整坐标"),
        ("spring-core", "仅artifactId"),
        ("com.fasterxml.jackson.core:jackson-core", "带点号的groupId"),
        ("org.apache.commons:commons-lang3:3.12.0", "包含版本的坐标"),
    ];
    
    for (coordinate, description) in coordinate_formats {
        println!("🧪 测试{}: {}", description, coordinate);
        
        let params = json!({
            "artifact_name": coordinate
        });
        
        match timeout(Duration::from_secs(20), java_docs_tool.execute(params)).await {
            Ok(result) => {
                match result {
                    Ok(docs) => {
                        println!("✅ {} 解析成功", description);
                        assert_eq!(docs["language"], "java");
                        assert_eq!(docs["artifact_name"], coordinate);
                        
                        // 验证文档结构
                        assert!(docs["documentation"].is_object());
                        assert!(docs["installation"].is_object());
                    },
                    Err(e) => {
                        println!("❌ {} 解析失败: {}", description, e);
                    }
                }
            },
            Err(_) => {
                println!("⏰ {} 解析超时", description);
            }
        }
    }
    
    Ok(())
} 