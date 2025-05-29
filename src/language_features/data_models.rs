use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// 编程语言版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageVersion {
    /// 语言名称 (rust, python, javascript, etc.)
    pub language: String,
    /// 版本号 (1.75.0, 3.12, ES2023, etc.)
    pub version: String,
    /// 发布日期
    pub release_date: DateTime<Utc>,
    /// 是否为稳定版本
    pub is_stable: bool,
    /// 是否为LTS版本
    pub is_lts: bool,
    /// 版本状态
    pub status: VersionStatus,
    /// 新增特性
    pub features: Vec<LanguageFeature>,
    /// 语法变化
    pub syntax_changes: Vec<SyntaxChange>,
    /// 弃用信息
    pub deprecations: Vec<Deprecation>,
    /// 破坏性变更
    pub breaking_changes: Vec<BreakingChange>,
    /// 性能改进
    pub performance_improvements: Vec<PerformanceImprovement>,
    /// 标准库变化
    pub stdlib_changes: Vec<StdlibChange>,
    /// 工具链变化
    pub toolchain_changes: Vec<ToolchainChange>,
    /// 版本元数据
    pub metadata: VersionMetadata,
}

/// 版本状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionStatus {
    /// 当前稳定版本
    Current,
    /// 旧版本但仍受支持
    Supported,
    /// 已停止支持
    EndOfLife,
    /// 预览版本
    Preview,
    /// Beta版本
    Beta,
    /// Alpha版本
    Alpha,
}

/// 语言特性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageFeature {
    /// 特性名称
    pub name: String,
    /// 特性描述
    pub description: String,
    /// 特性类别
    pub category: FeatureCategory,
    /// 代码示例
    pub examples: Vec<CodeExample>,
    /// RFC/提案链接
    pub proposal_link: Option<String>,
    /// 文档链接
    pub documentation_link: Option<String>,
    /// 稳定性级别
    pub stability: FeatureStability,
    /// 相关标签
    pub tags: Vec<String>,
    /// 影响程度
    pub impact: ImpactLevel,
}

/// 特性类别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureCategory {
    /// 语法特性
    Syntax,
    /// 标准库
    StandardLibrary,
    /// 类型系统
    TypeSystem,
    /// 异步编程
    Async,
    /// 内存管理
    Memory,
    /// 错误处理
    ErrorHandling,
    /// 模块系统
    Modules,
    /// 宏系统
    Macros,
    /// 工具链
    Toolchain,
    /// 性能优化
    Performance,
    /// 安全特性
    Security,
    /// 其他
    Other(String),
}

/// 稳定性级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureStability {
    /// 稳定特性
    Stable,
    /// Beta特性
    Beta,
    /// 实验性特性
    Experimental,
    /// 已弃用
    Deprecated,
    /// 已移除
    Removed,
}

/// 影响程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactLevel {
    /// 高影响 - 重大语法变化
    High,
    /// 中等影响 - 常用功能变化
    Medium,
    /// 低影响 - 小幅改进
    Low,
    /// 内部变化 - 用户不可见
    Internal,
}

/// 代码示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    /// 示例标题
    pub title: String,
    /// 代码内容
    pub code: String,
    /// 示例描述
    pub description: Option<String>,
    /// 运行环境要求
    pub requirements: Option<String>,
}

/// 语法变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxChange {
    /// 变化类型
    pub change_type: SyntaxChangeType,
    /// 变化描述
    pub description: String,
    /// 旧语法示例
    pub old_syntax: Option<String>,
    /// 新语法示例
    pub new_syntax: Option<String>,
    /// 迁移指南
    pub migration_guide: Option<String>,
}

/// 语法变化类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyntaxChangeType {
    /// 新增语法
    Addition,
    /// 修改语法
    Modification,
    /// 移除语法
    Removal,
    /// 语法改进
    Enhancement,
}

