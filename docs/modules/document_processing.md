# 文档处理模块设计文档

## 模块概览

文档处理模块 (Document Processing Module) 是 `grape-mcp-devtools` 中负责处理和查询编程语言文档的核心组件。它为各种语言特定的文档工具 (`SpecificDocTool`，如 `PythonDocsTool`, `RustDocsTool`) 提供统一的框架和流水线，以实现对不同语言官方文档、社区文档或本地CLI帮助信息的获取、解析、(可选)索引、缓存和格式化搜索。此外，它支持一个全局的 `SearchDocsTool`，该工具可以利用此模块构建的（可选）跨语言索引进行文档搜索。

### 模块基本信息
- **主要路径**: `src/tools/*_docs_tool.rs` (各语言特定工具), `src/tools/doc_processor.rs` (核心处理流水线), `src/tools/search.rs` (全局搜索服务), 相关部分在 `src/language_features/` (智能抓取、内容分析) 和 `src/storage/` (缓存、向量存储)。
- **主要作用**: 文档获取、内容提取与解析、(可选)向量化与索引、缓存管理、格式化文档输出、支持单语言和跨语言文档搜索。
- **核心特性**: 可插拔的文档源、多阶段处理流水线、智能内容提取、多种检索策略、强大的缓存机制、面向AI的可读输出格式。
- **支持的文档类型**: 官方在线文档 (HTML)、通过CLI工具获取的文档 (纯文本/Markdown)、本地Markdown文件等。

## 架构设计

### 1. 模块与工具的关系

- **`SpecificDocTool` (如 `PythonDocsTool`)**: 实现 `MCPTool` trait。它们是用户直接通过MCP协议调用的入口点。每个特定语言的工具负责：
    - 定义该语言的文档源 (URL模板、CLI命令)。
    - 将用户的自然语言查询或特定格式的查询转换为 `DocProcessor` 可以理解的请求。
    - 配置并调用 `DocProcessor` 来处理请求。
    - 格式化 `DocProcessor` 的结果以符合MCP响应规范。
- **`DocProcessor`**: 通用的文档处理流水线，不直接暴露给MCP客户端。它被各种 `SpecificDocTool` 使用。
- **`SearchDocsTool`**: 一个特殊的 `MCPTool`，它可能调用 `SearchService` 来执行跨多个已处理或索引文档源的搜索。

### 2. 内部组件与流程图

```mermaid
graph TD
    subgraph SpecificDocTool_Python [PythonDocsTool]
        PyQuery[User Query: "pydoc math.sin" or "Python requests library examples" or "Rust tokio changelog"]
    end

    subgraph DocProcessor_Pipeline [DocProcessor]
        A[SourceResolver (uses LanguageDocConfig, query keywords like 'examples', 'changelog', 'readme')]
        A -- Chooses Source & Retrieval Strategy --> B{Retrieval Strategy? (CLI, HTTP, Cache)};
        B -- CLI (e.g., `pydoc math.sin`, `git show origin/main:README.md` from a known repo URI) --> C[CLIExecutor (doc_processor.rs calls cli_integration_module)];
        B -- HTTP (e.g., for changelog URLs, specific doc pages, GitHub raw file URLs) --> D[IntelligentScraper (doc_processor.rs calls language_features_module)];
        B -- Cache --> E[CacheManager (doc_processor.rs calls storage_module)];
        
        C -- Raw CLI Output --> F[ContentExtractor (uses ContentAnalyzer from lang_features for complex text)];
        D -- Raw HTML/Text --> F;
        E -- Cached Content --> F;
        
        F -- Main Content (Text, Markdown, Code Snippets) --> G[ContentParser (uses ContentAnalyzer for structure, API sigs)];
        G -- Parsed Sections/Code --> H{Optional: Vectorizer (all processed content goes here)};
        H -- Vectors --> I[VectorStore (storage_module)];
        G -- Parsed Sections/Code --> J[Formatter];
        H -- Parsed Sections/Code (passthrough) --> J;
        J -- Formatted Output (Markdown) --> K[CacheManager_StoreProcessed];
        K -- Stored --> L[ProcessedDocument_Result];
    end
    
    PyQuery -->|1. Parse Query, Configure & Call DocProcessor with query details & LanguageDocConfig| DocProcessor_Pipeline;
    DocProcessor_Pipeline -->|2. Returns ProcessedDocument (containing Markdown, source info)| SpecificDocTool_Python;

    subgraph SearchService_Global [SearchService (for SearchDocsTool)]
        GlobalQuery[User Query: "search async file read"]
        SS_A[QueryAnalyzer] --> SS_B{IndexType?};
        SS_B -- Vector --> SS_C[VectorStore];
        SS_B -- Keyword/Hybrid --> SS_D[OtherIndex (e.g., Tantivy)];
        SS_C --> SS_E[RankedResults];
        SS_D --> SS_E;
        SS_E --> SS_F[ResultAggregator & Formatter];
        SS_F --> GlobalResult[Formatted Global Search Result];
    end

    style SpecificDocTool_Python fill:#lightgrey,stroke:#333,stroke-width:2px
    style DocProcessor_Pipeline fill:#lightblue,stroke:#333,stroke-width:2px
    style SearchService_Global fill:#lightgreen,stroke:#333,stroke-width:2px
```

### 3. 主要组件说明

