use reqwest;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 调试API响应");
    
    let client = reqwest::Client::builder()
        .user_agent("grape-mcp-devtools/2.0.0 (https://github.com/grape-mcp-devtools)")
        .build()?;
    
    // 测试crates.io API
    println!("\n📦 测试crates.io API:");
    let crates_url = "https://crates.io/api/v1/crates/serde";
    match client.get(crates_url).send().await {
        Ok(response) => {
            println!("状态码: {}", response.status());
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(data) => {
                        println!("响应数据结构:");
                        if let Some(crate_obj) = data.get("crate") {
                            println!("  crate字段存在");
                            if let Some(newest_version) = crate_obj.get("newest_version") {
                                println!("  newest_version: {}", newest_version);
                            }
                            if let Some(max_version) = crate_obj.get("max_version") {
                                println!("  max_version: {}", max_version);
                            }
                            if let Some(max_stable_version) = crate_obj.get("max_stable_version") {
                                println!("  max_stable_version: {}", max_stable_version);
                            }
                        }
                    }
                    Err(e) => println!("JSON解析失败: {}", e),
                }
            } else {
                println!("请求失败: {}", response.status());
            }
        }
        Err(e) => println!("请求错误: {}", e),
    }
    
    // 测试Maven Central API
    println!("\n📦 测试Maven Central API:");
    let maven_url = "https://search.maven.org/solrsearch/select?q=a:\"spring-core\"&core=gav&rows=5&wt=json";
    match client.get(maven_url).send().await {
        Ok(response) => {
            println!("状态码: {}", response.status());
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(data) => {
                        println!("响应数据结构:");
                        if let Some(response_obj) = data.get("response") {
                            println!("  response字段存在");
                            if let Some(docs) = response_obj.get("docs") {
                                println!("  docs字段存在，数量: {}", docs.as_array().map(|a| a.len()).unwrap_or(0));
                            }
                        }
                    }
                    Err(e) => println!("JSON解析失败: {}", e),
                }
            } else {
                println!("请求失败: {}", response.status());
            }
        }
        Err(e) => println!("请求错误: {}", e),
    }
    
    Ok(())
} 