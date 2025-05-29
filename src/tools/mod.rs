pub mod analysis;
pub mod api_docs;
pub mod base;
pub mod dependencies;
pub mod documentation_suggestions;
pub mod file_go_docs_tool;  // 重新启用Go工具
pub mod python_docs_tool;
pub mod javascript_docs_tool;
pub mod typescript_docs_tool;
pub mod rust_docs_tool;
pub mod java_docs_tool;
pub mod search;
pub mod security;
pub mod versioning;
pub mod vector_docs_tool;
pub mod doc_processor;
pub mod enhanced_language_tool;
pub mod environment_detector;
pub mod dynamic_registry;

/// 文档处理模块 - 提供多语言文档解析和处理功能
pub mod docs {
    pub mod doc_traits;
    pub mod openai_vectorizer;
}

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod test_simple_improvements;

// === 基础工具导出 ===

// === 语言特定工具导出 ===
  // 重新启用

// === 增强工具导出 ===

// === 语言特性工具导出 ===

// 重新导出主要组件
pub use search::SearchDocsTools as SearchDocsTool;
pub use dynamic_registry::{DynamicRegistryBuilder, RegistrationPolicy};
