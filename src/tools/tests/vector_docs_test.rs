use anyhow::Result;
use serde_json::json;

use crate::tools::vector_docs_tool::VectorDocsTool;
use crate::tools::base::MCPTool;

#[tokio::test]
async fn test_vector_docs_store_and_search() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 注意：这个测试使用嵌入式instant-distance，不需要外部服务器
    // 如果没有配置NVIDIA_API_KEY环境变量，测试可能会失败
    
    let tool = match VectorDocsTool::new() {
        Ok(tool) => tool,
        Err(e) => {
            println!("⚠️ 跳过向量化测试：API不可用 - {}", e);
            return Ok(());
        }
    };
    
    // 测试存储文档
    let store_params = json!({
        "action": "store",
        "title": "Rust Vec 文档",
        "content": "Vec<T> 是 Rust 中的动态数组类型，可以在运行时增长和缩小。它提供了高效的随机访问和末尾插入操作。",
        "language": "rust",
        "doc_type": "Class"
    });
    
    let store_result = tool.execute(store_params).await?;
    assert_eq!(store_result["status"], "success");
    
    let document_id = store_result["document_id"].as_str().unwrap();
    println!("存储的文档ID: {}", document_id);
    
    // 测试搜索文档
    let search_params = json!({
        "action": "search",
        "query": "Vec",
        "limit": 5
    });
    
    let search_result = tool.execute(search_params).await?;
    println!("搜索结果详情: {}", serde_json::to_string_pretty(&search_result)?);
    assert_eq!(search_result["status"], "success");
    assert!(search_result["results_count"].as_u64().unwrap() > 0);
    
    // 测试获取文档详情
    let get_params = json!({
        "action": "get",
        "id": document_id
    });
    
    let get_result = tool.execute(get_params).await?;
    assert_eq!(get_result["status"], "success");
    assert_eq!(get_result["document"]["title"], "Rust Vec 文档");
    
    // 测试删除文档
    let delete_params = json!({
        "action": "delete",
        "id": document_id
    });
    
    let delete_result = tool.execute(delete_params).await?;
    assert_eq!(delete_result["status"], "success");
    
    Ok(())
}

#[tokio::test]
async fn test_vector_docs_persistence() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 测试数据持久化功能
    let tool1 = match VectorDocsTool::new() {
        Ok(tool) => tool,
        Err(e) => {
            println!("⚠️ 跳过持久化测试：API不可用 - {}", e);
            return Ok(());
        }
    };
    
    // 存储一个文档
    let store_params = json!({
        "action": "store",
        "title": "持久化测试文档",
        "content": "这是一个用于测试数据持久化的文档。",
        "language": "test",
        "doc_type": "Test"
    });
    
    let store_result = tool1.execute(store_params).await?;
    assert_eq!(store_result["status"], "success");
    let document_id = store_result["document_id"].as_str().unwrap().to_string();
    
    // 创建新的工具实例（模拟程序重启）
    let tool2 = match VectorDocsTool::new() {
        Ok(tool) => tool,
        Err(e) => {
            println!("⚠️ 跳过持久化测试第二部分：API不可用 - {}", e);
            return Ok(());
        }
    };
    
    // 尝试获取之前存储的文档
    let get_params = json!({
        "action": "get",
        "id": document_id
    });
    
    let get_result = tool2.execute(get_params).await?;
    assert_eq!(get_result["status"], "success");
    assert_eq!(get_result["document"]["title"], "持久化测试文档");
    
    println!("✅ 数据持久化测试通过");
    
    Ok(())
}

#[test]
fn test_vector_docs_tool_schema() {
    // 测试工具的基本属性，不需要API密钥
    let tool = VectorDocsTool::default(); // 使用默认实现
    
    // 验证工具名称和描述
    assert_eq!(tool.name(), "vector_docs");
    assert!(!tool.description().is_empty());
    
    // 验证参数模式
    let schema = tool.parameters_schema();
    println!("工具参数模式: {:?}", schema);
}

#[test]
fn test_vector_docs_tool_creation_with_api_key() {
    // 只有在有API密钥时才测试工具创建
    if std::env::var("EMBEDDING_API_KEY").is_ok() {
        let tool = VectorDocsTool::new().expect("无法创建嵌入式向量化文档工具");
        assert_eq!(tool.name(), "vector_docs");
        println!("✅ 工具创建成功（有API密钥）");
    } else {
        println!("⚠️ 跳过工具创建测试（缺少EMBEDDING_API_KEY）");
    }
} 