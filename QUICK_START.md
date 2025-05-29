# 🚀 Grape MCP DevTools 快速开始指南

## 📋 一键测试流程

### 1. 启动MCP服务器
```powershell
# 方式1: 使用PowerShell脚本
.\start_mcp_server.ps1 start

# 方式2: 使用批处理文件
mcp start
```

### 2. 运行客户端测试
```powershell
# 基础功能测试
python simple_mcp_client.py test

# 交互式测试
python simple_mcp_client.py interactive
```

### 3. 停止服务器
```powershell
# 停止MCP服务器
.\start_mcp_server.ps1 stop
# 或
mcp stop
```

## 🧪 自动化测试

### 快速测试
```powershell
.\test_workflow.ps1 quick
```

### 完整测试
```powershell
.\test_workflow.ps1 full
```

### 交互式测试
```powershell
.\test_workflow.ps1 interactive
```

## 📚 服务器管理命令

| 命令 | 功能 |
|------|------|
| `mcp start` | 启动MCP服务器 |
| `mcp stop` | 停止MCP服务器 |
| `mcp status` | 检查服务器状态 |
| `mcp restart` | 重启服务器 |
| `mcp logs` | 查看服务器日志 |

## 🔧 客户端命令

| 命令 | 功能 |
|------|------|
| `python simple_mcp_client.py test` | 运行自动化测试 |
| `python simple_mcp_client.py interactive` | 交互式测试模式 |
| `python mcp_client.py chat` | AI对话模式 (需要API密钥) |

## 🎯 典型工作流程

1. **启动服务器**
   ```powershell
   mcp start
   ```

2. **验证服务器状态**
   ```powershell
   mcp status
   ```

3. **运行测试**
   ```powershell
   python simple_mcp_client.py test
   ```

4. **交互式测试** (可选)
   ```powershell
   python simple_mcp_client.py interactive
   ```

5. **停止服务器**
   ```powershell
   mcp stop
   ```

## 🔍 故障排除

### 服务器启动失败
```powershell
# 检查Rust环境
cargo --version

# 手动编译测试
cargo check --bin grape-mcp-devtools

# 查看错误日志
mcp logs
```

### 客户端连接失败
```powershell
# 确认服务器运行
mcp status

# 检查Python环境
python --version

# 安装依赖
pip install rich httpx python-dotenv
```

## 💡 提示

- 🔄 **后台运行**: MCP服务器在后台运行，不会阻塞控制台
- 📋 **日志记录**: 所有操作都有详细日志记录
- 🎮 **交互模式**: 支持实时测试各种工具功能
- 🤖 **AI功能**: 配置API密钥后可使用智能对话功能

---

**�� 现在你可以开始测试MCP功能了！** 