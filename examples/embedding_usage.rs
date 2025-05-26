use anyhow::Result;
use grape_mcp_devtools::tools::docs::{
    embedding_client::{EmbeddingConfig, OpenAIEmbeddingClient, HybridVectorizer},
    vectorizer_factory::{VectorizerFactory, VectorizerType, SimpleLocalVectorizer},
    doc_traits::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🚀 NVIDIA 向量化功能使用示例 (使用 async-openai BYOT)");
    println!("💡 这就像 Python 的 extra_body 参数一样简单！");

    // 1. 从环境变量创建 NVIDIA API 配置
    let config = EmbeddingConfig::from_env()?;
    println!("✅ 配置加载成功:");
    println!("   - API URL: {}", config.api_base_url);
    println!("   - 模型: {}", config.model_name);
    println!("   - API Key: {}...{}", 
        &config.api_key[..8], 
        &config.api_key[config.api_key.len()-8..]
    );

    // 展示 NVIDIA 特有的参数配置
    println!("\n🔧 NVIDIA 特有参数配置 (相当于 Python 的 extra_body):");
    if let Some(encoding_format) = &config.encoding_format {
        println!("   - encoding_format: {}", encoding_format);
    }
    if let Some(input_type) = &config.input_type {
        println!("   - input_type: {}", input_type);
    }
    if let Some(truncate) = &config.truncate {
        println!("   - truncate: {}", truncate);
    }

    // 2. 创建向量化器
    println!("\n📡 正在创建向量化器...");
    let vectorizer = VectorizerFactory::create_vectorizer(
        VectorizerType::Hybrid,
        Some(config.clone())
    )?;
    println!("✅ 向量化器创建成功");

    // 3. 验证 API 连接
    println!("\n🔍 验证 NVIDIA API 连接...");
    let client = OpenAIEmbeddingClient::new(config)?;
    if client.validate_connection().await? {
        println!("✅ NVIDIA API 连接验证成功");
        println!("💡 这证明了 async-openai 的 BYOT 功能可以完美支持 NVIDIA API！");
    } else {
        println!("❌ NVIDIA API 连接验证失败，将使用本地备用");
    }

    // 4. 创建测试文档片段
    let fragment = create_sample_go_document();
    println!("\n📄 创建测试文档: {}", fragment.title);

    // 5. 向量化文档
    println!("\n⚡ 正在向量化文档...");
    println!("💡 使用 async-openai 的 create_byot() 方法，支持自定义参数");
    
    match vectorizer.vectorize(&fragment).await {
        Ok(vector) => {
            println!("✅ 向量化成功!");
            println!("   - 文档ID: {}", vector.metadata.doc_id);
            println!("   - 向量维度: {}", vector.dimension);
            println!("   - 语言: {}", vector.metadata.language);
            println!("   - 关键词: {:?}", vector.metadata.keywords);
            println!("   - 向量前5个值: {:?}", &vector.data[..5.min(vector.data.len())]);
            
            // 展示向量的数值范围
            let min_val = vector.data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_val = vector.data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            println!("   - 向量值范围: [{:.6}, {:.6}]", min_val, max_val);
        }
        Err(e) => {
            println!("❌ 向量化失败: {}", e);
            println!("💡 请检查:");
            println!("   - API 密钥是否正确");
            println!("   - 网络连接是否正常");
            println!("   - NVIDIA API 配额是否充足");
        }
    }

    // 6. 演示相似度计算
    demonstrate_similarity_calculation().await?;

    // 7. 演示批量向量化
    demonstrate_batch_vectorization(&client).await?;

    // 8. 演示 BYOT 功能的优势
    demonstrate_byot_advantages().await?;

    println!("\n🎉 示例完成!");
    println!("💡 总结:");
    println!("   - 使用 async-openai 的 BYOT 功能，就像 Python 的 extra_body 一样简单");
    println!("   - 完美支持 NVIDIA API 的特殊参数 (input_type, truncate 等)");
    println!("   - 代码更简洁，更易维护");
    println!("   - 修改 .env 文件中的配置来测试不同的API提供商");
    
    Ok(())
}

