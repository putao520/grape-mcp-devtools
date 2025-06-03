use crate::tools::enhanced_doc_processor::EnhancedDocumentProcessor;
use crate::tools::vector_docs_tool::VectorDocsTool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{info, warn, error, debug};
use anyhow::Result;

// 我们需要从environment_detector获取的信息类型
use crate::tools::environment_detector::LanguageInfo as DetectorLanguageInfo;

/// 后台文档缓存服务配置
#[derive(Clone, Debug)]
pub struct DocCacherConfig {
    pub enabled: bool,
    pub concurrent_tasks: usize,
    // 可以添加更多配置，如忽略列表、优先列表等
}

impl Default for DocCacherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            concurrent_tasks: 2, // 默认2个并发任务
        }
    }
}

/// 简化的依赖信息结构，用于缓存
#[derive(Debug, Clone)]
pub struct SimpleDependency {
    pub name: String,
    pub version: Option<String>,
}

/// 后台文档缓存服务
/// 负责在环境检测到依赖后，异步地获取、处理和缓存这些依赖的文档。
pub struct BackgroundDocCacher {
    config: DocCacherConfig,
    doc_processor: Arc<EnhancedDocumentProcessor>,
    vector_tool: Arc<VectorDocsTool>, 
}

impl BackgroundDocCacher {
    pub fn new(
        config: DocCacherConfig,
        doc_processor: Arc<EnhancedDocumentProcessor>,
        vector_tool: Arc<VectorDocsTool>,
    ) -> Self {
        Self {
            config,
            doc_processor,
            vector_tool,
        }
    }

