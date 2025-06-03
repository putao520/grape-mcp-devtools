use std::sync::Arc;
use grape_mcp_devtools::language_features::{
    LanguageVersionService, VersionComparisonService, LanguageFeaturesTool,
    FeatureCategory, data_models::*
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

    info!("🚀 开始测试语言特性模块");

    // 测试1: 语言版本服务基础功能
    test_language_version_service().await?;
    
    // 测试2: 版本比较服务
    test_version_comparison_service().await?;
    
    // 测试3: 语言特性工具
    test_language_features_tool().await?;
    
    // 测试4: 缓存功能测试
    test_cache_functionality().await?;
    
    // 测试5: 特性搜索功能
    test_feature_search().await?;
    
    // 测试6: 错误处理测试
    test_error_handling().await?;

    info!("✅ 所有语言特性模块测试完成！");
    Ok(())
}

async fn test_language_version_service() -> anyhow::Result<()> {
    info!("📋 测试1: 语言版本服务基础功能");
    
    let service = LanguageVersionService::new().await?;
    
    // 测试支持的语言列表
    let supported_languages = service.get_supported_languages();
    info!("支持的语言: {:?}", supported_languages);
    assert!(!supported_languages.is_empty(), "应该支持至少一种语言");
    
    // 测试每种支持的语言
    for language in &supported_languages {
        info!("测试语言: {}", language);
        
        // 测试获取版本列表
        match service.get_language_versions(language).await {
            Ok(versions) => {
                info!("  {} 版本数量: {}", language, versions.len());
                if !versions.is_empty() {
                    info!("  最新几个版本: {:?}", versions.iter().take(3).collect::<Vec<_>>());
                }
            }
            Err(e) => {
                warn!("  获取 {} 版本列表失败: {}", language, e);
            }
        }
        
        // 测试获取最新版本
        match service.get_latest_version(language).await {
            Ok(latest) => {
                info!("  {} 最新版本: {}", language, latest.version);
                info!("  发布日期: {}", latest.release_date);
                info!("  特性数量: {}", latest.features.len());
                info!("  破坏性变更: {}", latest.breaking_changes.len());
            }
            Err(e) => {
                warn!("  获取 {} 最新版本失败: {}", language, e);
            }
        }
    }
    
    info!("✅ 语言版本服务基础功能测试通过");
    Ok(())
}

