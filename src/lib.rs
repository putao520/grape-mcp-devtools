//! # Grape MCP DevTools
//!
//! 一个基于 MCP (Model Context Protocol) 的多语言文档服务，专为 LLM 提供文档查询和版本检查功能。
//! 
//! ## 特性
//! 
//! - 🔍 **文档搜索** - 搜索各种编程语言的包信息、API文档和使用指南
//! - 📦 **版本检查** - 获取包的最新版本、版本历史和兼容性信息
//! - 📚 **API文档** - 获取编程语言API的详细文档信息
//! - 🚀 **MCP协议** - 基于标准MCP协议，支持stdio模式通信
//! 
//! ## 快速开始
//! 
//! ```rust
//! use grape_mcp_devtools::*;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), MCPError> {
//!     // 创建MCP服务器
//!     let server = mcp::create_server().await?;
//!     
//!     // 启动服务器
//!     server.run().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod errors;
pub mod mcp;
pub mod tools;
pub mod versioning;
pub mod cli;
pub mod language_features;
pub mod ai;
pub mod config;

// 新增：智能MCP服务器模块（同进程多Agent架构）
// pub mod intelligent_mcp_server;

pub use errors::{MCPError, MCPResult};

// Re-export commonly used types
pub use async_trait::async_trait;
pub use serde_json::{json, Value}; 