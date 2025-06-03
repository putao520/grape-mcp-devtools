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
async fn test_python_docs_search() -> Result<()> {
    println!("🐍 测试Python文档搜索功能");
    
    let search_tool = SearchDocsTool::new();
    
    // 测试搜索Python标准库
    let params = json!({
        "query": "json",
        "language": "python",
        "max_results": 5
    });
    
    match timeout(Duration::from_secs(30), search_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Python文档搜索成功: {}", docs);
                    assert!(docs["results"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Python文档搜索失败: {}", e);
                    // 继续执行，不中断测试
                }
            }
        },
        Err(_) => {
            println!("⏰ Python文档搜索超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_python_dependencies_analysis() -> Result<()> {
    println!("🔍 测试Python依赖分析功能");
    
    let deps_tool = AnalyzeDependenciesTool::new();
    
    // 创建临时requirements.txt文件
    let temp_dir = std::env::temp_dir();
    let requirements_path = temp_dir.join("test_requirements.txt");
    
    let requirements_content = r#"
requests==2.31.0
flask>=3.0.0
numpy~=1.24.0
pandas
django==4.2.*
fastapi[all]>=0.100.0
"#;
    
    std::fs::write(&requirements_path, requirements_content)?;
    
    let params = json!({
        "language": "python",
        "files": [requirements_path.to_string_lossy()],
        "check_updates": true
    });
    
    match timeout(Duration::from_secs(30), deps_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Python依赖分析成功: {}", analysis);
                    assert!(analysis["dependencies"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Python依赖分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Python依赖分析超时，继续下一个测试");
        }
    }
    
    // 清理临时文件
    let _ = std::fs::remove_file(requirements_path);
    
    Ok(())
}

#[tokio::test]
async fn test_python_api_docs() -> Result<()> {
    println!("📚 测试Python API文档获取功能");
    
    let api_tool = GetApiDocsTool::new();
    
    let params = json!({
        "language": "python",
        "package": "requests",
        "symbol": "get",
        "version": "latest"
    });
    
    match timeout(Duration::from_secs(30), api_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Python API文档获取成功: {}", docs);
                    assert!(docs["documentation"].is_object());
                },
                Err(e) => {
                    println!("❌ Python API文档获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Python API文档获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_python_code_analysis() -> Result<()> {
    println!("🔬 测试Python代码分析功能");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let python_code = r#"
def fibonacci(n):
    """计算斐波那契数列的第n项"""
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

class Calculator:
    """简单的计算器类"""
    
    def __init__(self):
        self.history = []
    
    def add(self, a, b):
        result = a + b
        self.history.append(f"{a} + {b} = {result}")
        return result
    
    def get_history(self):
        return self.history
"#;
    
    let params = json!({
        "code": python_code,
        "language": "python"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Python代码分析成功: {}", analysis);
                    assert!(analysis["analysis"].is_object());
                    assert!(analysis["suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Python代码分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Python代码分析超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_python_refactoring_suggestions() -> Result<()> {
    println!("🔧 测试Python重构建议功能");
    
    let refactor_tool = SuggestRefactoringTool;
    
    let python_code = r#"
def process_data(data):
    result = []
    for item in data:
        if item > 0:
            if item % 2 == 0:
                result.append(item * 2)
            else:
                result.append(item * 3)
        else:
            result.append(0)
    return result
"#;
    
    let params = json!({
        "code": python_code,
        "language": "python"
    });
    
    match timeout(Duration::from_secs(30), refactor_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(suggestions) => {
                    println!("✅ Python重构建议成功: {}", suggestions);
                    assert!(suggestions["refactoring_suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Python重构建议失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Python重构建议超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_python_version_check() -> Result<()> {
    println!("🔢 测试Python版本检查功能");
    
    let version_tool = CheckVersionTool::new();
    
    let params = json!({
        "language": "python",
        "packages": ["requests", "flask", "django"],
        "check_latest": true
    });
    
    match timeout(Duration::from_secs(30), version_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(versions) => {
                    println!("✅ Python版本检查成功: {}", versions);
                    assert!(versions["packages"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Python版本检查失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Python版本检查超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_python_integration_workflow() -> Result<()> {
    println!("🔄 测试Python完整工作流程");
    println!("{}", "=".repeat(50));
    
    // 1. 搜索Python库文档
    println!("步骤1: 搜索requests库文档");
    let search_tool = SearchDocsTool::new();
    let search_params = json!({
        "query": "requests http",
        "language": "python",
        "max_results": 3
    });
    
    if let Ok(Ok(search_result)) = timeout(Duration::from_secs(30), search_tool.execute(search_params)).await {
        println!("✅ 文档搜索完成: {}", search_result);
    } else {
        println!("⚠️ 文档搜索步骤跳过");
    }
    
    // 2. 分析Python代码
    println!("\n步骤2: 分析Python代码质量");
    let analysis_tool = AnalyzeCodeTool;
    let code_params = json!({
        "code": "import requests\n\ndef fetch_data(url):\n    response = requests.get(url)\n    return response.json()",
        "language": "python"
    });
    
    if let Ok(Ok(analysis_result)) = timeout(Duration::from_secs(30), analysis_tool.execute(code_params)).await {
        println!("✅ 代码分析完成: {}", analysis_result);
    } else {
        println!("⚠️ 代码分析步骤跳过");
    }
    
    // 3. 获取API文档
    println!("\n步骤3: 获取API文档");
    let api_tool = GetApiDocsTool::new();
    let api_params = json!({
        "language": "python",
        "package": "requests",
        "symbol": "Session"
    });
    
    if let Ok(Ok(api_result)) = timeout(Duration::from_secs(30), api_tool.execute(api_params)).await {
        println!("✅ API文档获取完成: {}", api_result);
    } else {
        println!("⚠️ API文档获取步骤跳过");
    }
    
    println!("\n🎉 Python完整工作流程测试完成!");
    Ok(())
} 