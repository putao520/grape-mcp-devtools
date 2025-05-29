use std::sync::Arc;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use tokio::process::Command as AsyncCommand;
use tokio::fs as async_fs;
use tracing::{info, warn, debug, error};
use uuid::Uuid;
use reqwest::Client;
use regex::Regex;

use crate::tools::base::{FileDocumentFragment, MCPTool};
use crate::vectorization::embeddings::{FileVectorizerImpl, EmbeddingConfig, VectorizationConfig};
use crate::tools::vector_docs_tool::VectorDocsTool;

/// 文档处理器 - 统一处理文档生成、向量化和存储
pub struct DocumentProcessor {
    /// 向量化器
    _vectorizer: Arc<FileVectorizerImpl>,
    /// 工作目录
    _work_dir: PathBuf,
    /// HTTP客户端
    client: Client,
    vector_tool: VectorDocsTool,
}

impl DocumentProcessor {
    /// 创建新的文档处理器
    pub async fn new() -> Result<Self> {
        let vector_tool = VectorDocsTool::new()?;
        
        // 创建向量化器配置
        let embedding_config = EmbeddingConfig::from_env()?;
        let vectorization_config = VectorizationConfig::from_env()?;
        
        // 创建工作目录
        let work_dir = std::env::temp_dir().join("grape-mcp-docs");
        std::fs::create_dir_all(&work_dir)?;
        
        // 异步创建向量化器
        let vectorizer = Arc::new(FileVectorizerImpl::new(embedding_config, vectorization_config).await?);
        
        Ok(Self {
            _vectorizer: vectorizer,
            _work_dir: work_dir,
            client: Client::new(),
            vector_tool,
        })
    }

    /// 处理文档请求的主要入口点
    /// 
    /// 流程：
    /// 1. 检查向量库中是否已有文档
    /// 2. 如果没有，生成新文档
    /// 3. 向量化文档内容
    /// 4. 存储到向量库
    /// 5. 返回处理结果
    pub async fn process_documentation_request(
        &self,
        language: &str,
        package_name: &str,
        version: Option<&str>,
        query: &str,
    ) -> Result<Vec<FileDocumentFragment>> {
        let version = version.unwrap_or("latest");
        
        info!("处理文档请求: {} {} {} - 查询: {}", language, package_name, version, query);
        
        // 1. 首先尝试从向量库搜索现有文档
        if let Ok(search_results) = self.search_existing_docs(language, package_name, version, query).await {
            if !search_results.is_empty() {
                info!("从向量库找到 {} 个相关文档", search_results.len());
                return Ok(search_results);
            }
        }
        
        // 2. 如果没有找到，生成新文档
        info!("向量库中没有找到相关文档，开始生成新文档");
        let fragments = self.generate_docs(language, package_name, version).await?;
        
        // 3. 向量化并存储文档
        self.vectorize_and_store_docs(&fragments).await?;
        
        // 4. 再次搜索以返回相关结果
        let search_results = self.search_existing_docs(language, package_name, version, query).await
            .unwrap_or_else(|_| fragments.clone());
        
        Ok(search_results)
    }
    
    /// 搜索现有文档
    async fn search_existing_docs(
        &self,
        language: &str,
        package_name: &str,
        version: &str,
        query: &str,
    ) -> Result<Vec<FileDocumentFragment>> {
        // 使用VectorDocsTool进行搜索
        let search_params = serde_json::json!({
            "action": "search",
            "query": format!("{} {} {}", language, package_name, query),
            "limit": 10
        });
        
        let search_result = self.vector_tool.execute(search_params).await?;
        
        if search_result["status"] == "success" && search_result["results_count"].as_u64().unwrap_or(0) > 0 {
            let empty_vec = vec![];
            let results = search_result["results"].as_array().unwrap_or(&empty_vec);
            let mut fragments = Vec::new();
            
            for result in results {
                if let (Some(title), Some(content), Some(lang)) = (
                    result["title"].as_str(),
                    result["content"].as_str(),
                    result["language"].as_str()
                ) {
                    if lang == language {
                        let fragment = FileDocumentFragment::new(
                            language.to_string(),
                            package_name.to_string(),
                            version.to_string(),
                            format!("{}.md", title.replace(" ", "_")),
                            content.to_string(),
                        );
                        fragments.push(fragment);
                    }
                }
            }
            
            return Ok(fragments);
        }
        
        Err(anyhow!("没有找到相关文档"))
    }
    