async fn demonstrate_byot_advantages() -> Result<()> {
    println!("\n🆚 BYOT 功能的优势演示");
    
    println!("📊 对比:");
    println!("  Python OpenAI 库:");
    println!("    response = client.embeddings.create(");
    println!("        input=[\"text\"],");
    println!("        model=\"nvidia/nv-embedcode-7b-v1\",");
    println!("        extra_body={{\"input_type\": \"query\", \"truncate\": \"NONE\"}}");
    println!("    )");
    
    println!("\n  Rust async-openai 库 (我们的实现):");
    println!("    let request = json!({{");
    println!("        \"model\": \"nvidia/nv-embedcode-7b-v1\",");
    println!("        \"input\": [\"text\"],");
    println!("        \"input_type\": \"query\",");
    println!("        \"truncate\": \"NONE\"");
    println!("    }});");
    println!("    let response = client.embeddings().create_byot(request).await?;");
    
    println!("\n✨ 优势:");
    println!("   - 类型安全：编译时检查");
    println!("   - 性能更好：零拷贝序列化");
    println!("   - 代码更清晰：没有嵌套的 extra_body");
    println!("   - 完全兼容：支持所有 OpenAI 兼容的 API");

    Ok(())
}

fn create_sample_go_document() -> DocumentFragment {
    DocumentFragment {
        id: "sample-go-http-handler".to_string(),
        title: "HTTP Request Handler".to_string(),
        kind: DocElementKind::Function,
        full_name: Some("main.handleHTTPRequest".to_string()),
        description: "处理HTTP请求的主要函数，支持GET、POST和PUT方法，包含错误处理和日志记录".to_string(),
        source_type: DocSourceType::ApiDoc,
        code_context: Some(CodeContext {
            code: r#"
func handleHTTPRequest(w http.ResponseWriter, r *http.Request) {
    // 记录请求
    log.Printf("收到请求: %s %s", r.Method, r.URL.Path)
    
    // 设置CORS头
    w.Header().Set("Access-Control-Allow-Origin", "*")
    w.Header().Set("Content-Type", "application/json")
    
    switch r.Method {
    case "GET":
        handleGetRequest(w, r)
    case "POST":
        handlePostRequest(w, r)
    case "PUT":
        handlePutRequest(w, r)
    case "OPTIONS":
        w.WriteHeader(http.StatusOK)
    default:
        http.Error(w, "方法不被支持", http.StatusMethodNotAllowed)
    }
}
"#.to_string(),
            location: CodeLocation {
                file: "handlers/http_handler.go".to_string(),
                start_line: 15,
                end_line: 35,
                context_before: vec![
                    "package handlers".to_string(), 
                    "import (".to_string(),
                    "    \"net/http\"".to_string(),
                    "    \"log\"".to_string(),
                    ")".to_string()
                ],
                context_after: vec![
                    "func handleGetRequest(w http.ResponseWriter, r *http.Request) {".to_string(),
                    "    // GET 请求处理逻辑".to_string()
                ],
            },
            comments: vec![],
            language: "go".to_string(),
        }),
        examples: vec![
            Example {
                title: Some("基本HTTP服务器设置".to_string()),
                description: Some("如何使用这个处理器创建HTTP服务器".to_string()),
                code: r#"
func main() {
    http.HandleFunc("/api/v1/", handleHTTPRequest)
    log.Println("服务器启动在端口 8080")
    log.Fatal(http.ListenAndServe(":8080", nil))
}
"#.to_string(),
                language: "go".to_string(),
                output: Some("服务器启动在端口 8080".to_string()),
                explanation: Some("将handleHTTPRequest函数注册为/api/v1/路径的处理器".to_string()),
            },
            Example {
                title: Some("中间件集成".to_string()),
                description: Some("与中间件一起使用".to_string()),
                code: r#"
router := mux.NewRouter()
router.HandleFunc("/api/v1/", authMiddleware(handleHTTPRequest)).Methods("GET", "POST", "PUT")
"#.to_string(),
                language: "go".to_string(),
                output: None,
                explanation: Some("在Gorilla Mux路由器中使用，并添加认证中间件".to_string()),
            },
        ],
        api_info: Some(ApiInfo {
            parameters: vec![
                Parameter {
                    name: "w".to_string(),
                    type_info: "http.ResponseWriter".to_string(),
                    description: "HTTP响应写入器".to_string(),
                    optional: false,
                    default_value: None,
                },
                Parameter {
                    name: "r".to_string(),
                    type_info: "*http.Request".to_string(),
                    description: "HTTP请求对象指针".to_string(),
                    optional: false,
                    default_value: None,
                },
            ],
            returns: Some(Returns {
                type_info: "void".to_string(),
                description: "无返回值，通过ResponseWriter写入响应".to_string(),
            }),
            throws: vec![
                ErrorInfo {
                    error_type: "http.StatusMethodNotAllowed".to_string(),
                    description: "当HTTP方法不被支持时返回405错误".to_string(),
                    conditions: vec!["请求方法不是GET、POST、PUT或OPTIONS".to_string()],
                }
            ],
            type_parameters: vec![],
            visibility: Visibility::Public,
        }),
        references: vec![
            DocReference {
                kind: ReferenceKind::Api,
                target: "http.ResponseWriter".to_string(),
                description: Some("Go标准库HTTP响应接口".to_string()),
            },
            DocReference {
                kind: ReferenceKind::Api,
                target: "http.Request".to_string(),
                description: Some("Go标准库HTTP请求结构".to_string()),
            },
        ],
        metadata: DocMetadata {
            package_name: "handlers".to_string(),
            version: Some("2.1.0".to_string()),
            language: "go".to_string(),
            source_url: Some("https://github.com/example/http-server".to_string()),
            deprecated: false,
            since_version: Some("1.0.0".to_string()),
            visibility: Visibility::Public,
        },
        changelog_info: None,
    }
}