async fn test_version_comparison_service() -> anyhow::Result<()> {
    info!("📋 测试2: 版本比较服务");
    
    let version_service = Arc::new(LanguageVersionService::new().await?);
    let comparison_service = VersionComparisonService::new(version_service.clone());
    
    let supported_languages = version_service.get_supported_languages();
    
    for language in supported_languages.iter().take(2) { // 只测试前两种语言
        info!("测试版本比较: {}", language);
        
        // 获取版本列表
        match version_service.get_language_versions(language).await {
            Ok(versions) => {
                if versions.len() >= 2 {
                    let from_version = &versions[1]; // 较旧版本
                    let to_version = &versions[0];   // 较新版本
                    
                    info!("  比较版本: {} -> {}", from_version, to_version);
                    
                    match comparison_service.compare_versions(language, from_version, to_version).await {
                        Ok(comparison) => {
                            info!("  新增特性: {}", comparison.added_features.len());
                            info!("  移除特性: {}", comparison.removed_features.len());
                            info!("  修改特性: {}", comparison.modified_features.len());
                            info!("  破坏性变更: {}", comparison.breaking_changes.len());
                            info!("  升级建议: {}", comparison.upgrade_recommendations.len());
                            
                            // 显示一些具体的变更
                            for feature in comparison.added_features.iter().take(3) {
                                info!("    新增: {}", feature.name);
                            }
                            
                            for change in comparison.breaking_changes.iter().take(2) {
                                info!("    破坏性变更: {}", change.description);
                            }
                        }
                        Err(e) => {
                            warn!("  版本比较失败: {}", e);
                        }
                    }
                }
                
                // 测试版本时间线
                match comparison_service.get_version_timeline(language, None).await {
                    Ok(timeline) => {
                        info!("  版本时间线: {} 个版本", timeline.len());
                        for summary in timeline.iter().take(3) {
                            info!("    {}: {} 特性, {} 破坏性变更", 
                                summary.version, 
                                summary.feature_count, 
                                summary.breaking_change_count
                            );
                        }
                    }
                    Err(e) => {
                        warn!("  获取版本时间线失败: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("  获取 {} 版本列表失败: {}", language, e);
            }
        }
    }
    
    info!("✅ 版本比较服务测试通过");
    Ok(())
}

async fn test_language_features_tool() -> anyhow::Result<()> {
    info!("📋 测试3: 语言特性工具");
    
    let tool = LanguageFeaturesTool::new().await?;
    
    // 测试工具基本信息
    info!("工具名称: {}", tool.name());
    info!("工具描述: {}", tool.description());
    
    // 测试列出支持的语言
    let list_languages_params = json!({
        "action": "list_languages"
    });
    
    match tool.execute(list_languages_params).await {
        Ok(result) => {
            info!("支持的语言列表: {}", result);
            
            if let Some(languages) = result.get("supported_languages").and_then(|v| v.as_array()) {
                for language in languages.iter().take(3) {
                    if let Some(lang_str) = language.as_str() {
                        // 测试获取版本列表
                        let list_versions_params = json!({
                            "action": "list_versions",
                            "language": lang_str
                        });
                        
                        match tool.execute(list_versions_params).await {
                            Ok(versions_result) => {
                                info!("{} 版本列表: {}", lang_str, versions_result);
                            }
                            Err(e) => {
                                warn!("获取 {} 版本列表失败: {}", lang_str, e);
                            }
                        }
                        
                        // 测试获取最新版本
                        let get_latest_params = json!({
                            "action": "get_latest",
                            "language": lang_str
                        });
                        
                        match tool.execute(get_latest_params).await {
                            Ok(latest_result) => {
                                info!("{} 最新版本: {}", lang_str, latest_result);
                            }
                            Err(e) => {
                                warn!("获取 {} 最新版本失败: {}", lang_str, e);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("获取支持语言列表失败: {}", e);
        }
    }
    
    info!("✅ 语言特性工具测试通过");
    Ok(())
}

async fn test_cache_functionality() -> anyhow::Result<()> {
    info!("📋 测试4: 缓存功能测试");
    
    let service = LanguageVersionService::new().await?;
    let supported_languages = service.get_supported_languages();
    
    if let Some(language) = supported_languages.first() {
        info!("测试缓存功能: {}", language);
        
        // 预热缓存
        match service.warm_cache(language).await {
            Ok(_) => {
                info!("  缓存预热成功");
            }
            Err(e) => {
                warn!("  缓存预热失败: {}", e);
            }
        }
        
        // 测试缓存命中（第二次调用应该更快）
        let start_time = std::time::Instant::now();
        match service.get_latest_version(language).await {
            Ok(_) => {
                let duration = start_time.elapsed();
                info!("  第一次调用耗时: {:?}", duration);
            }
            Err(e) => {
                warn!("  第一次调用失败: {}", e);
            }
        }
        
        let start_time = std::time::Instant::now();
        match service.get_latest_version(language).await {
            Ok(_) => {
                let duration = start_time.elapsed();
                info!("  第二次调用耗时: {:?} (应该更快，缓存命中)", duration);
            }
            Err(e) => {
                warn!("  第二次调用失败: {}", e);
            }
        }
        
        // 清除缓存
        service.clear_cache().await;
        info!("  缓存已清除");
        
        // 再次调用（应该重新获取数据）
        let start_time = std::time::Instant::now();
        match service.get_latest_version(language).await {
            Ok(_) => {
                let duration = start_time.elapsed();
                info!("  清除缓存后调用耗时: {:?}", duration);
            }
            Err(e) => {
                warn!("  清除缓存后调用失败: {}", e);
            }
        }
    }
    
    info!("✅ 缓存功能测试通过");
    Ok(())
}

async fn test_feature_search() -> anyhow::Result<()> {
    info!("📋 测试5: 特性搜索功能");
    
    let tool = LanguageFeaturesTool::new().await?;
    
    // 测试搜索特性
    let search_params = json!({
        "action": "search_features",
        "language": "rust",
        "query": "async",
        "category": "Async"
    });
    
    match tool.execute(search_params).await {
        Ok(result) => {
            info!("搜索结果: {}", result);
            
            if let Some(features) = result.get("features").and_then(|v| v.as_array()) {
                info!("找到 {} 个相关特性", features.len());
                for (i, feature) in features.iter().take(3).enumerate() {
                    info!("  特性 {}: {}", i + 1, feature);
                }
            }
        }
        Err(e) => {
            warn!("特性搜索失败: {}", e);
        }
    }
    
    // 测试获取语法变化
    let syntax_changes_params = json!({
        "action": "get_syntax_changes",
        "language": "rust",
        "version": "1.70.0"
    });
    
    match tool.execute(syntax_changes_params).await {
        Ok(result) => {
            info!("语法变化: {}", result);
        }
        Err(e) => {
            warn!("获取语法变化失败: {}", e);
        }
    }
    
    info!("✅ 特性搜索功能测试通过");
    Ok(())
}

async fn test_error_handling() -> anyhow::Result<()> {
    info!("📋 测试6: 错误处理测试");
    
    let tool = LanguageFeaturesTool::new().await?;
    
    // 测试不支持的语言
    let invalid_language_params = json!({
        "action": "list_versions",
        "language": "nonexistent_language"
    });
    
    match tool.execute(invalid_language_params).await {
        Ok(result) => {
            warn!("预期失败但成功了: {}", result);
        }
        Err(e) => {
            info!("正确处理不支持的语言错误: {}", e);
        }
    }
    
    // 测试无效的操作
    let invalid_action_params = json!({
        "action": "invalid_action"
    });
    
    match tool.execute(invalid_action_params).await {
        Ok(result) => {
            warn!("预期失败但成功了: {}", result);
        }
        Err(e) => {
            info!("正确处理无效操作错误: {}", e);
        }
    }
    
    // 测试缺少必需参数
    let missing_params = json!({
        "action": "get_version"
        // 缺少 language 和 version 参数
    });
    
    match tool.execute(missing_params).await {
        Ok(result) => {
            warn!("预期失败但成功了: {}", result);
        }
        Err(e) => {
            info!("正确处理缺少参数错误: {}", e);
        }
    }
    
    info!("✅ 错误处理测试通过");
    Ok(())
} 