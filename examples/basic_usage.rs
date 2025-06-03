use grape_vector_db::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::init();
    
    // 设置API密钥（在实际使用中从环境变量读取）
    env::set_var("EMBEDDING_API_KEY", "your_nvidia_api_key_here");
    
    println!("🍇 Grape Vector Database - 基础使用示例");
    
    // 创建向量数据库实例
    println!("📁 创建向量数据库...");
    let mut db = VectorDatabase::new("./example_data").await?;
    
    // 准备示例文档
    let documents = vec![
        Document {
            id: "rust_intro".to_string(),
            content: "Rust是一种系统编程语言，专注于安全、速度和并发性。".to_string(),
            title: Some("Rust介绍".to_string()),
            language: Some("zh".to_string()),
            doc_type: Some("tutorial".to_string()),
            ..Default::default()
        },
        Document {
            id: "python_web".to_string(),
            content: "Python是一种解释型、高级编程语言，广泛用于Web开发。".to_string(),
            title: Some("Python Web开发".to_string()),
            language: Some("zh".to_string()),
            doc_type: Some("tutorial".to_string()),
            ..Default::default()
        },
        Document {
            id: "js_async".to_string(),
            content: "JavaScript的异步编程使用Promise和async/await语法。".to_string(),
            title: Some("JavaScript异步编程".to_string()),
            language: Some("zh".to_string()),
            doc_type: Some("guide".to_string()),
            ..Default::default()
        },
    ];
    
    // 添加文档
    println!("📚 添加文档到数据库...");
    for doc in documents {
        let id = db.add_document(doc.clone()).await?;
        println!("  ✅ 添加文档: {} (ID: {})", doc.title.unwrap_or("无标题".to_string()), id);
    }
    
    // 保存数据
    println!("💾 保存数据到磁盘...");
    db.save().await?;
    
    // 搜索测试
    println!("\n🔍 搜索测试:");
    
    let search_queries = vec![
        "编程语言特性",
        "Web开发框架", 
        "异步处理方法",
        "系统级编程",
    ];
    
    for query in search_queries {
        println!("\n查询: '{}'", query);
        let results = db.search(query, 3).await?;
        
        if results.is_empty() {
            println!("  ❌ 未找到相关文档");
        } else {
            for (i, result) in results.iter().enumerate() {
                println!("  {}. {} (相似度: {:.3})", 
                    i + 1, 
                    result.title, 
                    result.score
                );
            }
        }
    }
    
    // 显示统计信息
    println!("\n📊 数据库统计:");
    let stats = db.stats();
    println!("  文档数量: {}", stats.document_count);
    println!("  向量数量: {}", stats.vector_count);
    println!("  内存使用: {:.2} MB", stats.memory_usage_mb);
    
    println!("\n✅ 示例完成！");
    
    Ok(())
} 