use std::io::Write;
use anyhow::Result;
use serde_json::Value;
use tracing::{debug, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::tools::base::MCPTool;
use super::protocol::MCPRequest;

use super::{Request, Response, InitializeParams, InitializeResult, MCP_VERSION, SERVER_CAPABILITIES};

/// 工具信息结构
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

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

    /// 获取所有工具列表
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        let tools = self.tools.read().await;
        let mut tool_list = Vec::new();
        
        for tool in tools.iter() {
            tool_list.push(ToolInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: serde_json::to_value(tool.parameters_schema()).unwrap_or(serde_json::json!({})),
            });
        }
        
        Ok(tool_list)
    }

    /// 获取指定工具的信息
    pub async fn get_tool_info(&self, tool_name: &str) -> Result<Option<ToolInfo>> {
        let tools = self.tools.read().await;
        
        for tool in tools.iter() {
            if tool.name() == tool_name {
                return Ok(Some(ToolInfo {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: serde_json::to_value(tool.parameters_schema()).unwrap_or(serde_json::json!({})),
                }));
            }
        }
        
        Ok(None)
    }

    /// 获取工具数量
    pub async fn get_tool_count(&self) -> Result<usize> {
        let tools = self.tools.read().await;
        Ok(tools.len())
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
    /// MCP 服务器实例
    mcp_server: Arc<RwLock<MCPServer>>,
}

impl Server {
    /// 创建新的 MCP 服务器实例
    pub fn new(name: String, version: String, mcp_server: MCPServer) -> Self {
        Self {
            name,
            version,
            initialized: false,
            mcp_server: Arc::new(RwLock::new(mcp_server)),
        }
    }

    /// 运行服务器
    pub async fn run(&mut self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);

        eprintln!("🔧 MCP服务器已启动，等待请求...");

        loop {
            let mut request_line = String::new();
            match reader.read_line(&mut request_line).await {
                Ok(0) => {
                    eprintln!("📡 客户端断开连接");
                    break; // EOF
                },
                Ok(n) => {
                    eprintln!("📥 收到 {} 字节数据: {}", n, request_line.trim());
                },
                Err(e) => {
                    eprintln!("❌ 读取stdin错误: {}", e);
                    break;
                }
            }

            // 解析请求
            let request: Request = match serde_json::from_str::<Request>(&request_line) {
                Ok(req) => {
                    eprintln!("✅ 请求解析成功: {} - {}", req.method, req.id);
                    req
                },
                Err(e) => {
                    eprintln!("❌ 请求解析失败: {}", e);
                    self.send_error_async(&mut stdout, "", -32700, &format!("Parse error: {}", e)).await?;
                    continue;
                }
            };

            debug!("Received request: {:?}", request);

            // 处理请求
            eprintln!("🔄 处理请求: {}", request.method);
            let response = self.handle_request(request).await;
            eprintln!("✅ 请求处理完成");

            // 发送响应
            let response_json = serde_json::to_string(&response)?;
            eprintln!("📤 发送响应: {}", response_json);
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
            eprintln!("✅ 响应发送完成");
        }

        eprintln!("👋 MCP服务器关闭");
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
                    "tools/list" => self.handle_list_tools(request.id).await,
                    "tools/call" => self.handle_tool_call(request.id, &request.params).await,
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

    /// 处理工具列表请求
    async fn handle_list_tools(&self, id: String) -> Response {
        let mcp_server = self.mcp_server.read().await;
        
        match mcp_server.list_tools().await {
            Ok(tools) => {
                let tool_list: Vec<Value> = tools.into_iter().map(|tool| {
                    serde_json::json!({
                        "name": tool.name,
                        "description": tool.description,
                        "inputSchema": tool.parameters
                    })
                }).collect();
                
                Response::success(id, serde_json::json!({
                    "tools": tool_list
                }))
            }
            Err(e) => Response::error(id, -32000, format!("获取工具列表失败: {}", e)),
        }
    }

    /// 处理工具调用请求
    async fn handle_tool_call(&self, id: String, params: &Value) -> Response {
        let mcp_server = self.mcp_server.read().await;
        
        // 解析工具调用参数
        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return Response::error(id, -32602, "Missing tool name".to_string());
            }
        };
        
        let tool_params = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
        
        // 执行工具
        match mcp_server.execute_tool(tool_name, tool_params).await {
            Ok(result) => Response::success(id, serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": result.to_string()
                    }
                ]
            })),
            Err(e) => Response::error(id, -32000, format!("工具执行失败: {}", e)),
        }
    }

    /// 发送错误响应的辅助方法
    #[allow(dead_code)]
    fn send_error(
        &self,
        writer: &mut impl Write,
        id: &str,
        code: i32,
        message: &str,
    ) -> Result<()> {
        let response = Response::error(id.to_string(), code, message.to_string());
        serde_json::to_writer(&mut *writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    async fn send_error_async(
        &self,
        writer: &mut tokio::io::Stdout,
        id: &str,
        code: i32,
        message: &str,
    ) -> Result<()> {
        let response = Response::error(id.to_string(), code, message.to_string());
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initialization() {
        let mut server = Server::new("test-server".to_string(), "1.0.0".to_string(), MCPServer::new());
        
        // 测试初始化请求
        let request = Request {
            jsonrpc: "2.0".to_string(),
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
