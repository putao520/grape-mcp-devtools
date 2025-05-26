use crate::versioning::models::package::Package;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use chrono::Utc;
use async_trait::async_trait;

pub struct GoProvider {
    client: Client,
}

impl GoProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl crate::versioning::traits::PackageProvider for GoProvider {
    async fn get_package_info(&self, package_name: &str) -> Result<Package> {
        // Go proxy API
        let url = format!("https://proxy.golang.org/{}/latest", package_name);
        let response: Value = self.client.get(&url).send().await?.json().await?;
        
        Ok(Package {
            name: package_name.to_string(),
            version: response["Version"].as_str().unwrap_or("unknown").to_string(),
            description: "".to_string(),
            license: "".to_string(),
            homepage: None,
            repository: None,
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