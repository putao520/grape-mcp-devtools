pub mod data_models;
pub mod collectors;
pub mod enhanced_collectors;
pub mod services;
pub mod storage;
pub mod tools;

// 新增AI驱动采集系统
pub mod ai_collector;
pub mod intelligent_scraper;
pub mod content_analyzer;
pub mod url_discovery;
pub mod doc_crawler;

// 新增增强内容提取器和智能URL分析器模块
pub mod smart_url_analyzer;

// 重新导出核心类型
pub use data_models::{
    LanguageVersion, VersionStatus, LanguageFeature, FeatureCategory, 
    FeatureStability, ImpactLevel, CodeExample, SyntaxChange, SyntaxChangeType,
    Deprecation, DeprecationLevel, BreakingChange, PerformanceImprovement,
    StdlibChange, StdlibChangeType, ToolchainChange, VersionMetadata,
    VersionComparison, FeatureModification, ModificationType, UpgradeRecommendation,
    RecommendationPriority
};
pub use services::{LanguageVersionService, VersionComparisonService, ServiceConfig, CacheStats};
pub use tools::LanguageFeaturesTool;

// 导出AI采集系统
pub use ai_collector::*;
pub use intelligent_scraper::*;
pub use content_analyzer::*;
pub use url_discovery::*;
pub use doc_crawler::{
    DocCrawlerEngine, DocCrawlerConfig, LibraryDocumentation, LibraryFeature,
    ApiDoc, FunctionDoc, ClassDoc, TypeDoc, ConstantDoc, Tutorial, LibraryCodeExample,
    InstallationGuide, Dependency, DocMetadata, DocCacheStats
};

// 新增
pub use collectors::{LanguageVersionCollector, CollectorFactory};
pub use enhanced_collectors::{EnhancedLanguageCollector, EnhancedCollectorFactory, CollectorConfig}; 