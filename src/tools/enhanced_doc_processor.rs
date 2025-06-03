use anyhow::{Result, anyhow};
use tracing::{info, warn, debug};
use serde::{Serialize, Deserialize};
use tokio::time::{timeout, Duration};
use std::sync::Arc;

use crate::tools::base::{FileDocumentFragment, MCPTool};
use crate::tools::vector_docs_tool::VectorDocsTool;
use crate::tools::doc_processor::DocumentProcessor;

/// 增强的文档处理器
pub struct EnhancedDocumentProcessor {
    /// 基础文档处理器
    base_processor: DocumentProcessor,
    /// 向量工具
    vector_tool: Arc<VectorDocsTool>,
    /// 配置
    config: ProcessorConfig,
}

/// 处理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// 最大文档长度（字符）
    pub max_document_length: usize,
    /// 文档分块大小
    pub chunk_size: usize,
    /// 分块重叠大小
    pub chunk_overlap: usize,
    /// 最大重试次数
    pub max_retries: u32,
    /// 请求超时时间（秒）
    pub request_timeout_secs: u64,
    /// 启用智能分块
    pub enable_smart_chunking: bool,
    /// 启用内容过滤
    pub enable_content_filtering: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            max_document_length: 10000,
            chunk_size: 1000,
            chunk_overlap: 100,
            max_retries: 3,
            request_timeout_secs: 30,
            enable_smart_chunking: true,
            enable_content_filtering: true,
        }
    }
}

/// 文档分块结果
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub id: String,
    pub content: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub metadata: ChunkMetadata,
}

/// 分块元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub original_file: String,
    pub language: String,
    pub package_name: String,
    pub version: String,
    pub content_type: String,
    pub keywords: Vec<String>,
    pub importance_score: f32,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSearchResult {
    pub fragment: FileDocumentFragment,
    pub score: f32,
    pub relevance_explanation: String,
    pub matched_keywords: Vec<String>,
    pub content_preview: String,
}

impl EnhancedDocumentProcessor {
    /// 创建新的增强文档处理器
    pub async fn new(vector_tool: Arc<VectorDocsTool>) -> Result<Self> {
        let base_processor = DocumentProcessor::new().await?;
        
        Ok(Self {
            base_processor,
            vector_tool,
            config: ProcessorConfig::default(),
        })
    }
    
    /// 使用自定义配置创建处理器
    pub async fn with_config(config: ProcessorConfig, vector_tool: Arc<VectorDocsTool>) -> Result<Self> {
        let mut processor = Self::new(vector_tool).await?;
        processor.config = config;
        Ok(processor)
    }
    
    /// 处理文档请求的主要入口点（增强版）
    pub async fn process_documentation_request_enhanced(
        &self,
        language: &str,
        package_name: &str,
        version: Option<&str>,
        query: &str,
    ) -> Result<Vec<EnhancedSearchResult>> {
        let version = version.unwrap_or("latest");
        
        info!("🔍 增强处理文档请求: {} {} {} - 查询: {}", language, package_name, version, query);
        
        // 1. 首先尝试智能搜索现有文档
        if let Ok(search_results) = self.smart_search_existing_docs(language, package_name, version, query).await {
            if !search_results.is_empty() {
                info!("✅ 从向量库找到 {} 个相关文档", search_results.len());
                return Ok(search_results);
            }
        }
        
        // 2. 生成新文档（带重试机制）
        info!("📝 向量库中没有找到相关文档，开始生成新文档");
        let fragments = self.generate_docs_with_retry(language, package_name, version).await?;
        
        if fragments.is_empty() {
            warn!("⚠️ 没有生成任何文档片段");
            return Ok(Vec::new());
        }
        
        // 3. 智能分块和向量化
        let chunks = self.smart_chunk_documents(&fragments).await?;
        self.vectorize_and_store_chunks(&chunks).await?;
        
        // 4. 再次搜索以返回相关结果
        let search_results = self.smart_search_existing_docs(language, package_name, version, query).await
            .unwrap_or_else(|_| self.create_fallback_results(&fragments, query));
        
        Ok(search_results)
    }
    
