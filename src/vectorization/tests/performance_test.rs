use std::time::{Duration, Instant};
use anyhow::Result;
use serde_json::json;

use crate::tools::base::{FileVectorizer, DocumentVector, FileDocumentFragment, FileVectorMetadata};
use crate::vectorization::{VectorizationPerformanceOptimizer, PerformanceConfig, PerformanceMonitor};

/// 模拟向量化器，用于测试
struct MockVectorizer {
    delay_ms: u64,
}

impl MockVectorizer {
    fn new(delay_ms: u64) -> Self {
        Self { delay_ms }
    }
}

#[async_trait::async_trait]
impl FileVectorizer for MockVectorizer {
    async fn vectorize_file(&self, fragment: &FileDocumentFragment) -> Result<DocumentVector> {
        // 模拟处理延迟
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        
        // 生成模拟向量
        let vector_data = vec![0.1, 0.2, 0.3, 0.4, 0.5]; // 5维向量
        
        Ok(DocumentVector {
            data: vector_data,
            dimension: 5,
            metadata: FileVectorMetadata {
                doc_id: fragment.id.clone(),
                language: fragment.language.clone(),
                package_name: fragment.package_name.clone(),
                version: fragment.version.clone(),
                file_path: fragment.file_path.clone(),
                hierarchy_path: fragment.hierarchy_path.clone(),
                keywords: vec!["test".to_string(), "mock".to_string()],
                content_hash: "mock_hash".to_string(),
                content_length: fragment.content.len(),
                created_at: std::time::SystemTime::now(),
                updated_at: std::time::SystemTime::now(),
            },
        })
    }

    async fn vectorize_files_batch(&self, fragments: &[FileDocumentFragment]) -> Result<Vec<DocumentVector>> {
        let mut results = Vec::new();
        for fragment in fragments {
            results.push(self.vectorize_file(fragment).await?);
        }
        Ok(results)
    }

    async fn vectorize_query(&self, _query: &str) -> Result<Vec<f32>> {
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        Ok(vec![0.1, 0.2, 0.3, 0.4, 0.5])
    }
}

/// 创建测试文档片段
fn create_test_fragments(count: usize) -> Vec<FileDocumentFragment> {
    (0..count)
        .map(|i| FileDocumentFragment {
            id: format!("rust/test-package-{}/1.0.0/src/lib{}.rs", i, i),
            package_name: format!("test-package-{}", i),
            version: "1.0.0".to_string(),
            language: "rust".to_string(),
            file_path: format!("src/lib{}.rs", i),
            content: format!("// Test file {}\npub fn test_function_{}() {{}}", i, i),
            hierarchy_path: vec!["src".to_string(), format!("lib{}.rs", i)],
            file_type: crate::tools::base::FileType::Source,
            created_at: std::time::SystemTime::now(),
        })
        .collect()
}

/// 测试基本性能优化功能
#[tokio::test]
async fn test_basic_performance_optimization() -> Result<()> {
    println!("⚡ 测试基本性能优化功能");

    let mock_vectorizer = MockVectorizer::new(50); // 50ms延迟
    let config = PerformanceConfig {
        batch_size: 10,
        max_concurrent: 5,
        cache_size: 100,
        cache_ttl_secs: 300,
        warmup_cache_size: 20,
        enable_metrics: true,
    };

    let optimizer = VectorizationPerformanceOptimizer::new(mock_vectorizer, config);
    let test_fragments = create_test_fragments(5);

    // 第一次向量化（缓存未命中）
    let start_time = Instant::now();
    let results1 = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;
    let first_duration = start_time.elapsed();

    assert_eq!(results1.len(), 5);
    println!("✅ 第一次批量向量化完成，耗时: {:?}", first_duration);

    // 第二次向量化（缓存命中）
    let start_time = Instant::now();
    let results2 = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;
    let second_duration = start_time.elapsed();

    assert_eq!(results2.len(), 5);
    println!("✅ 第二次批量向量化完成，耗时: {:?}", second_duration);

    // 第二次应该明显更快（缓存命中）
    assert!(second_duration < first_duration / 2);

    // 检查性能指标
    let metrics = optimizer.get_metrics().await;
    println!("📊 性能指标:");
    println!("  - 总处理文件数: {}", metrics.total_files_processed);
    println!("  - 缓存命中次数: {}", metrics.cache_hits);
    println!("  - 缓存未命中次数: {}", metrics.cache_misses);
    println!("  - 缓存命中率: {:.2}%", metrics.cache_hit_rate() * 100.0);
    println!("  - 平均处理时间: {:.2}ms", metrics.avg_processing_time_ms);

    assert!(metrics.cache_hits > 0);
    assert!(metrics.cache_hit_rate() > 0.0);

    Ok(())
}

