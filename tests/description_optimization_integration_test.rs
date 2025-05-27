use grape_mcp_devtools::tools::base::MCPTool;
use grape_mcp_devtools::tools::python_docs_tool::PythonDocsTool;
use grape_mcp_devtools::tools::javascript_docs_tool::JavaScriptDocsTool;
use grape_mcp_devtools::tools::typescript_docs_tool::TypeScriptDocsTool;
use grape_mcp_devtools::tools::versioning::CheckVersionTool;
use grape_mcp_devtools::tools::search::SearchDocsTools;
use grape_mcp_devtools::tools::dependencies::AnalyzeDependenciesTool;
use grape_mcp_devtools::tools::changelog::{GetChangelogTool, CompareVersionsTool};
use grape_mcp_devtools::tools::analysis::{AnalyzeCodeTool, SuggestRefactoringTool};

#[test]
fn test_all_tools_use_llm_context_descriptions() {
    // 测试所有工具描述都以"当LLM需要"开头，符合新的描述模式
    let tools: Vec<Box<dyn MCPTool>> = vec![
        Box::new(PythonDocsTool::new()),
        Box::new(JavaScriptDocsTool::new()),
        Box::new(TypeScriptDocsTool::new()),
        Box::new(CheckVersionTool::new()),
        Box::new(SearchDocsTools::new()),
        Box::new(AnalyzeDependenciesTool::new()),
        Box::new(GetChangelogTool::new()),
        Box::new(CompareVersionsTool::new()),
        Box::new(AnalyzeCodeTool),
        Box::new(SuggestRefactoringTool),
    ];

    for tool in tools {
        let description = tool.description();
        assert!(
            description.starts_with("当LLM需要"),
            "工具 {} 的描述应该以'当LLM需要'开头，实际描述: {}",
            tool.name(),
            description
        );
    }
}

#[test]
fn test_core_tools_contain_package_keywords() {
    // 测试核心工具描述包含库/包相关的关键词
    let core_tools: Vec<(Box<dyn MCPTool>, Vec<&str>)> = vec![
        (Box::new(PythonDocsTool::new()), vec!["Python包", "功能", "安装方法", "使用示例"]),
        (Box::new(JavaScriptDocsTool::new()), vec!["JavaScript", "包", "功能", "安装配置"]),
        (Box::new(TypeScriptDocsTool::new()), vec!["TypeScript包", "类型定义", "使用方法"]),
        (Box::new(CheckVersionTool::new()), vec!["包", "版本", "最新版本", "版本历史"]),
        (Box::new(GetChangelogTool::new()), vec!["包", "版本更新", "变更日志"]),
        (Box::new(CompareVersionsTool::new()), vec!["包", "版本", "差异", "升级"]),
    ];

    for (tool, keywords) in core_tools {
        let description = tool.description();
        for keyword in keywords {
            assert!(
                description.contains(keyword),
                "工具 {} 的描述应该包含关键词 '{}'，实际描述: {}",
                tool.name(),
                keyword,
                description
            );
        }
    }
}

#[test]
fn test_no_old_style_user_patterns() {
    // 测试确保没有旧式的"当用户..."描述模式
    let tools: Vec<Box<dyn MCPTool>> = vec![
        Box::new(PythonDocsTool::new()),
        Box::new(JavaScriptDocsTool::new()),
        Box::new(TypeScriptDocsTool::new()),
        Box::new(CheckVersionTool::new()),
        Box::new(SearchDocsTools::new()),
        Box::new(AnalyzeDependenciesTool::new()),
        Box::new(GetChangelogTool::new()),
        Box::new(CompareVersionsTool::new()),
        Box::new(AnalyzeCodeTool),
        Box::new(SuggestRefactoringTool),
    ];

    let old_patterns = vec![
        "当用户询问",
        "当用户遇到",
        "当用户觉得",
        "当用户的代码",
        "当用户想要",
    ];

    for tool in tools {
        let description = tool.description();
        for pattern in &old_patterns {
            assert!(
                !description.contains(pattern),
                "工具 {} 的描述不应该包含旧式模式 '{}'，实际描述: {}",
                tool.name(),
                pattern,
                description
            );
        }
    }
}

#[test]
fn test_descriptions_contain_action_keywords() {
    // 测试描述包含清晰的行动关键词
    let action_keywords = vec![
        "获取", "了解", "查询", "分析", "提供", "检查", "搜索", "比较"
    ];

    let tools: Vec<Box<dyn MCPTool>> = vec![
        Box::new(PythonDocsTool::new()),
        Box::new(JavaScriptDocsTool::new()),
        Box::new(TypeScriptDocsTool::new()),
        Box::new(CheckVersionTool::new()),
        Box::new(SearchDocsTools::new()),
        Box::new(AnalyzeDependenciesTool::new()),
        Box::new(GetChangelogTool::new()),
        Box::new(CompareVersionsTool::new()),
        Box::new(AnalyzeCodeTool),
        Box::new(SuggestRefactoringTool),
    ];

    for tool in tools {
        let description = tool.description();
        let has_action_keyword = action_keywords.iter().any(|keyword| description.contains(keyword));
        
        assert!(
            has_action_keyword,
            "工具 {} 的描述应该包含至少一个行动关键词 {:?}，实际描述: {}",
            tool.name(),
            action_keywords,
            description
        );
    }
}

#[test]
fn test_description_length_is_appropriate() {
    // 测试描述长度合理
    let tools: Vec<Box<dyn MCPTool>> = vec![
        Box::new(PythonDocsTool::new()),
        Box::new(JavaScriptDocsTool::new()),
        Box::new(TypeScriptDocsTool::new()),
        Box::new(CheckVersionTool::new()),
        Box::new(SearchDocsTools::new()),
        Box::new(AnalyzeDependenciesTool::new()),
        Box::new(GetChangelogTool::new()),
        Box::new(CompareVersionsTool::new()),
        Box::new(AnalyzeCodeTool),
        Box::new(SuggestRefactoringTool),
    ];

    for tool in tools {
        let description = tool.description();
        let length = description.chars().count();
        
        assert!(
            length >= 30,
            "工具 {} 的描述太短 ({} 字符): {}",
            tool.name(),
            length,
            description
        );
        
        assert!(
            length <= 200,
            "工具 {} 的描述太长 ({} 字符): {}",
            tool.name(),
            length,
            description
        );
    }
} 