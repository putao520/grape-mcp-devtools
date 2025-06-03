use anyhow::Result;
use serde_json::json;
use tracing::{info, error};

use grape_mcp_devtools::language_features::smart_url_analyzer::{
    SmartUrlAnalyzer, AnalysisConfig, AnalysisContext, SearchIntent
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    info!("🔍 启动智能URL分析工具测试");
    
    // 创建分析器
    let config = AnalysisConfig::default();
    let analyzer = SmartUrlAnalyzer::new(config).await?;
    
    // 测试用例：不同类型的URL
    let test_cases = vec![
        // Rust文档
        ("https://doc.rust-lang.org/std/", "rust", "官方Rust标准库文档"),
        ("https://docs.rs/tokio/latest/tokio/", "rust", "Tokio异步运行时文档"),
        ("https://docs.rs/serde/latest/serde/", "rust", "Serde序列化库文档"),
        
        // Python文档
        ("https://docs.python.org/3/library/", "python", "Python官方库文档"),
        ("https://requests.readthedocs.io/en/latest/", "python", "Requests库文档"),
        ("https://django-doc.readthedocs.io/", "python", "Django框架文档"),
        
        // JavaScript文档
        ("https://developer.mozilla.org/en-US/docs/Web/JavaScript", "javascript", "MDN JavaScript文档"),
        ("https://nodejs.org/api/", "javascript", "Node.js API文档"),
        ("https://reactjs.org/docs/", "javascript", "React框架文档"),
        
        // Java文档
        ("https://docs.oracle.com/javase/8/docs/api/", "java", "Java SE API文档"),
        ("https://spring.io/projects/spring-boot", "java", "Spring Boot文档"),
        
        // Go文档
        ("https://pkg.go.dev/", "go", "Go包文档"),
        ("https://golang.org/doc/", "go", "Go官方文档"),
        
        // 非文档URL（用于对比）
        ("https://github.com/", "rust", "GitHub主页"),
        ("https://stackoverflow.com/", "python", "Stack Overflow"),
        ("https://news.ycombinator.com/", "javascript", "Hacker News"),
    ];
    
    let mut results = Vec::new();
    
    for (url, language, description) in test_cases {
        info!("🧪 分析URL: {} ({})", description, url);
        
        let context = AnalysisContext {
            package_name: "test".to_string(),
            target_language: language.to_string(),
            search_intent: SearchIntent::Documentation,
        };
        
        match analyzer.analyze_url_relevance(url, language, &context).await {
            Ok(result) => {
                info!("✅ 分析成功: {} - 相关性: {:.2}, 置信度: {:.2}, 类型: {:?}", 
                    description, result.relevance_score, result.confidence, result.url_type);
                
                results.push((description.to_string(), result));
            }
            Err(e) => {
                error!("❌ 分析失败: {} - {}", description, e);
            }
        }
    }
    
    // 生成分析报告
    generate_analysis_report(&results);
    
    Ok(())
}

fn generate_analysis_report(results: &[(String, grape_mcp_devtools::language_features::smart_url_analyzer::UrlAnalysisResult)]) {
    println!("\n🎯 智能URL分析报告");
    println!("{}", "=".repeat(80));
    
    // 按相关性分数排序
    let mut sorted_results: Vec<_> = results.iter().collect();
    sorted_results.sort_by(|a, b| b.1.relevance_score.partial_cmp(&a.1.relevance_score).unwrap());
    
    println!("📊 按相关性排序的结果:");
    for (i, (description, result)) in sorted_results.iter().enumerate() {
        let status = if result.relevance_score >= 0.7 {
            "🟢 高相关"
        } else if result.relevance_score >= 0.4 {
            "🟡 中等相关"
        } else {
            "🔴 低相关"
        };
        
        println!("{}. {} {}", i + 1, status, description);
        println!("   URL: {}", result.url);
        println!("   相关性: {:.3} | 置信度: {:.3} | 类型: {:?}", 
            result.relevance_score, result.confidence, result.url_type);
        
        // 显示特征信息
        let features = &result.features;
        println!("   域名特征: 知名文档站点: {} | 权威性: {:.2} | 官方: {}", 
            features.domain_features.is_known_doc_site,
            features.domain_features.authority_score,
            features.domain_features.is_official);
        
        println!("   路径特征: 深度: {} | 文档关键词: {} | 有版本: {} | 类型: {:?}", 
            features.path_features.depth,
            features.path_features.doc_keyword_count,
            features.path_features.has_version,
            features.path_features.path_type);
        
        println!("   语言特征: 匹配度: {:.2} | 指示器: {:?}", 
            features.language_features.language_match_score,
            features.language_features.language_indicators);
        
        if let Some(content) = &features.content_features {
            println!("   内容特征: 质量: {:.2} | 代码块: {} | API结构: {}", 
                content.quality_score, content.code_block_count, content.api_structure_count);
        }
        
        println!();
    }
    
    // 统计信息
    let high_relevance = results.iter().filter(|(_, r)| r.relevance_score >= 0.7).count();
    let medium_relevance = results.iter().filter(|(_, r)| r.relevance_score >= 0.4 && r.relevance_score < 0.7).count();
    let low_relevance = results.iter().filter(|(_, r)| r.relevance_score < 0.4).count();
    
    println!("📈 统计信息:");
    println!("  总URL数: {}", results.len());
    println!("  高相关性 (≥0.7): {} ({:.1}%)", high_relevance, high_relevance as f32 / results.len() as f32 * 100.0);
    println!("  中等相关性 (0.4-0.7): {} ({:.1}%)", medium_relevance, medium_relevance as f32 / results.len() as f32 * 100.0);
    println!("  低相关性 (<0.4): {} ({:.1}%)", low_relevance, low_relevance as f32 / results.len() as f32 * 100.0);
    
    // 按URL类型分组
    let mut type_counts = std::collections::HashMap::new();
    for (_, result) in results {
        *type_counts.entry(format!("{:?}", result.url_type)).or_insert(0) += 1;
    }
    
    println!("\n📋 URL类型分布:");
    for (url_type, count) in type_counts {
        println!("  {}: {} ({:.1}%)", url_type, count, count as f32 / results.len() as f32 * 100.0);
    }
    
    println!("\n{}", "=".repeat(80));
} 