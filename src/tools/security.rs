use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use anyhow::Result;
use crate::errors::MCPError;
use super::base::{MCPTool, ToolAnnotations, Schema, SchemaObject, SchemaString, SchemaBoolean};
use serde::{Deserialize, Serialize};
use reqwest;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SecurityVulnerability {
    id: String,
    summary: String,
    details: String,
    severity: String, // "LOW", "MEDIUM", "HIGH", "CRITICAL"
    cvss_score: Option<f64>,
    cve_id: Option<String>,
    published: DateTime<Utc>,
    modified: DateTime<Utc>,
    affected_versions: Vec<String>,
    fixed_versions: Vec<String>,
    references: Vec<String>,
    ecosystem: String, // "npm", "cargo", "pip", etc.
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVQuery {
    package: OSVPackage,
    version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVPackage {
    name: String,
    ecosystem: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVResponse {
    vulns: Vec<OSVVulnerability>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVVulnerability {
    id: String,
    summary: Option<String>,
    details: Option<String>,
    severity: Option<Vec<OSVSeverity>>,
    published: Option<String>,
    modified: Option<String>,
    affected: Option<Vec<OSVAffected>>,
    references: Option<Vec<OSVReference>>,
    aliases: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVSeverity {
    #[serde(rename = "type")]
    severity_type: String,
    score: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVAffected {
    package: OSVPackage,
    ranges: Option<Vec<OSVRange>>,
    versions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVRange {
    #[serde(rename = "type")]
    range_type: String,
    events: Vec<OSVEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVEvent {
    introduced: Option<String>,
    fixed: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OSVReference {
    #[serde(rename = "type")]
    ref_type: String,
    url: String,
}

pub struct SecurityCheckTool {
    annotations: ToolAnnotations,
    cache: Arc<RwLock<HashMap<String, (Vec<SecurityVulnerability>, DateTime<Utc>)>>>,
    client: reqwest::Client,
}

impl SecurityCheckTool {
    pub fn new() -> Self {
        Self {
            annotations: ToolAnnotations {
                category: "安全检查".to_string(),
                tags: vec!["安全".to_string(), "漏洞".to_string(), "CVE".to_string()],
                version: "1.0".to_string(),
            },
            cache: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::new(),
        }
    }

    // 查询OSV数据库
    async fn query_osv_database(&self, ecosystem: &str, package: &str, version: Option<&str>) -> Result<Vec<SecurityVulnerability>> {
        let osv_url = "https://api.osv.dev/v1/query";
        
        let query = OSVQuery {
            package: OSVPackage {
                name: package.to_string(),
                ecosystem: self.map_ecosystem_to_osv(ecosystem),
            },
            version: version.map(String::from),
        };

        let response = self.client
            .post(osv_url)
            .json(&query)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Vec::new()); // 没有找到漏洞或查询失败
        }

        let osv_response: OSVResponse = response.json().await?;
        let mut vulnerabilities = Vec::new();

        for vuln in osv_response.vulns {
            let security_vuln = self.convert_osv_to_security_vuln(vuln, ecosystem)?;
            vulnerabilities.push(security_vuln);
        }

        Ok(vulnerabilities)
    }

    // 查询GitHub Advisory Database
    async fn query_github_advisory(&self, ecosystem: &str, package: &str) -> Result<Vec<SecurityVulnerability>> {
        let github_url = format!(
            "https://api.github.com/advisories?ecosystem={}&affects={}",
            self.map_ecosystem_to_github(ecosystem),
            package
        );

        let response = self.client
            .get(&github_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "grape-mcp-devtools/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let advisories: Vec<Value> = response.json().await?;
        let mut vulnerabilities = Vec::new();

        for advisory in advisories {
            if let Ok(security_vuln) = self.convert_github_advisory_to_security_vuln(advisory, ecosystem) {
                vulnerabilities.push(security_vuln);
            }
        }

        Ok(vulnerabilities)
    }

    // 使用RustSec数据库查询Rust包漏洞
    async fn query_rustsec_database(&self, package: &str) -> Result<Vec<SecurityVulnerability>> {
        // 这里可以集成rustsec库，目前使用HTTP API
        let rustsec_url = format!("https://rustsec.org/advisories/{}.json", package);
        
        let response = self.client
            .get(&rustsec_url)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let advisory: Value = resp.json().await?;
                let vuln = self.convert_rustsec_to_security_vuln(advisory, package)?;
                Ok(vec![vuln])
            },
            _ => Ok(Vec::new()),
        }
    }

    // 综合查询所有数据源
    async fn check_package_security(&self, ecosystem: &str, package: &str, version: Option<&str>) -> Result<Vec<SecurityVulnerability>> {
        let mut all_vulnerabilities = Vec::new();

        // 查询OSV数据库
        match self.query_osv_database(ecosystem, package, version).await {
            Ok(mut vulns) => all_vulnerabilities.append(&mut vulns),
            Err(e) => tracing::warn!("OSV查询失败: {}", e),
        }

        // 查询GitHub Advisory Database
        match self.query_github_advisory(ecosystem, package).await {
            Ok(mut vulns) => all_vulnerabilities.append(&mut vulns),
            Err(e) => tracing::warn!("GitHub Advisory查询失败: {}", e),
        }

        // 对于Rust包，额外查询RustSec
        if ecosystem == "cargo" || ecosystem == "rust" {
            match self.query_rustsec_database(package).await {
                Ok(mut vulns) => all_vulnerabilities.append(&mut vulns),
                Err(e) => tracing::warn!("RustSec查询失败: {}", e),
            }
        }

        // 去重（基于ID）
        all_vulnerabilities.sort_by(|a, b| a.id.cmp(&b.id));
        all_vulnerabilities.dedup_by(|a, b| a.id == b.id);

        Ok(all_vulnerabilities)
    }

    // 映射生态系统名称到OSV格式
    fn map_ecosystem_to_osv(&self, ecosystem: &str) -> String {
        match ecosystem.to_lowercase().as_str() {
            "npm" | "javascript" | "typescript" | "node" => "npm".to_string(),
            "pip" | "python" => "PyPI".to_string(),
            "cargo" | "rust" => "crates.io".to_string(),
            "maven" | "java" => "Maven".to_string(),
            "go" => "Go".to_string(),
            "pub" | "dart" | "flutter" => "Pub".to_string(),
            _ => ecosystem.to_string(),
        }
    }

    // 映射生态系统名称到GitHub格式
    fn map_ecosystem_to_github(&self, ecosystem: &str) -> String {
        match ecosystem.to_lowercase().as_str() {
            "npm" | "javascript" | "typescript" | "node" => "npm".to_string(),
            "pip" | "python" => "pip".to_string(),
            "cargo" | "rust" => "cargo".to_string(),
            "maven" | "java" => "maven".to_string(),
            "go" => "go".to_string(),
            "pub" | "dart" | "flutter" => "pub".to_string(),
            _ => ecosystem.to_string(),
        }
    }

    // 转换OSV漏洞格式
    fn convert_osv_to_security_vuln(&self, osv_vuln: OSVVulnerability, ecosystem: &str) -> Result<SecurityVulnerability> {
        let severity = osv_vuln.severity
            .as_ref()
            .and_then(|severities| severities.first())
            .map(|s| s.score.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let cvss_score = if severity.starts_with("CVSS:") {
            severity.split(':').nth(1)
                .and_then(|score| score.parse::<f64>().ok())
        } else {
            None
        };

        let cve_id = osv_vuln.aliases
            .as_ref()
            .and_then(|aliases| {
                aliases.iter().find(|alias| alias.starts_with("CVE-"))
            })
            .cloned();

        let affected_versions = osv_vuln.affected
            .as_ref()
            .map(|affected| {
                affected.iter()
                    .flat_map(|a| {
                        if let Some(versions) = a.versions.as_ref() {
                            versions.iter().cloned().collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let fixed_versions = osv_vuln.affected
            .as_ref()
            .map(|affected| {
                affected.iter()
                    .flat_map(|a| {
                        if let Some(ranges) = a.ranges.as_ref() {
                            ranges.iter()
                                .flat_map(|r| &r.events)
                                .filter_map(|e| e.fixed.as_ref())
                                .cloned()
                                .collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let references = osv_vuln.references
            .as_ref()
            .map(|refs| refs.iter().map(|r| r.url.clone()).collect())
            .unwrap_or_default();

        Ok(SecurityVulnerability {
            id: osv_vuln.id,
            summary: osv_vuln.summary.unwrap_or_else(|| "无摘要".to_string()),
            details: osv_vuln.details.unwrap_or_else(|| "无详细信息".to_string()),
            severity: self.normalize_severity(&severity),
            cvss_score,
            cve_id,
            published: osv_vuln.published
                .and_then(|p| DateTime::parse_from_rfc3339(&p).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            modified: osv_vuln.modified
                .and_then(|m| DateTime::parse_from_rfc3339(&m).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            affected_versions,
            fixed_versions,
            references,
            ecosystem: ecosystem.to_string(),
        })
    }

    // 转换GitHub Advisory格式
    fn convert_github_advisory_to_security_vuln(&self, advisory: Value, ecosystem: &str) -> Result<SecurityVulnerability> {
        let id = advisory["ghsa_id"].as_str().unwrap_or("unknown").to_string();
        let summary = advisory["summary"].as_str().unwrap_or("无摘要").to_string();
        let description = advisory["description"].as_str().unwrap_or("无描述").to_string();
        let severity = advisory["severity"].as_str().unwrap_or("unknown").to_string();
        let cvss_score = advisory["cvss"]["score"].as_f64();
        let cve_id = advisory["cve_id"].as_str().map(String::from);

        let published = advisory["published_at"]
            .as_str()
            .and_then(|p| DateTime::parse_from_rfc3339(p).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let updated = advisory["updated_at"]
            .as_str()
            .and_then(|u| DateTime::parse_from_rfc3339(u).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Ok(SecurityVulnerability {
            id,
            summary,
            details: description,
            severity: self.normalize_severity(&severity),
            cvss_score,
            cve_id,
            published,
            modified: updated,
            affected_versions: Vec::new(), // GitHub API可能需要额外查询
            fixed_versions: Vec::new(),
            references: vec![format!("https://github.com/advisories/{}", advisory["ghsa_id"].as_str().unwrap_or(""))],
            ecosystem: ecosystem.to_string(),
        })
    }

    // 转换RustSec格式
    fn convert_rustsec_to_security_vuln(&self, advisory: Value, package: &str) -> Result<SecurityVulnerability> {
        let id = advisory["id"].as_str().unwrap_or("unknown").to_string();
        let title = advisory["title"].as_str().unwrap_or("无标题").to_string();
        let description = advisory["description"].as_str().unwrap_or("无描述").to_string();

        Ok(SecurityVulnerability {
            id,
            summary: title,
            details: description,
            severity: "MEDIUM".to_string(), // RustSec通常不提供CVSS评分
            cvss_score: None,
            cve_id: None,
            published: Utc::now(), // 需要从advisory中解析
            modified: Utc::now(),
            affected_versions: Vec::new(),
            fixed_versions: Vec::new(),
            references: vec![format!("https://rustsec.org/advisories/{}", package)],
            ecosystem: "cargo".to_string(),
        })
    }

    // 标准化严重程度
    fn normalize_severity(&self, severity: &str) -> String {
        match severity.to_uppercase().as_str() {
            "LOW" | "MINOR" => "LOW".to_string(),
            "MEDIUM" | "MODERATE" => "MEDIUM".to_string(),
            "HIGH" | "IMPORTANT" => "HIGH".to_string(),
            "CRITICAL" | "SEVERE" => "CRITICAL".to_string(),
            _ => "UNKNOWN".to_string(),
        }
    }
}

#[async_trait]
impl MCPTool for SecurityCheckTool {
    fn name(&self) -> &str {
        "check_security_vulnerabilities"
    }

    fn description(&self) -> &str {
        "在需要检查包的安全漏洞、CVE信息或安全风险评估时，查询指定包的安全漏洞信息，包括漏洞详情、严重程度、影响版本和修复建议。"
    }

    fn parameters_schema(&self) -> &Schema {
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        SCHEMA.get_or_init(|| {
            Schema::Object(SchemaObject {
                required: vec!["ecosystem".to_string(), "package".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert(
                        "ecosystem".to_string(),
                        Schema::String(SchemaString {
                            description: Some("包所属的生态系统".to_string()),
                            enum_values: Some(vec![
                                "npm".to_string(),
                                "pip".to_string(),
                                "cargo".to_string(),
                                "maven".to_string(),
                                "go".to_string(),
                                "pub".to_string(),
                            ]),
                        }),
                    );
                    map.insert(
                        "package".to_string(),
                        Schema::String(SchemaString {
                            description: Some("要检查的包名称".to_string()),
                            ..Default::default()
                        }),
                    );
                    map.insert(
                        "version".to_string(),
                        Schema::String(SchemaString {
                            description: Some("要检查的包版本（可选）".to_string()),
                            ..Default::default()
                        }),
                    );
                    map.insert(
                        "include_fixed".to_string(),
                        Schema::Boolean(SchemaBoolean {
                            description: Some("是否包含已修复的漏洞".to_string()),
                        }),
                    );
                    map
                },
                ..Default::default()
            })
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let ecosystem = params["ecosystem"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("缺少ecosystem参数".to_string()))?;

        let package = params["package"]
            .as_str()
            .ok_or_else(|| MCPError::InvalidParameter("缺少package参数".to_string()))?;

        let version = params["version"].as_str();
        let include_fixed = params["include_fixed"].as_bool().unwrap_or(true);

        // 检查缓存
        let cache_key = format!("{}:{}:{}", ecosystem, package, version.unwrap_or("*"));
        let cache_ttl = chrono::Duration::hours(6); // 安全信息缓存6小时

        {
            let cache = self.cache.read().await;
            if let Some((vulns, timestamp)) = cache.get(&cache_key) {
                if Utc::now() - *timestamp < cache_ttl {
                    return Ok(json!({
                        "package": package,
                        "ecosystem": ecosystem,
                        "version": version,
                        "vulnerabilities": vulns,
                        "total_count": vulns.len(),
                        "critical_count": vulns.iter().filter(|v| v.severity == "CRITICAL").count(),
                        "high_count": vulns.iter().filter(|v| v.severity == "HIGH").count(),
                        "medium_count": vulns.iter().filter(|v| v.severity == "MEDIUM").count(),
                        "low_count": vulns.iter().filter(|v| v.severity == "LOW").count(),
                    }));
                }
            }
        }

        // 查询安全漏洞
        let vulnerabilities = self.check_package_security(ecosystem, package, version).await?;

        // 过滤已修复的漏洞（如果需要）
        let filtered_vulns: Vec<_> = if include_fixed {
            vulnerabilities
        } else {
            vulnerabilities.into_iter()
                .filter(|v| v.fixed_versions.is_empty())
                .collect()
        };

        // 更新缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, (filtered_vulns.clone(), Utc::now()));
        }

        Ok(json!({
            "package": package,
            "ecosystem": ecosystem,
            "version": version,
            "vulnerabilities": filtered_vulns,
            "total_count": filtered_vulns.len(),
            "critical_count": filtered_vulns.iter().filter(|v| v.severity == "CRITICAL").count(),
            "high_count": filtered_vulns.iter().filter(|v| v.severity == "HIGH").count(),
            "medium_count": filtered_vulns.iter().filter(|v| v.severity == "MEDIUM").count(),
            "low_count": filtered_vulns.iter().filter(|v| v.severity == "LOW").count(),
            "scan_timestamp": Utc::now(),
        }))
    }
} 