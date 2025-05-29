use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use super::data_models::*;
use super::collectors::{LanguageVersionCollector, CollectorFactory};

/// 语言版本服务
pub struct LanguageVersionService {
    collectors: HashMap<String, Box<dyn LanguageVersionCollector>>,
    cache: Arc<RwLock<HashMap<String, LanguageVersion>>>,
}

impl LanguageVersionService {
    pub async fn new() -> Result<Self> {
        let collectors = HashMap::new();
        let mut service = Self {
            collectors,
            cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // 初始化支持的语言采集器
        for language in CollectorFactory::supported_languages() {
            match CollectorFactory::create_collector(language) {
                Ok(collector) => {
                    info!("✅ 初始化语言采集器: {}", language);
                    service.collectors.insert(language.to_string(), collector);
                }
                Err(e) => {
                    warn!("⚠️ 初始化语言采集器失败 {}: {}", language, e);
                }
            }
        }
        
        Ok(service)
    }
    
    /// 获取支持的语言列表
    pub fn get_supported_languages(&self) -> Vec<String> {
        self.collectors.keys().cloned().collect()
    }
    
    /// 获取语言的所有版本
    pub async fn get_language_versions(&self, language: &str) -> Result<Vec<String>> {
        let collector = self.collectors.get(language)
            .ok_or_else(|| anyhow::anyhow!("不支持的语言: {}", language))?;
            
        collector.get_versions().await
    }
    
    /// 获取特定版本的详细信息（带缓存）
    pub async fn get_version_details(&self, language: &str, version: &str) -> Result<LanguageVersion> {
        let cache_key = format!("{}:{}", language, version);
        
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_version) = cache.get(&cache_key) {
                debug!("🎯 从缓存获取版本信息: {}", cache_key);
                return Ok(cached_version.clone());
            }
        }
        
        // 缓存未命中，从采集器获取
        let collector = self.collectors.get(language)
            .ok_or_else(|| anyhow::anyhow!("不支持的语言: {}", language))?;
            
        info!("🔍 获取版本详情: {} {}", language, version);
        let version_details = collector.get_version_details(version).await?;
        
        // 更新缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, version_details.clone());
        }
        
        Ok(version_details)
    }
    
    /// 获取最新版本
    pub async fn get_latest_version(&self, language: &str) -> Result<LanguageVersion> {
        let collector = self.collectors.get(language)
            .ok_or_else(|| anyhow::anyhow!("不支持的语言: {}", language))?;
            
        collector.get_latest_version().await
    }
    
    /// 搜索特定特性
    pub async fn search_features(
        &self,
        language: &str,
        version: Option<&str>,
        query: &str,
        category: Option<FeatureCategory>,
    ) -> Result<Vec<LanguageFeature>> {
        let version_details = if let Some(v) = version {
            self.get_version_details(language, v).await?
        } else {
            self.get_latest_version(language).await?
        };
        
        let mut matching_features = Vec::new();
        let query_lower = query.to_lowercase();
        
        for feature in version_details.features {
            // 检查类别过滤
            if let Some(ref cat) = category {
                if std::mem::discriminant(&feature.category) != std::mem::discriminant(cat) {
                    continue;
                }
            }
            
            // 检查搜索匹配
            if feature.name.to_lowercase().contains(&query_lower) ||
               feature.description.to_lowercase().contains(&query_lower) ||
               feature.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower)) {
                matching_features.push(feature);
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
    
    /// 清除缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("🧹 清除版本信息缓存");
    }
    
    /// 预热缓存
    pub async fn warm_cache(&self, language: &str) -> Result<()> {
        info!("🔥 开始预热缓存: {}", language);
        
        let versions = self.get_language_versions(language).await?;
        let mut loaded_count = 0;
        
        // 只预热最近的几个版本，避免过度负载
        for version in versions.iter().take(5) {
            match self.get_version_details(language, version).await {
                Ok(_) => {
                    loaded_count += 1;
                    debug!("✅ 预热版本: {} {}", language, version);
                }
                Err(e) => {
                    warn!("⚠️ 预热失败 {} {}: {}", language, version, e);
                }
            }
        }
        
        info!("🔥 预热完成: {} ({} 个版本)", language, loaded_count);
        Ok(())
    }
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