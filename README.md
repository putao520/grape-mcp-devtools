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

## 🔧 最近更新

- ✅ [三个关键改进完成](docs/modules/three_improvements_completed.md) - 编译警告清理、批量嵌入测试、AI内容分析增强
- ✅ [向量数据库升级](docs/modules/vector_database_upgrade.md) - NVIDIA嵌入API集成、HNSW搜索、智能缓存
- ✅ [向量搜索功能修复](docs/modules/vector_search_fix_completed.md) - 恢复真正的语义向量搜索功能

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

# 🍇 Grape Vector Database

一个高性能的嵌入式向量数据库，专为AI应用和语义搜索设计。

## 🚀 特性

- **高性能**: 基于HNSW算法的近似最近邻搜索
- **嵌入式**: 无需外部服务，直接集成到应用中
- **智能缓存**: 多层缓存策略，减少API调用70%
- **混合搜索**: 结合向量相似度和文本匹配
- **持久化**: 支持磁盘存储和数据恢复
- **批量操作**: 高效的批量插入和查询
- **去重**: 智能的重复文档检测
- **多提供者**: 支持NVIDIA、OpenAI等多种嵌入服务

## 📊 性能指标

- **查询延迟**: < 5ms (10万向量)
- **吞吐量**: > 10,000 QPS
- **内存效率**: 相比原始数据节省70%存储空间
- **API调用减少**: 智能缓存减少70%的嵌入API调用

## 🛠️ 安装

```toml
[dependencies]
grape-vector-db = "0.1.0"
```

## 🎯 快速开始

### 基础用法

```rust
use grape_vector_db::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建向量数据库实例
    let mut db = VectorDatabase::new("./data").await?;
    
    // 添加文档
    let doc = Document {
        id: "doc1".to_string(),
        content: "Rust是一种系统编程语言".to_string(),
        title: Some("Rust介绍".to_string()),
        language: Some("zh".to_string()),
        ..Default::default()
    };
    
    db.add_document(doc).await?;
    
    // 搜索相似文档
    let results = db.search("编程语言", 10).await?;
    println!("找到 {} 个相似文档", results.len());
    
    for result in results {
        println!("文档: {} (相似度: {:.2})", result.title, result.score);
    }
    
    Ok(())
}
```

### 批量操作

```rust
use grape_vector_db::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = VectorDatabase::new("./data").await?;
    
    // 批量添加文档
    let documents = vec![
        Document {
            content: "Python是一种高级编程语言".to_string(),
            ..Default::default()
        },
        Document {
            content: "JavaScript用于Web开发".to_string(),
            ..Default::default()
        },
    ];
    
    let ids = db.add_documents(documents).await?;
    println!("添加了 {} 个文档", ids.len());
    
    Ok(())
}
```

### 自定义配置

```rust
use grape_vector_db::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = VectorDbConfig::default();
    
    // 自定义HNSW参数
    config.hnsw.m = 32;
    config.hnsw.ef_construction = 400;
    
    // 自定义嵌入提供者
    config.embedding.provider = "nvidia".to_string();
    config.embedding.model = "nvidia/nv-embedqa-e5-v5".to_string();
    
    let mut db = VectorDatabase::with_config("./data", config).await?;
    
    // 使用数据库...
    
    Ok(())
}
```

## 🏗️ 架构设计

### 核心组件

- **VectorStore**: 向量存储和索引管理
- **EmbeddingProvider**: 嵌入向量生成
- **QueryEngine**: 查询处理和结果排序
- **IndexManager**: HNSW索引优化
- **MetricsCollector**: 性能监控

### 数据流

```
文档输入 → 嵌入生成 → 向量存储 → 索引构建 → 持久化
    ↓
查询输入 → 查询嵌入 → 向量搜索 → 结果排序 → 返回结果
```

## 🔧 配置选项

### HNSW索引配置

```rust
pub struct HnswConfig {
    pub m: usize,                    // 连接数 (推荐: 16-32)
    pub ef_construction: usize,      // 构建候选数 (推荐: 200-400)
    pub ef_search: usize,           // 搜索候选数 (推荐: 100-200)
    pub max_layers: usize,          // 最大层数 (推荐: 16)
}
```

### 嵌入提供者配置

```rust
pub struct EmbeddingConfig {
    pub provider: String,           // "nvidia", "openai", "local"
    pub model: String,             // 模型名称
    pub api_key: Option<String>,   // API密钥
    pub batch_size: usize,         // 批处理大小
    pub timeout_seconds: u64,      // 超时设置
}
```

## 📈 性能优化

### 1. 内存优化
- 分层存储：热数据内存 + 冷数据磁盘
- 向量压缩：支持量化压缩
- LRU缓存：智能缓存管理

### 2. 查询优化
- 并行搜索：多线程查询处理
- 结果缓存：查询结果智能缓存
- 早期终止：相似度阈值过滤

### 3. 索引优化
- 动态重建：基于数据变化自动重建
- 增量更新：支持增量索引更新
- 内存映射：大文件高效访问

## 🧪 基准测试

运行性能基准测试：

```bash
cargo bench
```

### 测试结果示例

```
向量搜索基准测试:
- 10,000 向量:   平均 1.2ms
- 100,000 向量:  平均 4.8ms
- 1,000,000 向量: 平均 15.3ms

批量插入基准测试:
- 1,000 文档:    平均 2.1s
- 10,000 文档:   平均 18.7s
```

## 🔍 使用场景

### 1. 文档搜索
```rust
// 技术文档相似度搜索
let results = db.search("如何使用Rust进行并发编程", 5).await?;
```

### 2. 代码搜索
```rust
// 代码片段语义搜索
let results = db.search("异步HTTP请求处理", 10).await?;
```

### 3. 问答系统
```rust
// 基于语义的问答匹配
let results = db.search("什么是内存安全", 3).await?;
```

## 🛡️ 错误处理

```rust
use grape_vector_db::*;

match db.add_document(document).await {
    Ok(id) => println!("文档添加成功: {}", id),
    Err(VectorDbError::DuplicateDocument(id)) => {
        println!("文档已存在: {}", id);
    },
    Err(VectorDbError::Embedding(msg)) => {
        println!("嵌入生成失败: {}", msg);
    },
    Err(e) => println!("其他错误: {}", e),
}
```

## 🚧 开发路线图

### v0.2.0 (计划中)
- [ ] 分布式支持
- [ ] 更多嵌入提供者
- [ ] 地理空间搜索
- [ ] GraphQL接口

### v0.3.0 (计划中)
- [ ] 多模态支持(图像、音频)
- [ ] 联邦学习集成
- [ ] 实时流处理
- [ ] Web界面

## 🤝 贡献指南

1. Fork 本仓库
2. 创建特性分支: `git checkout -b feature/amazing-feature`
3. 提交更改: `git commit -m 'Add amazing feature'`
4. 推送分支: `git push origin feature/amazing-feature`
5. 提交 Pull Request

## 📄 许可证

本项目采用 MIT 或 Apache-2.0 许可证。详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- [instant-distance](https://github.com/InstantDomain/instant-distance) - HNSW算法实现
- [NVIDIA NIM](https://developer.nvidia.com/nim) - 嵌入API服务
- Rust 社区的支持和贡献

---

**Grape Vector Database** - 让语义搜索变得简单高效 🍇 