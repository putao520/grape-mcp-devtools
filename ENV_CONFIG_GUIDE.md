# 🔧 环境变量配置指南

## 📋 概述

本指南说明 `grape-mcp-devtools` 项目的环境变量配置结构。我们使用现代化的环境变量命名规范，明确区分了不同类型的API配置。

## 🏗️ 配置结构

### 🤖 LLM 大语言模型配置
用于通用对话、文本生成等功能：

```bash
LLM_API_BASE_URL=https://integrate.api.nvidia.com/v1
LLM_API_KEY=your-llm-api-key
LLM_MODEL_NAME=nvidia/llama-3.1-nemotron-70b-instruct
```

### 🔍 Embedding 向量化模型配置
专门用于文档向量化和搜索功能：

```bash
EMBEDDING_API_BASE_URL=https://integrate.api.nvidia.com/v1
EMBEDDING_API_KEY=your-embedding-api-key
EMBEDDING_MODEL_NAME=nvidia/nv-embedqa-mistral-7b-v2
```

### ⚙️ 向量化处理参数
控制向量化处理的技术参数：

```bash
VECTOR_DIMENSION=768
CHUNK_SIZE=8192
CHUNK_OVERLAP=512
MAX_FILE_SIZE=1048576
MAX_CONCURRENT_FILES=10
VECTORIZATION_TIMEOUT_SECS=30
EMBEDDING_TIMEOUT_SECS=30
```

## 🎯 关键特性

### ✅ 现代化设计原则
- **明确命名**: 环境变量名清晰表达其用途
- **功能分离**: LLM和Embedding配置完全独立
- **一致性**: 所有相关变量使用统一的命名前缀
- **可扩展性**: 易于添加新的API服务配置

### ✅ 配置清晰度
- **LLM_*** : 所有大语言模型相关配置
- **EMBEDDING_*** : 所有向量化相关配置
- **VECTOR_*** : 向量化处理参数
- **CHUNK_*** : 文档分块参数

## 📚 使用场景

### 🔧 开发场景

#### 1. 仅使用向量化功能
如果只需要文档向量化和搜索：
```bash
# 只配置Embedding相关
EMBEDDING_API_BASE_URL=https://integrate.api.nvidia.com/v1
EMBEDDING_API_KEY=your-embedding-key
EMBEDDING_MODEL_NAME=nvidia/nv-embedqa-mistral-7b-v2
```

#### 2. 仅使用LLM功能
如果只需要文本生成功能：
```bash
# 只配置LLM相关
LLM_API_BASE_URL=https://integrate.api.nvidia.com/v1
LLM_API_KEY=your-llm-key
LLM_MODEL_NAME=nvidia/llama-3.1-nemotron-70b-instruct
```

#### 3. 使用不同API提供商
如果LLM和Embedding使用不同的服务：
```bash
# LLM使用OpenAI
LLM_API_BASE_URL=https://api.openai.com/v1
LLM_API_KEY=sk-your-openai-key
LLM_MODEL_NAME=gpt-4

# Embedding使用NVIDIA
EMBEDDING_API_BASE_URL=https://integrate.api.nvidia.com/v1
EMBEDDING_API_KEY=nvapi-your-nvidia-key
EMBEDDING_MODEL_NAME=nvidia/nv-embedqa-mistral-7b-v2
```

## 🔍 环境变量完整列表

| 类别 | 变量名 | 用途 | 是否必需 |
|------|--------|------|----------|
| LLM | `LLM_API_BASE_URL` | LLM API端点 | 可选 |
| LLM | `LLM_API_KEY` | LLM API密钥 | 如需LLM功能 |
| LLM | `LLM_MODEL_NAME` | LLM模型名称 | 可选 |
| Embedding | `EMBEDDING_API_BASE_URL` | Embedding API端点 | 可选 |
| Embedding | `EMBEDDING_API_KEY` | Embedding API密钥 | 如需向量化 |
| Embedding | `EMBEDDING_MODEL_NAME` | Embedding模型名称 | 可选 |
| 向量化 | `VECTOR_DIMENSION` | 向量维度 | 可选 |
| 向量化 | `CHUNK_SIZE` | 文档分块大小 | 可选 |
| 向量化 | `CHUNK_OVERLAP` | 分块重叠大小 | 可选 |
| 向量化 | `MAX_FILE_SIZE` | 最大文件大小 | 可选 |
| 向量化 | `MAX_CONCURRENT_FILES` | 最大并发文件数 | 可选 |
| 向量化 | `VECTORIZATION_TIMEOUT_SECS` | 向量化超时秒数 | 可选 |
| 向量化 | `EMBEDDING_TIMEOUT_SECS` | Embedding超时秒数 | 可选 |

## 🚀 快速开始

### 1. 复制示例配置
```bash
cp .env .env.local  # 创建本地副本
```

### 2. 编辑配置
```bash
# 编辑 .env.local 文件
# 设置你的API密钥和端点
```

### 3. 验证配置
```bash
cargo run --bin test_cli          # 基础功能测试
cargo run --bin detailed_test_cli # 完整功能测试
```

## 🔒 安全注意事项

### ⚠️ API密钥安全
- **不要提交**: 确保 `.env` 文件不被提交到版本控制
- **使用 .env.local**: 本地开发使用 `.env.local` 文件
- **环境隔离**: 生产和开发使用不同的密钥

### 📝 .gitignore 配置
确保你的 `.gitignore` 包含：
```
.env
.env.local
.env.*.local
```

## 🧪 测试验证

### 基础测试
```bash
cargo run --bin test_cli
```

### 完整测试
```bash
cargo run --bin detailed_test_cli
```

### MCP通信测试
```bash
python test_mcp_communication.py
```

## 🎉 总结

现代化的配置结构提供了：
- ✅ **清晰的环境变量命名**
- ✅ **功能模块完全分离**
- ✅ **一致的命名规范**
- ✅ **易于理解和维护**
- ✅ **支持灵活的部署选项**

现在您可以使用清晰、现代的环境变量配置来管理项目的API服务！ 