use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};
use chrono::{DateTime, Utc, Duration as ChronoDuration};

use super::data_models::*;
use super::collectors::{LanguageVersionCollector, CollectorFactory};
use super::enhanced_collectors::EnhancedCollectorFactory;

/// 语言版本服务
pub struct LanguageVersionService {
    collectors: Arc<RwLock<HashMap<String, Box<dyn LanguageVersionCollector>>>>,
    cache: Arc<RwLock<HashMap<String, CachedVersionData>>>,
    config: ServiceConfig,
}

/// 服务配置
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub use_enhanced_collectors: bool,
    pub cache_ttl_minutes: i64,
    pub max_cache_entries: usize,
    pub enable_fallback: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            use_enhanced_collectors: true, // 默认使用增强采集器
            cache_ttl_minutes: 60, // 1小时缓存
            max_cache_entries: 1000,
            enable_fallback: true,
        }
    }
}

/// 缓存的版本数据
#[derive(Debug, Clone)]
struct CachedVersionData {
    versions: Vec<String>,
    latest_version: Option<LanguageVersion>,
    cached_at: DateTime<Utc>,
    ttl_minutes: i64,
}

impl CachedVersionData {
    fn new(versions: Vec<String>, latest_version: Option<LanguageVersion>, ttl_minutes: i64) -> Self {
        Self {
            versions,
            latest_version,
            cached_at: Utc::now(),
            ttl_minutes,
        }
    }
    
    fn is_expired(&self) -> bool {
        let expiry = self.cached_at + ChronoDuration::minutes(self.ttl_minutes);
        Utc::now() > expiry
    }
}

impl LanguageVersionService {
    pub async fn new() -> Result<Self> {
        Self::with_config(ServiceConfig::default()).await
    }
    
    pub async fn with_config(config: ServiceConfig) -> Result<Self> {
        let service = Self {
            collectors: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        };
        
        // 初始化支持的语言采集器
        service.initialize_collectors().await?;
        
        Ok(service)
    }
    
    async fn initialize_collectors(&self) -> Result<()> {
        let mut collectors = self.collectors.write().await;
        
        let supported_languages = if self.config.use_enhanced_collectors {
            EnhancedCollectorFactory::supported_languages()
        } else {
            CollectorFactory::supported_languages()
        };
        
        for language in supported_languages {
            match self.create_collector(language) {
                Ok(collector) => {
                    info!("✅ 初始化语言采集器: {}", language);
                    collectors.insert(language.to_string(), collector);
                }
                Err(e) => {
                    warn!("❌ 初始化语言采集器失败 {}: {}", language, e);
                }
            }
        }
        
        info!("🎯 成功初始化 {} 个语言采集器", collectors.len());
        Ok(())
    }
    
    fn create_collector(&self, language: &str) -> Result<Box<dyn LanguageVersionCollector>> {
        if self.config.use_enhanced_collectors {
            EnhancedCollectorFactory::create_collector(language)
        } else {
            CollectorFactory::create_collector(language)
        }
    }
    
    pub fn get_supported_languages(&self) -> Vec<String> {
        if self.config.use_enhanced_collectors {
            EnhancedCollectorFactory::supported_languages()
                .into_iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            CollectorFactory::supported_languages()
                .into_iter()
                .map(|s| s.to_string())
                .collect()
        }
    }
    
