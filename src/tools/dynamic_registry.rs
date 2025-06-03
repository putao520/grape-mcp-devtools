use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use std::sync::atomic::{AtomicU64, Ordering};

use super::base::MCPTool;
use super::environment_detector::{EnvironmentDetector, DetectionReport};
use super::enhanced_language_tool::{EnhancedLanguageTool, DocumentStrategy};
use super::vector_docs_tool::VectorDocsTool;
use crate::cli::tool_installer::{ToolInstaller, ToolInstallConfig};
use super::flutter_docs_tool::FlutterDocsTool;
use super::enhanced_doc_processor::EnhancedDocumentProcessor;

// 新增：缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub detection_cache_ttl_seconds: u64,
    pub tool_cache_ttl_seconds: u64,
    pub max_cache_entries: usize,
    pub enable_persistent_cache: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            detection_cache_ttl_seconds: 300, // 5分钟
            tool_cache_ttl_seconds: 1800,     // 30分钟
            max_cache_entries: 100,
            enable_persistent_cache: true,
        }
    }
}

// 新增：性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_registrations: u64,
    pub successful_registrations: u64,
    pub failed_registrations: u64,
    pub average_registration_time_ms: f64,
    pub cache_hit_rate: f64,
    pub last_scan_duration_ms: u64,
    pub tools_per_language: HashMap<String, u32>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_registrations: 0,
            successful_registrations: 0,
            failed_registrations: 0,
            average_registration_time_ms: 0.0,
            cache_hit_rate: 0.0,
            last_scan_duration_ms: 0,
            tools_per_language: HashMap::new(),
        }
    }
}

// 新增：缓存条目
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    timestamp: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            timestamp: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }
}

// 增强的注册报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationReport {
    pub registered_tools: Vec<String>,
    pub failed_registrations: Vec<(String, String)>, // (tool_name, error)
    pub registration_duration_ms: u64,
    pub total_detected_languages: usize,
    pub registration_score: f32,
    pub missing_tools_detected: HashMap<String, Vec<String>>, // language -> missing tools
    pub tool_installation_report: Option<crate::cli::tool_installer::InstallationReport>,
    pub auto_install_enabled: bool,
    // 新增字段
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub performance_metrics: PerformanceMetrics,
    pub retry_attempts: HashMap<String, u32>, // tool_name -> retry_count
}

// 增强的注册策略
#[derive(Debug, Clone)]
pub enum RegistrationPolicy {
    /// 基于项目文件检测
    ProjectBased { min_files: usize },
    /// 基于用户配置偏好
    UserPreference { preferred_languages: Vec<String> },
    /// 基于CLI工具可用性
    ToolAvailability { min_tools: usize },
    /// 自适应策略（综合评分）
    Adaptive { score_threshold: f32 },
    /// 激进策略（注册所有检测到的）
    Aggressive,
    /// 保守策略（只注册高置信度的）
    Conservative { score_threshold: f32 },
    /// 新增：智能策略（基于历史数据和使用模式）
    Intelligent { 
        base_threshold: f32,
        usage_weight: f32,
        performance_weight: f32,
    },
    /// 新增：条件策略（基于复杂条件）
    Conditional {
        conditions: Vec<RegistrationCondition>,
    },
}

// 新增：注册条件
#[derive(Debug, Clone)]
pub enum RegistrationCondition {
    MinProjectFiles(usize),
    RequiredCliTools(Vec<String>),
    MinScore(f32),
    LanguageInList(Vec<String>),
    FrameworkDetected(String),
    CustomPredicate(String), // 可以是一个表达式字符串
}

impl Default for RegistrationPolicy {
    fn default() -> Self {
        RegistrationPolicy::Adaptive { score_threshold: 0.5 }
    }
}

// 增强的动态工具注册中心
pub struct DynamicToolRegistry {
    detector: EnvironmentDetector,
    policy: RegistrationPolicy,
    registered_tools: HashMap<String, Arc<dyn MCPTool>>,
    language_tool_mapping: HashMap<String, String>, // language -> tool_name
    max_tools_per_language: usize,
    tool_installer: Option<ToolInstaller>,
    auto_install_tools: bool,
    
    // 新增：缓存系统
    cache_config: CacheConfig,
    detection_cache: Arc<RwLock<HashMap<String, CacheEntry<DetectionReport>>>>,
    tool_cache: Arc<RwLock<HashMap<String, CacheEntry<Arc<dyn MCPTool>>>>>,
    
    // 新增：性能监控
    metrics: Arc<RwLock<PerformanceMetrics>>,
    
    // 新增：错误恢复
    max_retry_attempts: u32,
    retry_delay_ms: u64,
    
    // 新增：配置管理
    config_path: Option<std::path::PathBuf>,
    shared_doc_processor: Option<Arc<EnhancedDocumentProcessor>>,
}

impl DynamicToolRegistry {
    pub fn new() -> Self {
        Self {
            detector: EnvironmentDetector::new(),
            policy: RegistrationPolicy::default(),
            registered_tools: HashMap::new(),
            language_tool_mapping: HashMap::new(),
            max_tools_per_language: 3,
            tool_installer: None,
            auto_install_tools: false,
            
            // 缓存系统
            cache_config: CacheConfig::default(),
            detection_cache: Arc::new(RwLock::new(HashMap::new())),
            tool_cache: Arc::new(RwLock::new(HashMap::new())),
            
            // 性能监控
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            
            // 错误恢复
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            
            // 配置管理
            config_path: None,
            shared_doc_processor: None,
        }
    }

