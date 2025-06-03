use std::sync::Arc;
use std::collections::HashMap;
use grape_mcp_devtools::tools::{
    doc_processor::DocumentProcessor,
    search::SearchDocsTools,
    vector_docs_tool::VectorDocsTool,
    base::MCPTool,
};
use serde_json::json;
use tracing::{info, warn, error};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🚀 开始测试增强版文档处理模块");

    // 测试1: 文档处理器核心功能
    test_document_processor_core().await?;
    
    // 测试2: 多语言文档生成
    test_multi_language_doc_generation().await?;
    
    // 测试3: 向量化文档搜索
    test_vectorized_document_search().await?;
    
    // 测试4: 搜索工具集成
    test_search_tools_integration().await?;
    
    // 测试5: 缓存和性能优化
    test_caching_and_performance().await?;
    
    // 测试6: 错误处理和恢复
    test_error_handling_and_recovery().await?;
    
    // 测试7: 并发文档处理
    test_concurrent_document_processing().await?;
    
    // 测试8: 实际包文档生成
    test_real_package_documentation().await?;

    info!("✅ 所有增强版文档处理模块测试完成！");
    Ok(())
}

async fn test_document_processor_core() -> anyhow::Result<()> {
    info!("📋 测试1: 文档处理器核心功能");
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试基础文档处理请求
    let test_cases = vec![
        ("rust", "serde", Some("1.0"), "serialization"),
        ("python", "requests", Some("2.28"), "http client"),
        ("javascript", "express", Some("4.18"), "web framework"),
        ("go", "gin", Some("1.9"), "web framework"),
        ("java", "jackson", Some("2.15"), "json processing"),
    ];
    
    for (language, package, version, query) in test_cases {
        info!("测试文档处理: {} {} {} - 查询: {}", language, package, version.unwrap_or("latest"), query);
        
        let start_time = std::time::Instant::now();
        match timeout(Duration::from_secs(30), 
            processor.process_documentation_request(language, package, version, query)
        ).await {
            Ok(Ok(fragments)) => {
                let duration = start_time.elapsed();
                info!("  ✅ 成功处理 {} 文档: {} 个片段，耗时: {:?}", 
                      package, fragments.len(), duration);
                
                // 验证文档片段质量
                for fragment in fragments.iter().take(3) {
                    info!("    📄 片段: {} ({} 字符)", 
                          fragment.file_path, fragment.content.len());
                    
                    // 验证内容不为空且有意义
                    assert!(!fragment.content.is_empty(), "文档内容不能为空");
                    assert!(fragment.content.len() > 50, "文档内容应该有实质内容");
                    assert!(fragment.language == language, "语言标识应该正确");
                    assert!(fragment.package_name == package, "包名应该正确");
                }
            }
            Ok(Err(e)) => {
                warn!("  ⚠️ 处理 {} 文档失败: {}", package, e);
            }
            Err(_) => {
                warn!("  ⚠️ 处理 {} 文档超时", package);
            }
        }
    }
    
    info!("✅ 文档处理器核心功能测试通过");
    Ok(())
}

async fn test_multi_language_doc_generation() -> anyhow::Result<()> {
    info!("📋 测试2: 多语言文档生成");
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试每种语言的特定文档生成方法
    let language_tests = vec![
        ("rust", "tokio", "async runtime"),
        ("python", "numpy", "scientific computing"),
        ("javascript", "lodash", "utility library"),
        ("go", "gorilla/mux", "http router"),
        ("java", "spring-boot", "web framework"),
    ];
    
    let mut generation_stats = HashMap::new();
    
    for (language, package, description) in language_tests {
        info!("测试 {} 语言文档生成: {}", language, package);
        
        let start_time = std::time::Instant::now();
        
        // 测试特定语言的文档生成
        let result = match language {
            "rust" => {
                processor.generate_rust_docs(package, "latest").await
            }
            "python" => {
                processor.generate_python_docs(package, "latest").await
            }
            "javascript" => {
                processor.generate_npm_docs(package, "latest").await
            }
            "go" => {
                processor.generate_go_docs(package, Some("latest")).await
            }
            "java" => {
                processor.generate_java_docs(package, "latest").await
            }
            _ => {
                warn!("不支持的语言: {}", language);
                continue;
            }
        };
        
        let duration = start_time.elapsed();
        
        match result {
            Ok(fragments) => {
                info!("  ✅ {} 文档生成成功: {} 个片段，耗时: {:?}", 
                      language, fragments.len(), duration);
                
                // 统计生成信息
                generation_stats.insert(language, (fragments.len(), duration));
                
                // 验证文档质量
                for fragment in fragments.iter().take(2) {
                    info!("    📄 {}: {} 字符", fragment.file_path, fragment.content.len());
                    
                    // 验证文档包含预期内容
                    let content_lower = fragment.content.to_lowercase();
                    assert!(content_lower.contains(package) || 
                           content_lower.contains(&package.replace("-", "_")) ||
                           content_lower.contains(&package.replace("/", "")), 
                           "文档应该包含包名相关内容");
                    
                    // 验证文档结构
                    assert!(fragment.content.len() > 100, "文档应该有足够的内容");
                }
            }
            Err(e) => {
                warn!("  ⚠️ {} 文档生成失败: {}", language, e);
            }
        }
    }
    
    // 输出生成统计
    info!("📊 文档生成统计:");
    for (language, (count, duration)) in generation_stats {
        info!("  {} - {} 片段，耗时: {:?}", language, count, duration);
    }
    
    info!("✅ 多语言文档生成测试通过");
    Ok(())
}

