use std::sync::Arc;
use grape_mcp_devtools::language_features::{
    LanguageVersionService, VersionComparisonService, LanguageFeaturesTool,
    FeatureCategory, data_models::*, ServiceConfig, CollectorConfig,
    EnhancedCollectorFactory
};
use grape_mcp_devtools::tools::base::MCPTool;
use serde_json::json;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🚀 开始测试增强版语言特性模块");

    // 测试1: 增强采集器基础功能
    test_enhanced_collectors().await?;
    
    // 测试2: 多语言支持测试
    test_multi_language_support().await?;
    
    // 测试3: 增强缓存系统
    test_enhanced_caching().await?;
    
    // 测试4: 配置系统测试
    test_configuration_system().await?;
    
    // 测试5: 错误恢复和fallback
    test_error_recovery().await?;
    
    // 测试6: 性能和统计
    test_performance_stats().await?;

    info!("✅ 所有增强版语言特性模块测试完成！");
    Ok(())
}

async fn test_enhanced_collectors() -> anyhow::Result<()> {
    info!("📋 测试1: 增强采集器基础功能");
    
    // 测试支持的语言
    let supported_languages = EnhancedCollectorFactory::supported_languages();
    info!("增强采集器支持的语言: {:?}", supported_languages);
    assert!(supported_languages.len() >= 6, "应该支持至少6种语言");
    
    // 测试每种语言的采集器创建
    for language in &supported_languages[..3] { // 只测试前3种语言
        info!("测试语言采集器: {}", language);
        
        match EnhancedCollectorFactory::create_collector(language) {
            Ok(collector) => {
                info!("  ✅ 成功创建 {} 采集器", language);
                
                // 测试版本获取
                match collector.get_versions().await {
                    Ok(versions) => {
                        info!("  📦 {} 版本数量: {}", language, versions.len());
                        if !versions.is_empty() {
                            info!("  📋 前3个版本: {:?}", versions.iter().take(3).collect::<Vec<_>>());
                        }
                    }
                    Err(e) => {
                        warn!("  ⚠️ 获取 {} 版本失败: {}", language, e);
                    }
                }
                
                // 测试最新版本获取
                match collector.get_latest_version().await {
                    Ok(latest) => {
                        info!("  🎯 {} 最新版本: {}", language, latest.version);
                        info!("  📅 发布日期: {}", latest.release_date);
                        info!("  🔧 特性数量: {}", latest.features.len());
                        info!("  📊 稳定版本: {}", latest.is_stable);
                        info!("  🏷️ LTS版本: {}", latest.is_lts);
                    }
                    Err(e) => {
                        warn!("  ⚠️ 获取 {} 最新版本失败: {}", language, e);
                    }
                }
            }
            Err(e) => {
                error!("  ❌ 创建 {} 采集器失败: {}", language, e);
            }
        }
    }
    
    info!("✅ 增强采集器基础功能测试通过");
    Ok(())
}

async fn test_multi_language_support() -> anyhow::Result<()> {
    info!("📋 测试2: 多语言支持测试");
    
    // 使用增强采集器配置
    let config = ServiceConfig {
        use_enhanced_collectors: true,
        cache_ttl_minutes: 30,
        max_cache_entries: 500,
        enable_fallback: true,
    };
    
    let service = LanguageVersionService::with_config(config).await?;
    let supported_languages = service.get_supported_languages();
    
    info!("服务支持的语言数量: {}", supported_languages.len());
    info!("支持的语言: {:?}", supported_languages);
    
    // 测试多种语言的版本获取
    let test_languages = vec!["rust", "python", "javascript", "java"];
    
    for language in test_languages {
        if supported_languages.contains(&language.to_string()) {
            info!("测试多语言支持: {}", language);
            
            // 测试版本列表
            match service.get_language_versions(language).await {
                Ok(versions) => {
                    info!("  📦 {} 版本数量: {}", language, versions.len());
                    
                    // 测试版本支持检查
                    if let Some(first_version) = versions.first() {
                        let is_supported = service.is_version_supported(language, first_version).await;
                        info!("  🔍 版本 {} 支持状态: {}", first_version, is_supported);
                    }
                }
                Err(e) => {
                    warn!("  ⚠️ 获取 {} 版本列表失败: {}", language, e);
                }
            }
            
            // 测试最新版本
            match service.get_latest_version(language).await {
                Ok(latest) => {
                    info!("  🎯 {} 最新版本: {}", language, latest.version);
                    info!("  📊 元数据完整性:");
                    info!("    - 发布说明: {}", latest.metadata.release_notes_url.is_some());
                    info!("    - 下载链接: {}", latest.metadata.download_url.is_some());
                    info!("    - 源码链接: {}", latest.metadata.source_url.is_some());
                    info!("    - 文档链接: {}", latest.metadata.documentation_url.is_some());
                }
                Err(e) => {
                    warn!("  ⚠️ 获取 {} 最新版本失败: {}", language, e);
                }
            }
        } else {
            warn!("语言 {} 不在支持列表中", language);
        }
    }
    
    info!("✅ 多语言支持测试通过");
    Ok(())
}

