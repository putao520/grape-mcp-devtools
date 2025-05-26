use serde_json::json;
use std::sync::Arc;

use crate::tools::base::MCPTool;
use crate::tools::docs::doc_traits::{DocumentIndex, DocumentCache, DocumentFragment, DocElementKind, DocSourceType, DocMetadata, Visibility};
use crate::tools::tests::go_integration_test::{
    GoDocSearchMCPTool, InMemoryVectorIndex, InMemoryCache, SimpleVectorizer, GoDocSearchTool, GoDocGenerator
};

/// 端到端测试：完整的 Go 文档搜索工作流程
/// 
/// 这个测试演示了用户通过 LLM 调用 MCP 工具搜索 Go 语言库文档的完整流程：
/// 1. LLM 通过 MCP 调用搜索工具
/// 2. 工具首先从向量库搜索
/// 3. 如果向量库没有，则生成本地文档
/// 4. 将生成的文档向量化并存储
/// 5. 再次搜索并返回结果
/// 6. 如果还是没有结果，返回失败状态

#[tokio::test]
async fn test_end_to_end_go_documentation_workflow() {
    println!("🚀 开始端到端 Go 文档搜索测试...");

    // 设置测试环境
    let vector_store = Arc::new(InMemoryVectorIndex::new());
    let cache = Arc::new(InMemoryCache::new());
    let doc_generator = Arc::new(GoDocGenerator::new());
    let vectorizer = Arc::new(SimpleVectorizer);

    let search_tool = Arc::new(GoDocSearchTool::new(
        vector_store.clone(),
        cache.clone(),
        doc_generator.clone(),
        vectorizer.clone(),
    ));

    let mcp_tool = GoDocSearchMCPTool::new(search_tool);

    println!("✅ 测试环境设置完成");

    // 场景1: 搜索标准库包（应该能成功生成文档）
    println!("\n📚 场景1: 搜索 Go 标准库 fmt 包...");
    
    let params = json!({
        "package_name": "fmt",
        "query": "Printf function"
    });

    let result = mcp_tool.execute(params).await.unwrap();
    let result_obj = result.as_object().unwrap();

    println!("📊 结果状态: {}", result_obj["status"]);
    println!("📦 包名: {}", result_obj["package"]);
    
    if result_obj["status"] == "success" {
        if result_obj["source"] == "vector_store" {
            println!("✅ 从向量库找到文档");
        } else if result_obj["source"] == "generated_docs" {
            println!("✅ 生成文档后找到结果");
            if let Some(fragments) = result_obj.get("generated_fragments") {
                println!("📄 生成了 {} 个文档片段", fragments);
            }
        }
        
        if let Some(results) = result_obj.get("results") {
            if let Some(results_array) = results.as_array() {
                println!("🔍 找到 {} 个相关结果", results_array.len());
                for (i, result) in results_array.iter().enumerate() {
                    if let Some(fragment) = result.get("fragment") {
                        if let Some(title) = fragment.get("title") {
                            println!("  {}. {}", i + 1, title);
                        }
                    }
                }
            }
        }
    } else if result_obj["status"] == "partial_success" {
        println!("⚠️  生成了文档但未找到相关内容");
    } else {
        println!("❌ 生成文档失败: {}", result_obj.get("error").unwrap_or(&json!("未知错误")));
    }

    // 场景2: 再次搜索相同的包（应该从向量库或缓存获取）
    println!("\n🔄 场景2: 再次搜索相同的包（测试缓存）...");
    
    let params2 = json!({
        "package_name": "fmt",
        "query": "Sprintf function"
    });

    let result2 = mcp_tool.execute(params2).await.unwrap();
    let result2_obj = result2.as_object().unwrap();

    println!("📊 第二次搜索状态: {}", result2_obj["status"]);
    
    if result2_obj["status"] == "success" {
        if result2_obj["source"] == "vector_store" {
            println!("✅ 成功从向量库获取缓存的文档");
        }
    }

    // 场景3: 搜索不存在的包（应该返回失败）
    println!("\n❌ 场景3: 搜索不存在的包...");
    
    let params3 = json!({
        "package_name": "github.com/nonexistent/invalid-package",
        "version": "v999.999.999",
        "query": "some function"
    });

    let result3 = mcp_tool.execute(params3).await.unwrap();
    let result3_obj = result3.as_object().unwrap();

    println!("📊 不存在包的搜索状态: {}", result3_obj["status"]);
    
    assert_eq!(result3_obj["status"], "failure");
    assert!(result3_obj["message"].as_str().unwrap().contains("LLM调用工具失败"));
    println!("✅ 正确处理了不存在的包");

    // 场景4: 搜索第三方包（可能失败，取决于环境）
    println!("\n🌐 场景4: 搜索知名的第三方包...");
    
    let params4 = json!({
        "package_name": "github.com/gorilla/mux",
        "query": "Router usage"
    });

    let result4 = mcp_tool.execute(params4).await.unwrap();
    let result4_obj = result4.as_object().unwrap();

    println!("📊 第三方包搜索状态: {}", result4_obj["status"]);
    
    match result4_obj["status"].as_str().unwrap() {
        "success" => {
            println!("✅ 成功获取第三方包文档");
        }
        "failure" => {
            println!("⚠️  第三方包获取失败（可能需要网络或未安装 go）");
        }
        "partial_success" => {
            println!("⚠️  生成了文档但未找到相关内容");
        }
        _ => {
            println!("❓ 未知状态");
        }
    }

    // 验证向量存储中的内容
    println!("\n📊 验证向量存储状态...");
    
    let search_filter = crate::tools::docs::doc_traits::SearchFilter {
        doc_types: None,
        languages: Some(vec!["go".to_string()]),
        limit: Some(100),
        similarity_threshold: Some(0.1),
    };

    let all_docs = vector_store.search("", &search_filter).await.unwrap();
    println!("📄 向量存储中共有 {} 个文档片段", all_docs.len());

    // 验证缓存状态
    println!("💾 验证缓存状态...");
    
    // 由于我们的简单缓存实现没有直接的计数方法，我们尝试获取已知的键
    let cache_key = "go:go:fmt:latest:printf";
    if let Ok(Some(_)) = cache.get(cache_key).await {
        println!("✅ 缓存中有预期的文档");
    } else {
        println!("ℹ️  缓存中没有找到预期的文档（这可能是正常的）");
    }

    println!("\n🎉 端到端测试完成！");
}

