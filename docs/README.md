# Grape MCP DevTools 文档中心

## 📚 文档导航

### 🏗️ 架构设计
- [**系统架构概览**](architecture/overview.md) - 基于MCP协议的工具服务器架构
- [**接口设计规范**](architecture/interfaces.md) - MCP工具接口标准

### 🔧 核心工具
- [**文档搜索**](modules/search_docs.md) - 多语言包文档搜索
- [**版本检查**](modules/version_check.md) - 包版本和依赖管理
- [**安全扫描**](modules/security_check.md) - 依赖安全漏洞检测
- [**环境检测**](modules/environment_detection.md) - 开发环境自动检测
- [**GitHub集成**](modules/github_integration.md) - 项目信息和任务上下文获取
- [**向量数据库升级**](modules/vector_database_upgrade.md) - 智能相似度检测和缓存优化

### 🔧 核心模块
- [**MCP协议模块**](modules/mcp_protocol.md) - MCP协议实现和通信机制
- [**MCP客户端模块**](modules/mcp_client.md) - 外部MCP工具集成
- [**语言特性模块**](modules/language_features.md) - 多语言支持和特性分析
- [**文档处理模块**](modules/document_processing.md) - 文档解析和处理引擎
- [**存储层模块**](modules/storage_layer.md) - 数据存储和缓存机制
- [**CLI集成模块**](modules/cli_integration.md) - 命令行工具集成
- [**动态注册模块**](modules/dynamic_registry.md) - 动态工具注册机制

### 🛠️ 系统工具
- [**Windows管理员检测**](modules/windows_admin.md) - Windows环境权限检测
- [**工具自动安装器**](modules/tool_installer.md) - 开发工具自动安装

### 👥 用户指南
- [**快速入门**](user-guide/quick-start.md) - 5分钟快速上手指南
- [**MCP协议说明**](user-guide/mcp-protocol.md) - MCP协议使用指南

### 🔨 开发指南
- [**工具描述指南**](development/tool-description-guide.md) - 工具描述标准化指南
- [**文档规范要求**](development/documentation-requirements.md) - 文档编写和维护标准

## 🎯 项目概述

**Grape MCP DevTools** 是一个专注于**开发支持**的MCP (Model Context Protocol) 工具集合，为AI编程助手提供高质量的开发环境信息和项目上下文。

### 📌 协议合规性声明

**重要声明**：我们严格遵循行业标准协议，不进行任何修改：
- **MCP协议**：完全按照 [Model Context Protocol](https://modelcontextprotocol.io) 官方规范实现
- **A2A协议**：严格遵循Agent-to-Agent通信标准
- **JSON-RPC 2.0**：标准的JSON-RPC 2.0消息格式
- **兼容性保证**：确保与所有符合标准的MCP客户端完全兼容

我们是协议的标准实现者，不是协议的修改者。

### 📌 核心定位
- **工具集合**：一组精心设计的MCP工具，专注于特定的开发支持功能
- **开发支持**：为开发团队提供上下文信息，而非直接参与编程工作
- **第三方优先**：充分利用成熟的第三方库和服务，避免重复造轮子
- **Windows友好**：专门针对Windows开发环境优化设计

### 🔧 核心功能
- 🔍 **智能文档搜索** - 跨语言包信息和API文档查找
- 📦 **版本管理工具** - 包版本检查和依赖分析
- 🛡️ **安全检查工具** - 依赖安全漏洞扫描
- 📋 **项目上下文获取** - GitHub任务信息和技术背景分析
- 🌐 **多语言支持** - Rust、Python、JavaScript、Java、Go、Dart
- 🔌 **外部工具集成** - 通过MCP客户端调用Playwright等第三方工具

### 🏗️ 技术特点
- **协议基础**: 标准MCP (Model Context Protocol) 实现
- **核心语言**: Rust，确保高性能和内存安全
- **通信模式**: stdio 模式，兼容所有MCP客户端
- **第三方集成**: 通过成熟的开源库和API服务
- **Windows优化**: 针对Windows PowerShell环境优化

### 🌍 支持的语言生态
- **Rust**: cargo工具链、crates.io集成
- **Python**: pip工具、PyPI包信息
- **JavaScript/Node.js**: npm包管理、文档搜索
- **Java**: Maven/Gradle依赖管理
- **Go**: go mod工具、官方文档
- **Dart/Flutter**: pub包管理、Flutter文档

## 📖 快速导航

### 新用户
1. 阅读 [快速入门](user-guide/quick-start.md)
2. 了解 [MCP协议说明](user-guide/mcp-protocol.md)
3. 查看 [系统架构概览](architecture/overview.md)

### 开发者
1. 查看 [系统架构概览](architecture/overview.md)
2. 了解 [工具描述指南](development/tool-description-guide.md)
3. 参考 [接口设计规范](architecture/interfaces.md)

### AI系统集成
1. 配置MCP客户端连接到本工具服务器
2. 使用标准MCP工具调用获取开发信息
3. 利用项目上下文信息增强编程建议

## 🚀 实际能力

### ✅ 已规划功能
- **MCP协议服务器** - 完整的MCP协议实现
- **多语言工具链** - 支持6种主流编程语言的基础工具
- **文档搜索引擎** - 基于第三方API的智能文档搜索
- **版本管理工具** - 自动版本检查和更新建议
- **安全扫描工具** - 基于已知漏洞数据库的安全检查
- **环境检测工具** - 自动检测开发环境配置
- **GitHub集成工具** - 获取项目任务和技术背景信息
- **外部工具集成** - 通过MCP客户端调用Playwright等工具

### 🎯 技术优势
- **专注定位**: 聚焦于开发支持，不做不擅长的事
- **成熟技术**: 基于Rust生态和成熟第三方服务
- **标准协议**: 完全兼容MCP协议标准
- **Windows优化**: 专门针对Windows开发环境设计
- **可测试性**: 所有功能都可以在真实环境下测试
- **可维护性**: 简洁的架构设计，易于理解和维护

## 🔄 文档维护

本文档集合反映项目的实际功能和设计目标。如有问题或需要改进，请：

1. 提交 Issue 描述问题
2. 提交 Pull Request 修复文档
3. 参与项目讨论和改进

---

*文档版本：v3.0*  
*最后更新：2025年1月*  
*维护团队：Grape MCP DevTools 开发团队* 