    /// 智能搜索现有文档
    async fn smart_search_existing_docs(
        &self,
        language: &str,
        package_name: &str,
        _version: &str,
        query: &str,
    ) -> Result<Vec<EnhancedSearchResult>> {
        // 构建增强的搜索查询
        let enhanced_query = self.build_enhanced_query(language, package_name, query);
        
        let search_params = serde_json::json!({
            "action": "search",
            "query": enhanced_query,
            "limit": 10
        });
        
        let search_result = self.vector_tool.execute(search_params).await?;
        
        if search_result["status"] == "success" && search_result["results_count"].as_u64().unwrap_or(0) > 0 {
            let empty_vec = vec![];
            let results = search_result["results"].as_array().unwrap_or(&empty_vec);
            let mut enhanced_results = Vec::new();
            
            for result in results {
                if let Some(enhanced_result) = self.create_enhanced_result(result, query, language).await {
                    enhanced_results.push(enhanced_result);
                }
            }
            
            // 按相关性排序
            enhanced_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            
            Ok(enhanced_results)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// 构建增强的搜索查询
    fn build_enhanced_query(&self, language: &str, package_name: &str, query: &str) -> String {
        let keywords = vec![
            language.to_string(),
            package_name.to_string(),
            query.to_string(),
        ];
        
        // 添加语言特定的关键词
        let mut enhanced_keywords = keywords;
        match language.to_lowercase().as_str() {
            "rust" => {
                enhanced_keywords.extend(vec!["crate".to_string(), "cargo".to_string()]);
            }
            "python" => {
                enhanced_keywords.extend(vec!["pip".to_string(), "package".to_string()]);
            }
            "javascript" => {
                enhanced_keywords.extend(vec!["npm".to_string(), "node".to_string()]);
            }
            "go" => {
                enhanced_keywords.extend(vec!["module".to_string(), "pkg".to_string()]);
            }
            "java" => {
                enhanced_keywords.extend(vec!["maven".to_string(), "jar".to_string()]);
            }
            _ => {}
        }
        
        enhanced_keywords.join(" ")
    }
    
    /// 创建增强的搜索结果
    async fn create_enhanced_result(
        &self,
        result: &serde_json::Value,
        query: &str,
        expected_language: &str,
    ) -> Option<EnhancedSearchResult> {
        let title = result["title"].as_str()?;
        let content = result["content"].as_str()?;
        let score = result["score"].as_f64().unwrap_or(0.0) as f32;
        
        // 提取语言信息
        let language = result["language"].as_str().unwrap_or("unknown");
        
        // 计算相关性
        let relevance_score = self.calculate_relevance(content, query, language, expected_language);
        let final_score = (score + relevance_score) / 2.0;
        
        // 生成相关性解释
        let relevance_explanation = self.generate_relevance_explanation(content, query, language);
        
        // 提取匹配的关键词
        let matched_keywords = self.extract_matched_keywords(content, query);
        
        // 生成内容预览
        let content_preview = self.generate_content_preview(content, query);
        
        // 创建文档片段
        let fragment = FileDocumentFragment::new(
            language.to_string(),
            result["package"].as_str().unwrap_or("unknown").to_string(),
            result["version"].as_str().unwrap_or("latest").to_string(),
            title.to_string(),
            content.to_string(),
        );
        
        Some(EnhancedSearchResult {
            fragment,
            score: final_score,
            relevance_explanation,
            matched_keywords,
            content_preview,
        })
    }
    
    /// 计算相关性分数
    fn calculate_relevance(&self, content: &str, query: &str, language: &str, expected_language: &str) -> f32 {
        let mut score = 0.0;
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        
        // 语言匹配
        if language == expected_language {
            score += 0.3;
        }
        
        // 查询词匹配
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let matched_words = query_words.iter()
            .filter(|word| content_lower.contains(*word))
            .count();
        
        if !query_words.is_empty() {
            score += (matched_words as f32 / query_words.len() as f32) * 0.5;
        }
        
        // 内容质量评估
        if content.len() > 100 {
            score += 0.1;
        }
        if content.len() > 500 {
            score += 0.1;
        }
        
        score.min(1.0)
    }
    
    /// 生成相关性解释
    fn generate_relevance_explanation(&self, content: &str, query: &str, language: &str) -> String {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        
        let mut explanations = Vec::new();
        
        // 检查直接匹配
        if content_lower.contains(&query_lower) {
            explanations.push(format!("包含查询词 '{}'", query));
        }
        
        // 检查部分匹配
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let matched_words: Vec<&str> = query_words.iter()
            .filter(|word| content_lower.contains(*word))
            .cloned()
            .collect();
        
        if !matched_words.is_empty() && matched_words.len() < query_words.len() {
            explanations.push(format!("匹配关键词: {}", matched_words.join(", ")));
        }
        
        // 语言匹配
        explanations.push(format!("语言: {}", language));
        
        if explanations.is_empty() {
            "基于向量相似度匹配".to_string()
        } else {
            explanations.join("; ")
        }
    }
    
    /// 提取匹配的关键词
    fn extract_matched_keywords(&self, content: &str, query: &str) -> Vec<String> {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        query_words.iter()
            .filter(|word| content_lower.contains(*word))
            .map(|word| word.to_string())
            .collect()
    }
    
    /// 生成内容预览
    fn generate_content_preview(&self, content: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        
        // 查找包含查询词的句子
        for sentence in content.split('.') {
            if sentence.to_lowercase().contains(&query_lower) {
                let trimmed = sentence.trim();
                if trimmed.len() > 20 {
                    return if trimmed.len() > 200 {
                        format!("{}...", &trimmed[..200])
                    } else {
                        trimmed.to_string()
                    };
                }
            }
        }
        
        // 如果没有找到包含查询词的句子，返回开头部分
        if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content.to_string()
        }
    }
    
    /// 带重试机制的文档生成
    async fn generate_docs_with_retry(
        &self,
        language: &str,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<FileDocumentFragment>> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            info!("📝 尝试生成文档 (第 {}/{} 次): {} {} {}", 
                  attempt, self.config.max_retries, language, package_name, version);
            
            match timeout(
                Duration::from_secs(self.config.request_timeout_secs),
                self.base_processor.process_documentation_request(language, package_name, Some(version), "documentation")
            ).await {
                Ok(Ok(fragments)) => {
                    if !fragments.is_empty() {
                        info!("✅ 成功生成 {} 个文档片段", fragments.len());
                        return Ok(fragments);
                    } else {
                        warn!("⚠️ 生成了空的文档片段列表");
                    }
                }
                Ok(Err(e)) => {
                    warn!("⚠️ 第 {} 次尝试失败: {}", attempt, e);
                    last_error = Some(e);
                }
                Err(_) => {
                    warn!("⚠️ 第 {} 次尝试超时", attempt);
                    last_error = Some(anyhow!("请求超时"));
                }
            }
            
            if attempt < self.config.max_retries {
                tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("所有重试都失败了")))
    }
    
