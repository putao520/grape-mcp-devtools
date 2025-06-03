use grape_mcp_devtools::cli::{ToolInstaller, ToolInstallConfig, InstallStrategy, ToolInstallInfo, InstallMethod};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("grape_mcp_devtools=debug,info")
        .init();

    println!("🔧 测试Windows管理员权限检测功能");
    println!("═══════════════════════════════════════");

    // 创建工具安装配置
    let config = ToolInstallConfig {
        strategy: InstallStrategy::Interactive,
        auto_upgrade: false,
        install_timeout_secs: 60,
        prefer_global: true,
        backup_existing: false,
    };

    // 创建工具安装器
    let installer = ToolInstaller::new(config);

    // 模拟一些缺失的工具
    let mut missing_tools = HashMap::new();
    
    // Rust工具
    let rust_tools = vec![
        ToolInstallInfo {
            tool_name: "mdbook".to_string(),
            language: "rust".to_string(),
            install_command: "cargo install mdbook".to_string(),
            upgrade_command: Some("cargo install --force mdbook".to_string()),
            check_command: "mdbook --version".to_string(),
            required_dependencies: vec!["cargo".to_string()],
            install_method: InstallMethod::PackageManager("cargo".to_string()),
            priority: 8,
        }
    ];
    missing_tools.insert("rust".to_string(), rust_tools);

    // Python工具
    let python_tools = vec![
        ToolInstallInfo {
            tool_name: "mkdocs".to_string(),
            language: "python".to_string(),
            install_command: "pip install mkdocs".to_string(),
            upgrade_command: Some("pip install --upgrade mkdocs".to_string()),
            check_command: "mkdocs --version".to_string(),
            required_dependencies: vec!["python".to_string(), "pip".to_string()],
            install_method: InstallMethod::PackageManager("pip".to_string()),
            priority: 7,
        }
    ];
    missing_tools.insert("python".to_string(), python_tools);

    // C++工具（需要系统包管理器）
    let cpp_tools = vec![
        ToolInstallInfo {
            tool_name: "doxygen".to_string(),
            language: "cpp".to_string(),
            install_command: "choco install doxygen.install".to_string(),
            upgrade_command: Some("choco upgrade doxygen.install".to_string()),
            check_command: "doxygen --version".to_string(),
            required_dependencies: vec![],
            install_method: InstallMethod::SystemPackage("choco".to_string()),
            priority: 9,
        }
    ];
    missing_tools.insert("cpp".to_string(), cpp_tools);

    println!("📋 模拟缺失工具列表:");
    for (language, tools) in &missing_tools {
        println!("  🗣️ {}: {} 个工具", language, tools.len());
        for tool in tools {
            println!("    - {} ({})", tool.tool_name, tool.install_command);
        }
    }
    println!();

    // 测试自动安装功能
    println!("🚀 开始测试自动安装功能...");
    match installer.auto_install_tools(&missing_tools).await {
        Ok(report) => {
            println!("✅ 安装测试完成!");
            println!("📊 安装报告:");
            println!("  ✅ 成功安装: {} 个", report.installed.len());
            for tool in &report.installed {
                println!("    - {}", tool);
            }
            
            println!("  ❌ 安装失败: {} 个", report.failed.len());
            for (tool, error) in &report.failed {
                println!("    - {}: {}", tool, error);
            }
            
            println!("  ⏭️ 跳过安装: {} 个", report.skipped.len());
            for tool in &report.skipped {
                println!("    - {}", tool);
            }
        }
        Err(e) => {
            println!("❌ 安装测试失败: {}", e);
        }
    }

    println!();
    println!("🎯 测试完成!");
    
    Ok(())
} 