    /// 向量化并存储文档
    async fn vectorize_and_store_docs(&self, fragments: &[FileDocumentFragment]) -> Result<()> {
        info!("开始向量化并存储 {} 个文档片段", fragments.len());
        
        for fragment in fragments {
            let store_params = serde_json::json!({
                "action": "store",
                "title": fragment.file_path.clone(),
                "content": fragment.content.clone(),
                "language": fragment.language.clone(),
                "doc_type": "documentation"
            });
            
            match self.vector_tool.execute(store_params).await {
                Ok(result) => {
                    if result["status"] == "success" {
                        debug!("成功存储文档: {}", fragment.file_path);
                    } else {
                        warn!("存储文档失败: {} - {}", fragment.file_path, result);
                    }
                }
                Err(e) => {
                    error!("存储文档时发生错误: {} - {}", fragment.file_path, e);
                }
            }
        }
        
        info!("文档向量化和存储完成");
        Ok(())
    }

    /// 生成文档的主要方法
    async fn generate_docs(
        &self,
        language: &str,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<FileDocumentFragment>> {
        match language {
            "go" => self.generate_go_docs(package_name, Some(version)).await,
            "rust" => self.generate_rust_docs(package_name, version).await,
            "python" => self.generate_python_docs(package_name, version).await,
            "javascript" | "typescript" => self.generate_npm_docs(package_name, version).await,
            "java" => self.generate_java_docs(package_name, version).await,
            _ => Err(anyhow!("不支持的语言: {}", language)),
        }
    }
    
    /// 生成Go文档
    pub async fn generate_go_docs(&self, package_name: &str, version: Option<&str>) -> Result<Vec<FileDocumentFragment>> {
        let version = version.unwrap_or("latest");
        
        info!("生成Go文档: {} {}", package_name, version);
        
        // 1. 首先尝试使用go CLI工具
        if let Ok(fragments) = self.generate_go_docs_with_cli(package_name, version).await {
            return Ok(fragments);
        }
        
        // 2. 回退到pkg.go.dev API
        self.generate_go_docs_with_api(package_name, version).await
    }
    
    /// 使用go CLI生成文档
    async fn generate_go_docs_with_cli(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用go CLI生成文档: {} {}", package_name, version);
        
        // 检查go是否可用
        let go_check = AsyncCommand::new("go")
            .args(&["version"])
            .output()
            .await;
            
        if go_check.is_err() {
            return Err(anyhow!("go CLI不可用"));
        }
        
        // 使用go doc命令
        let doc_output = AsyncCommand::new("go")
            .args(&["doc", package_name])
            .output()
            .await?;
            
        if !doc_output.status.success() {
            return Err(anyhow!("go doc失败: {}", String::from_utf8_lossy(&doc_output.stderr)));
        }
        
        let doc_content = String::from_utf8_lossy(&doc_output.stdout);
        
        let fragment = FileDocumentFragment::new(
            "go".to_string(),
            package_name.to_string(),
            version.to_string(),
            "go_docs.md".to_string(),
            format!("# Go Package {}\n\nVersion: {}\n\n{}\n\nSource: go CLI", package_name, version, doc_content),
        );
        
        Ok(vec![fragment])
    }
    
    /// 使用pkg.go.dev API生成文档
    async fn generate_go_docs_with_api(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用pkg.go.dev API生成文档: {} {}", package_name, version);
        
        let url = format!("https://pkg.go.dev/{}", package_name);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Go包不存在: {}", package_name));
        }
        
        let html_content = response.text().await?;
        let cleaned_content = self.clean_html(&html_content);
        
        let fragment = FileDocumentFragment::new(
            "go".to_string(),
            package_name.to_string(),
            version.to_string(),
            "pkg_go_dev.md".to_string(),
            format!("# Go Package {}\n\nVersion: {}\n\n{}\n\nSource: pkg.go.dev", package_name, version, cleaned_content),
        );
        
        Ok(vec![fragment])
    }
    
    /// 生成Rust文档
    async fn generate_rust_docs(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("生成Rust文档: {} {}", package_name, version);
        
        // 1. 首先尝试使用cargo CLI工具
        if let Ok(fragments) = self.generate_rust_docs_with_cli(package_name, version).await {
            return Ok(fragments);
        }
        
        // 2. 回退到docs.rs API
        self.generate_rust_docs_with_api(package_name, version).await
    }
    
