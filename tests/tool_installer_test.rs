use grape_mcp_devtools::cli::{ToolInstaller, ToolInstallConfig, InstallStrategy};

#[tokio::test]
async fn test_tool_installer_creation() {
    let config = ToolInstallConfig {
        strategy: InstallStrategy::DetectOnly,
        auto_upgrade: false,
        install_timeout_secs: 60,
        prefer_global: false,
        backup_existing: false,
    };
    
    let installer = ToolInstaller::new(config);
    let supported_tools = installer.get_supported_tools();
    
    // 验证支持的工具包含主要语言的文档工具
    assert!(supported_tools.contains_key("rustdoc"));
    assert!(supported_tools.contains_key("sphinx"));
    assert!(supported_tools.contains_key("jsdoc"));
    assert!(supported_tools.contains_key("javadoc"));
    assert!(supported_tools.contains_key("godoc"));
}

#[tokio::test]
async fn test_missing_tools_detection() {
    let config = ToolInstallConfig::default();
    let installer = ToolInstaller::new(config);
    
    let detected_languages = vec!["rust".to_string(), "python".to_string()];
    let missing_tools = installer.detect_missing_tools(&detected_languages).await.unwrap();
    
    // 验证检测结果结构正确
    assert!(missing_tools.is_empty() || missing_tools.len() <= detected_languages.len());
    
    // 如果有缺失工具，验证它们按优先级排序
    for (_language, tools) in &missing_tools {
        for i in 1..tools.len() {
            assert!(tools[i-1].priority >= tools[i].priority, "工具应按优先级排序");
        }
    }
}

#[tokio::test]
async fn test_system_detection() {
    let config = ToolInstallConfig::default();
    let installer = ToolInstaller::new(config);
    let system_info = installer.get_system_info();
    
    // 验证系统检测
    let os_types = ["windows", "macos", "linux"];
    assert!(os_types.contains(&system_info.os_type.as_str()));
    
    // 在Windows上验证可能的包管理器
    if system_info.os_type == "windows" {
        let expected_managers = ["choco", "winget", "scoop"];
        for manager in &system_info.package_managers {
            assert!(expected_managers.contains(&manager.as_str()));
        }
    }
}

#[tokio::test]
async fn test_tool_install_info_structure() {
    let config = ToolInstallConfig::default();
    let installer = ToolInstaller::new(config);
    let supported_tools = installer.get_supported_tools();
    
    // 验证每个工具信息的完整性
    for (tool_name, tool_info) in supported_tools {
        assert_eq!(tool_name, &tool_info.tool_name);
        assert!(!tool_info.language.is_empty());
        assert!(!tool_info.install_command.is_empty());
        assert!(!tool_info.check_command.is_empty());
        assert!(tool_info.priority > 0 && tool_info.priority <= 10);
    }
}

#[tokio::test]
async fn test_detect_only_strategy() {
    let config = ToolInstallConfig {
        strategy: InstallStrategy::DetectOnly,
        auto_upgrade: false,
        install_timeout_secs: 60,
        prefer_global: false,
        backup_existing: false,
    };
    
    let installer = ToolInstaller::new(config);
    let detected_languages = vec!["rust".to_string()];
    let missing_tools = installer.detect_missing_tools(&detected_languages).await.unwrap();
    
    // 使用DetectOnly策略时，应该只检测不安装
    let install_report = installer.auto_install_tools(&missing_tools).await.unwrap();
    
    // 所有工具都应该被跳过
    assert!(install_report.installed.is_empty());
    assert!(install_report.failed.is_empty());
}

#[tokio::test]
async fn test_installation_report_summary() {
    let mut report = grape_mcp_devtools::cli::InstallationReport {
        installed: vec!["tool1".to_string(), "tool2".to_string()],
        failed: vec![("tool3".to_string(), "error".to_string())],
        skipped: vec!["tool4".to_string()],
    };
    
    let summary = report.generate_summary();
    assert!(summary.contains("成功: 2"));
    assert!(summary.contains("失败: 1"));
    assert!(summary.contains("跳过: 1"));
}

#[tokio::test]
async fn test_upgrade_report_summary() {
    let report = grape_mcp_devtools::cli::UpgradeReport {
        upgraded: vec!["tool1".to_string()],
        failed: vec![("tool2".to_string(), "error".to_string())],
        available: vec!["tool3".to_string(), "tool4".to_string()],
    };
    
    let summary = report.generate_summary();
    assert!(summary.contains("已升级: 1"));
    assert!(summary.contains("失败: 1"));
    assert!(summary.contains("可升级: 2"));
} 