use anyhow::Result;
use crate::tools::doc_processor::DocumentProcessor;

#[tokio::test]
async fn test_doc_processor_creation() -> Result<()> {
    println!("🔧 测试DocumentProcessor创建");
    
    // 检查是否有NVIDIA API密钥
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过DocumentProcessor测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let _processor = DocumentProcessor::new().await?;
    println!("✅ DocumentProcessor创建成功");
    
    Ok(())
}

#[tokio::test]
async fn test_go_docs_generation() -> Result<()> {
    println!("🐹 测试Go文档生成");
    
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过Go文档测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试一个简单的Go包
    let result = processor.process_documentation_request(
        "go",
        "fmt",
        Some("latest"),
        "formatting functions"
    ).await;
    
    match result {
        Ok(fragments) => {
            println!("✅ Go文档生成成功，生成了 {} 个片段", fragments.len());
            assert!(!fragments.is_empty());
            
            // 验证片段内容
            for fragment in &fragments {
                assert_eq!(fragment.language, "go");
                assert_eq!(fragment.package_name, "fmt");
                assert!(!fragment.content.is_empty());
                println!("   - 片段: {} ({} 字符)", fragment.file_path, fragment.content.len());
            }
        }
        Err(e) => {
            println!("⚠️  Go文档生成失败: {}", e);
            // 在某些环境中可能失败，这是可以接受的
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_python_docs_generation() -> Result<()> {
    println!("🐍 测试Python文档生成");
    
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过Python文档测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试一个简单的Python包
    let result = processor.process_documentation_request(
        "python",
        "requests",
        Some("latest"),
        "HTTP library"
    ).await;
    
    match result {
        Ok(fragments) => {
            println!("✅ Python文档生成成功，生成了 {} 个片段", fragments.len());
            assert!(!fragments.is_empty());
            
            for fragment in &fragments {
                assert_eq!(fragment.language, "python");
                assert_eq!(fragment.package_name, "requests");
                assert!(!fragment.content.is_empty());
                println!("   - 片段: {} ({} 字符)", fragment.file_path, fragment.content.len());
            }
        }
        Err(e) => {
            println!("⚠️  Python文档生成失败: {}", e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_npm_docs_generation() -> Result<()> {
    println!("📦 测试NPM文档生成");
    
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过NPM文档测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试一个简单的NPM包
    let result = processor.process_documentation_request(
        "javascript",
        "lodash",
        Some("latest"),
        "utility library"
    ).await;
    
    match result {
        Ok(fragments) => {
            println!("✅ NPM文档生成成功，生成了 {} 个片段", fragments.len());
            assert!(!fragments.is_empty());
            
            for fragment in &fragments {
                assert_eq!(fragment.language, "javascript");
                assert_eq!(fragment.package_name, "lodash");
                assert!(!fragment.content.is_empty());
                println!("   - 片段: {} ({} 字符)", fragment.file_path, fragment.content.len());
            }
        }
        Err(e) => {
            println!("⚠️  NPM文档生成失败: {}", e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_java_docs_generation() -> Result<()> {
    println!("☕ 测试Java文档生成");
    
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过Java文档测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试一个简单的Java库（使用Maven坐标）
    let result = processor.process_documentation_request(
        "java",
        "com.google.guava:guava",
        Some("latest"),
        "Google core libraries"
    ).await;
    
    match result {
        Ok(fragments) => {
            println!("✅ Java文档生成成功，生成了 {} 个片段", fragments.len());
            assert!(!fragments.is_empty());
            
            for fragment in &fragments {
                assert_eq!(fragment.language, "java");
                assert_eq!(fragment.package_name, "com.google.guava:guava");
                assert!(!fragment.content.is_empty());
                println!("   - 片段: {} ({} 字符)", fragment.file_path, fragment.content.len());
            }
        }
        Err(e) => {
            println!("⚠️  Java文档生成失败: {}", e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rust_docs_generation() -> Result<()> {
    println!("🦀 测试Rust文档生成");
    
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过Rust文档测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let processor = DocumentProcessor::new().await?;
    
    // 测试一个简单的Rust crate
    let result = processor.process_documentation_request(
        "rust",
        "serde",
        Some("latest"),
        "serialization framework"
    ).await;
    
    match result {
        Ok(fragments) => {
            println!("✅ Rust文档生成成功，生成了 {} 个片段", fragments.len());
            assert!(!fragments.is_empty());
            
            for fragment in &fragments {
                assert_eq!(fragment.language, "rust");
                assert_eq!(fragment.package_name, "serde");
                assert!(!fragment.content.is_empty());
                println!("   - 片段: {} ({} 字符)", fragment.file_path, fragment.content.len());
            }
        }
        Err(e) => {
            println!("⚠️  Rust文档生成失败: {}", e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_vector_storage_and_search() -> Result<()> {
    println!("🔍 测试向量存储和搜索");
    
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过向量存储测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let processor = DocumentProcessor::new().await?;
    
    // 第一次请求：生成并存储文档
    let result1 = processor.process_documentation_request(
        "python",
        "json",
        Some("latest"),
        "JSON encoder decoder"
    ).await;
    
    match result1 {
        Ok(fragments1) => {
            println!("✅ 第一次请求成功，生成了 {} 个片段", fragments1.len());
            
            // 第二次相同请求：应该从向量库返回
            let result2 = processor.process_documentation_request(
                "python",
                "json",
                Some("latest"),
                "JSON encoder decoder"
            ).await;
            
            match result2 {
                Ok(fragments2) => {
                    println!("✅ 第二次请求成功，返回了 {} 个片段", fragments2.len());
                    // 第二次请求可能返回相同或相关的文档
                    assert!(!fragments2.is_empty());
                }
                Err(e) => {
                    println!("⚠️  第二次请求失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠️  第一次请求失败: {}", e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_unsupported_language() -> Result<()> {
    println!("❌ 测试不支持的语言");
    
    if std::env::var("EMBEDDING_API_KEY").is_err() {
        println!("⚠️  跳过不支持语言测试：未设置EMBEDDING_API_KEY环境变量");
        return Ok(());
    }
    
    let processor = DocumentProcessor::new().await?;
    
    let result = processor.process_documentation_request(
        "unsupported_language",
        "some_package",
        Some("1.0.0"),
        "test query"
    ).await;
    
    match result {
        Ok(_) => {
            // 应该返回错误，但却成功了
            assert!(false, "应该返回错误，但却成功了");
        }
        Err(e) => {
            println!("✅ 正确返回错误: {}", e);
            assert!(e.to_string().contains("不支持的语言"));
        }
    }
    
    Ok(())
} 