/// 测试缓存预热功能
#[tokio::test]
async fn test_cache_warmup() -> Result<()> {
    println!("🔥 测试缓存预热功能");

    let mock_vectorizer = MockVectorizer::new(30);
    let config = PerformanceConfig {
        warmup_cache_size: 10,
        ..Default::default()
    };

    let optimizer = VectorizationPerformanceOptimizer::new(mock_vectorizer, config);
    let test_fragments = create_test_fragments(20);

    // 预热缓存
    let warmup_start = Instant::now();
    optimizer.warmup_cache(&test_fragments).await?;
    let warmup_duration = warmup_start.elapsed();
    println!("✅ 缓存预热完成，耗时: {:?}", warmup_duration);

    // 测试预热后的性能
    let query_start = Instant::now();
    let results = optimizer.vectorize_files_batch_optimized(&test_fragments[..5]).await?;
    let query_duration = query_start.elapsed();

    assert_eq!(results.len(), 5);
    println!("✅ 预热后查询完成，耗时: {:?}", query_duration);

    // 检查缓存统计
    let cache_stats = optimizer.get_cache_stats().await;
    println!("📊 缓存统计:");
    for (key, value) in cache_stats {
        println!("  - {}: {}", key, value);
    }

    let metrics = optimizer.get_metrics().await;
    assert!(metrics.cache_hits > 0);

    Ok(())
}

/// 测试并发控制
#[tokio::test]
async fn test_concurrency_control() -> Result<()> {
    println!("🔄 测试并发控制");

    let mock_vectorizer = MockVectorizer::new(100); // 较长延迟
    let config = PerformanceConfig {
        max_concurrent: 3,
        batch_size: 5,
        ..Default::default()
    };

    let optimizer = VectorizationPerformanceOptimizer::new(mock_vectorizer, config);
    let test_fragments = create_test_fragments(15);

    let start_time = Instant::now();
    let results = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;
    let duration = start_time.elapsed();

    assert_eq!(results.len(), 15);
    println!("✅ 并发控制测试完成，耗时: {:?}", duration);

    // 验证并发控制生效（应该比串行快，但不会太快）
    let expected_min_duration = Duration::from_millis(300); // 至少3批次 * 100ms
    let expected_max_duration = Duration::from_millis(1500); // 不应该太慢
    
    assert!(duration >= expected_min_duration);
    assert!(duration <= expected_max_duration);

    Ok(())
}

/// 测试性能监控器
#[tokio::test]
async fn test_performance_monitor() -> Result<()> {
    println!("📊 测试性能监控器");

    let mut monitor = PerformanceMonitor::new();

    // 模拟一些操作
    tokio::time::sleep(Duration::from_millis(50)).await;
    monitor.checkpoint("初始化完成");

    tokio::time::sleep(Duration::from_millis(100)).await;
    monitor.checkpoint("数据加载完成");

    tokio::time::sleep(Duration::from_millis(75)).await;
    monitor.checkpoint("处理完成");

    // 检查总耗时
    let total_duration = monitor.total_duration();
    assert!(total_duration >= Duration::from_millis(225));

    // 检查检查点耗时
    let checkpoint_durations = monitor.checkpoint_durations();
    assert_eq!(checkpoint_durations.len(), 3);

    println!("✅ 性能监控器测试完成");
    monitor.print_report();

    Ok(())
}

