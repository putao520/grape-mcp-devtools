# Grape MCP DevTools 快速开始指南

## 🚀 5分钟快速上手

本指南帮助您快速配置和使用 Grape MCP DevTools，为AI编程助手提供开发支持工具。

## 📋 系统要求

### 基础环境
- **操作系统**: Windows 10/11 (主要支持)
- **运行时**: Rust 1.70+
- **网络**: 可访问GitHub API和包管理器API
- **权限**: 读取本地项目文件的权限

### 可选工具
- **Git**: 用于GitHub集成功能
- **PowerShell**: 推荐使用PowerShell 7+
- **开发工具**: cargo、npm、pip等（根据需要）

## 🛠️ 安装和配置

### 步骤1: 克隆项目
```powershell
git clone https://github.com/your-org/grape-mcp-devtools.git
cd grape-mcp-devtools
```

### 步骤2: 构建项目
```powershell
cargo build --release
```

### 步骤3: 配置环境变量
创建 `.env` 文件（如果不存在）：
```env
# GitHub API配置（可选）
GITHUB_TOKEN=your_github_token_here

# 日志级别
RUST_LOG=info

# 缓存配置
CACHE_TTL_HOURS=24
MAX_CACHE_SIZE_MB=100
```

### 步骤4: 测试安装
```powershell
cargo run -- --help
```

## 🔧 MCP客户端配置

### Claude Desktop配置
在 `%APPDATA%\Claude\claude_desktop_config.json` 中添加：
```json
{
  "mcpServers": {
    "grape-devtools": {
      "command": "path/to/grape-mcp-devtools.exe",
      "args": [],
      "env": {}
    }
  }
}
```

### Cursor配置
在 `~/.cursor/mcp.json` 中添加：
```json
{
  "mcpServers": {
    "grape-devtools": {
      "command": "path/to/grape-mcp-devtools.exe",
      "args": [],
      "env": {}
    }
  }
}
```

## 🎯 基本使用示例

### 1. 搜索文档
```json
{
  "method": "tools/call",
  "params": {
    "name": "search_docs",
    "arguments": {
      "language": "rust",
      "query": "async programming",
      "limit": 5
    }
  }
}
```

### 2. 检查版本
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

### 3. 环境检测
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

### 4. GitHub项目信息
```json
{
  "method": "tools/call",
  "params": {
    "name": "github_info",
    "arguments": {
      "repo": "microsoft/vscode",
      "type": "basic"
    }
  }
}
```

## 🔍 可用工具列表

### 核心工具
- **search_docs**: 跨语言文档搜索
- **check_version**: 包版本检查和比较
- **security_check**: 依赖安全扫描
- **environment_detect**: 开发环境检测
- **github_info**: GitHub项目信息获取
- **dependency_analyze**: 依赖关系分析

### 系统工具
- **tool_installer**: 开发工具安装检测
- **external_tool_proxy**: 外部MCP工具代理

## 📝 配置文件说明

### 主配置文件 (config.toml)
```toml
[server]
# MCP服务器配置
host = "localhost"
port = 3000
log_level = "info"

[cache]
# 缓存配置
ttl_hours = 24
max_size_mb = 100
enable_disk_cache = true

[github]
# GitHub API配置
api_url = "https://api.github.com"
timeout_seconds = 30
rate_limit_per_hour = 5000

[tools]
# 工具配置
enable_all = true
timeout_seconds = 60
max_concurrent = 4
```

### 外部工具配置 (mcp_clients.json)
```json
{
  "playwright": {
    "command": "npx",
    "args": ["-y", "@executeautomation/playwright-mcp-server"],
    "env": {},
    "timeout": 30000
  }
}
```

## 🧪 测试配置

### 验证工具功能
```powershell
# 测试文档搜索
cargo run -- test search_docs --query "async" --language "rust"

# 测试版本检查
cargo run -- test check_version --package "tokio" --ecosystem "rust"

# 测试环境检测
cargo run -- test environment_detect
```

### 验证MCP连接
```powershell
# 启动MCP服务器
cargo run

# 在另一个终端测试连接
echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0.0"}}, "id": 1}' | cargo run
```

## 🔧 常见问题解决

### 问题1: stdio通信失败
**症状**: MCP客户端无法连接
**解决**: 确保使用PowerShell 7+，检查路径配置

### 问题2: GitHub API限制
**症状**: GitHub相关工具返回错误
**解决**: 配置GITHUB_TOKEN环境变量

### 问题3: 工具超时
**症状**: 工具调用超时
**解决**: 增加timeout_seconds配置值

### 问题4: 缓存问题
**症状**: 返回过期数据
**解决**: 清理缓存目录或减少ttl_hours

## 📚 进阶使用

### 自定义工具开发
1. 实现 `MCPTool` trait
2. 在 `ToolRegistry` 中注册
3. 添加配置和测试
4. 更新文档

### 性能优化
- 调整缓存配置
- 优化并发设置
- 监控资源使用

### 集成外部工具
- 配置MCP客户端
- 添加工具代理
- 测试集成功能

## 🆘 获取帮助

### 文档资源
- [系统架构概览](../architecture/overview.md)
- [工具描述指南](../development/tool-description-guide.md)
- [MCP协议说明](mcp-protocol.md)

### 社区支持
- 提交Issue报告问题
- 参与讨论和改进
- 贡献代码和文档

---

*快速开始指南版本：v3.0*  
*最后更新：2025年1月*  
*适用于简化架构设计* 