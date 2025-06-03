# 存储与数据层模块设计文档

## 模块概览

存储与数据层模块 (Storage & Data Layer Module) 是 `grape-mcp-devtools` 的基础支撑模块，为整个应用程序提供统一的、可配置的数据持久化、多级缓存管理以及高效的数据检索服务（包括可选的向量存储与搜索）。其核心目标是向上层模块（如文档处理、语言特性、工具配置等）透明化具体的存储技术实现细节，提供稳定、可靠、高性能的数据访问接口。

### 模块基本信息
- **模块路径**: `src/storage/` (主要包括 `cache.rs`, `vector_store.rs`, `file_store.rs`, `kv_store.rs`, `serialization.rs`), 以及 `src/vectorization_disabled/` (当向量化特性关闭时的备用实现)。
- **主要作用**: 数据持久化、内存与磁盘缓存、向量索引与搜索、键值存储、文件存储、序列化/反序列化。
- **核心特性**: 多级缓存 (内存L1、磁盘L2 - 可选)、可插拔的向量存储后端、原子文件操作、嵌入式键值数据库、统一序列化接口。
- **服务对象**: `DocProcessor`, `Vectorizer`, `AICollector`, `DynamicToolRegistry` (用于缓存检测结果), 以及任何需要持久化或缓存数据的模块。

## 架构设计

### 1. 模块在系统中的位置

存储与数据层是项目的最底层基础服务之一，被多个上层模块直接或间接调用。

```mermaid
graph TD
    A[DocProcessor] --> StorageLayer;
    B[LanguageFeatures Module (e.g., AICollector for LLM response caching)] --> StorageLayer;
    C[Vectorizer Module] --> StorageLayer;
    D[DynamicToolRegistry (for env detection cache)] --> StorageLayer;
    E[Application Configuration] --> StorageLayer;

    subgraph StorageLayer [Storage & Data Layer (`src/storage/`)]
        direction LR
        CM[CacheManager]
        VS[VectorStore / VectorStoreDisabled]
        FS[FileStorage]
        KVS[KeyValueStore (sled)]
        SM[SerializationManager]
    end

    CM --> KVS; # Disk cache can use KVStore
    CM --> FS;  # Disk cache can use FileStorage
```

### 2. 内部组件架构图

```mermaid
digraph StorageInternal {
    rankdir=LR;
    node [shape=box, style=rounded];

    subgraph ApplicationModules [label="调用方模块"]
        DocProc [label="DocProcessor"];
        VectorizerMod [label="Vectorizer"];
        ConfigMod [label="AppConfigLoader"];
    end

    subgraph StorageAndDataLayer [label="Storage & Data Layer (`src/storage/`)"]
        CacheManager [label="CacheManager\n(cache.rs)\n- In-Memory (Moka)\n- Disk (sled/file)"];
        VectorStore [label="VectorStore\n(vector_store.rs)\n- instant-distance (in-memory)\n- Persistence to file"];
        VectorStoreDisabled [label="VectorStoreDisabled\n(vectorization_disabled/store.rs)\n- No-op or simple file mock"];
        FileStorage [label="FileStorage\n(file_store.rs)\n- Atomic file R/W"];
        KeyValueStore [label="KeyValueStore\n(kv_store.rs)\n- Embedded KV (sled)"];
        SerializationManager [label="SerializationManager\n(serialization.rs)\n- JSON, Bincode helpers"];
        StorageConfig [label="StorageConfig\n(Loaded from app config)"];
    end

    subgraph ExternalCrates [label="External Crates"]
        Moka [label="moka"];
        Sled [label="sled"];
        InstantDistance [label="instant-distance"];
        Serde [label="serde, serde_json, bincode"];
        TokioFS [label="tokio::fs"];
    end

    DocProc --> CacheManager;
    VectorizerMod --> VectorStore;
    VectorizerMod --> VectorStoreDisabled; # Conditional compilation
    ConfigMod --> FileStorage; # For loading/saving some config files
    ConfigMod --> StorageConfig;
    CacheManager --> Moka;
    CacheManager --> KeyValueStore; # For disk cache tier
    CacheManager --> SerializationManager;
    VectorStore --> InstantDistance;
    VectorStore --> FileStorage; # For index persistence
    VectorStore --> SerializationManager;
    KeyValueStore --> Sled;
    FileStorage --> TokioFS;
    SerializationManager --> Serde;
    StorageConfig --> CacheManager;
    StorageConfig --> VectorStore;
    StorageConfig --> KeyValueStore;
    StorageConfig --> FileStorage;
}
```