/// 测试自适应批量大小调整
#[tokio::test]
async fn test_adaptive_batch_size() -> Result<()> {
    println!("🎯 测试自适应批量大小调整");

    let mock_vectorizer = MockVectorizer::new(20);
    let config = PerformanceConfig {
        batch_size: 50,
        ..Default::default()
    };

    let mut optimizer = VectorizationPerformanceOptimizer::new(mock_vectorizer, config);
    let test_fragments = create_test_fragments(10);

    // 第一次处理（缓存未命中，低命中率）
    let _ = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;
    
    // 调整批量大小
    optimizer.adaptive_batch_size_adjustment().await;
    
    // 第二次处理（缓存命中，高命中率）
    let _ = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;
    
    // 再次调整批量大小
    optimizer.adaptive_batch_size_adjustment().await;

    let metrics = optimizer.get_metrics().await;
    println!("📊 自适应调整后的指标:");
    println!("  - 缓存命中率: {:.2}%", metrics.cache_hit_rate() * 100.0);

    Ok(())
}

/// 测试处理时间预测
#[tokio::test]
async fn test_processing_time_estimation() -> Result<()> {
    println!("⏱️ 测试处理时间预测");

    let mock_vectorizer = MockVectorizer::new(40);
    let config = PerformanceConfig::default();

    let optimizer = VectorizationPerformanceOptimizer::new(mock_vectorizer, config);
    let test_fragments = create_test_fragments(5);

    // 处理一些文件以建立基准
    let _ = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;

    // 预测处理时间
    let estimated_time = optimizer.estimate_processing_time(10).await;
    println!("✅ 预测处理10个文件需要: {:?}", estimated_time);

    // 实际测试
    let test_fragments_10 = create_test_fragments(10);
    let start_time = Instant::now();
    let _ = optimizer.vectorize_files_batch_optimized(&test_fragments_10).await?;
    let actual_time = start_time.elapsed();

    println!("✅ 实际处理时间: {:?}", actual_time);

    // 预测应该在合理范围内（考虑缓存命中）
    assert!(estimated_time > Duration::from_millis(100));

    Ok(())
}

/// 压力测试
#[tokio::test]
async fn test_stress_performance() -> Result<()> {
    println!("💪 压力测试");

    let mock_vectorizer = MockVectorizer::new(10); // 快速处理
    let config = PerformanceConfig {
        batch_size: 20,
        max_concurrent: 8,
        cache_size: 1000,
        ..Default::default()
    };

    let optimizer = VectorizationPerformanceOptimizer::new(mock_vectorizer, config);
    let test_fragments = create_test_fragments(100); // 大量文件

    let start_time = Instant::now();
    let results = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;
    let duration = start_time.elapsed();

    assert_eq!(results.len(), 100);
    println!("✅ 压力测试完成，处理100个文件耗时: {:?}", duration);

    // 第二次处理（全部缓存命中）
    let start_time = Instant::now();
    let results2 = optimizer.vectorize_files_batch_optimized(&test_fragments).await?;
    let cached_duration = start_time.elapsed();

    assert_eq!(results2.len(), 100);
    println!("✅ 缓存命中测试完成，耗时: {:?}", cached_duration);

    // 缓存命中应该显著更快
    assert!(cached_duration < duration / 5);

    let metrics = optimizer.get_metrics().await;
    println!("📊 压力测试指标:");
    println!("  - 总处理文件数: {}", metrics.total_files_processed);
    println!("  - 缓存命中率: {:.2}%", metrics.cache_hit_rate() * 100.0);
    println!("  - 平均处理时间: {:.2}ms", metrics.avg_processing_time_ms);
    println!("  - 批量处理次数: {}", metrics.batch_count);

    Ok(())
} 