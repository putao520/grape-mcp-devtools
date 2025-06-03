# 工具自动安装器设计文档

## 概述

Grape MCP DevTools 具备智能检测缺失的文档生成工具并提供自动安装和升级功能。当系统有某种编程语言环境但缺少对应的文档生成CLI工具时，系统能够自动检测并询问用户是否需要安装。

## 核心功能

### 1. 自动检测缺失工具

支持检测以下编程语言的文档生成工具：

#### Rust
- **rustdoc** - Rust官方文档生成工具（通常随Cargo安装）
- **mdbook** - Rust社区的书籍文档生成器
- **cargo-doc** - Cargo子命令

#### Python  
- **pydoc** - Python内置文档工具
- **sphinx** - Python文档生成器标准
- **mkdocs** - Markdown文档站点生成器
- **pdoc** - 自动API文档生成器

#### JavaScript/Node.js
- **jsdoc** - JavaScript文档生成器
- **typedoc** - TypeScript API文档生成器  
- **docsify** - 动态文档网站生成器

#### Java
- **javadoc** - Java官方文档工具
- **maven** - 构建工具（包含文档插件）
- **gradle** - 现代构建工具

#### Go
- **godoc** - Go官方文档工具
- **pkgsite** - Go包文档服务器

#### C#/.NET
- **docfx** - 微软文档生成工具
- **xmldoc2md** - XML文档转Markdown工具

#### C++
- **doxygen** - C++文档生成标准
- **breathe** - Doxygen与Sphinx集成

#### 其他语言
- **PHP**: phpDocumentor, Sami
- **Ruby**: YARD, RDoc  
- **Swift**: swift-doc, Jazzy
- **Dart**: dartdoc

### 2. 安装策略

支持多种安装策略：

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

### 3. 安装方法

根据不同工具和系统环境，支持多种安装方法：

#### 包管理器安装
- **Windows**: Chocolatey, Scoop, winget
- **macOS**: Homebrew, MacPorts
- **Linux**: apt, yum, dnf, pacman, zypper

#### 语言包管理器
- **Node.js**: npm, yarn, pnpm
- **Python**: pip, pipx, conda
- **Rust**: cargo install
- **Ruby**: gem
- **Go**: go install

#### 下载安装
- 从官方GitHub释放页面下载
- 从官方网站下载安装包
- 使用curl/wget下载二进制文件

### 4. 升级检测

定期检查已安装工具的版本，并提供升级建议：

- 检查最新版本
- 比较当前版本
- 提供升级路径
- 支持自动升级

## 使用示例

### 基本配置

```rust
use grape_mcp_devtools::cli::{ToolInstaller, ToolInstallConfig, InstallStrategy};

// 创建安装配置
let config = ToolInstallConfig {
    strategy: InstallStrategy::Interactive,  // 询问用户确认
    auto_upgrade: true,                      // 自动升级
    install_timeout_secs: 300,               // 安装超时
    prefer_global: true,                     // 优先全局安装
    backup_existing: false,                  // 不备份现有版本
};

// 创建工具安装器
let installer = ToolInstaller::new(config);
```

### 检测和安装工具

```rust
// 检测缺失的工具
let missing_tools = installer.detect_missing_tools(&["rust", "python", "javascript"]).await?;

for (language, tools) in missing_tools {
    println!("语言 {} 缺少工具: {:?}", language, tools);
}

// 安装特定工具
let install_result = installer.install_tool("sphinx", "python").await?;
println!("安装结果: {:?}", install_result);
```

### 升级工具

```rust
// 检查升级
let upgrade_report = installer.check_and_upgrade_tool("rustdoc").await?;
println!("升级报告: {}", upgrade_report.generate_summary());
```

### 批量处理

```rust
// 批量检测和安装
let install_report = installer.auto_install_missing_tools(&detected_languages).await?;
println!("批量安装完成: {} 成功, {} 失败", 
    install_report.successful_installs.len(),
    install_report.failed_installs.len()
);
```

## 集成到动态注册

工具安装器已集成到动态工具注册流程中：

```rust
// 在主程序中启用自动安装
let install_config = cli::ToolInstallConfig {
    strategy: cli::InstallStrategy::Interactive,
    auto_upgrade: true,
    install_timeout_secs: 300,
    prefer_global: true,
    backup_existing: false,
};

let mut registry = tools::DynamicRegistryBuilder::new()
    .with_policy(tools::RegistrationPolicy::Adaptive { score_threshold: 0.7 })
    .with_auto_install(install_config)  // 启用自动安装
    .build();

// 执行注册（会自动检测并询问安装缺失工具）
let report = registry.auto_register_tools().await?;
```

## 安全考虑

1. **用户确认**: 默认策略需要用户确认才安装
2. **权限检查**: 检查是否有足够权限进行安装
3. **验证下载**: 验证下载文件的完整性
4. **日志记录**: 记录所有安装活动
5. **回滚机制**: 安装失败时的清理和回滚

## 配置选项

可通过环境变量或配置文件自定义行为：

```bash
# 设置默认安装策略
export GRAPE_INSTALL_STRATEGY=interactive

# 启用自动升级
export GRAPE_AUTO_UPGRADE=true

# 设置安装超时
export GRAPE_INSTALL_TIMEOUT=300

# 设置包管理器偏好
export GRAPE_PREFERRED_PACKAGE_MANAGER=homebrew
```

## 错误处理

工具安装器提供详细的错误信息和建议：

```rust
match installer.install_tool("sphinx", "python").await {
    Ok(result) => println!("安装成功: {:?}", result),
    Err(e) => {
        eprintln!("安装失败: {}", e);
        
        // 获取建议的解决方案
        let suggestions = installer.get_install_suggestions("sphinx", "python");
        for suggestion in suggestions {
            println!("建议: {}", suggestion);
        }
    }
}
```

## 监控和报告

系统提供详细的安装监控和报告：

- 安装进度跟踪
- 性能指标收集
- 错误统计和分析
- 用户行为分析

这使得Grape MCP DevTools能够在运行时动态适应开发环境，确保所有必要的文档生成工具都可用，从而提供最佳的开发体验。 