#### 3.1 `SpecificDocTool` (e.g., `PythonDocsTool` in `src/tools/python_docs_tool.rs`)
- **功能**: 
    - 实现 `MCPTool` trait (`get_name`, `get_description`, `get_args`, `call`).
    - 接收MCP客户端的文档查询请求 (e.g., `"tool_args": {"query": "math.sin", "language": "python"}`).
    - **配置 `LanguageDocConfig`**: 提供Python文档的URL模板 (e.g., `https://docs.python.org/3/library/{module}.html#{module}.{member}`), CLI命令 (`pydoc {query}`), 内容提取的CSS选择器 (e.g., `main > article`), 等。
    - 调用 `DocProcessor::process_documentation_request()` 并传递配置和查询。
    - 将返回的 `ProcessedDocument` 格式化为MCP工具响应。
- **关键接口**: `async fn call(&self, args: Value, tool_context: &ToolContext) -> Result<Value, MCPError>;`

#### 3.2 `DocProcessor` (`doc_processor.rs`)
- **功能**: 提供一个可配置的、多阶段的文档处理流水线。
- **关键接口**:
    ```rust
    // pub struct DocProcessor {
    //     cli_executor: Arc<CliExecutor>,
    //     scraper: Arc<IntelligentScraper>,
    //     cache_manager: Arc<CacheManager>,
    //     vectorizer: Option<Arc<dyn VectorizerTrait>>,
    //     // ... other shared services
    //     content_analyzer: Arc<ContentAnalyzer>, // Added for deeper analysis
    //     url_discoverer: Arc<URLDiscoveryEngine>, // Added for dynamic URL finding
    // }
    // 
    // impl DocProcessor {
    //     pub async fn process_documentation_request(
    //         &self,
    //         language_name: &str, 
    //         query: &str, 
    //         retrieval_strategy: RetrievalStrategy, 
    //         lang_doc_config: &LanguageDocConfig,
    //         target_content_type: Option<TargetContentType> // NEW: e.g., APIDoc, Examples, Changelog, Readme
    //     ) -> Result<ProcessedDocument, DocProcessingError>;
    // }
    ```
- **`LanguageDocConfig`** (传递给 `DocProcessor`):
    ```rust
    // #[derive(Debug, Clone, Deserialize)]
    // pub struct LanguageDocConfig {
    //     pub language_name: String,
    //     pub base_url_template: Option<String>, // e.g., "https://docs.python.org/3/library/{module}.html"
    //     pub cli_command_template: Option<String>, // e.g., "pydoc {query}"
    //     pub content_selectors_html: Option<Vec<String>>, // CSS selectors for main content in HTML
    //     pub code_block_selectors_html: Option<Vec<String>>,
    //     pub api_signature_patterns_text: Option<Vec<String>>, // Regex for text-based API signatures
    //     pub retrieval_strategy_default: RetrievalStrategy,
    //     pub chunking_strategy: Option<ChunkingConfig>,
    //     pub vectorization_enabled_default: bool,
    //     pub known_source_locations: Option<HashMap<TargetContentType, SourceLocationConfig>>, // NEW
    // }
    ```

#### 3.3 `SearchService` (`search.rs`)
- **功能**: (可选，主要为 `SearchDocsTool` 服务) 实现跨多个已处理/索引文档的全局搜索。
- **关键接口**:
    ```rust
    // pub struct SearchService {
    //     vector_store: Arc<dyn VectorStoreTrait>, // from storage_module
    //     // keyword_index: Option<Arc<TantivyIndex>>, // Example for keyword search
    //     doc_metadata_db: Arc<dyn DocMetadataDB>, // To get original links, titles etc.
    // }
    // 
    // impl SearchService {
    //     pub async fn search_across_sources(
    //         &self,
    //         global_query: &str, 
    //         filters: Option<SearchFilters>
    //     ) -> Result<Vec<GlobalSearchResultItem>, SearchServiceError>;
    // }
    ```
- **`SearchFilters`**: `language: Option<String>`, `source_type: Option<String>`, `max_results: usize`.
- **索引管理**: 依赖 `VectorStore`（如果使用向量搜索）或其它如 `Tantivy` 的全文检索引擎。元数据（如文档来源URL）也需存储和检索。

#### 3.4 `SourceResolver` (内部逻辑 in `DocProcessor`)
- **功能**: 根据 `language_name`, `query`, `RetrievalStrategy`, 和 `LanguageDocConfig` 决定实际的文档获取源和方法。
    - **URL निर्माण**: 使用 `base_url_template` 和查询参数 (e.g., module name, function name from query) 构建目标URL。如果 `target_content_type` (如`Changelog`) 有特定的 `known_source_locations`，则优先使用这些模板。
    - **CLI命令构建**: 使用 `cli_command_template` 和查询参数构建CLI命令。同样，特定 `target_content_type` 的CLI配置会被优先考虑。如果 `target_content_type` 是 `Readme` 或 `Examples` 且配置了 `GitRepo`，则可能构建 `git show` 或 `git archive` 命令。
    - **动态URL发现**: 如果模板不直接适用，或者需要发现如第三方库的README/示例，`SourceResolver` 会利用 `URLDiscoveryEngine` (来自语言特性模块)，结合查询中的实体 (库名、函数名) 和 `target_content_type` (如"示例"、"更新日志") 来查找相关URL。例如，搜索 `crates.io` 或 `GitHub` 查找仓库链接，然后定位到 `README.md`、`CHANGELOG.md` 或 `examples/` 目录。

