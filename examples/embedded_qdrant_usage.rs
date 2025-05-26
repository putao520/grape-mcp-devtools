use anyhow::Result;
use grape_mcp_devtools::{
    storage::{
        qdrant::{QdrantConfig, QdrantMode, QdrantFileStore},
        traits::{VectorStore, DocumentVectorStore},
    },
    vectorization::embeddings::{FileVectorizerImpl, EmbeddingConfig, VectorizationConfig},
    tools::base::{FileDocumentFragment, FileVectorizer},
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🚀 内嵌Qdrant + async-openai 向量化完整示例");
    println!("💡 无需Docker，直接在进程中运行Qdrant！");

    // 1. 创建内嵌Qdrant配置
    let qdrant_config = QdrantConfig {
        mode: QdrantMode::Embedded {
            storage_path: PathBuf::from("./data/example_qdrant"),
            enable_web: true,
            web_port: Some(6333),
        },
        collection_prefix: "example_".to_string(),
        vector_dimension: 768,
        recreate_collections: true, // 演示时重新创建
        ..Default::default()
    };

    println!("📋 Qdrant配置:");
    println!("   - 模式: 内嵌");
    println!("   - 存储路径: {:?}", match &qdrant_config.mode {
        QdrantMode::Embedded { storage_path, .. } => storage_path,
        _ => unreachable!(),
    });
    println!("   - Web界面: http://localhost:6333");

    // 2. 创建向量化器配置
    let embedding_config = EmbeddingConfig::from_env()?;
    let vectorization_config = VectorizationConfig::from_env()?;

    println!("\n🧠 向量化配置:");
    println!("   - API: {}", embedding_config.api_base_url);
    println!("   - 模型: {}", embedding_config.model_name);
    println!("   - 维度: {}", vectorization_config.vector_dimension);

    // 3. 初始化存储和向量化器
    println!("\n⚡ 初始化组件...");
    
    println!("🗃️ 启动内嵌Qdrant...");
    let storage = QdrantFileStore::new(qdrant_config).await?;
    
    println!("🧠 创建向量化器...");
    let vectorizer = FileVectorizerImpl::new(embedding_config, vectorization_config).await?;

    // 4. 健康检查
    println!("\n🔍 执行健康检查...");
    if storage.health_check().await? {
        println!("✅ Qdrant健康状态正常");
    } else {
        println!("❌ Qdrant健康状态异常");
        return Ok(());
    }

    // 5. 创建示例文档
    println!("\n📄 创建示例文档...");
    let docs = create_sample_documents();
    
    // 6. 批量向量化和存储
    println!("⚡ 批量向量化 {} 个文档...", docs.len());
    for (i, doc) in docs.iter().enumerate() {
        println!("  处理文档 {}: {}", i + 1, doc.file_path);
        
        // 向量化
        let vector = vectorizer.vectorize_file(doc).await?;
        
        // 存储
        storage.store_file_vector(&vector, doc).await?;
        
        println!("    ✅ 完成 (向量维度: {})", vector.dimension);
    }

    // 7. 执行语义搜索
    println!("\n🔍 执行语义搜索...");
    let queries = vec![
        "HTTP request handling",
        "error management", 
        "data serialization",
        "async programming",
    ];

    for query in queries {
        println!("\n查询: '{}'", query);
        
        // 向量化查询
        let query_vector = vectorizer.vectorize_query(query).await?;
        
        // 搜索
        let results = storage.search("rust", query_vector, None, Some(3)).await?;
        
        println!("找到 {} 个结果:", results.len());
        for (i, result) in results.iter().enumerate() {
            println!("  {}. 相似度: {:.3}", i + 1, result.score);
            if let Some(file_path) = result.metadata.get("file_path").and_then(|v| v.as_str()) {
                println!("     文件: {}", file_path);
            }
        }
    }

    // 8. 显示存储统计
    println!("\n📊 存储统计信息:");
    let info = storage.get_info().await?;
    println!("   - 存储类型: {}", info.store_type);
    println!("   - 版本: {}", info.version);
    println!("   - 集合数: {}", info.total_collections);
    println!("   - 向量总数: {}", info.total_vectors);

    // 9. 文件操作演示
    demonstrate_file_operations(&storage).await?;

    println!("\n🎉 示例完成！");
    println!("💡 提示:");
    println!("   - 内嵌Qdrant数据存储在: ./data/example_qdrant");
    println!("   - 可以访问Web界面: http://localhost:6333");
    println!("   - 程序退出后数据会持久保存");

    Ok(())
}