    /// 将检测到的依赖项加入后台缓存队列
    /// 处理检测到的语言信息，为每种语言的标准库和常用包创建缓存任务
    pub async fn queue_dependencies_for_caching(
        &self,
        detected_languages_map: &HashMap<String, DetectorLanguageInfo>, 
    ) -> Result<()> {
        if !self.config.enabled {
            info!("后台文档缓存服务已禁用。");
            return Ok(());
        }

        info!("启动后台文档缓存任务，并发数: {}", self.config.concurrent_tasks);
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_tasks));
        let mut task_set = JoinSet::new();

        for (language_name, lang_info) in detected_languages_map {
            // 检查是否已经处理过这个语言
            if self.is_language_cached(language_name).await {
                debug!("语言 {} 的文档已缓存，跳过处理", language_name);
                continue;
            }

            // 为每个检测到的语言创建标准库和常用包的缓存任务
            let language_packages = self.get_standard_packages_for_language(language_name);

            for package_info in language_packages {
                // 检查向量数据库中是否已经存在该包的文档
                if self.is_package_already_cached(language_name, &package_info.name).await {
                    debug!("包 {}/{} 已缓存，跳过处理", language_name, package_info.name);
                    continue;
                }
                
                let lang_clone = language_name.clone();
                let pkg_name_clone = package_info.name.clone();
                let pkg_version_clone = package_info.version.clone().unwrap_or_else(|| "latest".to_string());
                
                let doc_processor_clone = Arc::clone(&self.doc_processor);
                let vector_tool_clone = Arc::clone(&self.vector_tool);
                let semaphore_clone = Arc::clone(&semaphore);

                task_set.spawn(async move {
                    let permit = semaphore_clone.acquire().await.expect("信号量获取失败");
                    info!("开始处理文档缓存: {}/{}/{}...", lang_clone, pkg_name_clone, pkg_version_clone);
                    
                    match Self::cache_single_package(
                        doc_processor_clone,
                        vector_tool_clone,
                        &lang_clone,
                        &pkg_name_clone,
                        &pkg_version_clone,
                    ).await {
                        Ok(stats) => {
                            info!(
                                "成功缓存包 {}/{}/{}: {} 个文档片段已处理，{} 个新片段已添加。", 
                                lang_clone, pkg_name_clone, pkg_version_clone, stats.fragments_processed, stats.fragments_added
                            );
                        }
                        Err(e) => {
                            error!(
                                "缓存包 {}/{}/{} 文档失败: {:?}", 
                                lang_clone, pkg_name_clone, pkg_version_clone, e
                            );
                        }
                    }
                    drop(permit); 
                });
            }
        }
        info!("所有依赖的文档缓存任务已派发到后台。主程序将继续运行。");
        Ok(())
    }

    async fn cache_single_package(
        doc_processor: Arc<EnhancedDocumentProcessor>,
        vector_tool: Arc<VectorDocsTool>,
        language: &str,
        package_name: &str,
        version: &str,
    ) -> Result<CacheStats> {
        debug!("获取包 {}/{}/(version: {}) 的文档片段...", language, package_name, version);

        // 使用 EnhancedDocumentProcessor 的增强功能
        // 由于它没有直接的 generate_xxx_docs 方法，我们使用通用的处理方法
        match doc_processor.process_documentation_request_enhanced(language, package_name, Some(version), "documentation").await {
            Ok(results) => {
                if results.is_empty() {
                    info!("未找到包 {}/{}/(version: {}) 的文档片段。", language, package_name, version);
                    return Ok(CacheStats::default());
                }

                debug!("为包 {}/{}/(version: {}) 获取到 {} 个文档片段，准备批量添加到向量库...", language, package_name, version, results.len());
                
                // 将 EnhancedSearchResult 转换为 FileDocumentFragment 进行存储
                let fragments: Vec<_> = results.into_iter().map(|result| result.fragment).collect();
                let added_ids = vector_tool.add_file_fragments_batch(&fragments).await?;
                
                Ok(CacheStats {
                    fragments_processed: fragments.len(),
                    fragments_added: added_ids.len(),
                })
            }
            Err(e) => {
                warn!("后台文档缓存暂不支持语言或获取失败: {} - {}", language, e);
                Ok(CacheStats::default())
            }
        }
    }

    /// 检查语言是否已缓存
    async fn is_language_cached(&self, language: &str) -> bool {
        // 检查该语言是否已有缓存数据
        // 这里可以检查文件系统或内存缓存
        let cache_key = format!("language_docs_{}", language);
        
        // 简单的文件存在检查
        let cache_dir = std::path::Path::new("./cache/language_docs");
        let cache_file = cache_dir.join(format!("{}.json", language));
        
        if cache_file.exists() {
            // 检查文件是否是最近的（例如24小时内）
            if let Ok(metadata) = std::fs::metadata(&cache_file) {
                if let Ok(modified) = metadata.modified() {
                    let duration = std::time::SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or_default();
                    
                    // 如果文件在24小时内修改过，认为缓存仍然有效
                    return duration.as_secs() < 24 * 60 * 60;
                }
            }
        }
        
        false
    }

    /// 获取语言的标准包列表
    fn get_standard_packages_for_language(&self, language: &str) -> Vec<SimpleDependency> {
        match language {
            "rust" => vec![
                SimpleDependency { name: "std".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "serde".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "tokio".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "clap".to_string(), version: Some("latest".to_string()) },
            ],
            "python" => vec![
                SimpleDependency { name: "builtins".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "requests".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "numpy".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "pandas".to_string(), version: Some("latest".to_string()) },
            ],
            "javascript" => vec![
                SimpleDependency { name: "node".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "express".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "lodash".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "axios".to_string(), version: Some("latest".to_string()) },
            ],
            "java" => vec![
                SimpleDependency { name: "java.lang".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "org.springframework:spring-core".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "com.fasterxml.jackson.core:jackson-core".to_string(), version: Some("latest".to_string()) },
            ],
            "go" => vec![
                SimpleDependency { name: "fmt".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "github.com/gin-gonic/gin".to_string(), version: Some("latest".to_string()) },
                SimpleDependency { name: "github.com/gorilla/mux".to_string(), version: Some("latest".to_string()) },
            ],
            _ => vec![
                SimpleDependency { name: format!("{}_std", language), version: Some("latest".to_string()) }
            ],
        }
    }
    
    /// 检查包是否已在向量数据库中缓存
    async fn is_package_already_cached(&self, language: &str, package_name: &str) -> bool {
        // 通过搜索向量数据库来检查是否已有该包的文档
        let search_query = format!("{} {} documentation", language, package_name);
        
        // 生成查询的嵌入向量并搜索
        match self.vector_tool.generate_embedding(&search_query).await {
            Ok(query_embedding) => {
                // 在向量数据库中搜索相似文档（search_similar是同步方法）
                match self.vector_tool.search_similar(&query_embedding, 1) {
                    Ok(results) => {
                        // 如果找到相似结果，检查是否匹配语言和包名
                        !results.is_empty() && results.iter().any(|r| {
                            let score_threshold = 0.8; // 相似度阈值
                            r.score >= score_threshold &&
                            r.package_name.to_lowercase() == package_name.to_lowercase() &&
                            r.language.to_lowercase() == language.to_lowercase()
                        })
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    pub async fn save_config(&self) -> Result<()> {
        // Implementation of save_config method
        Ok(())
    }
}

#[derive(Debug, Default)]
struct CacheStats {
    fragments_processed: usize,
    fragments_added: usize,
} 