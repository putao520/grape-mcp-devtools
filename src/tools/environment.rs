use crate::tools::base::{MCPTool, Schema, SchemaObject};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use toml;
use async_trait::async_trait;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDetectionTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    pub weight: f64,
    pub file_count: usize,
    pub config_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectType {
    pub category: String,
    pub subcategory: Option<String>,
    pub frameworks: Vec<String>,
    pub build_system: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub latest: Option<String>,
    pub status: String, // "current", "outdated", "vulnerable"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDependencies {
    pub total_count: usize,
    pub dev_dependencies: usize,
    pub outdated: usize,
    pub vulnerable: usize,
    pub packages: Vec<DependencyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub primary_language: Option<String>,
    pub languages: Vec<LanguageInfo>,
    pub project_type: Option<ProjectType>,
    pub dependencies: HashMap<String, LanguageDependencies>,
    pub recommendations: Vec<String>,
}

#[async_trait]
impl MCPTool for EnvironmentDetectionTool {
    fn name(&self) -> &str {
        "detect_environment"
    }

    fn description(&self) -> &str {
        "检测当前工作目录的开发环境信息，包括编程语言、项目类型、依赖等"
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            let mut properties = HashMap::new();
            
            properties.insert("path".to_string(), Schema::String(crate::tools::base::SchemaString {
                description: Some("要检测的目录路径，默认为当前目录".to_string()),
                enum_values: None,
            }));
            
            properties.insert("depth".to_string(), Schema::Integer(crate::tools::base::SchemaInteger {
                description: Some("扫描深度，默认为3".to_string()),
                minimum: Some(1),
                maximum: Some(10),
            }));
            
            properties.insert("include_dependencies".to_string(), Schema::Boolean(crate::tools::base::SchemaBoolean {
                description: Some("是否包含依赖分析，默认为true".to_string()),
            }));
            
            properties.insert("include_toolchain".to_string(), Schema::Boolean(crate::tools::base::SchemaBoolean {
                description: Some("是否检测工具链状态，默认为false".to_string()),
            }));

            Schema::Object(SchemaObject {
                required: vec![],
                properties,
                description: Some("环境检测参数".to_string()),
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        
        let depth = params.get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;
        
        let include_dependencies = params.get("include_dependencies")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        let _include_toolchain = params.get("include_toolchain")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.detect_environment(path, depth, include_dependencies).await {
            Ok(env_info) => Ok(json!({
                "environment": env_info
            })),
            Err(e) => Err(anyhow::anyhow!("环境检测失败: {}", e)),
        }
    }
}

impl EnvironmentDetectionTool {
    pub fn new() -> Self {
        Self
    }

    async fn detect_environment(&self, path: &str, depth: usize, include_dependencies: bool) -> Result<EnvironmentInfo> {
        let root_path = PathBuf::from(path);
        
        // 扫描文件系统
        let file_info = self.scan_files(&root_path, depth)?;
        
        // 检测编程语言
        let languages = self.detect_languages(&file_info, &root_path)?;
        
        // 确定主要语言
        let primary_language = languages.first().map(|l| l.name.clone());
        
        // 分析项目类型
        let project_type = self.analyze_project_type(&languages, &root_path)?;
        
        // 分析依赖（如果需要）
        let dependencies = if include_dependencies {
            self.analyze_dependencies(&languages, &root_path)?
        } else {
            HashMap::new()
        };
        
        // 生成建议
        let recommendations = self.generate_recommendations(&languages, &project_type, &dependencies);
        
        Ok(EnvironmentInfo {
            primary_language,
            languages,
            project_type,
            dependencies,
            recommendations,
        })
    }

    fn scan_files(&self, root_path: &Path, depth: usize) -> Result<HashMap<String, usize>> {
        let mut file_counts = HashMap::new();
        
        let ignore_patterns = vec![
            "node_modules", "target", ".git", "__pycache__", 
            "dist", "build", ".vscode", ".idea", "vendor"
        ];
        
        for entry in WalkDir::new(root_path)
            .max_depth(depth)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();
                !ignore_patterns.iter().any(|pattern| {
                    path.components().any(|comp| comp.as_os_str() == *pattern)
                })
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    if let Some(ext_str) = extension.to_str() {
                        *file_counts.entry(ext_str.to_lowercase()).or_insert(0) += 1;
                    }
                }
            }
        }
        
