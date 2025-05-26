use std::collections::HashMap;
use std::process::Command;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// CLI工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliToolInfo {
    /// 工具名称
    pub name: String,
    /// 版本
    pub version: Option<String>,
    /// 可执行文件路径
    pub path: Option<String>,
    /// 是否可用
    pub available: bool,
    /// 检测到的特性
    pub features: Vec<String>,
}

/// CLI工具检测器
pub struct CliDetector {
    /// 已检测的工具缓存
    cache: HashMap<String, CliToolInfo>,
}

impl CliDetector {
    /// 创建新的CLI检测器
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// 检测所有相关的CLI工具
    pub async fn detect_all(&mut self) -> Result<HashMap<String, CliToolInfo>> {
        info!("🔍 开始检测环境中的CLI工具...");

        let tools_to_check = vec![
            // 版本控制工具
            ("git", vec!["--version"]),
            ("svn", vec!["--version"]),
            ("hg", vec!["--version"]),
            
            // 构建工具
            ("cargo", vec!["--version"]),
            ("npm", vec!["--version"]),
            ("yarn", vec!["--version"]),
            ("pnpm", vec!["--version"]),
            ("pip", vec!["--version"]),
            ("pipenv", vec!["--version"]),
            ("poetry", vec!["--version"]),
            ("maven", vec!["--version"]),
            ("mvn", vec!["--version"]),
            ("gradle", vec!["--version"]),
            ("go", vec!["version"]),
            ("dotnet", vec!["--version"]),
            ("swift", vec!["--version"]),
            ("flutter", vec!["--version"]),
            ("dart", vec!["--version"]),
            
            // 文档工具
            ("rustdoc", vec!["--version"]),
            ("cargo-doc", vec!["--version"]),
            ("jsdoc", vec!["--version"]),
            ("sphinx-build", vec!["--version"]),
            ("doxygen", vec!["--version"]),
            ("godoc", vec!["version"]),
            ("javadoc", vec!["--help"]),
            
            // 代码分析工具
            ("clippy", vec!["--version"]),
            ("eslint", vec!["--version"]),
            ("pylint", vec!["--version"]),
            ("flake8", vec!["--version"]),
            ("black", vec!["--version"]),
            ("prettier", vec!["--version"]),
            ("gofmt", vec!["--help"]),
            ("ktlint", vec!["--version"]),
            
            // 测试工具
            ("pytest", vec!["--version"]),
            ("jest", vec!["--version"]),
            ("mocha", vec!["--version"]),
            ("cypress", vec!["--version"]),
            ("newman", vec!["--version"]),
            
            // 容器工具
            ("docker", vec!["--version"]),
            ("podman", vec!["--version"]),
            ("kubernetes", vec!["version"]),
            ("kubectl", vec!["version", "--client"]),
            ("helm", vec!["version"]),
            
            // 云工具
            ("aws", vec!["--version"]),
            ("gcloud", vec!["--version"]),
            ("az", vec!["--version"]),
            ("terraform", vec!["--version"]),
            ("ansible", vec!["--version"]),
            
            // 数据库工具
            ("psql", vec!["--version"]),
            ("mysql", vec!["--version"]),
            ("redis-cli", vec!["--version"]),
            ("mongosh", vec!["--version"]),
            
            // 系统工具
            ("curl", vec!["--version"]),
            ("wget", vec!["--version"]),
            ("jq", vec!["--version"]),
            ("grep", vec!["--version"]),
            ("sed", vec!["--version"]),
            ("awk", vec!["--version"]),
        ];

        for (tool_name, version_args) in tools_to_check {
            let info = self.detect_tool(tool_name, &version_args).await;
            self.cache.insert(tool_name.to_string(), info);
        }

        // 检测特殊工具
        self.detect_special_tools().await?;

        info!("✅ CLI工具检测完成，找到 {} 个可用工具", 
            self.cache.values().filter(|t| t.available).count());

        Ok(self.cache.clone())
    }