### 3. 主要组件说明

#### 3.1 `CacheManager` (`cache.rs`)
- **功能**: 提供通用的、多级（内存L1、磁盘L2 - 可选）缓存接口，支持TTL和大小限制。
- **关键接口**:
    ```rust
    // pub struct CacheManager {
    //     memory_cache: Arc<MokaCache<String, Vec<u8>>>, // Stores serialized data
    //     disk_cache: Option<Arc<DiskCacheProvider>>, // e.g., using KeyValueStore or FileStorage
    //     serialization_manager: Arc<SerializationManager>,
    // }
    // 
    // impl CacheManager {
    //     pub async fn get<T: for<'de> Deserialize<'de> + Send + Sync>(&self, key: &str) -> Result<Option<T>, CacheError>;
    //     pub async fn put<T: Serialize + Send + Sync>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), CacheError>;
    //     pub async fn invalidate(&self, key: &str) -> Result<(), CacheError>;
    //     pub async fn clear_memory_cache(&self);
    //     pub async fn clear_disk_cache(&self); // If disk_cache is enabled
    // }
    ```
- **实现**: 
    - **L1内存缓存**: 使用 `moka::future::Cache`，配置最大容量和TTL。
    - **L2磁盘缓存 (可选)**: 可以基于 `KeyValueStore` (sled) 或直接使用 `FileStorage` 实现。将序列化后的对象（使用 `SerializationManager`）存入磁盘。磁盘缓存应有其独立的容量管理和清理策略。

#### 3.2 `VectorStore` (`vector_store.rs`) / `VectorStoreDisabled` (`vectorization_disabled/store.rs`)
- **功能**: 存储向量嵌入及其关联元数据，并提供高效的近似最近邻 (ANN) 搜索。`VectorStoreDisabled` 提供一个空操作或简单的基于文件/元数据的备用实现。
- **关键接口 (`VectorStoreTrait` defined in `src/storage/mod.rs` or `vector_store.rs`):**
    ```rust
    // #[async_trait]
    // pub trait VectorStoreTrait: Send + Sync {
    //     async fn add_vectors(&self, collection_name: &str, vectors_data: Vec<VectorData>) -> Result<(), VectorStoreError>;
    //     async fn search_vectors(&self, collection_name: &str, query_vector: &[f32], top_k: usize, filter: Option<&Value>) -> Result<Vec<SearchResult>, VectorStoreError>;
    //     async fn delete_collection(&self, collection_name: &str) -> Result<(), VectorStoreError>;
    //     async fn persist_collection(&self, collection_name: &str) -> Result<(), VectorStoreError>; // For in-memory stores that need saving
    //     async fn load_collection(&self, collection_name: &str) -> Result<(), VectorStoreError>;    // For in-memory stores that need loading
    // }
    // 
    // pub struct VectorData { pub id: String, pub vector: Vec<f32>, pub metadata: Value } // metadata is serde_json::Value
    // pub struct SearchResult { pub id: String, pub score: f32, pub metadata: Value }
    ```
- **实现 (`vector_store.rs` - when `vectorization` feature is enabled)**:
    - 使用 `instant-distance` 库构建内存向量索引。
    - `collections: Arc<Mutex<HashMap<String, (InstantDistanceIndex, HashMap<String, Value>)>>>`.
    - 索引的持久化: `persist_collection` 将 `InstantDistanceIndex::dump_index()` 的输出和元数据（`HashMap<String, Value>`）序列化 (Bincode) 后通过 `FileStorage` 写入文件。`load_collection` 反向操作。
- **实现 (`vectorization_disabled/store.rs` - when `vectorization` feature is disabled)**:
    - `add_vectors`: 可能将元数据写入JSON文件或简单地忽略。
    - `search_vectors`: 返回空结果或错误，或者如果元数据被存储，可以实现一个基于元数据过滤的简单（非向量）搜索。
    - 持久化/加载可能为空操作。

#### 3.3 `FileStorage` (`file_store.rs`)
- **功能**: 提供统一的、安全的、异步的原子文件读写操作，用于持久化任意数据 blob、配置文件、大型下载内容或序列化对象。
- **关键接口**:
    ```rust
    // pub struct FileStorage { base_data_path: PathBuf }
    // 
    // impl FileStorage {
    //     pub fn new(base_data_path: PathBuf) -> Self;
    //     pub async fn read_file_atomic(&self, relative_path: &Path) -> Result<Vec<u8>, std::io::Error>;
    //     pub async fn write_file_atomic(&self, relative_path: &Path, data: &[u8]) -> Result<(), std::io::Error>;
    //     pub async fn ensure_dir_exists(&self, relative_path: &Path) -> Result<(), std::io::Error>;
    //     pub async fn delete_file_or_dir(&self, relative_path: &Path) -> Result<(), std::io::Error>;
    //     pub fn get_absolute_path(&self, relative_path: &Path) -> PathBuf;
    // }
    ```
