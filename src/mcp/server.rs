use std::io::{self, BufRead, Write};
use anyhow::Result;
use serde_json::Value;
use tracing::{debug, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::tools::base::MCPTool;
use super::protocol::MCPRequest;

use super::{Request, Response, InitializeParams, InitializeResult, MCP_VERSION, SERVER_CAPABILITIES};

/// MCP 服务器
pub struct MCPServer {
    tools: Arc<RwLock<Vec<Box<dyn MCPTool>>>>,
}

impl MCPServer {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_tool(&self, tool: Box<dyn MCPTool>) -> Result<()> {
        let mut tools = self.tools.write().await;
        tools.push(tool);
        Ok(())
    }

    pub async fn execute_tool(&self, tool_name: &str, params: Value) -> Result<Value> {
        let tools = self.tools.read().await;
        
        for tool in tools.iter() {
            if tool.name() == tool_name {
                return tool.execute(params).await;
            }
        }
        
        Err(anyhow::anyhow!("工具不存在: {}", tool_name))
    }

    pub async fn handle_request(&self, _request: MCPRequest) -> Result<Value> {
        // 简单的请求处理逻辑
        Ok(serde_json::json!({
            "status": "success",
            "message": "请求处理成功"
        }))
    }
}

pub struct Server {
    /// 服务器名称
    name: String,
    /// 服务器版本
    version: String,
    /// 是否已初始化
    initialized: bool,
}

impl Server {
    /// 创建新的 MCP 服务器实例
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            initialized: false,
        }
    }

    /// 运行服务器
    pub async fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut reader = stdin.lock();

        loop {
            let mut request_line = String::new();
            if reader.read_line(&mut request_line)? == 0 {
                break; // EOF
            }

            // 解析请求
            let request: Request = match serde_json::from_str(&request_line) {
                Ok(req) => req,
                Err(e) => {
                    self.send_error(&mut stdout, "", -32700, &format!("Parse error: {}", e))?;
                    continue;
                }
            };

            debug!("Received request: {:?}", request);

            // 处理请求
            let response = self.handle_request(request).await;

            // 发送响应
            serde_json::to_writer(&mut stdout, &response)?;
            stdout.write_all(b"\\n")?;
            stdout.flush()?;
        }

        Ok(())
    }

    /// 处理 MCP 请求
    async fn handle_request(&mut self, request: Request) -> Response {
        // 检查版本兼容性
        if request.version != MCP_VERSION {
            return Response::error(
                request.id,
                -32600,
                format!("Unsupported protocol version: {}", request.version),
            );
        }

        // 处理初始化请求
        match request.method.as_str() {
            "initialize" => {
                if self.initialized {
                    return Response::error(request.id, -32600, "Server already initialized".to_string());
                }

                match self.handle_initialize(&request.params) {
                    Ok(result) => {
                        self.initialized = true;
                        Response::success(request.id, serde_json::to_value(result).unwrap())
                    }
                    Err(e) => Response::error(request.id, -32602, e.to_string()),
                }
            }
            _ => {
                if !self.initialized {
                    return Response::error(request.id, -32600, "Server not initialized".to_string());
                }

                match request.method.as_str() {
                    "shutdown" => {
                        self.initialized = false;
                        Response::success(request.id, Value::Null)
                    }
                    "documentSearch" => self.handle_document_search(request.id, &request.params).await,
                    "getApiExamples" => self.handle_get_api_examples(request.id, &request.params).await,
                    "checkVersionCompatibility" => {
                        self.handle_check_version_compatibility(request.id, &request.params).await
                    }
                    _ => Response::error(
                        request.id,
                        -32601,
                        format!("Method not found: {}", request.method),
                    ),
                }
            }
        }
    }

    /// 处理初始化请求
    fn handle_initialize(&self, params: &Value) -> Result<InitializeResult> {
        let params: InitializeParams = serde_json::from_value(params.clone())?;
        
        info!(
            "Client connected: {} {}",
            params.client_name, params.client_version
        );

        Ok(InitializeResult {
            server_name: self.name.clone(),
            server_version: self.version.clone(),
            protocol_version: MCP_VERSION.to_string(),
            capabilities: SERVER_CAPABILITIES.iter().map(|&s| s.to_string()).collect(),
        })
    }

    /// 处理文档搜索请求
    async fn handle_document_search(&self, id: String, params: &Value) -> Response {
        use crate::tools::{SearchDocsTools, MCPTool};
        
        let search_tool = SearchDocsTools::new();
        
        match search_tool.execute(params.clone()).await {
            Ok(result) => Response::success(id, result),
            Err(e) => Response::error(id, -32000, format!("文档搜索失败: {}", e)),
        }
    }

    /// 处理获取 API 示例请求
    async fn handle_get_api_examples(&self, id: String, params: &Value) -> Response {
        use crate::tools::{GetApiDocsTool, MCPTool};
        
        let api_docs_tool = GetApiDocsTool::new(None);
        
        match api_docs_tool.execute(params.clone()).await {
            Ok(result) => Response::success(id, result),
            Err(e) => Response::error(id, -32000, format!("获取API示例失败: {}", e)),
        }
    }

    /// 处理版本兼容性检查请求
    async fn handle_check_version_compatibility(&self, id: String, params: &Value) -> Response {
        use crate::tools::{CheckVersionTool, MCPTool};
        
        let version_tool = CheckVersionTool::new();
        
        match version_tool.execute(params.clone()).await {
            Ok(result) => Response::success(id, result),
            Err(e) => Response::error(id, -32000, format!("版本兼容性检查失败: {}", e)),
        }
    }

    /// 发送错误响应的辅助方法
    fn send_error(
        &self,
        writer: &mut impl Write,
        id: &str,
        code: i32,
        message: &str,
    ) -> Result<()> {
        let response = Response::error(id.to_string(), code, message.to_string());
        serde_json::to_writer(&mut *writer, &response)?;
        writer.write_all(b"\\n")?;
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initialization() {
        let mut server = Server::new("test-server".to_string(), "1.0.0".to_string());
        
        // 测试初始化请求
        let request = Request {
            version: MCP_VERSION.to_string(),
            id: "1".to_string(),
            method: "initialize".to_string(),
            params: serde_json::json!({
                "client_name": "test-client",
                "client_version": "1.0.0",
                "capabilities": ["documentSearch"]
            }),
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}
