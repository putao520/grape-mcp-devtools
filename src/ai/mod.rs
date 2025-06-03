/// AI增强服务模块
/// 
/// 提供基于大模型的智能分析和处理能力，用于增强：
/// 1. 文档处理器 - 智能内容提取和分析
/// 2. 自定义谓词 - 自然语言条件解析
/// 3. 智能URL分析器 - 语义理解和内容预测
/// 4. 智能网页分析器 - 相关性识别和内容区域分割
/// 5. 智能URL遍历器 - 防循环的相关URL发现和处理
/// 6. 任务导向爬虫 - 完整的目标导向爬虫解决方案

pub mod ai_service;
pub mod document_ai;
pub mod predicate_ai;
pub mod url_ai;
pub mod prompt_templates;
pub mod intelligent_web_analyzer;
pub mod smart_url_crawler;
pub mod task_oriented_crawler;
pub mod advanced_intelligent_crawler;
// pub mod ml_content_analyzer; // 禁用：需要unicode-segmentation模块
pub mod intelligent_parser;
pub mod high_performance_crawler;

#[cfg(test)]
pub mod tests;

pub use ai_service::*;
pub use document_ai::*;
pub use predicate_ai::*;
pub use url_ai::*;
// pub use ml_content_analyzer::*; // 禁用
pub use intelligent_web_analyzer::*;
pub use smart_url_crawler::*;
pub use task_oriented_crawler::*;
pub use advanced_intelligent_crawler::*;
// pub use high_performance_crawler::*; // 暂时禁用未使用的模块
pub use intelligent_parser::*; 