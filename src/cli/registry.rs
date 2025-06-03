use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, warn};

use crate::tools::base::MCPTool;
use crate::mcp::server::MCPServer;
use super::detector::{CliDetector, CliToolInfo};

/// 工具注册策略
#[derive(Debug, Clone)]
pub enum RegistrationStrategy {
    /// 仅注册可用的工具
    OnlyAvailable,
    /// 强制注册所有工具
    ForceAll,
    /// 基于特性的选择性注册
    FeatureBased(Vec<String>),
}

/// 动态工具注册器
pub struct DynamicToolRegistry {
    /// CLI检测器
    detector: CliDetector,
    /// 工具映射表：CLI工具名 -> MCP工具创建函数
    tool_factories: HashMap<String, Box<dyn Fn() -> Box<dyn MCPTool>>>,
    /// 注册策略
    strategy: RegistrationStrategy,
}

impl DynamicToolRegistry {
    /// 创建新的动态工具注册器
    pub fn new(strategy: RegistrationStrategy) -> Self {
        let mut registry = Self {
            detector: CliDetector::new(),
            tool_factories: HashMap::new(),
            strategy,
        };

        // 注册工具创建工厂
        registry.register_tool_factories();
        registry
    }

    /// 注册工具创建工厂
    fn register_tool_factories(&mut self) {
        
        use crate::tools::versioning::CheckVersionTool;
        use crate::tools::search::SearchDocsTools;
        use crate::tools::dependencies::AnalyzeDependenciesTool;
        use crate::tools::analysis::AnalyzeCodeTool;
        use crate::tools::api_docs::GetApiDocsTool;
        use crate::tools::python_docs_tool::PythonDocsTool;
        use crate::tools::javascript_docs_tool::JavaScriptDocsTool;
        use crate::tools::typescript_docs_tool::TypeScriptDocsTool;
        use crate::tools::rust_docs_tool::RustDocsTool;
        use crate::tools::java_docs_tool::JavaDocsTool;
        use crate::tools::vector_docs_tool::VectorDocsTool;

        // 版本控制相关工具
        self.register_factory("git", || {
            Box::new(AnalyzeCodeTool) // 可以根据git创建版本相关工具
        });

        // 包管理器相关工具
        self.register_factory("cargo", || {
            Box::new(CheckVersionTool::new())
        });

        self.register_factory("npm", || {
            Box::new(CheckVersionTool::new())
        });

        self.register_factory("pip", || {
            Box::new(CheckVersionTool::new())
        });

        self.register_factory("mvn", || {
            Box::new(CheckVersionTool::new())
        });

        self.register_factory("gradle", || {
            Box::new(CheckVersionTool::new())
        });

        // 文档工具相关
        self.register_factory("rustdoc", || {
            Box::new(RustDocsTool::new())
        });

        self.register_factory("jsdoc", || {
            Box::new(JavaScriptDocsTool::new())
        });

        self.register_factory("tsc", || {
            Box::new(TypeScriptDocsTool::new())
        });

        self.register_factory("typedoc", || {
            Box::new(TypeScriptDocsTool::new())
        });

        self.register_factory("sphinx-build", || {
            Box::new(PythonDocsTool::new())
        });

        self.register_factory("javadoc", || {
            Box::new(JavaDocsTool::new())
        });

        // 代码分析工具
        self.register_factory("clippy", || {
            Box::new(AnalyzeCodeTool)
        });

        self.register_factory("eslint", || {
            Box::new(AnalyzeCodeTool)
        });

        self.register_factory("pylint", || {
            Box::new(AnalyzeCodeTool)
        });

        // 依赖分析工具
        self.register_factory("cargo-audit", || {
            Box::new(AnalyzeDependenciesTool::new())
        });

        self.register_factory("npm-audit", || {
            Box::new(AnalyzeDependenciesTool::new())
        });

        // 通用工具（始终可用）
        self.register_factory("_universal_search", || {
            Box::new(SearchDocsTools::new())
        });

        self.register_factory("_universal_version_check", || {
            Box::new(CheckVersionTool::new())
        });

        self.register_factory("_universal_deps_analysis", || {
            Box::new(AnalyzeDependenciesTool::new())
        });

        self.register_factory("_universal_code_analysis", || {
            Box::new(AnalyzeCodeTool)
        });

        self.register_factory("_universal_api_docs", || {
            Box::new(GetApiDocsTool::new())
        });

        // 通用语言文档工具（始终可用）
        self.register_factory("_universal_python_docs", || {
            Box::new(PythonDocsTool::new())
        });

        self.register_factory("_universal_javascript_docs", || {
            Box::new(JavaScriptDocsTool::new())
        });

        self.register_factory("_universal_typescript_docs", || {
            Box::new(TypeScriptDocsTool::new())
        });

        self.register_factory("_universal_rust_docs", || {
            Box::new(RustDocsTool::new())
        });

        self.register_factory("_universal_java_docs", || {
            Box::new(JavaDocsTool::new())
        });

        // 嵌入式向量化文档工具（instant-distance，始终可用）
        self.register_factory("_universal_vector_docs", || {
            match VectorDocsTool::new() {
                Ok(tool) => Box::new(tool),
                Err(e) => {
                    warn!("创建嵌入式向量化文档工具失败: {}", e);
                    // 返回一个默认的占位工具，或者可以考虑其他处理方式
                    Box::new(VectorDocsTool::default())
                }
            }
        });
    }