#### 3.5 `CLIExecutor` (调用 `src/cli/executor.rs`)
- **功能**: 执行特定语言的文档CLI命令 (e.g., `pydoc sys.path`, `godoc net/http FileServer`, `rustup doc std::fs::File`)。也用于当需要从已知URI（如Git仓库）通过CLI获取内容时（例如，`git show origin/main:README.md`，或 `cargo doc --open` 后获取其输出的HTML路径再处理）。
- **调用**: `DocProcessor` 会实例化或获取一个 `CommandExecutor` 实例，并使用 `lang_doc_config.cli_command_template` (或特定 `SourceLocationConfig` 中的 `command_template`) 和用户查询来执行命令。如果CLI文档生成失败或不可用，此信息会被记录，获取流程会尝试其他策略（如HTTP）。

#### 3.6 `IntelligentScraper` (调用 `src/language_features/scraper.rs`)
- **功能**: 从原始HTML (使用 `lang_doc_config.content_selectors_html` 或特定 `SourceLocationConfig` 中的选择器) 或CLI输出中提取主要文档内容，去除导航栏、页眉、页脚、广告等无关信息。
- **实现**: 对于HTML，可以使用 `scraper` crate (Rust) 或类似的库，结合CSS选择器。对于文本，可能使用正则表达式或基于标记的分割。**此组件会紧密依赖 `ContentAnalyzer` (语言特性模块) 来进行更复杂的文本块识别和初步分类，例如识别更新日志条目或示例代码块的边界。**

#### 3.7 `ContentExtractor` (内部逻辑 in `DocProcessor` or uses `language_features`)
- **功能**: 从原始HTML (使用 `lang_doc_config.content_selectors_html`) 或CLI输出中提取主要文档内容，去除导航栏、页眉、页脚、广告等无关信息。
- **实现**: 对于HTML，可以使用 `scraper` crate (Rust) 或类似的库，结合CSS选择器。对于文本，可能使用正则表达式或基于标记的分割。

#### 3.8 `ContentParser` (内部逻辑 in `DocProcessor` or uses `language_features`)
- **功能**: 将提取的纯净内容解析为更结构化的格式。目标是识别语义块，如段落、列表、代码示例、API签名、函数/方法描述等。**对于特定内容类型 (如更新日志、README)，会尝试应用特定的解析逻辑以提取版本号、日期、章节标题等元数据。**
- **实现**: 
    - HTML/Markdown到内部表示。
    - 使用正则表达式或更复杂的解析技术（如基于`tree-sitter`的片段解析，如果 `language_features`提供）来识别API签名 (`api_signature_patterns_text`)。
    - 将代码块 (`code_block_selectors_html`) 明确标记出来。
    - **使用 `ContentAnalyzer` (语言特性模块) 来进行深入分析，包括但不限于：识别API签名、提取命名实体、判断代码块语言。**

#### 3.9 `Vectorizer` (可选, 调用 `src/vectorization/` 或 `vectorization_disabled/`)
- **功能**: 如果启用，将解析后的内容块（特别是文本描述和API用途摘要）转换为向量嵌入，用于语义搜索。**所有成功处理并格式化后的文档内容 (无论来源是CLI、直接HTTP爬取，还是AI辅助生成/提取的内容) 都会被传递到此组件进行向量化。**
- **调用**: `DocProcessor` 将合适的文本块传递给 `Vectorizer` 服务。

#### 3.10 `Formatter` (内部逻辑 in `DocProcessor`)
- **功能**: 将处理和解析后的文档内容块格式化为对AI模型友好的格式，通常是Markdown。
- **实现**: 组合文本、代码块、列表等，生成清晰、简洁的Markdown。保留指向原始文档源的链接。强调API签名和关键信息。
- **CacheFirstCliHttpFallback**: 缓存 -> CLI -> HTTP
- **CacheFirstHttpCliFallback**: 缓存 -> HTTP -> CLI
- **NetworkOnly**: 不使用缓存，直接访问网络 (CLI或HTTP)
- **GitRepoThenHttpFallback**: (新增) 尝试从配置的Git仓库获取文件 (e.g., README.md, examples/)，失败则回退到HTTP通用爬取。

#### 3.11 `CacheManager` (调用 `src/storage/cache.rs`)
- **功能**: 负责缓存文档处理流水线中各个阶段的产出，以避免重复的昂贵操作 (网络请求、CLI执行、解析)。
- **缓存层级**: 
    1.  **原始内容缓存**: 缓存从URL下载的HTML或CLI命令的原始输出。
    2.  **处理后内容缓存**: 缓存已提取、解析并格式化为Markdown的 `ProcessedDocument` 内容。
    3.  **(可选) 向量缓存**: 如果使用了向量化，可以缓存文本块对应的向量。

### 4. 文档来源与获取策略

#### 4.1 权威文档源列表 (示例，在 `LanguageDocConfig` 中配置)
- **Python**: `https://docs.python.org/3/`, `pydoc` CLI.
- **Rust**: `https://doc.rust-lang.org/std/`, `https://docs.rs/`, `rustup doc` CLI.
- **JavaScript (MDN)**: `https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/`.
- **Java (OpenJDK)**: `https://docs.oracle.com/en/java/javase/{version}/docs/api/index.html`, `javadoc` (通常本地生成)。
- **Go**: `https://pkg.go.dev/`, `godoc` CLI.
- **Dart/Flutter**: `https://api.flutter.dev/`, `https://dart.dev/guides`, `dart doc` CLI.

