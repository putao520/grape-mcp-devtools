// src/models/mod.rs
// 模型定义

/// 基础文档模型
pub mod document {
    use serde::{Serialize, Deserialize};
    
    /// 基本文档信息
    #[derive(Debug, Serialize, Deserialize)]
    pub struct DocumentInfo {
        /// 文档标题
        pub title: String,
        /// 文档类型
        pub doc_type: String,
        /// 文档内容
        pub content: String,
    }
}

/// 工具配置模型
pub mod config {
    use serde::{Serialize, Deserialize};
    
    /// 工具配置
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ToolConfig {
        /// 工具名称
        pub name: String,
        /// 工具描述
        pub description: String,
        /// 配置参数
        pub parameters: serde_json::Map<String, serde_json::Value>,
    }
}