- **实现**: 使用 `tokio::fs` 进行异步文件操作。原子写入可以通过先写入临时文件再重命名来实现。

#### 3.4 `KeyValueStore` (`kv_store.rs`)
- **功能**: (如果需要超出缓存的简单持久化键值对) 提供一个嵌入式的、持久化的键值存储服务。
- **关键接口**:
    ```rust
    // pub struct KeyValueStore { db: sled::Db }
    // 
    // impl KeyValueStore {
    //     pub fn new(db_path: &Path) -> Result<Self, sled::Error>;
    //     pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, sled::Error>;
    //     pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), sled::Error>;
    //     pub fn delete(&self, key: &[u8]) -> Result<Option<Vec<u8>>, sled::Error>;
    //     pub fn contains_key(&self, key: &[u8]) -> Result<bool, sled::Error>;
    //     // pub fn iter_prefix(&self, prefix: &[u8]) -> impl Iterator<Item = Result<(Vec<u8>, Vec<u8>), sled::Error>>;
    // }
    ```
- **实现**: 使用 `sled` 嵌入式数据库。

#### 3.5 `SerializationManager` (`serialization.rs`)
- **功能**: 封装 `serde` 的常用序列化/反序列化操作，提供便捷方法，并处理相关错误。
- **关键接口**:
    ```rust
    // pub struct SerializationManager;
    // 
    // impl SerializationManager {
    //     pub fn to_json_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error>;
    //     pub fn from_json_string<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, serde_json::Error>;
    //     pub fn to_bincode<T: Serialize>(value: &T) -> Result<Vec<u8>, bincode::Error>;
    //     pub fn from_bincode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, bincode::Error>;
    //     // pub fn to_toml_string<T: Serialize>(value: &T) -> Result<String, toml::ser::Error>;
    //     // pub fn from_toml_string<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, toml::de::Error>;
    // }
    ```

### 4. 数据流与交互 (示例)
- **文档缓存 (`DocProcessor` -> `CacheManager`)**: 
    1. `DocProcessor` 请求获取一篇文档，先查询 `CacheManager`。
    2. `CacheManager` 首先检查L1内存缓存，若未命中，检查L2磁盘缓存（如果启用）。
    3. 若命中，反序列化数据并返回。若未命中，`DocProcessor` 从源获取数据，然后通过 `CacheManager::put` 存入缓存（同时进入内存和磁盘）。
- **向量索引 (`Vectorizer` -> `VectorStore`)**: 
    1. `Vectorizer` 处理文本块生成向量和元数据。
    2. 调用 `VectorStore::add_vectors` 将 `Vec<VectorData>` 添加到指定集合。
    3. `VectorStore` (instant-distance impl) 更新内存索引，并将元数据与内部ID关联。
    4. 定期或在关闭时，`VectorStore` 调用 `persist_collection` 将索引和元数据序列化并通过 `FileStorage` 保存到磁盘。
- **配置加载 (App Startup -> `FileStorage` / `KeyValueStore`)**: 
    1. 应用启动时，配置模块可能使用 `FileStorage` 读取TOML配置文件。
    2. 某些动态或用户特定的配置可能存储在 `KeyValueStore` (sled) 中。

### 5. 数据模型 (示例, 位于各组件内部或共享 `models.rs`)
```rust
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct CachedObject<T> {
//     pub data: T,
//     pub expires_at: Option<DateTime<Utc>>,
//     pub created_at: DateTime<Utc>,
// }

// // For VectorStore
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct VectorIndexDiskData {
//     pub index_bytes: Vec<u8>, // Serialized instant_distance::SearchIndex
//     pub metadata_map: HashMap<String, Value>, // id -> metadata (serde_json::Value)
//     pub point_ids: Vec<String>, // Order matches points in index_bytes
// }
```