    /// 使用cargo CLI生成文档
    async fn generate_rust_docs_with_cli(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用cargo CLI生成文档: {} {}", package_name, version);
        
        // 检查cargo是否可用
        let cargo_check = AsyncCommand::new("cargo")
            .args(&["--version"])
            .output()
            .await;
            
        if cargo_check.is_err() {
            return Err(anyhow!("cargo CLI不可用"));
        }
        
        // 创建临时目录
        let temp_dir = std::env::temp_dir().join(format!("rust_docs_{}", Uuid::new_v4()));
        async_fs::create_dir_all(&temp_dir).await?;
        
        // 创建简单的Cargo.toml
        let cargo_content = format!(
            r#"[package]
name = "temp"
version = "0.1.0"
edition = "2021"

[dependencies]
{} = "{}"
"#,
            package_name, version
        );
        
        async_fs::write(temp_dir.join("Cargo.toml"), cargo_content).await?;
        async_fs::create_dir_all(temp_dir.join("src")).await?;
        async_fs::write(temp_dir.join("src").join("main.rs"), "fn main() {}").await?;
        
        // 生成文档
        let doc_output = AsyncCommand::new("cargo")
            .args(&["doc", "--no-deps"])
            .current_dir(&temp_dir)
            .output()
            .await?;
            
        if !doc_output.status.success() {
            return Err(anyhow!("cargo doc失败: {}", String::from_utf8_lossy(&doc_output.stderr)));
        }
        
        let fragment = FileDocumentFragment::new(
            "rust".to_string(),
            package_name.to_string(),
            version.to_string(),
            "cargo_docs.md".to_string(),
            format!("# Rust Crate {}\n\nVersion: {}\n\nDocumentation generated with cargo doc.\n\nSource: cargo CLI", package_name, version),
        );
        
        // 清理临时目录
        let _ = async_fs::remove_dir_all(&temp_dir).await;
        
        Ok(vec![fragment])
    }
    
    /// 使用docs.rs API生成文档
    async fn generate_rust_docs_with_api(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用docs.rs API生成文档: {} {}", package_name, version);
        
        let url = if version == "latest" {
            format!("https://docs.rs/{}", package_name)
        } else {
            format!("https://docs.rs/{}/{}", package_name, version)
        };
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Rust crate不存在: {}", package_name));
        }
        
        let html_content = response.text().await?;
        let cleaned_content = self.clean_html(&html_content);
        
        let fragment = FileDocumentFragment::new(
            "rust".to_string(),
            package_name.to_string(),
            version.to_string(),
            "docs_rs.md".to_string(),
            format!("# Rust Crate {}\n\nVersion: {}\n\n{}\n\nSource: docs.rs", package_name, version, cleaned_content),
        );
        
