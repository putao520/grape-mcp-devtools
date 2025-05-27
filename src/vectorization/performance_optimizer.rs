use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use anyhow::Result;
use tracing::{info, warn, debug};
use moka::future::Cache;

use crate::tools::base::{FileVectorizer, DocumentVector, FileDocumentFragment};

/// 性能优化配置
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// 批量处理大小
    pub batch_size: usize,
    /// 最大并发数
    pub max_concurrent: usize,
    /// 缓存大小（条目数）
    pub cache_size: u64,
    /// 缓存TTL（秒）
    pub cache_ttl_secs: u64,
    /// 预热缓存大小
    pub warmup_cache_size: usize,
    /// 启用性能监控
    pub enable_metrics: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            max_concurrent: 10,
            cache_size: 10000,
            cache_ttl_secs: 3600,
            warmup_cache_size: 100,
            enable_metrics: true,
        }
    }
}

/// 性能指标
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// 总处理文件数
    pub total_files_processed: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
    /// 平均处理时间（毫秒）
    pub avg_processing_time_ms: f64,
    /// 批量处理次数
    pub batch_count: u64,
    /// 总处理时间（毫秒）
    pub total_processing_time_ms: u64,
}

impl PerformanceMetrics {
    /// 计算缓存命中率
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    /// 更新处理时间
    pub fn update_processing_time(&mut self, duration_ms: u64) {
        self.total_processing_time_ms += duration_ms;
        self.total_files_processed += 1;
        self.avg_processing_time_ms = self.total_processing_time_ms as f64 / self.total_files_processed as f64;
    }
}

/// 向量化性能优化器
pub struct VectorizationPerformanceOptimizer<T: FileVectorizer> {
    /// 底层向量化器
    vectorizer: Arc<T>,
    /// 性能配置
    config: PerformanceConfig,
    /// 向量缓存
    vector_cache: Cache<String, DocumentVector>,
    /// 并发控制信号量
    semaphore: Arc<Semaphore>,
    /// 性能指标
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// 预热缓存
    warmup_cache: Arc<RwLock<HashMap<String, DocumentVector>>>,
}

