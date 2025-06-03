pub mod analysis;
pub mod api_docs;
pub mod base;
pub mod dependencies;
pub mod documentation_suggestions;
pub mod python_docs_tool;
pub mod javascript_docs_tool;
pub mod typescript_docs_tool;
pub mod rust_docs_tool;
pub mod java_docs_tool;
pub mod flutter_docs_tool;
pub mod search;
pub mod security;
pub mod versioning;
pub mod vector_docs_tool;
pub mod doc_processor;
pub mod enhanced_language_tool;
pub mod environment_detector;
pub mod dynamic_registry;
pub mod enhanced_doc_processor;
pub mod environment;
pub mod background_cacher;
// pub mod unified_vector_store; // 禁用：Tantivy兼容性问题

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
pub use flutter_docs_tool::FlutterDocsTool;
pub use versioning::CheckVersionTool;
pub use environment::EnvironmentDetectionTool;

// 重新导出主要类型
pub use base::{MCPTool, FileDocumentFragment, ToolAnnotations, Schema};
pub use dynamic_registry::DynamicToolRegistry;
pub use doc_processor::DocumentProcessor;
pub use enhanced_doc_processor::{EnhancedDocumentProcessor, ProcessorConfig, EnhancedSearchResult};
pub use vector_docs_tool::VectorDocsTool;
pub use search::SearchDocsTools;
