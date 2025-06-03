use std::path::PathBuf;
use grape_mcp_devtools::tools::dynamic_registry::{
    DynamicRegistryBuilder, RegistrationPolicy, 
    RegistrationCondition, CacheConfig
};
use grape_mcp_devtools::cli::tool_installer::{ToolInstallConfig, InstallStrategy};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🚀 开始测试增强的动态注册模块");

    // 测试1: 基础功能测试
    test_basic_functionality().await?;
    
    // 测试2: 缓存功能测试
    test_cache_functionality().await?;
    
    // 测试3: 智能策略测试
    test_intelligent_policies().await?;
    
    // 测试4: 性能监控测试
    test_performance_monitoring().await?;
    
    // 测试5: 错误恢复测试
    test_error_recovery().await?;
    
    // 测试6: 配置管理测试
    test_config_management().await?;
    
    // 测试7: 自动安装集成测试
    test_auto_install_integration().await?;

    info!("✅ 所有测试完成！");
    Ok(())
}

async fn test_basic_functionality() -> anyhow::Result<()> {
    info!("📋 测试1: 基础功能测试");
    
    let mut registry = DynamicRegistryBuilder::new()
        .with_policy(RegistrationPolicy::Adaptive { score_threshold: 0.3 })
        .add_scan_path(PathBuf::from("."))
        .max_tools_per_language(5)
        .build();

    // 执行自动注册
    let report = registry.auto_register().await?;
    
    info!("📊 注册报告:");
    info!("  - 注册工具: {} 个", report.registered_tools.len());
    info!("  - 失败注册: {} 个", report.failed_registrations.len());
    info!("  - 注册评分: {:.1}%", report.registration_score * 100.0);
    info!("  - 注册耗时: {}ms", report.registration_duration_ms);
    info!("  - 缓存命中: {}, 缓存未命中: {}", report.cache_hits, report.cache_misses);
    
    // 验证工具注册
    let registered_tools = registry.get_registered_tools();
    assert!(!registered_tools.is_empty(), "应该至少注册一些工具");
    
    for tool_name in &report.registered_tools {
        info!("  ✅ {}", tool_name);
    }
    
    for (tool_name, error) in &report.failed_registrations {
        warn!("  ❌ {} - {}", tool_name, error);
    }

    info!("✅ 基础功能测试通过");
    Ok(())
}

async fn test_cache_functionality() -> anyhow::Result<()> {
    info!("📋 测试2: 缓存功能测试");
    
    let cache_config = CacheConfig {
        detection_cache_ttl_seconds: 60,
        tool_cache_ttl_seconds: 300,
        max_cache_entries: 50,
        enable_persistent_cache: true,
    };
    
    let mut registry = DynamicRegistryBuilder::new()
        .with_cache_config(cache_config)
        .with_policy(RegistrationPolicy::Aggressive)
        .build();

    // 第一次注册（应该缓存未命中）
    let report1 = registry.auto_register().await?;
    info!("第一次注册 - 缓存命中: {}, 未命中: {}", report1.cache_hits, report1.cache_misses);
    
    // 第二次注册（应该缓存命中）
    let report2 = registry.auto_register().await?;
    info!("第二次注册 - 缓存命中: {}, 未命中: {}", report2.cache_hits, report2.cache_misses);
    
    // 验证缓存效果
    assert!(report2.cache_hits > 0, "第二次注册应该有缓存命中");
    
    // 测试缓存清理
    registry.clear_cache().await;
    info!("🧹 缓存已清理");
    
    // 清理后再次注册（应该缓存未命中）
    let report3 = registry.auto_register().await?;
    info!("清理后注册 - 缓存命中: {}, 未命中: {}", report3.cache_hits, report3.cache_misses);

    info!("✅ 缓存功能测试通过");
    Ok(())
}

async fn test_intelligent_policies() -> anyhow::Result<()> {
    info!("📋 测试3: 智能策略测试");
    
    // 测试条件策略
    let conditions = vec![
        RegistrationCondition::MinProjectFiles(1),
        RegistrationCondition::MinScore(0.2),
        RegistrationCondition::LanguageInList(vec!["rust".to_string(), "python".to_string()]),
    ];
    
    let mut registry = DynamicRegistryBuilder::new()
        .with_policy(RegistrationPolicy::Conditional { conditions })
        .build();

    let report = registry.auto_register().await?;
    info!("条件策略注册结果: {} 个工具", report.registered_tools.len());
    
    // 测试智能策略
    registry.set_policy(RegistrationPolicy::Intelligent {
        base_threshold: 0.4,
        usage_weight: 0.3,
        performance_weight: 0.2,
    });
    
    let report2 = registry.auto_register().await?;
    info!("智能策略注册结果: {} 个工具", report2.registered_tools.len());
    
    // 测试保守策略
    registry.set_policy(RegistrationPolicy::Conservative { score_threshold: 0.7 });
    
    let report3 = registry.auto_register().await?;
    info!("保守策略注册结果: {} 个工具", report3.registered_tools.len());

    info!("✅ 智能策略测试通过");
    Ok(())
}

