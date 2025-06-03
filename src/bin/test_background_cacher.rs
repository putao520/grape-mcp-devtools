use std::collections::HashMap;
use std::sync::Arc;
use std::fs;
use anyhow::Result;
use tracing::{info, warn};

use grape_mcp_devtools::tools::{
    VectorDocsTool, 
    EnhancedDocumentProcessor,
    EnvironmentDetectionTool,
    MCPTool,
    background_cacher::{BackgroundDocCacher, DocCacherConfig}
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("grape_mcp_devtools=info,background_cacher=debug")
        .init();

    info!("🧪 开始测试后台文档缓存系统...");

    // 创建数据目录
    let base_data_path = std::env::current_dir()?.join(".mcp_cache");
    let vector_store_path = base_data_path.join("vector_store");
    fs::create_dir_all(&vector_store_path)?;

    // 创建VectorDocsTool和EnhancedDocumentProcessor
    let vector_tool = Arc::new(VectorDocsTool::new()?);
    let enhanced_processor = Arc::new(
        EnhancedDocumentProcessor::new(Arc::clone(&vector_tool)).await?
    );

    // 使用真实的环境检测而非模拟数据
    info!("🔍 开始真实环境检测...");
    let env_detector = EnvironmentDetectionTool::new();
    
    // 执行真实的环境检测
    let detection_result = env_detector.execute(serde_json::json!({
        "action": "detect_all",
        "scan_path": std::env::current_dir()?.to_string_lossy()
    })).await?;

    // 解析检测结果
    let mut detected_languages = HashMap::new();
    if let Some(languages) = detection_result.get("detected_languages").and_then(|v| v.as_object()) {
        for (lang_name, lang_data) in languages {
            if let Ok(lang_info) = serde_json::from_value::<grape_mcp_devtools::tools::environment_detector::LanguageInfo>(lang_data.clone()) {
                detected_languages.insert(lang_name.clone(), lang_info);
            }
        }
    }

    // 如果没有检测到语言，添加当前项目的Rust信息作为备用
    if detected_languages.is_empty() {
        warn!("未检测到任何语言，添加当前Rust项目作为备用");
        detected_languages.insert("rust".to_string(), grape_mcp_devtools::tools::environment_detector::LanguageInfo {
            name: "rust".to_string(),
            score: 0.9,
            project_files: vec!["Cargo.toml".to_string(), "src/main.rs".to_string()],
            cli_tools: vec!["cargo".to_string()],
            detected_features: vec!["tokio".to_string(), "serde".to_string()],
        });
    }

    info!("📊 真实检测结果: 发现 {} 种语言", detected_languages.len());
    for (lang, info) in &detected_languages {
        info!("  - {}: 分数 {:.2}, {} 个项目文件", lang, info.score, info.project_files.len());
    }

    // 创建后台缓存器
    info!("⚙️ 创建后台文档缓存器...");
    let cacher_config = DocCacherConfig { 
        enabled: true, 
        concurrent_tasks: 2 
    };
    let doc_cacher = BackgroundDocCacher::new(
        cacher_config,
        Arc::clone(&enhanced_processor),
        Arc::clone(&vector_tool),
    );

    // 启动后台缓存
    info!("🚀 启动后台文档缓存...");
    match doc_cacher.queue_dependencies_for_caching(&detected_languages).await {
        Ok(_) => {
            info!("✅ 后台文档缓存任务已成功启动！");
        }
        Err(e) => {
            warn!("⚠️ 后台文档缓存启动失败: {}", e);
        }
    }

    // 等待一段时间让后台任务运行
    info!("⏳ 等待5秒让后台任务运行...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // 测试向量搜索（使用Arc<VectorDocsTool>的execute方法）
    info!("🔍 测试向量搜索功能...");
    let search_result = vector_tool.execute(serde_json::json!({
        "action": "search",
        "query": "rust standard library documentation",
        "limit": "3"
    })).await;

    match search_result {
        Ok(result) => {
            info!("🎯 搜索结果: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            warn!("⚠️ 搜索失败: {}", e);
        }
    }

    // 测试存储功能
    info!("📝 测试向量存储功能...");
    let store_result = vector_tool.execute(serde_json::json!({
        "action": "store",
        "content": "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.",
        "title": "Rust Overview",
        "language": "rust",
        "package_name": "test_package",
        "version": "1.0.0",
        "doc_type": "documentation"
    })).await;

    match store_result {
        Ok(result) => {
            info!("📦 存储结果: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            warn!("⚠️ 存储失败: {}", e);
        }
    }

    // 再次搜索测试存储的内容
    info!("🔍 再次测试搜索功能（搜索刚存储的内容）...");
    let search_result2 = vector_tool.execute(serde_json::json!({
        "action": "search",
        "query": "systems programming language thread safety",
        "limit": "3"
    })).await;

    match search_result2 {
        Ok(result) => {
            info!("🎯 第二次搜索结果: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            warn!("⚠️ 第二次搜索失败: {}", e);
        }
    }

    info!("🏁 后台文档缓存系统测试完成！");

    Ok(())
} 