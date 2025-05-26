# 动态MCP工具注册系统

## 概述

动态MCP工具注册系统能够根据当前环境中安装的CLI工具情况，智能地决定向LLM暴露哪些MCP工具。这提供了更好的用户体验，避免注册无法使用的工具。

## 🚀 主要特性

### 1. 智能CLI检测
- **自动检测**: 扫描系统中安装的CLI工具
- **版本识别**: 获取工具版本信息
- **特性分析**: 识别工具的功能特性
- **Windows兼容**: 支持Windows和Unix系统

### 2. 灵活的注册策略
- **OnlyAvailable**: 仅注册检测到的可用工具（默认）
- **ForceAll**: 强制注册所有工具（使用 `-all` 参数）
- **FeatureBased**: 基于特性的选择性注册

### 3. 实时报告
- **检测报告**: 显示环境中可用的CLI工具
- **注册报告**: 展示工具注册结果和统计

## 📋 使用方式

### 基本使用

```bash
# 默认模式：检测环境并仅注册可用工具
cargo run --bin dynamic-mcp-server

# 强制注册所有工具
cargo run --bin dynamic-mcp-server -- --all

# 仅查看检测报告
cargo run --bin dynamic-mcp-server -- --report-only
```

### 高级选项

```bash
# 基于特性过滤工具
cargo run --bin dynamic-mcp-server -- --feature build-tool --feature package-manager

# 启用详细日志
cargo run --bin dynamic-mcp-server -- --verbose

# 指定服务器配置
cargo run --bin dynamic-mcp-server serve --host 0.0.0.0 --port 9000
```

### 子命令

```bash
# 仅执行CLI检测
cargo run --bin dynamic-mcp-server detect --verbose

# 显示策略信息
cargo run --bin dynamic-mcp-server strategies

# 启动服务器
cargo run --bin dynamic-mcp-server serve --port 8080
```

## 🔧 支持的CLI工具

### 构建工具
- **Rust**: `cargo`, `rustdoc`, `clippy`
- **JavaScript**: `npm`, `yarn`, `pnpm`, `webpack`
- **Python**: `pip`, `pipenv`, `poetry`
- **Java**: `mvn`, `gradle`
- **Go**: `go`
- **其他**: `make`, `cmake`

### 版本控制
- **Git**: `git`, `git-lfs`
- **其他**: `svn`, `hg`

### 容器化
- **Docker**: `docker`, `docker-compose`
- **其他**: `podman`, `kubectl`

### 文档工具
- **Rust**: `rustdoc`, `cargo-doc`
- **JavaScript**: `jsdoc`
- **Python**: `sphinx-build`, `mkdocs`
- **其他**: `doxygen`

### 代码分析
- **Rust**: `clippy`, `cargo-audit`
- **JavaScript**: `eslint`, `prettier`
- **Python**: `pylint`, `flake8`, `black`
- **Go**: `gofmt`, `golint`

## 📊 工具映射

### CLI工具 → MCP工具映射

| CLI工具 | MCP工具 | 功能描述 |
|---------|---------|----------|
| `cargo` | `CheckVersionTool` | Rust包版本检查 |
| `npm` | `CheckVersionTool` | Node.js包版本检查 |
| `pip` | `CheckVersionTool` | Python包版本检查 |
| `rustdoc` | `SearchDocsTools` | Rust文档搜索 |
| `jsdoc` | `GetApiDocsTool` | JavaScript API文档 |
| `clippy` | `AnalyzeCodeTool` | Rust代码分析 |
| `eslint` | `AnalyzeCodeTool` | JavaScript代码分析 |
| `cargo-audit` | `AnalyzeDependenciesTool` | Rust依赖安全检查 |

### 通用工具（始终注册）

- `SearchDocsTools` - 文档搜索
- `CheckVersionTool` - 版本检查
- `AnalyzeDependenciesTool` - 依赖分析
- `AnalyzeCodeTool` - 代码分析
- `GetChangelogTool` - 变更日志
- `CompareVersionsTool` - 版本比较
- `GetApiDocsTool` - API文档

## 🎯 注册策略详解

### 1. OnlyAvailable（推荐）

```bash
# 自动检测模式
cargo run --bin dynamic-mcp-server
```

**行为**:
- 检测系统中安装的CLI工具
- 仅注册检测到的可用工具对应的MCP工具
- 始终注册通用工具

**优势**:
- 安全可靠，不会注册无法使用的工具
- 性能优化，减少无效工具的开销
- 用户体验好，LLM不会尝试使用不存在的工具

### 2. ForceAll（测试用）

```bash
# 强制注册所有工具
cargo run --bin dynamic-mcp-server -- --all
```

