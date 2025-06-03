use std::sync::Arc;
use anyhow::Result;
use tracing::info;

use grape_mcp_devtools::tools::{VectorDocsTool, MCPTool};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("grape_mcp_devtools=info")
        .init();

    info!("🧪 开始测试VectorDocsTool基础功能...");

    // 检查是否有向量化API密钥
    let has_api_key = std::env::var("EMBEDDING_API_KEY").is_ok();
    info!("🔑 向量化API密钥状态: {}", if has_api_key { "已设置" } else { "未设置（将使用简化模式）" });

    // 创建VectorDocsTool
    let vector_tool = Arc::new(VectorDocsTool::new()?);
    info!("✅ VectorDocsTool 创建成功");

    // 测试存储功能
    info!("📝 测试文档存储功能...");
    let store_result = vector_tool.execute(serde_json::json!({
        "action": "store",
        "content": "这是一个关于Rust编程语言的测试文档。Rust是一种系统编程语言，具有内存安全和并发性。",
        "title": "Rust 编程语言简介",
        "language": "rust",
        "package_name": "test_rust",
        "version": "1.0.0",
        "doc_type": "tutorial"
    })).await;

    match store_result {
        Ok(result) => {
            info!("✅ 文档存储成功: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            info!("⚠️ 文档存储失败: {} (可能因为没有API密钥)", e);
        }
    }

    // 测试搜索功能
    info!("🔍 测试文档搜索功能...");
    let search_result = vector_tool.execute(serde_json::json!({
        "action": "search",
        "query": "Rust 编程语言 内存安全",
        "limit": "3"
    })).await;

    match search_result {
        Ok(result) => {
            info!("🎯 搜索结果: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            info!("⚠️ 搜索失败: {} (这是正常的，如果没有API密钥)", e);
        }
    }

    // 如果没有API密钥，演示一下简化模式的功能
    if !has_api_key {
        info!("💡 由于没有向量化API密钥，VectorDocsTool运行在简化模式下");
        info!("   在简化模式下，系统不会进行实际的向量化处理");
        info!("   但后台文档缓存架构和其他组件仍然可以正常工作");
    }

    info!("🏁 VectorDocsTool测试完成！");

    Ok(())
} 