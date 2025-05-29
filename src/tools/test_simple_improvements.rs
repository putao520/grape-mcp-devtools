#[cfg(test)]
mod simple_improvement_tests {
    use crate::tools::{
        SearchDocsTool,
        dependencies::AnalyzeDependenciesTool,
        security::SecurityCheckTool,
        base::MCPTool,
    };
    use serde_json::json;

    #[tokio::test]
    async fn test_security_tool_basic() {
        let security_tool = SecurityCheckTool::new();
        
        let params = json!({
            "ecosystem": "npm",
            "package": "lodash"
        });

        let result = security_tool.execute(params).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response["package"].as_str().unwrap() == "lodash");
        assert!(response["ecosystem"].as_str().unwrap() == "npm");
    }

    #[tokio::test]
    async fn test_search_tool_basic() {
        let search_tool = SearchDocsTool::new();
        
        let params = json!({
            "query": "http client",
            "language": "rust"
        });

        let result = search_tool.execute(params).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response["results"].as_array().is_some());
        assert!(response["language"].as_str().unwrap() == "rust");
    }

    #[tokio::test]
    async fn test_dependency_tool_basic() {
        let deps_tool = AnalyzeDependenciesTool::new();
        
        // 创建一个临时的Cargo.toml文件
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        
        let cargo_toml_content = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#;
        
        std::fs::write(&cargo_toml_path, cargo_toml_content).unwrap();
        
        let params = json!({
            "language": "rust",
            "files": [cargo_toml_path.to_str().unwrap()]
        });

        let result = deps_tool.execute(params).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response["language"].as_str().unwrap() == "rust");
        assert!(response["dependencies"].as_array().is_some());
    }
} 