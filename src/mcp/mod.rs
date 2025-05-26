use serde::{Serialize, Deserialize};


/// MCP 协议版本
pub const MCP_VERSION: &str = "2025-03-26";

/// MCP 服务器的功能列表
pub const SERVER_CAPABILITIES: &[&str] = &[
    "documentSearch",      // 文档搜索
    "apiExamples",        // API 示例
    "versionInfo",        // 版本信息
    "compatibilityCheck", // 兼容性检查
];

/// MCP 请求
#[derive(Debug, Deserialize)]
pub struct Request {
    /// 协议版本号
    pub version: String,
    /// 请求 ID
    pub id: String,
    /// 请求的方法
    pub method: String,
    /// 请求参数
    pub params: serde_json::Value,
}

/// MCP 响应
#[derive(Debug, Serialize)]
pub struct Response {
    /// 协议版本号
    pub version: String,
    /// 请求 ID
    pub id: String,
    /// 响应结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
}

/// MCP 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// 错误代码
    pub code: i32,
    /// 错误消息
    pub message: String,
    /// 详细信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP 初始化参数
#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    /// 客户端名称
    pub client_name: String,
    /// 客户端版本
    pub client_version: String,
    /// 请求的功能列表
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// MCP 初始化结果
#[derive(Debug, Serialize)]
pub struct InitializeResult {
    /// 服务器名称
    pub server_name: String,
    /// 服务器版本
    pub server_version: String,
    /// 协议版本号
    pub protocol_version: String,
    /// 服务器支持的功能列表
    pub capabilities: Vec<String>,
}

impl Response {
    /// 创建一个成功响应
    pub fn success(id: String, result: serde_json::Value) -> Self {
        Self {
            version: MCP_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// 创建一个错误响应
    pub fn error(id: String, code: i32, message: String) -> Self {
        Self {
            version: MCP_VERSION.to_string(),
            id,
            result: None,
            error: Some(ErrorResponse {
                code,
                message,
                data: None,
            }),
        }
    }
}

// 错误代码定义
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    
    // MCP 特定错误码
    pub const DOC_NOT_FOUND: i32 = -33000;
    pub const VERSION_NOT_FOUND: i32 = -33001;
    pub const INCOMPATIBLE_VERSION: i32 = -33002;
    pub const SEARCH_FAILED: i32 = -33003;
    pub const VECTORIZATION_FAILED: i32 = -33004;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_creation() {
        let resp = Response::success(
            "test-1".to_string(),
            serde_json::json!({"status": "ok"}),
        );
        assert_eq!(resp.id, "test-1");
        assert!(resp.error.is_none());

        let err_resp = Response::error(
            "test-2".to_string(),
            error_codes::INVALID_REQUEST,
            "Invalid request".to_string(),
        );
        assert_eq!(err_resp.id, "test-2");
        assert!(err_resp.result.is_none());
        assert!(err_resp.error.is_some());
    }
}

pub mod server;
pub mod protocol;

pub use server::MCPServer;
pub use protocol::{MCPRequest, MCPResponse};