#### 4.2 `RetrievalStrategy` 枚举 (在 `DocProcessor` 中使用)
```rust
// #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
// pub enum RetrievalStrategy {
//     CliOnly,                    // 只使用CLI获取文档 (e.g., pydoc)
//     HttpOnly,                   // 只从HTTP源获取文档 (e.g., docs.python.org)
//     CacheOnly,                  // 只从缓存获取 (如果缓存未命中则失败)
//     CliFirstHttpFallback,       // 优先CLI，失败则尝试HTTP
//     HttpFirstCliFallback,       // 优先HTTP，失败则尝试CLI
//     CacheFirstNetworkFallback,  // 优先缓存，失败则尝试网络 (CLI或HTTP，根据lang_config决定顺序)
//     CacheFirstCliHttpFallback,  // 缓存 -> CLI -> HTTP
//     CacheFirstHttpCliFallback,  // 缓存 -> HTTP -> CLI
//     NetworkOnly,                // 不使用缓存，直接访问网络 (CLI或HTTP)
// }
```

### 5. 内容处理与向量化

#### 5.0 基于MCP和LLM的智能文档获取系统 (重要增强)

当CLI文档生成失败或不可用时，`DocProcessor` 会启动基于MCP客户端和LLM驱动的智能文档获取系统，这是一个高效的多阶段AI处理流程：

##### 5.0.1 LLM驱动的文档处理架构
```rust
pub struct LLMDrivenDocProcessor {
    // MCP客户端管理
    mcp_client_manager: Arc<MCPClientManager>,
    
    // LLM服务集成 - 核心组件
    llm_orchestrator: Arc<LLMOrchestrator>,
    prompt_manager: Arc<PromptManager>,
    
    // 智能处理组件
    intelligent_url_discovery: Arc<IntelligentURLDiscovery>,
    llm_content_extractor: Arc<LLMContentExtractor>,
    ai_content_synthesizer: Arc<AIContentSynthesizer>,
    
    // 缓存和配置
    smart_cache_manager: Arc<SmartCacheManager>,
    processor_config: ProcessorConfig,
}

impl LLMDrivenDocProcessor {
    pub async fn process_with_llm_enhancement(&self, request: DocRequest) -> ProcessedDocument {
        // 第一阶段：LLM驱动的需求分析和策略制定
        let processing_strategy = self.analyze_request_with_llm(&request).await?;
        
        // 第二阶段：智能URL发现和内容源确定
        let content_sources = self.intelligent_url_discovery.discover_sources_with_llm(&processing_strategy).await?;
        
        // 第三阶段：LLM指导的并行内容提取
        let extracted_content = self.llm_content_extractor.extract_with_llm_guidance(content_sources).await?;
        
        // 第四阶段：AI驱动的内容合成和优化
        let synthesized_content = self.ai_content_synthesizer.synthesize_with_llm(extracted_content, &processing_strategy).await?;
        
        ProcessedDocument {
            content: synthesized_content,
            sources: self.extract_source_metadata(&extracted_content),
            processing_metadata: self.create_processing_metadata(&processing_strategy),
            quality_score: self.assess_content_quality(&synthesized_content).await?,
        }
    }
    
    async fn analyze_request_with_llm(&self, request: &DocRequest) -> ProcessingStrategy {
        let prompt = format!(
            "分析以下文档请求并制定最佳处理策略：
            
            编程语言: {}
            目标库/框架: {}
            内容类型: {:?}
            用户查询: {}
            
            请分析：
            1. 用户的真实需求和意图
            2. 最可能包含相关信息的文档源类型
            3. 内容提取的重点和难点
            4. 推荐的处理优先级和策略
            5. 预期的输出格式和结构
            
            返回JSON格式的处理策略。",
            request.language,
            request.target,
            request.content_type,
            request.query.as_deref().unwrap_or("")
        );
        
        let llm_response = self.llm_orchestrator.generate_structured_response(
            &prompt,
            &json_schema_for_processing_strategy()
        ).await?;
        
        ProcessingStrategy::from_llm_response(llm_response)
    }
}
```

