use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, debug, warn};
use std::collections::{HashMap, HashSet};
use url::Url;
use chrono::{DateTime, Utc};
use regex;

use super::ai_service::{AIService, AIRequest};

/// 
/// API
pub struct IntelligentWebAnalyzer {
    ai_service: AIService,
    analysis_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, CachedAnalysis>>>,
}

/// 
#[derive(Debug, Clone)]
pub struct CrawlTask {
    /// ID
    pub task_id: String,
    /// "tokioAPI"
    pub target_description: String,
    /// URL
    pub start_url: String,
    /// /
    pub library_name: String,
    /// 
    pub programming_language: String,
    /// 
    pub expected_content_types: Vec<ContentType>,
    /// 
    pub max_depth: u32,
    /// 
    pub max_pages: u32,
    /// 
    pub created_at: DateTime<Utc>,
}

/// 
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Documentation,
    Tutorial,
    ApiReference,
    Examples,
    GettingStarted,
    Installation,
    Configuration,
    Troubleshooting,
    Changelog,
    Community,
}

/// 
#[derive(Debug, Clone)]
pub struct PageRelevanceAnalysis {
    ///  (0.0-1.0)
    pub relevance_score: f32,
    /// 
    pub is_relevant: bool,
    /// 
    pub relevance_reasons: Vec<String>,
    /// 
    pub detected_content_types: Vec<ContentType>,
    ///  (1-5)
    pub importance_level: u8,
    /// 
    pub recommended_actions: Vec<RecommendedAction>,
}

/// 
#[derive(Debug, Clone)]
pub enum RecommendedAction {
    ExtractContent,
    FollowLinks,
    SkipPage,
    PrioritizeHighly,
    AnalyzeDeeper,
}

/// 
#[derive(Debug, Clone)]
pub struct ContentRegionAnalysis {
    /// 
    pub main_content_regions: Vec<ContentRegion>,
    /// 
    pub navigation_regions: Vec<ContentRegion>,
    /// 
    pub sidebar_regions: Vec<ContentRegion>,
    /// 
    pub related_links_regions: Vec<ContentRegion>,
    /// 
    pub code_regions: Vec<ContentRegion>,
}

/// 
#[derive(Debug, Clone)]
pub struct ContentRegion {
    /// 
    pub region_type: RegionType,
    /// 
    pub content: String,
    /// 
    pub relevance_score: f32,
    /// HTML
    pub selector_path: Option<String>,
    /// 
    pub extracted_links: Vec<ExtractedLink>,
}

/// 
#[derive(Debug, Clone)]
pub enum RegionType {
    MainContent,
    Navigation,
    Sidebar,
    CodeExample,
    ApiDocumentation,
    Tutorial,
    RelatedLinks,
    TableOfContents,
}

/// 
#[derive(Debug, Clone)]
pub struct ExtractedLink {
    /// URL
    pub url: String,
    /// 
    pub text: String,
    /// 
    pub link_type: LinkType,
    /// 
    pub relevance_score: f32,
    ///  (1-5)
    pub priority: u8,
}

/// 
#[derive(Debug, Clone)]
pub enum LinkType {
    Documentation,
    Tutorial,
    ApiReference,
    Example,
    Download,
    ExternalReference,
    Navigation,
    Related,
}

/// 
#[derive(Debug, Clone)]
struct CachedAnalysis {
    relevance_analysis: PageRelevanceAnalysis,
    content_regions: ContentRegionAnalysis,
    timestamp: DateTime<Utc>,
}

