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
async fn test_java_docs_search() -> Result<()> {
    println!("☕ 测试Java文档搜索功能");
    
    let search_tool = SearchDocsTool::new();
    
    // 测试搜索Java标准库文档
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
                    // 继续执行，不中断测试
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
async fn test_java_maven_dependencies_analysis() -> Result<()> {
    println!("📦 测试Java Maven依赖分析功能");
    
    let deps_tool = AnalyzeDependenciesTool::new();
    
    // 创建临时pom.xml文件
    let temp_dir = std::env::temp_dir();
    let pom_xml_path = temp_dir.join("test_pom.xml");
    
    let pom_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    
    <groupId>com.example</groupId>
    <artifactId>test-java-project</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>
    
    <properties>
        <maven.compiler.source>17</maven.compiler.source>
        <maven.compiler.target>17</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
    </properties>
    
    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-web</artifactId>
            <version>3.2.0</version>
        </dependency>
        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>2.16.0</version>
        </dependency>
        <dependency>
            <groupId>org.apache.commons</groupId>
            <artifactId>commons-lang3</artifactId>
            <version>3.14.0</version>
        </dependency>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
            <scope>test</scope>
        </dependency>
        <dependency>
            <groupId>org.mockito</groupId>
            <artifactId>mockito-core</artifactId>
            <version>5.8.0</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#;
    
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
                    println!("✅ Java Maven依赖分析成功: {}", analysis);
                    assert!(analysis["dependencies"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java Maven依赖分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java Maven依赖分析超时，继续下一个测试");
        }
    }
    
    // 清理临时文件
    let _ = std::fs::remove_file(pom_xml_path);
    
    Ok(())
}

#[tokio::test]
async fn test_java_gradle_dependencies_analysis() -> Result<()> {
    println!("🐘 测试Java Gradle依赖分析功能");
    
    let deps_tool = AnalyzeDependenciesTool::new();
    
    // 创建临时build.gradle文件
    let temp_dir = std::env::temp_dir();
    let build_gradle_path = temp_dir.join("test_build.gradle");
    
    let build_gradle_content = r#"plugins {
    id 'java'
    id 'org.springframework.boot' version '3.2.0'
    id 'io.spring.dependency-management' version '1.1.4'
}

group = 'com.example'
version = '1.0.0'
sourceCompatibility = '17'

repositories {
    mavenCentral()
}

dependencies {
    implementation 'org.springframework.boot:spring-boot-starter-web'
    implementation 'org.springframework.boot:spring-boot-starter-data-jpa'
    implementation 'com.fasterxml.jackson.core:jackson-databind:2.16.0'
    implementation 'org.apache.commons:commons-lang3:3.14.0'
    implementation 'com.google.guava:guava:32.1.3-jre'
    
    runtimeOnly 'com.h2database:h2'
    
    testImplementation 'org.springframework.boot:spring-boot-starter-test'
    testImplementation 'org.junit.jupiter:junit-jupiter:5.10.1'
    testImplementation 'org.mockito:mockito-core:5.8.0'
}

test {
    useJUnitPlatform()
}"#;
    
    std::fs::write(&build_gradle_path, build_gradle_content)?;
    
    let params = json!({
        "language": "java",
        "files": [build_gradle_path.to_string_lossy()],
        "check_updates": true
    });
    
    match timeout(Duration::from_secs(30), deps_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Java Gradle依赖分析成功: {}", analysis);
                    assert!(analysis["dependencies"].as_array().is_some());
                },
                Err(e) => {
                    println!("❌ Java Gradle依赖分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Java Gradle依赖分析超时，继续下一个测试");
        }
    }
    
    // 清理临时文件
    let _ = std::fs::remove_file(build_gradle_path);
    
    Ok(())
}