### 6. 错误处理 (`StorageError` enum in `src/storage/mod.rs`)
```rust
// #[derive(Debug, thiserror::Error)]
// pub enum StorageError {
//     #[error("Cache operation failed: {0}")]
//     CacheError(#[from] CacheErrorSubtype),
//     #[error("I/O error: {0}")]
//     IoError(#[from] std::io::Error),
//     #[error("Serialization error: {0}")]
//     SerializationError(String), // Wraps serde_json, bincode, etc. errors
//     #[error("KeyValueStore (sled) error: {0}")]
//     SledError(#[from] sled::Error),
//     #[error("VectorStore operation failed: {0}")]
//     VectorStoreError(String), // Wraps specific vector store errors
//     #[error("Configuration error for storage: {0}")]
//     ConfigError(String),
//     #[error("Path error: {0}")]
//     PathError(String),
//     #[error("Data not found for key/path: {0}")]
//     NotFound(String),
// }
// 
// #[derive(Debug, thiserror::Error)]
// pub enum CacheErrorSubtype {
//     #[error("Memory cache error: {0}")]
//     MemoryCache(String),
//     #[error("Disk cache error: {0}")]
//     DiskCache(String),
//     #[error("Value not found in cache for key: {0}")]
//     NotFound(String),
//     #[error("Serialization error during cache op: {0}")]
//     Serialization(String),
// }
```

### 7. 配置管理 (`StorageConfig`)
- **来源**: 从主应用的配置文件 (e.g., `application_config.toml`) 中加载 `[storage]` 部分。
- **内容示例**:
    ```toml
    # [storage] in application_config.toml
    base_data_directory = ".mcp_data" # Relative to workspace or user home

    [storage.cache.memory]
    max_capacity_items = 10000
    default_ttl_seconds = 3600 # 1 hour

    [storage.cache.disk] # Optional second-level cache
    enabled = true
    path_suffix = "disk_cache"
    max_size_gb = 2
    # eviction_policy = "LRU"

    [storage.vector_store]
    persistence_path_suffix = "vector_indices"
    # For external vector DBs in future: 
    # type = "qdrant"
    # url = "http://localhost:6334"

    [storage.key_value_store.sled]
    db_path_suffix = "app_kv.sled"
    ```
- `StorageConfig` 结构体会被传递给各个存储组件的构造函数。

### 8. 测试策略

- **`CacheManager`**: 
    - 测试 `get`/`put`/`invalidate` 操作。
    - 测试TTL逻辑：数据在过期后应无法获取或返回 `None`。
    - 测试内存缓存的大小限制和淘汰策略 (Moka的内置行为)。
    - 如果启用了磁盘缓存，测试数据能否正确写入磁盘并在内存缓存失效后从磁盘加载。
    - 测试不同类型数据的序列化/反序列化。
- **`VectorStore` (instant-distance impl)**:
    - 使用少量样本向量测试 `add_vectors` 和 `search_vectors` (检查结果ID和分数)。
    - 测试元数据存储和过滤 (如果 `instant-distance` 的包装器支持)。
    - 测试 `persist_collection` 和 `load_collection`：确保索引可以正确保存到文件并重新加载后仍能正常工作。
    - 测试集合的创建和删除。
- **`VectorStoreDisabled`**: 确保其接口方法按预期工作 (e.g., 返回空、错误，或执行简单的文件操作)。
- **`FileStorage`**: 
    - 使用 `tempfile` crate 创建临时目录进行测试。
    - 测试 `read_file_atomic` 和 `write_file_atomic` 的正确性和原子性 (e.g., 写入一半时出错，不应破坏原文件或产生不完整文件)。
    - 测试 `ensure_dir_exists` 和 `delete_file_or_dir`。
- **`KeyValueStore` (sled impl)**:
    - 测试基本的 `get`/`put`/`delete`/`contains_key` 操作。
    - 测试 `sled` 数据库文件的创建和加载。
- **`SerializationManager`**: 测试对各种数据结构与JSON和Bincode之间的相互转换。
- **错误处理**: 确保所有组件在遇到 I/O 错误、序列化错误、数据库错误等时，都能正确转换为并返回 `StorageError`。
- **并发测试**: 对于 `CacheManager` 和 `VectorStore` (特别是其内部使用 `Mutex` 的部分)，进行并发访问测试以检查是否存在竞态条件或死锁。

## 总结

存储与数据层模块是 `grape-mcp-devtools` 稳定运行和高效执行的基石。通过提供统一的缓存、向量存储、文件和键值存储接口，它简化了上层模块的数据管理逻辑，并允许灵活配置和未来扩展存储技术栈（如集成外部向量数据库）。对原子操作、错误处理和测试的重视确保了数据的完整性和系统的可靠性。当向量化特性被禁用时，模块也能优雅地降级，提供基础的文件存储和缓存功能。 