        Ok(file_counts)
    }

    fn detect_languages(&self, file_info: &HashMap<String, usize>, root_path: &Path) -> Result<Vec<LanguageInfo>> {
        let mut languages = Vec::new();
        
        // 语言扩展名映射
        let language_extensions = self.get_language_extensions();
        
        // 配置文件映射
        let config_files = self.get_config_files();
        
        // 计算每种语言的权重
        let mut language_weights: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        
        // 基于文件扩展名计算权重
        for (ext, count) in file_info {
            if let Some(lang) = language_extensions.get(ext) {
                let entry = language_weights.entry(lang.clone()).or_insert((0, Vec::new()));
                entry.0 += count;
            }
        }
        
        // 检查配置文件
        for (file_name, lang) in &config_files {
            let config_path = root_path.join(file_name);
            if config_path.exists() {
                let entry = language_weights.entry(lang.clone()).or_insert((0, Vec::new()));
                entry.1.push(file_name.clone());
                // 配置文件存在给予额外权重
                entry.0 += 10;
            }
        }
        
        // 计算总文件数
        let total_files: usize = language_weights.values().map(|(count, _)| *count).sum();
        
        // 生成语言信息
        for (lang, (file_count, configs)) in language_weights {
            let weight = if total_files > 0 {
                file_count as f64 / total_files as f64
            } else {
                0.0
            };
            
            languages.push(LanguageInfo {
                name: lang,
                weight,
                file_count,
                config_files: configs,
            });
        }
        
        // 按权重排序
        languages.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(languages)
    }

    fn analyze_project_type(&self, languages: &[LanguageInfo], root_path: &Path) -> Result<Option<ProjectType>> {
        if languages.is_empty() {
            return Ok(None);
        }
        
        let primary_lang = &languages[0].name;
        let mut frameworks = Vec::new();
        let mut build_system = None;
        let mut category = "application".to_string();
        let mut subcategory = None;
        
        match primary_lang.as_str() {
            "rust" => {
                build_system = Some("cargo".to_string());
                if let Ok(cargo_toml) = self.read_cargo_toml(root_path) {
                    // 检测 Rust 框架
                    if cargo_toml.contains("rocket") {
                        frameworks.push("rocket".to_string());
                        subcategory = Some("web_server".to_string());
                    }
                    if cargo_toml.contains("actix-web") {
                        frameworks.push("actix-web".to_string());
                        subcategory = Some("web_server".to_string());
                    }
                    if cargo_toml.contains("tauri") {
                        frameworks.push("tauri".to_string());
                        subcategory = Some("desktop_app".to_string());
                    }
                    if cargo_toml.contains("serde") {
                        frameworks.push("serde".to_string());
                    }
                    if cargo_toml.contains("tokio") {
                        frameworks.push("tokio".to_string());
                    }
                    
                    // 检测项目类型
                    if cargo_toml.contains(r#"crate-type = ["cdylib"]"#) {
                        category = "library".to_string();
                    }
                }
            },
            "javascript" | "typescript" => {
                if let Ok(package_json) = self.read_package_json(root_path) {
                    if package_json.contains("react") {
                        frameworks.push("react".to_string());
                        subcategory = Some("web_frontend".to_string());
                    }
                    if package_json.contains("vue") {
                        frameworks.push("vue".to_string());
                        subcategory = Some("web_frontend".to_string());
                    }
                    if package_json.contains("express") {
                        frameworks.push("express".to_string());
                        subcategory = Some("web_server".to_string());
                    }
                    if package_json.contains("next") {
                        frameworks.push("next.js".to_string());
                        subcategory = Some("web_fullstack".to_string());
                    }
                }
                build_system = Some("npm".to_string());
            },
            "python" => {
                if let Ok(requirements) = self.read_requirements_txt(root_path) {
                    if requirements.contains("django") {
                        frameworks.push("django".to_string());
                        subcategory = Some("web_server".to_string());
                    }
                    if requirements.contains("fastapi") {
                        frameworks.push("fastapi".to_string());
                        subcategory = Some("web_api".to_string());
                    }
                    if requirements.contains("flask") {
                        frameworks.push("flask".to_string());
                        subcategory = Some("web_server".to_string());
                    }
                }
                build_system = Some("pip".to_string());
            },
            "java" => {
                if root_path.join("pom.xml").exists() {
                    build_system = Some("maven".to_string());
                } else if root_path.join("build.gradle").exists() {
                    build_system = Some("gradle".to_string());
                }
            },
            "go" => {
                build_system = Some("go".to_string());
                if let Ok(go_mod) = self.read_go_mod(root_path) {
                    if go_mod.contains("gin") {
                        frameworks.push("gin".to_string());
                        subcategory = Some("web_server".to_string());
                    }
                }
            },
            _ => {}
        }
        
        Ok(Some(ProjectType {
            category,
            subcategory,
            frameworks,
            build_system,
        }))
    }

    fn analyze_dependencies(&self, languages: &[LanguageInfo], root_path: &Path) -> Result<HashMap<String, LanguageDependencies>> {
        let mut dependencies = HashMap::new();
        
        for lang_info in languages {
            match lang_info.name.as_str() {
                "rust" => {
                    if let Ok(deps) = self.parse_rust_dependencies(root_path) {
                        dependencies.insert("rust".to_string(), deps);
                    }
                },
                "javascript" | "typescript" => {
                    if let Ok(deps) = self.parse_js_dependencies(root_path) {
                        dependencies.insert("javascript".to_string(), deps);
                    }
                },
                "python" => {
                    if let Ok(deps) = self.parse_python_dependencies(root_path) {
                        dependencies.insert("python".to_string(), deps);
                    }
                },
                _ => {}
            }
        }
        
        Ok(dependencies)
    }

    fn parse_rust_dependencies(&self, root_path: &Path) -> Result<LanguageDependencies> {
        let cargo_toml_path = root_path.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Ok(LanguageDependencies {
                total_count: 0,
                dev_dependencies: 0,
                outdated: 0,
                vulnerable: 0,
                packages: Vec::new(),
            });
        }
        
        let content = fs::read_to_string(&cargo_toml_path)?;
        let parsed: toml::Value = toml::from_str(&content)?;
        
        let mut packages = Vec::new();
        let mut total_count = 0;
        let mut dev_dependencies = 0;
        
        // 解析普通依赖
        if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_table()) {
            for (name, version_info) in deps {
                total_count += 1;
                let version = self.extract_version_from_toml_value(version_info);
                packages.push(DependencyInfo {
                    name: name.clone(),
                    version,
                    latest: None, // 需要调用外部API获取
                    status: "current".to_string(),
                });
            }
        }
        
        // 解析开发依赖
        if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|v| v.as_table()) {
            for (name, version_info) in dev_deps {
                dev_dependencies += 1;
                total_count += 1;
                let version = self.extract_version_from_toml_value(version_info);
                packages.push(DependencyInfo {
                    name: name.clone(),
                    version,
                    latest: None,
                    status: "current".to_string(),
                });
            }
        }
        
        Ok(LanguageDependencies {
            total_count,
            dev_dependencies,
            outdated: 0, // 需要版本检查来确定
            vulnerable: 0, // 需要安全扫描来确定
            packages,
        })
    }

    fn parse_js_dependencies(&self, root_path: &Path) -> Result<LanguageDependencies> {
        let package_json_path = root_path.join("package.json");
        if !package_json_path.exists() {
            return Ok(LanguageDependencies {
                total_count: 0,
                dev_dependencies: 0,
                outdated: 0,
                vulnerable: 0,
                packages: Vec::new(),
            });
        }
        
        let content = fs::read_to_string(&package_json_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&content)?;
        
        let mut packages = Vec::new();
        let mut total_count = 0;
        let mut dev_dependencies = 0;
        
        // 解析普通依赖
        if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                total_count += 1;
                packages.push(DependencyInfo {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("unknown").to_string(),
                    latest: None,
                    status: "current".to_string(),
                });
            }
        }
        
        // 解析开发依赖
        if let Some(dev_deps) = parsed.get("devDependencies").and_then(|v| v.as_object()) {
            for (name, version) in dev_deps {
                dev_dependencies += 1;
                total_count += 1;
                packages.push(DependencyInfo {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("unknown").to_string(),
                    latest: None,
                    status: "current".to_string(),
                });
            }
        }
        
        Ok(LanguageDependencies {
            total_count,
            dev_dependencies,
            outdated: 0,
            vulnerable: 0,
            packages,
        })
    }

    fn parse_python_dependencies(&self, root_path: &Path) -> Result<LanguageDependencies> {
        let mut packages = Vec::new();
        let mut total_count = 0;
        
        // 尝试读取 requirements.txt
        let requirements_path = root_path.join("requirements.txt");
        if requirements_path.exists() {
            let content = fs::read_to_string(&requirements_path)?;
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    if let Some((name, version)) = self.parse_python_requirement(line) {
                        total_count += 1;
                        packages.push(DependencyInfo {
                            name,
                            version,
                            latest: None,
                            status: "current".to_string(),
                        });
                    }
                }
            }
        }
        
        // 尝试读取 pyproject.toml
        let pyproject_path = root_path.join("pyproject.toml");
        if pyproject_path.exists() {
            let content = fs::read_to_string(&pyproject_path)?;
            if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
                if let Some(deps) = parsed.get("project")
                    .and_then(|p| p.get("dependencies"))
                    .and_then(|d| d.as_array()) {
                    for dep in deps {
                        if let Some(dep_str) = dep.as_str() {
                            if let Some((name, version)) = self.parse_python_requirement(dep_str) {
                                total_count += 1;
                                packages.push(DependencyInfo {
                                    name,
                                    version,
                                    latest: None,
                                    status: "current".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(LanguageDependencies {
            total_count,
            dev_dependencies: 0, // Python 的开发依赖通常在单独的文件中
            outdated: 0,
            vulnerable: 0,
            packages,
        })
    }

    fn generate_recommendations(&self, languages: &[LanguageInfo], project_type: &Option<ProjectType>, dependencies: &HashMap<String, LanguageDependencies>) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // 基于主要语言的建议
        if let Some(primary_lang) = languages.first() {
            match primary_lang.name.as_str() {
                "rust" => {
                    recommendations.push("检测到 Rust 项目，建议使用 cargo fmt 和 cargo clippy 保持代码质量".to_string());
                    if primary_lang.config_files.contains(&"Cargo.toml".to_string()) {
                        recommendations.push("建议定期运行 cargo update 更新依赖版本".to_string());
                    }
                },
                "javascript" | "typescript" => {
                    recommendations.push("检测到 JavaScript/TypeScript 项目，建议配置 ESLint 和 Prettier".to_string());
                    if primary_lang.config_files.contains(&"package.json".to_string()) {
                        recommendations.push("建议定期运行 npm audit 检查安全漏洞".to_string());
                    }
                },
                "python" => {
                    recommendations.push("检测到 Python 项目，建议使用虚拟环境管理依赖".to_string());
                    recommendations.push("建议配置 black 和 flake8 保持代码风格一致".to_string());
                },
                _ => {}
            }
        }
        
        // 基于项目类型的建议
        if let Some(proj_type) = project_type {
            if proj_type.frameworks.contains(&"rocket".to_string()) {
                recommendations.push("检测到 Rocket 框架，建议查看 Rocket 官方文档了解最佳实践".to_string());
            }
            if proj_type.frameworks.contains(&"react".to_string()) {
                recommendations.push("检测到 React 框架，建议使用 React DevTools 进行调试".to_string());
            }
        }
        
        // 基于依赖的建议
        for (lang, deps) in dependencies {
            if deps.total_count > 50 {
                recommendations.push(format!("{} 项目依赖较多 ({} 个)，建议定期清理不必要的依赖", lang, deps.total_count));
            }
        }
        
        // 多语言项目建议
        if languages.len() > 1 {
            recommendations.push("检测到多语言项目，建议在 README 中说明各语言的用途和构建方式".to_string());
        }
        
        recommendations
    }

    // 辅助方法
    fn get_language_extensions(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("rs".to_string(), "rust".to_string());
        map.insert("py".to_string(), "python".to_string());
        map.insert("pyi".to_string(), "python".to_string());
        map.insert("js".to_string(), "javascript".to_string());
        map.insert("jsx".to_string(), "javascript".to_string());
        map.insert("ts".to_string(), "typescript".to_string());
        map.insert("tsx".to_string(), "typescript".to_string());
        map.insert("java".to_string(), "java".to_string());
        map.insert("go".to_string(), "go".to_string());
        map.insert("dart".to_string(), "dart".to_string());
        map.insert("c".to_string(), "c".to_string());
        map.insert("cpp".to_string(), "cpp".to_string());
        map.insert("cc".to_string(), "cpp".to_string());
        map.insert("cxx".to_string(), "cpp".to_string());
        map.insert("cs".to_string(), "csharp".to_string());
        map.insert("php".to_string(), "php".to_string());
        map.insert("rb".to_string(), "ruby".to_string());
        map.insert("swift".to_string(), "swift".to_string());
        map.insert("kt".to_string(), "kotlin".to_string());
        map.insert("scala".to_string(), "scala".to_string());
        map
    }

    fn get_config_files(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("Cargo.toml".to_string(), "rust".to_string());
        map.insert("package.json".to_string(), "javascript".to_string());
        map.insert("pyproject.toml".to_string(), "python".to_string());
        map.insert("setup.py".to_string(), "python".to_string());
        map.insert("requirements.txt".to_string(), "python".to_string());
        map.insert("Pipfile".to_string(), "python".to_string());
        map.insert("pom.xml".to_string(), "java".to_string());
        map.insert("build.gradle".to_string(), "java".to_string());
        map.insert("go.mod".to_string(), "go".to_string());
        map.insert("pubspec.yaml".to_string(), "dart".to_string());
        map
    }

    fn read_cargo_toml(&self, root_path: &Path) -> Result<String> {
        let path = root_path.join("Cargo.toml");
        Ok(fs::read_to_string(path)?)
    }

    fn read_package_json(&self, root_path: &Path) -> Result<String> {
        let path = root_path.join("package.json");
        Ok(fs::read_to_string(path)?)
    }

    fn read_requirements_txt(&self, root_path: &Path) -> Result<String> {
        let path = root_path.join("requirements.txt");
        Ok(fs::read_to_string(path)?)
    }

    fn read_go_mod(&self, root_path: &Path) -> Result<String> {
        let path = root_path.join("go.mod");
        Ok(fs::read_to_string(path)?)
    }

    fn extract_version_from_toml_value(&self, value: &toml::Value) -> String {
        match value {
            toml::Value::String(s) => s.clone(),
            toml::Value::Table(table) => {
                table.get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            },
            _ => "unknown".to_string(),
        }
    }

    fn parse_python_requirement(&self, line: &str) -> Option<(String, String)> {
        // 简单的 Python 依赖解析，支持 name==version 格式
        if let Some(pos) = line.find("==") {
            let name = line[..pos].trim().to_string();
            let version = line[pos + 2..].trim().to_string();
            Some((name, version))
        } else if let Some(pos) = line.find(">=") {
            let name = line[..pos].trim().to_string();
            let version = format!(">={}", line[pos + 2..].trim());
            Some((name, version))
        } else if let Some(pos) = line.find("~=") {
            let name = line[..pos].trim().to_string();
            let version = format!("~={}", line[pos + 2..].trim());
            Some((name, version))
        } else {
            // 没有版本号的情况
            Some((line.trim().to_string(), "latest".to_string()))
        }
    }
} 