    // 新增：配置构建器
    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    pub fn with_retry_config(mut self, max_attempts: u32, delay_ms: u64) -> Self {
        self.max_retry_attempts = max_attempts;
        self.retry_delay_ms = delay_ms;
        self
    }

    pub fn with_config_path(mut self, path: std::path::PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    pub fn enable_auto_install(&mut self, install_config: ToolInstallConfig) {
        self.tool_installer = Some(ToolInstaller::new(install_config));
        self.auto_install_tools = true;
        info!("✅ 启用工具自动安装功能");
    }

    pub fn disable_auto_install(&mut self) {
        self.tool_installer = None;
        self.auto_install_tools = false;
        info!("❌ 禁用工具自动安装功能");
    }

    // 修改 auto_register 的返回类型
    pub async fn auto_register(&mut self) -> Result<(RegistrationReport, Option<DetectionReport>)> {
        let start_time = Instant::now();
        info!("🚀 开始动态工具注册...");

        let mut cache_hits = 0;
        let mut cache_misses = 0;
        let mut retry_attempts = HashMap::new();

        // 尝试从缓存获取检测报告
        let detection_report = match self.get_cached_detection_report().await {
            Some(cached_report) => {
                cache_hits += 1;
                info!("📋 使用缓存的环境检测报告");
                cached_report
            }
            None => {
                cache_misses += 1;
                info!("🔍 执行新的环境检测...");
                let report = self.detector.scan_environment().await?;
                self.cache_detection_report(report.clone()).await;
                report
            }
        };

        let mut registered_tools_names = Vec::new();
        let mut failed_registrations = Vec::new();
        let missing_tools_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut tool_installation_report = None;

        // 如果启用了自动安装，则先检查并安装工具
        if self.auto_install_tools && self.tool_installer.is_some() {
            info!("⚙️ 自动工具安装已启用，开始检测并安装缺失工具...");
            let installer = self.tool_installer.as_ref().unwrap();
            let detected_language_names: Vec<String> = detection_report.detected_languages.keys().cloned().collect();
            match installer.detect_missing_tools(&detected_language_names).await {
                Ok(missing_tools) => {
                    if !missing_tools.is_empty() {
                        match installer.auto_install_tools(&missing_tools).await {
                            Ok(report) => {
                                info!("✅ 工具安装检查完成。");
                                tool_installation_report = Some(report);
                            }
                            Err(e) => {
                                warn!("⚠️ 工具安装过程中发生错误: {}", e);
                            }
                        }
                    } else {
                        info!("✅ 所有需要的工具都已安装。");
                    }
                }
                Err(e) => {
                    warn!("⚠️ 工具检测过程中发生错误: {}", e);
                }
            }
        }

        // 根据注册计划创建和注册工具
        match self.create_registration_plan(&detection_report) {
            Ok(plan) => {
                info!("📝 注册计划: {:?} 个工具待处理", plan.len());
                for (language, score) in plan {
                    match self.create_and_register_tool_with_retry(&language, score, &mut retry_attempts).await {
                        Ok(tool_name) => {
                            info!("✅ 成功注册并缓存工具: {}", tool_name);
                            registered_tools_names.push(tool_name);
                        }
                        Err(e) => {
                            warn!("❌ 注册工具 {} 失败 (最终): {}", language, e);
                            failed_registrations.push((language.to_string(), e.to_string()));
                        }
                    }
                }
            }
            Err(e) => {
                warn!("⚠️ 创建注册计划失败: {}", e);
                // 即使计划失败，也继续生成报告，但可能没有注册任何工具
            }
        }

        let registration_duration_ms = start_time.elapsed().as_millis() as u64;
        info!("🏁 动态工具注册完成，耗时: {}ms", registration_duration_ms);

        // 更新性能指标
        let mut metrics_guard = self.metrics.write().await;
        metrics_guard.total_registrations += (registered_tools_names.len() + failed_registrations.len()) as u64;
        metrics_guard.successful_registrations += registered_tools_names.len() as u64;
        metrics_guard.failed_registrations += failed_registrations.len() as u64;
        // 计算平均注册时间
        if metrics_guard.successful_registrations > 0 {
            metrics_guard.average_registration_time_ms = registration_duration_ms as f64 / metrics_guard.successful_registrations as f64;
        }
        // 计算缓存命中率
        let total_cache_operations = cache_hits + cache_misses;
        if total_cache_operations > 0 {
            metrics_guard.cache_hit_rate = cache_hits as f64 / total_cache_operations as f64;
        }
        metrics_guard.last_scan_duration_ms = registration_duration_ms; 
        drop(metrics_guard);

        // 计算综合评分（基于成功率、缓存效率、性能）
        let registration_score = if (registered_tools_names.len() + failed_registrations.len()) > 0 {
            let success_rate = registered_tools_names.len() as f32 / (registered_tools_names.len() + failed_registrations.len()) as f32;
            let cache_efficiency = if total_cache_operations > 0 {
                cache_hits as f32 / total_cache_operations as f32
            } else {
                1.0
            };
            let speed_factor = if registration_duration_ms > 0 {
                (5000.0 / registration_duration_ms as f32).min(1.0) // 5秒内完成得满分
            } else {
                1.0
            };
            (success_rate * 0.5 + cache_efficiency * 0.3 + speed_factor * 0.2) * 100.0
        } else {
            0.0
        };

        // 检测缺失工具
        let missing_tools_detected = self.detect_missing_tools_for_languages(&detection_report.detected_languages).await;

        let final_report = RegistrationReport {
            registered_tools: registered_tools_names,
            failed_registrations,
            registration_duration_ms,
            total_detected_languages: detection_report.detected_languages.len(),
            registration_score,
            missing_tools_detected,
            tool_installation_report,
            auto_install_enabled: self.auto_install_tools,
            cache_hits,
            cache_misses,
            performance_metrics: self.get_performance_metrics().await, // 获取最新指标
            retry_attempts,
        };

        Ok((final_report, Some(detection_report)))
    }

    // 实现缺失工具检测
    async fn detect_missing_tools_for_languages(&self, detected_languages: &HashMap<String, crate::tools::environment_detector::LanguageInfo>) -> HashMap<String, Vec<String>> {
        let mut missing_tools = HashMap::new();
        
        for (language, info) in detected_languages {
            let mut language_missing_tools = Vec::new();
            
            // 检查每种语言推荐的工具
            match language.as_str() {
                "rust" => {
                    if !info.cli_tools.iter().any(|t| t.name == "cargo") {
                        language_missing_tools.push("cargo".to_string());
                    }
                    if !info.cli_tools.iter().any(|t| t.name == "rustdoc") {
                        language_missing_tools.push("rustdoc".to_string());
                    }
                }
                "python" => {
                    if !info.cli_tools.iter().any(|t| t.name == "pip") {
                        language_missing_tools.push("pip".to_string());
                    }
                    if !info.cli_tools.iter().any(|t| t.name == "python") {
                        language_missing_tools.push("python".to_string());
                    }
                }
                "javascript" | "typescript" => {
                    if !info.cli_tools.iter().any(|t| t.name == "npm") {
                        language_missing_tools.push("npm".to_string());
                    }
                    if !info.cli_tools.iter().any(|t| t.name == "node") {
                        language_missing_tools.push("node".to_string());
                    }
                }
                "java" => {
                    if !info.cli_tools.iter().any(|t| t.name == "mvn") {
                        language_missing_tools.push("maven".to_string());
                    }
                    if !info.cli_tools.iter().any(|t| t.name == "gradle") {
                        language_missing_tools.push("gradle".to_string());
                    }
                }
                _ => {
                    // 对于其他语言，检查是否有基本的编译器/解释器
                    if info.cli_tools.is_empty() {
                        language_missing_tools.push(format!("{}-compiler", language));
                    }
                }
            }
            
            if !language_missing_tools.is_empty() {
                missing_tools.insert(language.clone(), language_missing_tools);
            }
        }
        
        missing_tools
    }

    // 新增：带重试的工具创建和注册
    async fn create_and_register_tool_with_retry(&mut self, language: &str, score: f32, retry_attempts: &mut HashMap<String, u32>) -> Result<String> {
        let mut last_error = None;
        
        for attempt in 1..=self.max_retry_attempts {
            match self.create_and_register_tool(language, score).await {
                Ok(tool_name) => {
                    if attempt > 1 {
                        info!("✅ 重试成功: {} (第{}次尝试)", language, attempt);
                    }
                    return Ok(tool_name);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retry_attempts {
                        warn!("⚠️ 注册失败，将重试: {} (第{}次尝试)", language, attempt);
                        tokio::time::sleep(Duration::from_millis(self.retry_delay_ms * attempt as u64)).await;
                    }
                }
            }
        }
        
        if let Some(error) = last_error {
            retry_attempts.insert(language.to_string(), retry_attempts.get(language).cloned().unwrap_or(0) + 1);
            Err(error)
        } else {
            // 修复：如果执行到这里说明所有重试都没有记录到错误，这是异常情况
            error!("🚨 异常情况：重试循环完成但没有记录任何错误，语言: {}", language);
            Err(anyhow::anyhow!("工具注册失败：重试过程中未能成功注册 {} 工具，且未记录到具体错误信息", language))
        }
    }

    // 新增：缓存管理方法
    async fn get_cached_detection_report(&self) -> Option<DetectionReport> {
        let cache = self.detection_cache.read().await;
        let cache_key = "environment_detection".to_string();
        
        if let Some(entry) = cache.get(&cache_key) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    async fn cache_detection_report(&self, report: DetectionReport) {
        let mut cache = self.detection_cache.write().await;
        let cache_key = "environment_detection".to_string();
        let ttl = Duration::from_secs(self.cache_config.detection_cache_ttl_seconds);
        
        cache.insert(cache_key, CacheEntry::new(report, ttl));
        
        // 清理过期缓存
        cache.retain(|_, entry| !entry.is_expired());
    }

    // 新增：清理缓存
    pub async fn clear_cache(&self) {
        let mut detection_cache = self.detection_cache.write().await;
        let mut tool_cache = self.tool_cache.write().await;
        
        detection_cache.clear();
        tool_cache.clear();
        
        info!("🧹 缓存已清理");
    }

    // 新增：获取性能指标
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    // 新增：保存配置
    pub async fn save_config(&self) -> Result<()> {
        if let Some(config_path) = &self.config_path {
            let config = serde_json::json!({
                "policy": format!("{:?}", self.policy),
                "cache_config": self.cache_config,
                "max_retry_attempts": self.max_retry_attempts,
                "retry_delay_ms": self.retry_delay_ms,
                "auto_install_tools": self.auto_install_tools,
                "max_tools_per_language": self.max_tools_per_language,
            });
            
            tokio::fs::write(config_path, serde_json::to_string_pretty(&config)?).await?;
            debug!("💾 配置已保存到: {:?}", config_path);
        }
        Ok(())
    }

    // 新增：加载配置
    pub async fn load_config(&mut self) -> Result<()> {
        if let Some(config_path) = &self.config_path {
            if config_path.exists() {
                let content = tokio::fs::read_to_string(config_path).await?;
                let config: serde_json::Value = serde_json::from_str(&content)?;
                
                if let Some(cache_config) = config.get("cache_config") {
                    self.cache_config = serde_json::from_value(cache_config.clone())?;
                }
                
                if let Some(max_retry) = config.get("max_retry_attempts") {
                    self.max_retry_attempts = max_retry.as_u64().unwrap_or(3) as u32;
                }
                
                if let Some(retry_delay) = config.get("retry_delay_ms") {
                    self.retry_delay_ms = retry_delay.as_u64().unwrap_or(1000);
                }
                
                info!("📖 配置已从文件加载: {:?}", config_path);
            }
        }
        Ok(())
    }

    pub async fn check_and_upgrade_tools(&self) -> Result<crate::cli::tool_installer::UpgradeReport> {
        if let Some(installer) = &self.tool_installer {
            let mut detected_tools = HashMap::new();
            
            for tool_info in installer.get_supported_tools().values() {
                let cli_tool_info = crate::cli::detector::CliToolInfo {
                    name: tool_info.tool_name.clone(),
                    version: None,
                    path: None,
                    available: installer.is_tool_installed(&tool_info.check_command).await,
                    features: vec![],
                };
                detected_tools.insert(tool_info.tool_name.clone(), cli_tool_info);
            }

            installer.upgrade_tools(&detected_tools).await
        } else {
            info!("⚠️ 工具安装器未启用，跳过升级检查");
            Ok(crate::cli::tool_installer::UpgradeReport {
                upgraded: vec![],
                failed: vec![],
                available: vec![],
            })
        }
    }

    fn create_registration_plan(&self, report: &DetectionReport) -> Result<Vec<(String, f32)>> {
        let mut plan = Vec::new();
        
        for (language, info) in &report.detected_languages {
            let should_register = match &self.policy {
                RegistrationPolicy::ProjectBased { min_files } => {
                    info.project_files.len() >= *min_files
                }
                RegistrationPolicy::UserPreference { preferred_languages } => {
                    preferred_languages.contains(language)
                }
                RegistrationPolicy::ToolAvailability { min_tools } => {
                    info.cli_tools.iter().filter(|t| t.available).count() >= *min_tools
                }
                RegistrationPolicy::Adaptive { score_threshold } => {
                    info.score >= *score_threshold
                }
                RegistrationPolicy::Aggressive => true,
                RegistrationPolicy::Conservative { score_threshold } => {
                    info.score >= *score_threshold && info.project_files.len() > 0
                }
                RegistrationPolicy::Intelligent { base_threshold, usage_weight: _, performance_weight: _ } => {
                    // 智能策略：结合基础阈值和历史数据
                    info.score >= *base_threshold && info.project_files.len() > 0
                }
                RegistrationPolicy::Conditional { conditions } => {
                    // 条件策略：所有条件都必须满足
                    conditions.iter().all(|condition| {
                        self.evaluate_condition(condition, language, info)
                    })
                }
            };
            
            if should_register {
                plan.push((language.clone(), info.score));
                debug!("📝 计划注册: {} (评分: {:.2})", language, info.score);
            } else {
                debug!("⏭️ 跳过注册: {} (评分: {:.2})", language, info.score);
            }
        }
        
        // 按评分排序，优先注册高评分的工具
        plan.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(plan)
    }

    // 新增：评估注册条件
    fn evaluate_condition(&self, condition: &RegistrationCondition, language: &str, info: &super::environment_detector::LanguageInfo) -> bool {
        match condition {
            RegistrationCondition::MinProjectFiles(min_files) => {
                info.project_files.len() >= *min_files
            }
            RegistrationCondition::RequiredCliTools(required_tools) => {
                required_tools.iter().all(|required_tool| {
                    info.cli_tools.iter().any(|tool| tool.name == *required_tool && tool.available)
                })
            }
            RegistrationCondition::MinScore(min_score) => {
                info.score >= *min_score
            }
            RegistrationCondition::LanguageInList(languages) => {
                languages.contains(&language.to_string())
            }
            RegistrationCondition::FrameworkDetected(framework) => {
                info.detected_features.iter().any(|feature| feature.contains(framework)) ||
                info.cli_tools.iter().any(|tool| tool.name.contains(framework))
            }
            RegistrationCondition::CustomPredicate(predicate) => {
                // 实现表达式解析器
                self.evaluate_custom_predicate(predicate, language, info)
            }
        }
    }

    async fn create_and_register_tool(&mut self, language: &str, score: f32) -> Result<String> {
        let tool_name = format!("enhanced_{}_docs", language);
        
        // 尝试从缓存获取工具
        if let Some(cached_tool) = self.get_cached_tool(&tool_name).await {
            info!("📋 使用缓存的工具: {}", tool_name);
            self.registered_tools.insert(tool_name.clone(), cached_tool);
            self.language_tool_mapping.insert(language.to_string(), tool_name.clone());
            return Ok(tool_name);
        }
        
        let tool = self.create_language_tool(language, score).await?;
        
        // 缓存工具
        self.cache_tool(&tool_name, Arc::clone(&tool)).await;
        
        self.registered_tools.insert(tool_name.clone(), tool);
        self.language_tool_mapping.insert(language.to_string(), tool_name.clone());
        
        Ok(tool_name)
    }

    // 新增：缓存工具
    async fn get_cached_tool(&self, tool_name: &str) -> Option<Arc<dyn MCPTool>> {
        let cache = self.tool_cache.read().await;
        
        if let Some(entry) = cache.get(tool_name) {
            if !entry.is_expired() {
                return Some(Arc::clone(&entry.data));
            }
        }
        None
    }

    async fn cache_tool(&self, tool_name: &str, tool: Arc<dyn MCPTool>) {
        let mut cache = self.tool_cache.write().await;
        let ttl = Duration::from_secs(self.cache_config.tool_cache_ttl_seconds);
        
        cache.insert(tool_name.to_string(), CacheEntry::new(tool, ttl));
        
        // 清理过期缓存
        cache.retain(|_, entry| !entry.is_expired());
        
        // 限制缓存大小
        if cache.len() > self.cache_config.max_cache_entries {
            // 移除最旧的条目
            let oldest_key = cache.iter()
                .min_by_key(|(_, entry)| entry.timestamp)
                .map(|(key, _)| key.clone());
            
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }
    }

    async fn create_language_tool(&self, language: &str, _score: f32) -> Result<Arc<dyn MCPTool>> {
        match language {
            "rust" | "python" | "javascript" | "typescript" | "java" | "go" | "node" => {
                if let Some(processor_arc) = &self.shared_doc_processor {
                    Ok(Arc::new(EnhancedLanguageTool::new(language, Arc::clone(processor_arc)).await?))
                } else {
                    // Fallback or error if shared_doc_processor is not set
                    // For now, let's fallback to creating one, but ideally, it should be provided.
                    warn!("共享的 EnhancedDocumentProcessor 未设置，为 {} 动态创建一个新的实例。这可能导致 VectorStore 不一致。", language);
                    let fallback_vector_tool = Arc::new(VectorDocsTool::new()?);
                    let fallback_processor = Arc::new(EnhancedDocumentProcessor::new(fallback_vector_tool).await?);
                    Ok(Arc::new(EnhancedLanguageTool::new(language, fallback_processor).await?))
                }
            }
            "flutter" | "dart" => Ok(Arc::new(FlutterDocsTool::new())),
            "csharp" => {
                if let Some(processor_arc) = &self.shared_doc_processor {
                    Ok(Arc::new(EnhancedLanguageTool::new("csharp", Arc::clone(processor_arc)).await?))
                } else {
                    warn!("共享的 EnhancedDocumentProcessor 未设置，为 csharp 动态创建一个新的实例。这可能导致 VectorStore 不一致。");
                    let fallback_vector_tool = Arc::new(VectorDocsTool::new()?);
                    let fallback_processor = Arc::new(EnhancedDocumentProcessor::new(fallback_vector_tool).await?);
                    Ok(Arc::new(EnhancedLanguageTool::new("csharp", fallback_processor).await?))
                }
            }
            "cpp" => {
                if let Some(processor_arc) = &self.shared_doc_processor {
                    Ok(Arc::new(EnhancedLanguageTool::new("cpp", Arc::clone(processor_arc)).await?))
                } else {
                    warn!("共享的 EnhancedDocumentProcessor 未设置，为 cpp 动态创建一个新的实例。这可能导致 VectorStore 不一致。");
                    let fallback_vector_tool = Arc::new(VectorDocsTool::new()?);
                    let fallback_processor = Arc::new(EnhancedDocumentProcessor::new(fallback_vector_tool).await?);
                    Ok(Arc::new(EnhancedLanguageTool::new("cpp", fallback_processor).await?))
                }
            }
            "php" => {
                if let Some(processor_arc) = &self.shared_doc_processor {
                    Ok(Arc::new(EnhancedLanguageTool::new("php", Arc::clone(processor_arc)).await?))
                } else {
                    warn!("共享的 EnhancedDocumentProcessor 未设置，为 php 动态创建一个新的实例。这可能导致 VectorStore 不一致。");
                    let fallback_vector_tool = Arc::new(VectorDocsTool::new()?);
                    let fallback_processor = Arc::new(EnhancedDocumentProcessor::new(fallback_vector_tool).await?);
                    Ok(Arc::new(EnhancedLanguageTool::new("php", fallback_processor).await?))
                }
            }
            "ruby" => {
                if let Some(processor_arc) = &self.shared_doc_processor {
                    Ok(Arc::new(EnhancedLanguageTool::new("ruby", Arc::clone(processor_arc)).await?))
                } else {
                    warn!("共享的 EnhancedDocumentProcessor 未设置，为 ruby 动态创建一个新的实例。这可能导致 VectorStore 不一致。");
                    let fallback_vector_tool = Arc::new(VectorDocsTool::new()?);
                    let fallback_processor = Arc::new(EnhancedDocumentProcessor::new(fallback_vector_tool).await?);
                    Ok(Arc::new(EnhancedLanguageTool::new("ruby", fallback_processor).await?))
                }
            }
            "swift" => {
                if let Some(processor_arc) = &self.shared_doc_processor {
                    Ok(Arc::new(EnhancedLanguageTool::new("swift", Arc::clone(processor_arc)).await?))
                } else {
                    warn!("共享的 EnhancedDocumentProcessor 未设置，为 swift 动态创建一个新的实例。这可能导致 VectorStore 不一致。");
                    let fallback_vector_tool = Arc::new(VectorDocsTool::new()?);
                    let fallback_processor = Arc::new(EnhancedDocumentProcessor::new(fallback_vector_tool).await?);
                    Ok(Arc::new(EnhancedLanguageTool::new("swift", fallback_processor).await?))
                }
            }
            _ => {
                if let Some(processor_arc) = &self.shared_doc_processor {
                    Ok(Arc::new(EnhancedLanguageTool::new(language, Arc::clone(processor_arc)).await?))
                } else {
                    warn!("共享的 EnhancedDocumentProcessor 未设置，为 {} 动态创建一个新的实例。这可能导致 VectorStore 不一致。", language);
                    let fallback_vector_tool = Arc::new(VectorDocsTool::new()?);
                    let fallback_processor = Arc::new(EnhancedDocumentProcessor::new(fallback_vector_tool).await?);
                    Ok(Arc::new(EnhancedLanguageTool::new(language, fallback_processor).await?))
                }
            }
        }
    }

    pub fn get_registered_tools(&self) -> &HashMap<String, Arc<dyn MCPTool>> {
        &self.registered_tools
    }

    pub fn get_tool_for_language(&self, language: &str) -> Option<&Arc<dyn MCPTool>> {
        self.language_tool_mapping
            .get(language)
            .and_then(|tool_name| self.registered_tools.get(tool_name))
    }

    pub async fn on_demand_register(&mut self, language: &str) -> Result<String> {
        if self.language_tool_mapping.contains_key(language) {
            return Ok(format!("enhanced_{}_docs", language));
        }

        info!("📦 按需注册工具: {}", language);
        
        // 使用缓存的检测报告或执行快速检测
        let score = if let Some(cached_report) = self.get_cached_detection_report().await {
            cached_report
                .detected_languages
                .get(language)
                .map(|info| info.score)
                .unwrap_or(0.3)
        } else {
            // 执行轻量级检测
            let mut temp_detector = EnvironmentDetector::new();
            let report = temp_detector.scan_environment().await?;
            let score = report
                .detected_languages
                .get(language)
                .map(|info| info.score)
                .unwrap_or(0.3);
            
            // 缓存检测结果
            self.cache_detection_report(report).await;
            score
        };
        
        let mut retry_attempts = HashMap::new();
        self.create_and_register_tool_with_retry(language, score, &mut retry_attempts).await
    }

    pub fn set_policy(&mut self, policy: RegistrationPolicy) {
        self.policy = policy;
        info!("🔧 注册策略已更新: {:?}", self.policy);
    }

    pub fn add_scan_path(&mut self, path: std::path::PathBuf) {
        info!("📁 添加扫描路径: {:?}", path);
        self.detector.add_scan_path(path);
    }

    // 增强的定期重扫描
    pub async fn periodic_rescan(&mut self) -> Result<bool> {
        info!("🔄 执行定期环境重扫描...");
        
        // 清理过期缓存
        self.cleanup_expired_cache().await;
        
        let new_report = self.detector.scan_environment().await?;
        self.cache_detection_report(new_report.clone()).await;
        
        let new_plan = self.create_registration_plan(&new_report)?;
        
        let mut changes_made = false;
        
        for (language, score) in new_plan {
            if !self.language_tool_mapping.contains_key(&language) {
                let mut retry_attempts = HashMap::new();
                match self.create_and_register_tool_with_retry(&language, score, &mut retry_attempts).await {
                    Ok(_) => {
                        info!("🆕 新注册工具: {}", language);
                        changes_made = true;
                    }
                    Err(e) => {
                        warn!("❌ 重扫描注册失败: {} - {}", language, e);
                    }
                }
            }
        }
        
        // 检查是否有工具需要移除（项目文件不再存在）
        let current_languages: std::collections::HashSet<String> = new_report.detected_languages.keys().cloned().collect();
        let registered_languages: Vec<String> = self.language_tool_mapping.keys().cloned().collect();
        
        for registered_lang in registered_languages {
            if !current_languages.contains(&registered_lang) {
                if let Some(tool_name) = self.language_tool_mapping.remove(&registered_lang) {
                    self.registered_tools.remove(&tool_name);
                    info!("🗑️ 移除不再需要的工具: {} ({})", registered_lang, tool_name);
                    changes_made = true;
                }
            }
        }
        
        Ok(changes_made)
    }

    // 新增：清理过期缓存
    async fn cleanup_expired_cache(&self) {
        let mut detection_cache = self.detection_cache.write().await;
        let mut tool_cache = self.tool_cache.write().await;
        
        let before_detection = detection_cache.len();
        let before_tool = tool_cache.len();
        
        detection_cache.retain(|_, entry| !entry.is_expired());
        tool_cache.retain(|_, entry| !entry.is_expired());
        
        let cleaned_detection = before_detection - detection_cache.len();
        let cleaned_tool = before_tool - tool_cache.len();
        
        if cleaned_detection > 0 || cleaned_tool > 0 {
            debug!("🧹 清理过期缓存: 检测缓存 {} 项, 工具缓存 {} 项", cleaned_detection, cleaned_tool);
        }
    }

    // 增强的统计信息
    pub async fn get_statistics(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        let metrics = self.metrics.read().await;
        
        stats.insert(
            "total_registered_tools".to_string(),
            serde_json::Value::Number(self.registered_tools.len().into()),
        );
        
        stats.insert(
            "supported_languages".to_string(),
            serde_json::Value::Array(
                self.language_tool_mapping
                    .keys()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect(),
            ),
        );
        
        stats.insert(
            "policy".to_string(),
            serde_json::Value::String(format!("{:?}", self.policy)),
        );
        
        stats.insert(
            "performance_metrics".to_string(),
            serde_json::to_value(&*metrics).unwrap_or(serde_json::Value::Null),
        );
        
        // 缓存统计
        let detection_cache = self.detection_cache.read().await;
        let tool_cache = self.tool_cache.read().await;
        
        stats.insert(
            "cache_stats".to_string(),
            serde_json::json!({
                "detection_cache_size": detection_cache.len(),
                "tool_cache_size": tool_cache.len(),
                "cache_config": self.cache_config,
            }),
        );
        
        stats
    }

    // 新增：健康检查
    pub async fn health_check(&self) -> HashMap<String, serde_json::Value> {
        let mut health = HashMap::new();
        
        // 检查注册工具状态
        let total_tools = self.registered_tools.len();
        health.insert("total_tools".to_string(), serde_json::Value::Number(total_tools.into()));
        
        // 检查缓存状态
        let detection_cache = self.detection_cache.read().await;
        let tool_cache = self.tool_cache.read().await;
        
        health.insert("cache_health".to_string(), serde_json::json!({
            "detection_cache_entries": detection_cache.len(),
            "tool_cache_entries": tool_cache.len(),
            "cache_enabled": self.cache_config.enable_persistent_cache,
        }));
        
        // 检查性能指标
        let metrics = self.metrics.read().await;
        health.insert("performance_health".to_string(), serde_json::json!({
            "success_rate": if metrics.total_registrations > 0 {
                metrics.successful_registrations as f64 / metrics.total_registrations as f64
            } else { 1.0 },
            "average_registration_time": metrics.average_registration_time_ms,
            "cache_hit_rate": metrics.cache_hit_rate,
        }));
        
        health.insert("status".to_string(), serde_json::Value::String("healthy".to_string()));
        health.insert("timestamp".to_string(), serde_json::Value::String(
            chrono::Utc::now().to_rfc3339()
        ));
        
        health
    }

    /// 评估自定义谓词表达式
    /// 
    /// 支持的表达式语法：
    /// - score > 0.5
    /// - files_count >= 3
    /// - has_cli("cargo")
    /// - language == "rust"
    /// - (score > 0.5) && (files_count >= 2)
    /// - has_framework("tokio") || has_framework("actix")
    fn evaluate_custom_predicate(
        &self, 
        predicate: &str, 
        language: &str, 
        info: &super::environment_detector::LanguageInfo
    ) -> bool {
        match self.parse_and_evaluate_expression(predicate, language, info) {
            Ok(result) => {
                debug!("✅ 自定义谓词 '{}' 评估结果: {}", predicate, result);
                result
            }
            Err(e) => {
                warn!("❌ 自定义谓词 '{}' 评估失败: {}", predicate, e);
                false
            }
        }
    }

    /// 解析并评估表达式
    fn parse_and_evaluate_expression(
        &self,
        expr: &str,
        language: &str,
        info: &super::environment_detector::LanguageInfo
    ) -> Result<bool> {
        let expr = expr.trim();
        
        // 处理逻辑运算符
        if let Some(pos) = expr.find(" && ") {
            let left = &expr[..pos];
            let right = &expr[pos + 4..];
            let left_result = self.parse_and_evaluate_expression(left, language, info)?;
            let right_result = self.parse_and_evaluate_expression(right, language, info)?;
            return Ok(left_result && right_result);
        }
        
        if let Some(pos) = expr.find(" || ") {
            let left = &expr[..pos];
            let right = &expr[pos + 4..];
            let left_result = self.parse_and_evaluate_expression(left, language, info)?;
            let right_result = self.parse_and_evaluate_expression(right, language, info)?;
            return Ok(left_result || right_result);
        }
        
        // 处理括号
        if expr.starts_with('(') && expr.ends_with(')') {
            let inner = &expr[1..expr.len()-1];
            return self.parse_and_evaluate_expression(inner, language, info);
        }
        
        // 处理比较运算符
        if let Some(pos) = expr.find(" >= ") {
            let left = &expr[..pos];
            let right = &expr[pos + 4..];
            let left_val = self.evaluate_variable(left, language, info)?;
            let right_val = self.parse_number(right)?;
            return Ok(left_val >= right_val);
        }
        
        if let Some(pos) = expr.find(" <= ") {
            let left = &expr[..pos];
            let right = &expr[pos + 4..];
            let left_val = self.evaluate_variable(left, language, info)?;
            let right_val = self.parse_number(right)?;
            return Ok(left_val <= right_val);
        }
        
        if let Some(pos) = expr.find(" > ") {
            let left = &expr[..pos];
            let right = &expr[pos + 3..];
            let left_val = self.evaluate_variable(left, language, info)?;
            let right_val = self.parse_number(right)?;
            return Ok(left_val > right_val);
        }
        
        if let Some(pos) = expr.find(" < ") {
            let left = &expr[..pos];
            let right = &expr[pos + 3..];
            let left_val = self.evaluate_variable(left, language, info)?;
            let right_val = self.parse_number(right)?;
            return Ok(left_val < right_val);
        }
        
        if let Some(pos) = expr.find(" == ") {
            let left = &expr[..pos];
            let right = &expr[pos + 4..];
            let left_str = self.evaluate_string_variable(left, language, info)?;
            let right_str = right.trim_matches('"');
            return Ok(left_str == right_str);
        }
        
        if let Some(pos) = expr.find(" != ") {
            let left = &expr[..pos];
            let right = &expr[pos + 4..];
            let left_str = self.evaluate_string_variable(left, language, info)?;
            let right_str = right.trim_matches('"');
            return Ok(left_str != right_str);
        }
        
        // 处理函数调用
        if expr.starts_with("has_cli(") && expr.ends_with(')') {
            let tool_name = &expr[8..expr.len()-1].trim_matches('"');
            return Ok(info.cli_tools.iter().any(|tool| tool.name == *tool_name));
        }
        
        if expr.starts_with("has_framework(") && expr.ends_with(')') {
            let framework_name = &expr[14..expr.len()-1].trim_matches('"');
            return Ok(info.detected_features.iter().any(|feature| feature.contains(framework_name)));
        }
        
        if expr.starts_with("has_file(") && expr.ends_with(')') {
            let file_pattern = &expr[9..expr.len()-1].trim_matches('"');
            return Ok(info.project_files.iter().any(|file_path| {
                std::path::Path::new(file_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.contains(file_pattern))
                    .unwrap_or(false)
            }));
        }
        
        // 处理布尔值
        match expr {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(anyhow!("无法解析表达式: {}", expr))
        }
    }

    /// 评估数值变量
    fn evaluate_variable(
        &self,
        var_name: &str,
        _language: &str,
        info: &super::environment_detector::LanguageInfo
    ) -> Result<f32> {
        match var_name.trim() {
            "score" => Ok(info.score),
            "files_count" => Ok(info.project_files.len() as f32),
            "cli_tools_count" => Ok(info.cli_tools.len() as f32),
            "features_count" => Ok(info.detected_features.len() as f32),
            _ => Err(anyhow!("未知变量: {}", var_name))
        }
    }

    /// 评估字符串变量
    fn evaluate_string_variable(
        &self,
        var_name: &str,
        language: &str,
        _info: &super::environment_detector::LanguageInfo
    ) -> Result<String> {
        match var_name.trim() {
            "language" => Ok(language.to_string()),
            _ => Err(anyhow!("未知字符串变量: {}", var_name))
        }
    }

    /// 解析数字
    fn parse_number(&self, s: &str) -> Result<f32> {
        s.trim().parse::<f32>()
            .map_err(|_| anyhow!("无法解析数字: {}", s))
    }
}

// 增强的构建器
pub struct DynamicRegistryBuilder {
    policy: RegistrationPolicy,
    scan_paths: Vec<std::path::PathBuf>,
    max_tools_per_language: usize,
    cache_config: CacheConfig,
    retry_config: (u32, u64), // (max_attempts, delay_ms)
    config_path: Option<std::path::PathBuf>,
    shared_doc_processor: Option<Arc<EnhancedDocumentProcessor>>,
}

impl DynamicRegistryBuilder {
    pub fn new() -> Self {
        Self {
            policy: RegistrationPolicy::default(),
            scan_paths: vec![std::env::current_dir().unwrap_or_default()],
            max_tools_per_language: 3,
            cache_config: CacheConfig::default(),
            retry_config: (3, 1000),
            config_path: None,
            shared_doc_processor: None,
        }
    }

    pub fn with_policy(mut self, policy: RegistrationPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn add_scan_path(mut self, path: std::path::PathBuf) -> Self {
        self.scan_paths.push(path);
        self
    }

    pub fn max_tools_per_language(mut self, max: usize) -> Self {
        self.max_tools_per_language = max;
        self
    }

    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    pub fn with_retry_config(mut self, max_attempts: u32, delay_ms: u64) -> Self {
        self.retry_config = (max_attempts, delay_ms);
        self
    }

    pub fn with_config_path(mut self, path: std::path::PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    pub fn with_shared_doc_processor(mut self, processor: Arc<EnhancedDocumentProcessor>) -> Self {
        self.shared_doc_processor = Some(processor);
        self
    }

    pub fn build(self) -> DynamicToolRegistry {
        DynamicToolRegistry {
            detector: EnvironmentDetector::new(),
            policy: self.policy,
            registered_tools: HashMap::new(),
            language_tool_mapping: HashMap::new(),
            max_tools_per_language: self.max_tools_per_language,
            tool_installer: None,
            auto_install_tools: false,
            cache_config: self.cache_config,
            detection_cache: Arc::new(RwLock::new(HashMap::new())),
            tool_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            max_retry_attempts: self.retry_config.0,
            retry_delay_ms: self.retry_config.1,
            config_path: self.config_path,
            shared_doc_processor: self.shared_doc_processor,
        }
    }
}

impl Default for DynamicRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
} 