use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::process::Command as AsyncCommand;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    pub score: f32,
    pub project_files: Vec<String>,
    pub cli_tools: Vec<ToolInfo>,
    pub detected_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub available: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionReport {
    pub detected_languages: HashMap<String, LanguageInfo>,
    pub scan_duration_ms: u64,
    pub scan_paths: Vec<PathBuf>,
    pub total_files_scanned: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub path: PathBuf,
    pub languages: Vec<String>,
    pub primary_language: Option<String>,
    pub framework: Option<String>,
}

#[derive(Debug)]
pub struct EnvironmentDetector {
    scan_paths: Vec<PathBuf>,
    language_patterns: HashMap<String, Vec<String>>,
    cli_tools: HashMap<String, Vec<String>>,
}

impl EnvironmentDetector {
    pub fn new() -> Self {
        Self {
            scan_paths: vec![PathBuf::from(".")],
            language_patterns: Self::init_language_patterns(),
            cli_tools: Self::init_cli_tools(),
        }
    }

    fn init_language_patterns() -> HashMap<String, Vec<String>> {
        let mut patterns = HashMap::new();
        
        patterns.insert("rust".to_string(), vec![
            "Cargo.toml".to_string(),
            "Cargo.lock".to_string(),
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
        ]);
        
        patterns.insert("python".to_string(), vec![
            "requirements.txt".to_string(),
            "pyproject.toml".to_string(),
            "setup.py".to_string(),
            "poetry.lock".to_string(),
            "Pipfile".to_string(),
        ]);
        
        patterns.insert("javascript".to_string(), vec![
            "package.json".to_string(),
            "yarn.lock".to_string(),
            "pnpm-lock.yaml".to_string(),
            "package-lock.json".to_string(),
        ]);
        
        patterns.insert("java".to_string(), vec![
            "pom.xml".to_string(),
            "build.gradle".to_string(),
            "gradle.properties".to_string(),
            "settings.gradle".to_string(),
        ]);
        
        patterns.insert("go".to_string(), vec![
            "go.mod".to_string(),
            "go.sum".to_string(),
            "Gopkg.toml".to_string(),
        ]);
        
        // 新增语言支持
        patterns.insert("csharp".to_string(), vec![
            "*.csproj".to_string(),
            "*.sln".to_string(),
            "global.json".to_string(),
            "Directory.Build.props".to_string(),
        ]);
        
        patterns.insert("cpp".to_string(), vec![
            "CMakeLists.txt".to_string(),
            "conanfile.txt".to_string(),
            "vcpkg.json".to_string(),
            "Makefile".to_string(),
        ]);
        
        patterns.insert("php".to_string(), vec![
            "composer.json".to_string(),
            "composer.lock".to_string(),
            "artisan".to_string(), // Laravel
        ]);
        
        patterns.insert("ruby".to_string(), vec![
            "Gemfile".to_string(),
            "Gemfile.lock".to_string(),
            "*.gemspec".to_string(),
            "Rakefile".to_string(),
        ]);
        
        patterns.insert("swift".to_string(), vec![
            "Package.swift".to_string(),
            "*.xcodeproj".to_string(),
            "*.xcworkspace".to_string(),
        ]);
        
        patterns.insert("dart".to_string(), vec![
            "pubspec.yaml".to_string(),
            "pubspec.lock".to_string(),
            "analysis_options.yaml".to_string(),
        ]);
        
        patterns
    }