    /// 注册单个工具工厂
    fn register_factory<F>(&mut self, cli_tool: &str, factory: F)
    where
        F: Fn() -> Box<dyn MCPTool> + 'static,
    {
        self.tool_factories.insert(cli_tool.to_string(), Box::new(factory));
    }

    /// 检测环境并动态注册工具
    pub async fn detect_and_register(&mut self, mcp_server: &MCPServer) -> Result<RegistrationReport> {
        // 1. 检测CLI工具
        info!("🔍 开始检测环境中的CLI工具...");
        let detected_tools = self.detector.detect_all().await?;

        // 2. 生成注册计划
        let registration_plan = self.create_registration_plan(&detected_tools);

        // 3. 执行注册
        let mut report = RegistrationReport::new();
        for (tool_name, should_register, reason) in registration_plan {
            if should_register {
                if let Some(factory) = self.tool_factories.get(&tool_name) {
                    let mcp_tool = factory();
                    match mcp_server.register_tool(mcp_tool).await {
                        Ok(_) => {
                            info!("✅ 注册工具: {} ({})", tool_name, reason);
                            report.registered_tools.push(tool_name.clone());
                        }
                        Err(e) => {
                            warn!("❌ 注册工具失败: {} - {}", tool_name, e);
                            report.failed_tools.push((tool_name.clone(), e.to_string()));
                        }
                    }
                } else {
                    warn!("⚠️ 未找到工具工厂: {}", tool_name);
                    report.skipped_tools.push((tool_name.clone(), "未找到工具工厂".to_string()));
                }
            } else {
                report.skipped_tools.push((tool_name.clone(), reason));
            }
        }

        // 4. 注册通用工具（根据策略）
        self.register_universal_tools(mcp_server, &mut report).await?;

        info!("📊 工具注册完成: {} 成功, {} 失败, {} 跳过", 
            report.registered_tools.len(), 
            report.failed_tools.len(), 
            report.skipped_tools.len());

        Ok(report)
    }

    /// 创建注册计划
    fn create_registration_plan(&self, detected_tools: &HashMap<String, CliToolInfo>) -> Vec<(String, bool, String)> {
        let mut plan = Vec::new();

        match &self.strategy {
            RegistrationStrategy::OnlyAvailable => {
                // 只注册检测到的可用工具
                for (tool_name, tool_info) in detected_tools {
                    if tool_info.available {
                        plan.push((
                            tool_name.clone(), 
                            true, 
                            format!("CLI工具可用 ({})", tool_info.version.as_ref().unwrap_or(&"未知版本".to_string()))
                        ));
                    } else {
                        plan.push((
                            tool_name.clone(), 
                            false, 
                            "CLI工具不可用".to_string()
                        ));
                    }
                }
            }

            RegistrationStrategy::ForceAll => {
                // 强制注册所有工具
                for tool_name in self.tool_factories.keys() {
                    if !tool_name.starts_with("_universal") {
                        plan.push((
                            tool_name.clone(), 
                            true, 
                            "强制注册所有工具".to_string()
                        ));
                    }
                }
            }

            RegistrationStrategy::FeatureBased(required_features) => {
                // 基于特性的选择性注册
                for (tool_name, tool_info) in detected_tools {
                    if tool_info.available {
                        let has_required_feature = required_features.iter()
                            .any(|feature| tool_info.features.contains(feature));
                        
                        if has_required_feature {
                            plan.push((
                                tool_name.clone(), 
                                true, 
                                format!("具有所需特性: {:?}", tool_info.features)
                            ));
                        } else {
                            plan.push((
                                tool_name.clone(), 
                                false, 
                                format!("缺少所需特性，当前特性: {:?}", tool_info.features)
                            ));
                        }
                    }
                }
            }
        }

        plan
    }

