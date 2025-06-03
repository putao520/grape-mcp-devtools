use grape_mcp_devtools::tools::EnvironmentDetectionTool;
use grape_mcp_devtools::tools::base::MCPTool;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    println!("🔍 测试环境检测工具");
    println!("==================");

    // 创建环境检测工具
    let env_tool = EnvironmentDetectionTool::new();
    
    // 获取当前工作目录
    let current_dir = env::current_dir()?;
    println!("📁 当前目录: {}", current_dir.display());
    
    // 测试1: 检测当前项目环境
    println!("\n🧪 测试1: 检测当前项目环境");
    println!("--------------------------");
    
    let params = json!({
        "path": ".",
        "depth": 3,
        "include_dependencies": true,
        "include_toolchain": false
    });
    
    match env_tool.execute(params).await {
        Ok(result) => {
            println!("✅ 环境检测成功!");
            println!("📊 检测结果:");
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            println!("❌ 环境检测失败: {}", e);
        }
    }
    
    // 测试2: 检测不同深度
    println!("\n🧪 测试2: 检测不同扫描深度");
    println!("----------------------------");
    
    for depth in [1, 2, 5] {
        println!("\n📏 扫描深度: {}", depth);
        let params = json!({
            "path": ".",
            "depth": depth,
            "include_dependencies": false
        });
        
        match env_tool.execute(params).await {
            Ok(result) => {
                if let Some(env) = result.get("environment") {
                    if let Some(languages) = env.get("languages") {
                        if let Some(lang_array) = languages.as_array() {
                            println!("  🗣️ 检测到 {} 种语言", lang_array.len());
                            for lang in lang_array.iter().take(3) {
                                if let (Some(name), Some(weight), Some(count)) = (
                                    lang.get("name").and_then(|v| v.as_str()),
                                    lang.get("weight").and_then(|v| v.as_f64()),
                                    lang.get("file_count").and_then(|v| v.as_u64())
                                ) {
                                    println!("    - {}: {:.1}% ({} 文件)", name, weight * 100.0, count);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("  ❌ 深度 {} 检测失败: {}", depth, e);
            }
        }
    }
    
    // 测试3: 只检测语言，不分析依赖
    println!("\n🧪 测试3: 快速语言检测（无依赖分析）");
    println!("--------------------------------------");
    
    let params = json!({
        "path": ".",
        "depth": 2,
        "include_dependencies": false
    });
    
    match env_tool.execute(params).await {
        Ok(result) => {
            println!("✅ 快速检测成功!");
            if let Some(env) = result.get("environment") {
                if let Some(primary) = env.get("primary_language").and_then(|v| v.as_str()) {
                    println!("🎯 主要语言: {}", primary);
                }
                
                if let Some(project_type) = env.get("project_type") {
                    if let Some(category) = project_type.get("category").and_then(|v| v.as_str()) {
                        println!("📂 项目类型: {}", category);
                    }
                    if let Some(frameworks) = project_type.get("frameworks").and_then(|v| v.as_array()) {
                        if !frameworks.is_empty() {
                            let framework_names: Vec<String> = frameworks.iter()
                                .filter_map(|f| f.as_str())
                                .map(|s| s.to_string())
                                .collect();
                            println!("🔧 检测到框架: [{}]", framework_names.join(", "));
                        }
                    }
                }
                
                if let Some(recommendations) = env.get("recommendations").and_then(|v| v.as_array()) {
                    if !recommendations.is_empty() {
                        println!("💡 建议:");
                        for (i, rec) in recommendations.iter().enumerate() {
                            if let Some(text) = rec.as_str() {
                                println!("  {}. {}", i + 1, text);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ 快速检测失败: {}", e);
        }
    }
    
    // 测试4: 测试工具信息
    println!("\n🧪 测试4: 工具元信息");
    println!("--------------------");
    
    println!("🔧 工具名称: {}", env_tool.name());
    println!("📝 工具描述: {}", env_tool.description());
    println!("⚙️ 参数结构: 支持 path, depth, include_dependencies, include_toolchain");
    
    // 测试5: 错误处理
    println!("\n🧪 测试5: 错误处理测试");
    println!("----------------------");
    
    let invalid_params = json!({
        "path": "/nonexistent/path/that/does/not/exist",
        "depth": 1
    });
    
    match env_tool.execute(invalid_params).await {
        Ok(_) => {
            println!("⚠️ 预期应该失败，但成功了");
        }
        Err(e) => {
            println!("✅ 正确处理了无效路径: {}", e);
        }
    }
    
    println!("\n🎉 环境检测工具测试完成!");
    println!("========================");
    
    Ok(())
} 