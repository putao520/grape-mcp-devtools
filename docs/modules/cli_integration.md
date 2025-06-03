# CLI集成模块设计文档

## 模块概览

CLI集成模块是Grape MCP DevTools的重要组成部分，负责与本地安装的各种开发命令行工具进行交互。它为其他模块（如 `EnvironmentDetector`、各种 `SpecificDocTool`、`ToolInstaller` 等）提供统一的、安全可靠的方式来检测本地CLI工具的可用性、获取版本信息、执行命令以及（可选地）触发工具的自动安装。它是连接 `grape-mcp-devtools` 与用户本地开发环境的桥梁。

### 模块基本信息
- **模块路径**: `src/cli/` (主要包括 `executor.rs`, `detector.rs`, `tool_installer.rs`, `registry.rs`)
- **主要作用**: 检测本地CLI工具、安全执行CLI命令、管理工具安装指令、提供CLI元数据。
- **核心特性**: 跨平台兼容命令执行、异步操作、标准化错误处理、可配置的工具元数据。
- **支持工具**: 设计上应能支持任何可以通过命令行调用的工具。元数据 (`CLIToolRegistry`) 可以预定义常用开发工具（如: cargo, npm, pip, mvn, go, flutter, dart, rustup, poetry, conda, tsc, deno, git, docker, etc.）。

## 架构设计

### 1. 模块结构

```
src/cli/
├── mod.rs                # 模块导出和配置
├── executor.rs           # 命令执行器 (CommandExecutor) - 核心组件
├── detector.rs           # CLI工具检测器 (CliDetector) - 使用 CommandExecutor
├── tool_installer.rs     # CLI工具安装器 (ToolInstaller) - 使用 CommandExecutor
├── registry.rs           # CLI工具元数据注册表 (CLIToolRegistry) - 提供数据给 Detector 和 Installer
└── config/cli_metadata.toml # (Conceptual) 存放 CLIToolRegistry 的数据源
```

### 2. 核心组件架构

```mermaid
digraph CLI_Integration_Module {
    rankdir=TB;
    node [shape=box, style=rounded];

    subgraph "CLI Integration Module (`src/cli/`)" {
        CommandExecutor [label="CommandExecutor\n(executor.rs)\n- Executes any command securely"];
        CliDetector [label="CliDetector\n(detector.rs)\n- Detects tool presence/version"];
        ToolInstaller [label="ToolInstaller\n(tool_installer.rs)\n- Manages tool installation"];
        CLIToolRegistry [label="CLIToolRegistry\n(registry.rs)\n- Stores CLI metadata"];
        MetadataFile [label="cli_metadata.toml\n(Config File)"];
    }

    OtherModules [label="Other Modules\n(e.g., EnvironmentDetector,\nSpecificDocTools)"];
    OperatingSystem [label="Operating System Shell/Process"];

    OtherModules -> CliDetector [label="requests tool detection"];
    OtherModules -> ToolInstaller [label="requests tool installation"];
    OtherModules -> CommandExecutor [label="(less common direct use)\nrequests command execution"];
    
    CliDetector -> CommandExecutor [label="uses to run version/check commands"];
    CliDetector -> CLIToolRegistry [label="gets version_arg, version_regex"];
    
    ToolInstaller -> CommandExecutor [label="uses to run install commands"];
    ToolInstaller -> CLIToolRegistry [label="gets install_script_templates"];

    CommandExecutor -> OperatingSystem [label="spawns process"];
    CLIToolRegistry -> MetadataFile [label="loads from"];
}
```

### 3. 主要组件说明

#### 3.1 CommandExecutor (`executor.rs`)
**功能**: 封装异步、安全地执行外部命令行命令的核心逻辑。这是模块内所有与外部进程交互的唯一入口点。
- 使用 `tokio::process::Command` 构建和执行命令。
- 支持设置工作目录 (`cwd`)、环境变量 (`env_vars`)。
- 实现可配置的执行超时。
- 捕获子进程的 `stdout`、`stderr` 和退出状态 (`ExitStatus`)。
- 确保参数被正确传递，防止命令注入。

