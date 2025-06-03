use thiserror::Error;
use anyhow;

pub type MCPResult<T> = anyhow::Result<T>;
pub type DocGenResult<T> = anyhow::Result<T>;
pub type Result<T> = std::result::Result<T, VectorDbError>;

#[derive(Error, Debug)]
pub enum MCPError {
    #[error("参数无效: {0}")]
    InvalidParameter(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("服务器错误: {0}")]
    ServerError(String),

    #[error("请求超时: {0}")]
    Timeout(String),

    #[error("认证失败: {0}")]
    AuthenticationError(String),

    #[error("权限不足: {0}")]
    AuthorizationError(String),

    #[error("限流错误: {0}")]
    RateLimitError(String),

    #[error("缓存错误: {0}")]
    CacheError(String),

    #[error("版本格式错误: {0}")]
    InvalidVersion(String),

    #[error("不支持的编程语言: {0}")]
    UnsupportedLanguage(String),

    #[error("变更日志解析错误: {0}")]
    ChangelogParseError(String),

    #[error("版本比较错误: {0}")]
    VersionCompareError(String),

    #[error("API兼容性检查错误: {0}")]
    CompatibilityCheckError(String),

    #[error("文档生成错误: {0}")]
    DocumentationError(String),

    #[error("工具执行失败: {0}")]
    ToolExecutionFailed(String),

    #[error("工具未找到: {0}")]
    ToolNotFound(String),
}

#[derive(Error, Debug)]
pub enum DocGenError {
    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("进程执行错误: {0}")]
    ProcessError(String),

    #[error("输入无效: {0}")]
    InvalidInput(String),

    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("工具不可用: {0}")]
    ToolNotAvailable(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON序列化错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl MCPError {
    pub fn error_code(&self) -> &'static str {
        match self {
            MCPError::InvalidParameter(_) => "INVALID_PARAMETER",
            MCPError::NotFound(_) => "NOT_FOUND",
            MCPError::ServerError(_) => "SERVER_ERROR",
            MCPError::Timeout(_) => "TIMEOUT",
            MCPError::AuthenticationError(_) => "AUTH_ERROR",
            MCPError::AuthorizationError(_) => "FORBIDDEN",
            MCPError::RateLimitError(_) => "RATE_LIMIT",
            MCPError::CacheError(_) => "CACHE_ERROR",
            MCPError::InvalidVersion(_) => "INVALID_VERSION",
            MCPError::UnsupportedLanguage(_) => "UNSUPPORTED_LANGUAGE",
            MCPError::ChangelogParseError(_) => "CHANGELOG_PARSE_ERROR",
            MCPError::VersionCompareError(_) => "VERSION_COMPARE_ERROR",
            MCPError::CompatibilityCheckError(_) => "COMPATIBILITY_CHECK_ERROR",
            MCPError::DocumentationError(_) => "DOCUMENTATION_ERROR",
            MCPError::ToolExecutionFailed(_) => "TOOL_EXECUTION_FAILED",
            MCPError::ToolNotFound(_) => "TOOL_NOT_FOUND",
        }
    }

    pub fn suggestion(&self) -> &'static str {
        match self {
            MCPError::InvalidParameter(_) => "请检查参数格式并确保所有必需参数都已提供",
            MCPError::NotFound(_) => "请检查资源标识符是否正确，或尝试使用其他查询条件",
            MCPError::ServerError(_) => "请稍后重试，如果问题持续存在请联系管理员",
            MCPError::Timeout(_) => "请检查网络连接，或稍后重试",
            MCPError::AuthenticationError(_) => "请检查认证凭据是否有效",
            MCPError::AuthorizationError(_) => "请确保您有足够的权限执行此操作",
            MCPError::RateLimitError(_) => "请降低请求频率，稍后重试",
            MCPError::CacheError(_) => "缓存操作失败，请重试",
            MCPError::InvalidVersion(_) => "请检查版本号格式是否符合规范（如：semver）",
            MCPError::UnsupportedLanguage(_) => "请检查所请求的编程语言是否在支持列表中",
            MCPError::ChangelogParseError(_) => "变更日志解析失败，请确保文档格式正确",
            MCPError::VersionCompareError(_) => "版本比较失败，请确保两个版本都存在且格式正确",
            MCPError::CompatibilityCheckError(_) => "API兼容性检查失败，请检查版本信息",
            MCPError::DocumentationError(_) => "文档生成失败，请检查源文件和配置",
            MCPError::ToolExecutionFailed(_) => "工具执行失败，请检查工具和配置",
            MCPError::ToolNotFound(_) => "工具未找到，请检查工具路径和名称",
        }
    }

    /// 获取错误的详细描述
    pub fn details(&self) -> String {
        match self {
            MCPError::InvalidVersion(msg) => format!("版本格式错误: {}\n建议使用语义化版本(semver)格式，如: 1.0.0", msg),
            MCPError::UnsupportedLanguage(lang) => format!("不支持的语言: {}\n目前支持的语言包括: Rust, Python, JavaScript, TypeScript, Go, Java", lang),
            MCPError::ChangelogParseError(msg) => format!("变更日志解析失败: {}\n请确保文档使用标准的Markdown或reStructuredText格式", msg),
            MCPError::VersionCompareError(msg) => format!("版本比较失败: {}\n请确保指定的版本存在于项目的发布历史中", msg),
            MCPError::CompatibilityCheckError(msg) => format!("兼容性检查失败: {}\n建议查看语言或框架的官方兼容性指南", msg),
            MCPError::DocumentationError(msg) => format!("文档生成失败: {}\n请检查文档源文件的格式和必要的工具是否正确安装", msg),
            MCPError::ToolExecutionFailed(msg) => format!("工具执行失败: {}\n请检查工具和配置", msg),
            MCPError::ToolNotFound(msg) => format!("工具未找到: {}\n请检查工具路径和名称", msg),
            _ => self.to_string(),
        }
    }

    /// 检查错误是否可恢复
    pub fn is_recoverable(&self) -> bool {
        match self {
            MCPError::InvalidVersion(_) |
            MCPError::UnsupportedLanguage(_) |
            MCPError::ChangelogParseError(_) => true,
            MCPError::ServerError(_) |
            MCPError::Timeout(_) |
            MCPError::RateLimitError(_) |
            MCPError::CacheError(_) => true,
            _ => false,
        }
    }

    /// 获取错误的处理建议
    pub fn resolution_steps(&self) -> Vec<String> {
        match self {
            MCPError::InvalidVersion(_) => vec![
                "检查版本号格式是否符合semver规范".to_string(),
                "确保版本号包含主版本号、次版本号和修订号".to_string(),
                "移除任何非标准的版本标识符".to_string(),
            ],
            MCPError::UnsupportedLanguage(_) => vec![
                "查看支持的语言列表".to_string(),
                "检查语言名称的拼写是否正确".to_string(),
                "考虑请求添加新语言支持".to_string(),
            ],
            MCPError::ChangelogParseError(_) => vec![
                "检查变更日志的格式是否规范".to_string(),
                "确保文档使用正确的Markdown语法".to_string(),
                "验证文档的编码格式是否正确".to_string(),
            ],
            _ => vec![self.suggestion().to_string()],
        }
    }
}