    fn init_cli_tools() -> HashMap<String, Vec<String>> {
        let mut tools = HashMap::new();
        
        tools.insert("rust".to_string(), vec!["cargo".to_string(), "rustc".to_string()]);
        tools.insert("python".to_string(), vec!["python".to_string(), "pip".to_string(), "poetry".to_string()]);
        tools.insert("javascript".to_string(), vec!["node".to_string(), "npm".to_string(), "yarn".to_string()]);
        tools.insert("java".to_string(), vec!["java".to_string(), "javac".to_string(), "mvn".to_string(), "gradle".to_string()]);
        tools.insert("go".to_string(), vec!["go".to_string()]);
        
        // 新增语言工具
        tools.insert("csharp".to_string(), vec!["dotnet".to_string(), "nuget".to_string()]);
        tools.insert("cpp".to_string(), vec!["gcc".to_string(), "clang".to_string(), "cmake".to_string(), "vcpkg".to_string()]);
        tools.insert("php".to_string(), vec!["php".to_string(), "composer".to_string()]);
        tools.insert("ruby".to_string(), vec!["ruby".to_string(), "gem".to_string(), "bundle".to_string()]);
        tools.insert("swift".to_string(), vec!["swift".to_string(), "swiftpm".to_string()]);
        tools.insert("dart".to_string(), vec!["dart".to_string(), "flutter".to_string()]);
        
        tools
    }

    pub async fn scan_environment(&mut self) -> Result<DetectionReport> {
        let start_time = std::time::Instant::now();
        let mut detected_languages = HashMap::new();
        let mut total_files_scanned = 0;

        info!("🔍 开始环境检测...");

        for scan_path in &self.scan_paths.clone() {
            let file_detections = self.scan_project_files(scan_path).await?;
            total_files_scanned += file_detections.len();

            for (language, files) in file_detections {
                let cli_tools = self.check_cli_tools(&language).await;
                let detected_features = self.detect_language_features(&language, &files, scan_path).await;
                let score = self.calculate_language_score(&language, &files, &cli_tools);
                
                let lang_info = LanguageInfo {
                    name: language.clone(),
                    score,
                    project_files: files,
                    cli_tools,
                    detected_features,
                };

                detected_languages.insert(language, lang_info);
            }
        }

        let duration = start_time.elapsed();
        info!("✅ 环境检测完成，耗时: {:?}", duration);
        info!("📊 检测到 {} 种语言", detected_languages.len());

        Ok(DetectionReport {
            detected_languages,
            scan_duration_ms: duration.as_millis() as u64,
            scan_paths: self.scan_paths.clone(),
            total_files_scanned,
        })
    }

    async fn scan_project_files(&self, path: &Path) -> Result<HashMap<String, Vec<String>>> {
        let mut detections = HashMap::new();

        for (language, patterns) in &self.language_patterns {
            let mut found_files = Vec::new();

            for pattern in patterns {
                // 简单的文件存在检查（可以扩展为glob模式匹配）
                let file_path = path.join(pattern);
                if file_path.exists() {
                    found_files.push(pattern.clone());
                    debug!("✅ 发现 {} 文件: {}", language, pattern);
                }
            }

            if !found_files.is_empty() {
                detections.insert(language.clone(), found_files);
            }
        }

        Ok(detections)
    }

    async fn check_cli_tools(&self, language: &str) -> Vec<ToolInfo> {
        let mut tools = Vec::new();

        if let Some(tool_names) = self.cli_tools.get(language) {
            for tool_name in tool_names {
                let tool_info = self.check_single_tool(tool_name).await;
                tools.push(tool_info);
            }
        }

        tools
    }

    async fn check_single_tool(&self, tool_name: &str) -> ToolInfo {
        // 设置较短的超时时间，避免卡住
        let timeout = std::time::Duration::from_secs(3);
        
        match tokio::time::timeout(timeout, AsyncCommand::new(tool_name)
            .arg("--version")
            .output()).await
        {
            Ok(Ok(output)) if output.status.success() => {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = self.extract_version(&version_output);
                let tool_path = self.get_tool_path(tool_name).await;
                
                debug!("✅ 工具可用: {} {} (路径: {:?})", tool_name, version.as_deref().unwrap_or("unknown"), tool_path);
                
                ToolInfo {
                    name: tool_name.to_string(),
                    version,
                    available: true,
                    path: tool_path,
                }
            }
            _ => {
                debug!("❌ 工具不可用或超时: {}", tool_name);
                ToolInfo {
                    name: tool_name.to_string(),
                    version: None,
                    available: false,
                    path: None,
                }
            }
        }
    }

