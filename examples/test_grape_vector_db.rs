//! 测试grape-vector-db在主项目中的集成
//! 
//! 这个示例展示了如何在grape-mcp-devtools项目中使用独立的grape-vector-db库

use grape_vector_db::*;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🍇 测试 Grape Vector Database 集成");

    // 测试1：基本Mock提供商
    println!("\n📖 测试1: 使用Mock提供商");
    let mut db = VectorDatabase::new("./test_data").await?;
    
    let doc = Document {
        id: "test_doc_1".to_string(),
        content: "这是一个测试文档，用于验证向量数据库的基本功能。".to_string(),
        title: Some("测试文档".to_string()),
        language: Some("zh".to_string()),
        package_name: Some("grape-mcp-devtools".to_string()),
        version: Some("0.1.0".to_string()),
        doc_type: Some("test".to_string()),
        ..Default::default()
    };
    
    // 添加文档
    let doc_id = db.add_document(doc).await?;
    println!("✅ 成功添加文档: {}", doc_id);
    
    // 搜索文档
    let results = db.search("测试文档", 5).await?;
    println!("✅ 搜索结果: {} 个", results.len());
    
    for result in &results {
        println!("   - {}: {} (相似度: {:.3})", 
                 result.document_id, 
                 result.title, 
                 result.similarity_score);
    }
    
    // 获取统计信息
    let stats = db.stats();
    println!("✅ 数据库统计: {} 个文档, {} 个向量, {:.2} MB 内存使用", 
             stats.document_count, 
             stats.vector_count, 
             stats.memory_usage_mb);

    // 测试2：自定义配置
    println!("\n⚙️  测试2: 自定义配置");
    let mut config = VectorDbConfig::default();
    config.embedding.provider = "mock".to_string();
    config.embedding.dimension = Some(512);
    config.vector_dimension = 512;
    config.cache.embedding_cache_size = 1000;
    
    let mut custom_db = VectorDatabase::with_config("./test_data_custom", config).await?;
    
    let doc2 = Document {
        id: "custom_test_doc".to_string(),
        content: "这是使用自定义配置的测试文档。".to_string(),
        title: Some("自定义配置测试".to_string()),
        language: Some("zh".to_string()),
        ..Default::default()
    };
    
    custom_db.add_document(doc2).await?;
    let custom_stats = custom_db.stats();
    println!("✅ 自定义配置数据库统计: {} 个文档", custom_stats.document_count);

    // 测试3：批量添加
    println!("\n📚 测试3: 批量添加文档");
    let docs = vec![
        Document {
            id: "batch_doc_1".to_string(),
            content: "批量文档1：Rust编程语言介绍".to_string(),
            title: Some("Rust介绍".to_string()),
            ..Default::default()
        },
        Document {
            id: "batch_doc_2".to_string(),
            content: "批量文档2：向量数据库原理".to_string(),
            title: Some("向量数据库".to_string()),
            ..Default::default()
        },
        Document {
            id: "batch_doc_3".to_string(),
            content: "批量文档3：语义搜索技术".to_string(),
            title: Some("语义搜索".to_string()),
            ..Default::default()
        },
    ];
    
    let batch_ids = db.add_documents(docs).await?;
    println!("✅ 批量添加 {} 个文档", batch_ids.len());
    
    // 最终搜索测试
    let final_results = db.search("编程", 10).await?;
    println!("✅ 最终搜索结果: {} 个", final_results.len());
    
    let final_stats = db.stats();
    println!("✅ 最终统计: {} 个文档总计", final_stats.document_count);

    println!("\n🎉 所有测试完成！grape-vector-db 在主项目中工作正常。");
    
    Ok(())
} 