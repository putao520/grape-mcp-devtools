use anyhow::Result;
use tracing::{info, error, warn, debug};
use tracing_subscriber;
use dotenv;
use std::sync::Arc;
use std::path::PathBuf;
use std::{collections::HashMap, fs};

mod errors;
mod mcp;
mod tools;
mod versioning;
mod cli;

use mcp::server::MCPServer;
use tools::{VectorDocsTool, EnhancedDocumentProcessor, DynamicRegistryBuilder, EnvironmentDetectionTool};
use tools::background_cacher::{BackgroundDocCacher, DocCacherConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "grape_mcp_devtools=info,background_cacher=debug".to_string()))
        .init();

    info!("🚀 启动 Grape MCP DevTools 服务器...");

    let base_data_path = std::env::current_dir()?.join(".mcp_cache");
    let vector_store_path = base_data_path.join("vector_store");
    fs::create_dir_all(&vector_store_path).map_err(|e| anyhow::anyhow!("创建向量存储目录失败: {:?} - {}", vector_store_path, e))?;

    let vector_tool = Arc::new(
        VectorDocsTool::new()
            .map_err(|e| anyhow::anyhow!("初始化 VectorDocsTool 失败: {}", e))?
    );
    let enhanced_processor = Arc::new(
        EnhancedDocumentProcessor::new(Arc::clone(&vector_tool)).await
            .map_err(|e| anyhow::anyhow!("初始化 EnhancedDocumentProcessor 失败: {}", e))?
    );

    // 创建工具安装配置
    let install_config = cli::ToolInstallConfig::default();

    // 创建动态工具注册器
    let mut registry = DynamicRegistryBuilder::new()
        .with_policy(tools::RegistrationPolicy::Adaptive { score_threshold: 0.3 })
        .add_scan_path(std::env::current_dir()?)
        .with_shared_doc_processor(Arc::clone(&enhanced_processor))
        .with_config_path(base_data_path.join("registry_config.json")) // 为registry指定配置路径
        .build();

    // 启用工具自动安装功能
    registry.enable_auto_install(install_config);

    info!("🔍 执行环境检测和动态工具注册...");
    
    // 执行动态注册（包含自动工具安装）
    let (registration_report, detection_report_option) = match registry.auto_register().await {
        Ok((report, detection_opt)) => {
            info!("✅ 动态注册完成！");
            info!("📊 注册报告:");
            info!("   - 注册工具: {} 个", report.registered_tools.len());
            info!("   - 失败注册: {} 个", report.failed_registrations.len());
            info!("   - 注册评分: {:.1}%", report.registration_score * 100.0);
            info!("   - 注册耗时: {}ms", report.registration_duration_ms);
            info!("   - 自动安装: {}", if report.auto_install_enabled { "启用" } else { "禁用" });
            
            for tool in &report.registered_tools {
                info!("   ✅ {}", tool);
            }
            
            for (tool, error) in &report.failed_registrations {
                warn!("   ❌ {} - {}", tool, error);
            }

            // 显示缺失工具信息
            if !report.missing_tools_detected.is_empty() {
                info!("🔧 检测到缺失的文档工具:");
                for (language, tools) in &report.missing_tools_detected {
                    info!("   {} -> [{}]", language, tools.join(", "));
                }
            }

            // 显示工具安装报告
            if let Some(install_report) = &report.tool_installation_report {
                info!("📦 工具安装报告:");
                if !install_report.installed.is_empty() {
                    info!("   ✅ 成功安装: [{}]", install_report.installed.join(", "));
                }
                if !install_report.failed.is_empty() {
                    info!("   ❌ 安装失败:");
                    for (tool, error) in &install_report.failed {
                        info!("      {} - {}", tool, error);
                    }
                }
                if !install_report.skipped.is_empty() {
                    info!("   ⏭️ 跳过安装: [{}]", install_report.skipped.join(", "));
                }
            }

            (report, detection_opt)
        }
        Err(e) => {
            error!("❌ 动态注册失败: {}", e);
            return Err(e);
        }
    };

    if let Some(detection_report) = detection_report_option {
        if !detection_report.detected_languages.is_empty() {
            info!("ℹ️ 环境检测到项目依赖，准备启动后台文档缓存...");
            let cacher_config = DocCacherConfig { enabled: true, concurrent_tasks: 2 }; // 示例配置
            let doc_cacher = BackgroundDocCacher::new(
                cacher_config,
                Arc::clone(&enhanced_processor),
                Arc::clone(&vector_tool),
            );
            
            // 直接将 detection_report.detected_languages (HashMap<String, tools::environment_detector::LanguageInfo>) 传递
            if let Err(e) = doc_cacher.queue_dependencies_for_caching(&detection_report.detected_languages).await {
                warn!("启动后台文档缓存失败: {}", e);
            }
        } else {
            info!("环境检测未发现任何语言的依赖，跳过后台文档缓存。");
        }
    } else {
        info!("动态注册未返回环境检测报告，无法启动后台文档缓存。");
    }

    // 检查工具升级
    info!("⬆️ 检查工具升级...");
    if let Err(e) = registry.check_and_upgrade_tools().await {
        warn!("⚠️ 升级检查失败: {}", e);
    }

    // 创建MCP服务器实例
    let mcp_server = MCPServer::new();

    // 从注册器获取已注册的工具并添加到服务器
    info!("🔧 将动态注册的工具添加到MCP服务器...");
    let mut dynamic_tools_count = 0;
    
    for (tool_name, tool_arc) in registry.get_registered_tools() {
        if mcp_server.register_tool_arc(Arc::clone(tool_arc)).await.is_ok() {
            info!("✅ 工具已添加到MCP服务器: {}", tool_name);
            dynamic_tools_count += 1;
        } else {
            warn!("⚠️ 添加动态工具 {} 到MCP服务器失败", tool_name);
        }
    }

    // 手动注册基础工具
    let base_tools: Vec<Box<dyn tools::MCPTool>> = vec![
        Box::new(tools::SearchDocsTool::new()),
        Box::new(EnvironmentDetectionTool::new()), // Ensure this is tools::EnvironmentDetectionTool
        Box::new(tools::CheckVersionTool::new()),
        // VectorDocsTool本身也可以是一个MCP工具，如果它的execute方法被设计为如此
        // 但我们这里主要通过 BackgroundCacher 和 EnhancedDocumentProcessor 间接使用其功能
        // 如果需要MCP接口直接操作VectorStore，可以取消注释下面这行，并确保它实现了MCPTool
        // Box::new(VectorDocsTool::new(vector_store_path.clone())?), 
    ];

    for tool in base_tools {
        let name = tool.name().to_string();
        if mcp_server.register_tool(tool).await.is_ok() {
            info!("✅ 基础工具已添加到服务器: {}", name);
        } else {
            warn!("⚠️ 添加基础工具 {} 到MCP服务器失败", name);
        }
    }

    let tool_count = mcp_server.get_tool_count().await?;
    info!("📋 服务器工具总数: {} (动态注册: {}, 基础工具: {})", 
          tool_count, dynamic_tools_count, tool_count - dynamic_tools_count);
    
    // 打印所有注册的工具详细信息
    info!("📋 所有已注册的MCP工具:");
    match mcp_server.list_tools().await {
        Ok(tool_infos) => {
            for (index, tool_info) in tool_infos.iter().enumerate() {
                info!("   {}. 🔧 {}", index + 1, tool_info.name);
                info!("      📝 描述: {}", tool_info.description);
                if let Some(language) = &tool_info.language {
                    info!("      🗣️ 语言: {}", language);
                }
                if let Some(category) = &tool_info.category {
                    info!("      📂 类别: {}", category);
                }
                if let Some(version) = &tool_info.version {
                    info!("      🔖 版本: {}", version);
                }
                
                // 显示参数schema的简要信息
                if !tool_info.parameters.is_null() {
                    if let Some(props) = tool_info.parameters.get("properties") {
                        if let Some(props_obj) = props.as_object() {
                            let param_names: Vec<String> = props_obj.keys().map(|k| k.clone()).collect();
                            if !param_names.is_empty() {
                                info!("      ⚙️ 参数: [{}]", param_names.join(", "));
                            }
                        }
                    }
                }
                info!(""); // 空行分隔
            }
            
            // 按语言分组统计
            let mut language_stats: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for tool_info in &tool_infos {
                if let Some(language) = &tool_info.language {
                    *language_stats.entry(language.clone()).or_insert(0) += 1;
                } else {
                    *language_stats.entry("通用".to_string()).or_insert(0) += 1;
                }
            }
            
            info!("📊 工具语言分布:");
            for (language, count) in &language_stats {
                info!("   - {}: {} 个工具", language, count);
            }
        }
        Err(e) => {
            warn!("⚠️ 获取工具列表失败: {}", e);
        }
    }

    // 显示动态注册统计信息
    let stats = registry.get_statistics().await;
    info!("📈 动态注册统计:");
    for (key, value) in stats {
        info!("   - {}: {}", key, value);
    }

    // 创建并运行完整的MCP服务器
    let mut server = mcp::server::Server::new(
        "grape-mcp-devtools".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
        mcp_server,
    );

    info!("🌐 启动MCP服务器...");
    server.run().await?;

    Ok(())
} 