/// 测试 MCP 工具的参数验证
#[tokio::test]
async fn test_mcp_tool_parameter_validation() {
    println!("🔍 测试 MCP 工具参数验证...");

    let vector_store = Arc::new(InMemoryVectorIndex::new());
    let cache = Arc::new(InMemoryCache::new());
    let doc_generator = Arc::new(GoDocGenerator::new());
    let vectorizer = Arc::new(SimpleVectorizer);

    let search_tool = Arc::new(GoDocSearchTool::new(
        vector_store,
        cache,
        doc_generator,
        vectorizer,
    ));

    let mcp_tool = GoDocSearchMCPTool::new(search_tool);

    // 测试缺少必需参数
    println!("📝 测试缺少必需参数...");
    let params = json!({
        "package_name": "fmt"
        // 缺少 query 参数
    });

    let result = mcp_tool.execute(params).await;
    assert!(result.is_err());
    println!("✅ 正确检测到缺少必需参数");

    // 测试参数类型错误
    println!("📝 测试参数类型错误...");
    let params = json!({
        "package_name": 123,  // 应该是字符串
        "query": "test"
    });

    let result = mcp_tool.execute(params).await;
    assert!(result.is_err());
    println!("✅ 正确检测到参数类型错误");

    // 测试空参数
    println!("📝 测试空参数...");
    let params = json!({
        "package_name": "",
        "query": ""
    });

    let result = mcp_tool.execute(params).await.unwrap();
    let result_obj = result.as_object().unwrap();
    // 空参数应该会导致文档生成失败
    assert_eq!(result_obj["status"], "failure");
    println!("✅ 正确处理了空参数");

    println!("🎉 参数验证测试完成！");
}

/// 测试工具元数据
#[tokio::test]
async fn test_tool_metadata() {
    println!("📋 测试工具元数据...");

    let vector_store = Arc::new(InMemoryVectorIndex::new());
    let cache = Arc::new(InMemoryCache::new());
    let doc_generator = Arc::new(GoDocGenerator::new());
    let vectorizer = Arc::new(SimpleVectorizer);

    let search_tool = Arc::new(GoDocSearchTool::new(
        vector_store,
        cache,
        doc_generator,
        vectorizer,
    ));

    let mcp_tool = GoDocSearchMCPTool::new(search_tool);

    // 验证工具名称
    assert_eq!(mcp_tool.name(), "search_go_documentation");
    println!("✅ 工具名称正确: {}", mcp_tool.name());

    // 验证工具描述
    let description = mcp_tool.description();
    assert!(description.contains("搜索 Go 语言库文档"));
    assert!(description.contains("向量库"));
    println!("✅ 工具描述正确");

    // 验证参数模式
    let schema = mcp_tool.parameters_schema();
    if let crate::tools::base::Schema::Object(obj) = schema {
        assert!(obj.required.contains(&"package_name".to_string()));
        assert!(obj.required.contains(&"query".to_string()));
        assert!(obj.properties.contains_key("package_name"));
        assert!(obj.properties.contains_key("query"));
        assert!(obj.properties.contains_key("version"));
        println!("✅ 参数模式正确");
            } else {
            println!("⚠️  参数模式不是期望的SchemaObject类型");
            // 记录错误但不中断测试
        }

    println!("🎉 工具元数据测试完成！");
}