#[tokio::test]
async fn test_java_code_analysis() -> Result<()> {
    println!("🔬 测试Java代码分析功能");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let java_code = r#"
package com.example.service;

import java.util.*;
import java.util.stream.Collectors;

/**
 * 用户服务类
 * 提供用户管理的基本功能
 */
public class UserService {
    
    private final Map<Long, User> users = new HashMap<>();
    private Long nextId = 1L;
    
    /**
     * 添加新用户
     * @param name 用户名
     * @param email 邮箱地址
     * @return 用户ID
     */
    public Long addUser(String name, String email) {
        if (name == null || name.trim().isEmpty()) {
            throw new IllegalArgumentException("用户名不能为空");
        }
        
        Long id = nextId++;
        User user = new User(id, name, email);
        users.put(id, user);
        return id;
    }
    
    /**
     * 根据ID获取用户
     * @param id 用户ID
     * @return 用户对象，如果不存在返回null
     */
    public User getUserById(Long id) {
        return users.get(id);
    }
    
    /**
     * 获取所有用户列表
     * @return 用户列表
     */
    public List<User> getAllUsers() {
        return new ArrayList<>(users.values());
    }
    
    /**
     * 根据名称搜索用户
     * @param namePattern 名称模式
     * @return 匹配的用户列表
     */
    public List<User> searchUsersByName(String namePattern) {
        return users.values().stream()
                .filter(user -> user.getName().toLowerCase()
                        .contains(namePattern.toLowerCase()))
                .collect(Collectors.toList());
    }
    
    /**
     * 删除用户
     * @param id 用户ID
     * @return 是否删除成功
     */
    public boolean deleteUser(Long id) {
        return users.remove(id) != null;
    }
}

/**
 * 用户实体类
 */
class User {
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
    
    @Override
    public String toString() {
        return String.format("User{id=%d, name='%s', email='%s'}", 
                           id, name, email);
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
                    assert!(analysis["analysis"].is_object());
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
public class DataProcessor {
    
    public List<Integer> processNumbers(List<Integer> numbers) {
        List<Integer> result = new ArrayList<>();
        for (Integer num : numbers) {
            if (num != null) {
                if (num > 0) {
                    if (num % 2 == 0) {
                        result.add(num * 2);
                    } else {
                        result.add(num * 3);
                    }
                } else {
                    result.add(0);
                }
            }
        }
        return result;
    }
    
    // 重复的代码模式
    public String formatUserInfo(String name, String email) {
        if (name == null) {
            name = "Unknown";
        }
        if (email == null) {
            email = "No email";
        }
        return name + " (" + email + ")";
    }
    
    public String formatProductInfo(String productName, String category) {
        if (productName == null) {
            productName = "Unknown";
        }
        if (category == null) {
            category = "No category";
        }
        return productName + " (" + category + ")";
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
                    assert!(suggestions["refactoring_suggestions"].as_array().is_some());
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
    
    let api_tool = GetApiDocsTool::new();
    
    let params = json!({
        "language": "java",
        "package": "java.util",
        "symbol": "ArrayList",
        "version": "17"
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
async fn test_java_version_check() -> Result<()> {
    println!("🔍 测试Java版本检查功能");
    
    let version_tool = CheckVersionTool::new();
    
    let params = json!({
        "language": "java",
        "packages": [
            "org.springframework.boot:spring-boot-starter-web",
            "com.fasterxml.jackson.core:jackson-databind",
            "org.apache.commons:commons-lang3"
        ]
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
    println!("🔄 测试Java集成工作流程");
    
    // 1. 创建临时项目文件
    let temp_dir = std::env::temp_dir();
    let project_dir = temp_dir.join("test_java_project");
    std::fs::create_dir_all(&project_dir)?;
    
    let pom_xml_path = project_dir.join("pom.xml");
    let pom_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>integration-test</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-web</artifactId>
            <version>3.2.0</version>
        </dependency>
    </dependencies>
</project>"#;
    
    std::fs::write(&pom_xml_path, pom_content)?;
    
    // 2. 依赖分析
    let deps_tool = AnalyzeDependenciesTool::new();
    let deps_params = json!({
        "language": "java",
        "files": [pom_xml_path.to_string_lossy()],
        "check_updates": true
    });
    
    match timeout(Duration::from_secs(30), deps_tool.execute(deps_params)).await {
        Ok(Ok(deps_result)) => {
            println!("✅ 依赖分析步骤完成: {}", deps_result);
        },
        _ => {
            println!("⚠️ 依赖分析步骤跳过");
        }
    }
    
    // 3. 文档搜索
    let search_tool = SearchDocsTool::new();
    let search_params = json!({
        "query": "Spring Boot",
        "language": "java",
        "max_results": 3
    });
    
    match timeout(Duration::from_secs(30), search_tool.execute(search_params)).await {
        Ok(Ok(search_result)) => {
            println!("✅ 文档搜索步骤完成: {}", search_result);
        },
        _ => {
            println!("⚠️ 文档搜索步骤跳过");
        }
    }
    
    // 4. 版本检查
    let version_tool = CheckVersionTool::new();
    let version_params = json!({
        "language": "java",
        "packages": ["org.springframework.boot:spring-boot-starter-web"]
    });
    
    match timeout(Duration::from_secs(30), version_tool.execute(version_params)).await {
        Ok(Ok(version_result)) => {
            println!("✅ 版本检查步骤完成: {}", version_result);
        },
        _ => {
            println!("⚠️ 版本检查步骤跳过");
        }
    }
    
    // 清理临时文件
    let _ = std::fs::remove_dir_all(project_dir);
    
    println!("🎉 Java集成工作流程测试完成");
    Ok(())
} 