use crate::versioning::models::package::Package;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use chrono::Utc;
use async_trait::async_trait;

pub struct MavenProvider {
    client: Client,
}

#[async_trait]
impl crate::versioning::traits::PackageProvider for MavenProvider {
    async fn get_package_info(&self, package_name: &str) -> Result<Package> {
        // Maven Central API URL
        let url = format!("https://search.maven.org/solrsearch/select?q=g:%22{}%22&rows=1&wt=json", package_name);
        let response: Value = self.client.get(&url).send().await?.json().await?;
        
        let docs = response["response"]["docs"].as_array();
        let empty_vec = vec![];
        let docs = docs.unwrap_or(&empty_vec);
        
        if let Some(doc) = docs.first() {
            Ok(Package {
                name: package_name.to_string(),
                version: doc["latestVersion"].as_str().unwrap_or("unknown").to_string(),
                description: doc["p"].as_str().unwrap_or("").to_string(),
                license: "".to_string(),
                homepage: None,
                repository: None,
                author: None,
                release_date: Utc::now(),
                download_count: None,
                available_versions: Vec::new(),
            })
        } else {
            Err(anyhow::anyhow!("Package not found: {}", package_name))
        }
    }
    
    async fn get_dependencies(&self, _package: &Package) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }
} 