    /// 获取语言版本列表（带缓存）
    pub async fn get_language_versions(&self, language: &str) -> Result<Vec<String>> {
        // 检查缓存
        if let Some(cached_data) = self.get_cached_data(language).await {
            if !cached_data.is_expired() {
                debug!("🎯 缓存命中: {} 版本列表", language);
                return Ok(cached_data.versions);
            }
        }
        
        // 缓存未命中或已过期，从采集器获取
        let collectors = self.collectors.read().await;
        if let Some(collector) = collectors.get(language) {
            match collector.get_versions().await {
                Ok(versions) => {
                    debug!("📦 从采集器获取 {} 版本: {} 个", language, versions.len());
                    
                    // 更新缓存
                    self.update_cache(language, versions.clone(), None).await;
                    
                    Ok(versions)
                }
                Err(e) => {
                    error!("❌ 获取 {} 版本失败: {}", language, e);
                    
                    // 如果启用了fallback，尝试返回过期的缓存数据
                    if self.config.enable_fallback {
                        if let Some(cached_data) = self.get_cached_data(language).await {
                            warn!("🔄 使用过期缓存数据: {}", language);
                            return Ok(cached_data.versions);
                        }
                    }
                    
                    Err(e)
                }
            }
        } else {
            Err(anyhow::anyhow!("不支持的语言: {}", language))
        }
    }
    
    /// 获取最新版本（带缓存）
    pub async fn get_latest_version(&self, language: &str) -> Result<LanguageVersion> {
        // 检查缓存
        if let Some(cached_data) = self.get_cached_data(language).await {
            if !cached_data.is_expired() {
                if let Some(latest) = cached_data.latest_version {
                    debug!("🎯 缓存命中: {} 最新版本", language);
                    return Ok(latest);
                }
            }
        }
        
        // 缓存未命中或已过期，从采集器获取
        let collectors = self.collectors.read().await;
        if let Some(collector) = collectors.get(language) {
            match collector.get_latest_version().await {
                Ok(latest_version) => {
                    debug!("📦 从采集器获取 {} 最新版本: {}", language, latest_version.version);
                    
                    // 更新缓存
                    self.update_cache(language, vec![], Some(latest_version.clone())).await;
                    
                    Ok(latest_version)
                }
                Err(e) => {
                    error!("❌ 获取 {} 最新版本失败: {}", language, e);
                    
                    // 如果启用了fallback，尝试返回过期的缓存数据
                    if self.config.enable_fallback {
                        if let Some(cached_data) = self.get_cached_data(language).await {
                            if let Some(latest) = cached_data.latest_version {
                                warn!("🔄 使用过期缓存数据: {} 最新版本", language);
                                return Ok(latest);
                            }
                        }
                    }
                    
                    Err(e)
                }
            }
        } else {
            Err(anyhow::anyhow!("不支持的语言: {}", language))
        }
    }
    
    /// 获取特定版本详情
    pub async fn get_version_details(&self, language: &str, version: &str) -> Result<LanguageVersion> {
        let collectors = self.collectors.read().await;
        if let Some(collector) = collectors.get(language) {
            collector.get_version_details(version).await
        } else {
            Err(anyhow::anyhow!("不支持的语言: {}", language))
        }
    }
    
    /// 检查版本是否支持
    pub async fn is_version_supported(&self, language: &str, version: &str) -> bool {
        let collectors = self.collectors.read().await;
        if let Some(collector) = collectors.get(language) {
            collector.is_version_supported(version).await
        } else {
            false
        }
    }
    
    /// 预热缓存
    pub async fn warm_cache(&self, language: &str) -> Result<()> {
        info!("🔥 开始预热缓存: {}", language);
        
        // 预热版本列表
        if let Ok(versions) = self.get_language_versions(language).await {
            info!("🔥 预热完成: {} ({} 个版本)", language, versions.len());
        }
        
        // 预热最新版本
        if let Ok(_) = self.get_latest_version(language).await {
            debug!("🔥 预热最新版本完成: {}", language);
        }
        
        Ok(())
    }
    