**关键接口**:
```rust
// pub struct CommandExecutor;

// #[derive(Debug, Clone)]
// pub struct CommandOutput {
//     pub stdout: String,
//     pub stderr: String,
//     pub status: std::process::ExitStatus, // Contains exit code and signal info
//     pub duration: Duration,
// }

// impl CommandExecutor {
    // pub async fn execute(
    //     command_name: &str, // e.g., "cargo", "npm"
    //     args: &[&str],      // e.g., ["--version"]
    //     cwd: Option<PathBuf>,
    //     env_vars: Option<&HashMap<String, String>>,
    //     timeout: Option<Duration>,
    // ) -> Result<CommandOutput, std::io::Error>; // std::io::Error for spawn failures, timeout errors etc.
// }
```

#### 3.2 CLIDetector (`detector.rs`)
**功能**: 检测本地环境中是否安装了特定的CLI工具及其版本。
- **检测方法**: 
    1. (可选) 从 `CLIToolRegistry` 获取目标工具的 `version_arg` (如 `--version`, `-v`) 和 `version_regex` (用于从输出中提取版本号)。
    2. 调用 `CommandExecutor::execute(tool_name, &[version_arg])`。
    3. 如果命令成功执行 (exit_code 0)，则解析 `stdout` (使用 `version_regex` 如果提供) 来提取版本字符串。
    4. 如果工具的元数据包含 `subcommands_for_check`，也会执行这些子命令来进一步确认功能。
- **缓存机制**: 内部缓存 `CliToolInfo` 结果，键为工具名 (e.g., "cargo")。缓存具有可配置的TTL。

**关键接口与数据结构**:
```rust
// pub struct CliDetector {
//     command_executor: Arc<CommandExecutor>,
//     cli_registry: Arc<CLIToolRegistry>,
//     cache: Arc<MokaCache<String, Option<CliToolInfo>>>, // Using moka for caching
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<PathBuf>, // Full path to the executable, if found
    pub available: bool,
    pub checked_at: DateTime<Utc>,
    pub error_message: Option<String>, // If detection failed
}

// impl CliDetector {
    // pub fn new(executor: Arc<CommandExecutor>, registry: Arc<CLIToolRegistry>, cache_config: CacheConfig) -> Self;
    // pub async fn detect(&self, tool_name: &str) -> Option<CliToolInfo>;
    // pub async fn detect_many(&self, tool_names: &[&str]) -> HashMap<String, Option<CliToolInfo>>;
// }
```

#### 3.3 ToolInstaller (`tool_installer.rs`)
**功能**: (可选功能，由全局配置启用) 尝试自动安装或提供安装指定CLI工具的指令。
- **安装逻辑**: 
    1. 根据当前操作系统 (Windows, macOS, Linux) 和目标工具名，从 `CLIToolRegistry` (或内部预定义映射) 查找合适的安装命令/脚本模板。
    2. 替换模板中的占位符 (如版本号)。
    3. 调用 `CommandExecutor::execute()` 执行安装命令。
    4. (可选) 执行后调用 `CliDetector::detect()` 验证安装是否成功。
- 支持不同平台的包管理器 (e.g., `choco`, `brew`, `apt`, `yum`, `npm`, `pip`, `sdkman`, `rustup`).

