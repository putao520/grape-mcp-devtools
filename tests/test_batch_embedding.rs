//! 批量嵌入功能测试

use anyhow::Result;
use grape_mcp_devtools::tools::vector_docs_tool::VectorDocsTool;
use std::env;
use tokio;

#[tokio::test]
async fn test_batch_embedding_performance() -> Result<()> {
    // 设置测试环境
    if env::var("NVIDIA_API_KEY").is_err() {
        println!("⚠️ 跳过批量嵌入测试：未设置 NVIDIA_API_KEY 环境变量");
        return Ok(());
    }

    let vector_tool = VectorDocsTool::new().await?;
    
    // 测试单个文本的嵌入
    let start_time = std::time::Instant::now();
    let single_text = "这是一个用于测试的示例文档内容。它包含了一些技术术语如async、await、Result等。";
    let _single_embedding = vector_tool.generate_embedding(single_text).await?;
    let single_duration = start_time.elapsed();
    
    println!("✅ 单个嵌入耗时: {:?}", single_duration);

    // 测试批量文本的嵌入
    let test_texts = vec![
        "Rust是一种系统编程语言，专注于安全、速度和并发。".to_string(),
        "async/await是Rust中处理异步编程的关键特性。".to_string(),
        "向量数据库可以高效地存储和检索嵌入向量。".to_string(),
        "语义搜索通过理解文本含义提供更准确的搜索结果。".to_string(),
        "MCP协议提供了模型上下文协议的标准化实现。".to_string(),
    ];
    
    let batch_start_time = std::time::Instant::now();
    let _batch_embeddings = vector_tool.generate_embeddings_batch(&test_texts).await?;
    let batch_duration = batch_start_time.elapsed();
    
    println!("✅ 批量嵌入（{}个文档）耗时: {:?}", test_texts.len(), batch_duration);
    
    // 性能对比
    let theoretical_single_time = single_duration * test_texts.len() as u32;
    let efficiency_ratio = theoretical_single_time.as_millis() as f64 / batch_duration.as_millis() as f64;
    
    println!("📊 性能对比:");
    println!("   单个操作预估总时间: {:?}", theoretical_single_time);
    println!("   批量操作实际时间: {:?}", batch_duration);
    println!("   效率提升比例: {:.2}x", efficiency_ratio);
    
    // 验证批量操作确实更高效（至少提升20%）
    assert!(efficiency_ratio > 1.2, "批量嵌入应该比单个嵌入更高效");
    
    Ok(())
}

#[tokio::test]
async fn test_embedding_cache_mechanism() -> Result<()> {
    if env::var("NVIDIA_API_KEY").is_err() {
        println!("⚠️ 跳过缓存测试：未设置 NVIDIA_API_KEY 环境变量");
        return Ok(());
    }

    let vector_tool = VectorDocsTool::new().await?;
    
    let test_content = "缓存测试内容：这段文本将被用来测试嵌入向量的缓存机制。";
    
    // 第一次调用（应该调用API）
    let start_time = std::time::Instant::now();
    let _first_embedding = vector_tool.generate_embedding(test_content).await?;
    let first_duration = start_time.elapsed();
    
    println!("✅ 首次嵌入（调用API）耗时: {:?}", first_duration);
    
    // 第二次调用（应该命中缓存）
    let cache_start_time = std::time::Instant::now();
    let _cached_embedding = vector_tool.generate_embedding(test_content).await?;
    let cache_duration = cache_start_time.elapsed();
    
    println!("✅ 缓存嵌入耗时: {:?}", cache_duration);
    
    // 验证缓存确实提升了性能（至少快10倍）
    let speed_up_ratio = first_duration.as_millis() as f64 / cache_duration.as_millis() as f64;
    println!("📊 缓存加速比例: {:.2}x", speed_up_ratio);
    
    assert!(speed_up_ratio > 10.0, "缓存应该显著提升性能");
    
    Ok(())
}

#[tokio::test] 
async fn test_hybrid_search_functionality() -> Result<()> {
    let vector_tool = VectorDocsTool::new().await?;
    
    // 测试混合搜索功能（即使没有嵌入向量也应该工作）
    let query_text = "Rust编程语言";
    let dummy_embedding = vec![0.1f32; 1024]; // 模拟查询嵌入
    
    let search_start_time = std::time::Instant::now();
    let search_results = vector_tool.hybrid_search(&dummy_embedding, query_text, 5)?;
    let search_duration = search_start_time.elapsed();
    
    println!("✅ 混合搜索耗时: {:?}", search_duration);
    println!("📋 搜索结果数量: {}", search_results.len());
    
    // 验证搜索功能基本可用
    assert!(search_duration.as_millis() < 1000, "搜索应该在1秒内完成");
    
    Ok(())
} 