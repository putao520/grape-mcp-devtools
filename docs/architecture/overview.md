# Grape MCP DevTools 系统架构设计文档

## 项目概览

**Grape MCP DevTools** 是一个基于 MCP (Model Context Protocol) 协议的**开发支持工具集合**，专注于为AI编程助手提供高质量的开发环境信息和项目上下文。项目遵循"简洁实用、第三方优先、Windows友好"的设计原则。

### 🎯 核心价值使命

**专注开发支持**：为开发团队和AI编程助手提供准确、及时的开发环境信息

🔧 **工具集合定位**：提供一组精心设计的MCP工具，每个工具专注于特定的开发支持功能

📋 **项目上下文感知**：获取GitHub项目信息、任务状态和技术背景，帮助AI更好地理解项目

🌐 **第三方集成优先**：充分利用现有的成熟服务和工具，避免重复造轮子

🖥️ **Windows环境优化**：专门针对Windows开发环境设计和优化

### 典型应用场景

1. **AI编程助手增强**：为Claude、GPT等AI提供项目上下文信息
2. **开发环境诊断**：快速检测和分析当前开发环境配置
3. **依赖管理支持**：版本检查、安全扫描、兼容性分析
4. **文档快速查找**：跨语言的包文档和API信息搜索
5. **项目背景理解**：GitHub任务信息和技术栈分析

### 项目基本信息
- **项目名称**: grape-mcp-devtools
- **版本**: 3.0.0
- **协议基础**: MCP (Model Context Protocol)
- **核心价值**: 开发支持工具集合
- **目标用户**: AI编程助手、开发者、IDE插件
- **支持语言**: Rust, Python, JavaScript/TypeScript, Java, Go, Dart/Flutter

## 核心架构理念

### 简洁三层架构

**清晰的职责分层**：
- **第一层**：MCP协议层（通信和协议处理）
- **第二层**：工具服务层（具体功能实现）
- **第三层**：第三方集成层（外部服务调用）

**工具集合协调**：
- 每个工具独立实现特定功能
- 通过MCP协议提供统一接口
- 支持工具的动态注册和管理

### 🏗️ 实际技术架构

**核心组件**：
- `MCPServer`：标准MCP协议服务器实现
- `ToolRegistry`：工具注册和管理中心
- `ConfigManager`：配置和环境管理
- `MCPClient`：外部MCP工具集成（Playwright等）
- 各种专门化工具：文档搜索、版本检查、安全扫描等

## 整体架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI客户端 (Claude, GPT, IDE插件)              │
│                         ⬇ MCP请求                              │
└─────────────────────┬───────────────────────────────────────────┘
                      │ MCP Protocol (stdio)
┌─────────────────────▼───────────────────────────────────────────┐
│                   MCP协议层                                     │
│  ┌─────────────────┬─────────────────┬─────────────────────────┐ │
│  │ 协议处理器       │ 请求路由器       │ 响应格式化器             │ │
│  │ ProtocolHandler │ RequestRouter   │ ResponseFormatter       │ │
│  └─────────────────┴─────────────────┴─────────────────────────┘ │
└─────────────────────┬───────────────────────────────────────────┘
                      │ 工具调用
┌─────────────────────▼───────────────────────────────────────────┐
│                  工具服务层                                      │
│  ┌─────────────────┬─────────────────┬─────────────────────────┐ │
│  │ 文档搜索工具     │ 版本检查工具     │ 安全扫描工具             │ │
│  │ SearchDocs      │ CheckVersion    │ SecurityCheck           │ │
│  └─────────────────┴─────────────────┴─────────────────────────┘ │
│  ┌─────────────────┬─────────────────┬─────────────────────────┐ │
│  │ 环境检测工具     │ GitHub集成工具   │ 外部工具代理             │ │
│  │ EnvDetector     │ GitHubIntegration│ ExternalToolProxy      │ │
│  └─────────────────┴─────────────────┴─────────────────────────┘ │
│  ┌─────────────────┬─────────────────┬─────────────────────────┐ │
│  │ 工具注册中心     │ 配置管理器       │ 缓存管理器               │ │
│  │ ToolRegistry    │ ConfigManager   │ CacheManager            │ │
│  └─────────────────┴─────────────────┴─────────────────────────┘ │
└─────────────────────┬───────────────────────────────────────────┘
                      │ 第三方服务调用