        Ok(vec![fragment])
    }
    
    /// 生成Python文档
    async fn generate_python_docs(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("生成Python文档: {} {}", package_name, version);
        
        // 1. 首先尝试使用pip CLI
        if let Ok(fragments) = self.generate_python_docs_with_cli(package_name, version).await {
            return Ok(fragments);
        }
        
        // 2. 回退到PyPI API
        self.generate_python_docs_with_api(package_name, version).await
    }
    
    /// 使用pip CLI生成文档
    async fn generate_python_docs_with_cli(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用Python CLI工具生成文档: {} {}", package_name, version);
        
        // 1. 首先尝试使用pip CLI
        if let Ok(fragment) = self.try_pip_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        // 2. 尝试使用poetry CLI
        if let Ok(fragment) = self.try_poetry_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        // 3. 尝试使用conda CLI
        if let Ok(fragment) = self.try_conda_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        // 4. 尝试使用pydoc CLI
        if let Ok(fragment) = self.try_pydoc_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        Err(anyhow!("所有Python CLI工具都不可用"))
    }
    
    /// 尝试使用pip CLI
    async fn try_pip_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查pip是否可用
        let pip_check = AsyncCommand::new("pip")
            .args(&["--version"])
            .output()
            .await;
            
        if pip_check.is_err() {
            return Err(anyhow!("pip CLI不可用"));
        }
        
        // 使用pip show命令获取包信息
        let show_output = AsyncCommand::new("pip")
            .args(&["show", package_name])
            .output()
            .await?;
            
        if !show_output.status.success() {
            return Err(anyhow!("pip show失败: {}", String::from_utf8_lossy(&show_output.stderr)));
        }
        
        let show_content = String::from_utf8_lossy(&show_output.stdout);
        
        // 尝试获取包的依赖信息
        let deps_output = AsyncCommand::new("pip")
            .args(&["show", package_name, "--verbose"])
            .output()
            .await;
            
        let deps_info = if let Ok(output) = deps_output {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                show_content.to_string()
            }
        } else {
            show_content.to_string()
        };
        
        let content = format!(
            "# Python Package {}\n\nVersion: {}\n\n## Package Information\n\n```\n{}\n```\n\n## Installation\n\n```bash\npip install {}=={}\n```\n\nSource: pip CLI",
            package_name, version, deps_info, package_name, version
        );
        
        Ok(FileDocumentFragment::new(
            "python".to_string(),
            package_name.to_string(),
            version.to_string(),
            "pip_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用poetry CLI
    async fn try_poetry_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查poetry是否可用
        let poetry_check = AsyncCommand::new("poetry")
            .args(&["--version"])
            .output()
            .await;
            
        if poetry_check.is_err() {
            return Err(anyhow!("poetry CLI不可用"));
        }
        
        // 使用poetry show命令获取包信息
        let show_output = AsyncCommand::new("poetry")
            .args(&["show", package_name])
            .output()
            .await?;
            
        if !show_output.status.success() {
            return Err(anyhow!("poetry show失败: {}", String::from_utf8_lossy(&show_output.stderr)));
        }
        
        let show_content = String::from_utf8_lossy(&show_output.stdout);
        
        let content = format!(
            "# Python Package {}\n\nVersion: {}\n\n## Poetry Information\n\n```\n{}\n```\n\n## Installation\n\n### Poetry\n```bash\npoetry add {}=={}\n```\n\n### pip\n```bash\npip install {}=={}\n```\n\nSource: Poetry CLI",
            package_name, version, show_content, package_name, version, package_name, version
        );
        
        Ok(FileDocumentFragment::new(
            "python".to_string(),
            package_name.to_string(),
            version.to_string(),
            "poetry_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用conda CLI
    async fn try_conda_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查conda是否可用
        let conda_check = AsyncCommand::new("conda")
            .args(&["--version"])
            .output()
            .await;
            
        if conda_check.is_err() {
            return Err(anyhow!("conda CLI不可用"));
        }
        
        // 使用conda search命令查找包
        let search_output = AsyncCommand::new("conda")
            .args(&["search", package_name])
            .output()
            .await?;
            
        let search_content = if search_output.status.success() {
            String::from_utf8_lossy(&search_output.stdout).to_string()
        } else {
            "Package not found in conda repositories".to_string()
        };
        
        let content = format!(
            "# Python Package {}\n\nVersion: {}\n\n## Conda Information\n\n```\n{}\n```\n\n## Installation\n\n### Conda\n```bash\nconda install {}={}\n```\n\n### pip (fallback)\n```bash\npip install {}=={}\n```\n\nSource: Conda CLI",
            package_name, version, search_content, package_name, version, package_name, version
        );
        
        Ok(FileDocumentFragment::new(
            "python".to_string(),
            package_name.to_string(),
            version.to_string(),
            "conda_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用pydoc CLI
    async fn try_pydoc_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查python是否可用
        let python_check = AsyncCommand::new("python")
            .args(&["--version"])
            .output()
            .await;
            
        if python_check.is_err() {
            return Err(anyhow!("python CLI不可用"));
        }
        
        // 尝试使用pydoc获取模块文档
        let pydoc_output = AsyncCommand::new("python")
            .args(&["-m", "pydoc", package_name])
            .output()
            .await?;
            
        let pydoc_content = if pydoc_output.status.success() {
            String::from_utf8_lossy(&pydoc_output.stdout).to_string()
        } else {
            // 如果pydoc失败，尝试导入模块获取基本信息
            let import_output = AsyncCommand::new("python")
                .args(&["-c", &format!("import {}; print({}.__doc__ or 'No documentation available')", package_name, package_name)])
                .output()
                .await;
                
            if let Ok(output) = import_output {
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout).to_string()
                } else {
                    format!("Module {} documentation not available", package_name)
                }
            } else {
                format!("Module {} not found", package_name)
            }
        };
        
        let content = format!(
            "# Python Package {}\n\nVersion: {}\n\n## Module Documentation\n\n```\n{}\n```\n\n## Installation\n\n```bash\npip install {}=={}\n```\n\n## Usage\n\n```python\nimport {}\n```\n\nSource: pydoc CLI",
            package_name, version, pydoc_content, package_name, version, package_name
        );
        
        Ok(FileDocumentFragment::new(
            "python".to_string(),
            package_name.to_string(),
            version.to_string(),
            "pydoc_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 使用PyPI API生成文档
    async fn generate_python_docs_with_api(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用PyPI API生成文档: {} {}", package_name, version);
        
        let url = format!("https://pypi.org/pypi/{}/json", package_name);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Python包不存在: {}", package_name));
        }
        
        let json_content: serde_json::Value = response.json().await?;
        let description = json_content["info"]["description"].as_str().unwrap_or("No description available");
        
        let fragment = FileDocumentFragment::new(
            "python".to_string(),
            package_name.to_string(),
            version.to_string(),
            "pypi_docs.md".to_string(),
            format!("# Python Package {}\n\nVersion: {}\n\n{}\n\nSource: PyPI API", package_name, version, description),
        );
        
        Ok(vec![fragment])
    }
    
    /// 生成NPM文档
    async fn generate_npm_docs(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("生成NPM文档: {} {}", package_name, version);
        
        // 1. 首先尝试使用npm CLI工具
        if let Ok(fragments) = self.generate_npm_docs_with_cli(package_name, version).await {
            return Ok(fragments);
        }
        
        // 2. 回退到NPM API
        self.generate_npm_docs_with_api(package_name, version).await
    }
    
    /// 使用npm CLI生成文档
    async fn generate_npm_docs_with_cli(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用JavaScript/Node.js CLI工具生成文档: {} {}", package_name, version);
        
        // 1. 首先尝试使用npm CLI
        if let Ok(fragment) = self.try_npm_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        // 2. 尝试使用yarn CLI
        if let Ok(fragment) = self.try_yarn_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        // 3. 尝试使用pnpm CLI
        if let Ok(fragment) = self.try_pnpm_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        // 4. 尝试使用node CLI
        if let Ok(fragment) = self.try_node_cli(package_name, version).await {
            return Ok(vec![fragment]);
        }
        
        Err(anyhow!("所有JavaScript/Node.js CLI工具都不可用"))
    }
    
    /// 尝试使用npm CLI
    async fn try_npm_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查npm是否可用
        let npm_check = AsyncCommand::new("npm")
            .args(&["--version"])
            .output()
            .await;
            
        if npm_check.is_err() {
            return Err(anyhow!("npm CLI不可用"));
        }
        
        // 使用npm view命令获取包信息
        let view_output = AsyncCommand::new("npm")
            .args(&["view", package_name, "--json"])
            .output()
            .await?;
            
        if !view_output.status.success() {
            return Err(anyhow!("npm view失败: {}", String::from_utf8_lossy(&view_output.stderr)));
        }
        
        let view_content = String::from_utf8_lossy(&view_output.stdout);
        
        // 尝试获取包的依赖信息
        let deps_output = AsyncCommand::new("npm")
            .args(&["view", package_name, "dependencies", "--json"])
            .output()
            .await;
            
        let deps_info = if let Ok(output) = deps_output {
            if output.status.success() {
                format!("\n\n## Dependencies\n\n```json\n{}\n```", String::from_utf8_lossy(&output.stdout))
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        let content = format!(
            "# NPM Package {}\n\nVersion: {}\n\n## Package Information\n\n```json\n{}\n```{}\n\n## Installation\n\n```bash\nnpm install {}@{}\n```\n\nSource: npm CLI",
            package_name, version, view_content, deps_info, package_name, version
        );
        
        Ok(FileDocumentFragment::new(
            "javascript".to_string(),
            package_name.to_string(),
            version.to_string(),
            "npm_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用yarn CLI
    async fn try_yarn_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查yarn是否可用
        let yarn_check = AsyncCommand::new("yarn")
            .args(&["--version"])
            .output()
            .await;
            
        if yarn_check.is_err() {
            return Err(anyhow!("yarn CLI不可用"));
        }
        
        // 使用yarn info命令获取包信息
        let info_output = AsyncCommand::new("yarn")
            .args(&["info", package_name, "--json"])
            .output()
            .await?;
            
        if !info_output.status.success() {
            return Err(anyhow!("yarn info失败: {}", String::from_utf8_lossy(&info_output.stderr)));
        }
        
        let info_content = String::from_utf8_lossy(&info_output.stdout);
        
        let content = format!(
            "# NPM Package {}\n\nVersion: {}\n\n## Yarn Information\n\n```json\n{}\n```\n\n## Installation\n\n### Yarn\n```bash\nyarn add {}@{}\n```\n\n### npm\n```bash\nnpm install {}@{}\n```\n\nSource: Yarn CLI",
            package_name, version, info_content, package_name, version, package_name, version
        );
        
        Ok(FileDocumentFragment::new(
            "javascript".to_string(),
            package_name.to_string(),
            version.to_string(),
            "yarn_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用pnpm CLI
    async fn try_pnpm_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查pnpm是否可用
        let pnpm_check = AsyncCommand::new("pnpm")
            .args(&["--version"])
            .output()
            .await;
            
        if pnpm_check.is_err() {
            return Err(anyhow!("pnpm CLI不可用"));
        }
        
        // 使用pnpm view命令获取包信息
        let view_output = AsyncCommand::new("pnpm")
            .args(&["view", package_name, "--json"])
            .output()
            .await?;
            
        if !view_output.status.success() {
            return Err(anyhow!("pnpm view失败: {}", String::from_utf8_lossy(&view_output.stderr)));
        }
        
        let view_content = String::from_utf8_lossy(&view_output.stdout);
        
        let content = format!(
            "# NPM Package {}\n\nVersion: {}\n\n## pnpm Information\n\n```json\n{}\n```\n\n## Installation\n\n### pnpm\n```bash\npnpm add {}@{}\n```\n\n### npm\n```bash\nnpm install {}@{}\n```\n\n### Yarn\n```bash\nyarn add {}@{}\n```\n\nSource: pnpm CLI",
            package_name, version, view_content, package_name, version, package_name, version, package_name, version
        );
        
        Ok(FileDocumentFragment::new(
            "javascript".to_string(),
            package_name.to_string(),
            version.to_string(),
            "pnpm_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用node CLI
    async fn try_node_cli(&self, package_name: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查node是否可用
        let node_check = AsyncCommand::new("node")
            .args(&["--version"])
            .output()
            .await;
            
        if node_check.is_err() {
            return Err(anyhow!("node CLI不可用"));
        }
        
        // 尝试使用node获取模块信息
        let module_script = format!(
            "try {{ const pkg = require('{}'); console.log(JSON.stringify({{ name: '{}', version: pkg.version || 'unknown', description: pkg.description || 'No description' }}, null, 2)); }} catch(e) {{ console.log(JSON.stringify({{ error: e.message }}, null, 2)); }}",
            package_name, package_name
        );
        
        let node_output = AsyncCommand::new("node")
            .args(&["-e", &module_script])
            .output()
            .await?;
            
        let node_content = if node_output.status.success() {
            String::from_utf8_lossy(&node_output.stdout).to_string()
        } else {
            format!("{{ \"error\": \"Module {} not found or not installed\" }}", package_name)
        };
        
        let content = format!(
            "# NPM Package {}\n\nVersion: {}\n\n## Node.js Module Information\n\n```json\n{}\n```\n\n## Installation\n\n```bash\nnpm install {}@{}\n```\n\n## Usage\n\n```javascript\nconst {} = require('{}');\n```\n\nSource: Node.js CLI",
            package_name, version, node_content, package_name, version, package_name, package_name
        );
        
        Ok(FileDocumentFragment::new(
            "javascript".to_string(),
            package_name.to_string(),
            version.to_string(),
            "node_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 使用NPM API生成文档
    async fn generate_npm_docs_with_api(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用NPM API生成文档: {} {}", package_name, version);
        
        let url = format!("https://registry.npmjs.org/{}", package_name);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("NPM包不存在: {}", package_name));
        }
        
        let json_content: serde_json::Value = response.json().await?;
        let description = json_content["description"].as_str().unwrap_or("No description available");
        let readme = json_content["readme"].as_str().unwrap_or("No README available");
        
        let fragment = FileDocumentFragment::new(
            "javascript".to_string(),
            package_name.to_string(),
            version.to_string(),
            "npm_api_docs.md".to_string(),
            format!("# NPM Package {}\n\nVersion: {}\n\n## Description\n{}\n\n## README\n{}\n\nSource: NPM API", package_name, version, description, readme),
        );
        
        Ok(vec![fragment])
    }
    
    /// 生成Java文档
    async fn generate_java_docs(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("生成Java文档: {} {}", package_name, version);
        
        // 1. 首先尝试使用mvn CLI工具
        if let Ok(fragments) = self.generate_java_docs_with_cli(package_name, version).await {
            return Ok(fragments);
        }
        
        // 2. 回退到Maven Central API
        self.generate_java_docs_with_api(package_name, version).await
    }
    
    /// 使用mvn CLI生成文档
    async fn generate_java_docs_with_cli(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用Java CLI工具生成文档: {} {}", package_name, version);
        
        // 解析Maven坐标
        let parts: Vec<&str> = package_name.split(':').collect();
        if parts.len() < 2 {
            return Err(anyhow!("无效的Maven坐标格式，应为 groupId:artifactId"));
        }
        
        let group_id = parts[0];
        let artifact_id = parts[1];
        
        // 1. 首先尝试使用mvn CLI
        if let Ok(fragment) = self.try_mvn_cli(group_id, artifact_id, version).await {
            return Ok(vec![fragment]);
        }
        
        // 2. 尝试使用gradle CLI
        if let Ok(fragment) = self.try_gradle_cli(group_id, artifact_id, version).await {
            return Ok(vec![fragment]);
        }
        
        // 3. 尝试使用javadoc CLI
        if let Ok(fragment) = self.try_javadoc_cli(group_id, artifact_id, version).await {
            return Ok(vec![fragment]);
        }
        
        Err(anyhow!("所有Java CLI工具都不可用"))
    }
    
    /// 尝试使用mvn CLI
    async fn try_mvn_cli(&self, group_id: &str, artifact_id: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查mvn是否可用
        let mvn_check = AsyncCommand::new("mvn")
            .args(&["--version"])
            .output()
            .await;
            
        if mvn_check.is_err() {
            return Err(anyhow!("mvn CLI不可用"));
        }
        
        // 创建临时目录
        let temp_dir = std::env::temp_dir().join(format!("java_docs_{}", Uuid::new_v4()));
        async_fs::create_dir_all(&temp_dir).await?;
        
        // 创建简单的pom.xml
        let pom_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>temp</groupId>
    <artifactId>temp</artifactId>
    <version>1.0.0</version>
    <properties>
        <maven.compiler.source>11</maven.compiler.source>
        <maven.compiler.target>11</maven.compiler.target>
    </properties>
    <dependencies>
        <dependency>
            <groupId>{}</groupId>
            <artifactId>{}</artifactId>
            <version>{}</version>
        </dependency>
    </dependencies>
</project>"#,
            group_id, artifact_id, version
        );
        
        async_fs::write(temp_dir.join("pom.xml"), pom_content).await?;
        
        // 使用mvn dependency:resolve命令解析依赖
        let resolve_output = AsyncCommand::new("mvn")
            .args(&["dependency:resolve", "-q"])
            .current_dir(&temp_dir)
            .output()
            .await?;
            
        if !resolve_output.status.success() {
            return Err(anyhow!("Maven依赖解析失败: {}", String::from_utf8_lossy(&resolve_output.stderr)));
        }
        
        // 使用mvn dependency:tree获取依赖树
        let tree_output = AsyncCommand::new("mvn")
            .args(&["dependency:tree", "-q"])
            .current_dir(&temp_dir)
            .output()
            .await?;
            
        let dependency_tree = if tree_output.status.success() {
            String::from_utf8_lossy(&tree_output.stdout).to_string()
        } else {
            "Dependency tree not available".to_string()
        };
        
        // 清理临时目录
        let _ = async_fs::remove_dir_all(&temp_dir).await;
        
        let content = format!(
            "# Java Library {}:{}\n\nVersion: {}\n\n## Maven Information\n\nGroup ID: {}\nArtifact ID: {}\n\n## Dependency Tree\n\n```\n{}\n```\n\n## Installation\n\n### Maven\n```xml\n<dependency>\n    <groupId>{}</groupId>\n    <artifactId>{}</artifactId>\n    <version>{}</version>\n</dependency>\n```\n\n### Gradle\n```gradle\nimplementation '{}:{}:{}'\n```\n\nSource: Maven CLI",
            group_id, artifact_id, version, group_id, artifact_id, dependency_tree, group_id, artifact_id, version, group_id, artifact_id, version
        );
        
        Ok(FileDocumentFragment::new(
            "java".to_string(),
            format!("{}:{}", group_id, artifact_id),
            version.to_string(),
            "maven_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用gradle CLI
    async fn try_gradle_cli(&self, group_id: &str, artifact_id: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查gradle是否可用
        let gradle_check = AsyncCommand::new("gradle")
            .args(&["--version"])
            .output()
            .await;
            
        if gradle_check.is_err() {
            return Err(anyhow!("gradle CLI不可用"));
        }
        
        // 创建临时目录
        let temp_dir = std::env::temp_dir().join(format!("gradle_docs_{}", Uuid::new_v4()));
        async_fs::create_dir_all(&temp_dir).await?;
        
        // 创建简单的build.gradle
        let build_gradle_content = format!(
            r#"plugins {{
    id 'java'
}}

repositories {{
    mavenCentral()
}}

dependencies {{
    implementation '{}:{}:{}'
}}
"#,
            group_id, artifact_id, version
        );
        
        async_fs::write(temp_dir.join("build.gradle"), build_gradle_content).await?;
        
        // 使用gradle dependencies命令获取依赖信息
        let deps_output = AsyncCommand::new("gradle")
            .args(&["dependencies", "--configuration", "compileClasspath", "-q"])
            .current_dir(&temp_dir)
            .output()
            .await?;
            
        let dependencies_info = if deps_output.status.success() {
            String::from_utf8_lossy(&deps_output.stdout).to_string()
        } else {
            "Dependencies information not available".to_string()
        };
        
        // 清理临时目录
        let _ = async_fs::remove_dir_all(&temp_dir).await;
        
        let content = format!(
            "# Java Library {}:{}\n\nVersion: {}\n\n## Gradle Information\n\nGroup ID: {}\nArtifact ID: {}\n\n## Dependencies\n\n```\n{}\n```\n\n## Installation\n\n### Gradle\n```gradle\nimplementation '{}:{}:{}'\n```\n\n### Maven\n```xml\n<dependency>\n    <groupId>{}</groupId>\n    <artifactId>{}</artifactId>\n    <version>{}</version>\n</dependency>\n```\n\nSource: Gradle CLI",
            group_id, artifact_id, version, group_id, artifact_id, dependencies_info, group_id, artifact_id, version, group_id, artifact_id, version
        );
        
        Ok(FileDocumentFragment::new(
            "java".to_string(),
            format!("{}:{}", group_id, artifact_id),
            version.to_string(),
            "gradle_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 尝试使用javadoc CLI
    async fn try_javadoc_cli(&self, group_id: &str, artifact_id: &str, version: &str) -> Result<FileDocumentFragment> {
        // 检查javadoc是否可用
        let javadoc_check = AsyncCommand::new("javadoc")
            .args(&["-version"])
            .output()
            .await;
            
        if javadoc_check.is_err() {
            return Err(anyhow!("javadoc CLI不可用"));
        }
        
        let javadoc_output = javadoc_check.unwrap();
        let javadoc_version = String::from_utf8_lossy(&javadoc_output.stdout);
        
        let content = format!(
            "# Java Library {}:{}\n\nVersion: {}\n\n## Javadoc Information\n\nGroup ID: {}\nArtifact ID: {}\nJavadoc Version: {}\n\n## Documentation Links\n\n- [Javadoc.io](https://javadoc.io/doc/{}/{})\n- [Maven Central](https://search.maven.org/artifact/{}/{})\n\n## Installation\n\n### Maven\n```xml\n<dependency>\n    <groupId>{}</groupId>\n    <artifactId>{}</artifactId>\n    <version>{}</version>\n</dependency>\n```\n\n### Gradle\n```gradle\nimplementation '{}:{}:{}'\n```\n\nSource: Javadoc CLI",
            group_id, artifact_id, version, group_id, artifact_id, javadoc_version.trim(), group_id, artifact_id, group_id, artifact_id, group_id, artifact_id, version, group_id, artifact_id, version
        );
        
        Ok(FileDocumentFragment::new(
            "java".to_string(),
            format!("{}:{}", group_id, artifact_id),
            version.to_string(),
            "javadoc_cli_docs.md".to_string(),
            content,
        ))
    }
    
    /// 使用Maven Central API生成文档
    async fn generate_java_docs_with_api(&self, package_name: &str, version: &str) -> Result<Vec<FileDocumentFragment>> {
        info!("使用Maven Central API生成文档: {} {}", package_name, version);
        
        // 解析Maven坐标
        let parts: Vec<&str> = package_name.split(':').collect();
        if parts.len() < 2 {
            return Err(anyhow!("无效的Maven坐标格式，应为 groupId:artifactId"));
        }
        
        let group_id = parts[0];
        let artifact_id = parts[1];
        
        let url = format!("https://search.maven.org/solrsearch/select?q=g:{}+AND+a:{}&rows=1&wt=json", group_id, artifact_id);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Maven Central API请求失败"));
        }
        
        let json_content: serde_json::Value = response.json().await?;
        let empty_docs = vec![];
        let docs = json_content["response"]["docs"].as_array().unwrap_or(&empty_docs);
        
        if docs.is_empty() {
            return Err(anyhow!("在Maven Central中找不到该库"));
        }
        
        let doc = &docs[0];
        let latest_version = doc["latestVersion"].as_str().unwrap_or(version);
        
        let fragment = FileDocumentFragment::new(
            "java".to_string(),
            package_name.to_string(),
            version.to_string(),
            "maven_central_docs.md".to_string(),
            format!("# Java Library {}\n\nVersion: {}\nLatest Version: {}\n\nGroup ID: {}\nArtifact ID: {}\n\nSource: Maven Central API", package_name, version, latest_version, group_id, artifact_id),
        );
        
        Ok(vec![fragment])
    }
    
    /// 清理HTML标签，保留文本内容
    fn clean_html(&self, html: &str) -> String {
        // 移除脚本和样式标签及其内容
        let script_re = Regex::new(r"(?s)<script[^>]*>.*?</script>").unwrap();
        let style_re = Regex::new(r"(?s)<style[^>]*>.*?</style>").unwrap();
        let mut cleaned = script_re.replace_all(html, "").to_string();
        cleaned = style_re.replace_all(&cleaned, "").to_string();
        
        // 移除HTML注释
        let comment_re = Regex::new(r"(?s)<!--.*?-->").unwrap();
        cleaned = comment_re.replace_all(&cleaned, "").to_string();
        
        // 移除所有HTML标签
        let tag_re = Regex::new(r"<[^>]*>").unwrap();
        cleaned = tag_re.replace_all(&cleaned, "").to_string();
        
        // 解码HTML实体
        cleaned = cleaned
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");
        
        // 清理多余的空白字符
        let space_re = Regex::new(r"\s+").unwrap();
        let result = space_re.replace_all(&cleaned, " ").trim().to_string();
        
        // 如果清理后内容太短，返回默认内容
        if result.len() < 10 {
            "Documentation content extracted from HTML".to_string()
        } else {
            result
        }
    }
} 