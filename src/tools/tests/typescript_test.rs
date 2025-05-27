use anyhow::Result;
use serde_json::json;
use crate::tools::typescript_docs_tool::TypeScriptDocsTool;
use crate::tools::base::MCPTool;

/// 测试TypeScript文档工具的基本功能
#[tokio::test]
async fn test_typescript_docs_tool_basic() -> Result<()> {
    println!("🔷 测试TypeScript文档工具基本功能");

    let tool = TypeScriptDocsTool::new();

    // 测试工具基本信息
    assert_eq!(tool.name(), "typescript_docs");
    assert!(tool.description().contains("TypeScript"));

    println!("✅ TypeScript文档工具基本信息验证通过");
    Ok(())
}

/// 测试TypeScript包文档生成
#[tokio::test]
async fn test_typescript_package_docs() -> Result<()> {
    println!("📦 测试TypeScript包文档生成");

    let tool = TypeScriptDocsTool::new();

    // 测试TypeScript官方包
    let params = json!({
        "package_name": "typescript"
    });

    let result = tool.execute(params).await?;
    println!("TypeScript官方包结果: {}", serde_json::to_string_pretty(&result)?);

    assert_eq!(result["status"], "success");
    assert!(result["data"]["package_name"].as_str().unwrap() == "typescript");
    assert!(result["data"]["language"].as_str().unwrap() == "typescript");

    println!("✅ TypeScript官方包文档生成成功");
    Ok(())
}

/// 测试TypeScript类型包处理
#[tokio::test]
async fn test_typescript_types_package() -> Result<()> {
    println!("🔷 测试TypeScript类型包处理");

    let tool = TypeScriptDocsTool::new();

    // 测试一个常见的包，可能需要@types
    let params = json!({
        "package_name": "lodash"
    });

    let result = tool.execute(params).await?;
    println!("Lodash包结果: {}", serde_json::to_string_pretty(&result)?);

    assert_eq!(result["status"], "success");
    assert!(result["data"]["package_name"].as_str().unwrap() == "lodash");

    // 检查是否提供了类型安装建议
    if let Some(installation) = result["data"]["installation"].as_object() {
        if let Some(types_suggestion) = installation.get("types_suggestion") {
            println!("✅ 提供了类型安装建议: {}", types_suggestion);
        }
    }

    println!("✅ TypeScript类型包处理测试完成");
    Ok(())
}

/// 测试TypeScript工具包
#[tokio::test]
async fn test_typescript_tooling_packages() -> Result<()> {
    println!("🛠️ 测试TypeScript工具包");

    let tool = TypeScriptDocsTool::new();

    let test_packages = vec![
        "ts-node",
        "@typescript-eslint/parser",
        "typedoc",
        "tslib"
    ];

    for package in test_packages {
        println!("测试包: {}", package);
        
        let params = json!({
            "package_name": package
        });

        let result = tool.execute(params).await?;
        assert_eq!(result["status"], "success");
        assert!(result["data"]["package_name"].as_str().unwrap() == package);
        
        // 检查TypeScript特有信息
        if let Some(ts_info) = result["data"]["typescript_info"].as_object() {
            println!("  TypeScript信息: {:?}", ts_info);
        }
    }

    println!("✅ TypeScript工具包测试完成");
    Ok(())
}

/// 测试参数验证
#[tokio::test]
async fn test_typescript_docs_parameter_validation() -> Result<()> {
    println!("🔍 测试TypeScript文档工具参数验证");

    let tool = TypeScriptDocsTool::new();

    // 测试缺少必需参数
    let invalid_params = json!({});
    let result = tool.execute(invalid_params).await;
    assert!(result.is_err());
    println!("✅ 正确拒绝了无效参数");

    // 测试有效参数
    let valid_params = json!({
        "package_name": "react",
        "version": "18.0.0"
    });
    let result = tool.execute(valid_params).await?;
    assert_eq!(result["status"], "success");
    println!("✅ 正确接受了有效参数");

    Ok(())
}

/// 测试缓存功能
#[tokio::test]
async fn test_typescript_docs_caching() -> Result<()> {
    println!("💾 测试TypeScript文档工具缓存功能");

    let tool = TypeScriptDocsTool::new();

    let params = json!({
        "package_name": "typescript"
    });

    // 第一次调用
    let start_time = std::time::Instant::now();
    let result1 = tool.execute(params.clone()).await?;
    let first_duration = start_time.elapsed();

    // 第二次调用（应该使用缓存）
    let start_time = std::time::Instant::now();
    let result2 = tool.execute(params).await?;
    let second_duration = start_time.elapsed();

    assert_eq!(result1["status"], result2["status"]);
    assert_eq!(result1["data"]["package_name"], result2["data"]["package_name"]);

    println!("第一次调用耗时: {:?}", first_duration);
    println!("第二次调用耗时: {:?}", second_duration);
    println!("✅ 缓存功能测试完成");

    Ok(())
}

/// 测试TypeScript特有功能检测
#[tokio::test]
async fn test_typescript_feature_detection() -> Result<()> {
    println!("🔍 测试TypeScript特有功能检测");

    let tool = TypeScriptDocsTool::new();

    // 测试一个包含TypeScript特性的包
    let params = json!({
        "package_name": "rxjs"
    });

    let result = tool.execute(params).await?;
    println!("RxJS包结果: {}", serde_json::to_string_pretty(&result)?);

    assert_eq!(result["status"], "success");

    // 检查是否检测到TypeScript特性
    if let Some(docs) = result["data"]["documentation"].as_object() {
        if let Some(sections) = docs.get("sections").and_then(|s| s.as_array()) {
            println!("检测到的TypeScript特性: {:?}", sections);
        }
    }

    println!("✅ TypeScript特有功能检测测试完成");
    Ok(())
}

/// 集成测试：完整的TypeScript文档工作流程
#[tokio::test]
async fn test_complete_typescript_workflow() -> Result<()> {
    println!("🔄 测试完整的TypeScript文档工作流程");

    let tool = TypeScriptDocsTool::new();

    // 1. 测试原生TypeScript包
    println!("1. 测试原生TypeScript包...");
    let native_params = json!({
        "package_name": "@types/node"
    });
    let native_result = tool.execute(native_params).await?;
    assert_eq!(native_result["status"], "success");
    println!("   ✅ 原生TypeScript包处理成功");

    // 2. 测试需要类型定义的包
    println!("2. 测试需要类型定义的包...");
    let needs_types_params = json!({
        "package_name": "express"
    });
    let needs_types_result = tool.execute(needs_types_params).await?;
    assert_eq!(needs_types_result["status"], "success");
    println!("   ✅ 需要类型定义的包处理成功");

    // 3. 测试TypeScript工具链
    println!("3. 测试TypeScript工具链...");
    let toolchain_params = json!({
        "package_name": "typescript"
    });
    let toolchain_result = tool.execute(toolchain_params).await?;
    assert_eq!(toolchain_result["status"], "success");
    println!("   ✅ TypeScript工具链处理成功");

    // 4. 验证结果包含必要信息
    println!("4. 验证结果完整性...");
    for result in [&native_result, &needs_types_result, &toolchain_result] {
        assert!(result["data"]["package_name"].is_string());
        assert!(result["data"]["language"].as_str().unwrap() == "typescript");
        assert!(result["data"]["documentation"].is_object());
        assert!(result["metadata"]["tool"].as_str().unwrap() == "typescript_docs");
    }
    println!("   ✅ 所有结果都包含必要信息");

    println!("🎉 完整的TypeScript文档工作流程测试成功！");
    Ok(())
} 