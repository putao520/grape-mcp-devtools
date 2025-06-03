use async_trait::async_trait;
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;

use crate::versioning::{
    base::VersionChecker,
    models::{Package, VersionInfo, Registry},
};

pub struct CratesIoChecker {
    client: Client,
}

impl CratesIoChecker {
    async fn fetch_crate_data(&self, name: &str) -> Result<Value> {
        let url = format!("{}/crates/{}", Registry::Cargo.base_url(), name);
        let response = self.client.get(&url).send().await?;
        let data = response.json().await?;
        Ok(data)
    }
}

#[async_trait]
impl VersionChecker for CratesIoChecker {
    fn registry(&self) -> Registry {
        Registry::Cargo
    }

    async fn check_version(&self, package: &Package) -> Result<VersionInfo> {
        let data = self.fetch_crate_data(&package.name).await?;
        let crate_data = data["crate"].as_object()
            .ok_or_else(|| anyhow::anyhow!("无效的crates.io响应"))?;

        Ok(VersionInfo {
            package: package.clone(),
            latest_stable: crate_data["max_stable_version"]
                .as_str()
                .unwrap_or("0.0.0")
                .to_string(),
            latest_preview: crate_data["max_version"]
                .as_str()
                .filter(|v| v.contains("-"))
                .map(String::from),
            release_date: DateTime::parse_from_rfc3339(
                crate_data["updated_at"]
                    .as_str()
                    .unwrap_or_default()
            )?.with_timezone(&Utc),
            eol_date: None,
            available_versions: self.list_versions(package).await?,
            dependencies: self.get_dependencies(package).await?,
            downloads: crate_data["downloads"]
                .as_u64(),
        })
    }

    async fn list_versions(&self, package: &Package) -> Result<Vec<String>> {
        let url = format!(
            "{}/crates/{}/versions",
            Registry::Cargo.base_url(),
            package.name
        );
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;
        
        Ok(data["versions"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v["num"].as_str().map(String::from))
            .collect())
    }

    async fn get_dependencies(&self, package: &Package) -> Result<Option<Value>> {
        let data = self.fetch_crate_data(&package.name).await?;
        let latest_version = data["versions"]
            .as_array()
            .and_then(|versions| versions.first())
            .and_then(|version| version["dependencies"].as_array())
            .map(|deps| serde_json::json!(deps));
        
        Ok(latest_version)
    }
}
