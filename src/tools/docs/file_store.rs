use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, debug};

use super::doc_traits::*;

/// 文件系统存储的索引结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileStoreIndex {
    /// 文档ID到文件路径的映射
    documents: HashMap<String, String>,
    /// 语言索引
    language_index: HashMap<String, Vec<String>>,
    /// 类型索引
    type_index: HashMap<String, Vec<String>>,
    /// 包名索引
    package_index: HashMap<String, Vec<String>>,
}

impl Default for FileStoreIndex {
    fn default() -> Self {
        Self {
            documents: HashMap::new(),
            language_index: HashMap::new(),
            type_index: HashMap::new(),
            package_index: HashMap::new(),
        }
    }
}

/// 基于文件系统的文档存储实现
pub struct FileDocumentStore {
    /// 存储根目录
    root_dir: PathBuf,
    /// 索引文件路径
    index_path: PathBuf,
    /// 内存中的索引
    index: tokio::sync::RwLock<FileStoreIndex>,
    /// 向量化器
    vectorizer: Box<dyn DocumentVectorizer>,
}

impl FileDocumentStore {
    /// 创建新的文件系统文档存储
    pub async fn new(
        root_dir: impl AsRef<Path>,
        vectorizer: Box<dyn DocumentVectorizer>,
    ) -> Result<Self> {
        let root_dir = root_dir.as_ref().to_path_buf();
        let index_path = root_dir.join("index.json");
        
        // 确保根目录存在
        fs::create_dir_all(&root_dir).await?;
        
        // 加载或创建索引
        let index = if index_path.exists() {
            let index_content = fs::read_to_string(&index_path).await?;
            serde_json::from_str(&index_content).unwrap_or_default()
        } else {
            FileStoreIndex::default()
        };
        
        info!("初始化文件系统文档存储: {:?}", root_dir);
        info!("加载了 {} 个文档", index.documents.len());
        
        Ok(Self {
            root_dir,
            index_path,
            index: tokio::sync::RwLock::new(index),
            vectorizer,
        })
    }
    
    /// 保存索引到文件
    async fn save_index(&self) -> Result<()> {
        let index = self.index.read().await;
        let index_content = serde_json::to_string_pretty(&*index)?;
        fs::write(&self.index_path, index_content).await?;
        Ok(())
    }
    
    /// 获取文档文件路径
    fn get_document_path(&self, id: &str) -> PathBuf {
        // 使用文档ID的前两个字符作为子目录，避免单个目录文件过多
        let prefix = if id.len() >= 2 { &id[..2] } else { "00" };
        self.root_dir.join("documents").join(prefix).join(format!("{}.json", id))
    }
    
    /// 更新索引
    async fn update_index(&self, fragment: &DocumentFragment, file_path: &str) {
        let mut index = self.index.write().await;
        
        // 更新文档映射
        index.documents.insert(fragment.id.clone(), file_path.to_string());
        
        // 更新语言索引
        index.language_index
            .entry(fragment.language.clone())
            .or_insert_with(Vec::new)
            .push(fragment.id.clone());
            
        // 更新类型索引
        let type_key = format!("{:?}", fragment.doc_type);
        index.type_index
            .entry(type_key)
            .or_insert_with(Vec::new)
            .push(fragment.id.clone());
            
        // 更新包名索引
        index.package_index
            .entry(fragment.package_name.clone())
            .or_insert_with(Vec::new)
            .push(fragment.id.clone());
    }
    
    /// 从索引中移除文档
    async fn remove_from_index(&self, id: &str) {
        let mut index = self.index.write().await;
        
        // 从文档映射中移除
        index.documents.remove(id);
        
        // 从各个索引中移除
        for doc_list in index.language_index.values_mut() {
            doc_list.retain(|doc_id| doc_id != id);
        }
        
        for doc_list in index.type_index.values_mut() {
            doc_list.retain(|doc_id| doc_id != id);
        }
        
        for doc_list in index.package_index.values_mut() {
            doc_list.retain(|doc_id| doc_id != id);
        }
    }
    