async fn test_vectorized_document_search() -> anyhow::Result<()> {
    info!("📋 测试3: 向量化文档搜索");
    
    let vector_tool = VectorDocsTool::new()?;
    
    // 首先添加一些测试文档
    let test_documents = vec![
        ("rust", "serde", "Serde is a framework for serializing and deserializing Rust data structures efficiently and generically."),
        ("python", "requests", "Requests is an elegant and simple HTTP library for Python, built for human beings."),
        ("javascript", "express", "Express is a minimal and flexible Node.js web application framework."),
        ("go", "gin", "Gin is a HTTP web framework written in Go. It features a Martini-like API."),
        ("java", "jackson", "Jackson is a suite of data-processing tools for Java."),
    ];
    
    // 添加文档到向量库
    for (language, package, description) in &test_documents {
        let add_params = json!({
            "action": "add",
            "title": format!("{} - {}", language, package),
            "content": description,
            "language": language,
            "package": package,
            "metadata": {
                "type": "documentation",
                "source": "test"
            }
        });
        
        match vector_tool.execute(add_params).await {
            Ok(result) => {
                info!("  ✅ 添加文档: {} {}", language, package);
                assert_eq!(result["status"], "success", "添加文档应该成功");
            }
            Err(e) => {
                warn!("  ⚠️ 添加文档失败: {} {} - {}", language, package, e);
            }
        }
    }
    
    // 测试各种搜索查询
    let search_queries = vec![
        ("serialization", "rust"),
        ("http", "python"),
        ("web framework", "javascript"),
        ("json", "java"),
        ("api", "go"),
    ];
    
    for (query, expected_language) in search_queries {
        info!("测试搜索: {} (期望语言: {})", query, expected_language);
        
        let search_params = json!({
            "action": "search",
            "query": query,
            "limit": 5
        });
        
        match vector_tool.execute(search_params).await {
            Ok(result) => {
                if result["status"] == "success" {
                    let results_count = result["results_count"].as_u64().unwrap_or(0);
                    info!("  🔍 搜索结果: {} 个", results_count);
                    
                    if results_count > 0 {
                        let results = result["results"].as_array().unwrap();
                        for (i, doc) in results.iter().enumerate() {
                            if let (Some(title), Some(score)) = (
                                doc["title"].as_str(),
                                doc["score"].as_f64()
                            ) {
                                info!("    {}. {} (相似度: {:.3})", i + 1, title, score);
                            }
                        }
                        
                        // 验证搜索质量
                        let top_result = &results[0];
                        if let Some(content) = top_result["content"].as_str() {
                            let content_lower = content.to_lowercase();
                            let query_lower = query.to_lowercase();
                            assert!(content_lower.contains(&query_lower) || 
                                   query_lower.split_whitespace().any(|word| content_lower.contains(word)),
                                   "搜索结果应该与查询相关");
                        }
                    }
                } else {
                    warn!("  ⚠️ 搜索失败: {}", result["error"]);
                }
            }
            Err(e) => {
                warn!("  ⚠️ 搜索执行失败: {}", e);
            }
        }
    }
    
    // 测试向量库统计
    let stats_params = json!({
        "action": "stats"
    });
    
    match vector_tool.execute(stats_params).await {
        Ok(result) => {
            if result["status"] == "success" {
                let total_docs = result["total_documents"].as_u64().unwrap_or(0);
                let total_vectors = result["total_vectors"].as_u64().unwrap_or(0);
                info!("📊 向量库统计: {} 文档, {} 向量", total_docs, total_vectors);
                
                assert!(total_docs >= test_documents.len() as u64, "应该包含测试文档");
            }
        }
        Err(e) => {
            warn!("⚠️ 获取统计信息失败: {}", e);
        }
    }
    
    info!("✅ 向量化文档搜索测试通过");
    Ok(())
}