**关键接口与数据结构**:
```rust
// pub struct ToolInstaller {
//     command_executor: Arc<CommandExecutor>,
//     cli_registry: Arc<CLIToolRegistry>,
//     config: ToolInstallerConfig, // Contains auto_install flag, preferred methods
// }

// #[derive(Debug, Clone, Deserialize)]
// pub struct ToolInstallerConfig {
//     pub auto_install_globally: bool,       // Global flag to enable/disable installer
//     pub default_install_timeout: Duration,
//     // pub per_tool_install_prefs: HashMap<String, PerToolInstallPref>,
// }

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("Installation for '{tool_name}' is not supported on this platform or no install script found.")]
    NotSupported { tool_name: String },
    #[error("Installation command for '{tool_name}' failed. Stderr: {stderr}")]
    ExecutionFailed { tool_name: String, stderr: String, stdout: String, status: ExitStatus },
    #[error("Installation for '{tool_name}' timed out after {duration:?}.")]
    Timeout { tool_name: String, duration: Duration },
    #[error("Verification failed after attempting to install '{tool_name}'. Detection error: {detect_error}")]
    VerificationFailed { tool_name: String, detect_error: String },
    #[error("User cancelled installation for '{tool_name}'.")] // If interactive prompt is used
    UserCancelled { tool_name: String },
}

// impl ToolInstaller {
    // pub fn new(executor: Arc<CommandExecutor>, registry: Arc<CLIToolRegistry>, config: ToolInstallerConfig) -> Self;
    // pub async fn ensure_tool_installed(&self, tool_name: &str, required_version: Option<&str>) -> Result<CliToolInfo, InstallError>;
    // pub async fn get_install_instructions(&self, tool_name: &str) -> Option<String>; // Returns textual instructions
// }
```

#### 3.4 CLIToolRegistry (`registry.rs`)
**功能**: 存储已知CLI工具的元数据。这是一个配置驱动的静态数据存储，帮助其他组件（`CliDetector`, `ToolInstaller`）更智能地与CLI工具交互。
- 从一个或多个配置文件 (e.g., `configs/cli_metadata.toml`) 加载数据。
- 数据应包含工具名、获取版本号的参数、解析版本号的正则表达式、各平台安装命令模板等。

**数据结构 (`CliToolMetadata`)**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliToolMetadata {
    pub name: String, // Canonical name, e.g., "node"
    pub version_arg: String, // e.g., "--version"
    pub version_regex: Option<String>, // Regex to extract version from stdout, e.g., "v([0-9]+\.[0-9]+\.[0-9]+)"
    pub help_arg: Option<String>, // e.g., "--help"
    pub install_commands: Option<HashMap<String, String>>, // Key: OS (e.g., "windows", "linux_debian", "macos"), Value: command template
    pub subcommands_for_check: Option<HashMap<String, String>>, // e.g., {"list_components": "rustup component list"}
    pub detection_priority: Option<i32>, // Used if multiple tools provide similar functionality
}

// pub struct CLIToolRegistry {
//     metadata: HashMap<String, CliToolMetadata>,
// }

// impl CLIToolRegistry {
    // pub fn load_from_files(paths: &[PathBuf]) -> Result<Self, std::io::Error>;
    // pub fn get_metadata(&self, tool_name: &str) -> Option<&CliToolMetadata>;