/// 向量数据库错误类型
#[derive(Error, Debug)]
pub enum VectorDbError {
    #[error("存储错误: {0}")]
    Storage(String),
    
    #[error("索引错误: {0}")]
    Index(String),
    
    #[error("嵌入错误: {0}")]
    Embedding(String),
    
    #[error("配置错误: {0}")]
    Config(String),
    
    #[error("查询错误: {0}")]
    Query(String),
    
    #[error("无效的向量维度: 期望 {expected}, 实际 {actual}")]
    InvalidVectorDimension { expected: usize, actual: usize },
    
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("HTTP 错误: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Sled 数据库错误: {0}")]
    Sled(#[from] sled::Error),
    
    #[error("二进制编码错误: {0}")]
    Bincode(#[from] bincode::Error),
    
    #[error("任务错误: {0}")]
    Task(#[from] tokio::task::JoinError),
    
    #[error("其他错误: {0}")]
    Other(String),
}

impl VectorDbError {
    pub fn storage_error(msg: String) -> Self {
        Self::Storage(msg)
    }
    
    pub fn index_error(msg: String) -> Self {
        Self::Index(msg)
    }
    
    pub fn embedding_error(msg: String) -> Self {
        Self::Embedding(msg)
    }
    
    pub fn config_error(msg: String) -> Self {
        Self::Config(msg)
    }
    
    pub fn query_error(msg: String) -> Self {
        Self::Query(msg)
    }
    
    pub fn other_error(msg: String) -> Self {
        Self::Other(msg)
    }
}
