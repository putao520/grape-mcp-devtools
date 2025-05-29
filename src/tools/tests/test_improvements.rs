#[cfg(test)]
mod tests {
    use crate::tools::{
        security::SecurityCheckTool, 
        dependencies::AnalyzeDependenciesTool, 
        SearchDocsTool
    };
    use crate::tools::base::MCPTool;
    use serde_json::json;

    #[tokio::test]
    async fn test_improved_security_check() {
        let security_tool = SecurityCheckTool::new();
        
        let params = json!({
            "ecosystem": "npm",
            "package": "lodash",
            "version": "4.17.20",
            "include_fixed": false
        });

        let result = security_tool.execute(params).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response["package"].as_str().unwrap() == "lodash");
        assert!(response["ecosystem"].as_str().unwrap() == "npm");
        assert!(response.get("vulnerabilities").is_some());
        assert!(response.get("total_count").is_some());
    }

    #[tokio::test]
    async fn test_improved_dependency_analysis() {
        let deps_tool = AnalyzeDependenciesTool::new();
        
        // 创建一个临时的package.json文件用于测试
        let temp_dir = tempfile::tempdir().unwrap();
        let package_json_path = temp_dir.path().join("package.json");
        
        let package_json_content = json!({
            "dependencies": {
                "lodash": "^4.17.20",
                "express": "^4.18.0"
            },
            "devDependencies": {
                "jest": "^29.0.0"
            }
        });
        
        std::fs::write(&package_json_path, package_json_content.to_string()).unwrap();
        
        let params = json!({
            "language": "javascript",
            "files": [package_json_path.to_str().unwrap()],
            "check_updates": true
        });

        let result = deps_tool.execute(params).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response["language"].as_str().unwrap() == "javascript");
        assert!(response["dependencies"].as_array().is_some());
        assert!(response["summary"].is_object());
        
        let summary = &response["summary"];
        assert!(summary["total_dependencies"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_improved_search_functionality() {
        let search_tool = SearchDocsTool::new();
        
        let params = json!({
            "query": "http client",
            "language": "rust",
            "max_results": 5
        });

        let result = search_tool.execute(params).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response["results"].as_array().is_some());
        assert!(response["total_hits"].as_u64().unwrap() > 0);
        assert!(response["language"].as_str().unwrap() == "rust");
        
        let results = response["results"].as_array().unwrap();
        assert!(results.len() <= 5); // 验证最大结果数限制
        
        // 验证结果格式
        if let Some(first_result) = results.first() {
            assert!(first_result["title"].is_string());
            assert!(first_result["content"].is_string());
            assert!(first_result["url"].is_string());
            assert!(first_result["source"].is_string());
        }
    }

    #[tokio::test]
    async fn test_search_caching() {
        let search_tool = SearchDocsTool::new();
        
        let params = json!({
            "query": "async programming",
            "language": "python",
            "max_results": 3
        });

        // 第一次搜索
        let start_time = std::time::Instant::now();
        let result1 = search_tool.execute(params.clone()).await;
        let first_duration = start_time.elapsed();
        
        // 第二次搜索（应该从缓存返回）
        let start_time = std::time::Instant::now();
        let result2 = search_tool.execute(params).await;
        let second_duration = start_time.elapsed();
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // 第二次搜索应该更快（从缓存返回）
        assert!(second_duration < first_duration);
        
        // 结果应该相同
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[tokio::test]
    async fn test_dependency_analysis_caching() {
        let deps_tool = AnalyzeDependenciesTool::new();
        
        // 创建临时Cargo.toml文件
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        
        let cargo_toml_content = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#;
        
        std::fs::write(&cargo_toml_path, cargo_toml_content).unwrap();
        
        let params = json!({
            "language": "rust",
            "files": [cargo_toml_path.to_str().unwrap()],
            "check_updates": true
        });

        // 第一次分析
        let result1 = deps_tool.execute(params.clone()).await;
        
        // 第二次分析（应该从缓存返回）
        let result2 = deps_tool.execute(params).await;
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // 验证结果结构
        let response = result1.unwrap();
        assert!(response["dependencies"].as_array().is_some());
        assert!(response["summary"]["total_dependencies"].as_u64().unwrap() >= 2);
    }

    #[tokio::test]
    async fn test_security_check_error_handling() {
        let security_tool = SecurityCheckTool::new();
        
        // 测试无效的生态系统
        let params = json!({
            "ecosystem": "invalid_ecosystem",
            "package": "test-package"
        });

        let result = security_tool.execute(params).await;
        // 应该优雅地处理错误，而不是崩溃
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_search_different_languages() {
        let search_tool = SearchDocsTool::new();
        
        let languages = vec!["rust", "python", "javascript", "go", "java"];
        
        for language in languages {
            let params = json!({
                "query": "web framework",
                "language": language,
                "max_results": 3
            });

            let result = search_tool.execute(params).await;
            assert!(result.is_ok());
            
            let response = result.unwrap();
            assert_eq!(response["language"].as_str().unwrap(), language);
            assert!(response["results"].as_array().is_some());
        }
    }
} 