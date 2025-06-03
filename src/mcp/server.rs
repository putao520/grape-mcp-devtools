use std::io::Write;
use anyhow::Result;
use serde_json::Value;
use tracing::{debug, info, warn, error};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use crate::tools::base::MCPTool;
use super::protocol::MCPRequest;

use super::{Request, Response, InitializeParams, InitializeResult, MCP_VERSION, SERVER_CAPABILITIES};

/// 工具信息结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub language: Option<String>,
    pub category: Option<String>,
    pub version: Option<String>,
}

/// 工具执行请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolRequest {
    pub tool_name: String,
    pub params: Value,
    pub timeout: Option<Duration>,
}

/// 工具执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub result: Value,
    pub execution_time: Duration,
    pub success: bool,
    pub error: Option<String>,
}

/// 工具健康状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ToolHealth {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

/// MCP 服务器
pub struct MCPServer {
    tools: Arc<RwLock<Vec<Arc<dyn MCPTool>>>>,
    default_timeout: Duration,
    performance_metrics: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
}

impl MCPServer {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(Vec::new())),
            default_timeout: Duration::from_secs(30),
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            tools: Arc::new(RwLock::new(Vec::new())),
            default_timeout: timeout,
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_tool(&self, tool: Box<dyn MCPTool>) -> Result<()> {
        let mut tools = self.tools.write().await;
        tools.push(Arc::from(tool));
        info!("工具注册成功: {}", tools.last().unwrap().name());
        Ok(())
    }

    /// 注册Arc包装的工具
    pub async fn register_tool_arc(&self, tool: Arc<dyn MCPTool>) -> Result<()> {
        let tool_name = tool.name().to_string();
        let mut tools = self.tools.write().await;
        tools.push(tool);
        info!("Arc工具注册成功: {}", tool_name);
        Ok(())
    }

    /// 带超时的工具执行
    pub async fn execute_tool_with_timeout(&self, tool_name: &str, params: Value, timeout_duration: Duration) -> Result<Value> {
        let start_time = Instant::now();
        
        let tools = self.tools.read().await;
        let tool = tools.iter()
            .find(|t| t.name() == tool_name)
            .ok_or_else(|| anyhow::anyhow!("工具不存在: {}", tool_name))?
            .clone();
        
        // 释放读锁
        drop(tools);
        
        let result = timeout(timeout_duration, tool.execute(params))
            .await
            .map_err(|_| anyhow::anyhow!("工具执行超时: {}", tool_name))?;
        
        let execution_time = start_time.elapsed();
        
        // 记录性能指标
        self.record_performance_metric(tool_name, execution_time).await;
        
        result
    }

    pub async fn execute_tool(&self, tool_name: &str, params: Value) -> Result<Value> {
        self.execute_tool_with_timeout(tool_name, params, self.default_timeout).await
    }

    /// 批量执行工具
    pub async fn batch_execute_tools(&self, requests: Vec<ToolRequest>) -> Result<Vec<ToolResult>> {
        let mut results = Vec::with_capacity(requests.len());
        let futures: Vec<_> = requests.into_iter().map(|req| {
            let timeout_duration = req.timeout.unwrap_or(self.default_timeout);
            async move {
                let start_time = Instant::now();
                let result = self.execute_tool_with_timeout(&req.tool_name, req.params, timeout_duration).await;
                let execution_time = start_time.elapsed();
                
                match result {
                    Ok(value) => ToolResult {
                        tool_name: req.tool_name,
                        result: value,
                        execution_time,
                        success: true,
                        error: None,
                    },
                    Err(e) => ToolResult {
                        tool_name: req.tool_name,
                        result: Value::Null,
                        execution_time,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            }
        }).collect();
        
        // 并行执行所有请求
        for future in futures {
            results.push(future.await);
        }
        
        Ok(results)
    }

    /// 获取工具健康状态
    pub async fn get_tool_health_status(&self) -> Result<HashMap<String, ToolHealth>> {
        let tools = self.tools.read().await;
        let mut health_status = HashMap::new();
        
        for tool in tools.iter() {
            let tool_name = tool.name();
            let health = self.check_tool_health(tool_name).await;
            health_status.insert(tool_name.to_string(), health);
        }
        
        Ok(health_status)
    }

    /// 检查单个工具的健康状态
    async fn check_tool_health(&self, tool_name: &str) -> ToolHealth {
        let metrics = self.performance_metrics.read().await;
        
        if let Some(durations) = metrics.get(tool_name) {
            if durations.is_empty() {
                return ToolHealth::Degraded { 
                    reason: "无执行历史记录".to_string() 
                };
            }
            
            let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
            let failure_rate = durations.iter()
                .filter(|d| *d > &Duration::from_secs(30))
                .count() as f64 / durations.len() as f64;
            
            if failure_rate > 0.3 {
                ToolHealth::Unhealthy { 
                    reason: format!("失败率过高: {:.1}%", failure_rate * 100.0) 
                }
            } else if avg_duration > Duration::from_secs(10) {
                ToolHealth::Degraded { 
                    reason: format!("平均响应时间过长: {:?}", avg_duration) 
                }
            } else {
                ToolHealth::Healthy
            }
        } else {
            ToolHealth::Degraded { 
                reason: "无性能数据".to_string() 
            }
        }
    }

    /// 记录性能指标
    async fn record_performance_metric(&self, tool_name: &str, duration: Duration) {
        let mut metrics = self.performance_metrics.write().await;
        let durations = metrics.entry(tool_name.to_string()).or_insert_with(Vec::new);
        
        durations.push(duration);
        
        // 保持最近100次执行记录
        if durations.len() > 100 {
            durations.remove(0);
        }
    }

    /// 获取所有工具列表
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        let tools = self.tools.read().await;
        let mut tool_list = Vec::new();
        
        for tool in tools.iter() {
            let description = tool.description();
            
            // 从描述中尝试提取语言信息
            let language = if description.contains("Rust") {
                Some("rust".to_string())
            } else if description.contains("Python") {
                Some("python".to_string())
            } else if description.contains("JavaScript") || description.contains("TypeScript") {
                Some("javascript".to_string())
            } else if description.contains("Java") {
                Some("java".to_string())
            } else if description.contains("Go") {
                Some("go".to_string())
            } else if description.contains("C#") {
                Some("csharp".to_string())
            } else if description.contains("C++") {
                Some("cpp".to_string())
            } else {
                None
            };

            tool_list.push(ToolInfo {
                name: tool.name().to_string(),
                description: description.to_string(),
                parameters: serde_json::to_value(tool.parameters_schema()).unwrap_or(serde_json::json!({})),
                language,
                category: Some("documentation".to_string()),
                version: Some("1.0.0".to_string()),
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
                    language: None,
                    category: None,
                    version: None,
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

    /// 获取性能统计信息
    pub async fn get_performance_stats(&self) -> Result<HashMap<String, Value>> {
        let metrics = self.performance_metrics.read().await;
        let mut stats = HashMap::new();
        
        for (tool_name, durations) in metrics.iter() {
            if !durations.is_empty() {
                let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
                let min_duration = durations.iter().min().unwrap();
                let max_duration = durations.iter().max().unwrap();
                
                stats.insert(tool_name.clone(), serde_json::json!({
                    "avg_duration_ms": avg_duration.as_millis(),
                    "min_duration_ms": min_duration.as_millis(),
                    "max_duration_ms": max_duration.as_millis(),
                    "execution_count": durations.len(),
                }));
            }
        }
        
        Ok(stats)
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
        match request.method.as_str() {
            "initialize" => {
                match self.handle_initialize(&request.params) {
                    Ok(result) => {
                        self.initialized = true;
                        info!("服务器初始化成功");
                        Response::success(request.id, serde_json::to_value(result).unwrap())
                    }
                    Err(e) => {
                        error!("服务器初始化失败: {}", e);
                        Response::error(request.id, -32600, format!("初始化失败: {}", e))
                    }
                }
            }
            "tools/list" => {
                if !self.initialized {
                    warn!("服务器未初始化，拒绝tools/list请求");
                    return Response::error(request.id, -32002, "服务器未初始化".to_string());
                }
                self.handle_list_tools(request.id).await
            }
            "tools/call" => {
                if !self.initialized {
                    warn!("服务器未初始化，拒绝tools/call请求");
                    return Response::error(request.id, -32002, "服务器未初始化".to_string());
                }
                self.handle_tool_call(request.id, &request.params).await
            }
            "health_check" => {
                if !self.initialized {
                    return Response::error(request.id, -32002, "服务器未初始化".to_string());
                }
                self.handle_health_check(request.id).await
            }
            "get_stats" => {
                if !self.initialized {
                    return Response::error(request.id, -32002, "服务器未初始化".to_string());
                }
                self.handle_stats_request(request.id).await
            }
            "tools/batch_call" => {
                if !self.initialized {
                    return Response::error(request.id, -32002, "服务器未初始化".to_string());
                }
                self.handle_batch_tool_call(request.id, &request.params).await
            }
            _ => {
                warn!("不支持的方法: {}", request.method);
                Response::error(request.id, -32601, format!("不支持的方法: {}", request.method))
            }
        }
    }

    fn handle_initialize(&self, params: &Value) -> Result<InitializeResult> {
        info!("处理初始化请求: {:?}", params);
        
        // 解析初始化参数（如果需要）
        if let Ok(_init_params) = serde_json::from_value::<InitializeParams>(params.clone()) {
            debug!("初始化参数解析成功");
        }
        
        Ok(InitializeResult {
            server_name: self.name.clone(),
            server_version: self.version.clone(),
            protocol_version: MCP_VERSION.to_string(),
            capabilities: SERVER_CAPABILITIES.iter().map(|&s| s.to_string()).collect(),
        })
    }

    async fn handle_list_tools(&self, id: String) -> Response {
        debug!("处理工具列表请求");
        
        let server = self.mcp_server.read().await;
        match server.list_tools().await {
            Ok(tools) => {
                info!("成功获取工具列表，共 {} 个工具", tools.len());
                Response::success(id, serde_json::json!({
                    "tools": tools
                }))
            }
            Err(e) => {
                error!("获取工具列表失败: {}", e);
                Response::error(id, -32603, format!("获取工具列表失败: {}", e))
            }
        }
    }

    async fn handle_tool_call(&self, id: String, params: &Value) -> Response {
        debug!("处理工具调用请求: {:?}", params);
        
        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                warn!("工具调用请求缺少name参数");
                return Response::error(id, -32602, "缺少name参数".to_string());
            }
        };

        let tool_params = params.get("arguments").unwrap_or(&Value::Null).clone();
        
        let server = self.mcp_server.read().await;
        match server.execute_tool(tool_name, tool_params).await {
            Ok(result) => {
                info!("工具 {} 执行成功", tool_name);
                
                // 智能处理结果格式
                let content_text = if result.is_string() {
                    result.as_str().unwrap_or("").to_string()
                } else {
                    // 如果是对象，提取有意义的内容
                    if let Some(results_array) = result.get("results").and_then(|r| r.as_array()) {
                        let mut formatted_content = String::new();
                        formatted_content.push_str(&format!("# {} 搜索结果\n\n", tool_name));
                        
                        if let Some(summary) = result.get("summary").and_then(|s| s.as_str()) {
                            formatted_content.push_str(&format!("{}\n\n", summary));
                        }
                        
                        for (i, item) in results_array.iter().enumerate() {
                            if let (Some(title), Some(content)) = (
                                item.get("title").and_then(|t| t.as_str()),
                                item.get("content").and_then(|c| c.as_str())
                            ) {
                                formatted_content.push_str(&format!("## {}. {}\n\n", i + 1, title));
                                formatted_content.push_str(&format!("{}\n\n", content));
                                
                                if let Some(url) = item.get("url").and_then(|u| u.as_str()) {
                                    formatted_content.push_str(&format!("🔗 链接: {}\n\n", url));
                                }
                            }
                        }
                        
                        formatted_content
                    } else {
                        // 如果没有results数组，尝试格式化整个JSON
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                    }
                };
                
                Response::success(id, serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": content_text
                        }
                    ]
                }))
            }
            Err(e) => {
                error!("工具 {} 执行失败: {}", tool_name, e);
                Response::error(id, -32603, format!("工具执行失败: {}", e))
            }
        }
    }

    async fn handle_health_check(&self, id: String) -> Response {
        debug!("处理健康检查请求");
        
        let server = self.mcp_server.read().await;
        match server.get_tool_health_status().await {
            Ok(health_status) => {
                let overall_status = if health_status.values().all(|h| matches!(h, ToolHealth::Healthy)) {
                    "healthy"
                } else if health_status.values().any(|h| matches!(h, ToolHealth::Unhealthy { .. })) {
                    "unhealthy"
                } else {
                    "degraded"
                };
                
                info!("健康检查完成，状态: {}", overall_status);
                Response::success(id, serde_json::json!({
                    "overall_status": overall_status,
                    "tool_health": health_status,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }
            Err(e) => {
                error!("健康检查失败: {}", e);
                Response::error(id, -32603, format!("健康检查失败: {}", e))
            }
        }
    }

    async fn handle_stats_request(&self, id: String) -> Response {
        debug!("处理统计信息请求");
        
        let server = self.mcp_server.read().await;
        match server.get_performance_stats().await {
            Ok(stats) => {
                let tool_count = server.get_tool_count().await.unwrap_or(0);
                
                info!("成功获取性能统计信息");
                Response::success(id, serde_json::json!({
                    "tool_count": tool_count,
                    "performance_stats": stats,
                    "server_info": {
                        "name": self.name,
                        "version": self.version,
                        "uptime": "not_implemented"
                    },
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }
            Err(e) => {
                error!("获取统计信息失败: {}", e);
                Response::error(id, -32603, format!("获取统计信息失败: {}", e))
            }
        }
    }

    async fn handle_batch_tool_call(&self, id: String, params: &Value) -> Response {
        debug!("处理批量工具调用请求: {:?}", params);
        
        let requests = match params.get("requests").and_then(|v| v.as_array()) {
            Some(reqs) => reqs,
            None => {
                warn!("批量工具调用请求缺少requests参数");
                return Response::error(id, -32602, "缺少requests参数".to_string());
            }
        };

        let mut tool_requests = Vec::new();
        for req in requests {
            if let (Some(tool_name), tool_params) = (
                req.get("name").and_then(|v| v.as_str()),
                req.get("arguments").unwrap_or(&Value::Null).clone()
            ) {
                let timeout = req.get("timeout")
                    .and_then(|v| v.as_u64())
                    .map(|t| Duration::from_secs(t));
                
                tool_requests.push(ToolRequest {
                    tool_name: tool_name.to_string(),
                    params: tool_params,
                    timeout,
                });
            }
        }

        if tool_requests.is_empty() {
            return Response::error(id, -32602, "没有有效的工具请求".to_string());
        }

        let server = self.mcp_server.read().await;
        match server.batch_execute_tools(tool_requests).await {
            Ok(results) => {
                info!("批量工具执行完成，共 {} 个结果", results.len());
                Response::success(id, serde_json::json!({
                    "results": results
                }))
            }
            Err(e) => {
                error!("批量工具执行失败: {}", e);
                Response::error(id, -32603, format!("批量工具执行失败: {}", e))
            }
        }
    }

    async fn send_error_async(
        &self,
        writer: &mut tokio::io::Stdout,
        id: &str,
        code: i32,
        message: &str,
    ) -> Result<()> {
        let error_response = Response::error(id.to_string(), code, message.to_string());
        let response_json = serde_json::to_string(&error_response)?;
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
        let mcp_server = MCPServer::new();
        let mut server = Server::new(
            "Test Server".to_string(),
            "1.0.0".to_string(),
            mcp_server,
        );

        assert!(!server.initialized);
    }
}
