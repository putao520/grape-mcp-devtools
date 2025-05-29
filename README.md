# Grape MCP DevTools

一个基于 MCP (Model Context Protocol) 的多语言文档服务，专为 LLM 提供文档查询和版本检查功能。

## 功能特性

- 🔍 **文档搜索** - 搜索各种编程语言的包信息、API文档和使用指南
- 📦 **版本检查** - 获取包的最新版本、版本历史和兼容性信息
- 📚 **API文档** - 获取编程语言API的详细文档信息
- 🚀 **MCP协议** - 基于标准MCP协议，支持stdio模式通信

## 支持的语言和包管理器

- **Rust** - Cargo
- **JavaScript/TypeScript** - npm
- **Python** - pip
- **Java** - Maven
- **Go** - Go modules
- **Dart** - pub

## 安装和运行

### 前置要求

- Rust 1.70+
- 配置环境变量（可选）

### 环境变量配置

创建 `.env` 文件（可选，用于向量化功能）：

```env
EMBEDDING_API_KEY=your_nvidia_api_key
EMBEDDING_API_BASE_URL=https://integrate.api.nvidia.com/v1
EMBEDDING_MODEL_NAME=nvidia/nv-embedcode-7b-v1
```

### 编译和运行

```bash
# 编译项目
cargo build --release

# 运行MCP服务器（stdio模式）
cargo run --bin grape-mcp-devtools

# 运行测试
cargo run --bin mcp_server_test
```

## MCP协议使用

### 初始化

```json
{
  "jsonrpc": "2.0",
  "version": "2025-03-26",
  "id": "1",
  "method": "initialize",
  "params": {
    "client_name": "your-client",
    "client_version": "1.0.0",
    "capabilities": ["documentSearch"]
  }
}
```

### 获取工具列表

```json
{
  "jsonrpc": "2.0",
  "version": "2025-03-26",
  "id": "2",
  "method": "tools/list",
  "params": {}
}
```

### 调用工具

#### 搜索文档

```json
{
  "jsonrpc": "2.0",
  "version": "2025-03-26",
  "id": "3",
  "method": "tools/call",
  "params": {
    "name": "search_docs",
    "arguments": {
      "query": "HTTP client library",
      "language": "rust",
      "max_results": 10
    }
  }
}
```

#### 检查版本

```json
{
  "jsonrpc": "2.0",
  "version": "2025-03-26",
  "id": "4",
  "method": "tools/call",
  "params": {
    "name": "check_latest_version",
    "arguments": {
      "type": "cargo",
      "name": "reqwest"
    }
  }
}
```

#### 获取API文档

```json
{
  "jsonrpc": "2.0",
  "version": "2025-03-26",
  "id": "5",
  "method": "tools/call",
  "params": {
    "name": "get_api_docs",
    "arguments": {
      "language": "rust",
      "package": "std",
      "symbol": "Vec"
    }
  }
}
```

## 可用工具

### 1. search_docs

搜索编程语言的包信息和文档。

**参数：**
- `query` (必需) - 要搜索的功能或技术需求
- `language` (必需) - 目标编程语言
- `max_results` (可选) - 最大结果数 (1-100)
- `scope` (可选) - 搜索范围: api|tutorial|best_practices

### 2. check_latest_version

获取包的版本信息。

**参数：**
- `type` (必需) - 包管理器类型 (cargo/npm/pip/maven/go/pub)
- `name` (必需) - 包名称
- `include_preview` (可选) - 是否包含预览版本

### 3. get_api_docs

获取API的详细文档。

**参数：**
- `language` (必需) - 编程语言
- `package` (必需) - 包名称
- `symbol` (必需) - API符号
- `version` (可选) - API版本

## 开发和测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行MCP服务器测试
cargo run --bin mcp_server_test

# 运行特定测试
cargo test --test integration_tests
```

### 开发模式

```bash
# 启用详细日志
RUST_LOG=debug cargo run --bin grape-mcp-devtools

# 检查代码
cargo check
cargo clippy
```

## 架构说明

### 核心组件

- **MCP服务器** (`src/mcp/server.rs`) - 处理MCP协议通信
- **工具系统** (`src/tools/`) - 实现各种文档查询工具
- **向量化系统** (`src/vectorization/`) - 文档向量化和相似度搜索
- **存储系统** (`src/storage/`) - 文档存储和索引

### 通信模式

本项目专注于 **stdio模式** 的MCP服务器：

- 通过标准输入/输出进行JSON-RPC通信
- 支持异步请求处理
- 完全兼容MCP协议规范

## 许可证

MIT License

## 贡献

欢迎提交Issue和Pull Request！

## 更新日志

### v0.1.0
- ✅ 实现基础MCP服务器（stdio模式）
- ✅ 添加文档搜索工具
- ✅ 添加版本检查工具
- ✅ 添加API文档工具
- ✅ 移除HTTP服务器依赖，专注stdio模式
- ✅ 完整的测试覆盖 