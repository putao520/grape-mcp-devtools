use std::collections::HashMap;
use std::process::Command;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};
use crate::cli::detector::CliToolInfo;

/// Windows管理员权限检测
#[cfg(target_os = "windows")]
fn is_elevated() -> bool {
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    
    unsafe {
        let mut token_handle = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) == 0 {
            return false;
        }
        
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len = 0u32;
        
        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        
        CloseHandle(token_handle);
        
        if result != 0 {
            elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn is_elevated() -> bool {
    // 在非Windows系统上，检查是否为root用户
    unsafe { libc::getuid() == 0 }
}

/// 工具安装策略
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// 工具安装配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInstallConfig {
    pub strategy: InstallStrategy,
    pub auto_upgrade: bool,
    pub install_timeout_secs: u64,
    pub prefer_global: bool,
    pub backup_existing: bool,
}

impl Default for ToolInstallConfig {
    fn default() -> Self {
        Self {
            strategy: InstallStrategy::Interactive,
            auto_upgrade: true,
            install_timeout_secs: 300, // 5分钟
            prefer_global: true,
            backup_existing: false,
        }
    }
}

/// 工具安装信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInstallInfo {
    pub tool_name: String,
    pub language: String,
    pub install_command: String,
    pub upgrade_command: Option<String>,
    pub check_command: String,
    pub required_dependencies: Vec<String>,
    pub install_method: InstallMethod,
    pub priority: u8, // 1-10, 10最高
}

/// 安装方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallMethod {
    /// 包管理器安装
    PackageManager(String), // cargo, npm, pip, etc.
    /// 系统包管理器
    SystemPackage(String), // apt, yum, brew, choco, etc.
    /// 直接下载
    DirectDownload(String),
    /// 从源码编译
    CompileFromSource(String),
    /// 自定义脚本
    CustomScript(String),
}

/// 工具安装器
pub struct ToolInstaller {
    config: ToolInstallConfig,
    supported_tools: HashMap<String, ToolInstallInfo>,
    system_detector: SystemDetector,
}

/// 系统检测器
#[derive(Debug)]
pub struct SystemDetector {
    pub os_type: String,
    pub package_managers: Vec<String>,
}

impl SystemDetector {
    fn new() -> Self {
        let os_type = if cfg!(target_os = "windows") {
            "windows".to_string()
        } else if cfg!(target_os = "macos") {
            "macos".to_string()
        } else {
            "linux".to_string()
        };

        let package_managers = Self::detect_package_managers(&os_type);
        
        Self {
            os_type,
            package_managers,
        }
    }

    /// 获取操作系统类型
    pub fn get_os_type(&self) -> &str {
        &self.os_type
    }

    /// 获取可用的包管理器列表
    pub fn get_package_managers(&self) -> &Vec<String> {
        &self.package_managers
    }

    fn detect_package_managers(os_type: &str) -> Vec<String> {
        let mut managers = Vec::new();
        
        match os_type {
            "windows" => {
                if Self::command_exists("choco") {
                    managers.push("choco".to_string());
                }
                if Self::command_exists("winget") {
                    managers.push("winget".to_string());
                }
                if Self::command_exists("scoop") {
                    managers.push("scoop".to_string());
                }
            }
            "macos" => {
                if Self::command_exists("brew") {
                    managers.push("brew".to_string());
                }
                if Self::command_exists("port") {
                    managers.push("port".to_string());
                }
            }
            "linux" => {
                if Self::command_exists("apt") {
                    managers.push("apt".to_string());
                }
                if Self::command_exists("yum") {
                    managers.push("yum".to_string());
                }
                if Self::command_exists("dnf") {
                    managers.push("dnf".to_string());
                }
                if Self::command_exists("pacman") {
                    managers.push("pacman".to_string());
                }
                if Self::command_exists("zypper") {
                    managers.push("zypper".to_string());
                }
            }
            _ => {}
        }
        
        managers
    }

    fn command_exists(command: &str) -> bool {
        Command::new(command)
            .arg("--version")
            .output()
            .is_ok()
    }
}