##### 5.0.2 LLM增强的内容提取引擎
```rust
impl LLMContentExtractor {
    pub async fn extract_with_llm_guidance(&self, sources: Vec<ContentSource>) -> Vec<ExtractedContent> {
        let mut extraction_tasks = Vec::new();
        
        for source in sources {
            let task = self.extract_single_source_with_llm(source);
            extraction_tasks.push(task);
        }
        
        let results = futures::try_join_all(extraction_tasks).await?;
        results.into_iter().flatten().collect()
    }
    
    async fn extract_single_source_with_llm(&self, source: ContentSource) -> Option<ExtractedContent> {
        match source.source_type {
            SourceType::WebPage => self.extract_webpage_with_llm(source).await,
            SourceType::GitRepository => self.extract_git_content_with_llm(source).await,
            SourceType::APIEndpoint => self.extract_api_docs_with_llm(source).await,
        }
    }
    
    async fn extract_webpage_with_llm(&self, source: ContentSource) -> Option<ExtractedContent> {
        // 第一步：LLM分析页面类型和结构
        let page_analysis = self.analyze_page_structure_with_llm(&source).await?;
        
        // 第二步：基于分析结果调整Playwright提取策略
        let extraction_strategy = self.adapt_extraction_strategy(&page_analysis);
        
        // 第三步：使用Playwright执行提取
        let raw_content = self.extract_with_playwright(&source, &extraction_strategy).await?;
        
        // 第四步：LLM后处理和内容清洗
        let cleaned_content = self.clean_content_with_llm(&raw_content, &page_analysis).await?;
        
        // 第五步：LLM结构化内容组织
        let structured_content = self.structure_content_with_llm(&cleaned_content, &source.content_type).await?;
        
        Some(ExtractedContent {
            source: source.url,
            content: structured_content,
            extraction_metadata: self.create_extraction_metadata(&page_analysis),
            confidence_score: self.calculate_confidence(&structured_content),
        })
    }
    
    async fn analyze_page_structure_with_llm(&self, source: &ContentSource) -> Option<PageStructureAnalysis> {
        // 获取页面预览信息
        let page_preview = self.get_page_preview(&source.url).await?;
        
        let prompt = format!(
            "分析网页结构以优化内容提取：
            
            URL: {}
            内容类型: {:?}
            页面标题: {}
            主要标签: {:?}
            页面大纲: {}
            
            请分析：
            1. 主要内容区域的位置和特征
            2. 需要避免的无关内容（导航、广告、页脚等）
            3. 动态加载内容的处理方法
            4. 代码示例和文档的具体位置
            5. 最优的CSS选择器策略
            
            返回JSON格式的结构分析。",
            source.url,
            source.content_type,
            page_preview.title,
            page_preview.main_tags,
            page_preview.outline
        );
        
        let llm_response = self.llm_orchestrator.generate_structured_response(
            &prompt,
            &json_schema_for_page_analysis()
        ).await.ok()?;
        
        PageStructureAnalysis::from_llm_response(llm_response)
    }
    
    async fn clean_content_with_llm(&self, raw_content: &RawContent, analysis: &PageStructureAnalysis) -> Option<CleanedContent> {
        let prompt = format!(
            "清洗和优化提取的网页内容：
            
            原始内容: {}
            HTML片段: {}
            页面分析: {:?}
            
            请执行：
            1. 移除无关的导航、广告、页脚内容
            2. 保留并突出关键的技术内容
            3. 修复格式问题和断行
            4. 提取和标注代码示例
            5. 保持重要的链接和引用
            
            返回JSON格式的清洗后内容。",
            raw_content.text.chars().take(3000).collect::<String>(),
            raw_content.html.as_deref().unwrap_or("").chars().take(1000).collect::<String>(),
            analysis
        );
        
        let llm_response = self.llm_orchestrator.generate_structured_response(
            &prompt,
            &json_schema_for_cleaned_content()
        ).await.ok()?;
        
        CleanedContent::from_llm_response(llm_response)
    }
    
    async fn structure_content_with_llm(&self, cleaned: &CleanedContent, content_type: &ContentType) -> Option<StructuredContent> {
        let prompt = match content_type {
            ContentType::Examples => format!(
                "将以下内容结构化为代码示例文档：
                
                {}
                
                请：
                1. 识别并分类所有代码示例
                2. 为每个示例添加适当的说明
                3. 按复杂度和学习顺序排列
                4. 确保代码的完整性和可执行性
                5. 添加必要的上下文和注释
                
                返回Markdown格式的结构化示例文档。",
                cleaned.main_content
            ),
            ContentType::ApiDocs => format!(
                "将以下内容结构化为API文档：
                
                {}
                
                请：
                1. 提取API函数/方法的签名
                2. 整理参数说明和返回值
                3. 保留使用示例
                4. 添加清晰的章节结构
                5. 突出重要的注意事项
                
                返回Markdown格式的API文档。",
                cleaned.main_content
            ),
            ContentType::Changelog => format!(
                "将以下内容整理为更新日志：
                
                {}
                
                请：
                1. 提取版本号和发布日期
                2. 分类变更类型（新功能、修复、破坏性变更）
                3. 按时间倒序排列
                4. 突出重要变更
                5. 保持简洁明了的格式
                
                返回Markdown格式的更新日志。",
                cleaned.main_content
            ),
            _ => format!(
                "将以下技术内容结构化：
                
                {}
                
                请创建清晰的文档结构，包含适当的标题、段落和格式。",
                cleaned.main_content
            ),
        };
        
        let structured_content = self.llm_orchestrator.generate_completion(&prompt).await
            .unwrap_or_else(|_| self.fallback_structure_content(cleaned));
        
        Some(StructuredContent {
            content: structured_content,
            content_type: content_type.clone(),
            sections: self.extract_sections(&structured_content),
            metadata: self.create_content_metadata(cleaned),
        })
    }
}
```

