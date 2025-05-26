use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 包信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// 包名称
    pub name: String,
    /// 包版本
    pub version: String,
    /// 包的描述
    pub description: String,
    /// 许可证
    pub license: String,
    /// 包的主页
    pub homepage: Option<String>,
    /// 代码仓库
    pub repository: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// 发布日期
    pub release_date: DateTime<Utc>,
    /// 下载次数
    pub download_count: Option<u64>,
    /// 可用版本列表
    pub available_versions: Vec<String>,
}

/// 依赖信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// 依赖名称
    pub name: String,
    /// 依赖版本
    pub version: Option<String>,
    /// 是否为开发依赖
    pub is_dev: bool,
    /// 源代码行号
    pub source_line: usize,
}