**行为**:
- 忽略CLI检测结果
- 注册所有已定义的MCP工具
- 适用于测试和开发环境

**使用场景**:
- 测试完整的工具集
- 开发和调试MCP工具
- 演示所有可用功能

### 3. FeatureBased（定制化）

```bash
# 基于特性注册
cargo run --bin dynamic-mcp-server -- --feature build-tool --feature version-control
```

**行为**:
- 检测CLI工具
- 根据工具特性过滤
- 仅注册具有指定特性的工具

**特性类别**:
- `build-tool` - 构建工具
- `package-manager` - 包管理器
- `version-control` - 版本控制
- `containerization` - 容器化
- `rust` - Rust生态
- `javascript` - JavaScript生态
- `python` - Python生态
- `java` - Java生态

## 📈 示例输出

### CLI检测报告

```
🔧 CLI工具检测报告
==================================================
📊 总结: 12/25 工具可用

📁 构建工具
  ✅ cargo (1.75.0)
  ✅ npm (10.2.4)
  ✅ go (1.21.5)

📁 包管理器
  ✅ pip (23.3.1)
  ✅ npm (10.2.4)

📁 版本控制
  ✅ git (2.42.0)

📁 其他工具
  ✅ docker (24.0.7)
  ✅ jq (1.6)
  ✅ curl (8.4.0)
```

### MCP工具注册报告

```
🎯 MCP 工具注册报告
==================================================
📊 总结: 15 成功, 0 失败, 8 跳过

✅ 成功注册的工具:
  • cargo
  • npm
  • pip
  • git
  • docker
  • _universal_search
  • _universal_version_check
  • _universal_deps_analysis
  • _universal_code_analysis
  • _universal_changelog
  • _universal_compare_versions
  • _universal_api_docs

⏭️ 跳过的工具:
  • mvn: CLI工具不可用
  • gradle: CLI工具不可用
  • poetry: CLI工具不可用
```

## 🛠️ 开发指南

### 添加新工具支持

1. **更新CLI检测器**:
```rust
// 在 detector.rs 中添加新工具
("new-tool", vec!["--version"]),
```

2. **注册工具工厂**:
```rust
// 在 registry.rs 中添加映射
self.register_factory("new-tool", || {
    Box::new(NewMCPTool::new())
});
```

3. **实现MCP工具**:
```rust
// 创建新的MCP工具实现
pub struct NewMCPTool;

impl MCPTool for NewMCPTool {
    // 实现必要的方法
}
```

### 自定义检测逻辑

```rust
// 为特殊工具添加自定义检测
async fn detect_special_tools(&mut self) -> Result<()> {
    // 检测特殊情况
    if some_condition {
        self.cache.insert("special-tool".to_string(), CliToolInfo {
            name: "special-tool".to_string(),
            available: true,
            // ...
        });
    }
    Ok(())
}
```

## 💡 最佳实践

### 生产环境
```bash
# 推荐配置
cargo run --bin dynamic-mcp-server serve --host 127.0.0.1 --port 8080
```

### 开发环境
```bash
# 详细日志模式
cargo run --bin dynamic-mcp-server -- --verbose --report-only
```

### CI/CD环境
```bash
# 强制注册模式（确保一致性）
cargo run --bin dynamic-mcp-server -- --all
```

### 特定场景
```bash
# 仅容器相关工具
cargo run --bin dynamic-mcp-server -- --feature containerization

# 仅构建工具
cargo run --bin dynamic-mcp-server -- --feature build-tool
```

## 🔍 故障排除

### 常见问题

1. **工具检测失败**
   - 检查PATH环境变量
   - 确认工具已正确安装
   - 使用 `--verbose` 查看详细日志

2. **注册失败**
   - 检查工具依赖是否满足
   - 查看错误日志确定原因
   - 尝试使用 `--all` 强制注册

3. **性能问题**
   - 减少检测的工具数量
   - 使用特性过滤
   - 禁用不必要的检测

### 调试命令

```bash
# 详细检测信息
cargo run --bin dynamic-mcp-server detect --verbose

# 查看所有策略
cargo run --bin dynamic-mcp-server strategies

# 测试特定特性
cargo run --bin dynamic-mcp-server -- --feature rust --verbose
```

## 🎉 总结

动态MCP工具注册系统提供了智能、灵活的工具管理方案：

- **智能检测**: 自动发现环境中的CLI工具
- **灵活策略**: 支持多种注册策略满足不同需求
- **用户友好**: 只暴露可用的工具，提升LLM交互体验
- **易于扩展**: 简单的工厂模式支持新工具的快速集成

这种设计使MCP服务器能够根据实际环境自适应，为不同的开发环境提供最优的工具集合。 