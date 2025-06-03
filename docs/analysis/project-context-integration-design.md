# Grape MCP DevTools 项目上下文集成设计

## 📋 概述

本文档描述了 **Grape MCP DevTools** 如何通过 GitHub API 集成获取项目上下文信息，为 AI 编程助手提供准确的项目背景和开发环境信息。我们专注于简洁实用的设计，避免过度复杂的架构。

## 🎯 设计目标

### 核心价值
1. **项目信息获取**：从 GitHub 获取项目基本信息、技术栈和开发状态
2. **任务上下文提供**：获取当前的 Issues、Pull Requests 和 Milestones
3. **技术背景分析**：分析项目的主要技术栈和开发工具
4. **简洁实用原则**：避免复杂的 AI 分析，专注于数据获取和格式化

### 非目标
- 不进行复杂的 AI 分析和推理
- 不构建智能代理系统
- 不提供直接的编程建议
- 不进行深度的代码分析

## 🏗️ 技术架构

### 整体设计原则
- **第三方优先**：使用 GitHub API 和成熟的 HTTP 库
- **简洁架构**：避免复杂的多层抽象
- **Windows 友好**：确保在 Windows 环境下正常工作
- **可测试性**：所有功能都可以在真实环境下测试

### 架构组件

```
┌─────────────────────────────────────────┐
│             GitHub API 层               │
│  ┌─────────────┬─────────────────────┐   │
│  │ REST API    │ GraphQL API         │   │
│  │ 调用器      │ 调用器              │   │
│  └─────────────┴─────────────────────┘   │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│          GitHub 集成工具层               │
│  ┌─────────────┬─────────────────────┐   │
│  │ 项目信息    │ 任务管理            │   │
│  │ 获取器      │ 数据收集器           │   │
│  └─────────────┴─────────────────────┘   │
│  ┌─────────────┬─────────────────────┐   │
│  │ 技术栈      │ 数据格式化器         │   │
│  │ 分析器      │                     │   │
│  └─────────────┴─────────────────────┘   │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│             MCP 工具接口                │
│         github_info 工具                │
└─────────────────────────────────────────┘
```

## 🔧 核心功能实现

### 1. GitHub 项目信息工具

#### 基础实现
```rust
pub struct GitHubInfoTool {
    github_client: GitHubClient,
    cache: Arc<SimpleCache>,
    config: GitHubConfig,
}

impl MCPTool for GitHubInfoTool {
    fn name(&self) -> &str {
        "github_info"
    }
    
    fn description(&self) -> &str {
        "在需要了解GitHub项目背景时，获取项目基本信息、当前任务状态和技术栈信息。"
    }
    
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["repo"],
            "properties": {
                "repo": {
                    "type": "string",
                    "description": "GitHub仓库路径（如：microsoft/vscode）"
                },
                "type": {
                    "type": "string",
                    "description": "信息类型",
                    "enum": ["basic", "tasks", "tech_stack", "recent_activity"],
                    "default": "basic"
                },
                "include_details": {
                    "type": "boolean",
                    "description": "是否包含详细信息",
                    "default": false
                }
            }
        })
    }
    
    async fn execute(&self, params: Value) -> Result<Value> {
        let repo = params["repo"].as_str().unwrap_or("");
        let info_type = params["type"].as_str().unwrap_or("basic");
        let include_details = params["include_details"].as_bool().unwrap_or(false);
        
        // 检查缓存
        let cache_key = format!("github_{}_{}", repo, info_type);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(cached);
        }
        
        // 获取信息
        let result = match info_type {
            "basic" => self.get_basic_info(repo, include_details).await?,
            "tasks" => self.get_task_info(repo, include_details).await?,
            "tech_stack" => self.get_tech_stack_info(repo, include_details).await?,
            "recent_activity" => self.get_recent_activity(repo, include_details).await?,
            _ => return Err(anyhow::anyhow!("不支持的信息类型: {}", info_type)),
        };
        
        // 格式化输出
        let output = json!({
            "content": [{
                "type": "text",
                "text": result
            }],
            "metadata": {
                "tool": "github_info",
                "repo": repo,
                "type": info_type,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });
        
        // 缓存结果
        self.cache.set(&cache_key, &output).await?;
        
        Ok(output)
    }
}
```