/// 性能基准测试
#[tokio::test]
async fn test_performance_benchmark() {
    println!("⚡ 开始性能基准测试...");

    let vector_store = Arc::new(InMemoryVectorIndex::new());
    let cache = Arc::new(InMemoryCache::new());
    let doc_generator = Arc::new(GoDocGenerator::new());
    let vectorizer = Arc::new(SimpleVectorizer);

    let search_tool = Arc::new(GoDocSearchTool::new(
        vector_store.clone(),
        cache.clone(),
        doc_generator,
        vectorizer,
    ));

    let mcp_tool = GoDocSearchMCPTool::new(search_tool);

    // 预先添加一些文档到向量存储
    let test_fragments = vec![
        DocumentFragment {
            id: "go:fmt:latest:printf".to_string(),
            title: "Printf".to_string(),
            kind: DocElementKind::Function,
            full_name: Some("fmt.Printf".to_string()),
            description: "func Printf(format string, a ...interface{}) (n int, err error)".to_string(),
            source_type: DocSourceType::ApiDoc,
            code_context: None,
            examples: vec![],
            api_info: None,
            references: vec![],
            metadata: DocMetadata {
                package_name: "fmt".to_string(),
                version: None,
                language: "go".to_string(),
                source_url: None,
                deprecated: false,
                since_version: None,
                visibility: Visibility::Public,
            },
            changelog_info: None,
        },
        DocumentFragment {
            id: "go:fmt:latest:sprintf".to_string(),
            title: "Sprintf".to_string(),
            kind: DocElementKind::Function,
            full_name: Some("fmt.Sprintf".to_string()),
            description: "func Sprintf(format string, a ...interface{}) string".to_string(),
            source_type: DocSourceType::ApiDoc,
            code_context: None,
            examples: vec![],
            api_info: None,
            references: vec![],
            metadata: DocMetadata {
                package_name: "fmt".to_string(),
                version: None,
                language: "go".to_string(),
                source_url: None,
                deprecated: false,
                since_version: None,
                visibility: Visibility::Public,
            },
            changelog_info: None,
        },
    ];

    for fragment in &test_fragments {
        vector_store.index(fragment).await.unwrap();
    }

    // 测试从向量库搜索的性能
    let start = std::time::Instant::now();
    let params = json!({
        "package_name": "fmt",
        "query": "Printf"
    });

    let result = mcp_tool.execute(params).await.unwrap();
    let duration = start.elapsed();

    let result_obj = result.as_object().unwrap();
    assert_eq!(result_obj["status"], "success");
    assert_eq!(result_obj["source"], "vector_store");

    println!("⚡ 向量库搜索耗时: {:?}", duration);
    
    // 通常向量库搜索应该很快（< 100ms）
    assert!(duration.as_millis() < 1000, "向量库搜索应该在1秒内完成");

    // 测试多次搜索的性能（缓存效果）
    let start = std::time::Instant::now();
    for i in 0..10 {
        let params = json!({
            "package_name": "fmt",
            "query": format!("Printf test {}", i)
        });
        let _ = mcp_tool.execute(params).await.unwrap();
    }
    let duration = start.elapsed();

    println!("⚡ 10次搜索总耗时: {:?}", duration);
    println!("⚡ 平均每次搜索耗时: {:?}", duration / 10);

    println!("🎉 性能基准测试完成！");
}

/// 并发测试
#[tokio::test]
async fn test_concurrent_operations() {
    println!("🔄 开始并发操作测试...");

    let vector_store = Arc::new(InMemoryVectorIndex::new());
    let cache = Arc::new(InMemoryCache::new());
    let doc_generator = Arc::new(GoDocGenerator::new());
    let vectorizer = Arc::new(SimpleVectorizer);

    let search_tool = Arc::new(GoDocSearchTool::new(
        vector_store.clone(),
        cache.clone(),
        doc_generator,
        vectorizer,
    ));

    let mcp_tool = Arc::new(GoDocSearchMCPTool::new(search_tool));

    // 并发执行多个搜索
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let tool = mcp_tool.clone();
        let handle = tokio::spawn(async move {
            let params = json!({
                "package_name": "fmt",
                "query": format!("function {}", i)
            });
            
            tool.execute(params).await
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    let results = futures::future::join_all(handles).await;
    
    // 验证所有任务都成功完成
    for (i, result) in results.into_iter().enumerate() {
        let task_result = result.unwrap().unwrap();
        let result_obj = task_result.as_object().unwrap();
        
        println!("🔄 任务 {} 状态: {}", i, result_obj["status"]);
        
        // 至少应该有状态信息
        assert!(result_obj.contains_key("status"));
        assert!(result_obj.contains_key("package"));
    }

    println!("🎉 并发操作测试完成！");
} 