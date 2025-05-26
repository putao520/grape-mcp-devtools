use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use grape_mcp_devtools::cli::{CliDetector, registry::RegistrationStrategy};

/// 简化的动态MCP服务器 - 专注于CLI检测功能
#[derive(Parser)]
#[command(name = "simple-dynamic-server")]
#[command(about = "简化的动态MCP服务器 - 专注于CLI工具检测")]
#[command(version = "0.1.0")]
struct Cli {
    /// 启用详细日志
    #[arg(short, long)]
    verbose: bool,

    /// 强制注册所有工具（忽略CLI检测结果）
    #[arg(short = 'a', long = "all")]
    force_all: bool,

    /// 基于特性过滤工具（可多次指定）
    #[arg(short = 'f', long = "feature", action = clap::ArgAction::Append)]
    features: Vec<String>,

    /// 子命令
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 检测环境中的CLI工具
    Detect {
        /// 输出详细信息
        #[arg(short, long)]
        verbose: bool,
    },
    /// 显示工具注册策略信息
    Strategies,
    /// 分析特定特性
    Analyze {
        /// 要分析的特性
        #[arg(short, long)]
        feature: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // 显示欢迎信息
    println!("🚀 简化动态MCP服务器");
    println!("📋 专注于CLI工具检测和分析");
    println!("{}", "=".repeat(50));

    match cli.command {
        Some(Commands::Detect { verbose }) => {
            handle_detect(verbose).await?;
        }
        Some(Commands::Strategies) => {
            handle_strategies();
        }
        Some(Commands::Analyze { feature }) => {
            handle_analyze(&feature).await?;
        }
        None => {
            // 默认行为：检测工具
            handle_detect(cli.verbose).await?;
        }
    }

    Ok(())
}

/// 处理检测命令
async fn handle_detect(verbose: bool) -> Result<()> {
    info!("🔍 执行CLI工具检测...");
    
    let mut detector = CliDetector::new();
    let detected_tools = detector.detect_all().await?;
    
    println!("{}", detector.generate_report());
    
    if verbose {
        println!("\n📝 详细信息:");
        for (name, tool) in &detected_tools {
            if tool.available {
                println!("🔧 {}", name);
                println!("   版本: {:?}", tool.version);
                println!("   路径: {:?}", tool.path);
                println!("   特性: {:?}", tool.features);
                println!();
            }
        }
    }
    
    let available_tools = detector.get_available_tools();
    println!("💡 总计找到 {} 个可用的CLI工具", available_tools.len());
    
    // 生成注册建议
    println!("\n🎯 注册建议:");
    println!("• 使用 OnlyAvailable 策略注册 {} 个可用工具", available_tools.len());
    
    let build_tools = detector.filter_by_feature("build-tool");
    if !build_tools.is_empty() {
        println!("• 构建工具: {} 个 (cargo, npm, mvn 等)", build_tools.len());
    }
    
    let package_managers = detector.filter_by_feature("package-manager");
    if !package_managers.is_empty() {
        println!("• 包管理器: {} 个 (npm, pip, yarn 等)", package_managers.len());
    }
    
    let version_control = detector.filter_by_feature("version-control");
    if !version_control.is_empty() {
        println!("• 版本控制: {} 个 (git, svn 等)", version_control.len());
    }
    
    Ok(())
}

/// 处理策略信息命令
fn handle_strategies() {
    println!("🎯 可用的工具注册策略:\n");
    
    println!("1. 📦 OnlyAvailable (推荐)");
    println!("   - 只注册检测到的可用CLI工具");
    println!("   - 安全且高效，避免注册无法使用的工具");
    println!("   - 自动适应不同的开发环境\n");
    
    println!("2. 🔧 ForceAll (测试用)");
    println!("   - 强制注册所有已定义的工具");
    println!("   - 忽略CLI检测结果");
    println!("   - 适用于测试和演示\n");
    
    println!("3. 🎯 FeatureBased (定制化)");
    println!("   - 基于特性选择性注册工具");
    println!("   - 可指定多个特性进行过滤");
    println!("   - 灵活的工具组合\n");
    
    println!("📚 支持的特性类别:");
    let features = [
        ("build-tool", "构建工具 (cargo, npm, gradle等)"),
        ("package-manager", "包管理器 (npm, pip, cargo等)"),
        ("version-control", "版本控制 (git, svn等)"),
        ("containerization", "容器化 (docker, podman等)"),
        ("documentation", "文档工具 (rustdoc, jsdoc等)"),
        ("code-analysis", "代码分析 (clippy, eslint等)"),
        ("cloud", "云工具 (aws, gcloud等)"),
        ("rust", "Rust生态工具"),
        ("javascript", "JavaScript生态工具"),
        ("python", "Python生态工具"),
        ("java", "Java生态工具"),
        ("go", "Go生态工具"),
    ];
    
    for (feature, description) in features {
        println!("   • {:<20} - {}", feature, description);
    }
}

/// 处理特性分析命令
async fn handle_analyze(feature: &str) -> Result<()> {
    info!("🎯 分析特性: {}", feature);
    
    let mut detector = CliDetector::new();
    detector.detect_all().await?;
    
    let tools = detector.filter_by_feature(feature);
    
    if tools.is_empty() {
        println!("❌ 未找到具有特性 '{}' 的工具", feature);
        return Ok(());
    }
    
    println!("🔍 特性 '{}' 的工具分析:", feature);
    println!("{}", "=".repeat(40));
    
    for tool in &tools {
        println!("🔧 {}", tool.name);
        if let Some(version) = &tool.version {
            println!("   版本: {}", version);
        }
        if let Some(path) = &tool.path {
            println!("   路径: {}", path);
        }
        println!("   所有特性: {:?}", tool.features);
        
        // 根据特性给出建议
        let suggestions = get_tool_suggestions(&tool.name, feature);
        if !suggestions.is_empty() {
            println!("   💡 建议: {}", suggestions);
        }
        println!();
    }
    
    println!("📊 总结: 找到 {} 个具有 '{}' 特性的工具", tools.len(), feature);
    
    Ok(())
}

/// 获取工具建议
fn get_tool_suggestions(tool_name: &str, feature: &str) -> String {
    match (tool_name, feature) {
        ("cargo", "build-tool") => "可注册版本检查、依赖分析、代码分析工具".to_string(),
        ("npm", "package-manager") => "可注册版本检查、依赖分析工具".to_string(),
        ("git", "version-control") => "可注册代码分析、变更日志工具".to_string(),
        ("docker", "containerization") => "可注册部署、环境管理工具".to_string(),
        ("rustdoc", "documentation") => "可注册API文档、文档搜索工具".to_string(),
        ("clippy", "code-analysis") => "可注册Rust代码质量分析工具".to_string(),
        ("eslint", "code-analysis") => "可注册JavaScript代码质量分析工具".to_string(),
        (_, "build-tool") => "可注册通用构建和版本管理工具".to_string(),
        (_, "package-manager") => "可注册包版本检查和依赖管理工具".to_string(),
        _ => "可注册相关的MCP工具".to_string(),
    }
}

/// 确定注册策略
fn determine_strategy(cli: &Cli) -> RegistrationStrategy {
    if cli.force_all {
        RegistrationStrategy::ForceAll
    } else if !cli.features.is_empty() {
        RegistrationStrategy::FeatureBased(cli.features.clone())
    } else {
        RegistrationStrategy::OnlyAvailable
    }
} 