### 2. 具体功能实现

#### 2.1 基本项目信息获取
```rust
impl GitHubInfoTool {
    async fn get_basic_info(&self, repo: &str, include_details: bool) -> Result<String> {
        let repo_info = self.github_client.get_repository(repo).await?;
        
        let mut info = format!(
            "## {} 项目信息\n\n",
            repo_info.name
        );
        
        info.push_str(&format!("**描述**: {}\n", repo_info.description.unwrap_or("无描述".to_string())));
        info.push_str(&format!("**主要语言**: {}\n", repo_info.language.unwrap_or("未知".to_string())));
        info.push_str(&format!("**Stars**: {} | **Forks**: {}\n", repo_info.stargazers_count, repo_info.forks_count));
        info.push_str(&format!("**最后更新**: {}\n", repo_info.updated_at));
        
        if include_details {
            // 获取额外的详细信息
            let languages = self.github_client.get_languages(repo).await?;
            info.push_str("\n**技术栈分布**:\n");
            for (lang, bytes) in languages {
                let percentage = (bytes as f64 / languages.values().sum::<u64>() as f64) * 100.0;
                info.push_str(&format!("- {}: {:.1}%\n", lang, percentage));
            }
            
            let topics = repo_info.topics.unwrap_or_default();
            if !topics.is_empty() {
                info.push_str(&format!("\n**标签**: {}\n", topics.join(", ")));
            }
        }
        
        Ok(info)
    }
    
    async fn get_task_info(&self, repo: &str, include_details: bool) -> Result<String> {
        let issues = self.github_client.get_open_issues(repo, 10).await?;
        let prs = self.github_client.get_open_pull_requests(repo, 10).await?;
        
        let mut info = format!("## {} 当前任务状态\n\n", repo);
        
        info.push_str(&format!("**开放Issues**: {}\n", issues.len()));
        info.push_str(&format!("**开放Pull Requests**: {}\n\n", prs.len()));
        
        if include_details && !issues.is_empty() {
            info.push_str("**最近的Issues**:\n");
            for issue in issues.iter().take(5) {
                info.push_str(&format!(
                    "- [#{}] {} ({})\n",
                    issue.number,
                    issue.title,
                    issue.labels.iter().map(|l| &l.name).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        
        if include_details && !prs.is_empty() {
            info.push_str("\n**最近的Pull Requests**:\n");
            for pr in prs.iter().take(5) {
                info.push_str(&format!(
                    "- [#{}] {} (by {})\n",
                    pr.number,
                    pr.title,
                    pr.user.login
                ));
            }
        }
        
        Ok(info)
    }
    
    async fn get_tech_stack_info(&self, repo: &str, include_details: bool) -> Result<String> {
        // 获取语言统计
        let languages = self.github_client.get_languages(repo).await?;
        
        // 分析配置文件
        let config_files = self.analyze_config_files(repo).await?;
        
        let mut info = format!("## {} 技术栈分析\n\n", repo);
        
        info.push_str("**主要编程语言**:\n");
        let total_bytes: u64 = languages.values().sum();
        for (lang, bytes) in languages.iter().take(5) {
            let percentage = (*bytes as f64 / total_bytes as f64) * 100.0;
            info.push_str(&format!("- {}: {:.1}%\n", lang, percentage));
        }
        
        if include_details {
            info.push_str("\n**开发工具和框架**:\n");
            for config in config_files {
                info.push_str(&format!("- {}: {}\n", config.file_type, config.detected_tools.join(", ")));
            }
        }
        
        Ok(info)
    }
    
    async fn get_recent_activity(&self, repo: &str, include_details: bool) -> Result<String> {
        let commits = self.github_client.get_recent_commits(repo, 10).await?;
        let releases = self.github_client.get_recent_releases(repo, 5).await?;
        
        let mut info = format!("## {} 最近活动\n\n", repo);
        
        if !commits.is_empty() {
            info.push_str("**最近提交**:\n");
            for commit in commits.iter().take(if include_details { 10 } else { 5 }) {
                info.push_str(&format!(
                    "- {} by {} ({})\n",
                    commit.commit.message.lines().next().unwrap_or(""),
                    commit.commit.author.name,
                    commit.commit.author.date
                ));
            }
        }
        
        if include_details && !releases.is_empty() {
            info.push_str("\n**最近发布**:\n");
            for release in releases {
                info.push_str(&format!(
                    "- {} ({}) - {}\n",
                    release.tag_name,
                    release.published_at.unwrap_or("未知日期".to_string()),
                    release.name.unwrap_or("无标题".to_string())
                ));
            }
        }
        
        Ok(info)
    }
}
```