fn create_sample_documents() -> Vec<FileDocumentFragment> {
    vec![
        FileDocumentFragment {
            id: "rust_http_client".to_string(),
            package_name: "reqwest".to_string(),
            version: "0.11.0".to_string(),
            language: "rust".to_string(),
            file_path: "client.rs".to_string(),
            content: r#"
                /// HTTP客户端实现
                pub struct Client {
                    inner: reqwest::Client,
                }
                
                impl Client {
                    /// 创建新的HTTP客户端
                    pub fn new() -> Self {
                        Self {
                            inner: reqwest::Client::new(),
                        }
                    }
                    
                    /// 发送GET请求
                    pub async fn get(&self, url: &str) -> Result<Response> {
                        self.inner.get(url).send().await
                    }
                    
                    /// 发送POST请求
                    pub async fn post(&self, url: &str, body: String) -> Result<Response> {
                        self.inner.post(url).body(body).send().await
                    }
                }
            "#.to_string(),
            hierarchy_path: vec!["src".to_string(), "client.rs".to_string()],
            metadata: Default::default(),
        },
        FileDocumentFragment {
            id: "rust_error_handling".to_string(),
            package_name: "anyhow".to_string(),
            version: "1.0.0".to_string(),
            language: "rust".to_string(),
            file_path: "error.rs".to_string(),
            content: r#"
                /// 错误处理工具
                use anyhow::{Result, anyhow};
                
                /// 自定义错误类型
                #[derive(Debug)]
                pub enum AppError {
                    Network(String),
                    Parse(String),
                    IO(std::io::Error),
                }
                
                impl AppError {
                    /// 创建网络错误
                    pub fn network<T: Into<String>>(msg: T) -> Self {
                        Self::Network(msg.into())
                    }
                    
                    /// 处理错误并记录日志
                    pub fn handle_error(err: &AppError) {
                        match err {
                            AppError::Network(msg) => eprintln!("网络错误: {}", msg),
                            AppError::Parse(msg) => eprintln!("解析错误: {}", msg),
                            AppError::IO(err) => eprintln!("IO错误: {}", err),
                        }
                    }
                }
            "#.to_string(),
            hierarchy_path: vec!["src".to_string(), "error.rs".to_string()],
            metadata: Default::default(),
        },
        FileDocumentFragment {
            id: "rust_serialization".to_string(),
            package_name: "serde".to_string(),
            version: "1.0.0".to_string(),
            language: "rust".to_string(),
            file_path: "serialize.rs".to_string(),
            content: r#"
                /// 数据序列化工具
                use serde::{Serialize, Deserialize};
                
                /// 用户数据结构
                #[derive(Debug, Serialize, Deserialize)]
                pub struct User {
                    pub id: u64,
                    pub name: String,
                    pub email: String,
                    pub active: bool,
                }
                
                impl User {
                    /// 创建新用户
                    pub fn new(id: u64, name: String, email: String) -> Self {
                        Self {
                            id,
                            name,
                            email,
                            active: true,
                        }
                    }
                    
                    /// 序列化为JSON
                    pub fn to_json(&self) -> Result<String> {
                        serde_json::to_string(self)
                            .map_err(|e| anyhow!("序列化失败: {}", e))
                    }
                    
                    /// 从JSON反序列化
                    pub fn from_json(json: &str) -> Result<Self> {
                        serde_json::from_str(json)
                            .map_err(|e| anyhow!("反序列化失败: {}", e))
                    }
                }
            "#.to_string(),
            hierarchy_path: vec!["src".to_string(), "serialize.rs".to_string()],
            metadata: Default::default(),
        },
        FileDocumentFragment {
            id: "rust_async_utils".to_string(),
            package_name: "tokio".to_string(),
            version: "1.0.0".to_string(),
            language: "rust".to_string(),
            file_path: "async_utils.rs".to_string(),
            content: r#"
                /// 异步工具函数
                use tokio::time::{Duration, sleep, timeout};
                use std::future::Future;
                
                /// 异步重试机制
                pub async fn retry_with_backoff<F, Fut, T, E>(
                    mut operation: F,
                    max_retries: usize,
                    initial_delay: Duration,
                ) -> Result<T, E>
                where
                    F: FnMut() -> Fut,
                    Fut: Future<Output = Result<T, E>>,
                {
                    let mut delay = initial_delay;
                    
                    for attempt in 0..max_retries {
                        match operation().await {
                            Ok(result) => return Ok(result),
                            Err(error) => {
                                if attempt == max_retries - 1 {
                                    return Err(error);
                                }
                                sleep(delay).await;
                                delay *= 2; // 指数退避
                            }
                        }
                    }
                    
                    unreachable!()
                }
                
                /// 带超时的异步操作
                pub async fn with_timeout<F, T>(
                    future: F,
                    timeout_duration: Duration,
                ) -> Result<T, tokio::time::error::Elapsed>
                where
                    F: Future<Output = T>,
                {
                    timeout(timeout_duration, future).await
                }
            "#.to_string(),
            hierarchy_path: vec!["src".to_string(), "async_utils.rs".to_string()],
            metadata: Default::default(),
        },
    ]
}

async fn demonstrate_file_operations(storage: &QdrantFileStore) -> Result<()> {
    println!("\n🗂️ 文件操作演示:");
    
    // 检查文件是否存在
    let exists = storage.file_exists("rust", "reqwest", "0.11.0", "client.rs").await?;
    println!("   - client.rs 存在: {}", exists);
    
    // 获取文件内容
    if let Some(file) = storage.get_file("rust", "reqwest", "0.11.0", "client.rs").await? {
        println!("   - 获取到文件: {} ({} 字符)", file.file_path, file.content.len());
    }
    
    // 列出包中的文件
    let files = storage.list_package_files("rust", "reqwest", "0.11.0").await?;
    println!("   - reqwest包中的文件数: {}", files.len());
    for file in files.iter().take(3) {
        println!("     * {}", file);
    }
    
    Ok(())
} 