impl ToolInstaller {
    pub fn new(config: ToolInstallConfig) -> Self {
        let mut installer = Self {
            config,
            supported_tools: HashMap::new(),
            system_detector: SystemDetector::new(),
        };
        
        installer.initialize_tool_definitions();
        
        // 检查管理员权限
        #[cfg(target_os = "windows")]
        {
            let is_admin = is_elevated();
            if !is_admin {
                warn!("⚠️ 检测到非管理员模式运行，部分工具安装需要管理员权限");
                info!("💡 提示: 使用 '以管理员身份运行' 启动程序可启用自动安装功能");
            } else {
                info!("✅ 检测到管理员权限，可以执行自动安装");
            }
        }
        
        installer
    }

    /// 初始化支持的工具定义
    fn initialize_tool_definitions(&mut self) {
        // Rust 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "rustdoc".to_string(),
            language: "rust".to_string(),
            install_command: "rustup component add rustfmt".to_string(),
            upgrade_command: Some("rustup update".to_string()),
            check_command: "rustdoc --version".to_string(),
            required_dependencies: vec!["rust".to_string(), "cargo".to_string()],
            install_method: InstallMethod::PackageManager("rustup".to_string()),
            priority: 10,
        });

        self.add_tool_definition(ToolInstallInfo {
            tool_name: "cargo-doc".to_string(),
            language: "rust".to_string(),
            install_command: "cargo install cargo-doc".to_string(),
            upgrade_command: Some("cargo install --force cargo-doc".to_string()),
            check_command: "cargo doc --help".to_string(),
            required_dependencies: vec!["cargo".to_string()],
            install_method: InstallMethod::PackageManager("cargo".to_string()),
            priority: 8,
        });

        // Python 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "sphinx".to_string(),
            language: "python".to_string(),
            install_command: "pip install sphinx".to_string(),
            upgrade_command: Some("pip install --upgrade sphinx".to_string()),
            check_command: "sphinx-build --version".to_string(),
            required_dependencies: vec!["python".to_string(), "pip".to_string()],
            install_method: InstallMethod::PackageManager("pip".to_string()),
            priority: 9,
        });

        self.add_tool_definition(ToolInstallInfo {
            tool_name: "pydoc".to_string(),
            language: "python".to_string(),
            install_command: "python -m pydoc".to_string(), // 内置模块
            upgrade_command: None, // 随Python一起更新
            check_command: "python -m pydoc -h".to_string(),
            required_dependencies: vec!["python".to_string()],
            install_method: InstallMethod::PackageManager("built-in".to_string()),
            priority: 7,
        });

        // JavaScript/Node.js 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "jsdoc".to_string(),
            language: "javascript".to_string(),
            install_command: "npm install -g jsdoc".to_string(),
            upgrade_command: Some("npm update -g jsdoc".to_string()),
            check_command: "jsdoc --version".to_string(),
            required_dependencies: vec!["node".to_string(), "npm".to_string()],
            install_method: InstallMethod::PackageManager("npm".to_string()),
            priority: 9,
        });

        self.add_tool_definition(ToolInstallInfo {
            tool_name: "typedoc".to_string(),
            language: "typescript".to_string(),
            install_command: "npm install -g typedoc".to_string(),
            upgrade_command: Some("npm update -g typedoc".to_string()),
            check_command: "typedoc --version".to_string(),
            required_dependencies: vec!["node".to_string(), "npm".to_string(), "typescript".to_string()],
            install_method: InstallMethod::PackageManager("npm".to_string()),
            priority: 9,
        });

        // Java 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "javadoc".to_string(),
            language: "java".to_string(),
            install_command: "# javadoc comes with JDK".to_string(),
            upgrade_command: None, // 随JDK更新
            check_command: "javadoc -help".to_string(),
            required_dependencies: vec!["java".to_string()],
            install_method: InstallMethod::PackageManager("built-in".to_string()),
            priority: 10,
        });

        // Go 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "godoc".to_string(),
            language: "go".to_string(),
            install_command: "go install golang.org/x/tools/cmd/godoc@latest".to_string(),
            upgrade_command: Some("go install golang.org/x/tools/cmd/godoc@latest".to_string()),
            check_command: "godoc -help".to_string(),
            required_dependencies: vec!["go".to_string()],
            install_method: InstallMethod::PackageManager("go".to_string()),
            priority: 9,
        });

        // C# 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "docfx".to_string(),
            language: "csharp".to_string(),
            install_command: "dotnet tool install -g docfx".to_string(),
            upgrade_command: Some("dotnet tool update -g docfx".to_string()),
            check_command: "docfx --version".to_string(),
            required_dependencies: vec!["dotnet".to_string()],
            install_method: InstallMethod::PackageManager("dotnet".to_string()),
            priority: 9,
        });

        // C++ 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "doxygen".to_string(),
            language: "cpp".to_string(),
            install_command: self.get_system_install_command("doxygen"),
            upgrade_command: Some(self.get_system_upgrade_command("doxygen")),
            check_command: "doxygen --version".to_string(),
            required_dependencies: vec![],
            install_method: InstallMethod::SystemPackage("system".to_string()),
            priority: 9,
        });

        // PHP 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "phpdoc".to_string(),
            language: "php".to_string(),
            install_command: "composer global require phpdocumentor/phpdocumentor".to_string(),
            upgrade_command: Some("composer global update phpdocumentor/phpdocumentor".to_string()),
            check_command: "phpdoc --version".to_string(),
            required_dependencies: vec!["php".to_string(), "composer".to_string()],
            install_method: InstallMethod::PackageManager("composer".to_string()),
            priority: 8,
        });

        // Ruby 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "yard".to_string(),
            language: "ruby".to_string(),
            install_command: "gem install yard".to_string(),
            upgrade_command: Some("gem update yard".to_string()),
            check_command: "yard --version".to_string(),
            required_dependencies: vec!["ruby".to_string(), "gem".to_string()],
            install_method: InstallMethod::PackageManager("gem".to_string()),
            priority: 9,
        });

        // Swift 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "swift-doc".to_string(),
            language: "swift".to_string(),
            install_command: "brew install swift-doc".to_string(),
            upgrade_command: Some("brew upgrade swift-doc".to_string()),
            check_command: "swift-doc --version".to_string(),
            required_dependencies: vec!["swift".to_string()],
            install_method: InstallMethod::SystemPackage("brew".to_string()),
            priority: 8,
        });

        // Dart 文档工具
        self.add_tool_definition(ToolInstallInfo {
            tool_name: "dartdoc".to_string(),
            language: "dart".to_string(),
            install_command: "dart pub global activate dartdoc".to_string(),
            upgrade_command: Some("dart pub global activate dartdoc".to_string()),
            check_command: "dartdoc --version".to_string(),
            required_dependencies: vec!["dart".to_string()],
            install_method: InstallMethod::PackageManager("dart".to_string()),
            priority: 9,
        });
    }

    fn add_tool_definition(&mut self, tool_info: ToolInstallInfo) {
        self.supported_tools.insert(tool_info.tool_name.clone(), tool_info);
    }

    fn get_system_install_command(&self, package: &str) -> String {
        match self.system_detector.os_type.as_str() {
            "windows" => {
                if self.system_detector.package_managers.contains(&"choco".to_string()) {
                    format!("choco install {}", package)
                } else if self.system_detector.package_managers.contains(&"winget".to_string()) {
                    format!("winget install {}", package)
                } else {
                    format!("# Please install {} manually", package)
                }
            }
            "macos" => {
                if self.system_detector.package_managers.contains(&"brew".to_string()) {
                    format!("brew install {}", package)
                } else {
                    format!("# Please install {} manually", package)
                }
            }
            "linux" => {
                if self.system_detector.package_managers.contains(&"apt".to_string()) {
                    format!("sudo apt install {}", package)
                } else if self.system_detector.package_managers.contains(&"yum".to_string()) {
                    format!("sudo yum install {}", package)
                } else if self.system_detector.package_managers.contains(&"dnf".to_string()) {
                    format!("sudo dnf install {}", package)
                } else {
                    format!("# Please install {} manually", package)
                }
            }
            _ => format!("# Please install {} manually", package),
        }
    }

    fn get_system_upgrade_command(&self, package: &str) -> String {
        match self.system_detector.os_type.as_str() {
            "windows" => {
                if self.system_detector.package_managers.contains(&"choco".to_string()) {
                    format!("choco upgrade {}", package)
                } else if self.system_detector.package_managers.contains(&"winget".to_string()) {
                    format!("winget upgrade {}", package)
                } else {
                    format!("# Please upgrade {} manually", package)
                }
            }
            "macos" => {
                if self.system_detector.package_managers.contains(&"brew".to_string()) {
                    format!("brew upgrade {}", package)
                } else {
                    format!("# Please upgrade {} manually", package)
                }
            }
            "linux" => {
                if self.system_detector.package_managers.contains(&"apt".to_string()) {
                    format!("sudo apt upgrade {}", package)
                } else if self.system_detector.package_managers.contains(&"yum".to_string()) {
                    format!("sudo yum update {}", package)
                } else if self.system_detector.package_managers.contains(&"dnf".to_string()) {
                    format!("sudo dnf upgrade {}", package)
                } else {
                    format!("# Please upgrade {} manually", package)
                }
            }
            _ => format!("# Please upgrade {} manually", package),
        }
    }

    /// 检测语言环境缺失的文档工具
    pub async fn detect_missing_tools(&self, detected_languages: &[String]) -> Result<HashMap<String, Vec<ToolInstallInfo>>> {
        info!("🔍 检测缺失的文档生成工具...");
        
        let mut missing_tools = HashMap::new();
        
        for language in detected_languages {
            let mut missing_for_language = Vec::new();
            
            // 获取该语言需要的所有文档工具
            let required_tools: Vec<&ToolInstallInfo> = self.supported_tools
                .values()
                .filter(|tool| tool.language == *language)
                .collect();
            
            for tool_info in required_tools {
                // 检查工具是否已安装
                if !self.is_tool_installed(&tool_info.check_command).await {
                    info!("❌ 缺失工具: {} ({})", tool_info.tool_name, language);
                    missing_for_language.push(tool_info.clone());
                } else {
                    debug!("✅ 已安装工具: {} ({})", tool_info.tool_name, language);
                }
            }
            
            if !missing_for_language.is_empty() {
                // 按优先级排序
                missing_for_language.sort_by(|a, b| b.priority.cmp(&a.priority));
                missing_tools.insert(language.clone(), missing_for_language);
            }
        }
        
        info!("📊 检测完成，发现 {} 种语言缺失工具", missing_tools.len());
        Ok(missing_tools)
    }

    /// 检查工具是否已安装
    pub async fn is_tool_installed(&self, check_command: &str) -> bool {
        let parts: Vec<&str> = check_command.split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// 自动安装缺失的工具
    pub async fn auto_install_tools(&self, missing_tools: &HashMap<String, Vec<ToolInstallInfo>>) -> Result<InstallationReport> {
        info!("🔧 开始自动安装缺失的工具...");
        
        let mut report = InstallationReport::new();
        
        // 检查Windows管理员权限
        #[cfg(target_os = "windows")]
        let is_admin = is_elevated();
        #[cfg(not(target_os = "windows"))]
        let is_admin = true; // 非Windows系统不需要特殊权限检查
        
        if !is_admin {
            #[cfg(target_os = "windows")]
            {
                warn!("⚠️ 非管理员模式运行，将显示手动安装命令");
                self.print_manual_install_commands(missing_tools);
                
                // 将所有工具标记为跳过
                for (_language, tools) in missing_tools {
                    for tool_info in tools {
                        report.skipped.push(format!("{} (需要管理员权限)", tool_info.tool_name));
                    }
                }
                
                return Ok(report);
            }
        }
        
        for (language, tools) in missing_tools {
            info!("📦 处理 {} 语言的工具...", language);
            
            for tool_info in tools {
                match self.config.strategy {
                    InstallStrategy::DetectOnly => {
                        info!("🔍 仅检测模式 - 跳过安装: {}", tool_info.tool_name);
                        continue;
                    }
                    InstallStrategy::Interactive => {
                        if !self.ask_user_permission(&tool_info.tool_name, &tool_info.install_command).await {
                            info!("⏭️ 用户跳过安装: {}", tool_info.tool_name);
                            report.skipped.push(tool_info.tool_name.clone());
                            continue;
                        }
                    }
                    _ => {}
                }

                // 检查依赖
                if !self.check_dependencies(&tool_info.required_dependencies).await {
                    warn!("❌ 依赖不满足，跳过安装: {}", tool_info.tool_name);
                    report.failed.push((tool_info.tool_name.clone(), "依赖不满足".to_string()));
                    continue;
                }

                // 执行安装
                match self.install_tool(tool_info).await {
                    Ok(_) => {
                        info!("✅ 安装成功: {}", tool_info.tool_name);
                        report.installed.push(tool_info.tool_name.clone());
                    }
                    Err(e) => {
                        error!("❌ 安装失败: {} - {}", tool_info.tool_name, e);
                        report.failed.push((tool_info.tool_name.clone(), e.to_string()));
                    }
                }
            }
        }
        
        info!("📊 安装完成 - 成功: {}, 失败: {}, 跳过: {}", 
              report.installed.len(), report.failed.len(), report.skipped.len());
        
        Ok(report)
    }

    /// 打印手动安装命令
    #[cfg(target_os = "windows")]
    fn print_manual_install_commands(&self, missing_tools: &HashMap<String, Vec<ToolInstallInfo>>) {
        info!("📋 请手动执行以下命令安装缺失的工具:");
        info!("═══════════════════════════════════════════════════");
        
        for (language, tools) in missing_tools {
            info!("🗣️ {} 语言工具:", language);
            
            for tool_info in tools {
                info!("   🔧 {}", tool_info.tool_name);
                // 根据工具名称生成简单描述
                let description = match tool_info.tool_name.as_str() {
                    "rustdoc" => "Rust官方文档生成工具",
                    "sphinx" => "Python文档生成器标准",
                    "jsdoc" => "JavaScript文档生成器",
                    "javadoc" => "Java官方文档工具",
                    "godoc" => "Go官方文档工具",
                    "docfx" => "微软文档生成工具",
                    "doxygen" => "C++文档生成标准",
                    _ => "文档生成工具",
                };
                info!("      📝 描述: {}", description);
                
                if tool_info.install_command.starts_with('#') {
                    info!("      ⚠️ {}", tool_info.install_command);
                } else {
                    // 检查是否需要管理员权限
                    let needs_admin = tool_info.install_command.contains("choco ") 
                        || tool_info.install_command.contains("winget ") 
                        || tool_info.install_command.contains("scoop ");
                    
                    if needs_admin {
                        info!("      🛡️ 需要管理员权限:");
                        info!("      📋 命令: {}", tool_info.install_command);
                        info!("      💡 请以管理员身份运行 PowerShell 或 CMD");
                    } else {
                        info!("      📋 命令: {}", tool_info.install_command);
                    }
                }
                
                if let Some(upgrade_cmd) = &tool_info.upgrade_command {
                    info!("      🔄 升级命令: {}", upgrade_cmd);
                }
                
                info!("      ✅ 验证命令: {}", tool_info.check_command);
                info!(""); // 空行分隔
            }
        }
        
        info!("═══════════════════════════════════════════════════");
        info!("💡 安装完成后，重新启动程序将自动检测已安装的工具");
        info!("🛡️ 如需自动安装，请以管理员身份重新运行程序");
    }

    /// 检查依赖是否满足
    async fn check_dependencies(&self, dependencies: &[String]) -> bool {
        for dep in dependencies {
            if dep == "built-in" {
                continue;
            }
            
            if !self.is_tool_installed(&format!("{} --version", dep)).await {
                warn!("⚠️ 缺失依赖: {}", dep);
                return false;
            }
        }
        true
    }

    /// 询问用户安装权限
    async fn ask_user_permission(&self, tool_name: &str, command: &str) -> bool {
        // 在实际实现中，这里应该是一个交互式提示
        // 现在先简单返回true，可以后续集成终端UI库
        info!("🤔 是否安装 {}? 命令: {}", tool_name, command);
        info!("⚡ 自动确认安装 (配置为交互模式时应提示用户)");
        true
    }

    /// 安装单个工具
    async fn install_tool(&self, tool_info: &ToolInstallInfo) -> Result<()> {
        info!("🔧 安装工具: {}", tool_info.tool_name);
        
        let command = &tool_info.install_command;
        
        // 跳过注释和手动安装提示
        if command.starts_with('#') {
            return Err(anyhow!("需要手动安装"));
        }

        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("无效的安装命令"));
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .output()
            .map_err(|e| anyhow!("执行安装命令失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("安装命令执行失败: {}", stderr));
        }

        // 验证安装是否成功
        if !self.is_tool_installed(&tool_info.check_command).await {
            return Err(anyhow!("安装后验证失败"));
        }

        Ok(())
    }

    /// 升级已安装的工具
    pub async fn upgrade_tools(&self, detected_tools: &HashMap<String, CliToolInfo>) -> Result<UpgradeReport> {
        info!("⬆️ 开始检查工具升级...");
        
        let mut report = UpgradeReport::new();
        
        for (tool_name, tool_info) in detected_tools {
            if !tool_info.available {
                continue;
            }

            if let Some(install_info) = self.supported_tools.get(tool_name) {
                if let Some(upgrade_command) = &install_info.upgrade_command {
                    if self.config.auto_upgrade {
                        match self.upgrade_tool(install_info, upgrade_command).await {
                            Ok(_) => {
                                info!("✅ 升级成功: {}", tool_name);
                                report.upgraded.push(tool_name.clone());
                            }
                            Err(e) => {
                                warn!("⚠️ 升级失败: {} - {}", tool_name, e);
                                report.failed.push((tool_name.clone(), e.to_string()));
                            }
                        }
                    } else {
                        info!("🔍 发现可升级工具: {} (自动升级已禁用)", tool_name);
                        report.available.push(tool_name.clone());
                    }
                }
            }
        }
        
        info!("📊 升级检查完成 - 已升级: {}, 失败: {}, 可升级: {}", 
              report.upgraded.len(), report.failed.len(), report.available.len());
        
        Ok(report)
    }

    /// 升级单个工具
    async fn upgrade_tool(&self, tool_info: &ToolInstallInfo, upgrade_command: &str) -> Result<()> {
        info!("⬆️ 升级工具: {}", tool_info.tool_name);
        
        let parts: Vec<&str> = upgrade_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("无效的升级命令"));
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .output()
            .map_err(|e| anyhow!("执行升级命令失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("升级命令执行失败: {}", stderr));
        }

        Ok(())
    }

    /// 获取支持的工具列表
    pub fn get_supported_tools(&self) -> &HashMap<String, ToolInstallInfo> {
        &self.supported_tools
    }

    /// 获取系统信息
    pub fn get_system_info(&self) -> &SystemDetector {
        &self.system_detector
    }
}

/// 安装报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationReport {
    pub installed: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub skipped: Vec<String>,
}

impl InstallationReport {
    fn new() -> Self {
        Self {
            installed: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn generate_summary(&self) -> String {
        format!(
            "📊 安装报告:\n  ✅ 成功: {}\n  ❌ 失败: {}\n  ⏭️ 跳过: {}",
            self.installed.len(),
            self.failed.len(),
            self.skipped.len()
        )
    }
}

/// 升级报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeReport {
    pub upgraded: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub available: Vec<String>,
}

impl UpgradeReport {
    fn new() -> Self {
        Self {
            upgraded: Vec::new(),
            failed: Vec::new(),
            available: Vec::new(),
        }
    }

    pub fn generate_summary(&self) -> String {
        format!(
            "📊 升级报告:\n  ⬆️ 已升级: {}\n  ❌ 失败: {}\n  🔍 可升级: {}",
            self.upgraded.len(),
            self.failed.len(),
            self.available.len()
        )
    }
} 