##### 5.0.3 AI驱动的内容合成引擎
```rust
impl AIContentSynthesizer {
    pub async fn synthesize_with_llm(&self, content_list: Vec<ExtractedContent>, strategy: &ProcessingStrategy) -> String {
        // 第一步：LLM分析内容关系和质量
        let content_analysis = self.analyze_content_with_llm(&content_list, strategy).await?;
        
        // 第二步：LLM驱动的内容分组和优先级排序
        let organized_content = self.organize_content_with_llm(&content_list, &content_analysis).await?;
        
        // 第三步：LLM合成最终文档
        let synthesized_document = self.synthesize_final_document_with_llm(&organized_content, strategy).await?;
        
        synthesized_document
    }
    
    async fn analyze_content_with_llm(&self, content_list: &[ExtractedContent], strategy: &ProcessingStrategy) -> ContentAnalysis {
        let content_summaries: Vec<String> = content_list.iter()
            .map(|content| format!(
                "来源: {}\n类型: {:?}\n质量评分: {:.2}\n内容摘要: {}",
                content.source,
                content.content.content_type,
                content.confidence_score,
                content.content.content.chars().take(500).collect::<String>()
            ))
            .collect();
        
        let prompt = format!(
            "分析多源技术文档内容的质量和关系：
            
            用户需求: {:?}
            内容来源:
            {}
            
            请分析：
            1. 每个来源的权威性和可靠性
            2. 内容之间的重叠、冲突和互补关系
            3. 信息的时效性和准确性
            4. 内容的完整性和深度
            5. 最佳的内容整合策略
            
            返回JSON格式的内容分析结果。",
            strategy.user_intent,
            content_summaries.join("\n\n---\n\n")
        );
        
        let llm_response = self.llm_orchestrator.generate_structured_response(
            &prompt,
            &json_schema_for_content_analysis()
        ).await?;
        
        ContentAnalysis::from_llm_response(llm_response)
    }
    
    async fn synthesize_final_document_with_llm(&self, organized: &OrganizedContent, strategy: &ProcessingStrategy) -> String {
        let prompt = format!(
            "基于分析结果合成高质量的技术文档：
            
            用户需求: {:?}
            目标格式: {:?}
            组织后的内容: {}
            
            请生成：
            1. 结构清晰的完整文档
            2. 适当的标题层级和组织
            3. 准确的技术信息和示例
            4. 必要的链接和引用
            5. 简洁明了的表达方式
            
            返回高质量的Markdown格式文档。",
            strategy.user_intent,
            strategy.output_format,
            organized.sections.iter()
                .map(|s| format!("## {}\n{}", s.title, s.content.chars().take(1000).collect::<String>()))
                .collect::<Vec<_>>()
                .join("\n\n")
        );
        
        self.llm_orchestrator.generate_completion(&prompt).await
            .unwrap_or_else(|_| self.fallback_synthesize(organized))
    }
}
```

##### 5.0.4 智能Git仓库内容处理
```rust
impl GitContentProcessor {
    pub async fn extract_git_content_with_llm(&self, repo_info: &GitRepoInfo, strategy: &ProcessingStrategy) -> Vec<ExtractedContent> {
        // 第一步：LLM分析仓库结构和目标文件
        let repo_analysis = self.analyze_repository_with_llm(repo_info, strategy).await?;
        
        // 第二步：基于分析结果获取相关文件
        let target_files = self.determine_target_files(&repo_analysis);
        let file_contents = self.fetch_files_with_git_mcp(&repo_info, target_files).await?;
        
        // 第三步：LLM处理每个文件内容
        let mut processed_contents = Vec::new();
        for file_content in file_contents {
            if let Some(processed) = self.process_file_with_llm(&file_content, &repo_analysis).await {
                processed_contents.push(processed);
            }
        }
        
        processed_contents
    }
    
    async fn analyze_repository_with_llm(&self, repo_info: &GitRepoInfo, strategy: &ProcessingStrategy) -> RepositoryAnalysis {
        // 先获取仓库的基本信息
        let repo_structure = self.get_repository_structure(repo_info).await?;
        
        let prompt = format!(
            "分析Git仓库结构以找到相关文档：
            
            仓库URL: {}
            用户需求: {:?}
            目录结构: {:?}
            README预览: {}
            
            请确定：
            1. 最可能包含相关文档的目录和文件
            2. 文档的组织方式和结构
            3. 示例代码的位置
            4. 更新日志和发布说明的位置
            5. 配置文件和安装指南的位置
            
            返回JSON格式的仓库分析。",
            repo_info.url,
            strategy.user_intent,
            repo_structure.directories,
            repo_structure.readme_preview
        );
        
        let llm_response = self.llm_orchestrator.generate_structured_response(
            &prompt,
            &json_schema_for_repository_analysis()
        ).await?;
        
        RepositoryAnalysis::from_llm_response(llm_response)
    }
    
    async fn process_file_with_llm(&self, file_content: &FileContent, repo_analysis: &RepositoryAnalysis) -> Option<ExtractedContent> {
        let prompt = format!(
            "处理Git仓库中的技术文档文件：
            
            文件路径: {}
            文件类型: {}
            文件内容: {}
            仓库上下文: {:?}
            
            请：
            1. 提取关键的技术信息
            2. 识别代码示例和配置
            3. 整理安装和使用说明
            4. 保留重要的链接和引用
            5. 修正格式和结构问题
            
            返回处理后的Markdown格式文档。",
            file_content.path,
            file_content.file_type,
            file_content.content.chars().take(3000).collect::<String>(),
            repo_analysis.context
        );
        
        let processed_content = self.llm_orchestrator.generate_completion(&prompt).await
            .unwrap_or_else(|_| file_content.content.clone());
        
        Some(ExtractedContent {
            source: format!("{}/{}", repo_analysis.repo_url, file_content.path),
            content: StructuredContent {
                content: processed_content,
                content_type: self.infer_content_type(&file_content.path),
                sections: vec![],
                metadata: ContentMetadata::default(),
            },
            extraction_metadata: ExtractionMetadata::default(),
            confidence_score: 0.9,
        })
    }
}
```

##### 5.0.5 实际应用场景