    /// 智能文档分块
    async fn smart_chunk_documents(&self, fragments: &[FileDocumentFragment]) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        
        for fragment in fragments {
            let fragment_chunks = if self.config.enable_smart_chunking {
                self.smart_chunk_single_document(fragment).await?
            } else {
                self.simple_chunk_document(fragment)
            };
            
            chunks.extend(fragment_chunks);
        }
        
        info!("📦 总共创建了 {} 个文档分块", chunks.len());
        Ok(chunks)
    }
    
    /// 智能分块单个文档
    async fn smart_chunk_single_document(&self, fragment: &FileDocumentFragment) -> Result<Vec<DocumentChunk>> {
        let content = &fragment.content;
        
        // 如果文档较短，不需要分块
        if content.len() <= self.config.chunk_size {
            return Ok(vec![self.create_single_chunk(fragment, 0, 1)]);
        }
        
        // 按段落分块
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_index = 0;
        
        for paragraph in paragraphs {
            if current_chunk.len() + paragraph.len() > self.config.chunk_size && !current_chunk.is_empty() {
                // 创建当前分块
                chunks.push(self.create_chunk_from_content(fragment, &current_chunk, chunk_index));
                chunk_index += 1;
                
                // 开始新分块（保留重叠）
                current_chunk = if current_chunk.len() > self.config.chunk_overlap {
                    current_chunk[current_chunk.len() - self.config.chunk_overlap..].to_string()
                } else {
                    String::new()
                };
            }
            
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
        }
        
        // 添加最后一个分块
        if !current_chunk.is_empty() {
            chunks.push(self.create_chunk_from_content(fragment, &current_chunk, chunk_index));
        }
        
        // 更新总分块数
        let total_chunks = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total_chunks;
        }
        
        Ok(chunks)
    }
    
    /// 简单分块文档
    fn simple_chunk_document(&self, fragment: &FileDocumentFragment) -> Vec<DocumentChunk> {
        let content = &fragment.content;
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;
        
        while start < content.len() {
            let end = (start + self.config.chunk_size).min(content.len());
            let chunk_content = content[start..end].to_string();
            
            chunks.push(self.create_chunk_from_content(fragment, &chunk_content, chunk_index));
            
            start = if end == content.len() {
                break;
            } else {
                (end - self.config.chunk_overlap).max(start + 1)
            };
            chunk_index += 1;
        }
        
        // 更新总分块数
        let total_chunks = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total_chunks;
        }
        
        chunks
    }
    
    /// 创建单个分块
    fn create_single_chunk(&self, fragment: &FileDocumentFragment, chunk_index: usize, _total_chunks: usize) -> DocumentChunk {
        self.create_chunk_from_content(fragment, &fragment.content, chunk_index)
    }
    
    /// 从内容创建分块
    fn create_chunk_from_content(&self, fragment: &FileDocumentFragment, content: &str, chunk_index: usize) -> DocumentChunk {
        let chunk_id = format!("{}_{}", fragment.id, chunk_index);
        let keywords = self.extract_keywords(content);
        let importance_score = self.calculate_importance_score(content);
        
        DocumentChunk {
            id: chunk_id,
            content: content.to_string(),
            chunk_index,
            total_chunks: 1, // 将在后面更新
            metadata: ChunkMetadata {
                original_file: fragment.file_path.clone(),
                language: fragment.language.clone(),
                package_name: fragment.package_name.clone(),
                version: fragment.version.clone(),
                content_type: "documentation".to_string(),
                keywords,
                importance_score,
            },
        }
    }
    
    /// 提取关键词
    fn extract_keywords(&self, content: &str) -> Vec<String> {
        let words: Vec<&str> = content
            .split_whitespace()
            .filter(|word| word.len() > 3)
            .take(10)
            .collect();
        
        words.iter().map(|word| word.to_lowercase()).collect()
    }
    
    /// 计算重要性分数
    fn calculate_importance_score(&self, content: &str) -> f32 {
        let mut score = 0.0;
        
        // 基于长度
        score += (content.len() as f32 / 1000.0).min(0.3);
        
        // 基于关键词
        let important_keywords = ["function", "class", "method", "api", "example", "usage"];
        let content_lower = content.to_lowercase();
        for keyword in important_keywords {
            if content_lower.contains(keyword) {
                score += 0.1;
            }
        }
        
        score.min(1.0)
    }
    
    /// 向量化并存储分块
    async fn vectorize_and_store_chunks(&self, chunks: &[DocumentChunk]) -> Result<()> {
        info!("🔄 开始向量化并存储 {} 个文档分块", chunks.len());
        
        let mut successful_stores = 0;
        let mut failed_stores = 0;
        
        for chunk in chunks {
            // 检查内容长度，如果太长则跳过
            if chunk.content.len() > self.config.max_document_length {
                warn!("⚠️ 跳过过长的分块: {} ({} 字符)", chunk.id, chunk.content.len());
                failed_stores += 1;
                continue;
            }
            
            let store_params = serde_json::json!({
                "action": "store",
                "title": format!("{} - 分块 {}", chunk.metadata.original_file, chunk.chunk_index),
                "content": chunk.content,
                "language": chunk.metadata.language,
                "package": chunk.metadata.package_name,
                "version": chunk.metadata.version,
                "metadata": {
                    "chunk_id": chunk.id,
                    "chunk_index": chunk.chunk_index,
                    "total_chunks": chunk.total_chunks,
                    "content_type": chunk.metadata.content_type,
                    "keywords": chunk.metadata.keywords,
                    "importance_score": chunk.metadata.importance_score
                }
            });
            
            match self.vector_tool.execute(store_params).await {
                Ok(result) => {
                    if result["status"] == "success" {
                        debug!("✅ 成功存储分块: {}", chunk.id);
                        successful_stores += 1;
                    } else {
                        warn!("⚠️ 存储分块失败: {} - {}", chunk.id, result["error"]);
                        failed_stores += 1;
                    }
                }
                Err(e) => {
                    warn!("⚠️ 存储分块时发生错误: {} - {}", chunk.id, e);
                    failed_stores += 1;
                }
            }
        }
        
        info!("📊 分块存储完成: {} 成功, {} 失败", successful_stores, failed_stores);
        Ok(())
    }
    
    /// 创建回退结果
    fn create_fallback_results(&self, fragments: &[FileDocumentFragment], query: &str) -> Vec<EnhancedSearchResult> {
        fragments.iter().map(|fragment| {
            let score = self.calculate_relevance(&fragment.content, query, &fragment.language, &fragment.language);
            let relevance_explanation = "基于生成的文档内容".to_string();
            let matched_keywords = self.extract_matched_keywords(&fragment.content, query);
            let content_preview = self.generate_content_preview(&fragment.content, query);
            
            EnhancedSearchResult {
                fragment: fragment.clone(),
                score,
                relevance_explanation,
                matched_keywords,
                content_preview,
            }
        }).collect()
    }
    
    /// 获取处理器统计信息
    pub async fn get_processor_stats(&self) -> Result<ProcessorStats> {
        let stats_params = serde_json::json!({
            "action": "stats"
        });
        
        let result = self.vector_tool.execute(stats_params).await?;
        
        Ok(ProcessorStats {
            total_documents: result["total_documents"].as_u64().unwrap_or(0),
            total_vectors: result["total_vectors"].as_u64().unwrap_or(0),
            supported_languages: vec!["rust".to_string(), "python".to_string(), "javascript".to_string(), "go".to_string(), "java".to_string()],
            config: self.config.clone(),
        })
    }
}

/// 处理器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorStats {
    pub total_documents: u64,
    pub total_vectors: u64,
    pub supported_languages: Vec<String>,
    pub config: ProcessorConfig,
} 