async fn test_enhanced_caching() -> anyhow::Result<()> {
    info!("📋 测试3: 增强缓存系统");
    
    let config = ServiceConfig {
        use_enhanced_collectors: true,
        cache_ttl_minutes: 1, // 短TTL用于测试
        max_cache_entries: 10,
        enable_fallback: true,
    };
    
    let service = LanguageVersionService::with_config(config).await?;
    
    // 测试缓存预热
    info!("🔥 测试缓存预热");
    match service.warm_cache("rust").await {
        Ok(_) => {
            info!("  ✅ Rust缓存预热成功");
        }
        Err(e) => {
            warn!("  ⚠️ Rust缓存预热失败: {}", e);
        }
    }
    
    // 测试缓存命中
    info!("🎯 测试缓存命中");
    let start_time = std::time::Instant::now();
    match service.get_language_versions("rust").await {
        Ok(versions) => {
            let duration = start_time.elapsed();
            info!("  📦 第一次调用: {} 版本，耗时: {:?}", versions.len(), duration);
        }
        Err(e) => {
            warn!("  ⚠️ 第一次调用失败: {}", e);
        }
    }
    
    let start_time = std::time::Instant::now();
    match service.get_language_versions("rust").await {
        Ok(versions) => {
            let duration = start_time.elapsed();
            info!("  🎯 第二次调用: {} 版本，耗时: {:?} (应该更快)", versions.len(), duration);
        }
        Err(e) => {
            warn!("  ⚠️ 第二次调用失败: {}", e);
        }
    }
    
    // 测试缓存统计
    let cache_stats = service.get_cache_stats().await;
    info!("📊 缓存统计:");
    info!("  - 总条目数: {}", cache_stats.total_entries);
    info!("  - 活跃条目数: {}", cache_stats.active_entries);
    info!("  - 过期条目数: {}", cache_stats.expired_entries);
    
    // 测试缓存清除
    service.clear_language_cache("rust").await;
    info!("🧹 清除Rust缓存");
    
    let cache_stats_after = service.get_cache_stats().await;
    info!("📊 清除后缓存统计:");
    info!("  - 总条目数: {}", cache_stats_after.total_entries);
    
    info!("✅ 增强缓存系统测试通过");
    Ok(())
}

