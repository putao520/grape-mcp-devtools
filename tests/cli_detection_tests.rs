use grape_mcp_devtools::cli::{CliDetector, DynamicToolRegistry, RegistrationStrategy};
use grape_mcp_devtools::mcp::server::MCPServer;

#[tokio::test]
async fn test_cli_detector_creation() {
    let mut detector = CliDetector::new();
    
    // 检测器应该能够创建
    assert!(!detector.is_tool_available("nonexistent-tool"));
}

#[tokio::test]
async fn test_cli_detection() {
    let mut detector = CliDetector::new();
    
    // 执行检测
    let detected_tools = detector.detect_all().await.expect("检测应该成功");
    
    // 应该检测到一些工具
    assert!(!detected_tools.is_empty(), "应该检测到至少一些工具");
    
    // 检查是否有可用工具
    let available_tools = detector.get_available_tools();
    println!("检测到 {} 个可用工具", available_tools.len());
    
    for tool in available_tools {
        println!("工具: {} (版本: {:?})", tool.name, tool.version);
        assert!(tool.available, "可用工具列表中的工具应该标记为可用");
    }
}

#[tokio::test]
async fn test_feature_filtering() {
    let mut detector = CliDetector::new();
    detector.detect_all().await.expect("检测应该成功");
    
    // 测试特性过滤
    let build_tools = detector.filter_by_feature("build-tool");
    let package_managers = detector.filter_by_feature("package-manager");
    let version_control = detector.filter_by_feature("version-control");
    
    println!("构建工具: {}", build_tools.len());
    println!("包管理器: {}", package_managers.len());
    println!("版本控制: {}", version_control.len());
    
    // 所有过滤出的工具应该都是可用的
    for tool in build_tools {
        assert!(tool.available);
        assert!(tool.features.contains(&"build-tool".to_string()));
    }
}

#[tokio::test]
async fn test_dynamic_registry_only_available() {
    let strategy = RegistrationStrategy::OnlyAvailable;
    let mut registry = DynamicToolRegistry::new(strategy);
    let mcp_server = MCPServer::new();
    
    // 执行动态注册
    let report = registry.detect_and_register(&mcp_server).await.expect("注册应该成功");
    
    // 检查注册结果
    let (success, failed, skipped) = report.get_stats();
    
    println!("注册统计: {} 成功, {} 失败, {} 跳过", success, failed, skipped);
    
    // 应该至少注册了通用工具
    assert!(success > 0, "应该至少注册一些工具");
    
    // 检查报告内容
    let report_str = report.generate_report();
    assert!(report_str.contains("MCP 工具注册报告"));
    assert!(report_str.contains("总结:"));
}

#[tokio::test]
async fn test_dynamic_registry_force_all() {
    let strategy = RegistrationStrategy::ForceAll;
    let mut registry = DynamicToolRegistry::new(strategy);
    let mcp_server = MCPServer::new();
    
    // 执行强制注册
    let report = registry.detect_and_register(&mcp_server).await.expect("注册应该成功");
    
    let (success, failed, skipped) = report.get_stats();
    
    println!("强制注册统计: {} 成功, {} 失败, {} 跳过", success, failed, skipped);
    
    // 强制模式下应该注册更多工具
    assert!(success > 0, "强制模式下应该注册工具");
}

#[tokio::test]
async fn test_dynamic_registry_feature_based() {
    let features = vec!["build-tool".to_string(), "package-manager".to_string()];
    let strategy = RegistrationStrategy::FeatureBased(features);
    let mut registry = DynamicToolRegistry::new(strategy);
    let mcp_server = MCPServer::new();
    
    // 执行基于特性的注册
    let report = registry.detect_and_register(&mcp_server).await.expect("注册应该成功");
    
    let (success, failed, skipped) = report.get_stats();
    
    println!("特性注册统计: {} 成功, {} 失败, {} 跳过", success, failed, skipped);
    
    // 应该注册了一些工具
    assert!(success > 0, "基于特性的注册应该注册一些工具");
}

#[tokio::test]
async fn test_detection_report_generation() {
    let mut detector = CliDetector::new();
    detector.detect_all().await.expect("检测应该成功");
    
    let report = detector.generate_report();
    
    // 检查报告格式
    assert!(report.contains("CLI工具检测报告"));
    assert!(report.contains("总结:"));
    assert!(report.contains("工具可用"));
    
    println!("检测报告:\n{}", report);
}

#[tokio::test]
async fn test_tool_info_access() {
    let mut detector = CliDetector::new();
    detector.detect_all().await.expect("检测应该成功");
    
    let available_tools = detector.get_available_tools();
    
    for tool in available_tools {
        // 测试工具信息访问
        let tool_info = detector.get_tool_info(&tool.name);
        assert!(tool_info.is_some(), "应该能获取工具信息");
        
        let info = tool_info.unwrap();
        assert_eq!(info.name, tool.name);
        assert!(info.available);
        
        // 测试工具可用性检查
        assert!(detector.is_tool_available(&tool.name));
    }
}

#[tokio::test]
async fn test_registration_strategies_comparison() {
    let mcp_server = MCPServer::new();
    
    // 测试三种策略的结果差异
    let strategies = vec![
        ("OnlyAvailable", RegistrationStrategy::OnlyAvailable),
        ("ForceAll", RegistrationStrategy::ForceAll),
        ("FeatureBased", RegistrationStrategy::FeatureBased(vec!["build-tool".to_string()])),
    ];
    
    for (name, strategy) in strategies {
        let mut registry = DynamicToolRegistry::new(strategy);
        let report = registry.detect_and_register(&mcp_server).await.expect("注册应该成功");
        
        let (success, failed, skipped) = report.get_stats();
        println!("策略 {}: {} 成功, {} 失败, {} 跳过", name, success, failed, skipped);
        
        assert!(success > 0, "每种策略都应该注册一些工具");
    }
} 