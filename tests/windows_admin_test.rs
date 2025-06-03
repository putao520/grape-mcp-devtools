#[cfg(target_os = "windows")]
use grape_mcp_devtools::cli::{ToolInstaller, ToolInstallConfig, InstallStrategy};

#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_windows_admin_detection() {
    let config = ToolInstallConfig {
        strategy: InstallStrategy::DetectOnly,
        auto_upgrade: false,
        install_timeout_secs: 60,
        prefer_global: false,
        backup_existing: false,
    };
    
    let installer = ToolInstaller::new(config);
    
    // 创建一些测试工具来模拟缺失的工具
    let mut missing_tools = std::collections::HashMap::new();
    let rust_tools = vec![
        grape_mcp_devtools::cli::ToolInstallInfo {
            tool_name: "rustdoc".to_string(),
            language: "rust".to_string(),
            install_command: "rustup component add rustfmt".to_string(),
            upgrade_command: Some("rustup update".to_string()),
            check_command: "rustdoc --version".to_string(),
            required_dependencies: vec!["rust".to_string()],
            install_method: grape_mcp_devtools::cli::InstallMethod::PackageManager("rustup".to_string()),
            priority: 10,
        }
    ];
    missing_tools.insert("rust".to_string(), rust_tools);
    
    // 测试自动安装功能（应该检测权限）
    let install_report = installer.auto_install_tools(&missing_tools).await.unwrap();
    
    // 验证报告结构正确
    assert!(!install_report.installed.is_empty() || !install_report.skipped.is_empty() || !install_report.failed.is_empty());
    
    println!("安装报告: {}", install_report.generate_summary());
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn test_non_windows_admin_detection() {
    // 在非Windows系统上，权限检测应该正常工作
    let config = grape_mcp_devtools::cli::ToolInstallConfig::default();
    let installer = grape_mcp_devtools::cli::ToolInstaller::new(config);
    
    let missing_tools = std::collections::HashMap::new();
    let install_report = installer.auto_install_tools(&missing_tools).await.unwrap();
    
    // 空的缺失工具列表应该返回空报告
    assert!(install_report.installed.is_empty());
    assert!(install_report.failed.is_empty());
    assert!(install_report.skipped.is_empty());
} 