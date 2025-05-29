# 🤖 Grape MCP DevTools 客户端

这是一套完整的MCP客户端工具，用于测试和使用Grape MCP DevTools服务器。

## 📋 功能概述

### 🔧 简易MCP客户端 (`simple_mcp_client.py`)
- **基础MCP通信**：与MCP服务器进行标准协议通信
- **工具调用测试**：测试所有MCP工具的功能
- **交互式模式**：手动测试各种工具和参数
- **自动化测试**：预定义的测试套件

### 🤖 智能MCP客户端 (`mcp_client.py`)
- **AI对话功能**：集成LLM，智能理解用户需求
- **自动工具选择**：AI自动选择合适的MCP工具
- **上下文对话**：保持对话历史，支持多轮交互
- **工具链调用**：AI可以组合使用多个工具

### 🔧 环境设置脚本 (`setup_mcp_client.py`)
- **依赖自动安装**：自动安装所需的Python库
- **环境检查**：验证Python和Rust环境
- **配置文件生成**：自动创建.env配置文件
- **快速测试脚本**：生成便捷的测试脚本

## 🚀 快速开始

### 1. 环境设置

```bash
# 运行自动设置脚本
python setup_mcp_client.py

# 或手动安装依赖
pip install rich httpx python-dotenv click asyncio-subprocess
```

### 2. 配置环境变量

编辑生成的`.env`文件：

```bash
# LLM配置 (用于AI对话功能)
LLM_API_BASE_URL=https://integrate.api.nvidia.com/v1
LLM_API_KEY=your-actual-api-key
LLM_MODEL_NAME=nvidia/llama-3.1-nemotron-70b-instruct

# Embedding配置 (用于向量化功能)
EMBEDDING_API_BASE_URL=https://integrate.api.nvidia.com/v1
EMBEDDING_API_KEY=your-actual-api-key
EMBEDDING_MODEL_NAME=nvidia/nv-embedqa-mistral-7b-v2
```

### 3. 测试MCP服务器

```bash
# 基础功能测试
python simple_mcp_client.py test

# 交互式测试
python simple_mcp_client.py interactive

# 快速测试
python quick_test.py
```

### 4. 启动AI对话

```bash
# 智能对话模式 (需要LLM API密钥)
python mcp_client.py chat
```

## 💡 使用示例

### 📱 简易客户端使用

```bash
# 运行自动测试
python simple_mcp_client.py test

# 交互式模式
python simple_mcp_client.py interactive
# 然后输入命令:
# - list: 显示可用工具
# - test: 运行预定义测试
# - search_docs: 测试文档搜索
# - check_version: 测试版本检查
# - quit: 退出
```

### 🤖 智能客户端对话示例

```
🤔 你: 我想了解Rust中的HTTP客户端库

🤖 助手: 我来帮你搜索Rust的HTTP客户端库信息...
[自动调用 search_docs 工具]

🤔 你: reqwest的最新版本是什么？

🤖 助手: 让我查询reqwest的版本信息...
[自动调用 check_version 工具]

🤔 你: 能给我一些reqwest的API文档吗？

🤖 助手: 我来获取reqwest的API文档...
[自动调用 get_api_docs 工具]
```

### 🔧 直接工具调用

```bash
# 直接调用特定工具
python mcp_client.py call \
  --tool-name search_docs \
  --args '{"query": "HTTP client", "language": "rust", "limit": 5}'
```

## 🧪 测试功能

### 自动化测试
- **文档搜索测试**：测试多种语言的文档搜索
- **版本检查测试**：验证版本查询功能
- **API文档测试**：测试API文档获取
- **错误处理测试**：验证错误情况的处理

### 交互式测试
- **实时工具调用**：手动测试各种参数组合
- **结果验证**：查看详细的返回结果
- **性能测试**：观察响应时间和效率

## 🔍 功能详解

### MCP协议支持
- **标准初始化**：符合MCP 2025-03-26协议
- **工具发现**：自动获取服务器支持的工具列表
- **错误处理**：完整的错误响应处理
- **版本兼容**：协议版本验证

### 工具调用能力
- **search_docs**：文档和包信息搜索
- **check_version**：包版本查询和兼容性检查
- **get_api_docs**：API文档和使用示例获取
- **vector_docs**：向量化文档管理

### AI智能功能
- **自然语言理解**：理解用户的查询意图
- **工具自动选择**：根据问题选择最合适的工具
- **参数智能填充**：自动生成工具调用参数
- **结果整理汇总**：将多个工具结果整合为友好回复

## 🔧 故障排除

### 常见问题

#### MCP服务器启动失败
```bash
# 检查Rust环境
cargo --version

# 手动编译测试
cargo check --bin grape-mcp-devtools

# 查看详细错误
cargo run --bin grape-mcp-devtools
```

#### Python依赖问题
```bash
# 升级pip
python -m pip install --upgrade pip

# 重新安装依赖
pip install -r requirements.txt

# 检查Python版本
python --version  # 需要3.7+
```

#### API配置问题
```bash
# 检查环境变量
python -c "import os; print(os.getenv('LLM_API_KEY'))"

# 验证API连接
python -c "import httpx; print(httpx.get('https://integrate.api.nvidia.com/v1/models').status_code)"
```

### 调试模式

```python
# 启用详细日志
import logging
logging.basicConfig(level=logging.DEBUG)

# 查看MCP通信详情
# 在客户端代码中添加调试打印
```

## 📚 开发指南

### 扩展客户端功能

```python
# 添加新的测试案例
async def test_custom_tool(client):
    """自定义工具测试"""
    result = await client.call_tool("custom_tool", {
        "param1": "value1",
        "param2": "value2"
    })
    # 处理结果...

# 扩展AI功能
def add_custom_prompt():
    """添加自定义系统提示"""
    return """
    你是一个专门的代码助手...
    [自定义提示内容]
    """
```

### 集成到其他项目

```python
from simple_mcp_client import SimpleMCPClient

# 在你的项目中使用MCP客户端
async def use_mcp_in_project():
    client = SimpleMCPClient()
    await client.start_server(["cargo", "run", "--bin", "grape-mcp-devtools"])
    
    # 使用MCP功能
    await client.initialize()
    await client.list_tools()
    result = await client.call_tool("search_docs", {"query": "example"})
    
    await client.stop_server()
```

## 🎯 最佳实践

1. **环境隔离**：使用虚拟环境安装Python依赖
2. **配置管理**：将敏感信息存储在.env文件中
3. **错误处理**：总是检查MCP调用的返回结果
4. **资源清理**：确保正确关闭MCP服务器进程
5. **测试覆盖**：使用多种测试案例验证功能

## 📖 相关文档

- [MCP协议规范](https://github.com/anthropic/mcp)
- [Grape MCP DevTools服务器文档](../README.md)
- [环境配置指南](../ENV_CONFIG_GUIDE.md)
- [现代化优化报告](../MODERNIZATION_REPORT.md)

---

**🎉 现在你可以开始使用MCP客户端了！**

选择合适的客户端模式：
- 📱 **简单测试**：`python simple_mcp_client.py test`
- 🤖 **AI对话**：`python mcp_client.py chat`  
- 🔧 **交互调试**：`python simple_mcp_client.py interactive` 