async fn demonstrate_similarity_calculation() -> Result<()> {
    println!("\n🔍 相似度计算演示");

    // 创建两个相似的文档
    let doc1 = create_sample_go_document();
    let mut doc2 = doc1.clone();
    doc2.id = "similar-function".to_string();
    doc2.title = "HTTP Request Handler".to_string();
    doc2.description = "另一个处理HTTP请求的函数".to_string();

    // 创建一个不同的文档
    let mut doc3 = doc1.clone();
    doc3.id = "different-function".to_string();
    doc3.title = "Database Connection".to_string();
    doc3.description = "连接到数据库的函数".to_string();
    doc3.metadata.language = "python".to_string();

    // 使用本地向量化器进行演示（不需要API密钥）
    let local_vectorizer = SimpleLocalVectorizer;

    let vector1 = local_vectorizer.vectorize(&doc1).await?;
    let vector2 = local_vectorizer.vectorize(&doc2).await?;
    let vector3 = local_vectorizer.vectorize(&doc3).await?;

    let similarity_12 = local_vectorizer.calculate_similarity(&vector1, &vector2);
    let similarity_13 = local_vectorizer.calculate_similarity(&vector1, &vector3);

    println!("📊 相似度结果:");
    println!("   - 文档1 vs 文档2 (相似): {:.3}", similarity_12);
    println!("   - 文档1 vs 文档3 (不同): {:.3}", similarity_13);

    Ok(())
}