    /// 清除缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("🧹 清除版本信息缓存");
    }
    
    /// 清除特定语言的缓存
    pub async fn clear_language_cache(&self, language: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(language);
        info!("🧹 清除 {} 版本信息缓存", language);
    }
    
    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|data| data.is_expired()).count();
        
        CacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
            cache_hit_rate: 0.0, // 需要实际统计
        }
    }
    
    /// 搜索特性
    pub async fn search_features(
        &self,
        language: &str,
        query: &str,
        category: Option<FeatureCategory>,
        version: Option<&str>,
    ) -> Result<Vec<LanguageFeature>> {
        let version_to_search = if let Some(v) = version {
            v.to_string()
        } else {
            // 使用最新版本
            let latest = self.get_latest_version(language).await?;
            latest.version
        };
        
        let version_details = self.get_version_details(language, &version_to_search).await?;
        
        let mut matching_features = Vec::new();
        let query_lower = query.to_lowercase();
        
        for feature in version_details.features {
            // 检查查询匹配
            let name_match = feature.name.to_lowercase().contains(&query_lower);
            let desc_match = feature.description.to_lowercase().contains(&query_lower);
            let tag_match = feature.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));
            
            if name_match || desc_match || tag_match {
                // 检查分类匹配
                if let Some(ref cat) = category {
                    if std::mem::discriminant(&feature.category) == std::mem::discriminant(cat) {
                        matching_features.push(feature);
                    }
                } else {
                    matching_features.push(feature);
                }
            }
        }
        
        info!("🔍 搜索到 {} 个匹配特性", matching_features.len());
        Ok(matching_features)
    }
    
    /// 获取语法变化
    pub async fn get_syntax_changes(
        &self,
        language: &str,
        version: &str,
    ) -> Result<Vec<SyntaxChange>> {
        let version_details = self.get_version_details(language, version).await?;
        Ok(version_details.syntax_changes)
    }
    
    /// 获取破坏性变更
    pub async fn get_breaking_changes(
        &self,
        language: &str,
        version: &str,
    ) -> Result<Vec<BreakingChange>> {
        let version_details = self.get_version_details(language, version).await?;
        Ok(version_details.breaking_changes)
    }
    
    // 私有辅助方法
    async fn get_cached_data(&self, language: &str) -> Option<CachedVersionData> {
        let cache = self.cache.read().await;
        cache.get(language).cloned()
    }
    
    async fn update_cache(&self, language: &str, versions: Vec<String>, latest_version: Option<LanguageVersion>) {
        let mut cache = self.cache.write().await;
        
        // 检查缓存大小限制
        if cache.len() >= self.config.max_cache_entries {
            // 移除最旧的条目
            if let Some((oldest_key, _)) = cache.iter()
                .min_by_key(|(_, data)| data.cached_at)
                .map(|(k, v)| (k.clone(), v.clone()))
            {
                cache.remove(&oldest_key);
            }
        }
        
        // 更新或插入缓存条目
        let cached_data = if let Some(existing) = cache.get_mut(language) {
            if !versions.is_empty() {
                existing.versions = versions;
            }
            if let Some(latest) = latest_version {
                existing.latest_version = Some(latest);
            }
            existing.cached_at = Utc::now();
            existing.clone()
        } else {
            let new_data = CachedVersionData::new(versions, latest_version, self.config.cache_ttl_minutes);
            cache.insert(language.to_string(), new_data.clone());
            new_data
        };
        
        debug!("💾 更新缓存: {} (版本数: {})", language, cached_data.versions.len());
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub cache_hit_rate: f64,
}

/// 版本比较服务
pub struct VersionComparisonService {
    version_service: Arc<LanguageVersionService>,
}

impl VersionComparisonService {
    pub fn new(version_service: Arc<LanguageVersionService>) -> Self {
        Self { version_service }
    }
    
