// 代码分析工具测试
use crate::tools::analysis::AnalyzeCodeTool;
use crate::tools::base::MCPTool;
use serde_json::json;
use tokio;

#[tokio::test]
async fn test_analyze_code_with_todo() {
    let tool = AnalyzeCodeTool;
    let params = json!({
        "code": "fn main() {\n    // TODO: 优化此函数\n    let x = 1;\n    let y = 2;\n    let z = x + y;\n    println!(\"{}\", z);\n}",
        "language": "rust"
    });
    
    let result = tool.execute(params).await.unwrap();
    assert_eq!(result["message"], "代码分析完成");
    assert_eq!(result["analysis"]["language"], "rust");
    assert!(result["suggestions"].as_array().unwrap().len() > 0);
}
