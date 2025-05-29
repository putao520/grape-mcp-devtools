use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

use super::base::MCPTool;
use super::environment_detector::{EnvironmentDetector, DetectionReport};
use super::enhanced_language_tool::EnhancedLanguageTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationReport {
    pub registered_tools: Vec<String>,
    pub failed_registrations: Vec<(String, String)>, // (tool_name, error)
    pub registration_duration_ms: u64,
    pub total_detected_languages: usize,
    pub registration_score: f32,
}

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
}

impl Default for RegistrationPolicy {
    fn default() -> Self {
        RegistrationPolicy::Adaptive { score_threshold: 0.5 }
    }
}

pub struct DynamicToolRegistry {
    detector: EnvironmentDetector,
    policy: RegistrationPolicy,
    registered_tools: HashMap<String, Arc<dyn MCPTool>>,
    language_tool_mapping: HashMap<String, String>, // language -> tool_name
    max_tools_per_language: usize,
}

impl DynamicToolRegistry {
    pub fn new(policy: RegistrationPolicy) -> Self {
        Self {
            detector: EnvironmentDetector::new(),
            policy,
            registered_tools: HashMap::new(),
            language_tool_mapping: HashMap::new(),
            max_tools_per_language: 3,
        }
    }

    pub async fn auto_register(&mut self) -> Result<RegistrationReport> {
        let start_time = std::time::Instant::now();
        
        info!("🚀 开始动态工具注册...");
        
        // 1. 环境检测
        let detection_report = self.detector.scan_environment().await?;
        info!("📊 检测报告: {} 种语言", detection_report.detected_languages.len());
        
        // 2. 计算注册计划
        let registration_plan = self.create_registration_plan(&detection_report)?;
        info!("📋 注册计划: {} 个工具", registration_plan.len());
        
        // 3. 执行注册
        let mut registered_tools = Vec::new();
        let mut failed_registrations = Vec::new();
        
        for (language, score) in registration_plan {
            match self.create_and_register_tool(&language, score).await {
                Ok(tool_name) => {
                    registered_tools.push(tool_name);
                    info!("✅ 注册工具成功: {}", language);
                }
                Err(e) => {
                    failed_registrations.push((language.clone(), e.to_string()));
                    warn!("❌ 注册工具失败: {} - {}", language, e);
                }
            }
        }
        
        let duration = start_time.elapsed();
        let total_tools = registered_tools.len() + failed_registrations.len();
        let registration_score = if total_tools > 0 {
            registered_tools.len() as f32 / total_tools as f32
        } else {
            0.0
        };
        
        info!("🎯 动态注册完成: {}/{} 成功", registered_tools.len(), total_tools);
        
        Ok(RegistrationReport {
            registered_tools,
            failed_registrations,
            registration_duration_ms: duration.as_millis() as u64,
            total_detected_languages: detection_report.detected_languages.len(),
            registration_score,
        })
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

    async fn create_and_register_tool(&mut self, language: &str, score: f32) -> Result<String> {
        let tool_name = format!("enhanced_{}_docs", language);
        
        // 创建语言特定的工具
        let tool = self.create_language_tool(language, score).await?;
        
        // 注册工具
        self.registered_tools.insert(tool_name.clone(), tool);
        self.language_tool_mapping.insert(language.to_string(), tool_name.clone());
        
        Ok(tool_name)
    }

    async fn create_language_tool(&self, language: &str, _score: f32) -> Result<Arc<dyn MCPTool>> {
        // 导入DocumentStrategy
        use super::enhanced_language_tool::DocumentStrategy;
        
        match language {
            "rust" | "python" | "javascript" | "java" | "go" => {
                // 使用现有的 EnhancedLanguageTool
                let tool = EnhancedLanguageTool::new(language.to_string(), DocumentStrategy::CLIPrimary).await?;
                Ok(Arc::new(tool))
            }
            "csharp" => {
                // C# 工具实现 - 使用 .NET 生态系统的文档策略
                let tool = EnhancedLanguageTool::new("csharp".to_string(), DocumentStrategy::HTTPOnly).await?;
                info!("✅ 创建 C# 工具，支持 NuGet、.NET API 文档");
                Ok(Arc::new(tool))
            }
            "cpp" => {
                // C++ 工具实现 - 使用官方文档和标准库文档
                let tool = EnhancedLanguageTool::new("cpp".to_string(), DocumentStrategy::HTTPOnly).await?;
                info!("✅ 创建 C++ 工具，支持 CPP 参考文档、标准库");
                Ok(Arc::new(tool))
            }
            "php" => {
                // PHP 工具实现 - 使用 Packagist 和官方文档
                let tool = EnhancedLanguageTool::new("php".to_string(), DocumentStrategy::HTTPOnly).await?;
                info!("✅ 创建 PHP 工具，支持 Composer、Packagist 文档");
                Ok(Arc::new(tool))
            }
            "ruby" => {
                // Ruby 工具实现 - 使用 RubyGems 和官方文档
                let tool = EnhancedLanguageTool::new("ruby".to_string(), DocumentStrategy::HTTPOnly).await?;
                info!("✅ 创建 Ruby 工具，支持 Gem、Ruby 官方文档");
                Ok(Arc::new(tool))
            }
            "swift" => {
                // Swift 工具实现 - 使用 Swift Package Manager 和官方文档
                let tool = EnhancedLanguageTool::new("swift".to_string(), DocumentStrategy::HTTPOnly).await?;
                info!("✅ 创建 Swift 工具，支持 SPM、Apple 开发者文档");
                Ok(Arc::new(tool))
            }
            "dart" => {
                // Dart 工具实现 - 使用 pub.dev 和 Flutter 文档
                let tool = EnhancedLanguageTool::new("dart".to_string(), DocumentStrategy::HTTPOnly).await?;
                info!("✅ 创建 Dart 工具，支持 pub.dev、Flutter 文档");
                Ok(Arc::new(tool))
            }
            _ => {
                // 通用语言工具 - 尝试使用 Web 文档策略
                let tool = EnhancedLanguageTool::new(language.to_string(), DocumentStrategy::HTTPOnly).await?;
                info!("✅ 创建通用语言工具: {}", language);
                Ok(Arc::new(tool))
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
        
        // 为该语言创建临时检测
        let mut temp_detector = EnvironmentDetector::new();
        let report = temp_detector.scan_environment().await?;
        
        let score = report
            .detected_languages
            .get(language)
            .map(|info| info.score)
            .unwrap_or(0.3); // 默认分数
        
        self.create_and_register_tool(language, score).await
    }

    pub fn set_policy(&mut self, policy: RegistrationPolicy) {
        self.policy = policy;
    }

    pub fn add_scan_path(&mut self, path: std::path::PathBuf) {
        self.detector.add_scan_path(path);
    }

    pub async fn periodic_rescan(&mut self) -> Result<bool> {
        info!("🔄 执行定期环境重扫描...");
        
        let new_report = self.detector.scan_environment().await?;
        let new_plan = self.create_registration_plan(&new_report)?;
        
        let mut changes_made = false;
        
        // 检查是否有新语言需要注册
        for (language, score) in new_plan {
            if !self.language_tool_mapping.contains_key(&language) {
                match self.create_and_register_tool(&language, score).await {
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
        
        Ok(changes_made)
    }

    pub fn get_statistics(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
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
        
        stats
    }
}

// 配置构建器，方便用户配置
pub struct DynamicRegistryBuilder {
    policy: RegistrationPolicy,
    scan_paths: Vec<std::path::PathBuf>,
    max_tools_per_language: usize,
}

impl DynamicRegistryBuilder {
    pub fn new() -> Self {
        Self {
            policy: RegistrationPolicy::default(),
            scan_paths: vec![std::path::PathBuf::from(".")],
            max_tools_per_language: 3,
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

    pub fn build(self) -> DynamicToolRegistry {
        let mut registry = DynamicToolRegistry::new(self.policy);
        
        for path in self.scan_paths {
            registry.add_scan_path(path);
        }
        
        registry.max_tools_per_language = self.max_tools_per_language;
        registry
    }
}

impl Default for DynamicRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
} 