async fn demonstrate_batch_vectorization(client: &OpenAIEmbeddingClient) -> Result<()> {
    println!("\n📦 批量向量化演示");

    // 创建多个不同的文档
    let docs = vec![
        create_sample_go_document(),
        create_sample_python_document(),
        create_sample_rust_document(),
    ];

    println!("⚡ 正在批量向量化 {} 个文档...", docs.len());
    match client.vectorize_batch(&docs).await {
        Ok(vectors) => {
            println!("✅ 批量向量化成功!");
            println!("📊 批量向量化结果:");
            for (i, vector) in vectors.iter().enumerate() {
                println!("  文档 {}:", i + 1);
                println!("     - ID: {}", vector.metadata.doc_id);
                println!("     - 维度: {}", vector.dimension);
                println!("     - 语言: {}", vector.metadata.language);
                println!("     - 关键词: {:?}", &vector.metadata.keywords[..3.min(vector.metadata.keywords.len())]);
                
                // 展示向量的数值范围
                let min_val = vector.data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_val = vector.data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                println!("     - 向量范围: [{:.6}, {:.6}]", min_val, max_val);
            }
            
            // 计算文档间的相似度
            if vectors.len() >= 2 {
                let similarity = client.calculate_similarity(&vectors[0], &vectors[1]);
                println!("  📊 文档1和文档2的相似度: {:.3}", similarity);
            }
        }
        Err(e) => {
            println!("❌ 批量向量化失败: {}", e);
        }
    }

    Ok(())
}