**场景1：Python FastAPI库的全面文档获取**
```rust
// 用户查询：获取FastAPI的完整使用指南
let request = DocRequest {
    language: "python".to_string(),
    target: "fastapi".to_string(),
    content_type: ContentType::Comprehensive,
    query: Some("complete guide with examples".to_string()),
};

// LLM驱动的处理流程：
// 1. LLM分析：识别为Web框架，需要安装、基础使用、高级特性等
// 2. 智能发现：官方文档、GitHub仓库、社区教程
// 3. 并行提取：
//    - 官方文档的结构化内容
//    - GitHub示例代码
//    - 社区最佳实践
// 4. LLM合成：生成从入门到高级的完整指南
```

**场景2：新兴Rust库的技术文档生成**
```rust
// 用户查询：了解某个新Rust库的使用方法
let request = DocRequest {
    language: "rust".to_string(),
    target: "some-new-crate".to_string(),
    content_type: ContentType::Examples,
    query: Some("usage examples and best practices".to_string()),
};

// LLM增强的处理：
// 1. LLM识别：Rust生态系统特点，docs.rs、crates.io等
// 2. 策略制定：重点关注Cargo.toml、examples/目录、README
// 3. 内容提取：API文档、代码示例、配置指南
// 4. 智能整理：生成可执行的示例和最佳实践
```

#### 5.1 HTML 清理与内容提取
- 使用 `scraper` (Rust) crate，通过 `LanguageDocConfig` 中提供的CSS选择器 (`content_selectors_html`) 来定位主要内容区域。
- 移除脚本、样式、导航、页眉/页脚等非主体内容。

#### 5.2 Markdown 解析与生成
- 如果源是Markdown或需要转换为Markdown，使用 `pulldown-cmark` (Rust) 进行解析和渲染。
- 代码块识别: 从HTML (`code_block_selectors_html`) 或Markdown中正确提取代码块，并标记语言类型。

#### 5.3 API签名提取
- 针对文本输出 (CLI) 或解析后的HTML内容，使用 `LanguageDocConfig` 中的 `api_signature_patterns_text` (正则表达式列表) 来识别和提取函数、方法、类的签名。
- 对于某些语言，可能利用 `language_features` 模块的更高级AST级分析能力（如果可用且适用）。

#### 5.4 分块策略 (`ChunkingConfig` in `LanguageDocConfig`)
- **目的**: 将长文档分割成大小合适、语义连贯的块，以便于向量化、索引和精确检索。
- **策略**: 
    - 按HTML的语义标签 (e.g., `<section>`, `<article>`, `<h1>`, `<h2>`)。
    - 按段落 (`<p>`) 或固定数量的句子。
    - 针对API文档，可以按每个函数/方法/类的完整描述作为一个块。
    - **配置**: `max_chunk_size_tokens`, `overlap_tokens`.

#### 5.5 向量化 (可选)
- 如果启用 (e.g., `lang_doc_config.vectorization_enabled_default` 为 `true`)，则将分块后的文本块通过 `Vectorizer` 组件转换为向量嵌入，并与块元数据 (ID, 源文档链接) 一同存入 `VectorStore`。
- **所有经过`DocProcessor`处理并由`Formatter`最终输出的Markdown内容，都会被送入`Vectorizer`进行向量化和存储，确保无论是CLI获取、网络爬取还是AI辅助生成的内容，都能为LLM提供语义检索能力。**

### 6. 缓存机制 (`CacheManager` 细节)
- **缓存键生成**: 组合 `language_name`, `query_or_document_id`, `retrieval_mode_if_raw`, `content_version_or_hash`。
- **缓存内容与结构**:
    - **Raw Cache**: `key -> RawContent { content: Vec<u8>, source_url: String, fetched_at: DateTime }`
    - **Processed Cache**: `key -> ProcessedDocument { formatted_markdown: String, metadata: DocMetadata, ... }`
- **缓存策略**: 
    - **TTL**: 为不同类型的缓存内容设置不同的过期时间 (e.g., 原始HTML缓存时间较长，处理后的文档根据版本更新频率决定)。
    - **LRU**: 当缓存达到容量上限时，使用LRU策略进行淘汰。
    - **主动失效**: 当检测到上游文档源更新时（如果可能），可以主动使相关缓存失效。
- **存储后端**: 使用 `src/storage/cache.rs` 提供的 `MokaCache` (内存) 和可选的磁盘缓存 (文件系统)。

### 7. 错误处理 (`DocProcessingError` enum)
```rust
// #[derive(Debug, thiserror::Error)]
// pub enum DocProcessingError {
//     #[error("Configuration error for language '{0}': {1}")]
//     ConfigError(String, String),
//     #[error("Source not found for query '{query}' using strategy {strategy:?}. Details: {details}")]
//     SourceNotFound { query: String, strategy: RetrievalStrategy, details: String },
//     #[error("CLI command execution failed for '{command}': {source}")]
//     CliCommandFailed { command: String, #[source] source: std::io::Error }, // Assuming CliError from cli_integration
//     #[error("HTTP download failed for URL '{url}': {source}")]
//     HttpDownloadFailed { url: String, #[source] source: reqwest::Error }, // Or other HTTP client error
//     #[error("Content extraction failed from source '{source_ref}': {details}")]
//     ExtractionFailed { source_ref: String, details: String },
//     #[error("Content parsing failed for source '{source_ref}': {details}")]
//     ParsingFailed { source_ref: String, details: String },
//     #[error("Vectorization failed: {0}")]
//     VectorizationFailed(String),
//     #[error("Formatting failed: {0}")]
//     FormattingFailed(String),
//     #[error("Cache operation failed: {0}")]
//     CacheError(String),
//     #[error("Unsupported query format or type for tool '{tool_name}': {query}")]
//     UnsupportedQueryFormat { tool_name: String, query: String },
//     #[error("Language '{0}' not supported or configured for documentation processing.")]
//     LanguageNotSupported(String),
// }
```

