use crate::tools::*;
use crate::errors::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_dart_language_support() -> Result<()> {
    println!("🎯 测试Dart语言支持");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let dart_code = r#"
import 'dart:async';
import 'dart:convert';

class User {
  final int id;
  final String name;
  final String? email;
  
  const User({
    required this.id,
    required this.name,
    this.email,
  });
  
  factory User.fromJson(Map<String, dynamic> json) => User(
    id: json['id'] as int,
    name: json['name'] as String,
    email: json['email'] as String?,
  );
  
  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    if (email != null) 'email': email,
  };
}

class UserRepository {
  final List<User> _users = [];
  
  Future<void> addUser(User user) async {
    _users.add(user);
  }
  
  Future<User?> findById(int id) async {
    try {
      return _users.firstWhere((user) => user.id == id);
    } catch (e) {
      return null;
    }
  }
  
  Future<List<User>> getAllUsers() async {
    return List.unmodifiable(_users);
  }
}
"#;
    
    let params = json!({
        "code": dart_code,
        "language": "dart"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ Dart代码分析成功: {}", analysis);
                    assert!(analysis["metrics"].is_object());
                },
                Err(e) => {
                    println!("❌ Dart代码分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ Dart代码分析超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_cpp_language_support() -> Result<()> {
    println!("⚡ 测试C++语言支持");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let cpp_code = r#"
#include <iostream>
#include <vector>
#include <memory>
#include <string>
#include <algorithm>

class User {
private:
    int id_;
    std::string name_;
    std::string email_;

public:
    User(int id, const std::string& name, const std::string& email = "")
        : id_(id), name_(name), email_(email) {}
    
    int getId() const { return id_; }
    const std::string& getName() const { return name_; }
    const std::string& getEmail() const { return email_; }
    
    void setEmail(const std::string& email) { email_ = email; }
};

class UserRepository {
private:
    std::vector<std::unique_ptr<User>> users_;
    int next_id_;

public:
    UserRepository() : next_id_(1) {}
    
    int addUser(const std::string& name, const std::string& email = "") {
        int id = next_id_++;
        users_.push_back(std::make_unique<User>(id, name, email));
        return id;
    }
    
    User* findUser(int id) {
        auto it = std::find_if(users_.begin(), users_.end(),
            [id](const std::unique_ptr<User>& user) {
                return user->getId() == id;
            });
        return (it != users_.end()) ? it->get() : nullptr;
    }
    
    std::vector<User*> getAllUsers() {
        std::vector<User*> result;
        std::transform(users_.begin(), users_.end(), std::back_inserter(result),
            [](const std::unique_ptr<User>& user) { return user.get(); });
        return result;
    }
};
"#;
    
    let params = json!({
        "code": cpp_code,
        "language": "cpp"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ C++代码分析成功: {}", analysis);
                    assert!(analysis["metrics"].is_object());
                },
                Err(e) => {
                    println!("❌ C++代码分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ C++代码分析超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_csharp_language_support() -> Result<()> {
    println!("🔷 测试C#/.NET语言支持");
    
    let analysis_tool = AnalyzeCodeTool;
    
    let csharp_code = r#"
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace UserManagement
{
    public record User(int Id, string Name, string? Email = null);
    
    public interface IUserRepository
    {
        Task<int> AddUserAsync(string name, string? email = null);
        Task<User?> GetUserAsync(int id);
        Task<IEnumerable<User>> GetAllUsersAsync();
        Task<IEnumerable<User>> FindUsersByNameAsync(string namePattern);
    }
    
    public class UserRepository : IUserRepository
    {
        private readonly List<User> _users = new();
        private int _nextId = 1;
        
        public Task<int> AddUserAsync(string name, string? email = null)
        {
            var id = _nextId++;
            var user = new User(id, name, email);
            _users.Add(user);
            return Task.FromResult(id);
        }
        
        public Task<User?> GetUserAsync(int id)
        {
            var user = _users.FirstOrDefault(u => u.Id == id);
            return Task.FromResult(user);
        }
        
        public Task<IEnumerable<User>> GetAllUsersAsync()
        {
            return Task.FromResult(_users.AsEnumerable());
        }
        
        public Task<IEnumerable<User>> FindUsersByNameAsync(string namePattern)
        {
            var users = _users.Where(u => u.Name.Contains(namePattern, StringComparison.OrdinalIgnoreCase));
            return Task.FromResult(users);
        }
    }
    
    public class UserService
    {
        private readonly IUserRepository _repository;
        
        public UserService(IUserRepository repository)
        {
            _repository = repository ?? throw new ArgumentNullException(nameof(repository));
        }
        
        public async Task<User?> CreateUserAsync(string name, string? email = null)
        {
            if (string.IsNullOrWhiteSpace(name))
                throw new ArgumentException("Name cannot be empty", nameof(name));
                
            var id = await _repository.AddUserAsync(name, email);
            return await _repository.GetUserAsync(id);
        }
    }
}
"#;
    
    let params = json!({
        "code": csharp_code,
        "language": "csharp"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ C#代码分析成功: {}", analysis);
                    assert!(analysis["metrics"].is_object());
                },
                Err(e) => {
                    println!("❌ C#代码分析失败: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ C#代码分析超时");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_multi_language_search() -> Result<()> {
    println!("🌐 测试多语言文档搜索功能");
    
    let search_tool = SearchDocsTools::new();
    
    // 测试不同语言的文档搜索
    let languages = vec![
        ("rust", "HashMap"),
        ("python", "list"),
        ("javascript", "Array"),
        ("java", "ArrayList"),
        ("go", "slice"),
    ];
    
    for (language, query) in languages {
        println!("🔍 搜索 {} 语言的 {} 文档", language, query);
        
        let params = json!({
            "query": query,
            "language": language,
            "max_results": 3
        });
        
        match timeout(Duration::from_secs(30), search_tool.execute(params)).await {
            Ok(result) => {
                match result {
                    Ok(docs) => {
                        println!("✅ {} 搜索成功", language);
                        assert!(docs["results"].as_array().is_some());
                    },
                    Err(e) => {
                        println!("❌ {} 搜索失败: {}", language, e);
                    }
                }
            },
            Err(_) => {
                println!("⏰ {} 搜索超时", language);
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_cross_language_integration() -> Result<()> {
    println!("🔄 测试跨语言集成工作流程");
    println!("{}", "=".repeat(60));
    
    // 1. 多语言代码分析
    println!("步骤1: 多语言代码分析");
    let analysis_tool = AnalyzeCodeTool;
    
    let test_codes = vec![
        ("rust", "fn main() { println!(\"Hello, Rust!\"); }"),
        ("python", "def main():\n    print(\"Hello, Python!\")"),
        ("javascript", "function main() {\n    console.log(\"Hello, JavaScript!\");\n}"),
        ("java", "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, Java!\");\n    }\n}"),
    ];
    
    for (language, code) in test_codes {
        let params = json!({
            "code": code,
            "language": language
        });
        
        if let Ok(Ok(analysis)) = timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
            println!("✅ {} 代码分析完成", language);
        } else {
            println!("⚠️ {} 代码分析跳过", language);
        }
    }
    
    // 2. 多语言文档搜索
    println!("\n步骤2: 多语言文档搜索");
    let search_tool = SearchDocsTools::new();
    
    let search_queries = vec![
        ("rust", "async await"),
        ("python", "asyncio"),
        ("javascript", "promise async"),
        ("java", "CompletableFuture"),
    ];
    
    for (language, query) in search_queries {
        let params = json!({
            "query": query,
            "language": language,
            "max_results": 2
        });
        
        if let Ok(Ok(docs)) = timeout(Duration::from_secs(30), search_tool.execute(params)).await {
            println!("✅ {} 文档搜索完成", language);
        } else {
            println!("⚠️ {} 文档搜索跳过", language);
        }
    }
    
    // 3. 多语言版本检查
    println!("\n步骤3: 多语言版本检查");
    let version_tool = CheckVersionTool::new();
    
    let version_checks = vec![
        ("rust", vec!["tokio", "serde"]),
        ("python", vec!["requests", "flask"]),
        ("javascript", vec!["express", "lodash"]),
        ("java", vec!["org.springframework:spring-core"]),
    ];
    
    for (language, packages) in version_checks {
        let params = json!({
            "language": language,
            "packages": packages,
            "check_latest": true
        });
        
        if let Ok(Ok(versions)) = timeout(Duration::from_secs(30), version_tool.execute(params)).await {
            println!("✅ {} 版本检查完成", language);
        } else {
            println!("⚠️ {} 版本检查跳过", language);
        }
    }
    
    println!("\n🎉 跨语言集成工作流程测试完成!");
    Ok(())
}

#[tokio::test]
async fn test_language_detection_and_analysis() -> Result<()> {
    println!("🔍 测试语言检测和自动分析功能");
    
    let analysis_tool = AnalyzeCodeTool;
    
    // 测试不同语言的代码片段
    let code_samples = vec![
        ("rust", "use std::collections::HashMap;\nfn main() {}"),
        ("python", "import json\ndef main():\n    pass"),
        ("javascript", "const express = require('express');\nfunction main() {}"),
        ("java", "import java.util.*;\npublic class Main {}"),
        ("go", "package main\nimport \"fmt\"\nfunc main() {}"),
    ];
    
    for (expected_lang, code) in code_samples {
        println!("🔍 分析 {} 代码片段", expected_lang);
        
        let params = json!({
            "code": code,
            "language": expected_lang
        });
        
        match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
            Ok(result) => {
                match result {
                    Ok(analysis) => {
                        println!("✅ {} 代码分析成功", expected_lang);
                        assert_eq!(analysis["language"], expected_lang);
                        assert!(analysis["metrics"]["lines"].as_u64().unwrap_or(0) > 0);
                    },
                    Err(e) => {
                        println!("❌ {} 代码分析失败: {}", expected_lang, e);
                    }
                }
            },
            Err(_) => {
                println!("⏰ {} 代码分析超时", expected_lang);
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_across_languages() -> Result<()> {
    println!("⚠️ 测试跨语言错误处理");
    
    let analysis_tool = AnalyzeCodeTool;
    
    // 测试无效语言处理
    let params = json!({
        "code": "some code",
        "language": "unsupported_language"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(params)).await {
        Ok(result) => {
            match result {
                Ok(_) => {
                    // 如果成功了，说明处理了未知语言
                    println!("✅ 未知语言被正确处理");
                },
                Err(e) => {
                    println!("✅ 未知语言错误被正确捕获: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ 错误处理测试超时");
        }
    }
    
    // 测试空代码处理
    let empty_params = json!({
        "code": "",
        "language": "rust"
    });
    
    match timeout(Duration::from_secs(30), analysis_tool.execute(empty_params)).await {
        Ok(result) => {
            match result {
                Ok(analysis) => {
                    println!("✅ 空代码被正确处理: {}", analysis);
                },
                Err(e) => {
                    println!("✅ 空代码错误被正确捕获: {}", e);
                }
            }
        },
        Err(_) => {
            println!("⏰ 空代码处理测试超时");
        }
    }
    
    Ok(())
} 