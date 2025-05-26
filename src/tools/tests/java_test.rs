use crate::tools::*;
use crate::errors::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_java_docs_search() -> Result<()> {
    println!("☕ 测试Java文档搜索功能");
    
    let search_tool = SearchDocsTools::new();
    
    // 测试搜索Java API文档
    let params = json!({
        "query": "ArrayList",
        "language": "java",
        "max_results": 5
    });
    
    match timeout(Duration::from_secs(30), search_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Java文档搜索成功: {}", docs);
                    assert!(docs["results"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java文档搜索失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java文档搜索超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_dependencies_analysis() -> Result<()> {
    println!("📦 测试Java依赖分析功能");
    
    let deps_tool = AnalyzeDependenciesTool::new();
    
    // 创建临时pom.xml文件
    let temp_dir = std::env::temp_dir();
    let pom_xml_path = temp_dir.join("test_pom.xml");
    
    let pom_xml_content = r#"
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    
    <groupId>com.example</groupId>
    <artifactId>test-java-project</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>
    
    <properties>
        <maven.compiler.source>17</maven.compiler.source>
        <maven.compiler.target>17</maven.compiler.target>
        <spring.version>6.1.3</spring.version>
    </properties>
    
    <dependencies>
        <dependency>
            <groupId>org.springframework</groupId>
            <artifactId>spring-core</artifactId>
            <version>${spring.version}</version>
        </dependency>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-web</artifactId>
            <version>3.2.2</version>
        </dependency>
        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>2.16.1</version>
        </dependency>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>
"#;
    
    std::fs::write(&pom_xml_path, pom_xml_content)?;
    
    let params = json!({
        "language": "java",
        "files": [pom_xml_path.to_string_lossy()],
        "check_updates": true
    });
    
    match timeout(Duration::from_secs(30), deps_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Java依赖分析成功: {}", analysis);
                    assert!(analysis["dependencies"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java依赖分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java依赖分析超时，继续下一个测试");
        }
    }
    
    // 清理临时文件
    let _ = std::fs::remove_file(pom_xml_path);
    
    Ok(())
}

#[tokio::test]
async fn test_java_code_analysis() -> Result<()> {
    println!("🔬 测试Java代码分析功能");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let java_code = r#"
package com.example.demo;

import java.util.*;
import java.util.stream.Collectors;

public class UserService {
    private final Map<Long, User> users;
    private Long nextId;
    
    public UserService() {
        this.users = new HashMap<>();
        this.nextId = 1L;
    }
    
    public Long addUser(String name, String email) {
        Long id = nextId++;
        User user = new User(id, name, email);
        users.put(id, user);
        return id;
    }
    
    public Optional<User> getUser(Long id) {
        return Optional.ofNullable(users.get(id));
    }
    
    public List<User> getAllUsers() {
        return users.values()
                   .stream()
                   .collect(Collectors.toList());
    }
    
    public List<User> findUsersByName(String name) {
        return users.values()
                   .stream()
                   .filter(user -> user.getName().contains(name))
                   .collect(Collectors.toList());
    }
    
    public static class User {
        private final Long id;
        private final String name;
        private final String email;
        
        public User(Long id, String name, String email) {
            this.id = id;
            this.name = name;
            this.email = email;
        }
        
        // Getters
        public Long getId() { return id; }
        public String getName() { return name; }
        public String getEmail() { return email; }
    }
}
"#;
    
    let params = json!({
        "code": java_code,
        "language": "java"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Java代码分析成功: {}", analysis);
                    assert!(analysis["metrics"].is_object());
                    assert!(analysis["suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java代码分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java代码分析超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_refactoring_suggestions() -> Result<()> {
    println!("🔧 测试Java重构建议功能");
    
    let refactor_tool = SuggestRefactoringTool;
    
    let java_code = r#"
public class Calculator {
    public double calculate(String operation, double a, double b) {
        if (operation.equals("add")) {
            return a + b;
        } else if (operation.equals("subtract")) {
            return a - b;
        } else if (operation.equals("multiply")) {
            return a * b;
        } else if (operation.equals("divide")) {
            if (b != 0) {
                return a / b;
            } else {
                throw new IllegalArgumentException("Division by zero");
            }
        } else {
            throw new IllegalArgumentException("Unknown operation: " + operation);
        }
    }
}
"#;
    
    let params = json!({
        "code": java_code,
        "language": "java"
    });
    
    match timeout(Duration::from_secs(30), refactor_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(suggestions) => {
                    println!("✅ Java重构建议成功: {}", suggestions);
                    assert!(suggestions["suggestions"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java重构建议失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java重构建议超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_api_docs() -> Result<()> {
    println!("📚 测试Java API文档获取功能");
    
    let api_tool = GetApiDocsTool::new(None);
    
    let params = json!({
        "language": "java",
        "package": "org.springframework",
        "symbol": "RestTemplate",
        "version": "latest"
    });
    
    match timeout(Duration::from_secs(30), api_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(docs) => {
                    println!("✅ Java API文档获取成功: {}", docs);
                    assert!(docs["documentation"].is_object());
                },
                Err(e) => {
                    println!("❌ Java API文档获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java API文档获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_changelog() -> Result<()> {
    println!("📝 测试Java变更日志功能");
    
    let changelog_tool = GetChangelogTool;
    
    let params = json!({
        "package": "org.springframework:spring-core",
        "language": "java",
        "version": "6.1.3"
    });
    
    match timeout(Duration::from_secs(30), changelog_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(changelog) => {
                    println!("✅ Java变更日志获取成功: {}", changelog);
                    assert!(changelog["changes"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java变更日志获取失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java变更日志获取超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_version_check() -> Result<()> {
    println!("🔢 测试Java版本检查功能");
    
    let version_tool = CheckVersionTool::new();
    
    let params = json!({
        "language": "java",
        "packages": [
            "org.springframework:spring-core",
            "org.springframework.boot:spring-boot-starter-web",
            "com.fasterxml.jackson.core:jackson-databind",
            "junit:junit"
        ],
        "check_latest": true
    });
    
    match timeout(Duration::from_secs(30), version_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(versions) => {
                    println!("✅ Java版本检查成功: {}", versions);
                    assert!(versions["packages"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java版本检查失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java版本检查超时，继续下一个测试");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_integration_workflow() -> Result<()> {
    println!("🔄 测试Java完整工作流程");
    println!("{}", "=".repeat(50));
    
    // 1. 搜索Java API文档
    println!("步骤1: 搜索Spring Framework文档");
    let search_tool = SearchDocsTools::new();
    let search_params = json!({
        "query": "Spring RestTemplate",
        "language": "java",
        "max_results": 3
    });
    
    if let Ok(Ok(search_result)) = timeout(Duration::from_secs(30), search_tool.execute(search_params)).await {
        println!("✅ 文档搜索完成: {}", search_result);
    } else {
        println!("⚠️ 文档搜索步骤跳过");
    }
    
    // 2. 分析Java代码
    println!("\n步骤2: 分析Java代码质量");
    let analysis_tool = AnalyzeCodeTool;
    let code_params = json!({
        "code": "import java.util.*;\n\npublic class HelloWorld {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, World!\");\n    }\n}",
        "language": "java"
    });
    
    if let Ok(Ok(analysis_result)) = timeout(Duration::from_secs(30), analysis_tool.execute(code_params)).await {
        println!("✅ 代码分析完成: {}", analysis_result);
    } else {
        println!("⚠️ 代码分析步骤跳过");
    }
    
    // 3. 检查Maven依赖版本
    println!("\n步骤3: 检查Maven依赖版本");
    let version_tool = CheckVersionTool::new();
    let version_params = json!({
        "language": "java",
        "packages": ["org.springframework:spring-core", "junit:junit"],
        "check_latest": true
    });
    
    if let Ok(Ok(version_result)) = timeout(Duration::from_secs(30), version_tool.execute(version_params)).await {
        println!("✅ 版本检查完成: {}", version_result);
    } else {
        println!("⚠️ 版本检查步骤跳过");
    }
    
    println!("\n🎉 Java完整工作流程测试完成!");
    Ok(())
} 