    /// 搜索匹配的文档ID
    async fn search_matching_ids(&self, filter: &SearchFilter) -> Vec<String> {
        let index = self.index.read().await;
        let mut candidate_ids: Option<Vec<String>> = None;
        
        // 按语言过滤
        if let Some(ref languages) = filter.languages {
            let mut lang_ids = Vec::new();
            for lang in languages {
                if let Some(ids) = index.language_index.get(lang) {
                    lang_ids.extend(ids.clone());
                }
            }
            candidate_ids = Some(lang_ids);
        }
        
        // 按类型过滤
        if let Some(ref doc_types) = filter.doc_types {
            let mut type_ids = Vec::new();
            for doc_type in doc_types {
                let type_key = format!("{:?}", doc_type);
                if let Some(ids) = index.type_index.get(&type_key) {
                    type_ids.extend(ids.clone());
                }
            }
            
            if let Some(ref mut candidates) = candidate_ids {
                candidates.retain(|id| type_ids.contains(id));
            } else {
                candidate_ids = Some(type_ids);
            }
        }
        
        // 如果没有过滤条件，返回所有文档ID
        candidate_ids.unwrap_or_else(|| index.documents.keys().cloned().collect())
    }
}

#[async_trait]
impl DocumentStore for FileDocumentStore {
    async fn store(&self, fragment: &DocumentFragment) -> Result<()> {
        debug!("存储文档片段到文件系统: {}", fragment.id);
        
        let file_path = self.get_document_path(&fragment.id);
        
        // 确保目录存在
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // 生成向量并保存到文档中
        let vector = self.vectorizer.vectorize(&fragment.content).await?;
        
        // 创建扩展的文档结构，包含向量
        #[derive(Serialize)]
        struct StoredDocument {
            #[serde(flatten)]
            fragment: DocumentFragment,
            vector: DocumentVector,
        }
        
        let stored_doc = StoredDocument {
            fragment: fragment.clone(),
            vector,
        };
        
        // 序列化并保存
        let content = serde_json::to_string_pretty(&stored_doc)?;
        fs::write(&file_path, content).await?;
        
        // 更新索引
        let relative_path = file_path.strip_prefix(&self.root_dir)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .to_string();
        self.update_index(fragment, &relative_path).await;
        
        // 保存索引
        self.save_index().await?;
        
        info!("文档片段存储成功: {}", fragment.id);
        Ok(())
    }
    
    async fn get(&self, id: &str) -> Result<Option<DocumentFragment>> {
        debug!("从文件系统获取文档片段: {}", id);
        
        let index = self.index.read().await;
        if let Some(relative_path) = index.documents.get(id) {
            let file_path = self.root_dir.join(relative_path);
            
            if file_path.exists() {
                let content = fs::read_to_string(&file_path).await?;
                
                #[derive(Deserialize)]
                struct StoredDocument {
                    #[serde(flatten)]
                    fragment: DocumentFragment,
                    vector: DocumentVector,
                }
                
                let stored_doc: StoredDocument = serde_json::from_str(&content)?;
                Ok(Some(stored_doc.fragment))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        debug!("从文件系统删除文档片段: {}", id);
        
        let index = self.index.read().await;
        if let Some(relative_path) = index.documents.get(id) {
            let file_path = self.root_dir.join(relative_path);
            
            // 删除文件
            if file_path.exists() {
                fs::remove_file(&file_path).await?;
            }
        }
        drop(index);
        
        // 从索引中移除
        self.remove_from_index(id).await;
        
        // 保存索引
        self.save_index().await?;
        
        info!("文档片段删除成功: {}", id);
        Ok(())
    }
    
    async fn search(&self, query: &str, filter: &SearchFilter) -> Result<Vec<SearchResult>> {
        debug!("在文件系统中搜索: {}", query);
        
        // 获取候选文档ID
        let candidate_ids = self.search_matching_ids(filter).await;
        
        let mut results = Vec::new();
        
        // 遍历候选文档，计算相似度
        for doc_id in candidate_ids {
            if let Ok(Some(fragment)) = self.get(&doc_id).await {
                // 简单的文本匹配作为基础分数
                let text_score = if fragment.content.to_lowercase().contains(&query.to_lowercase()) ||
                                   fragment.title.to_lowercase().contains(&query.to_lowercase()) {
                    0.8
                } else {
                    0.1
                };
                
                // 如果有相似度阈值要求，且文本分数不够，跳过
                if let Some(threshold) = filter.similarity_threshold {
                    if text_score < threshold {
                        continue;
                    }
                }
                
                results.push(SearchResult {
                    fragment,
                    score: text_score,
                });
            }
        }
        
        // 按分数排序
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // 应用数量限制
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }
        
        info!("搜索完成，找到 {} 个结果", results.len());
        Ok(results)
    }
} 