    /// 检测单个CLI工具
    async fn detect_tool(&self, tool_name: &str, version_args: &[&str]) -> CliToolInfo {
        debug!("检测工具: {}", tool_name);

        // 首先检查工具是否存在
        let available = match Command::new(tool_name)
            .args(version_args)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let version_output = String::from_utf8_lossy(&output.stdout);
                    let version = self.extract_version(&version_output);
                    debug!("✅ {} 可用 (版本: {:?})", tool_name, version);
                    true
                } else {
                    debug!("❌ {} 命令执行失败", tool_name);
                    false
                }
            }
            Err(e) => {
                debug!("❌ {} 不可用: {}", tool_name, e);
                false
            }
        };

        // 如果工具可用，尝试获取更详细信息
        let (version, path, features) = if available {
            let version = self.get_tool_version(tool_name, version_args).await;
            let path = self.get_tool_path(tool_name).await;
            let features = self.detect_tool_features(tool_name).await;
            (version, path, features)
        } else {
            (None, None, vec![])
        };

        CliToolInfo {
            name: tool_name.to_string(),
            version,
            path,
            available,
            features,
        }
    }

    /// 检测特殊工具（需要特殊处理的工具）
    async fn detect_special_tools(&mut self) -> Result<()> {
        // 检测 cargo 子命令
        if self.cache.get("cargo").map(|t| t.available).unwrap_or(false) {
            self.detect_cargo_subcommands().await?;
        }

        // 检测 npm 全局包
        if self.cache.get("npm").map(|t| t.available).unwrap_or(false) {
            self.detect_npm_global_packages().await?;
        }

        // 检测 Python 包
        if self.cache.get("pip").map(|t| t.available).unwrap_or(false) {
            self.detect_python_packages().await?;
        }

        Ok(())
    }

    /// 检测 cargo 子命令
    async fn detect_cargo_subcommands(&mut self) -> Result<()> {
        let subcommands = vec![
            "doc", "test", "bench", "clippy", "fmt", "audit", "outdated", 
            "tree", "expand", "machete", "deny", "generate", "watch"
        ];

        for subcmd in subcommands {
            let cmd_name = format!("cargo-{}", subcmd);
            if let Ok(output) = Command::new("cargo")
                .args(&[subcmd, "--help"])
                .output()
            {
                if output.status.success() {
                    self.cache.insert(cmd_name.clone(), CliToolInfo {
                        name: cmd_name,
                        version: None,
                        path: None,
                        available: true,
                        features: vec!["cargo-subcommand".to_string()],
                    });
                }
            }
        }

        Ok(())
    }

    /// 检测 npm 全局包
    async fn detect_npm_global_packages(&mut self) -> Result<()> {
        if let Ok(output) = Command::new("npm")
            .args(&["list", "-g", "--depth=0", "--json"])
            .output()
        {
            if output.status.success() {
                let json_output = String::from_utf8_lossy(&output.stdout);
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_output) {
                    if let Some(dependencies) = parsed.get("dependencies").and_then(|d| d.as_object()) {
                        for (package_name, package_info) in dependencies {
                            let version = package_info.get("version")
                                .and_then(|v| v.as_str())
                                .map(String::from);

                            self.cache.insert(package_name.clone(), CliToolInfo {
                                name: package_name.clone(),
                                version,
                                path: None,
                                available: true,
                                features: vec!["npm-global-package".to_string()],
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 检测 Python 包
    async fn detect_python_packages(&mut self) -> Result<()> {
        let packages = vec![
            "sphinx", "mkdocs", "pdoc", "pydoc", "autopep8", "yapf",
            "mypy", "pycodestyle", "pydocstyle", "bandit"
        ];

        for package in packages {
            if let Ok(output) = Command::new("python")
                .args(&["-m", package, "--version"])
                .output()
            {
                if output.status.success() {
                    let version_output = String::from_utf8_lossy(&output.stdout);
                    let version = self.extract_version(&version_output);

                    self.cache.insert(package.to_string(), CliToolInfo {
                        name: package.to_string(),
                        version,
                        path: None,
                        available: true,
                        features: vec!["python-package".to_string()],
                    });
                }
            }
        }

        Ok(())
    }

    /// 获取工具版本
    async fn get_tool_version(&self, tool_name: &str, version_args: &[&str]) -> Option<String> {
        match Command::new(tool_name)
            .args(version_args)
            .output()
        {
            Ok(output) if output.status.success() => {
                let version_output = String::from_utf8_lossy(&output.stdout);
                self.extract_version(&version_output)
            }
            _ => None,
        }
    }

    /// 获取工具路径
    async fn get_tool_path(&self, tool_name: &str) -> Option<String> {
        match Command::new("which")
            .arg(tool_name)
            .output()
        {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => {
                // Windows fallback
                match Command::new("where")
                    .arg(tool_name)
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        let paths = String::from_utf8_lossy(&output.stdout);
                        paths.lines().next().map(|s| s.trim().to_string())
                    }
                    _ => None,
                }
            }
        }
    }

    /// 检测工具特性
    async fn detect_tool_features(&self, tool_name: &str) -> Vec<String> {
        let mut features = vec![];

        match tool_name {
            "git" => {
                features.push("version-control".to_string());
                // 检测 git 扩展功能
                if self.command_exists("git-lfs").await {
                    features.push("git-lfs".to_string());
                }
            }
            "docker" => {
                features.push("containerization".to_string());
                // 检测 docker-compose
                if self.command_exists("docker-compose").await {
                    features.push("docker-compose".to_string());
                }
            }
            "npm" => {
                features.push("package-manager".to_string());
                features.push("javascript".to_string());
            }
            "cargo" => {
                features.push("package-manager".to_string());
                features.push("rust".to_string());
                features.push("build-tool".to_string());
            }
            "pip" => {
                features.push("package-manager".to_string());
                features.push("python".to_string());
            }
            "mvn" | "maven" => {
                features.push("build-tool".to_string());
                features.push("java".to_string());
            }
            "gradle" => {
                features.push("build-tool".to_string());
                features.push("java".to_string());
                features.push("kotlin".to_string());
            }
            "go" => {
                features.push("compiler".to_string());
                features.push("go".to_string());
            }
            _ => {}
        }

        features
    }

    /// 检查命令是否存在
    async fn command_exists(&self, command: &str) -> bool {
        Command::new(command)
            .arg("--help")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// 从版本输出中提取版本号
    fn extract_version(&self, output: &str) -> Option<String> {
        use regex::Regex;
        
        // 常见的版本模式
        let patterns = vec![
            r"(\d+\.\d+\.\d+(?:\.\d+)?)",           // x.y.z 或 x.y.z.w
            r"v(\d+\.\d+\.\d+(?:\.\d+)?)",          // vx.y.z
            r"version\s+(\d+\.\d+\.\d+)",           // version x.y.z
            r"(\d+\.\d+)",                          // x.y
        ];

        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(captures) = re.captures(output) {
                    if let Some(version) = captures.get(1) {
                        return Some(version.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    /// 获取可用工具列表
    pub fn get_available_tools(&self) -> Vec<&CliToolInfo> {
        self.cache.values().filter(|tool| tool.available).collect()
    }

    /// 检查特定工具是否可用
    pub fn is_tool_available(&self, tool_name: &str) -> bool {
        self.cache.get(tool_name)
            .map(|tool| tool.available)
            .unwrap_or(false)
    }

    /// 获取工具信息
    pub fn get_tool_info(&self, tool_name: &str) -> Option<&CliToolInfo> {
        self.cache.get(tool_name)
    }

    /// 按特性过滤工具
    pub fn filter_by_feature(&self, feature: &str) -> Vec<&CliToolInfo> {
        self.cache.values()
            .filter(|tool| tool.available && tool.features.contains(&feature.to_string()))
            .collect()
    }

    /// 生成工具报告
    pub fn generate_report(&self) -> String {
        let available_count = self.cache.values().filter(|t| t.available).count();
        let total_count = self.cache.len();

        let mut report = format!(
            "🔧 CLI工具检测报告\n{}\n", 
            "=".repeat(50)
        );
        
        report.push_str(&format!(
            "📊 总结: {}/{} 工具可用\n\n", 
            available_count, total_count
        ));

        // 按类别分组
        let mut categories = HashMap::new();
        for tool in self.cache.values() {
            if tool.available {
                let category = if tool.features.contains(&"build-tool".to_string()) {
                    "构建工具"
                } else if tool.features.contains(&"package-manager".to_string()) {
                    "包管理器"
                } else if tool.features.contains(&"version-control".to_string()) {
                    "版本控制"
                } else if tool.features.contains(&"containerization".to_string()) {
                    "容器化"
                } else {
                    "其他工具"
                };

                categories.entry(category).or_insert_with(Vec::new).push(tool);
            }
        }

        for (category, tools) in categories {
            report.push_str(&format!("📁 {}\n", category));
            for tool in tools {
                let version_str = tool.version.as_ref()
                    .map(|v| format!(" ({})", v))
                    .unwrap_or_default();
                report.push_str(&format!("  ✅ {}{}\n", tool.name, version_str));
            }
            report.push('\n');
        }

        report
    }
} 