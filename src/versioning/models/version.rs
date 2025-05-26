use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use super::package::Package;

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// 包信息
    pub package: Package,
    /// 最新稳定版本
    pub latest_stable: String,
    /// 最新预览版本(如有)
    pub latest_preview: Option<String>,
    /// 发布日期
    pub release_date: DateTime<Utc>,
    /// 生命周期结束日期(如有)
    pub eol_date: Option<DateTime<Utc>>,
    /// 可用版本列表
    pub available_versions: Vec<String>,
    /// 依赖信息
    pub dependencies: Option<serde_json::Value>,
    /// 下载量
    pub downloads: Option<u64>,
}

/// 版本比较结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    /// 当前版本
    pub current: String,
    /// 目标版本
    pub target: String,
    /// 是否需要更新
    pub needs_update: bool,
    /// 版本差异类型
    pub diff_type: VersionDiffType,
}

/// 版本差异类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionDiffType {
    /// 主版本更新
    Major,
    /// 次版本更新
    Minor,
    /// 补丁更新
    Patch,
    /// 预览版本
    Preview,
    /// 无变化
    None,
}
