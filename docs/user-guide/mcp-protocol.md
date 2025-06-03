# Grape MCP DevTools MCP协议使用指南

## 📋 概述

本指南介绍如何在 **Grape MCP DevTools** 中使用 Model Context Protocol (MCP)，以及如何配置AI客户端与我们的工具服务器进行通信。

## 🔧 MCP协议基础

### 什么是MCP
MCP (Model Context Protocol) 是一个开放协议，用于连接AI应用和外部工具服务。在我们的项目中：

- **服务器角色**：Grape MCP DevTools 作为MCP服务器，提供开发支持工具
- **客户端角色**：Claude Desktop、Cursor等AI编程助手作为MCP客户端
- **通信方式**：通过stdio（标准输入输出）进行JSON-RPC通信

### 核心概念
- **工具 (Tools)**：我们提供的具体功能，如文档搜索、版本检查等
- **资源 (Resources)**：可选，我们主要通过工具提供信息
- **提示 (Prompts)**：可选，未来可能支持

## 🚀 客户端配置

### Claude Desktop配置

在 `%APPDATA%\Claude\claude_desktop_config.json` 中添加：

```json
{
  "mcpServers": {
    "grape-devtools": {
      "command": "C:\\path\\to\\grape-mcp-devtools.exe",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Cursor配置

在 Cursor 的 MCP 配置中添加：

```json
{
  "mcpServers": {
    "grape-devtools": {
      "command": "C:\\path\\to\\grape-mcp-devtools.exe",
      "args": ["--mode", "stdio"],
      "env": {}
    }
  }
}
```

### VSCode with Continue配置

在 Continue 插件配置中：

```json
{
  "mcpServers": [
    {
      "name": "grape-devtools",
      "command": "C:\\path\\to\\grape-mcp-devtools.exe",
      "args": [],
      "env": {}
    }
  ]
}
```

## 🔧 可用工具

### 1. search_docs - 文档搜索
```json
{
  "method": "tools/call",
  "params": {
    "name": "search_docs",
    "arguments": {
      "language": "rust",
      "query": "async programming",
      "limit": 10
    }
  }
}
```

**参数说明**：
- `language`: 编程语言（rust、python、javascript、java、go、dart）
- `query`: 搜索关键词
- `limit`: 结果数量限制（可选，默认10）

### 2. check_version - 版本检查
```json
{
  "method": "tools/call",
  "params": {
    "name": "check_version",
    "arguments": {
      "package": "tokio",
      "ecosystem": "rust"
    }
  }
}
```

**参数说明**：
- `package`: 包名称
- `ecosystem`: 包管理器生态系统（rust、npm、pypi、maven）

### 3. environment_detect - 环境检测
```json
{
  "method": "tools/call",
  "params": {
    "name": "environment_detect",
    "arguments": {
      "check_languages": true,
      "check_tools": true
    }
  }
}
```

**参数说明**：
- `check_languages`: 检测编程语言环境
- `check_tools`: 检测开发工具

### 4. github_info - GitHub信息
```json
{
  "method": "tools/call",
  "params": {
    "name": "github_info",
    "arguments": {
      "repo": "microsoft/vscode",
      "type": "basic",
      "include_details": false
    }
  }
}
```

**参数说明**：
- `repo`: GitHub仓库路径
- `type`: 信息类型（basic、tasks、tech_stack、recent_activity）
- `include_details`: 是否包含详细信息

### 5. security_check - 安全检查
```json
{
  "method": "tools/call",
  "params": {
    "name": "security_check",
    "arguments": {
      "package": "axios",
      "ecosystem": "npm",
      "check_vulnerabilities": true
    }
  }
}
```

### 6. external_tool_proxy - 外部工具代理
```json
{
  "method": "tools/call",
  "params": {
    "name": "external_tool_proxy",
    "arguments": {
      "server": "playwright",
      "tool": "screenshot",
      "params": {
        "url": "https://example.com"
      }
    }
  }
}
```

## 📊 响应格式

### 成功响应
```json
{
  "content": [
    {
      "type": "text",
      "text": "搜索结果或工具输出内容"
    }
  ],
  "metadata": {
    "tool": "search_docs",
    "timestamp": "2025-01-01T12:00:00Z",
    "source": "third_party_api"
  }
}
```

### 错误响应
```json
{
  "error": {
    "code": -32602,
    "message": "参数验证失败: 缺少必需参数 'language'"
  }
}
```

## 🛠️ 调试和测试

### 手动测试连接

使用PowerShell测试MCP连接：

```powershell
# 启动工具服务器
.\grape-mcp-devtools.exe

# 在另一个终端测试初始化
$initMessage = @{
    jsonrpc = "2.0"
    method = "initialize"
    params = @{
        protocolVersion = "2024-11-05"
        capabilities = @{}
        clientInfo = @{
            name = "test-client"
            version = "1.0.0"
        }
    }
    id = 1
} | ConvertTo-Json -Depth 5

echo $initMessage | .\grape-mcp-devtools.exe
```

### 工具列表查询
```powershell
$listTools = @{
    jsonrpc = "2.0"
    method = "tools/list"
    id = 2
} | ConvertTo-Json

echo $listTools | .\grape-mcp-devtools.exe
```

### 工具调用测试
```powershell
$callTool = @{
    jsonrpc = "2.0"
    method = "tools/call"
    params = @{
        name = "environment_detect"
        arguments = @{
            check_languages = $true
        }
    }
    id = 3
} | ConvertTo-Json -Depth 5

echo $callTool | .\grape-mcp-devtools.exe
```

## 🔧 故障排除

### 常见问题

#### 1. 连接失败
**症状**：客户端无法连接到MCP服务器
**解决**：
- 检查可执行文件路径是否正确
- 确保在PowerShell环境下运行
- 检查环境变量配置

#### 2. 工具调用超时
**症状**：工具调用长时间无响应
**解决**：
- 检查网络连接（某些工具需要访问外部API）
- 增加客户端超时时间
- 查看日志文件了解详细错误

#### 3. 参数验证错误
**症状**：工具返回参数验证失败
**解决**：
- 检查必需参数是否提供
- 确认参数类型正确
- 参考工具schema验证参数格式

### 日志配置

设置环境变量启用详细日志：

```env
RUST_LOG=debug
```

或在配置文件中设置：
```toml
[logging]
level = "debug"
targets = ["stdout", "file"]
```

## 📚 进阶用法

### 自定义配置

在 `config.toml` 中自定义工具行为：

```toml
[tools.search_docs]
timeout_seconds = 30
cache_ttl_hours = 24
default_limit = 10

[tools.github_info]
timeout_seconds = 15
enable_cache = true
default_include_details = false
```

### 环境变量覆盖

使用环境变量覆盖配置：

```env
# GitHub API配置
GITHUB_TOKEN=your_token_here
GITHUB_API_TIMEOUT=30

# 缓存配置
CACHE_TTL_HOURS=12
MAX_CACHE_SIZE_MB=50
```

## 🤝 最佳实践

### 客户端使用建议
1. **合理设置超时**：大部分工具在1-5秒内响应，设置10-30秒超时
2. **缓存友好**：相同参数的工具调用会使用缓存，提高响应速度
3. **错误处理**：实现适当的错误处理和重试机制
4. **日志监控**：启用日志以便调试和监控

### 性能优化
1. **并发调用**：多个工具调用可以并行执行
2. **参数优化**：使用 `limit` 参数控制返回数据量
3. **缓存利用**：充分利用工具的缓存机制

---

*MCP协议使用指南版本：v3.0*  
*最后更新：2025年1月*  
*适用于Grape MCP DevTools*