    /// 比较两个版本的差异
    pub async fn compare_versions(
        &self,
        language: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<VersionComparison> {
        info!("🔄 比较版本: {} {} -> {}", language, from_version, to_version);
        
        let from_details = self.version_service.get_version_details(language, from_version).await?;
        let to_details = self.version_service.get_version_details(language, to_version).await?;
        
        let mut comparison = VersionComparison {
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            language: language.to_string(),
            added_features: Vec::new(),
            removed_features: Vec::new(),
            modified_features: Vec::new(),
            breaking_changes: to_details.breaking_changes.clone(),
            deprecations: to_details.deprecations.clone(),
            upgrade_recommendations: Vec::new(),
        };
        
        // 查找新增特性
        let from_feature_names: std::collections::HashSet<_> = 
            from_details.features.iter().map(|f| &f.name).collect();
            
        for feature in &to_details.features {
            if !from_feature_names.contains(&feature.name) {
                comparison.added_features.push(feature.clone());
            }
        }
        
        // 查找移除的特性
        let to_feature_names: std::collections::HashSet<_> = 
            to_details.features.iter().map(|f| &f.name).collect();
            
        for feature in &from_details.features {
            if !to_feature_names.contains(&feature.name) {
                comparison.removed_features.push(feature.name.clone());
            }
        }
        
        // 生成升级建议
        comparison.upgrade_recommendations = self.generate_upgrade_recommendations(&comparison);
        
        info!("✅ 版本比较完成: +{} -{} 特性", 
              comparison.added_features.len(), 
              comparison.removed_features.len());
        
        Ok(comparison)
    }
    
    /// 获取版本演进历史
    pub async fn get_version_timeline(
        &self,
        language: &str,
        since_version: Option<&str>,
    ) -> Result<Vec<VersionSummary>> {
        let versions = self.version_service.get_language_versions(language).await?;
        let mut timeline = Vec::new();
        
        for version in versions {
            // 如果指定了起始版本，则跳过更早的版本
            if let Some(since) = since_version {
                if version < since.to_string() {
                    continue;
                }
            }
            
            match self.version_service.get_version_details(language, &version).await {
                Ok(details) => {
                    timeline.push(VersionSummary {
                        version: details.version,
                        release_date: details.release_date,
                        is_stable: details.is_stable,
                        feature_count: details.features.len(),
                        breaking_change_count: details.breaking_changes.len(),
                        major_features: details.features.iter()
                            .filter(|f| f.impact == ImpactLevel::High)
                            .map(|f| f.name.clone())
                            .collect(),
                    });
                }
                Err(e) => {
                    warn!("⚠️ 跳过版本 {}: {}", version, e);
                }
            }
        }
        
        // 按发布日期倒序排列
        timeline.sort_by(|a, b| b.release_date.cmp(&a.release_date));
        
        Ok(timeline)
    }
    
    fn generate_upgrade_recommendations(&self, comparison: &VersionComparison) -> Vec<UpgradeRecommendation> {
        let mut recommendations = Vec::new();
        
        // 基于破坏性变更生成建议
        if !comparison.breaking_changes.is_empty() {
            recommendations.push(UpgradeRecommendation {
                title: "注意破坏性变更".to_string(),
                description: format!("此版本包含 {} 个破坏性变更，请仔细审查您的代码", 
                                   comparison.breaking_changes.len()),
                priority: RecommendationPriority::Critical,
                links: vec![],
            });
        }
        
        // 基于弃用生成建议
        if !comparison.deprecations.is_empty() {
            recommendations.push(UpgradeRecommendation {
                title: "更新弃用的功能".to_string(),
                description: format!("有 {} 个功能已被弃用，建议尽快替换", 
                                   comparison.deprecations.len()),
                priority: RecommendationPriority::High,
                links: vec![],
            });
        }
        
        // 基于新特性生成建议
        if comparison.added_features.len() > 5 {
            recommendations.push(UpgradeRecommendation {
                title: "探索新特性".to_string(),
                description: format!("此版本新增了 {} 个特性，可能对您的项目有帮助", 
                                   comparison.added_features.len()),
                priority: RecommendationPriority::Medium,
                links: vec![],
            });
        }
        
        recommendations
    }
}

/// 版本摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionSummary {
    pub version: String,
    pub release_date: chrono::DateTime<chrono::Utc>,
    pub is_stable: bool,
    pub feature_count: usize,
    pub breaking_change_count: usize,
    pub major_features: Vec<String>,
} 