fn create_sample_python_document() -> DocumentFragment {
    DocumentFragment {
        id: "sample-python-function".to_string(),
        title: "FastAPI Route Handler".to_string(),
        kind: DocElementKind::Function,
        full_name: Some("app.handle_api_request".to_string()),
        description: "FastAPI 路由处理器，支持异步处理和数据验证".to_string(),
        source_type: DocSourceType::ApiDoc,
        code_context: Some(CodeContext {
            code: r#"
@app.post("/api/v1/data")
async def handle_api_request(request: DataRequest) -> DataResponse:
    """处理API数据请求"""
    try:
        # 验证请求数据
        validated_data = await validate_request(request)
        
        # 处理业务逻辑
        result = await process_data(validated_data)
        
        # 返回响应
        return DataResponse(
            success=True,
            data=result,
            message="请求处理成功"
        )
    except ValidationError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        logger.error(f"处理请求失败: {e}")
        raise HTTPException(status_code=500, detail="内部服务器错误")
"#.to_string(),
            location: CodeLocation {
                file: "app/handlers.py".to_string(),
                start_line: 25,
                end_line: 45,
                context_before: vec![
                    "from fastapi import FastAPI, HTTPException".to_string(),
                    "from pydantic import BaseModel".to_string(),
                    "import logging".to_string(),
                ],
                context_after: vec![
                    "async def validate_request(request: DataRequest):".to_string(),
                ],
            },
            comments: vec![],
            language: "python".to_string(),
        }),
        examples: vec![
            Example {
                title: Some("基本使用".to_string()),
                description: Some("发送POST请求到API端点".to_string()),
                code: r#"
import requests

response = requests.post(
    "http://localhost:8000/api/v1/data",
    json={"name": "测试", "value": 123}
)
print(response.json())
"#.to_string(),
                language: "python".to_string(),
                output: Some(r#"{"success": true, "data": {...}, "message": "请求处理成功"}"#.to_string()),
                explanation: Some("使用requests库发送POST请求".to_string()),
            },
        ],
        api_info: Some(ApiInfo {
            parameters: vec![
                Parameter {
                    name: "request".to_string(),
                    type_info: "DataRequest".to_string(),
                    description: "请求数据模型".to_string(),
                    optional: false,
                    default_value: None,
                },
            ],
            returns: Some(Returns {
                type_info: "DataResponse".to_string(),
                description: "响应数据模型".to_string(),
            }),
            throws: vec![ErrorInfo {
                error_type: "HTTPException".to_string(),
                description: "HTTP异常".to_string(),
                conditions: vec!["请求数据无效".to_string()],
            }],
            type_parameters: vec![],
            visibility: Visibility::Public,
        }),
        references: vec![
            DocReference {
                kind: ReferenceKind::Api,
                target: "FastAPI".to_string(),
                description: Some("FastAPI 框架".to_string()),
            },
        ],
        metadata: DocMetadata {
            package_name: "app".to_string(),
            version: Some("1.0.0".to_string()),
            language: "python".to_string(),
            source_url: Some("https://github.com/example/fastapi-app".to_string()),
            deprecated: false,
            since_version: Some("1.0.0".to_string()),
            visibility: Visibility::Public,
        },
        changelog_info: None,
    }
}

fn create_sample_rust_document() -> DocumentFragment {
    DocumentFragment {
        id: "sample-rust-function".to_string(),
        title: "Async HTTP Client".to_string(),
        kind: DocElementKind::Function,
        full_name: Some("client::send_request".to_string()),
        description: "异步HTTP客户端函数，支持重试机制和错误处理".to_string(),
        source_type: DocSourceType::ApiDoc,
        code_context: Some(CodeContext {
            code: r#"
pub async fn send_request<T: Serialize + DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    payload: &T,
) -> Result<T, ClientError> {
    let mut retries = 0;
    const MAX_RETRIES: u32 = 3;
    
    loop {
        match client
            .post(url)
            .json(payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    return response.json::<T>().await.map_err(ClientError::Parse);
                } else {
                    return Err(ClientError::Http(response.status()));
                }
            }
            Err(e) => {
                retries += 1;
                if retries >= MAX_RETRIES {
                    return Err(ClientError::Network(e));
                }
                tokio::time::sleep(Duration::from_millis(100 * retries as u64)).await;
            }
        }
    }
}
"#.to_string(),
            location: CodeLocation {
                file: "src/client.rs".to_string(),
                start_line: 15,
                end_line: 40,
                context_before: vec![
                    "use reqwest;".to_string(),
                    "use serde::{Serialize, de::DeserializeOwned};".to_string(),
                    "use std::time::Duration;".to_string(),
                ],
                context_after: vec![
                    "#[derive(Debug)]".to_string(),
                    "pub enum ClientError {".to_string(),
                ],
            },
            comments: vec![],
            language: "rust".to_string(),
        }),
        examples: vec![
            Example {
                title: Some("基本使用".to_string()),
                description: Some("发送JSON请求".to_string()),
                code: r#"
#[derive(Serialize, Deserialize)]
struct ApiRequest {
    name: String,
    value: i32,
}

let client = reqwest::Client::new();
let request = ApiRequest {
    name: "test".to_string(),
    value: 42,
};

let response = send_request(&client, "https://api.example.com/data", &request).await?;
"#.to_string(),
                language: "rust".to_string(),
                output: None,
                explanation: Some("创建HTTP客户端并发送JSON请求".to_string()),
            },
        ],
        api_info: Some(ApiInfo {
            parameters: vec![
                Parameter {
                    name: "client".to_string(),
                    type_info: "&reqwest::Client".to_string(),
                    description: "HTTP客户端引用".to_string(),
                    optional: false,
                    default_value: None,
                },
                Parameter {
                    name: "url".to_string(),
                    type_info: "&str".to_string(),
                    description: "请求URL".to_string(),
                    optional: false,
                    default_value: None,
                },
                Parameter {
                    name: "payload".to_string(),
                    type_info: "&T".to_string(),
                    description: "请求负载数据".to_string(),
                    optional: false,
                    default_value: None,
                },
            ],
            returns: Some(Returns {
                type_info: "Result<T, ClientError>".to_string(),
                description: "请求结果或错误".to_string(),
            }),
            throws: vec![ErrorInfo {
                error_type: "ClientError".to_string(),
                description: "客户端错误".to_string(),
                conditions: vec!["网络错误".to_string(), "解析错误".to_string()],
            }],
            type_parameters: vec![TypeParameter {
                name: "T".to_string(),
                bounds: vec!["Serialize + DeserializeOwned".to_string()],
                default_type: None,
                variance: None,
            }],
            visibility: Visibility::Public,
        }),
        references: vec![
            DocReference {
                kind: ReferenceKind::Api,
                target: "reqwest::Client".to_string(),
                description: Some("reqwest HTTP客户端".to_string()),
            },
        ],
        metadata: DocMetadata {
            package_name: "client".to_string(),
            version: Some("0.1.0".to_string()),
            language: "rust".to_string(),
            source_url: Some("https://github.com/example/rust-client".to_string()),
            deprecated: false,
            since_version: Some("0.1.0".to_string()),
            visibility: Visibility::Public,
        },
        changelog_info: None,
    }
} 