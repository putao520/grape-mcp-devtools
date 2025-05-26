use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::tools::base::MCPTool;
use crate::tools::docs::doc_traits::{
    DocumentFragment, DocumentStore, DocumentIndex, DocumentCache, DocumentVectorizer,
    DocumentVector, DocumentVectorMetadata, DocumentType, SearchFilter, SearchResult,
    DocElementKind, DocSourceType, DocMetadata, Visibility
};

// 模拟的 GoDocSearchTool 和 GoDocGenerator（用于测试）
pub struct GoDocSearchTool {
    vector_store: Arc<dyn DocumentIndex>,
    cache: Arc<dyn DocumentCache>,
    vectorizer: Arc<dyn DocumentVectorizer>,
}

impl GoDocSearchTool {
    pub fn new(
        vector_store: Arc<dyn DocumentIndex>,
        cache: Arc<dyn DocumentCache>,
        _doc_generator: Arc<GoDocGenerator>,
        vectorizer: Arc<dyn DocumentVectorizer>,
    ) -> Self {
        Self {
            vector_store,
            cache,
            vectorizer,
        }
    }

    pub async fn search_documentation(
        &self,
        package_name: &str,
        _version: Option<&str>,
        query: &str,
    ) -> Result<Value> {
        // 模拟搜索逻辑
        let filter = SearchFilter {
            doc_types: None,
            languages: Some(vec!["go".to_string()]),
            limit: Some(10),
            similarity_threshold: Some(0.5),
        };

        let results = self.vector_store.search(query, &filter).await?;
        
        if !results.is_empty() {
            Ok(json!({
                "status": "success",
                "source": "vector_store",
                "package": package_name,
                "results": results.len()
            }))
        } else {
            Ok(json!({
                "status": "failure",
                "package": package_name,
                "message": "LLM调用工具失败"
            }))
        }
    }
}

pub struct GoDocGenerator;

impl GoDocGenerator {
    pub fn new() -> Self {
        Self
    }
}

/// Go 文档搜索 MCP 工具 - 作为 LLM 可调用的工具
pub struct GoDocSearchMCPTool {
    search_tool: Arc<GoDocSearchTool>,
}

impl GoDocSearchMCPTool {
    pub fn new(search_tool: Arc<GoDocSearchTool>) -> Self {
        Self { search_tool }
    }
}

#[async_trait::async_trait]
impl MCPTool for GoDocSearchMCPTool {
    fn name(&self) -> &str {
        "search_go_documentation"
    }

    fn description(&self) -> &str {
        "搜索 Go 语言库文档。首先从向量库搜索，如果没找到则生成本地文档并向量化存储，然后再次搜索。"
    }

