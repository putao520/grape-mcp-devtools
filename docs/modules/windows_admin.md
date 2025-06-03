# Windows管理员权限检测功能

## 概述

Grape MCP DevTools 现在具备智能的Windows管理员权限检测功能。当程序在Windows系统上运行时，会自动检测当前是否以管理员身份运行，并根据权限状态调整工具安装行为。

## 核心功能

### 1. 自动权限检测

程序启动时会自动检测：
- ✅ **管理员模式**: 可以执行自动安装
- ⚠️ **普通用户模式**: 显示手动安装命令

### 2. 智能安装策略

#### 管理员模式下
- 自动执行工具安装命令
- 支持系统包管理器（choco, winget, scoop）
- 可以安装需要管理员权限的工具

#### 普通用户模式下
- 不执行任何安装命令
- 显示详细的手动安装指南
- 区分需要管理员权限的工具

### 3. 详细的安装指南

当检测到非管理员模式时，程序会显示：

```
📋 请手动执行以下命令安装缺失的工具:
═══════════════════════════════════════════════════
🗣️ cpp 语言工具:
   🔧 doxygen
      📝 描述: C++文档生成标准
      🛡️ 需要管理员权限:
      📋 命令: choco install doxygen.install
      💡 请以管理员身份运行 PowerShell 或 CMD
      🔄 升级命令: choco upgrade doxygen.install
      ✅ 验证命令: doxygen --version

🗣️ python 语言工具:
   🔧 mkdocs
      📝 描述: 文档生成工具
      📋 命令: pip install mkdocs
      🔄 升级命令: pip install --upgrade mkdocs
      ✅ 验证命令: mkdocs --version
```

## 技术实现

### Windows API集成

使用Windows API进行权限检测：

```rust
#[cfg(target_os = "windows")]
fn is_elevated() -> bool {
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    
    // 检查当前进程的提升令牌
    // 返回 true 表示管理员权限，false 表示普通用户权限
}
```

### 跨平台兼容

- **Windows**: 使用WinAPI检测管理员权限
- **Linux/macOS**: 检查是否为root用户（uid = 0）

### 安装策略分类

工具按权限需求分类：

1. **需要管理员权限**:
   - `choco install` 命令
   - `winget install` 命令
   - 系统级安装

2. **普通用户权限**:
   - `cargo install` 命令
   - `pip install --user` 命令
   - `npm install -g` 命令（如果npm配置允许）

## 使用示例

### 测试权限检测

运行测试程序：

```bash
cargo run --bin test_windows_admin
```

### 在主程序中使用

```rust
let config = ToolInstallConfig {
    strategy: InstallStrategy::Interactive,
    auto_upgrade: true,
    install_timeout_secs: 300,
    prefer_global: true,
    backup_existing: false,
};

let installer = ToolInstaller::new(config);
let missing_tools = installer.detect_missing_tools(&detected_languages).await?;
let install_report = installer.auto_install_tools(&missing_tools).await?;
```

## 安全考虑

### 权限最小化原则
- 只在必要时要求管理员权限
- 优先使用用户级安装方式
- 明确标识需要管理员权限的操作

### 透明度
- 清楚显示将要执行的命令
- 解释为什么需要管理员权限
- 提供手动安装的替代方案

## 配置选项

### InstallStrategy 枚举

```rust
pub enum InstallStrategy {
    /// 自动安装（静默）
    Auto,
    /// 询问用户确认
    Interactive,
    /// 只检测不安装
    DetectOnly,
    /// 强制重新安装
    Force,
}
```

### ToolInstallConfig 结构

```rust
pub struct ToolInstallConfig {
    pub strategy: InstallStrategy,
    pub auto_upgrade: bool,
    pub install_timeout_secs: u64,
    pub prefer_global: bool,
    pub backup_existing: bool,
}
```

## 错误处理

### 权限不足
- 检测到权限不足时，优雅降级到手动模式
- 提供清晰的错误信息和解决方案

### 安装失败
- 记录详细的失败原因
- 提供故障排除建议
- 支持重试机制

## 日志输出

程序会输出详细的日志信息：

```
⚠️ 检测到非管理员模式运行，部分工具安装需要管理员权限
💡 提示: 使用 '以管理员身份运行' 启动程序可启用自动安装功能
🔧 开始自动安装缺失的工具...
⚠️ 非管理员模式运行，将显示手动安装命令
```

## 未来改进

### 计划功能
1. **UAC提示集成**: 自动请求管理员权限提升
2. **包管理器检测**: 智能检测可用的包管理器
3. **安装进度显示**: 实时显示安装进度
4. **回滚功能**: 支持安装失败时的回滚

### 性能优化
1. **并行安装**: 支持多个工具并行安装
2. **缓存机制**: 缓存权限检测结果
3. **预检查**: 安装前验证所有依赖

## 总结

Windows管理员权限检测功能为Grape MCP DevTools提供了：

- ✅ **智能权限检测**: 自动识别运行权限
- ✅ **安全的安装策略**: 根据权限调整行为
- ✅ **用户友好的提示**: 清晰的手动安装指南
- ✅ **跨平台兼容**: 支持Windows/Linux/macOS
- ✅ **详细的日志记录**: 便于问题诊断

这确保了工具在各种权限环境下都能正常工作，同时保持了安全性和用户体验。 