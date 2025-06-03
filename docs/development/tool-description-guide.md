# Grape MCP DevTools 工具开发指南

## 📋 概述

本指南提供了开发和集成MCP工具的标准化方法，确保所有工具都能在Grape MCP DevTools中正常工作并提供一致的用户体验。

## 🎯 工具开发原则

### 核心原则
1. **功能明确**：每个工具专注于特定的开发支持功能
2. **简洁实用**：避免过度复杂的设计，专注于核心价值
3. **Windows友好**：确保在Windows环境下正常工作
4. **第三方优先**：优先使用成熟的第三方库和服务
5. **可测试性**：所有功能都可以在真实环境下测试

### 设计标准
- 清晰的输入参数定义
- 结构化的输出格式
- 完善的错误处理
- 适当的缓存机制
- 详细的文档说明

## 🔧 工具实现标准

### 1. 基础接口实现

所有工具必须实现 `MCPTool` trait：

```rust
use serde_json::Value;
use anyhow::Result;

pub trait MCPTool: Send + Sync {
    /// 工具名称（用于MCP调用）
    fn name(&self) -> &str;
    
    /// 工具描述（简洁明了）
    fn description(&self) -> &str;
    
    /// JSON Schema定义（参数验证）
    fn schema(&self) -> Value;
    
    /// 工具执行逻辑
    async fn execute(&self, params: Value) -> Result<Value>;
}
```

### 2. 工具描述格式

**标准格式**：
```
"在需要[具体使用场景]时，[核心功能描述]，[为用户提供的价值]。"
```

**示例**：
```rust
fn description(&self) -> &str {
    "在需要查找特定功能的包或库时，搜索相关的包信息和文档，帮助找到合适的技术解决方案。"
}
```

### 3. 参数Schema设计

使用标准的JSON Schema格式：

```rust
fn schema(&self) -> Value {
    json!({
        "type": "object",
        "required": ["language", "query"],
        "properties": {
            "language": {
                "type": "string",
                "description": "编程语言（rust、python、javascript等）",
                "enum": ["rust", "python", "javascript", "java", "go", "dart"]
            },
            "query": {
                "type": "string",
                "description": "搜索关键词或功能描述",
                "minLength": 1,
                "maxLength": 200
            },
            "limit": {
                "type": "integer",
                "description": "返回结果数量限制（可选，默认10）",
                "minimum": 1,
                "maximum": 50,
                "default": 10
            }
        }
    })
}
```

### 4. 输出格式标准

统一使用MCP标准的输出格式：

```rust
async fn execute(&self, params: Value) -> Result<Value> {
    // 工具逻辑实现
    let result = self.perform_search(&params).await?;
    
    // 标准化输出
    Ok(json!({
        "content": [{
            "type": "text",
            "text": result
        }],
        "metadata": {
            "tool": self.name(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "third_party_api"
        }
    }))
}
```

## 🛠️ 具体工具实现示例

### 示例1: 文档搜索工具

```rust
pub struct SearchDocsTool {
    http_client: reqwest::Client,
    cache: Arc<SimpleCache>,
}

impl MCPTool for SearchDocsTool {
    fn name(&self) -> &str {
        "search_docs"
    }
    
    fn description(&self) -> &str {
        "在需要查找特定功能的包或库时，搜索相关的包信息和文档，帮助找到合适的技术解决方案。"
    }
    
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["language", "query"],
            "properties": {
                "language": {
                    "type": "string",
                    "description": "编程语言",
                    "enum": ["rust", "python", "javascript", "java", "go", "dart"]
                },
                "query": {
                    "type": "string",
                    "description": "搜索关键词",
                    "minLength": 1
                }
            }
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        let language = params["language"].as_str().unwrap_or("rust");
        let query = params["query"].as_str().unwrap_or("");
        
        // 检查缓存
        let cache_key = format!("docs_{}_{}", language, query);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(cached);
        }
        
        // 执行搜索
        let results = match language {
            "rust" => self.search_rust_docs(query).await?,
            "python" => self.search_python_docs(query).await?,
            _ => return Err(anyhow::anyhow!("不支持的语言: {}", language)),
        };
        
        // 格式化输出
        let output = json!({
            "content": [{
                "type": "text",
                "text": results
            }],
            "metadata": {
                "tool": "search_docs",
                "language": language,
                "query": query
            }
        });
        
        // 缓存结果
        self.cache.set(&cache_key, &output).await?;
        
        Ok(output)
    }
}
```

### 示例2: 版本检查工具

