use crate::versioning::models::package::Package;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use chrono::Utc;
use async_trait::async_trait;

pub struct PubDevProvider {
    client: Client,
}

impl PubDevProvider {
    // 移除未使用的new方法
}

#[async_trait]
impl crate::versioning::traits::PackageProvider for PubDevProvider {
    async fn get_package_info(&self, package_name: &str) -> Result<Package> {
        // pub.dev API
        let url = format!("https://pub.dev/api/packages/{}", package_name);
        let response: Value = self.client.get(&url).send().await?.json().await?;
        
        let latest = &response["latest"];
        
        Ok(Package {
            name: package_name.to_string(),
            version: latest["version"].as_str().unwrap_or("unknown").to_string(),
            description: latest["pubspec"]["description"].as_str().unwrap_or("").to_string(),
            license: "".to_string(),
            homepage: latest["pubspec"]["homepage"].as_str().map(|s| s.to_string()),
            repository: latest["pubspec"]["repository"].as_str().map(|s| s.to_string()),
            author: None,
            release_date: Utc::now(),
            download_count: None,
            available_versions: Vec::new(),
        })
    }
    
    async fn get_dependencies(&self, _package: &Package) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }
} 