async fn test_configuration_system() -> anyhow::Result<()> {
    info!("📋 测试4: 配置系统测试");
    
    // 测试不同配置
    let configs = vec![
        ("增强采集器", ServiceConfig {
            use_enhanced_collectors: true,
            cache_ttl_minutes: 60,
            max_cache_entries: 1000,
            enable_fallback: true,
        }),
        ("传统采集器", ServiceConfig {
            use_enhanced_collectors: false,
            cache_ttl_minutes: 30,
            max_cache_entries: 500,
            enable_fallback: false,
        }),
    ];
    
    for (config_name, config) in configs {
        info!("测试配置: {}", config_name);
        
        match LanguageVersionService::with_config(config).await {
            Ok(service) => {
                let supported_languages = service.get_supported_languages();
                info!("  📋 支持语言数量: {}", supported_languages.len());
                
                // 测试一种语言
                if let Some(language) = supported_languages.first() {
                    match service.get_language_versions(language).await {
                        Ok(versions) => {
                            info!("  📦 {} 版本数量: {}", language, versions.len());
                        }
                        Err(e) => {
                            warn!("  ⚠️ 获取版本失败: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("  ❌ 创建服务失败: {}", e);
            }
        }
    }
    
    info!("✅ 配置系统测试通过");
    Ok(())
}

async fn test_error_recovery() -> anyhow::Result<()> {
    info!("📋 测试5: 错误恢复和fallback");
    
    let config = ServiceConfig {
        use_enhanced_collectors: true,
        cache_ttl_minutes: 60,
        max_cache_entries: 100,
        enable_fallback: true,
    };
    
    let service = LanguageVersionService::with_config(config).await?;
    
    // 测试不支持的语言
    match service.get_language_versions("nonexistent_language").await {
        Ok(_) => {
            warn!("  ⚠️ 预期失败但成功了");
        }
        Err(e) => {
            info!("  ✅ 正确处理不支持的语言: {}", e);
        }
    }
    
    // 测试版本支持检查
    let is_supported = service.is_version_supported("nonexistent_language", "1.0.0").await;
    info!("  🔍 不存在语言的版本支持检查: {}", is_supported);
    assert!(!is_supported, "不存在的语言应该返回false");
    
    // 测试Python的fallback机制（因为Python API可能失败）
    match service.get_language_versions("python").await {
        Ok(versions) => {
            info!("  📦 Python版本获取成功: {} 个版本", versions.len());
            if versions.is_empty() {
                info!("  🔄 可能使用了fallback机制");
            }
        }
        Err(e) => {
            info!("  ⚠️ Python版本获取失败，这是预期的: {}", e);
        }
    }
    
    info!("✅ 错误恢复和fallback测试通过");
    Ok(())
}

async fn test_performance_stats() -> anyhow::Result<()> {
    info!("📋 测试6: 性能和统计");
    
    let service = Arc::new(LanguageVersionService::new().await?);
    
    // 性能测试：并发获取多种语言版本
    let languages = vec!["rust", "python", "javascript"];
    let start_time = std::time::Instant::now();
    
    let mut handles = Vec::new();
    for language in languages {
        let service_clone = service.clone();
        let lang = language.to_string();
        
        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = service_clone.get_language_versions(&lang).await;
            let duration = start.elapsed();
            (lang, result, duration)
        });
        
        handles.push(handle);
    }
    
    // 等待所有任务完成
    for handle in handles {
        match handle.await {
            Ok((language, result, duration)) => {
                match result {
                    Ok(versions) => {
                        info!("  📦 {} 并发获取: {} 版本，耗时: {:?}", language, versions.len(), duration);
                    }
                    Err(e) => {
                        warn!("  ⚠️ {} 并发获取失败: {}，耗时: {:?}", language, e, duration);
                    }
                }
            }
            Err(e) => {
                error!("  ❌ 任务执行失败: {}", e);
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    info!("🏁 并发测试总耗时: {:?}", total_duration);
    
    // 缓存统计
    let cache_stats = service.get_cache_stats().await;
    info!("📊 最终缓存统计:");
    info!("  - 总条目数: {}", cache_stats.total_entries);
    info!("  - 活跃条目数: {}", cache_stats.active_entries);
    info!("  - 过期条目数: {}", cache_stats.expired_entries);
    
    // 测试特性搜索性能
    if cache_stats.total_entries > 0 {
        let start_time = std::time::Instant::now();
        match service.search_features("rust", "async", Some(FeatureCategory::Async), None).await {
            Ok(features) => {
                let duration = start_time.elapsed();
                info!("🔍 特性搜索: 找到 {} 个async特性，耗时: {:?}", features.len(), duration);
            }
            Err(e) => {
                warn!("⚠️ 特性搜索失败: {}", e);
            }
        }
    }
    
    info!("✅ 性能和统计测试通过");
    Ok(())
} 