// }
```

### 4. 核心流程

#### 4.1 工具检测流程 (`CliDetector::detect`)
1.  **Input**: `tool_name: &str`.
2.  Check internal cache for `tool_name`. If found and not expired, return cached `CliToolInfo`.
3.  Fetch `CliToolMetadata` for `tool_name` from `CLIToolRegistry`.
    - If no metadata, attempt a generic `tool_name --version` or similar common patterns.
4.  Construct command: `tool_name` and `metadata.version_arg`.
5.  Call `CommandExecutor::execute(tool_name, &[version_arg], ...)`.
6.  **On Success (`Ok(CommandOutput)`)**: 
    - If `output.status` is success (e.g., exit code 0):
        - Parse `output.stdout` using `metadata.version_regex` to extract version string.
        - Attempt to find full path of the executable (e.g., using `which` crate or OS-specific commands if not directly given by `Command`).
        - Create `CliToolInfo { available: true, version, path, ... }`.
    - Else (`output.status` indicates failure):
        - Create `CliToolInfo { available: false, error_message: Some(output.stderr), ... }`.
7.  **On Execution Failure (`Err(io_error)`)**:
    - Create `CliToolInfo { available: false, error_message: Some(io_error.to_string()), ... }`.
8.  Store the new `CliToolInfo` in cache.
9.  Return the `CliToolInfo`.

#### 4.2 命令执行流程 (`CommandExecutor::execute`)
1.  **Input**: `command_name`, `args`, `cwd`, `env_vars`, `timeout`.
2.  Log the command to be executed (with redaction for sensitive args if necessary).
3.  Create `std::process::Command` (or `tokio::process::Command`).
    - Set `command_name` as the program.
    - Add `args`.
    - If `cwd` is Some, set current directory.
    - If `env_vars` is Some, set/override environment variables.
    - Configure `stdin` (usually `Stdio::null()`), `stdout` (`Stdio::piped()`), `stderr` (`Stdio::piped()`).
4.  Spawn the child process.
    - If spawn fails (e.g., command not found), return `Err(std::io::Error)`.
5.  If `timeout` is Some, wrap the process waiting logic in `tokio::time::timeout()`.
6.  Asynchronously wait for the child process to complete (`child.wait_with_output().await`).
    - If timeout occurs, kill the process and return a timeout error (custom `std::io::Error` or specific enum).
7.  Collect `stdout`, `stderr` (as Strings, ensuring UTF-8 validity), and `ExitStatus`.
8.  Record execution `duration`.
9.  Return `Ok(CommandOutput { stdout, stderr, status, duration })`.

#### 4.3 工具安装流程 (`ToolInstaller::ensure_tool_installed` - simplified)
1.  **Input**: `tool_name`, `required_version`.
2.  Call `CliDetector::detect(tool_name)`.
    - If tool is available and version matches (if `required_version` is Some), return `Ok(detected_info)`.
3.  If auto-install is disabled in config, return `Err(InstallError::NotSupported)` or a specific error indicating auto-install is off.
4.  Fetch `CliToolMetadata` for `tool_name` from `CLIToolRegistry`.
    - Get `install_commands` for the current OS.
    - If no install command found, return `Err(InstallError::NotSupported)`.
5.  (Optional) If multiple install commands, select based on preference or try sequentially.
6.  (Optional) Prompt user for confirmation if configured.
7.  Execute the install command using `CommandExecutor::execute()`.
    - If execution fails (error or non-zero exit code), return `Err(InstallError::ExecutionFailed)` with details from `CommandOutput`.
8.  After successful command execution, call `CliDetector::detect(tool_name)` again to verify.
    - If verification fails or version doesn't match, return `Err(InstallError::VerificationFailed)`.
9.  Return `Ok(verified_info)`.

### 5. 跨平台兼容性
- **`CommandExecutor`**: 
    - Must handle differences in how commands and arguments are passed on Windows (e.g., `cmd /C ...`) vs. Unix-like systems. Generally, directly executing binaries is preferred over shell passthrough.
    - Path handling: Use `PathBuf` and Rust's path manipulation for OS-agnostic path construction.
- **`CliDetector`**: Relies on `CLIToolRegistry` to store platform-specific version arguments if they differ significantly (though most modern tools use `--version`).
- **`ToolInstaller`**: This is the most platform-sensitive component. `install_commands` in `CLIToolRegistry` must be OS-specific. The installer needs robust OS detection (`std::env::consts::OS`).
- **Shell differences**: Avoid reliance on specific shell features or built-in commands unless explicitly handled for each platform.

### 6. 错误处理和日志
- **`CommandExecutor`**: Returns `std::io::Error` for spawn/timeout issues. `CommandOutput.status` (an `ExitStatus`) provides rich information about command success/failure. `CommandOutput.stderr` is critical for diagnostics.
- **`CliDetector`**: `CliToolInfo.available` and `CliToolInfo.error_message` convey detection status.
- **`ToolInstaller`**: `InstallError` enum captures various failure modes, including underlying `CommandOutput` details when an install command fails.
- **Logging**: Use `tracing` extensively. Log executed commands (potentially with argument redaction), exit codes, stdout/stderr for failed commands, and outcomes of detection/installation attempts.

### 7. 安全考虑
- **Command Injection**: `CommandExecutor` is the primary defense. It MUST ensure that `command_name` is treated as a literal command and `args` are passed as distinct arguments to the process, never by concatenating them into a shell string.
- **Path Traversal/Arbitrary Execution**: If `command_name` or parts of `args` can be influenced by external (MCP client) input, they must be rigorously validated against an allow-list or known tool names from `CLIToolRegistry`. Avoid executing arbitrary paths provided by clients.
- **Resource Consumption**: `CommandExecutor`'s timeout feature is crucial. Long-running or stuck CLI tools should not hang the `grape-mcp-devtools` process indefinitely.
- **Permissions**: `ToolInstaller` executing system-wide install commands (e.g., `sudo apt install`) is risky and generally discouraged for a tool like this. Prefer user-level installations (e.g., `pip install --user`, `npm install -g` if user has rights, `rustup component add`). If admin rights are unavoidable for a core tool, the user must be explicitly prompted or documentation must guide manual installation.

### 8. 配置选项
- **`CLIToolRegistry` data source**: Path to `cli_metadata.toml` (or multiple paths).
- **`CommandExecutor` defaults**: Default timeout for command execution (can be overridden per call).
- **`CliDetector` cache**: TTL for cached `CliToolInfo`.
- **`ToolInstaller` behavior**: Global `auto_install_missing_tools` flag, preferred installation methods for certain tools (e.g., use `conda` before `pip` for Python packages if both are available).

## 模块接口暴露 (`src/cli/mod.rs`)

```rust
pub mod detector;
pub mod executor;
pub mod registry;
pub mod tool_installer;