```rust
pub struct CheckVersionTool {
    http_client: reqwest::Client,
}

impl MCPTool for CheckVersionTool {
    fn name(&self) -> &str {
        "check_version"
    }
    
    fn description(&self) -> &str {
        "在需要了解包的版本信息时，获取指定包的最新版本、发布历史和兼容性信息。"
    }
    
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["package", "ecosystem"],
            "properties": {
                "package": {
                    "type": "string",
                    "description": "包名称"
                },
                "ecosystem": {
                    "type": "string",
                    "description": "包管理器生态系统",
                    "enum": ["rust", "npm", "pypi", "maven"]
                }
            }
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        let package = params["package"].as_str().unwrap_or("");
        let ecosystem = params["ecosystem"].as_str().unwrap_or("");
        
        let version_info = match ecosystem {
            "rust" => self.check_crates_io(package).await?,
            "npm" => self.check_npm(package).await?,
            "pypi" => self.check_pypi(package).await?,
            _ => return Err(anyhow::anyhow!("不支持的生态系统: {}", ecosystem)),
        };
        
        Ok(json!({
            "content": [{
                "type": "text",
                "text": version_info
            }],
            "metadata": {
                "tool": "check_version",
                "package": package,
                "ecosystem": ecosystem
            }
        }))
    }
}
```

## 📝 工具注册和配置

### 1. 工具注册

在 `src/tools/mod.rs` 中注册新工具：

```rust
use crate::tools::{SearchDocsTool, CheckVersionTool};

pub fn register_all_tools(registry: &mut ToolRegistry) {
    registry.register("search_docs", Arc::new(SearchDocsTool::new()));
    registry.register("check_version", Arc::new(CheckVersionTool::new()));
    // 添加其他工具...
}
```

### 2. 配置管理

在 `config.toml` 中添加工具配置：

```toml
[tools.search_docs]
enabled = true
timeout_seconds = 30
cache_ttl_hours = 24

[tools.check_version]
enabled = true
timeout_seconds = 15
cache_ttl_hours = 6
```

## 🧪 测试标准

### 1. 单元测试

每个工具都应该有完整的单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_search_docs_rust() {
        let tool = SearchDocsTool::new();
        let params = json!({
            "language": "rust",
            "query": "async"
        });
        
        let result = tool.execute(params).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output["content"].is_array());
        assert!(!output["content"][0]["text"].as_str().unwrap().is_empty());
    }
    
    #[tokio::test]
    async fn test_invalid_language() {
        let tool = SearchDocsTool::new();
        let params = json!({
            "language": "invalid",
            "query": "test"
        });
        
        let result = tool.execute(params).await;
        assert!(result.is_err());
    }
}
```

### 2. 集成测试

在 `tests/` 目录下创建集成测试：

```rust
// tests/tools_integration.rs
use grape_mcp_devtools::tools::*;

#[tokio::test]
async fn test_real_api_calls() {
    // 测试真实的API调用
    let tool = SearchDocsTool::new();
    let result = tool.execute(json!({
        "language": "rust",
        "query": "tokio"
    })).await;
    
    assert!(result.is_ok());
}
```

## 🔧 错误处理标准

### 1. 错误类型定义

```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("网络请求失败: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("参数验证失败: {0}")]
    ValidationError(String),
    
    #[error("第三方API错误: {0}")]
    ApiError(String),
    
    #[error("缓存操作失败: {0}")]
    CacheError(String),
}
```

### 2. 错误处理实践

```rust
async fn execute(&self, params: Value) -> Result<Value> {
    // 参数验证
    let language = params["language"].as_str()
        .ok_or_else(|| ToolError::ValidationError("缺少language参数".to_string()))?;
    
    // API调用
    let response = self.http_client
        .get(&format!("https://api.example.com/{}", language))
        .send()
        .await
        .map_err(ToolError::NetworkError)?;
    
    if !response.status().is_success() {
        return Err(ToolError::ApiError(
            format!("API返回错误状态: {}", response.status())
        ).into());
    }
    
    // 处理响应...
    Ok(result)
}
```

## 📚 文档要求

### 1. 工具文档

每个工具都应该有对应的文档文件：

```markdown
# search_docs 工具

## 功能描述
搜索指定编程语言的包和库信息。

## 参数说明
- `language`: 编程语言（必需）
- `query`: 搜索关键词（必需）
- `limit`: 结果数量限制（可选，默认10）

## 使用示例
```json
{
  "name": "search_docs",
  "arguments": {
    "language": "rust",
    "query": "async programming"
  }
}
```

## 返回格式
返回包含搜索结果的文本内容。
```

### 2. API文档

使用标准的OpenAPI格式描述工具接口。

## 🚀 部署和发布

### 1. 版本管理

使用语义化版本控制：
- 主版本号：不兼容的API修改
- 次版本号：向下兼容的功能性新增
- 修订号：向下兼容的问题修正

### 2. 发布检查清单

- [ ] 所有测试通过
- [ ] 文档更新完整
- [ ] 性能测试通过
- [ ] Windows兼容性验证
- [ ] 安全扫描通过

---

*工具开发指南版本：v3.0*  
*最后更新：2025年1月*  
*适用于简化架构设计* 