┌─────────────────────▼───────────────────────────────────────────┐
│                  第三方集成层                                    │
│  ┌─────────────────┬─────────────────┬─────────────────────────┐ │
│  │ GitHub API      │ 包管理器API      │ 官方文档站点             │ │
│  │ REST API调用    │ crates.io/npm   │ docs.rs/readthedocs     │ │
│  └─────────────────┴─────────────────┴─────────────────────────┘ │
│  ┌─────────────────┬─────────────────┬─────────────────────────┐ │
│  │ MCP外部工具     │ 安全数据库       │ CLI工具集成              │ │
│  │ Playwright等    │ OSV/NVD API     │ cargo/pip/npm           │ │
│  └─────────────────┴─────────────────┴─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## 核心组件详细设计

### 1. MCP协议层

#### MCPServer - 协议服务器
```rust
pub struct MCPServer {
    tool_registry: Arc<ToolRegistry>,
    config_manager: Arc<ConfigManager>,
    session_state: Arc<RwLock<SessionState>>,
}

impl MCPServer {
    pub async fn run(&self) -> Result<()> {
        // 标准MCP协议服务器主循环
        // 处理initialize、tools/list、tools/call等标准请求
    }
    
    pub async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        // 请求分发和处理
        match request {
            MCPRequest::Initialize(_) => self.handle_initialize().await,
            MCPRequest::ToolsList => self.handle_tools_list().await,
            MCPRequest::ToolsCall(params) => self.handle_tools_call(params).await,
            _ => MCPResponse::Error("不支持的请求类型".to_string()),
        }
    }
}
```

#### 协议消息处理
```rust
pub enum MCPRequest {
    Initialize(InitializeParams),
    ToolsList,
    ToolsCall(ToolCallParams),
    Shutdown,
}

pub enum MCPResponse {
    Initialize(InitializeResult),
    ToolsList(ToolsListResult),
    ToolsCall(ToolCallResult),
    Error(String),
}
```

### 2. 工具服务层

#### 工具注册中心
```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn MCPTool>>,
    metadata: HashMap<String, ToolMetadata>,
}

impl ToolRegistry {
    pub fn register_tool(&mut self, name: String, tool: Arc<dyn MCPTool>) {
        // 注册工具实例
    }
    
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn MCPTool>> {
        // 获取工具实例
    }
    
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        // 返回所有可用工具信息
    }
}
```

#### 标准工具接口
```rust
pub trait MCPTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult>;
}

pub struct ToolResult {
    pub content: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}
```

### 3. 具体工具实现

#### 文档搜索工具
```rust
pub struct SearchDocsTool {
    http_client: reqwest::Client,
    cache_manager: Arc<CacheManager>,
}

impl MCPTool for SearchDocsTool {
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let language = params["language"].as_str().unwrap_or("rust");
        let query = params["query"].as_str().unwrap_or("");
        
        // 1. 检查缓存
        if let Some(cached) = self.cache_manager.get(&format!("docs_{}_{}", language, query)).await? {
            return Ok(cached);
        }
        
        // 2. 调用对应语言的文档API
        let result = match language {
            "rust" => self.search_rust_docs(query).await?,
            "python" => self.search_python_docs(query).await?,
            "javascript" => self.search_js_docs(query).await?,
            _ => return Err(anyhow::anyhow!("不支持的语言: {}", language)),
        };
        
        // 3. 缓存结果
        self.cache_manager.set(&format!("docs_{}_{}", language, query), &result).await?;
        
        Ok(result)
    }
}
```

