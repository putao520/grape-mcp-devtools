use grape_mcp_devtools::tools::versioning::CheckVersionTool;
use grape_mcp_devtools::tools::base::MCPTool;
use serde_json::json;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 测试包版本检查工具");
    
    let version_tool = CheckVersionTool::new();
    
    // 测试不同包管理器的版本检查
    let test_cases = vec![
        ("cargo", "serde", "Rust包版本检查"),
        ("npm", "express", "npm包版本检查"),
        ("pip", "requests", "Python包版本检查"),
        ("maven", "org.springframework:spring-core", "Maven包版本检查"),
        ("go", "github.com/gin-gonic/gin", "Go包版本检查"),
        ("pub", "http", "Dart包版本检查"),
        ("flutter", "flutter", "Flutter SDK版本检查"),
        ("dart", "dart", "Dart SDK版本检查"),
    ];
    
    for (package_type, package_name, description) in test_cases {
        println!("\n📦 {}: {}", description, package_name);
        
        let params = json!({
            "type": package_type,
            "name": package_name,
            "include_preview": false
        });
        
        match version_tool.execute(params).await {
            Ok(result) => {
                println!("✅ 成功获取版本信息:");
                println!("   最新稳定版: {}", result["latest_stable"].as_str().unwrap_or("未知"));
                println!("   发布日期: {}", result["release_date"].as_str().unwrap_or("未知"));
                println!("   下载地址: {}", result["download_url"].as_str().unwrap_or("未知"));
                
                if let Some(versions) = result["available_versions"].as_array() {
                    println!("   可用版本数量: {}", versions.len());
                }
                
                if let Some(repo_url) = result["repository_url"].as_str() {
                    println!("   代码仓库: {}", repo_url);
                }
            }
            Err(e) => {
                println!("❌ 获取版本信息失败: {}", e);
            }
        }
        
        // 添加延迟避免API限制
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    println!("\n🎉 版本检查工具测试完成！");
    Ok(())
} 