    /// 获取工具的完整路径
    async fn get_tool_path(&self, tool_name: &str) -> Option<String> {
        let timeout = std::time::Duration::from_secs(2);
        
        // 在Windows上使用where，在Unix系统上使用which
        let command = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };
        
        match tokio::time::timeout(timeout, AsyncCommand::new(command)
            .arg(tool_name)
            .output()).await
        {
            Ok(Ok(output)) if output.status.success() => {
                let path_output = String::from_utf8_lossy(&output.stdout);
                let path = path_output.trim();
                if !path.is_empty() {
                    Some(path.to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn extract_version(&self, output: &str) -> Option<String> {
        // 简单的版本提取逻辑，可以根据需要改进
        for line in output.lines() {
            if let Some(version) = line.split_whitespace().nth(1) {
                if version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    return Some(version.to_string());
                }
            }
        }
        None
    }

    fn calculate_language_score(&self, language: &str, files: &[String], tools: &[ToolInfo]) -> f32 {
        let mut score = 0.0;

        // 项目文件权重 (60%)
        let file_weight = 0.6 / self.language_patterns.get(language).map(|p| p.len() as f32).unwrap_or(1.0);
        score += files.len() as f32 * file_weight;

        // CLI工具可用性权重 (40%)
        let available_tools = tools.iter().filter(|t| t.available).count() as f32;
        let total_tools = tools.len() as f32;
        if total_tools > 0.0 {
            score += (available_tools / total_tools) * 0.4;
        }

        score.min(1.0) // 确保分数不超过1.0
    }

    pub fn add_scan_path(&mut self, path: PathBuf) {
        self.scan_paths.push(path);
    }

    pub fn get_detected_languages(&self) -> Vec<String> {
        self.language_patterns.keys().cloned().collect()
    }

    /// 检测语言特定的特征
    async fn detect_language_features(&self, language: &str, files: &[String], scan_path: &Path) -> Vec<String> {
        let mut features = Vec::new();
        
        match language {
            "rust" => {
                features.extend(self.detect_rust_features(files, scan_path).await);
            }
            "python" => {
                features.extend(self.detect_python_features(files, scan_path).await);
            }
            "javascript" => {
                features.extend(self.detect_javascript_features(files, scan_path).await);
            }
            "java" => {
                features.extend(self.detect_java_features(files, scan_path).await);
            }
            "go" => {
                features.extend(self.detect_go_features(files, scan_path).await);
            }
            _ => {
                // 通用特征检测
                features.extend(self.detect_generic_features(files, scan_path).await);
            }
        }
        
        features
    }

    async fn detect_rust_features(&self, _files: &[String], scan_path: &Path) -> Vec<String> {
        let mut features = Vec::new();
        
        // 检查Cargo.toml中的特征
        let cargo_toml_path = scan_path.join("Cargo.toml");
        if let Ok(content) = tokio::fs::read_to_string(&cargo_toml_path).await {
            if content.contains("[workspace]") {
                features.push("workspace".to_string());
            }
            if content.contains("serde") {
                features.push("serialization".to_string());
            }
            if content.contains("tokio") || content.contains("async-std") {
                features.push("async".to_string());
            }
            if content.contains("wasm") {
                features.push("webassembly".to_string());
            }
            if content.contains("no_std") {
                features.push("no_std".to_string());
            }
        }
        
        features
    }

    async fn detect_python_features(&self, _files: &[String], scan_path: &Path) -> Vec<String> {
        let mut features = Vec::new();
        
        // 检查requirements.txt
        let requirements_path = scan_path.join("requirements.txt");
        if let Ok(content) = tokio::fs::read_to_string(&requirements_path).await {
            if content.contains("django") {
                features.push("django".to_string());
            }
            if content.contains("flask") {
                features.push("flask".to_string());
            }
            if content.contains("fastapi") {
                features.push("fastapi".to_string());
            }
            if content.contains("numpy") || content.contains("pandas") {
                features.push("data_science".to_string());
            }
            if content.contains("tensorflow") || content.contains("pytorch") {
                features.push("machine_learning".to_string());
            }
        }
        
        // 检查pyproject.toml
        let pyproject_path = scan_path.join("pyproject.toml");
        if let Ok(content) = tokio::fs::read_to_string(&pyproject_path).await {
            if content.contains("poetry") {
                features.push("poetry".to_string());
            }
            if content.contains("pep621") {
                features.push("modern_packaging".to_string());
            }
        }
        
        features
    }

    async fn detect_javascript_features(&self, _files: &[String], scan_path: &Path) -> Vec<String> {
        let mut features = Vec::new();
        
        let package_json_path = scan_path.join("package.json");
        if let Ok(content) = tokio::fs::read_to_string(&package_json_path).await {
            if content.contains("react") {
                features.push("react".to_string());
            }
            if content.contains("vue") {
                features.push("vue".to_string());
            }
            if content.contains("angular") {
                features.push("angular".to_string());
            }
            if content.contains("express") {
                features.push("express".to_string());
            }
            if content.contains("typescript") {
                features.push("typescript".to_string());
            }
            if content.contains("webpack") {
                features.push("webpack".to_string());
            }
            if content.contains("vite") {
                features.push("vite".to_string());
            }
        }
        
        features
    }

    async fn detect_java_features(&self, _files: &[String], scan_path: &Path) -> Vec<String> {
        let mut features = Vec::new();
        
        // 检查pom.xml (Maven)
        let pom_path = scan_path.join("pom.xml");
        if let Ok(content) = tokio::fs::read_to_string(&pom_path).await {
            if content.contains("spring") {
                features.push("spring".to_string());
            }
            if content.contains("junit") {
                features.push("junit".to_string());
            }
            if content.contains("jackson") {
                features.push("json_processing".to_string());
            }
        }
        
        // 检查build.gradle (Gradle)
        let gradle_path = scan_path.join("build.gradle");
        if let Ok(content) = tokio::fs::read_to_string(&gradle_path).await {
            if content.contains("android") {
                features.push("android".to_string());
            }
            if content.contains("kotlin") {
                features.push("kotlin".to_string());
            }
        }
        
        features
    }

    async fn detect_go_features(&self, _files: &[String], scan_path: &Path) -> Vec<String> {
        let mut features = Vec::new();
        
        let go_mod_path = scan_path.join("go.mod");
        if let Ok(content) = tokio::fs::read_to_string(&go_mod_path).await {
            if content.contains("gin") {
                features.push("gin".to_string());
            }
            if content.contains("echo") {
                features.push("echo".to_string());
            }
            if content.contains("gorm") {
                features.push("gorm".to_string());
            }
            if content.contains("kubernetes") {
                features.push("kubernetes".to_string());
            }
        }
        
        features
    }

    async fn detect_generic_features(&self, _files: &[String], scan_path: &Path) -> Vec<String> {
        let mut features = Vec::new();
        
        // 检查通用配置文件
        if scan_path.join("Dockerfile").exists() {
            features.push("docker".to_string());
        }
        if scan_path.join("docker-compose.yml").exists() || scan_path.join("docker-compose.yaml").exists() {
            features.push("docker_compose".to_string());
        }
        if scan_path.join(".github").exists() {
            features.push("github_actions".to_string());
        }
        if scan_path.join(".gitlab-ci.yml").exists() {
            features.push("gitlab_ci".to_string());
        }
        
        features
    }
}

impl Default for EnvironmentDetector {
    fn default() -> Self {
        Self::new()
    }
} 