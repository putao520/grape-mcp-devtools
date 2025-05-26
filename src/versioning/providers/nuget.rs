use crate::versioning::models::package::Package;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use chrono::Utc;
use async_trait::async_trait;

pub struct NugetProvider {
    client: Client,
}

impl NugetProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl crate::versioning::traits::PackageProvider for NugetProvider {
    async fn get_package_info(&self, package_name: &str) -> Result<Package> {
        // NuGet API
        let url = format!("https://api.nuget.org/v3-flatcontainer/{}/index.json", package_name.to_lowercase());
        let response: Value = self.client.get(&url).send().await?.json().await?;
        
        let versions = response["versions"].as_array();
        let empty_vec = vec![];
        let versions = versions.unwrap_or(&empty_vec);
        let latest_version = versions.last()
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        Ok(Package {
            name: package_name.to_string(),
            version: latest_version,
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