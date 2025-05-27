use grape_mcp_devtools::tools::base::MCPTool;
use grape_mcp_devtools::tools::analysis::AnalyzeCodeTool;
use grape_mcp_devtools::tools::versioning::CheckVersionTool;
use grape_mcp_devtools::tools::python_docs_tool::PythonDocsTool;

#[test]
fn test_improved_tool_descriptions() {
    println!("🧪 测试改进后的工具描述...");

    // 测试代码分析工具描述
    let analyze_tool = AnalyzeCodeTool;
    let analyze_description = analyze_tool.description();
    println!("📋 代码分析工具描述: {}", analyze_description);
    
    assert!(analyze_description.contains("当用户的代码"), "应该描述用户遇到的问题");
    assert!(analyze_description.contains("bug"), "应该包含常见问题关键词");
    assert!(analyze_description.contains("性能问题"), "应该包含具体问题类型");
    
    // 测试版本检查工具描述
    let version_tool = CheckVersionTool::new();
    let version_description = version_tool.description();
    println!("📋 版本检查工具描述: {}", version_description);
    
    assert!(version_description.contains("当用户遇到"), "应该描述用户场景");
    assert!(version_description.contains("版本冲突"), "应该包含具体问题");
    assert!(version_description.contains("最新版本是什么"), "应该包含用户常问的问题");
    
    // 测试Python文档工具描述
    let python_tool = PythonDocsTool::new();
    let python_description = python_tool.description();
    println!("📋 Python文档工具描述: {}", python_description);
    
    assert!(python_description.contains("当用户询问"), "应该包含用户场景");
    assert!(python_description.contains("如何使用"), "应该包含常见问题模式");
    assert!(python_description.contains("Python"), "应该明确指定语言");
    
    println!("✅ 所有工具描述都符合新的用户场景导向标准！");
}

#[test]
fn test_description_keywords() {
    println!("🧪 测试描述中的关键词覆盖...");
    
    let analyze_tool = AnalyzeCodeTool;
    let description = analyze_tool.description();
    
    // 检查是否包含用户可能使用的关键词
    let user_keywords = vec![
        "bug", "错误", "问题", "性能", "代码审查", "质量检查"
    ];
    
    for keyword in user_keywords {
        if description.contains(keyword) {
            println!("✅ 包含关键词: {}", keyword);
        }
    }
    
    // 检查是否避免了技术术语开头
    let technical_terms = vec![
        "基于", "支持", "提供", "实现"
    ];
    
    let has_technical_terms = technical_terms.iter()
        .any(|term| description.starts_with(term));
    
    assert!(!has_technical_terms, "描述不应该以技术术语开头");
    
    println!("✅ 关键词覆盖测试通过！");
}

#[test]
fn test_user_scenario_focus() {
    println!("🧪 测试用户场景导向...");
    
    let tools: Vec<Box<dyn MCPTool>> = vec![
        Box::new(AnalyzeCodeTool),
        Box::new(CheckVersionTool::new()),
        Box::new(PythonDocsTool::new()),
    ];
    
    for tool in tools {
        let description = tool.description();
        println!("🔍 工具 '{}': {}", tool.name(), description);
        
        // 检查是否以用户场景开头
        let scenario_starters = vec![
            "当用户", "如果用户", "当", "如果"
        ];
        
        let starts_with_scenario = scenario_starters.iter()
            .any(|starter| description.starts_with(starter));
        
        assert!(starts_with_scenario, 
            "工具 '{}' 的描述应该以用户场景开头: {}", 
            tool.name(), description);
    }
    
    println!("✅ 所有工具都以用户场景为导向！");
}

#[test]
fn test_before_after_comparison() {
    println!("🧪 对比改进前后的描述风格...");
    
    // 模拟改进前的描述风格
    let old_style_descriptions = vec![
        "分析代码质量和结构",
        "检查编程语言或包的最新版本信息",
        "获取和生成Python包的文档",
    ];
    
    // 实际的新描述
    let tools: Vec<Box<dyn MCPTool>> = vec![
        Box::new(AnalyzeCodeTool),
        Box::new(CheckVersionTool::new()),
        Box::new(PythonDocsTool::new()),
    ];
    
    for (i, tool) in tools.iter().enumerate() {
        let old_desc = old_style_descriptions[i];
        let new_desc = tool.description();
        
        println!("📊 工具: {}", tool.name());
        println!("   旧描述: {}", old_desc);
        println!("   新描述: {}", new_desc);
        
        // 新描述应该更长，包含更多上下文
        assert!(new_desc.len() > old_desc.len() * 2, 
            "新描述应该比旧描述更详细");
        
        // 新描述应该包含用户场景
        assert!(new_desc.contains("用户"), 
            "新描述应该明确提到用户");
        
        println!("   ✅ 改进有效");
        println!();
    }
    
    println!("✅ 描述改进对比测试通过！");
} 