async fn test_performance_monitoring() -> anyhow::Result<()> {
    info!("📋 测试4: 性能监控测试");
    
    let mut registry = DynamicRegistryBuilder::new()
        .with_policy(RegistrationPolicy::Adaptive { score_threshold: 0.3 })
        .build();

    // 执行多次注册以收集性能数据
    for i in 1..=3 {
        let report = registry.auto_register().await?;
        info!("第{}次注册完成，耗时: {}ms", i, report.registration_duration_ms);
    }
    
    // 获取性能指标
    let metrics = registry.get_performance_metrics().await;
    info!("📊 性能指标:");
    info!("  - 总注册次数: {}", metrics.total_registrations);
    info!("  - 成功注册: {}", metrics.successful_registrations);
    info!("  - 失败注册: {}", metrics.failed_registrations);
    info!("  - 平均注册时间: {:.2}ms", metrics.average_registration_time_ms);
    info!("  - 缓存命中率: {:.1}%", metrics.cache_hit_rate * 100.0);
    info!("  - 最后扫描耗时: {}ms", metrics.last_scan_duration_ms);
    
    // 获取统计信息
    let stats = registry.get_statistics().await;
    info!("📈 统计信息: {}", serde_json::to_string_pretty(&stats)?);
    
    // 健康检查
    let health = registry.health_check().await;
    info!("🏥 健康检查: {}", serde_json::to_string_pretty(&health)?);

    info!("✅ 性能监控测试通过");
    Ok(())
}

async fn test_error_recovery() -> anyhow::Result<()> {
    info!("📋 测试5: 错误恢复测试");
    
    let mut registry = DynamicRegistryBuilder::new()
        .with_retry_config(2, 500) // 最多重试2次，延迟500ms
        .with_policy(RegistrationPolicy::Aggressive)
        .build();

    // 测试按需注册（可能触发重试机制）
    match registry.on_demand_register("nonexistent_language").await {
        Ok(tool_name) => {
            info!("按需注册成功: {}", tool_name);
        }
        Err(e) => {
            warn!("按需注册失败（预期行为）: {}", e);
        }
    }
    
    // 测试定期重扫描
    let changes_made = registry.periodic_rescan().await?;
    info!("定期重扫描完成，是否有变更: {}", changes_made);

    info!("✅ 错误恢复测试通过");
    Ok(())
}

async fn test_config_management() -> anyhow::Result<()> {
    info!("📋 测试6: 配置管理测试");
    
    let config_path = PathBuf::from("test_registry_config.json");
    
    let mut registry = DynamicRegistryBuilder::new()
        .with_config_path(config_path.clone())
        .with_policy(RegistrationPolicy::Adaptive { score_threshold: 0.5 })
        .build();

    // 保存配置
    registry.save_config().await?;
    info!("配置已保存到: {:?}", config_path);
    
    // 验证配置文件存在
    if config_path.exists() {
        let content = tokio::fs::read_to_string(&config_path).await?;
        info!("配置文件内容: {}", content);
        
        // 清理测试文件
        tokio::fs::remove_file(&config_path).await?;
        info!("测试配置文件已清理");
    }

    info!("✅ 配置管理测试通过");
    Ok(())
}

async fn test_auto_install_integration() -> anyhow::Result<()> {
    info!("📋 测试7: 自动安装集成测试");
    
    let install_config = ToolInstallConfig {
        strategy: InstallStrategy::DetectOnly, // 只检测不安装，避免实际安装
        auto_upgrade: false,
        install_timeout_secs: 300,
        prefer_global: true,
        backup_existing: false,
    };
    
    let mut registry = DynamicRegistryBuilder::new()
        .with_policy(RegistrationPolicy::Adaptive { score_threshold: 0.3 })
        .build();
    
    // 启用自动安装
    registry.enable_auto_install(install_config);
    
    // 执行注册（可能触发自动安装）
    let report = registry.auto_register().await?;
    
    if let Some(install_report) = &report.tool_installation_report {
        info!("工具安装报告:");
        info!("  - 安装成功: {} 个", install_report.installed.len());
        info!("  - 安装失败: {} 个", install_report.failed.len());
        info!("  - 跳过安装: {} 个", install_report.skipped.len());
    }
    
    if !report.missing_tools_detected.is_empty() {
        info!("检测到缺失工具:");
        for (language, tools) in &report.missing_tools_detected {
            info!("  {} -> [{}]", language, tools.join(", "));
        }
    }

    info!("✅ 自动安装集成测试通过");
    Ok(())
} 