impl<T: FileVectorizer + Send + Sync + 'static> VectorizationPerformanceOptimizer<T> {
    /// 创建新的性能优化器
    pub fn new(vectorizer: T, config: PerformanceConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.cache_size)
            .time_to_live(Duration::from_secs(config.cache_ttl_secs))
            .build();

        Self {
            vectorizer: Arc::new(vectorizer),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
            vector_cache: cache,
            warmup_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            config,
        }
    }

    /// 生成缓存键
    fn generate_cache_key(&self, fragment: &FileDocumentFragment) -> String {
        format!("{}:{}:{}:{}", 
            fragment.package_name, 
            fragment.version, 
            fragment.language,
            fragment.file_path
        )
    }

    /// 预热缓存
    pub async fn warmup_cache(&self, fragments: &[FileDocumentFragment]) -> Result<()> {
        info!("开始预热向量化缓存，文件数: {}", fragments.len());
        let start_time = Instant::now();

        let warmup_fragments = fragments.iter()
            .take(self.config.warmup_cache_size)
            .collect::<Vec<_>>();

        // 批量向量化预热数据
        let vectors = self.vectorize_files_batch_internal(&warmup_fragments).await?;

        // 存储到预热缓存
        {
            let mut warmup_cache = self.warmup_cache.write().await;
            for (fragment, vector) in warmup_fragments.iter().zip(vectors.iter()) {
                let cache_key = self.generate_cache_key(fragment);
                warmup_cache.insert(cache_key.clone(), vector.clone());
                // 同时存储到主缓存
                self.vector_cache.insert(cache_key, vector.clone()).await;
            }
        }

        let duration = start_time.elapsed();
        info!("缓存预热完成，耗时: {:?}", duration);
        Ok(())
    }

    /// 优化的单文件向量化
    pub async fn vectorize_file_optimized(&self, fragment: &FileDocumentFragment) -> Result<DocumentVector> {
        let cache_key = self.generate_cache_key(fragment);
        let start_time = Instant::now();

        // 1. 检查主缓存
        if let Some(cached_vector) = self.vector_cache.get(&cache_key).await {
            {
                let mut metrics = self.metrics.write().await;
                metrics.cache_hits += 1;
            }
            debug!("从主缓存返回向量: {}", cache_key);
            return Ok(cached_vector);
        }

        // 2. 检查预热缓存
        {
            let warmup_cache = self.warmup_cache.read().await;
            if let Some(cached_vector) = warmup_cache.get(&cache_key) {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.cache_hits += 1;
                }
                debug!("从预热缓存返回向量: {}", cache_key);
                // 同时存储到主缓存
                self.vector_cache.insert(cache_key, cached_vector.clone()).await;
                return Ok(cached_vector.clone());
            }
        }

        // 3. 缓存未命中，执行向量化
        {
            let mut metrics = self.metrics.write().await;
            metrics.cache_misses += 1;
        }

        // 获取并发许可
        let _permit = self.semaphore.acquire().await?;
        
        let vector = self.vectorizer.vectorize_file(fragment).await?;
        
        // 存储到缓存
        self.vector_cache.insert(cache_key, vector.clone()).await;

        // 更新性能指标
        if self.config.enable_metrics {
            let duration = start_time.elapsed();
            let mut metrics = self.metrics.write().await;
            metrics.update_processing_time(duration.as_millis() as u64);
        }

        Ok(vector)
    }

    /// 优化的批量向量化
    pub async fn vectorize_files_batch_optimized(&self, fragments: &[FileDocumentFragment]) -> Result<Vec<DocumentVector>> {
        let start_time = Instant::now();
        info!("开始批量向量化，文件数: {}", fragments.len());

        let mut results = Vec::with_capacity(fragments.len());
        let mut uncached_fragments = Vec::new();
        let mut uncached_indices = Vec::new();

        // 1. 检查缓存，分离已缓存和未缓存的文件
        for (index, fragment) in fragments.iter().enumerate() {
            let cache_key = self.generate_cache_key(fragment);
            
            if let Some(cached_vector) = self.vector_cache.get(&cache_key).await {
                results.push((index, cached_vector));
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.cache_hits += 1;
                }
            } else {
                uncached_fragments.push(fragment);
                uncached_indices.push(index);
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.cache_misses += 1;
                }
            }
        }

        // 2. 批量处理未缓存的文件
        if !uncached_fragments.is_empty() {
            info!("批量向量化未缓存文件数: {}", uncached_fragments.len());
            let uncached_vectors = self.vectorize_files_batch_internal(&uncached_fragments).await?;
            
            // 存储到缓存并添加到结果
            for (fragment, vector) in uncached_fragments.iter().zip(uncached_vectors.iter()) {
                let cache_key = self.generate_cache_key(fragment);
                self.vector_cache.insert(cache_key, vector.clone()).await;
            }

            // 将结果按原始索引排序
            for (index, vector) in uncached_indices.into_iter().zip(uncached_vectors.into_iter()) {
                results.push((index, vector));
            }
        }

        // 3. 按原始顺序排序结果
        results.sort_by_key(|(index, _)| *index);
        let final_results: Vec<DocumentVector> = results.into_iter().map(|(_, vector)| vector).collect();

        // 更新性能指标
        if self.config.enable_metrics {
            let duration = start_time.elapsed();
            let mut metrics = self.metrics.write().await;
            metrics.batch_count += 1;
            metrics.total_processing_time_ms += duration.as_millis() as u64;
        }

        info!("批量向量化完成，耗时: {:?}", start_time.elapsed());
        Ok(final_results)
    }

    /// 内部批量向量化实现
    async fn vectorize_files_batch_internal(&self, fragments: &[&FileDocumentFragment]) -> Result<Vec<DocumentVector>> {
        let mut all_vectors = Vec::new();
        
        // 按批次处理
        for chunk in fragments.chunks(self.config.batch_size) {
            // 获取并发许可
            let _permit = self.semaphore.acquire().await?;
            
            // 转换为拥有所有权的片段
            let owned_fragments: Vec<FileDocumentFragment> = chunk.iter().map(|&f| f.clone()).collect();
            
            // 批量向量化
            let batch_vectors = self.vectorizer.vectorize_files_batch(&owned_fragments).await?;
            all_vectors.extend(batch_vectors);
        }

        Ok(all_vectors)
    }

    /// 获取性能指标
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// 重置性能指标
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = PerformanceMetrics::default();
    }

    /// 清理缓存
    pub async fn clear_cache(&self) {
        self.vector_cache.invalidate_all();
        let mut warmup_cache = self.warmup_cache.write().await;
        warmup_cache.clear();
        info!("向量化缓存已清理");
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("main_cache_size".to_string(), self.vector_cache.entry_count());
        
        let warmup_cache = self.warmup_cache.read().await;
        stats.insert("warmup_cache_size".to_string(), warmup_cache.len() as u64);
        
        stats
    }

    /// 预测处理时间
    pub async fn estimate_processing_time(&self, file_count: usize) -> Duration {
        let metrics = self.metrics.read().await;
        if metrics.avg_processing_time_ms > 0.0 {
            let estimated_ms = metrics.avg_processing_time_ms * file_count as f64;
            Duration::from_millis(estimated_ms as u64)
        } else {
            // 默认估计：每个文件100ms
            Duration::from_millis(100 * file_count as u64)
        }
    }

    /// 自适应批量大小调整
    pub async fn adaptive_batch_size_adjustment(&mut self) {
        let metrics = self.metrics.read().await;
        
        // 根据缓存命中率调整批量大小
        let hit_rate = metrics.cache_hit_rate();
        
        if hit_rate > 0.8 {
            // 高缓存命中率，可以增加批量大小
            self.config.batch_size = (self.config.batch_size * 120 / 100).min(200);
        } else if hit_rate < 0.3 {
            // 低缓存命中率，减少批量大小以减少内存压力
            self.config.batch_size = (self.config.batch_size * 80 / 100).max(10);
        }

        debug!("自适应调整批量大小为: {}", self.config.batch_size);
    }
}

/// 性能监控器
pub struct PerformanceMonitor {
    start_time: Instant,
    checkpoints: Vec<(String, Instant)>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            checkpoints: Vec::new(),
        }
    }

    /// 添加检查点
    pub fn checkpoint(&mut self, name: &str) {
        self.checkpoints.push((name.to_string(), Instant::now()));
    }

    /// 获取总耗时
    pub fn total_duration(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 获取检查点间隔时间
    pub fn checkpoint_durations(&self) -> Vec<(String, Duration)> {
        let mut durations = Vec::new();
        let mut last_time = self.start_time;

        for (name, time) in &self.checkpoints {
            durations.push((name.clone(), time.duration_since(last_time)));
            last_time = *time;
        }

        durations
    }

    /// 打印性能报告
    pub fn print_report(&self) {
        info!("=== 性能监控报告 ===");
        info!("总耗时: {:?}", self.total_duration());
        
        for (name, duration) in self.checkpoint_durations() {
            info!("  {}: {:?}", name, duration);
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
} 