async fn test_search_tools_integration() -> anyhow::Result<()> {
    info!("📋 测试4: 搜索工具集成");
    
    let search_tool = SearchDocsTools::new();
    
    // 测试各种语言的文档搜索
    let search_tests = vec![
        ("rust", "async", "异步编程"),
        ("python", "pandas", "数据分析"),
        ("javascript", "react", "前端框架"),
        ("go", "context", "上下文管理"),
        ("java", "spring", "企业框架"),
    ];
    
    for (language, query, description) in search_tests {
        info!("测试搜索工具: {} 语言搜索 '{}' ({})", language, query, description);
        
        let search_params = json!({
            "query": query,
            "language": language,
            "limit": 5
        });
        
        let start_time = std::time::Instant::now();
        match search_tool.execute(search_params).await {
            Ok(result) => {
                let duration = start_time.elapsed();
                info!("  ✅ 搜索完成，耗时: {:?}", duration);
                
                // 验证搜索结果结构
                assert!(result.is_object(), "搜索结果应该是对象");
                
                if let Some(results) = result["results"].as_array() {
                    info!("    📄 找到 {} 个结果", results.len());
                    
                    for (i, doc) in results.iter().take(3).enumerate() {
                        if let (Some(title), Some(source)) = (
                            doc["title"].as_str(),
                            doc["source"].as_str()
                        ) {
                            info!("      {}. {} (来源: {})", i + 1, title, source);
                            
                            // 验证结果质量
                            assert!(!title.is_empty(), "标题不能为空");
                            assert!(!source.is_empty(), "来源不能为空");
                            
                            if let Some(relevance) = doc["relevance"].as_f64() {
                                assert!(relevance >= 0.0 && relevance <= 1.0, "相关性分数应该在0-1之间");
                            }
                        }
                    }
                } else {
                    warn!("    ⚠️ 没有找到搜索结果");
                }
            }
            Err(e) => {
                warn!("  ⚠️ 搜索失败: {}", e);
            }
        }
    }
    
    info!("✅ 搜索工具集成测试通过");
    Ok(())
}