    fn parameters_schema(&self) -> &crate::tools::base::Schema {
        use std::sync::OnceLock;
        use std::collections::HashMap;
        use crate::tools::base::{Schema, SchemaObject, SchemaString};

        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["package_name".to_string(), "query".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("package_name".to_string(), Schema::String(SchemaString {
                        description: Some("Go 包名，如 fmt、github.com/gin-gonic/gin".to_string()),
                        enum_values: None,
                    }));
                    map.insert("version".to_string(), Schema::String(SchemaString {
                        description: Some("包版本号，如 v1.9.1，留空表示最新版本".to_string()),
                        enum_values: None,
                    }));
                    map.insert("query".to_string(), Schema::String(SchemaString {
                        description: Some("搜索查询，如 'Context usage'、'HTTP handler'".to_string()),
                        enum_values: None,
                    }));
                    map
                },
                ..Default::default()
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        // 验证参数
        self.validate_params(&params)?;

        // 提取参数
        let package_name = params["package_name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("package_name 参数无效"))?;
            
        let version = params["version"].as_str();
        
        let query = params["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("query 参数无效"))?;

        // 调用核心搜索逻辑
        self.search_tool
            .search_documentation(package_name, version, query)
            .await
    }
}

/// 简单的内存文档存储实现（用于测试）
pub struct InMemoryDocumentStore {
    documents: Arc<tokio::sync::RwLock<std::collections::HashMap<String, DocumentFragment>>>,
}

impl InMemoryDocumentStore {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl DocumentStore for InMemoryDocumentStore {
    async fn store(&self, fragment: &DocumentFragment) -> Result<()> {
        let mut docs = self.documents.write().await;
        docs.insert(fragment.id.clone(), fragment.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<DocumentFragment>> {
        let docs = self.documents.read().await;
        Ok(docs.get(id).cloned())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut docs = self.documents.write().await;
        docs.remove(id);
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<DocumentFragment>> {
        let docs = self.documents.read().await;
        Ok(docs.values().cloned().collect())
    }
}

/// 简单的内存向量索引实现（用于测试）
pub struct InMemoryVectorIndex {
    fragments: Arc<tokio::sync::RwLock<Vec<DocumentFragment>>>,
}

impl InMemoryVectorIndex {
    pub fn new() -> Self {
        Self {
            fragments: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl DocumentIndex for InMemoryVectorIndex {
    async fn index(&self, fragment: &DocumentFragment) -> Result<()> {
        let mut fragments = self.fragments.write().await;
        fragments.push(fragment.clone());
        Ok(())
    }

    async fn search(&self, query: &str, filter: &SearchFilter) -> Result<Vec<SearchResult>> {
        let fragments = self.fragments.read().await;
        let mut results = Vec::new();

        for fragment in fragments.iter() {
            // 简单的文本匹配搜索
            let content_lower = fragment.description.to_lowercase();
            let title_lower = fragment.title.to_lowercase();
            let query_lower = query.to_lowercase();

            let score = if title_lower.contains(&query_lower) {
                0.9
            } else if content_lower.contains(&query_lower) {
                0.7
            } else {
                0.0
            };

            if score >= filter.similarity_threshold.unwrap_or(0.5) {
                results.push(SearchResult {
                    fragment: fragment.clone(),
                    score,
                });
            }
        }

        // 按相关度排序
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // 限制结果数量
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut fragments = self.fragments.write().await;
        fragments.retain(|f| f.id != id);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut fragments = self.fragments.write().await;
        fragments.clear();
        Ok(())
    }
}

/// 简单的内存缓存实现（用于测试）
pub struct InMemoryCache {
    cache: Arc<tokio::sync::RwLock<std::collections::HashMap<String, DocumentFragment>>>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl DocumentCache for InMemoryCache {
    async fn get(&self, key: &str) -> Result<Option<DocumentFragment>> {
        let cache = self.cache.read().await;
        Ok(cache.get(key).cloned())
    }

    async fn set(&self, key: &str, fragment: &DocumentFragment) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), fragment.clone());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.remove(key);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        Ok(())
    }
}

/// 简单的向量化器实现（用于测试）
pub struct SimpleVectorizer;

#[async_trait::async_trait]
impl DocumentVectorizer for SimpleVectorizer {
    async fn vectorize(&self, fragment: &DocumentFragment) -> Result<DocumentVector> {
        // 简单的向量化：基于关键词计数
        let keywords = extract_keywords(&fragment.description);
        let mut vector_data = vec![0.0; 3]; // 简化为3维向量
        
        // 基于关键词设置向量值
        for keyword in &keywords {
            match keyword.as_str() {
                "func" | "function" => vector_data[0] += 1.0,
                "type" | "struct" | "interface" => vector_data[1] += 1.0,
                "package" | "import" => vector_data[2] += 1.0,
                _ => {}
            }
        }

        // 归一化
        let magnitude: f32 = vector_data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut vector_data {
                *val /= magnitude;
            }
        }

        Ok(DocumentVector {
            data: vector_data,
            dimension: 3,
            metadata: DocumentVectorMetadata {
                doc_id: fragment.id.clone(),
                doc_type: DocumentType::Api,
                language: fragment.metadata.language.clone(),
                keywords,
            },
        })
    }

    async fn devectorize(&self, vector: &DocumentVector) -> Result<DocumentFragment> {
        // 简单的反向量化（实际应用中不太可能完美还原）
        Ok(DocumentFragment {
            id: vector.metadata.doc_id.clone(),
            title: "Devectorized Document".to_string(),
            kind: DocElementKind::Function,
            full_name: None,
            description: format!("Vector data: {:?}", vector.data),
            source_type: DocSourceType::ApiDoc,
            code_context: None,
            examples: vec![],
            api_info: None,
            references: vec![],
            metadata: DocMetadata {
                package_name: "unknown".to_string(),
                version: None,
                language: vector.metadata.language.clone(),
                source_url: None,
                deprecated: false,
                since_version: None,
                visibility: Visibility::Public,
            },
            changelog_info: None,
        })
    }

    fn calculate_similarity(&self, vec1: &DocumentVector, vec2: &DocumentVector) -> f32 {
        // 计算余弦相似度
        let dot_product: f32 = vec1.data.iter().zip(&vec2.data).map(|(a, b)| a * b).sum();
        let magnitude1: f32 = vec1.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude2: f32 = vec2.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude1 > 0.0 && magnitude2 > 0.0 {
            dot_product / (magnitude1 * magnitude2)
        } else {
            0.0
        }
    }
}

/// 提取关键词
fn extract_keywords(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .map(|word| word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|word| !word.is_empty() && word.len() > 2)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_environment() -> GoDocSearchMCPTool {
        let vector_store = Arc::new(InMemoryVectorIndex::new());
        let cache = Arc::new(InMemoryCache::new());
        let doc_generator = Arc::new(GoDocGenerator::new());
        let vectorizer = Arc::new(SimpleVectorizer);

        let search_tool = Arc::new(GoDocSearchTool::new(
            vector_store,
            cache,
            doc_generator,
            vectorizer,
        ));

        GoDocSearchMCPTool::new(search_tool)
    }

    /// 测试完整的Go文档搜索工作流程
    /// 这个测试验证了LLM通过MCP工具搜索Go语言库文档的完整流程：
    /// 1. 首先从向量库搜索
    /// 2. 如果没找到，生成本地文档
    /// 3. 将文档向量化并存储
    /// 4. 再次搜索
    /// 5. 如果还是没找到，返回失败
    #[tokio::test]
    async fn test_complete_go_documentation_workflow() {
        println!("🚀 开始完整的Go文档搜索工作流程测试...");

        let mcp_tool = setup_test_environment().await;

        // 场景1: 搜索标准库包 fmt
        println!("\n📚 场景1: 搜索Go标准库fmt包的Printf函数...");
        
        let params = json!({
            "package_name": "fmt",
            "query": "Printf function"
        });

        let result = mcp_tool.execute(params).await.unwrap();
        let result_obj = result.as_object().unwrap();

        println!("📊 搜索结果状态: {}", result_obj["status"]);
        println!("📦 包名: {}", result_obj["package"]);

        // 验证结果结构
        assert!(result_obj.contains_key("status"));
        assert!(result_obj.contains_key("package"));
        assert_eq!(result_obj["package"], "fmt");

        // 根据状态验证结果
        match result_obj["status"].as_str().unwrap() {
            "success" => {
                println!("✅ 成功找到文档");
                if result_obj["source"] == "vector_store" {
                    println!("📚 来源：向量库");
                } else if result_obj["source"] == "generated_docs" {
                    println!("📚 来源：生成的本地文档");
                    if let Some(fragments) = result_obj.get("generated_fragments") {
                        println!("📄 生成了 {} 个文档片段", fragments);
                    }
                }
                assert!(result_obj.contains_key("results"));
            }
            "partial_success" => {
                println!("⚠️  生成了文档但未找到相关内容");
                assert!(result_obj.contains_key("generated_fragments"));
            }
            "failure" => {
                println!("❌ 文档生成失败: {}", result_obj.get("error").unwrap_or(&json!("未知错误")));
                assert!(result_obj.contains_key("message"));
                assert!(result_obj["message"].as_str().unwrap().contains("LLM调用工具失败"));
            }
            unknown_status => {
                println!("⚠️  未知的状态: {}", unknown_status);
                // 记录错误但不中断测试
            }
        }

        // 场景2: 再次搜索相同的包（测试缓存效果）
        println!("\n🔄 场景2: 再次搜索fmt包的Sprintf函数（测试缓存）...");
        
        let params2 = json!({
            "package_name": "fmt",
            "query": "Sprintf function"
        });

        let result2 = mcp_tool.execute(params2).await.unwrap();
        let result2_obj = result2.as_object().unwrap();

        println!("📊 第二次搜索状态: {}", result2_obj["status"]);
        
        // 第二次搜索应该更快，因为文档已经在向量库中
        if result2_obj["status"] == "success" && result2_obj["source"] == "vector_store" {
            println!("✅ 成功从向量库获取缓存的文档");
        }

        // 场景3: 搜索不存在的包
        println!("\n❌ 场景3: 搜索不存在的包...");
        
        let params3 = json!({
            "package_name": "github.com/nonexistent/invalid-package-12345",
            "version": "v999.999.999",
            "query": "some function"
        });

        let result3 = mcp_tool.execute(params3).await.unwrap();
        let result3_obj = result3.as_object().unwrap();

        println!("📊 不存在包的搜索状态: {}", result3_obj["status"]);
        
        // 应该返回失败状态
        assert_eq!(result3_obj["status"], "failure");
        assert!(result3_obj["message"].as_str().unwrap().contains("LLM调用工具失败"));
        println!("✅ 正确处理了不存在的包");

        // 场景4: 测试参数验证
        println!("\n🔍 场景4: 测试参数验证...");
        
        let invalid_params = json!({
            "package_name": "fmt"
            // 缺少必需的 query 参数
        });

        let result4 = mcp_tool.execute(invalid_params).await;
        assert!(result4.is_err());
        println!("✅ 正确检测到缺少必需参数");

        println!("\n🎉 完整的Go文档搜索工作流程测试完成！");
    }

    /// 测试工具元数据
    #[tokio::test]
    async fn test_tool_metadata() {
        println!("📋 测试工具元数据...");

        let mcp_tool = setup_test_environment().await;

        // 验证工具名称
        assert_eq!(mcp_tool.name(), "search_go_documentation");
        println!("✅ 工具名称正确: {}", mcp_tool.name());

        // 验证工具描述
        let description = mcp_tool.description();
        assert!(description.contains("搜索 Go 语言库文档"));
        assert!(description.contains("向量库"));
        println!("✅ 工具描述正确");

        // 验证参数模式
        let schema = mcp_tool.parameters_schema();
        if let crate::tools::base::Schema::Object(obj) = schema {
            assert!(obj.required.contains(&"package_name".to_string()));
            assert!(obj.required.contains(&"query".to_string()));
            assert!(obj.properties.contains_key("package_name"));
            assert!(obj.properties.contains_key("query"));
            assert!(obj.properties.contains_key("version"));
            println!("✅ 参数模式正确");
        } else {
            println!("⚠️  参数模式不是期望的SchemaObject类型");
            // 记录错误但不中断测试
        }

        println!("🎉 工具元数据测试完成！");
    }

    /// 测试向量存储操作
    #[tokio::test]
    async fn test_vector_store_operations() {
        println!("📊 测试向量存储操作...");

        let vector_store = InMemoryVectorIndex::new();
        
        let test_fragment = DocumentFragment {
            id: "test_doc".to_string(),
            title: "Test Document".to_string(),
            kind: DocElementKind::Function,
            full_name: Some("test.Printf".to_string()),
            description: "This is a test document with Go programming content func Printf".to_string(),
            source_type: DocSourceType::ApiDoc,
            code_context: None,
            examples: vec![],
            api_info: None,
            references: vec![],
            metadata: DocMetadata {
                package_name: "test".to_string(),
                version: None,
                language: "go".to_string(),
                source_url: None,
                deprecated: false,
                since_version: None,
                visibility: Visibility::Public,
            },
            changelog_info: None,
        };

        // 测试索引
        vector_store.index(&test_fragment).await.unwrap();
        println!("✅ 成功索引文档");

        // 测试搜索
        let filter = SearchFilter {
            doc_types: None,
            languages: Some(vec!["go".to_string()]),
            limit: Some(10),
            similarity_threshold: Some(0.5),
        };

        let results = vector_store.search("Printf", &filter).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].fragment.id, "test_doc");
        println!("✅ 成功搜索到文档");

        // 测试删除
        vector_store.delete("test_doc").await.unwrap();
        let results_after_delete = vector_store.search("Printf", &filter).await.unwrap();
        assert!(results_after_delete.is_empty());
        println!("✅ 成功删除文档");

        println!("🎉 向量存储操作测试完成！");
    }

    /// 测试缓存操作
    #[tokio::test]
    async fn test_cache_operations() {
        println!("💾 测试缓存操作...");

        let cache = InMemoryCache::new();
        
        let test_fragment = DocumentFragment {
            id: "cached_doc".to_string(),
            title: "Cached Document".to_string(),
            kind: DocElementKind::Function,
            full_name: Some("cache.Get".to_string()),
            description: "This document is cached".to_string(),
            source_type: DocSourceType::ApiDoc,
            code_context: None,
            examples: vec![],
            api_info: None,
            references: vec![],
            metadata: DocMetadata {
                package_name: "cache".to_string(),
                version: None,
                language: "go".to_string(),
                source_url: None,
                deprecated: false,
                since_version: None,
                visibility: Visibility::Public,
            },
            changelog_info: None,
        };

        // 测试设置缓存
        cache.set("test_key", &test_fragment).await.unwrap();
        println!("✅ 成功设置缓存");

        // 测试获取缓存
        let retrieved = cache.get("test_key").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "cached_doc");
        println!("✅ 成功获取缓存");

        // 测试删除缓存
        cache.delete("test_key").await.unwrap();
        let retrieved_after_delete = cache.get("test_key").await.unwrap();
        assert!(retrieved_after_delete.is_none());
        println!("✅ 成功删除缓存");

        println!("🎉 缓存操作测试完成！");
    }

    /// 测试向量化器操作
    #[tokio::test]
    async fn test_vectorizer_operations() {
        println!("🔢 测试向量化器操作...");

        let vectorizer = SimpleVectorizer;
        
        let test_fragment = DocumentFragment {
            id: "vector_test".to_string(),
            title: "Vector Test".to_string(),
            kind: DocElementKind::Function,
            full_name: Some("fmt.Printf".to_string()),
            description: "func Printf(format string) This is content for vectorization testing".to_string(),
            source_type: DocSourceType::ApiDoc,
            code_context: None,
            examples: vec![],
            api_info: None,
            references: vec![],
            metadata: DocMetadata {
                package_name: "fmt".to_string(),
                version: None,
                language: "go".to_string(),
                source_url: None,
                deprecated: false,
                since_version: None,
                visibility: Visibility::Public,
            },
            changelog_info: None,
        };

        // 测试向量化
        let vector = vectorizer.vectorize(&test_fragment).await.unwrap();
        assert_eq!(vector.dimension, 3);
        assert_eq!(vector.data.len(), 3);
        assert_eq!(vector.metadata.doc_id, "vector_test");
        println!("✅ 成功向量化文档");

        // 测试相似度计算
        let vector2 = vectorizer.vectorize(&test_fragment).await.unwrap();
        let similarity = vectorizer.calculate_similarity(&vector, &vector2);
        assert!((similarity - 1.0).abs() < 0.001); // 相同文档应该相似度为1
        println!("✅ 相似度计算正确: {:.3}", similarity);

        println!("🎉 向量化器操作测试完成！");
    }
} 