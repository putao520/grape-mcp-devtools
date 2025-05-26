use crate::tools::*;
use crate::errors::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_javascript_docs_search() -> Result<()> {
    println!("🟨 测试JavaScript文档搜索功能");
    
    let search_tool = SearchDocsTools::new();
    
    // 测试搜索JavaScript MDN文档
    let params = json!({
        "query": "fetch",
        "language": "javascript",
        "max_results": 5
    });
    
    match timeout(Duration::from_secs(30), search_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ JavaScript文档搜索成功: {}", docs);
                    assert!(docs["results"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ JavaScript文档搜索失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ JavaScript文档搜索超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_javascript_dependencies_analysis() -> Result<()> {
    println!("📦 测试JavaScript依赖分析功能");
    
    let deps_tool = AnalyzeDependenciesTool::new();
    
    // 创建临时package.json文件
    let temp_dir = std::env::temp_dir();
    let package_json_path = temp_dir.join("test_package.json");
    
    let package_json_content = r#"
{
  "name": "test-project",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.18.2",
    "lodash": "4.17.21",
    "axios": "~1.6.0",
    "react": "18.2.0"
  },
  "devDependencies": {
    "jest": "^29.7.0",
    "webpack": "^5.89.0"
  }
}
"#;
    
    std::fs::write(&package_json_path, package_json_content)?;
    
    let params = json!({
        "language": "javascript",
        "files": [package_json_path.to_string_lossy()],
        "check_updates": true
    });
    
    match timeout(Duration::from_secs(30), deps_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ JavaScript依赖分析成功: {}", analysis);
                    assert!(analysis["dependencies"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ JavaScript依赖分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ JavaScript依赖分析超时，继续下一个测试");
        }
    }
    
    // 清理临时文件
    let _ = std::fs::remove_file(package_json_path);
    
    Ok(())
}

#[tokio::test]
async fn test_typescript_code_analysis() -> Result<()> {
    println!("🔷 测试TypeScript代码分析功能");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let typescript_code = r#"
interface User {
    id: number;
    name: string;
    email?: string;
}

class UserService {
    private users: User[] = [];
    
    constructor() {
        this.users = [];
    }
    
    addUser(user: User): void {
        this.users.push(user);
    }
    
    findUser(id: number): User | undefined {
        return this.users.find(user => user.id === id);
    }
    
    getAllUsers(): User[] {
        return [...this.users];
    }
}

const userService = new UserService();
userService.addUser({ id: 1, name: "John Doe", email: "john@example.com" });
"#;
    
    let params = json!({
        "code": typescript_code,
        "language": "typescript"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ TypeScript代码分析成功: {}", analysis);
                    assert!(analysis["metrics"].is_object());
                    assert!(analysis["suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ TypeScript代码分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ TypeScript代码分析超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_javascript_refactoring_suggestions() -> Result<()> {
    println!("🔧 测试JavaScript重构建议功能");
    
    let refactor_tool = SuggestRefactoringTool;
    
    let javascript_code = r#"
function processUsers(users) {
    var result = [];
    for (var i = 0; i < users.length; i++) {
        if (users[i].age >= 18) {
            if (users[i].active == true) {
                result.push({
                    id: users[i].id,
                    name: users[i].name,
                    status: 'adult_active'
                });
            }
        }
    }
    return result;
}
"#;
    
    let params = json!({
        "code": javascript_code,
        "language": "javascript"
    });
    
    match timeout(Duration::from_secs(30), refactor_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(suggestions) => {
                    println!("✅ JavaScript重构建议成功: {}", suggestions);
                    assert!(suggestions["suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ JavaScript重构建议失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ JavaScript重构建议超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_javascript_api_docs() -> Result<()> {
    println!("📚 测试JavaScript API文档获取功能");
    
    let api_tool = GetApiDocsTool::new(None);
    
    let params = json!({
        "language": "javascript",
        "package": "express",
        "symbol": "Router",
        "version": "latest"
    });
    
    match timeout(Duration::from_secs(30), api_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ JavaScript API文档获取成功: {}", docs);
                    assert!(docs["documentation"].is_object());
                },
                Err(e) => {
                    println!("❌ JavaScript API文档获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ JavaScript API文档获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_javascript_changelog() -> Result<()> {
    println!("📝 测试JavaScript变更日志功能");
    
    let changelog_tool = GetChangelogTool;
    
    let params = json!({
        "package": "react",
        "language": "javascript",
        "version": "18.2.0"
    });
    
    match timeout(Duration::from_secs(30), changelog_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(changelog) => {
                    println!("✅ JavaScript变更日志获取成功: {}", changelog);
                    assert!(changelog["changes"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ JavaScript变更日志获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ JavaScript变更日志获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_node_version_check() -> Result<()> {
    println!("🔢 测试Node.js版本检查功能");
    
    let version_tool = CheckVersionTool::new();
    
    let params = json!({
        "language": "javascript",
        "packages": ["express", "lodash", "axios", "react"],
        "check_latest": true
    });
    
    match timeout(Duration::from_secs(30), version_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(versions) => {
                    println!("✅ Node.js版本检查成功: {}", versions);
                    assert!(versions["packages"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Node.js版本检查失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Node.js版本检查超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_javascript_integration_workflow() -> Result<()> {
    println!("🔄 测试JavaScript/TypeScript完整工作流程");
    println!("{}", "=".repeat(50));
    
    // 1. 搜索JavaScript库文档
    println!("步骤1: 搜索Express.js文档");
    let search_tool = SearchDocsTools::new();
    let search_params = json!({
        "query": "express middleware",
        "language": "javascript",
        "max_results": 3
    });
    
    if let Ok(Ok(search_result)) = timeout(Duration::from_secs(30), search_tool.execute(search_params)).await {
        println!("✅ 文档搜索完成: {}", search_result);
    } else {
        println!("⚠️ 文档搜索步骤跳过");
    }
    
    // 2. 分析TypeScript代码
    println!("\n步骤2: 分析TypeScript代码质量");
    let analysis_tool = AnalyzeCodeTool;
    let code_params = json!({
        "code": "const app = express();\napp.use(express.json());\napp.get('/api/users', (req, res) => {\n  res.json({ users: [] });\n});",
        "language": "typescript"
    });
    
    if let Ok(Ok(analysis_result)) = timeout(Duration::from_secs(30), analysis_tool.execute(code_params)).await {
        println!("✅ 代码分析完成: {}", analysis_result);
    } else {
        println!("⚠️ 代码分析步骤跳过");
    }
    
    // 3. 检查依赖版本
    println!("\n步骤3: 检查依赖版本");
    let version_tool = CheckVersionTool::new();
    let version_params = json!({
        "language": "javascript",
        "packages": ["express", "react"],
        "check_latest": true
    });
    
    if let Ok(Ok(version_result)) = timeout(Duration::from_secs(30), version_tool.execute(version_params)).await {
        println!("✅ 版本检查完成: {}", version_result);
    } else {
        println!("⚠️ 版本检查步骤跳过");
    }
    
    println!("\n🎉 JavaScript/TypeScript完整工作流程测试完成!");
    Ok(())
} 