/// 弃用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deprecation {
    /// 弃用的功能名称
    pub feature_name: String,
    /// 弃用原因
    pub reason: String,
    /// 推荐替代方案
    pub replacement: Option<String>,
    /// 完全移除的版本
    pub removal_version: Option<String>,
    /// 弃用警告级别
    pub warning_level: DeprecationLevel,
}

/// 弃用警告级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeprecationLevel {
    /// 软弃用 - 仍然可用
    Soft,
    /// 硬弃用 - 产生警告
    Hard,
    /// 即将移除
    PendingRemoval,
}

/// 破坏性变更
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// 变更描述
    pub description: String,
    /// 影响的功能
    pub affected_features: Vec<String>,
    /// 迁移指南
    pub migration_guide: String,
    /// 自动迁移工具
    pub automation_available: bool,
}

/// 性能改进
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImprovement {
    /// 改进描述
    pub description: String,
    /// 性能提升百分比
    pub improvement_percentage: Option<f64>,
    /// 基准测试链接
    pub benchmark_link: Option<String>,
    /// 影响的操作
    pub affected_operations: Vec<String>,
}

/// 标准库变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdlibChange {
    /// 模块名称
    pub module_name: String,
    /// 变化类型
    pub change_type: StdlibChangeType,
    /// 变化描述
    pub description: String,
    /// 示例代码
    pub examples: Vec<CodeExample>,
}

/// 标准库变化类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StdlibChangeType {
    /// 新增模块/函数
    Addition,
    /// 修改现有功能
    Modification,
    /// 弃用功能
    Deprecation,
    /// 移除功能
    Removal,
    /// 性能改进
    Performance,
}

/// 工具链变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainChange {
    /// 工具名称
    pub tool_name: String,
    /// 变化描述
    pub description: String,
    /// 新增的选项或功能
    pub new_options: Vec<String>,
    /// 使用示例
    pub usage_examples: Vec<String>,
}

/// 版本元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMetadata {
    /// 发布说明链接
    pub release_notes_url: Option<String>,
    /// 下载链接
    pub download_url: Option<String>,
    /// 源代码链接
    pub source_url: Option<String>,
    /// 文档链接
    pub documentation_url: Option<String>,
    /// 变更日志链接
    pub changelog_url: Option<String>,
    /// 升级指南链接
    pub upgrade_guide_url: Option<String>,
    /// 额外标签
    pub tags: HashMap<String, String>,
}

/// 版本比较结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparison {
    /// 源版本
    pub from_version: String,
    /// 目标版本
    pub to_version: String,
    /// 语言名称
    pub language: String,
    /// 新增特性
    pub added_features: Vec<LanguageFeature>,
    /// 移除的特性
    pub removed_features: Vec<String>,
    /// 修改的特性
    pub modified_features: Vec<FeatureModification>,
    /// 破坏性变更
    pub breaking_changes: Vec<BreakingChange>,
    /// 弃用信息
    pub deprecations: Vec<Deprecation>,
    /// 升级建议
    pub upgrade_recommendations: Vec<UpgradeRecommendation>,
}

/// 特性修改信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureModification {
    /// 特性名称
    pub feature_name: String,
    /// 修改类型
    pub modification_type: ModificationType,
    /// 修改描述
    pub description: String,
    /// 影响程度
    pub impact: ImpactLevel,
}

/// 修改类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    /// 行为变更
    BehaviorChange,
    /// API变更
    ApiChange,
    /// 性能改进
    PerformanceImprovement,
    /// Bug修复
    BugFix,
    /// 安全修复
    SecurityFix,
}

/// 升级建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeRecommendation {
    /// 建议标题
    pub title: String,
    /// 建议描述
    pub description: String,
    /// 优先级
    pub priority: RecommendationPriority,
    /// 相关链接
    pub links: Vec<String>,
}

/// 建议优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    /// 必须执行
    Critical,
    /// 强烈建议
    High,
    /// 建议执行
    Medium,
    /// 可选
    Low,
} 