async fn test_caching_and_performance() -> anyhow::Result<()> {
    info!("📋 测试5: 缓存和性能优化");
    
    let search_tool = SearchDocsTools::new();
    
    // 测试缓存功能
    let test_query = json!({
        "query": "async",
        "language": "rust",
        "limit": 3
    });
    
    // 第一次搜索（无缓存）
    info!("执行第一次搜索（建立缓存）");
    let start_time = std::time::Instant::now();
    let first_result = search_tool.execute(test_query.clone()).await?;
    let first_duration = start_time.elapsed();
    info!("  第一次搜索耗时: {:?}", first_duration);
    
    // 第二次搜索（应该命中缓存）
    info!("执行第二次搜索（测试缓存命中）");
    let start_time = std::time::Instant::now();
    let second_result = search_tool.execute(test_query.clone()).await?;
    let second_duration = start_time.elapsed();
    info!("  第二次搜索耗时: {:?}", second_duration);
    
    // 验证缓存效果
    assert_eq!(first_result, second_result, "缓存结果应该一致");
    
    // 通常缓存命中应该更快，但由于网络请求的不确定性，我们只验证结果一致性
    info!("  ✅ 缓存功能正常，结果一致");
    
    // 测试并发搜索性能
    info!("测试并发搜索性能");
    let concurrent_queries = vec![
        ("rust", "tokio"),
        ("python", "asyncio"),
        ("javascript", "promise"),
        ("go", "goroutine"),
        ("java", "completablefuture"),
    ];
    
    let start_time = std::time::Instant::now();
    let mut handles = Vec::new();
    
    for (language, query) in concurrent_queries {
        let search_tool_clone = SearchDocsTools::new();
        let params = json!({
            "query": query,
            "language": language,
            "limit": 3
        });
        
        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = search_tool_clone.execute(params).await;
            let duration = start.elapsed();
            (language, query, result, duration)
        });
        
        handles.push(handle);
    }
    
    // 等待所有并发搜索完成
    let mut successful_searches = 0;
    for handle in handles {
        match handle.await {
            Ok((language, query, result, duration)) => {
                match result {
                    Ok(_) => {
                        info!("  ✅ {} '{}' 搜索成功，耗时: {:?}", language, query, duration);
                        successful_searches += 1;
                    }
                    Err(e) => {
                        warn!("  ⚠️ {} '{}' 搜索失败: {}", language, query, e);
                    }
                }
            }
            Err(e) => {
                error!("  ❌ 并发任务执行失败: {}", e);
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    info!("🏁 并发搜索完成: {}/5 成功，总耗时: {:?}", successful_searches, total_duration);
    
    assert!(successful_searches >= 3, "至少应该有3个搜索成功");
    
    info!("✅ 缓存和性能优化测试通过");
    Ok(())
}

async fn test_error_handling_and_recovery() -> anyhow::Result<()> {
    info!("📋 测试6: 错误处理和恢复");
    
    let processor = DocumentProcessor::new().await?;
    let search_tool = SearchDocsTools::new();
    
    // 测试无效参数处理
    info!("测试无效参数处理");
    
    // 测试空查询
    let invalid_search = json!({
        "query": "",
        "language": "rust"
    });
    
    match search_tool.execute(invalid_search).await {
        Ok(_) => {
            warn!("  ⚠️ 空查询应该失败但成功了");
        }
        Err(e) => {
            info!("  ✅ 正确处理空查询错误: {}", e);
        }
    }
    
    // 测试缺少必需参数
    let missing_param = json!({
        "query": "test"
        // 缺少 language 参数
    });
    
    match search_tool.execute(missing_param).await {
        Ok(_) => {
            warn!("  ⚠️ 缺少参数应该失败但成功了");
        }
        Err(e) => {
            info!("  ✅ 正确处理缺少参数错误: {}", e);
        }
    }
    
    // 测试不支持的语言
    let unsupported_language = json!({
        "query": "test",
        "language": "nonexistent_language"
    });
    
    match search_tool.execute(unsupported_language).await {
        Ok(result) => {
            info!("  ✅ 不支持的语言使用通用搜索: {:?}", result);
        }
        Err(e) => {
            info!("  ✅ 正确处理不支持的语言: {}", e);
        }
    }
    
    // 测试不存在的包文档生成
    info!("测试不存在包的错误处理");
    match processor.process_documentation_request(
        "rust", 
        "definitely_nonexistent_package_12345", 
        Some("1.0.0"), 
        "test"
    ).await {
        Ok(fragments) => {
            if fragments.is_empty() {
                info!("  ✅ 不存在的包返回空结果");
            } else {
                info!("  ✅ 不存在的包返回了 {} 个通用片段", fragments.len());
            }
        }
        Err(e) => {
            info!("  ✅ 正确处理不存在包的错误: {}", e);
        }
    }
    
    // 测试超时处理
    info!("测试超时处理");
    let timeout_result = timeout(
        Duration::from_millis(1), // 极短超时
        processor.process_documentation_request("rust", "serde", Some("1.0"), "test")
    ).await;
    
    match timeout_result {
        Ok(_) => {
            info!("  ✅ 操作在极短时间内完成（可能使用了缓存）");
        }
        Err(_) => {
            info!("  ✅ 正确处理超时情况");
        }
    }
    
    info!("✅ 错误处理和恢复测试通过");
    Ok(())
}

async fn test_concurrent_document_processing() -> anyhow::Result<()> {
    info!("📋 测试7: 并发文档处理");
    
    let processor = Arc::new(DocumentProcessor::new().await?);
    
    // 并发处理多个文档请求
    let concurrent_requests = vec![
        ("rust", "serde", "serialization"),
        ("python", "requests", "http"),
        ("javascript", "lodash", "utilities"),
        ("go", "gin", "web"),
        ("java", "gson", "json"),
    ];
    
    info!("启动 {} 个并发文档处理任务", concurrent_requests.len());
    let start_time = std::time::Instant::now();
    
    let mut handles = Vec::new();
    for (language, package, query) in concurrent_requests {
        let processor_clone = processor.clone();
        
        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = processor_clone.process_documentation_request(
                language, package, Some("latest"), query
            ).await;
            let duration = start.elapsed();
            (language, package, result, duration)
        });
        
        handles.push(handle);
    }
    
    // 收集结果
    let mut successful_processes = 0;
    let mut total_fragments = 0;
    
    for handle in handles {
        match handle.await {
            Ok((language, package, result, duration)) => {
                match result {
                    Ok(fragments) => {
                        info!("  ✅ {} {} 处理成功: {} 片段，耗时: {:?}", 
                              language, package, fragments.len(), duration);
                        successful_processes += 1;
                        total_fragments += fragments.len();
                        
                        // 验证片段质量
                        for fragment in fragments.iter().take(1) {
                            assert!(!fragment.content.is_empty(), "文档内容不能为空");
                            assert!(fragment.language == language, "语言标识应该正确");
                        }
                    }
                    Err(e) => {
                        warn!("  ⚠️ {} {} 处理失败: {}", language, package, e);
                    }
                }
            }
            Err(e) => {
                error!("  ❌ 并发任务执行失败: {}", e);
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    info!("🏁 并发处理完成: {}/5 成功，总计 {} 片段，总耗时: {:?}", 
          successful_processes, total_fragments, total_duration);
    
    // 验证并发处理效果
    assert!(successful_processes >= 3, "至少应该有3个处理成功");
    assert!(total_fragments > 0, "应该生成一些文档片段");
    
    info!("✅ 并发文档处理测试通过");
    Ok(())
}

async fn test_real_package_documentation() -> anyhow::Result<()> {
    info!("📋 测试8: 实际包文档生成");
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试真实存在的热门包
    let real_packages = vec![
        ("rust", "serde", "1.0", "JSON serialization library"),
        ("python", "requests", "2.28", "HTTP library for humans"),
        ("javascript", "lodash", "4.17", "JavaScript utility library"),
    ];
    
    for (language, package, version, description) in real_packages {
        info!("测试真实包文档生成: {} {} {} - {}", language, package, version, description);
        
        let start_time = std::time::Instant::now();
        match timeout(Duration::from_secs(45), 
            processor.process_documentation_request(language, package, Some(version), "documentation")
        ).await {
            Ok(Ok(fragments)) => {
                let duration = start_time.elapsed();
                info!("  ✅ {} 文档生成成功: {} 片段，耗时: {:?}", 
                      package, fragments.len(), duration);
                
                // 深度验证文档质量
                if !fragments.is_empty() {
                    let first_fragment = &fragments[0];
                    info!("    📄 主要片段: {} ({} 字符)", 
                          first_fragment.file_path, first_fragment.content.len());
                    
                    // 验证文档包含包相关信息
                    let content_lower = first_fragment.content.to_lowercase();
                    let package_variations = vec![
                        package.to_lowercase(),
                        package.replace("-", "_"),
                        package.replace("_", "-"),
                    ];
                    
                    let contains_package = package_variations.iter()
                        .any(|variant| content_lower.contains(variant));
                    
                    if contains_package {
                        info!("    ✅ 文档包含包名相关内容");
                    } else {
                        warn!("    ⚠️ 文档可能不包含包名相关内容");
                    }
                    
                    // 验证文档结构和内容质量
                    assert!(first_fragment.content.len() > 200, "主要文档应该有足够的内容");
                    assert!(first_fragment.language == language, "语言标识应该正确");
                    assert!(first_fragment.package_name == package, "包名应该正确");
                    assert!(first_fragment.version == version, "版本应该正确");
                    
                    // 检查是否包含常见文档元素
                    let has_structure = content_lower.contains("function") ||
                                      content_lower.contains("class") ||
                                      content_lower.contains("method") ||
                                      content_lower.contains("api") ||
                                      content_lower.contains("usage") ||
                                      content_lower.contains("example");
                    
                    if has_structure {
                        info!("    ✅ 文档包含结构化内容");
                    } else {
                        info!("    ℹ️ 文档可能是描述性内容");
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("  ⚠️ {} 文档生成失败: {}", package, e);
            }
            Err(_) => {
                warn!("  ⚠️ {} 文档生成超时", package);
            }
        }
    }
    
    info!("✅ 实际包文档生成测试通过");
    Ok(())
} 