impl IntelligentWebAnalyzer {
    /// 
    pub async fn new(ai_service: AIService) -> Result<Self> {
        Ok(Self {
            ai_service,
            analysis_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// 
    pub async fn analyze_page_relevance(&self, html_content: &str, url: &str, task: &CrawlTask) -> Result<PageRelevanceAnalysis> {
        info!(" : {} (: {})", url, task.target_description);

        // 
        let cache_key = format!("relevance:{}:{}", url, task.task_id);
        if let Some(cached) = self.get_cached_relevance(&cache_key).await {
            debug!(" ");
            return Ok(cached);
        }

        // AI
        let system_prompt = self.get_relevance_analysis_system_prompt();
        let user_message = self.get_relevance_analysis_user_prompt(html_content, url, task);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.2),
            max_tokens: Some(3000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        let analysis = self.parse_relevance_analysis_response(&ai_response.content, task).await?;

        // 
        self.cache_relevance_analysis(&cache_key, &analysis).await;

        Ok(analysis)
    }

    /// 
    pub async fn analyze_content_regions(&self, html_content: &str, url: &str, task: &CrawlTask) -> Result<ContentRegionAnalysis> {
        info!(" : {} (: {})", url, task.target_description);

        // AI
        let system_prompt = self.get_content_region_system_prompt();
        let user_message = self.get_content_region_user_prompt(html_content, url, task);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(4000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        let regions = self.parse_content_region_response(&ai_response.content, task).await?;

        Ok(regions)
    }

    /// 
    pub async fn extract_relevant_links(&self, html_content: &str, current_url: &str, task: &CrawlTask) -> Result<Vec<ExtractedLink>> {
        info!(" : {} (: {})", current_url, task.target_description);

        let system_prompt = self.get_link_extraction_system_prompt();
        let user_message = self.get_link_extraction_user_prompt(html_content, current_url, task);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(3000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        let links = self.parse_link_extraction_response(&ai_response.content, current_url, task).await?;

        Ok(links)
    }

    ///  +  + 
    pub async fn comprehensive_page_analysis(&self, html_content: &str, url: &str, task: &CrawlTask) -> Result<(PageRelevanceAnalysis, ContentRegionAnalysis, Vec<ExtractedLink>)> {
        info!(" : {}", url);

        // 
        let (relevance_result, regions_result, links_result) = tokio::try_join!(
            self.analyze_page_relevance(html_content, url, task),
            self.analyze_content_regions(html_content, url, task),
            self.extract_relevant_links(html_content, url, task)
        )?;

        info!(" : {:.2}", relevance_result.relevance_score);

        Ok((relevance_result, regions_result, links_result))
    }

    /// 
    pub async fn generate_task_focused_summary(&self, content_regions: &ContentRegionAnalysis, task: &CrawlTask) -> Result<String> {
        info!(" ");

        // 
        let relevant_content: String = content_regions.main_content_regions
            .iter()
            .filter(|region| region.relevance_score > 0.7)
            .map(|region| region.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n");

        let system_prompt = self.get_task_summary_system_prompt();
        let user_message = self.get_task_summary_user_prompt(&relevant_content, task);

        let ai_request = AIRequest {
            model: None,
            system_prompt: Some(system_prompt),
            user_message,
            temperature: Some(0.3),
            max_tokens: Some(2000),
            stream: false,
        };

        let ai_response = self.ai_service.request(ai_request).await?;
        Ok(ai_response.content)
    }

    /// 
    fn get_relevance_analysis_system_prompt(&self) -> String {
        r#"


1.  - /
2.  - API
3.  - 
4.  - 
5.  - 

JSON
- relevance_score: (0.0-1.0)
- is_relevant: (true/false)
- relevance_reasons: 
- detected_content_types: 
- importance_level: (1-5)
- recommended_actions: 

"#.to_string()
    }

    /// 
    fn get_relevance_analysis_user_prompt(&self, html_content: &str, url: &str, task: &CrawlTask) -> String {
        // HTMLtoken
        let truncated_content = if html_content.len() > 6000 {
            format!("{}...", &html_content[..6000])
        } else {
            html_content.to_string()
        };

        format!(r#"

{}
{}
{}
{:?}

URL{}

{}

"#, 
            task.target_description,
            task.library_name,
            task.programming_language,
            task.expected_content_types,
            url,
            truncated_content
        )
    }

    /// 
    fn get_content_region_system_prompt(&self) -> String {
        r#"


1.  - 
2.  - 
3.  - 
4.  - 
5. API - API
6.  - 
7.  - 


- 
- 
- 
- 

JSON"#.to_string()
    }

    /// 
    fn get_content_region_user_prompt(&self, html_content: &str, url: &str, task: &CrawlTask) -> String {
        let truncated_content = if html_content.len() > 7000 {
            format!("{}...", &html_content[..7000])
        } else {
            html_content.to_string()
        };

        format!(r#"

{}
{}

URL{}

{}

{}"#,
            task.target_description,
            task.library_name,
            url,
            truncated_content,
            task.library_name
        )
    }

    /// 
    fn get_link_extraction_system_prompt(&self) -> String {
        r#"


1. /
2. API
3. 
4. 


- URLURL
- 
- 
- (0.0-1.0)
- (1-5)

JSON"#.to_string()
    }

    /// 
    fn get_link_extraction_user_prompt(&self, html_content: &str, current_url: &str, task: &CrawlTask) -> String {
        let truncated_content = if html_content.len() > 6000 {
            format!("{}...", &html_content[..6000])
        } else {
            html_content.to_string()
        };

        format!(r#"

{}
{}
{}


{}

{}"#,
            task.target_description,
            task.library_name,
            current_url,
            truncated_content,
            task.library_name
        )
    }

    /// 
    fn get_task_summary_system_prompt(&self) -> String {
        r#"


1. 
2. 
3. 
4. 
5. 

"#.to_string()
    }

    /// 
    fn get_task_summary_user_prompt(&self, content: &str, task: &CrawlTask) -> String {
        format!(r#"

{}
{}
{}


{}

{}"#,
            task.target_description,
            task.library_name,
            task.programming_language,
            content,
            task.library_name
        )
    }

    /// 
    async fn parse_relevance_analysis_response(&self, response: &str, task: &CrawlTask) -> Result<PageRelevanceAnalysis> {
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let relevance_score = json_value.get("relevance_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.3) as f32;

            let is_relevant = json_value.get("is_relevant")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let relevance_reasons = json_value.get("relevance_reasons")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let detected_content_types = json_value.get("detected_content_types")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| self.parse_content_type(v.as_str().unwrap_or(""))).collect())
                .unwrap_or_default();

            let importance_level = json_value.get("importance_level")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as u8;

            let recommended_actions = json_value.get("recommended_actions")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| self.parse_recommended_action(v.as_str().unwrap_or(""))).collect())
                .unwrap_or_default();

            Ok(PageRelevanceAnalysis {
                relevance_score,
                is_relevant,
                relevance_reasons,
                detected_content_types,
                importance_level,
                recommended_actions,
            })
        } else {
            // 
            let is_relevant = response.to_lowercase().contains(&task.library_name.to_lowercase());
            Ok(PageRelevanceAnalysis {
                relevance_score: if is_relevant { 0.7 } else { 0.3 },
                is_relevant,
                relevance_reasons: vec!["".to_string()],
                detected_content_types: vec![ContentType::Documentation],
                importance_level: 3,
                recommended_actions: vec![if is_relevant { RecommendedAction::ExtractContent } else { RecommendedAction::SkipPage }],
            })
        }
    }

    /// 
    async fn parse_content_region_response(&self, response: &str, task: &CrawlTask) -> Result<ContentRegionAnalysis> {
        // 
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let mut main_content_regions = Vec::new();
            let mut navigation_regions = Vec::new();
            let mut sidebar_regions = Vec::new();
            let mut related_links_regions = Vec::new();
            let mut code_regions = Vec::new();

            // 
            if let Some(main_regions) = json_value.get("main_content_regions").and_then(|v| v.as_array()) {
                for region_data in main_regions {
                    if let Some(region) = self.parse_content_region(region_data, RegionType::MainContent) {
                        main_content_regions.push(region);
                    }
                }
            }

            // 
            if let Some(nav_regions) = json_value.get("navigation_regions").and_then(|v| v.as_array()) {
                for region_data in nav_regions {
                    if let Some(region) = self.parse_content_region(region_data, RegionType::Navigation) {
                        navigation_regions.push(region);
                    }
                }
            }

            // 
            if let Some(sidebar_regions_data) = json_value.get("sidebar_regions").and_then(|v| v.as_array()) {
                for region_data in sidebar_regions_data {
                    if let Some(region) = self.parse_content_region(region_data, RegionType::Sidebar) {
                        sidebar_regions.push(region);
                    }
                }
            }

            // 
            if let Some(links_regions) = json_value.get("related_links_regions").and_then(|v| v.as_array()) {
                for region_data in links_regions {
                    if let Some(region) = self.parse_content_region(region_data, RegionType::RelatedLinks) {
                        related_links_regions.push(region);
                    }
                }
            }

            // 
            if let Some(code_regions_data) = json_value.get("code_regions").and_then(|v| v.as_array()) {
                for region_data in code_regions_data {
                    if let Some(region) = self.parse_content_region(region_data, RegionType::CodeExample) {
                        code_regions.push(region);
                    }
                }
            }

            Ok(ContentRegionAnalysis {
                main_content_regions,
                navigation_regions,
                sidebar_regions,
                related_links_regions,
                code_regions,
            })
        } else {
            // 
            self.parse_content_regions_from_text(response, "", task).await
        }
    }

    /// 
    async fn parse_link_extraction_response(&self, response: &str, current_url: &str, task: &CrawlTask) -> Result<Vec<ExtractedLink>> {
        // 
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            let mut extracted_links = Vec::new();

            if let Some(links_array) = json_value.get("extracted_links").and_then(|v| v.as_array()) {
                for link_data in links_array {
                    if let Some(link) = self.parse_extracted_link(link_data, current_url, task) {
                        extracted_links.push(link);
                    }
                }
            }

            // 
            extracted_links.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

            Ok(extracted_links)
        } else {
            // 
            self.extract_links_from_text(response, current_url, task).await
        }
    }