    /// 注册通用工具
    async fn register_universal_tools(&self, mcp_server: &MCPServer, report: &mut RegistrationReport) -> Result<()> {
        // 始终注册的通用工具
        let universal_tools = vec![
            "_universal_search",
            "_universal_version_check", 
            "_universal_deps_analysis",
            "_universal_code_analysis",
            "_universal_api_docs",
            "_universal_python_docs",
            "_universal_javascript_docs",
            "_universal_typescript_docs",
            "_universal_rust_docs",
            "_universal_java_docs",
            "_universal_vector_docs",
        ];

        for tool_name in universal_tools {
            if let Some(factory) = self.tool_factories.get(tool_name) {
                let mcp_tool = factory();
                match mcp_server.register_tool(mcp_tool).await {
                    Ok(_) => {
                        info!("✅ 注册通用工具: {}", tool_name);
                        report.registered_tools.push(tool_name.to_string());
                    }
                    Err(e) => {
                        warn!("❌ 注册通用工具失败: {} - {}", tool_name, e);
                        report.failed_tools.push((tool_name.to_string(), e.to_string()));
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取检测报告
    pub fn get_detection_report(&self) -> String {
        self.detector.generate_report()
    }

    /// 检查特定工具是否可用
    pub fn is_tool_available(&self, tool_name: &str) -> bool {
        self.detector.is_tool_available(tool_name)
    }

    /// 获取可用工具列表
    pub fn get_available_tools(&self) -> Vec<&CliToolInfo> {
        self.detector.get_available_tools()
    }
}

/// 工具注册报告
#[derive(Debug, Clone)]
pub struct RegistrationReport {
    /// 成功注册的工具
    pub registered_tools: Vec<String>,
    /// 注册失败的工具
    pub failed_tools: Vec<(String, String)>,
    /// 跳过的工具
    pub skipped_tools: Vec<(String, String)>,
}

impl RegistrationReport {
    pub fn new() -> Self {
        Self {
            registered_tools: Vec::new(),
            failed_tools: Vec::new(),
            skipped_tools: Vec::new(),
        }
    }

    /// 生成报告字符串
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("🎯 MCP 工具注册报告\n");
        report.push_str(&"=".repeat(50));
        report.push('\n');

        report.push_str(&format!("📊 总结: {} 成功, {} 失败, {} 跳过\n\n", 
            self.registered_tools.len(), 
            self.failed_tools.len(), 
            self.skipped_tools.len()));

        if !self.registered_tools.is_empty() {
            report.push_str("✅ 成功注册的工具:\n");
            for tool in &self.registered_tools {
                report.push_str(&format!("  • {}\n", tool));
            }
            report.push('\n');
        }

        if !self.failed_tools.is_empty() {
            report.push_str("❌ 注册失败的工具:\n");
            for (tool, error) in &self.failed_tools {
                report.push_str(&format!("  • {}: {}\n", tool, error));
            }
            report.push('\n');
        }

        if !self.skipped_tools.is_empty() {
            report.push_str("⏭️ 跳过的工具:\n");
            for (tool, reason) in &self.skipped_tools {
                report.push_str(&format!("  • {}: {}\n", tool, reason));
            }
            report.push('\n');
        }

        report
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> (usize, usize, usize) {
        (
            self.registered_tools.len(),
            self.failed_tools.len(), 
            self.skipped_tools.len()
        )
    }
} 