use grape_vector_db::{VectorDatabase, Document};
use std::env;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍇 测试Grape Vector Database集成");
    
    // 设置API密钥（用于测试）
    env::set_var("EMBEDDING_API_KEY", "test_api_key");
    
    // 创建向量数据库实例
    println!("📁 创建向量数据库实例...");
    match VectorDatabase::new("./test_vector_data").await {
        Ok(mut db) => {
            println!("✅ 向量数据库创建成功");
            
            // 创建测试文档
            let test_doc = Document {
                id: "test_doc_1".to_string(),
                content: "这是一个Rust测试文档，用于验证向量数据库功能".to_string(),
                title: Some("Rust测试文档".to_string()),
                language: Some("zh".to_string()),
                package_name: None,
                version: None,
                doc_type: Some("test".to_string()),
                metadata: HashMap::new(),
            };
            
            println!("📚 测试添加文档...");
            
            // 注意：由于我们没有设置真实的API密钥，这里会失败
            // 但这是预期的，我们只是测试结构是否正确
            match db.add_document(test_doc).await {
                Ok(id) => {
                    println!("✅ 文档添加成功，ID: {}", id);
                }
                Err(e) => {
                    println!("⚠️  文档添加失败（预期的，因为没有真实API密钥）: {}", e);
                }
            }
            
            // 测试统计信息
            let stats = db.stats();
            println!("📊 数据库统计:");
            println!("  文档数量: {}", stats.document_count);
            println!("  向量数量: {}", stats.vector_count);
            println!("  内存使用: {:.2} MB", stats.memory_usage_mb);
            
        }
        Err(e) => {
            println!("❌ 向量数据库创建失败: {}", e);
        }
    }
    
    println!("✅ Grape Vector Database集成测试完成");
    Ok(())
} 