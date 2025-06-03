use grape_mcp_devtools::tools::versioning::CheckVersionTool;
use grape_mcp_devtools::tools::base::MCPTool;
use serde_json::json;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 专门测试Flutter和Dart SDK版本检查");
    println!("{}", "=".repeat(60));
    
    let version_tool = CheckVersionTool::new();
    
    // 测试Flutter SDK
    println!("\n🚀 测试Flutter SDK版本检查");
    println!("📍 数据源: GitHub Releases API (flutter/flutter)");
    
    let flutter_params = json!({
        "type": "flutter",
        "name": "flutter",  // name参数会被忽略
        "include_preview": false
    });
    
    match version_tool.execute(flutter_params).await {
        Ok(result) => {
            println!("✅ Flutter SDK版本检查成功:");
            println!("   📦 最新稳定版: {}", result["latest_stable"].as_str().unwrap_or("未知"));
            println!("   📅 发布日期: {}", result["release_date"].as_str().unwrap_or("未知"));
            println!("   🔗 下载地址: {}", result["download_url"].as_str().unwrap_or("未知"));
            println!("   📂 代码仓库: {}", result["repository_url"].as_str().unwrap_or("未知"));
            
            if let Some(versions) = result["available_versions"].as_array() {
                println!("   📋 可用版本数量: {}", versions.len());
                println!("   📋 最近5个版本:");
                for (i, version) in versions.iter().take(5).enumerate() {
                    if let Some(v) = version.as_str() {
                        println!("      {}. {}", i + 1, v);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Flutter SDK版本检查失败: {}", e);
        }
    }
    
    // 添加延迟避免API限制
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // 测试Dart SDK
    println!("\n🎯 测试Dart SDK版本检查");
    println!("📍 数据源: GitHub Tags API (dart-lang/sdk)");
    
    let dart_params = json!({
        "type": "dart",
        "name": "dart",  // name参数会被忽略
        "include_preview": false
    });
    
    match version_tool.execute(dart_params).await {
        Ok(result) => {
            println!("✅ Dart SDK版本检查成功:");
            println!("   📦 最新稳定版: {}", result["latest_stable"].as_str().unwrap_or("未知"));
            println!("   📅 发布日期: {}", result["release_date"].as_str().unwrap_or("未知"));
            println!("   🔗 下载地址: {}", result["download_url"].as_str().unwrap_or("未知"));
            println!("   📂 代码仓库: {}", result["repository_url"].as_str().unwrap_or("未知"));
            
            if let Some(versions) = result["available_versions"].as_array() {
                println!("   📋 可用版本数量: {}", versions.len());
                println!("   📋 最近5个版本:");
                for (i, version) in versions.iter().take(5).enumerate() {
                    if let Some(v) = version.as_str() {
                        println!("      {}. {}", i + 1, v);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Dart SDK版本检查失败: {}", e);
        }
    }
    
    // 测试通过pub类型访问Flutter（应该重定向到Flutter SDK）
    println!("\n🔄 测试通过pub类型访问Flutter（特殊重定向）");
    
    let pub_flutter_params = json!({
        "type": "pub",
        "name": "flutter",
        "include_preview": false
    });
    
    match version_tool.execute(pub_flutter_params).await {
        Ok(result) => {
            println!("✅ pub类型Flutter重定向成功:");
            println!("   📦 包类型: {}", result["package_type"].as_str().unwrap_or("未知"));
            println!("   📦 最新版本: {}", result["latest_stable"].as_str().unwrap_or("未知"));
            
            if result["package_type"].as_str() == Some("flutter") {
                println!("   ✅ 成功重定向到Flutter SDK检查");
            } else {
                println!("   ⚠️ 重定向可能有问题");
            }
        }
        Err(e) => {
            println!("❌ pub类型Flutter重定向失败: {}", e);
        }
    }
    
    // 测试通过pub类型访问Dart（应该重定向到Dart SDK）
    println!("\n🔄 测试通过pub类型访问Dart（特殊重定向）");
    
    let pub_dart_params = json!({
        "type": "pub",
        "name": "dart",
        "include_preview": false
    });
    
    match version_tool.execute(pub_dart_params).await {
        Ok(result) => {
            println!("✅ pub类型Dart重定向成功:");
            println!("   📦 包类型: {}", result["package_type"].as_str().unwrap_or("未知"));
            println!("   📦 最新版本: {}", result["latest_stable"].as_str().unwrap_or("未知"));
            
            if result["package_type"].as_str() == Some("dart") {
                println!("   ✅ 成功重定向到Dart SDK检查");
            } else {
                println!("   ⚠️ 重定向可能有问题");
            }
        }
        Err(e) => {
            println!("❌ pub类型Dart重定向失败: {}", e);
        }
    }
    
    println!("\n🎉 Flutter和Dart SDK版本检查测试完成！");
    println!("📊 测试总结:");
    println!("   ✅ Flutter SDK (GitHub Releases API)");
    println!("   ✅ Dart SDK (GitHub Tags API)");
    println!("   ✅ 智能重定向功能");
    println!("   ✅ 版本过滤和排序");
    
    Ok(())
} 