pub use detector::{CliDetector, CliToolInfo};
pub use executor::{CommandExecutor, CommandOutput};
pub use registry::{CLIToolRegistry, CliToolMetadata};
pub use tool_installer::{InstallError, ToolInstaller, ToolInstallerConfig}; // ToolInstallerConfig might be loaded from main app config

// Example of a higher-level utility function that might be useful
// pub async fn check_tools_availability(
//     detector: &CliDetector,
//     tool_names: &[&str],
// ) -> HashMap<String, bool> {
//     let results = detector.detect_many(tool_names).await;
//     results.into_iter().map(|(name, info_opt)| (name, info_opt.map_or(false, |info| info.available))).collect()
// }
```

## 测试策略

- **单元测试**:
    - `CommandExecutor`: 
        - Mock `tokio::process::Command` (can be tricky, might need a helper binary that echoes args/env).
        - Test execution of known system commands (e.g., `echo` on Linux/macOS, `cmd /C echo` on Windows).
        - Test argument passing, `cwd` changes, environment variable setting.
        - Test stdout/stderr capture and exit code reporting.
        - Test timeout functionality.
    - `CliDetector`:
        - Mock `CommandExecutor` to return predefined `CommandOutput` for version commands.
        - Mock `CLIToolRegistry` to provide metadata.
        - Test version string parsing (with and without `version_regex`).
        - Test cache logic (hit, miss, expiry).
    - `ToolInstaller`:
        - Mock `CommandExecutor` and `CliDetector`.
        - Mock `CLIToolRegistry` to provide OS-specific install commands.
        - Test correct install command selection for different OSes.
        - Test verification logic after simulated successful/failed installations.
    - `CLIToolRegistry`: Test loading metadata from TOML files, handling malformed files, and querying.
- **集成测试**:
    - Requires a controlled environment where actual CLI tools can be installed/uninstalled or are known to be present/absent.
    - Test `CliDetector::detect()` against real CLI tools (e.g., `cargo --version`, `node -v`).
    - (Very carefully, in isolated environments like Docker containers) Test `ToolInstaller` end-to-end for a small, safe, easily installable/removable CLI tool. This is often complex to automate reliably.
    - Test cross-platform behavior by running tests in CI on Windows, macOS, and Linux build agents.

## 总结

CLI集成模块是 `grape-mcp-devtools` 与本地系统环境进行交互的基石。通过 `CommandExecutor` 提供了一个统一、安全的命令执行接口，`CliDetector` 在此基础上实现了可靠的工具检测，而 `CLIToolRegistry` 则通过元数据增强了这种交互的智能性。可选的 `ToolInstaller` 进一步提升了用户体验，但需谨慎处理权限和平台差异。模块的健壮性、安全性和跨平台兼容性对于整个应用的稳定运行至关重要。 