### 3. GitHub API 客户端

#### 简化的 HTTP 客户端
```rust
pub struct GitHubClient {
    http_client: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "grape-mcp-devtools/1.0.0".parse().unwrap()
        );
        
        if let Some(ref token) = token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap()
            );
        }
        
        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            http_client,
            base_url: "https://api.github.com".to_string(),
            token,
        }
    }
    
    pub async fn get_repository(&self, repo: &str) -> Result<Repository> {
        let url = format!("{}/repos/{}", self.base_url, repo);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API错误: {}", response.status()));
        }
        
        let repo_info: Repository = response.json().await?;
        Ok(repo_info)
    }
    
    pub async fn get_open_issues(&self, repo: &str, limit: usize) -> Result<Vec<Issue>> {
        let url = format!("{}/repos/{}/issues", self.base_url, repo);
        let response = self.http_client
            .get(&url)
            .query(&[("state", "open"), ("per_page", &limit.to_string())])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API错误: {}", response.status()));
        }
        
        let issues: Vec<Issue> = response.json().await?;
        Ok(issues)
    }
    
    // 其他 API 方法...
}
```

## 📊 数据结构设计

### 简化的数据模型
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub updated_at: String,
    pub topics: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub labels: Vec<Label>,
    pub user: User,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub user: User,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFileAnalysis {
    pub file_type: String,
    pub detected_tools: Vec<String>,
    pub framework_info: Option<String>,
}
```

## 🧪 测试策略

### 真实环境测试
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_github_basic_info() {
        let tool = GitHubInfoTool::new(None); // 无需token的公开仓库测试
        
        let params = json!({
            "repo": "microsoft/vscode",
            "type": "basic"
        });
        
        let result = tool.execute(params).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output["content"][0]["text"].as_str().unwrap().contains("vscode"));
    }
    
    #[tokio::test]
    async fn test_github_task_info() {
        let tool = GitHubInfoTool::new(None);
        
        let params = json!({
            "repo": "rust-lang/rust",
            "type": "tasks",
            "include_details": true
        });
        
        let result = tool.execute(params).await;
        assert!(result.is_ok());
    }
}
```

## ⚙️ 配置管理

### 环境变量配置
```env
# GitHub API配置
GITHUB_TOKEN=your_token_here
GITHUB_API_TIMEOUT_SECONDS=30

# 缓存配置
GITHUB_CACHE_TTL_HOURS=6
GITHUB_MAX_REQUESTS_PER_HOUR=4000
```

### 配置文件
```toml
[github]
api_url = "https://api.github.com"
timeout_seconds = 30
max_requests_per_hour = 4000
cache_ttl_hours = 6

[github.rate_limit]
enable_respect = true
fallback_delay_seconds = 60
max_retries = 3
```

## 🚀 部署和使用

### Windows 环境优化
- 使用标准的 Windows 路径处理
- 支持 PowerShell 环境变量
- 确保 HTTPS 证书验证正常工作

### 性能考虑
- 合理的缓存策略（6小时TTL）
- 遵守 GitHub API 限制
- 异步并发处理
- 错误重试机制

## 📈 监控和维护

### 关键指标
- GitHub API 调用成功率
- 响应时间统计
- 缓存命中率
- 错误类型分布

### 错误处理
- 网络超时自动重试
- API 限制友好降级
- 详细的错误日志记录
- 用户友好的错误信息

---

*项目上下文集成设计版本：v3.0*  
*最后更新：2025年1月*  
*简化设计专注实用价值* 