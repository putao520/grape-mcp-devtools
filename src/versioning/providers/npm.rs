use crate::versioning::models::package::Package;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use chrono::Utc;
use async_trait::async_trait;

pub struct NpmProvider {
    client: Client,
}

#[async_trait]
impl crate::versioning::traits::PackageProvider for NpmProvider {
    async fn get_package_info(&self, package_name: &str) -> Result<Package> {
        let url = format!("https://registry.npmjs.org/{}", package_name);
        let response: Value = self.client.get(&url).send().await?.json().await?;
        
        let latest_version = response["dist-tags"]["latest"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
            
        Ok(Package {
            name: package_name.to_string(),
            version: latest_version,
            description: response["description"].as_str().unwrap_or("").to_string(),
            license: response["license"].as_str().unwrap_or("").to_string(),
            homepage: response["homepage"].as_str().map(|s| s.to_string()),
            repository: response["repository"]["url"].as_str().map(|s| s.to_string()),
            author: response["author"]["name"].as_str().map(|s| s.to_string()),
            release_date: Utc::now(),
            download_count: None,
            available_versions: Vec::new(),
        })
    }
    
    async fn get_dependencies(&self, _package: &Package) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }
} 