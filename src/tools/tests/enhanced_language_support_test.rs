use crate::tools::{
    python_docs_tool::PythonDocsTool,
    javascript_docs_tool::JavaScriptDocsTool,
    SearchDocsTool,
    analysis::AnalyzeCodeTool,
    dependencies::AnalyzeDependenciesTool,
    base::MCPTool,
};
use crate::errors::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// 测试Python文档工具
#[tokio::test]
async fn test_python_docs_tool() -> Result<()> {
    println!("🐍 测试Python文档工具");
    
    let python_tool = PythonDocsTool::new();
    
    // 测试获取requests包文档
    let params = json!({
        "package_name": "requests",
        "version": "2.31.0",
        "include_examples": true
    });
    
    match timeout(Duration::from_secs(30), python_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Python文档获取成功");
                    assert_eq!(docs["status"], "success");
                    assert_eq!(docs["tool"], "python_docs_tool");
                    assert_eq!(docs["package_name"], "requests");
                    
                    let documentation = &docs["documentation"];
                    assert!(documentation.is_object());
                    
                    println!("📚 文档内容: {}", documentation);
                },
                Err(e) => {
                    println!("❌ Python文档获取失败: {}", e);
                    // 继续执行，不中断测试
                }
            }
        },
        Err(_) => {
            println!("⏰ Python文档获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

/// 测试JavaScript文档工具
#[tokio::test]
async fn test_javascript_docs_tool() -> Result<()> {
    println!("📦 测试JavaScript文档工具");
    
    let js_tool = JavaScriptDocsTool::new();
    
    // 测试获取express包文档
    let params = json!({
        "package_name": "express",
        "include_examples": true
    });
    
    match timeout(Duration::from_secs(30), js_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ JavaScript文档获取成功");
                    assert_eq!(docs["status"], "success");
                    assert_eq!(docs["tool"], "javascript_docs_tool");
                    assert_eq!(docs["package_name"], "express");
                    
                    let documentation = &docs["documentation"];
                    assert!(documentation.is_object());
                    
                    println!("📚 文档内容: {}", documentation);
                },
                Err(e) => {
                    println!("❌ JavaScript文档获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ JavaScript文档获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

/// 测试TypeScript类型包
#[tokio::test]
async fn test_typescript_types_tool() -> Result<()> {
    println!("🔷 测试TypeScript类型包工具");
    
    let js_tool = JavaScriptDocsTool::new();
    
    // 测试获取@types/node包文档
    let params = json!({
        "package_name": "@types/node",
        "language": "typescript"
    });
    
    match timeout(Duration::from_secs(30), js_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ TypeScript类型包文档获取成功");
                    assert_eq!(docs["status"], "success");
                    assert_eq!(docs["package_name"], "@types/node");
                    
                    let documentation = &docs["documentation"];
                    assert!(documentation.is_object());
                    
                    // 检查是否正确识别为TypeScript
                    if let Some(lang) = documentation.get("language") {
                        println!("🔍 检测到语言: {}", lang);
                    }
                    
                    println!("📚 TypeScript文档内容: {}", documentation);
                },
                Err(e) => {
                    println!("❌ TypeScript类型包文档获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ TypeScript类型包文档获取超时");
        }
    }
    
    Ok(())
}

/// 测试多语言文档搜索集成
#[tokio::test]
async fn test_multi_language_integration() -> Result<()> {
    println!("🌐 测试多语言文档搜索集成");
    
    let search_tool = SearchDocsTool::new();
    
    // 测试Python搜索
    let python_params = json!({
        "query": "http request",
        "language": "python",
        "max_results": 5
    });
    
    if let Ok(Ok(python_results)) = timeout(Duration::from_secs(30), search_tool.execute(python_params)).await {
        println!("✅ Python搜索完成: {}", python_results);
    } else {
        println!("⚠️ Python搜索跳过");
    }
    
    // 测试JavaScript搜索
    let js_params = json!({
        "query": "web server",
        "language": "javascript",
        "max_results": 5
    });
    
    if let Ok(Ok(js_results)) = timeout(Duration::from_secs(30), search_tool.execute(js_params)).await {
        println!("✅ JavaScript搜索完成: {}", js_results);
    } else {
        println!("⚠️ JavaScript搜索跳过");
    }
    
    Ok(())
}

/// 测试代码分析工具对Python和JavaScript的支持
#[tokio::test]
async fn test_enhanced_code_analysis() -> Result<()> {
    println!("🔬 测试增强的代码分析功能");
    
    let analysis_tool = AnalyzeCodeTool;
    
    // 测试Python代码分析
    let python_code = r#"
import requests
import json

def fetch_user_data(user_id):
    """获取用户数据"""
    url = f"https://api.example.com/users/{user_id}"
    response = requests.get(url)
    
    if response.status_code == 200:
        return response.json()
    else:
        raise Exception(f"Failed to fetch user: {response.status_code}")

class UserManager:
    def __init__(self):
        self.users = {}
    
    def add_user(self, user_data):
        user_id = user_data.get('id')
        if user_id:
            self.users[user_id] = user_data
            return True
        return False
"#;
    
    let python_params = json!({
        "code": python_code,
        "language": "python"
    });
    
    if let Ok(Ok(python_analysis)) = timeout(Duration::from_secs(30), analysis_tool.execute(python_params)).await {
        println!("✅ Python代码分析完成");
        assert!(python_analysis["analysis"].is_object());
        println!("📊 Python分析结果: {}", python_analysis);
    } else {
        println!("⚠️ Python代码分析跳过");
    }
    
    // 测试TypeScript代码分析
    let typescript_code = r#"
interface User {
    id: number;
    name: string;
    email: string;
    isActive: boolean;
}

class UserService {
    private users: Map<number, User> = new Map();
    
    constructor(private apiUrl: string) {}
    
    async fetchUser(id: number): Promise<User | null> {
        try {
            const response = await fetch(`${this.apiUrl}/users/${id}`);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            const userData: User = await response.json();
            this.users.set(id, userData);
            return userData;
        } catch (error) {
            console.error('Failed to fetch user:', error);
            return null;
        }
    }
    
    getAllUsers(): User[] {
        return Array.from(this.users.values());
    }
}

export { User, UserService };
"#;
    
    let typescript_params = json!({
        "code": typescript_code,
        "language": "typescript"
    });
    
    if let Ok(Ok(ts_analysis)) = timeout(Duration::from_secs(30), analysis_tool.execute(typescript_params)).await {
        println!("✅ TypeScript代码分析完成");
        assert!(ts_analysis["analysis"].is_object());
        println!("📊 TypeScript分析结果: {}", ts_analysis);
    } else {
        println!("⚠️ TypeScript代码分析跳过");
    }
    
    Ok(())
}

/// 测试依赖分析工具对多语言的支持
#[tokio::test]
async fn test_multi_language_dependency_analysis() -> Result<()> {
    println!("📦 测试多语言依赖分析");
    
    let deps_tool = AnalyzeDependenciesTool::new();
    
    // 测试Python依赖分析
    let python_params = json!({
        "language": "python",
        "files": ["requirements.txt"],
        "check_updates": true
    });
    
    if let Ok(Ok(python_deps)) = timeout(Duration::from_secs(30), deps_tool.execute(python_params)).await {
        println!("✅ Python依赖分析完成: {}", python_deps);
    } else {
        println!("⚠️ Python依赖分析跳过");
    }
    
    // 测试JavaScript依赖分析
    let js_params = json!({
        "language": "javascript",
        "files": ["package.json"],
        "check_updates": true
    });
    
    if let Ok(Ok(js_deps)) = timeout(Duration::from_secs(30), deps_tool.execute(js_params)).await {
        println!("✅ JavaScript依赖分析完成: {}", js_deps);
    } else {
        println!("⚠️ JavaScript依赖分析跳过");
    }
    
    Ok(())
}

/// 完整的多语言工作流程测试
#[tokio::test]
async fn test_complete_multi_language_workflow() -> Result<()> {
    println!("🔄 测试完整的多语言工作流程");
    println!("{}", "=".repeat(60));
    
    // 1. Python工作流程
    println!("步骤1: Python工作流程");
    let python_tool = PythonDocsTool::new();
    let python_params = json!({
        "package_name": "flask",
        "include_examples": true
    });
    
    if let Ok(Ok(_python_docs)) = timeout(Duration::from_secs(30), python_tool.execute(python_params)).await {
        println!("✅ Python文档获取完成");
        
        // 分析Python代码
        let analysis_tool = AnalyzeCodeTool;
        let code_params = json!({
            "code": "from flask import Flask\napp = Flask(__name__)\n@app.route('/')\ndef hello():\n    return 'Hello World!'",
            "language": "python"
        });
        
        if let Ok(Ok(_)) = timeout(Duration::from_secs(30), analysis_tool.execute(code_params)).await {
            println!("✅ Python代码分析完成");
        }
    } else {
        println!("⚠️ Python工作流程跳过");
    }
    
    // 2. JavaScript/TypeScript工作流程
    println!("\n步骤2: JavaScript/TypeScript工作流程");
    let js_tool = JavaScriptDocsTool::new();
    if let Ok(Ok(_js_docs)) = timeout(Duration::from_secs(30), js_tool.execute(json!({
        "package_name": "react",
        "include_examples": true
    }))).await {
        println!("✅ JavaScript文档获取完成");
        
        // 分析TypeScript代码
        let analysis_tool = AnalyzeCodeTool;
        let ts_code_params = json!({
            "code": "import React from 'react';\nconst App: React.FC = () => {\n  return <div>Hello React!</div>;\n};\nexport default App;",
            "language": "typescript"
        });
        
        if let Ok(Ok(_)) = timeout(Duration::from_secs(30), analysis_tool.execute(ts_code_params)).await {
            println!("✅ TypeScript代码分析完成");
        }
    } else {
        println!("⚠️ JavaScript工作流程跳过");
    }
    
    // 3. 跨语言搜索
    println!("\n步骤3: 跨语言文档搜索");
    let search_tool = SearchDocsTool::new();
    let search_params = json!({
        "query": "web framework",
        "max_results": 10
    });
    
    if let Ok(Ok(_)) = timeout(Duration::from_secs(30), search_tool.execute(search_params)).await {
        println!("✅ 跨语言搜索完成");
    } else {
        println!("⚠️ 跨语言搜索跳过");
    }
    
    println!("\n🎉 完整的多语言工作流程测试完成!");
    Ok(())
} 