    /// 
    fn parse_content_region(&self, region_data: &Value, default_type: RegionType) -> Option<ContentRegion> {
        let content = region_data.get("content")?.as_str()?.to_string();
        let relevance_score = region_data.get("relevance_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;
        let selector_path = region_data.get("selector_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let region_type = region_data.get("region_type")
            .and_then(|v| v.as_str())
            .and_then(|s| self.parse_region_type(s))
            .unwrap_or(default_type);

        let extracted_links = region_data.get("extracted_links")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|link_data| self.parse_extracted_link(link_data, "", &CrawlTask {
                        task_id: String::new(),
                        target_description: String::new(),
                        start_url: String::new(),
                        library_name: String::new(),
                        programming_language: String::new(),
                        expected_content_types: Vec::new(),
                        max_depth: 0,
                        max_pages: 0,
                        created_at: chrono::Utc::now(),
                    }))
                    .collect()
            })
            .unwrap_or_default();

        Some(ContentRegion {
            region_type,
            content,
            relevance_score,
            selector_path,
            extracted_links,
        })
    }

    /// 
    fn parse_extracted_link(&self, link_data: &Value, current_url: &str, task: &CrawlTask) -> Option<ExtractedLink> {
        let url = link_data.get("url")?.as_str()?.to_string();
        let text = link_data.get("text")?.as_str()?.to_string();
        
        let link_type = link_data.get("link_type")
            .and_then(|v| v.as_str())
            .and_then(|s| self.parse_link_type(s))
            .unwrap_or(LinkType::Related);

        let relevance_score = link_data.get("relevance_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let priority = link_data.get("priority")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u8;

        Some(ExtractedLink {
            url,
            text,
            link_type,
            relevance_score,
            priority,
        })
    }

    /// 
    fn parse_region_type(&self, type_str: &str) -> Option<RegionType> {
        match type_str.to_lowercase().as_str() {
            "main_content" => Some(RegionType::MainContent),
            "navigation" => Some(RegionType::Navigation),
            "sidebar" => Some(RegionType::Sidebar),
            "code_example" => Some(RegionType::CodeExample),
            "api_documentation" => Some(RegionType::ApiDocumentation),
            "tutorial" => Some(RegionType::Tutorial),
            "related_links" => Some(RegionType::RelatedLinks),
            "table_of_contents" => Some(RegionType::TableOfContents),
            _ => None,
        }
    }

    /// 
    fn parse_link_type(&self, type_str: &str) -> Option<LinkType> {
        match type_str.to_lowercase().as_str() {
            "documentation" => Some(LinkType::Documentation),
            "tutorial" => Some(LinkType::Tutorial),
            "api_reference" => Some(LinkType::ApiReference),
            "example" => Some(LinkType::Example),
            "download" => Some(LinkType::Download),
            "external_reference" => Some(LinkType::ExternalReference),
            "navigation" => Some(LinkType::Navigation),
            "related" => Some(LinkType::Related),
            _ => None,
        }
    }

    /// 改进的内容区域分析（基于文本智能分析）
    async fn parse_content_regions_from_text(&self, text: &str, _current_url: &str, task: &CrawlTask) -> Result<ContentRegionAnalysis> {
        let mut main_content_regions = Vec::new();
        let mut navigation_regions = Vec::new(); 
        let mut code_regions = Vec::new();
        let mut related_links_regions = Vec::new();
        
        // 1. 智能内容分割
        let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
        
        for (index, paragraph) in paragraphs.iter().enumerate() {
            let content = paragraph.trim();
            if content.is_empty() { continue; }
            
            // 2. 基于关键词和模式的区域分类
            let (region_type, relevance_score) = self.classify_content_region(content, task);
            
            // 3. 提取该段落中的链接
            let extracted_links = self.extract_links_from_paragraph(content, task).await.unwrap_or_default();
            
            let region = ContentRegion {
                region_type: region_type.clone(),
                content: content.to_string(),
                relevance_score,
                selector_path: Some(format!("p[{}]", index)),
                extracted_links,
            };
            
            // 4. 按类型分组
            match region_type {
                RegionType::MainContent => main_content_regions.push(region),
                RegionType::Navigation => navigation_regions.push(region),
                RegionType::CodeExample => code_regions.push(region),
                RegionType::RelatedLinks => related_links_regions.push(region),
                _ => main_content_regions.push(region), // 默认归类为主要内容
            }
        }

        Ok(ContentRegionAnalysis {
            main_content_regions,
            navigation_regions,
            sidebar_regions: Vec::new(), // 文本分析模式下暂时不支持
            related_links_regions,
            code_regions,
        })
    }
    
    /// 智能内容区域分类
    fn classify_content_region(&self, content: &str, task: &CrawlTask) -> (RegionType, f32) {
        let content_lower = content.to_lowercase();
        let library_name_lower = task.library_name.to_lowercase();
        let programming_language_lower = task.programming_language.to_lowercase();
        
        let mut score = 0.0f32;
        let mut region_type = RegionType::MainContent;
        
        // 代码示例检测
        if self.is_code_content(&content_lower, &programming_language_lower) {
            region_type = RegionType::CodeExample;
            score = 0.9;
        }
        // API文档检测
        else if self.is_api_documentation(&content_lower, &library_name_lower) {
            region_type = RegionType::ApiDocumentation;
            score = 0.8;
        }
        // 教程内容检测
        else if self.is_tutorial_content(&content_lower) {
            region_type = RegionType::Tutorial;
            score = 0.7;
        }
        // 导航内容检测
        else if self.is_navigation_content(&content_lower) {
            region_type = RegionType::Navigation;
            score = 0.4;
        }
        // 相关链接检测
        else if self.contains_multiple_links(content) {
            region_type = RegionType::RelatedLinks;
            score = 0.5;
        }
        // 主要内容检测
        else if content_lower.contains(&library_name_lower) || 
                content_lower.contains(&programming_language_lower) {
            score = 0.8;
        } else {
            score = 0.3; // 默认相关性
        }
        
        (region_type, score)
    }
    
    /// 检测是否为代码内容
    fn is_code_content(&self, content: &str, language: &str) -> bool {
        // 代码块标识符
        let code_indicators = vec![
            "```", "fn ", "def ", "class ", "import ", "use ",
            "function", "var ", "let ", "const ", "async fn",
            "#include", "public class", "struct ", "enum ",
        ];
        
        // 语言特定关键词
        let language_keywords = match language {
            "rust" => vec!["use ", "fn ", "struct ", "enum ", "impl ", "trait ", "async ", "await"],
            "python" => vec!["def ", "class ", "import ", "from ", "async def", "__init__"],
            "javascript" => vec!["function", "const ", "let ", "var ", "async ", "await", "=>"],
            "java" => vec!["public class", "private ", "public ", "import ", "package "],
            "go" => vec!["func ", "package ", "import ", "type ", "struct ", "interface "],
            _ => vec![],
        };
        
        // 检查通用代码指示器
        for indicator in code_indicators {
            if content.contains(indicator) {
                return true;
            }
        }
        
        // 检查语言特定关键词
        for keyword in language_keywords {
            if content.contains(keyword) {
                return true;
            }
        }
        
        // 检查代码特征：括号密度、分号等
        let brace_count = content.chars().filter(|&c| c == '{' || c == '}').count();
        let paren_count = content.chars().filter(|&c| c == '(' || c == ')').count();
        let total_chars = content.len();
        
        if total_chars > 0 {
            let special_char_ratio = (brace_count + paren_count) as f32 / total_chars as f32;
            return special_char_ratio > 0.05; // 5%以上的特殊字符
        }
        
        false
    }
    
    /// 检测是否为API文档内容
    fn is_api_documentation(&self, content: &str, library_name: &str) -> bool {
        let api_indicators = vec![
            "api", "method", "function", "parameter", "return", "example",
            "usage", "documentation", "reference", library_name,
            "endpoint", "request", "response", "struct", "trait",
        ];
        
        let mut matches = 0;
        for indicator in &api_indicators {
            if content.contains(indicator) {
                matches += 1;
            }
        }
        
        matches >= 2 // 至少匹配2个指示器
    }
    
    /// 检测是否为教程内容
    fn is_tutorial_content(&self, content: &str) -> bool {
        let tutorial_indicators = vec![
            "tutorial", "guide", "step", "how to", "getting started",
            "introduction", "first", "begin", "start", "learn",
            "example", "walkthrough", "lesson",
        ];
        
        for indicator in tutorial_indicators {
            if content.contains(indicator) {
                return true;
            }
        }
        
        false
    }
    
    /// 检测是否为导航内容
    fn is_navigation_content(&self, content: &str) -> bool {
        let nav_indicators = vec![
            "home", "back", "next", "previous", "menu", "index",
            "table of contents", "toc", "navigation", "nav",
            "breadcrumb", "sitemap",
        ];
        
        // 短内容更可能是导航
        if content.len() < 100 {
            for indicator in nav_indicators {
                if content.contains(indicator) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// 检测是否包含多个链接
    fn contains_multiple_links(&self, content: &str) -> bool {
        let url_pattern = regex::Regex::new(r"https?://[^\s<>]+").unwrap();
        let link_count = url_pattern.find_iter(content).count();
        link_count >= 3
    }
    
    /// 从段落中提取链接
    async fn extract_links_from_paragraph(&self, content: &str, task: &CrawlTask) -> Result<Vec<ExtractedLink>> {
        let mut links = Vec::new();
        let url_pattern = regex::Regex::new(r"https?://[^\s<>]+").unwrap();
        
        for url_match in url_pattern.find_iter(content) {
            let url = url_match.as_str().to_string();
            
            // 智能相关性评分
            let relevance_score = self.calculate_link_relevance(&url, content, task);
            
            // 链接类型推断
            let link_type = self.infer_link_type(&url, content);
            
            // 优先级计算
            let priority = self.calculate_link_priority(relevance_score, &link_type);
            
            // 提取链接周围的描述文本
            let surrounding_text = self.extract_surrounding_text(content, url_match.start());
            
            links.push(ExtractedLink {
                url: url.clone(),
                text: surrounding_text,
                link_type,
                relevance_score,
                priority,
            });
        }
        
        Ok(links)
    }
    
    /// 计算链接相关性
    fn calculate_link_relevance(&self, url: &str, context: &str, task: &CrawlTask) -> f32 {
        let mut score = 0.0f32;
        
        let url_lower = url.to_lowercase();
        let context_lower = context.to_lowercase();
        let library_name_lower = task.library_name.to_lowercase();
        let language_lower = task.programming_language.to_lowercase();
        
        // URL中包含库名或语言
        if url_lower.contains(&library_name_lower) {
            score += 0.4;
        }
        if url_lower.contains(&language_lower) {
            score += 0.3;
        }
        
        // 上下文相关性
        if context_lower.contains(&library_name_lower) {
            score += 0.2;
        }
        
        // 域名权威性
        if url_lower.contains("github.com") || url_lower.contains("docs.rs") || 
           url_lower.contains("python.org") || url_lower.contains("doc.rust-lang.org") {
            score += 0.2;
        }
        
        // 特定内容类型加分
        for expected_type in &task.expected_content_types {
            match expected_type {
                ContentType::Documentation if url_lower.contains("doc") => score += 0.1,
                ContentType::Tutorial if url_lower.contains("tutorial") => score += 0.1,
                ContentType::Examples if url_lower.contains("example") => score += 0.1,
                ContentType::ApiReference if url_lower.contains("api") => score += 0.1,
                _ => {}
            }
        }
        
        score.min(1.0)
    }
    
    /// 推断链接类型
    fn infer_link_type(&self, url: &str, context: &str) -> LinkType {
        let url_lower = url.to_lowercase();
        let context_lower = context.to_lowercase();
        
        if url_lower.contains("doc") || context_lower.contains("documentation") {
            LinkType::Documentation
        } else if url_lower.contains("tutorial") || context_lower.contains("tutorial") {
            LinkType::Tutorial
        } else if url_lower.contains("api") || context_lower.contains("api reference") {
            LinkType::ApiReference
        } else if url_lower.contains("example") || context_lower.contains("example") {
            LinkType::Example
        } else if url_lower.contains("download") || context_lower.contains("download") {
            LinkType::Download
        } else if url_lower.contains("github.com") || url_lower.contains("gitlab.com") {
            LinkType::ExternalReference
        } else {
            LinkType::Related
        }
    }
    
    /// 计算链接优先级
    fn calculate_link_priority(&self, relevance_score: f32, link_type: &LinkType) -> u8 {
        let mut priority = match link_type {
            LinkType::Documentation => 5,
            LinkType::ApiReference => 5,
            LinkType::Tutorial => 4,
            LinkType::Example => 4,
            LinkType::Download => 3,
            LinkType::ExternalReference => 3,
            LinkType::Navigation => 2,
            LinkType::Related => 2,
        };
        
        // 基于相关性调整优先级
        if relevance_score > 0.7 {
            priority = std::cmp::min(priority + 1, 5);
        } else if relevance_score < 0.3 {
            priority = std::cmp::max(priority - 1, 1);
        }
        
        priority
    }
    
    /// 提取链接周围的描述文本
    fn extract_surrounding_text(&self, content: &str, url_position: usize) -> String {
        let words: Vec<&str> = content.split_whitespace().collect();
        let url_word_index = words.iter().position(|&word| 
            word.contains("http") && content[url_position..].starts_with(word)
        );
        
        if let Some(index) = url_word_index {
            let start = index.saturating_sub(3);
            let end = std::cmp::min(index + 4, words.len());
            words[start..end].join(" ")
        } else {
            // 如果找不到确切位置，返回前后各20个字符
            let start = url_position.saturating_sub(20);
            let end = std::cmp::min(url_position + 20, content.len());
            content[start..end].to_string()
        }
    }

    /// 获取缓存的相关性分析
    async fn get_cached_relevance(&self, cache_key: &str) -> Option<PageRelevanceAnalysis> {
        let cache = self.analysis_cache.read().await;
        cache.get(cache_key).map(|cached| cached.relevance_analysis.clone())
    }

    /// 缓存相关性分析结果
    async fn cache_relevance_analysis(&self, cache_key: &str, analysis: &PageRelevanceAnalysis) {
        let mut cache = self.analysis_cache.write().await;
        cache.insert(cache_key.to_string(), CachedAnalysis {
            relevance_analysis: analysis.clone(),
            content_regions: ContentRegionAnalysis {
                main_content_regions: Vec::new(),
                navigation_regions: Vec::new(),
                sidebar_regions: Vec::new(),
                related_links_regions: Vec::new(),
                code_regions: Vec::new(),
            },
            timestamp: Utc::now(),
        });
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.analysis_cache.write().await;
        cache.clear();
    }

    /// Get cache stats
    pub async fn get_cache_stats(&self) -> usize {
        let cache = self.analysis_cache.read().await;
        cache.len()
    }

    /// 向后兼容的链接提取方法
    async fn extract_links_from_text(&self, text: &str, current_url: &str, task: &CrawlTask) -> Result<Vec<ExtractedLink>> {
        // 调用新的智能段落分析
        self.extract_links_from_paragraph(text, task).await
    }
    
    /// 解析内容类型（保留向后兼容）
    fn parse_content_type(&self, type_str: &str) -> Option<ContentType> {
        match type_str.to_lowercase().as_str() {
            "documentation" => Some(ContentType::Documentation),
            "tutorial" => Some(ContentType::Tutorial),
            "api_reference" => Some(ContentType::ApiReference),
            "examples" => Some(ContentType::Examples),
            "getting_started" => Some(ContentType::GettingStarted),
            "installation" => Some(ContentType::Installation),
            "configuration" => Some(ContentType::Configuration),
            "troubleshooting" => Some(ContentType::Troubleshooting),
            "changelog" => Some(ContentType::Changelog),
            "community" => Some(ContentType::Community),
            _ => None,
        }
    }
    
    /// 解析推荐操作（保留向后兼容）
    fn parse_recommended_action(&self, action_str: &str) -> Option<RecommendedAction> {
        match action_str.to_lowercase().as_str() {
            "extract_content" => Some(RecommendedAction::ExtractContent),
            "follow_links" => Some(RecommendedAction::FollowLinks),
            "skip_page" => Some(RecommendedAction::SkipPage),
            "prioritize_highly" => Some(RecommendedAction::PrioritizeHighly),
            "analyze_deeper" => Some(RecommendedAction::AnalyzeDeeper),
            _ => None,
        }
    }
}