### 8. 模块接口与配置

#### 8.1 `ProcessedDocument` 结构体 (返回给 `SpecificDocTool`)
```rust
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ProcessedDocument {
//     pub id: String, // Unique ID for this processed document/query result
//     pub language: String,
//     pub query: String, // The original query that led to this document
//     pub title: Option<String>, // Extracted title if available
//     pub formatted_content_markdown: String, // The main result, formatted as Markdown
//     pub source_url: Option<String>, // Link to the original web page, if applicable
//     pub source_cli_command: Option<String>, // CLI command used, if applicable
//     pub retrieval_strategy_used: RetrievalStrategy,
//     pub processed_at: DateTime<Utc>,
//     pub from_cache: bool, // Was this result primarily from cache?
//     pub raw_content_hash: Option<String>, // Hash of the raw content it was derived from
//     pub relevance_score: Option<f32>, // If part of a search result list
//     // pub chunks: Option<Vec<DocumentChunk>>, // If chunked for vectorization/display
// }
```

#### 8.2 主要配置文件 (`LanguageDocConfig` per language)
- 这些配置可以存储在 `configs/language_docs/python.toml`, `configs/language_docs/rust.toml` 等文件中，由 `SpecificDocTool` 加载并传递给 `DocProcessor`。
- 示例 (`python.toml`):
    ```toml
    language_name = "python"
    base_url_template = "https://docs.python.org/3/library/{module}.html#{module}.{member}"
    cli_command_template = "pydoc3 {query}"
    retrieval_strategy_default = "CacheFirstCliHttpFallback"
    vectorization_enabled_default = true

    # NEW: Known locations for specific content types
    [known_source_locations.Changelog]
    Http = { url_template = "https://github.com/psf/requests/blob/main/HISTORY.md", content_selectors = ["article"] }
    
    [known_source_locations.Readme]
    GitRepo = { repo_uri_template = "https://github.com/psf/requests.git", file_path_template = "README.md" }

    [known_source_locations.Examples]
    GitRepo = { repo_uri_template = "https://github.com/psf/requests.git", file_path_template = "docs/source/user/quickstart.rst" }
    # Or, an HTTP location if examples are on a webpage
    # Http = { url_template = "https://requests.readthedocs.io/en/latest/user/quickstart/", content_selectors = ["#quickstart"] }

    [content_selectors_html]
    main_content = ["div[role='main']", "article"]
    code_blocks = ["pre", ".code-block-caption + .highlight"]

    [api_signature_patterns_text] # For pydoc output
    function_def = "^(\w+\s+)?\w+\s*\(\s*[^)]*\s*\)(?:\s*->\s*\w+)?\s*:"

    [chunking_strategy]
    type = "semantic_section" # or "paragraph", "fixed_token"
    max_chunk_size_tokens = 512
    # section_delimiters_html = ["h1", "h2", "h3"] # For semantic_section if HTML
    ```

### 9. 测试策略

- **单元测试**:
    - `SpecificDocTool` (e.g., `PythonDocsTool`): Mock `DocProcessor`. Test query parsing, `LanguageDocConfig` creation, and MCP response formatting.
    - `DocProcessor` (core流水线逻辑): 
        - Mock `CLIExecutor`, `IntelligentScraper`, `CacheManager`, `Vectorizer`.
        - Test different `RetrievalStrategy` logic flow.
        - Test interaction with `ContentExtractor`, `ContentParser`, `Formatter` using predefined raw inputs (HTML snippets, CLI text output) and expected processed outputs.
    - `SourceResolver`: Test URL and CLI command template rendering.
    - `ContentExtractor`, `ContentParser`, `Formatter`: Test with various sample inputs and verify output structure/content.
    - `CacheManager` interaction: Test cache hit/miss logic, key generation.
- **集成测试**:
    - `DocProcessor` with real (but minimal and controlled) CLI output or local HTML files. For example, save a few small, representative HTML doc pages locally and test the full processing from file source.
    - Test `SpecificDocTool` end-to-end by mocking MCP client requests and verifying the final formatted Markdown output, possibly using a live (but controlled) CLI tool if environment permits (e.g., `pydoc sys`).
    - For `SearchService`, test with a small, pre-built vector index and metadata store.
- **端到端测试 (有限范围)**:
    - For a very small subset of queries and a specific language, test the entire flow from an MCP `tools/call` (mocked) to the final output, possibly involving a live (but local/sandboxed) web server for HTML fetching or a known CLI tool.

## 总结

文档处理模块是 `grape-mcp-devtools` 提供核心文档查询能力的关键。通过其灵活的、多阶段的处理流水线，以及对多种文档源和检索策略的支持，它能够为AI编程助手提供准确、相关且格式友好的编程文档。良好的缓存机制和模块化设计（特别是 `DocProcessor` 作为通用框架，由 `SpecificDocTool` 进行语言特化配置）确保了其高效性和可扩展性，能够适应未来更多编程语言和文档格式的需求。可选的向量化和全局搜索功能进一步增强了其智能文档处理的能力。 