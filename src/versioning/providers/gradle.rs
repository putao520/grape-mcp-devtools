use crate::versioning::models::package::Package;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use chrono::Utc;
use async_trait::async_trait;

pub struct GradleProvider {
    client: Client,
}

#[async_trait]
impl crate::versioning::traits::PackageProvider for GradleProvider {
    async fn get_package_info(&self, package_name: &str) -> Result<Package> {
        // Gradle plugins portal API
        let url = format!("https://plugins.gradle.org/api/gradle/{}", package_name);
        let response: Value = self.client.get(&url).send().await?.json().await?;
        
        Ok(Package {
            name: package_name.to_string(),
            version: response["version"].as_str().unwrap_or("unknown").to_string(),
            description: response["description"].as_str().unwrap_or("").to_string(),
            license: "".to_string(),
            homepage: response["website"].as_str().map(|s| s.to_string()),
            repository: response["vcs"].as_str().map(|s| s.to_string()),
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