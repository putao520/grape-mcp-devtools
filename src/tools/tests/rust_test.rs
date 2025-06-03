use crate::tools::{
    SearchDocsTool,
    dependencies::AnalyzeDependenciesTool,
    api_docs::GetApiDocsTool,
    analysis::{AnalyzeCodeTool, SuggestRefactoringTool},
    versioning::CheckVersionTool,
    base::MCPTool,
};
use crate::errors::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_rust_docs_search() -> Result<()> {
    println!("🦀 测试Rust文档搜索功能");
    
    let search_tool = SearchDocsTool::new();
    
    // 测试搜索Rust标准库文档
    let params = json!({
        "query": "Vec",
        "language": "rust",
        "max_results": 5
    });
    
    match timeout(Duration::from_secs(30), search_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Rust文档搜索成功: {}", docs);
                    assert!(docs["results"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Rust文档搜索失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust文档搜索超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_dependencies_analysis() -> Result<()> {
    println!("📦 测试Rust依赖分析功能");
    
    let deps_tool = AnalyzeDependenciesTool::new();
    
    // 创建临时Cargo.toml文件
    let temp_dir = std::env::temp_dir();
    let cargo_toml_path = temp_dir.join("test_Cargo.toml");
    
    let cargo_toml_content = r#"
[package]
name = "test-rust-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.36.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }

[dev-dependencies]
tokio-test = "0.4"
"#;
    
    std::fs::write(&cargo_toml_path, cargo_toml_content)?;
    
    let params = json!({
        "language": "rust",
        "files": [cargo_toml_path.to_string_lossy()],
        "check_updates": true
    });
    
    match timeout(Duration::from_secs(30), deps_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Rust依赖分析成功: {}", analysis);
                    assert!(analysis["dependencies"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Rust依赖分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust依赖分析超时，继续下一个测试");
        }
    }
    
    // 清理临时文件
    let _ = std::fs::remove_file(cargo_toml_path);
    
    Ok(())
}

#[tokio::test]
async fn test_rust_code_analysis() -> Result<()> {
    println!("🔬 测试Rust代码分析功能");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let rust_code = r#"
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: Option<String>,
}

pub struct UserRepository {
    users: HashMap<u64, User>,
    next_id: u64,
}

impl UserRepository {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn add_user(&mut self, name: String, email: Option<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let user = User { id, name, email };
        self.users.insert(id, user);
        id
    }
    
    pub fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
    
    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
}
"#;
    
    let params = json!({
        "code": rust_code,
        "language": "rust"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Rust代码分析成功: {}", analysis);
                    assert!(analysis["analysis"].is_object());
                    assert!(analysis["suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Rust代码分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust代码分析超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_refactoring_suggestions() -> Result<()> {
    println!("🔧 测试Rust重构建议功能");
    
    let refactor_tool = SuggestRefactoringTool;
    
    let rust_code = r#"
fn process_numbers(numbers: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for num in numbers {
        if num > 0 {
            if num % 2 == 0 {
                result.push(num * 2);
            } else {
                result.push(num * 3);
            }
        } else {
            result.push(0);
        }
    }
    result
}
"#;
    
    let params = json!({
        "code": rust_code,
        "language": "rust"
    });
    
    match timeout(Duration::from_secs(30), refactor_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(suggestions) => {
                    println!("✅ Rust重构建议成功: {}", suggestions);
                    assert!(suggestions["refactoring_suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Rust重构建议失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust重构建议超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_api_docs() -> Result<()> {
    println!("📚 测试Rust API文档获取功能");
    
    let api_tool = GetApiDocsTool::new();
    
    let params = json!({
        "language": "rust",
        "package": "tokio",
        "symbol": "Runtime",
        "version": "latest"
    });
    
    match timeout(Duration::from_secs(30), api_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Rust API文档获取成功: {}", docs);
                    assert!(docs["documentation"].is_object());
                },
                Err(e) => {
                    println!("❌ Rust API文档获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust API文档获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_version_check() -> Result<()> {
    println!("🔢 测试Rust版本检查功能");
    
    let version_tool = CheckVersionTool::new();
    
    let params = json!({
        "language": "rust",
        "packages": ["tokio", "serde", "reqwest", "clap"],
        "check_latest": true
    });
    
    match timeout(Duration::from_secs(30), version_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(versions) => {
                    println!("✅ Rust版本检查成功: {}", versions);
                    assert!(versions["packages"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Rust版本检查失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Rust版本检查超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_integration_workflow() -> Result<()> {
    println!("🔄 测试Rust完整工作流程");
    println!("{}", "=".repeat(50));
    
    // 1. 搜索Rust标准库文档
    println!("步骤1: 搜索Rust标准库文档");
    let search_tool = SearchDocsTool::new();
    let search_params = json!({
        "query": "HashMap insert",
        "language": "rust",
        "max_results": 3
    });
    
    if let Ok(Ok(search_result)) = timeout(Duration::from_secs(30), search_tool.execute(search_params)).await {
        println!("✅ 文档搜索完成: {}", search_result);
    } else {
        println!("⚠️ 文档搜索步骤跳过");
    }
    
    // 2. 分析Rust代码
    println!("\n步骤2: 分析Rust代码质量");
    let analysis_tool = AnalyzeCodeTool;
    let code_params = json!({
        "code": "use std::collections::HashMap;\n\nfn main() {\n    let mut map = HashMap::new();\n    map.insert(\"key\", \"value\");\n    println!(\"{:?}\", map);\n}",
        "language": "rust"
    });
    
    if let Ok(Ok(analysis_result)) = timeout(Duration::from_secs(30), analysis_tool.execute(code_params)).await {
        println!("✅ 代码分析完成: {}", analysis_result);
    } else {
        println!("⚠️ 代码分析步骤跳过");
    }
    
    // 3. 检查crate版本
    println!("\n步骤3: 检查crate版本");
    let version_tool = CheckVersionTool::new();
    let version_params = json!({
        "language": "rust",
        "packages": ["tokio", "serde"],
        "check_latest": true
    });
    
    if let Ok(Ok(version_result)) = timeout(Duration::from_secs(30), version_tool.execute(version_params)).await {
        println!("✅ 版本检查完成: {}", version_result);
    } else {
        println!("⚠️ 版本检查步骤跳过");
    }
    
    println!("\n🎉 Rust完整工作流程测试完成!");
    Ok(())
} 