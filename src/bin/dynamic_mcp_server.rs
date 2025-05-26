
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use grape_mcp_devtools::{
    mcp::server::{MCPServer, Server},
    cli::{DynamicToolRegistry, registry::RegistrationStrategy},
};

/// 动态MCP服务器 - 根据环境自动检测和注册工具
#[derive(Parser)]
#[command(name = "dynamic-mcp-server")]
#[command(about = "动态MCP服务器 - 智能检测环境并注册相应工具")]
#[command(version = "0.1.0")]
struct Cli {
    /// 启用详细日志
    #[arg(short, long)]
    verbose: bool,

    /// 强制注册所有工具（忽略CLI检测结果）
    #[arg(short = 'a', long = "all")]
    force_all: bool,

    /// 仅输出检测报告，不启动服务器
    #[arg(short = 'r', long = "report-only")]
    report_only: bool,

    /// 基于特性过滤工具（可多次指定）
    #[arg(short = 'f', long = "feature", action = clap::ArgAction::Append)]
    features: Vec<String>,

    /// 服务器名称
    #[arg(long, default_value = "grape-mcp-devtools")]
    server_name: String,

    /// 服务器版本
    #[arg(long, default_value = "0.1.0")]
    server_version: String,

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
    /// 启动MCP服务器
    Serve {
        /// 服务器端口
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// 服务器主机
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// 显示工具注册策略信息
    Strategies,
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
    println!("🚀 动态MCP服务器启动");
    println!("📋 服务器: {} v{}", cli.server_name, cli.server_version);
    println!("{}", "=".repeat(60));

    match cli.command {
        Some(Commands::Detect { verbose }) => {
            handle_detect(verbose).await?;
        }
        Some(Commands::Serve { port, ref host }) => {
            handle_serve(&cli, host.clone(), port).await?;
        }
        Some(Commands::Strategies) => {
            handle_strategies();
        }
        None => {
            // 默认行为：检测并启动服务器
            handle_default(&cli).await?;
        }
    }

    Ok(())
}

/// 处理检测命令
async fn handle_detect(verbose: bool) -> Result<()> {
    info!("🔍 执行CLI工具检测...");
    
    let strategy = RegistrationStrategy::OnlyAvailable;
    let mut registry = DynamicToolRegistry::new(strategy);
    
    // 创建临时MCP服务器用于检测
    let mcp_server = MCPServer::new();
    let report = registry.detect_and_register(&mcp_server).await?;
    
    println!("{}", registry.get_detection_report());
    
    if verbose {
        println!("{}", report.generate_report());
    }
    
    let available_tools = registry.get_available_tools();
    println!("💡 提示: 找到 {} 个可用的CLI工具", available_tools.len());
    println!("🎯 运行 `dynamic-mcp-server serve` 启动服务器并注册这些工具");
    
    Ok(())
}

/// 处理服务命令
async fn handle_serve(cli: &Cli, host: String, port: u16) -> Result<()> {
    info!("🚀 启动MCP服务器...");
    
    // 确定注册策略
    let strategy = determine_strategy(cli);
    info!("📋 使用注册策略: {:?}", strategy);
    
    // 创建MCP服务器
    let mcp_server = MCPServer::new();
    
    // 创建动态注册器并执行注册
    let mut registry = DynamicToolRegistry::new(strategy);
    let report = registry.detect_and_register(&mcp_server).await?;
    
    // 显示注册报告
    if !cli.report_only {
        println!("{}", report.generate_report());
        let (success, failed, skipped) = report.get_stats();
        info!("📊 工具注册统计: {} 成功, {} 失败, {} 跳过", success, failed, skipped);
    }
    
    if cli.report_only {
        println!("📋 仅输出报告模式，不启动服务器");
        return Ok(());
    }
    
    // 启动服务器
    info!("🌐 MCP服务器启动在 {}:{}", host, port);
    println!("💡 使用 Ctrl+C 停止服务器");
    
    let mut server = Server::new(cli.server_name.clone(), cli.server_version.clone());
    server.run().await?;
    
    Ok(())
}

/// 处理策略信息命令
fn handle_strategies() {
    println!("🎯 可用的工具注册策略:\n");
    
    println!("1. 📦 OnlyAvailable (默认)");
    println!("   - 只注册检测到的可用CLI工具");
    println!("   - 安全且高效，推荐用于生产环境");
    println!("   - 使用方式: 直接运行或 --feature 指定特性\n");
    
    println!("2. 🔧 ForceAll");
    println!("   - 强制注册所有已定义的工具");
    println!("   - 忽略CLI检测结果");
    println!("   - 使用方式: --all 参数\n");
    
    println!("3. 🎯 FeatureBased");
    println!("   - 基于特性选择性注册工具");
    println!("   - 可指定多个特性进行过滤");
    println!("   - 使用方式: --feature build-tool --feature package-manager\n");
    
    println!("📚 支持的特性类别:");
    println!("   • build-tool     - 构建工具 (cargo, npm, gradle等)");
    println!("   • package-manager - 包管理器 (npm, pip, cargo等)");
    println!("   • version-control - 版本控制 (git, svn等)");
    println!("   • containerization - 容器化 (docker, podman等)");
    println!("   • rust           - Rust生态工具");
    println!("   • javascript     - JavaScript生态工具");
    println!("   • python         - Python生态工具");
    println!("   • java           - Java生态工具");
}

/// 处理默认行为
async fn handle_default(cli: &Cli) -> Result<()> {
    if cli.report_only {
        handle_detect(cli.verbose).await
    } else {
        handle_serve(cli, "127.0.0.1".to_string(), 8080).await
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

/// 在程序退出时显示信息
fn setup_signal_handlers() {
    use tokio::signal;
    
    tokio::spawn(async {
        if let Ok(_) = signal::ctrl_c().await {
            info!("🛑 收到中断信号，正在关闭服务器...");
            println!("\n👋 感谢使用动态MCP服务器！");
            std::process::exit(0);
        }
    });
} 