#### GitHub集成工具
```rust
pub struct GitHubIntegrationTool {
    github_client: GitHubClient,
    cache_manager: Arc<CacheManager>,
}

impl MCPTool for GitHubIntegrationTool {
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let repo = params["repo"].as_str().unwrap_or("");
        let info_type = params["type"].as_str().unwrap_or("basic");
        
        match info_type {
            "basic" => self.get_repo_basic_info(repo).await,
            "issues" => self.get_repo_issues(repo).await,
            "tech_stack" => self.analyze_tech_stack(repo).await,
            _ => Err(anyhow::anyhow!("不支持的信息类型: {}", info_type)),
        }
    }
    
    async fn get_repo_basic_info(&self, repo: &str) -> Result<ToolResult> {
        // 获取仓库基本信息：描述、语言、stars等
    }
    
    async fn get_repo_issues(&self, repo: &str) -> Result<ToolResult> {
        // 获取当前开放的issues和最近更新的任务
    }
    
    async fn analyze_tech_stack(&self, repo: &str) -> Result<ToolResult> {
        // 分析仓库的技术栈：主要语言、框架、工具等
    }
}
```

### 4. 第三方集成层

#### MCP客户端集成
```rust
pub struct MCPClientManager {
    clients: HashMap<String, MCPClient>,
    config: MCPClientConfig,
}

impl MCPClientManager {
    pub async fn call_external_tool(&self, server_name: &str, tool_name: &str, params: serde_json::Value) -> Result<ToolResult> {
        // 调用外部MCP工具（如Playwright）
        let client = self.clients.get(server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP服务器未配置: {}", server_name))?;
        
        client.call_tool(tool_name, params).await
    }
}
```

#### 第三方API集成
```rust
pub struct ThirdPartyAPIManager {
    github_client: GitHubClient,
    crates_io_client: CratesIOClient,
    npm_client: NPMClient,
    http_client: reqwest::Client,
}

impl ThirdPartyAPIManager {
    pub async fn search_package(&self, ecosystem: &str, query: &str) -> Result<PackageSearchResult> {
        match ecosystem {
            "rust" => self.crates_io_client.search(query).await,
            "javascript" => self.npm_client.search(query).await,
            "python" => self.pypi_search(query).await,
            _ => Err(anyhow::anyhow!("不支持的生态系统")),
        }
    }
}
```

## 🔧 工具目录设计

### 当前规划的工具列表

1. **search_docs** - 跨语言文档搜索
2. **check_version** - 包版本检查和比较
3. **security_check** - 依赖安全扫描
4. **environment_detect** - 开发环境检测
5. **github_info** - GitHub项目信息获取
6. **dependency_analyze** - 依赖关系分析
7. **tool_installer** - 开发工具安装检测
8. **external_tool_proxy** - 外部MCP工具代理

### 工具开发规范

每个工具应当：
- 实现标准的`MCPTool` trait
- 提供清晰的JSON Schema描述
- 支持结果缓存（适当时）
- 提供详细的错误信息
- 在Windows环境下测试通过

## 🚀 部署和运行

### 系统要求
- **操作系统**: Windows 10/11 (主要支持)
- **运行时**: Rust 1.70+ 
- **网络**: 可访问GitHub API和各包管理器API
- **权限**: 读取本地项目文件的权限

### 启动流程
1. 加载配置文件和环境变量
2. 初始化工具注册中心
3. 注册所有可用工具
4. 启动MCP协议服务器
5. 等待客户端连接和请求

### 性能特点
- **启动时间**: < 2秒
- **内存占用**: < 50MB
- **响应延迟**: 大部分工具 < 1秒
- **并发支持**: 支持多个客户端同时连接

## 🔄 扩展和维护

### 添加新工具
1. 实现`MCPTool` trait
2. 在`ToolRegistry`中注册
3. 添加相应的测试用例
4. 更新文档

### 配置管理
- 使用标准的配置文件格式
- 支持环境变量覆盖
- 提供配置验证和默认值

### 监控和日志
- 标准的Rust日志框架
- 工具调用统计和性能监控
- 错误跟踪和报告

---

*架构文档版本：v3.0*  
*最后更新：2025年1月* 