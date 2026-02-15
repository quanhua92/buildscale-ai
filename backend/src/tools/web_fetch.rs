use crate::{DbConn, error::{Error, Result}};
use crate::models::requests::{ToolResponse, WebFetchArgs, WebFetchResult, WebFetchFormat, WebLink};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};
use std::time::Instant;
use futures::StreamExt;
use dom_smoothie::{Readability, Config as ReadabilityConfig};

/// Default timeout for HTTP requests in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default maximum content size (1MB)
const DEFAULT_MAX_CONTENT_SIZE: usize = 1024 * 1024;

/// Hard ceiling for content size (5MB) - prevents abuse
const MAX_CONTENT_SIZE_CEILING: usize = 5 * 1024 * 1024;

/// Web fetch tool for retrieving and converting web content.
///
/// Fetches content from URLs and converts to AI-friendly formats.
/// Supports markdown (default), HTML, text, and JSON output formats.
///
/// # Examples
///
/// ```text
/// // Fetch a URL as markdown (default)
/// {"url": "https://example.com/docs"}
///
/// // Fetch with custom format
/// {"url": "https://api.example.com/data", "format": "json"}
///
/// // Fetch with custom headers
/// {"url": "https://api.example.com/data", "headers": {"Authorization": "Bearer token"}}
///
/// // Extract links from content
/// {"url": "https://example.com/page", "extract_links": true}
/// ```
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "web_fetch"
    }

    fn description(&self) -> &'static str {
        r#"Fetches and converts web content to AI-friendly formats.

FORMATS:
- markdown (default): Best for reading web pages, most token-efficient
- html: Raw HTML content
- text: Plain text with HTML tags stripped
- json: Smart extraction - for APIs returns pretty-printed JSON; for HTML extracts structured data (title, JSON-LD, Open Graph, meta tags, headings)

PARAMETERS: url (required), format ("markdown"/"html"/"text"/"json"), method, body, headers, timeout (seconds), follow_redirects, extract_links, max_content_size (bytes, default 1MB, max 5MB)

EXAMPLE: {"url":"https://docs.rs/serde","format":"markdown","max_content_size":512000}"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch (required)"
                },
                "format": {
                    "type": ["string", "null"],
                    "enum": ["markdown", "html", "text", "json", null],
                    "description": "Output format. Default: markdown (most token-efficient)"
                },
                "method": {
                    "type": ["string", "null"],
                    "description": "HTTP method. Default: GET"
                },
                "body": {
                    "type": ["string", "null"],
                    "description": "Request body (for POST/PUT)"
                },
                "headers": {
                    "type": ["object", "null"],
                    "additionalProperties": {"type": "string"},
                    "description": "Custom headers as key-value pairs"
                },
                "timeout": {
                    "type": ["integer", "string", "null"],
                    "description": "Timeout in seconds. Default: 30. Accepts integer or string."
                },
                "follow_redirects": {
                    "type": ["boolean", "string", "null"],
                    "description": "Follow redirects. Default: true. Accepts boolean or string."
                },
                "extract_links": {
                    "type": ["boolean", "string", "null"],
                    "description": "Extract links from content. Default: false. Accepts boolean or string."
                },
                "max_content_size": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum content size in bytes. Default: 1048576 (1MB), Max: 5242880 (5MB). Accepts integer or string."
                }
            },
            "required": ["url"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _conn: &mut DbConn,
        _storage: &FileStorageService,
        _workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let fetch_args: WebFetchArgs = serde_json::from_value(args)?;

        // Validate URL
        let url = &fetch_args.url;
        let parsed_url = url::Url::parse(url)
            .map_err(|e| Error::Validation(crate::error::ValidationErrors::Single {
                field: "url".to_string(),
                message: format!("Invalid URL '{}': {}", url, e),
            }))?;

        // Only allow http and https schemes
        if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some(format!("URL scheme '{}' not allowed. Only http and https are supported.", parsed_url.scheme())),
            });
        }

        // Build client with timeout and redirect settings
        let timeout_secs = fetch_args.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS as usize) as u64;
        let follow_redirects = fetch_args.follow_redirects.unwrap_or(true);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .redirect(if follow_redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .user_agent("BuildScale-AI/1.0")
            .build()
            .map_err(|e| Error::Internal(format!("Failed to create HTTP client: {}", e)))?;

        // Calculate max content size with ceiling
        let max_content_size = fetch_args.max_content_size
            .unwrap_or(DEFAULT_MAX_CONTENT_SIZE)
            .min(MAX_CONTENT_SIZE_CEILING);

        // Build request with method parsing
        let method = fetch_args.method.as_deref()
            .and_then(|m| m.parse::<reqwest::Method>().ok())
            .unwrap_or(reqwest::Method::GET);

        let mut request_builder = client.request(method, url.as_str());

        // Add custom headers
        if let Some(ref headers) = fetch_args.headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }

        // Add body for POST/PUT
        if let Some(body) = fetch_args.body {
            request_builder = request_builder.body(body);
        }

        // Execute request
        let start_time = Instant::now();
        let response = request_builder.send().await.map_err(|e| {
            Error::Internal(format!("HTTP request failed: {}", e))
        })?;

        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        let status_code = response.status().as_u16();
        let final_url = response.url().to_string();
        let content_type = response.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Get response body with streaming to prevent OOM
        let mut body_bytes = Vec::new();
        let mut stream = response.bytes_stream();
        let mut truncated = false;

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| Error::Internal(format!("Failed to read response chunk: {}", e)))?;
            if body_bytes.len() + chunk.len() > max_content_size {
                // Only take what fits within the limit
                let remaining = max_content_size.saturating_sub(body_bytes.len());
                if remaining > 0 {
                    body_bytes.extend_from_slice(&chunk[..remaining]);
                }
                truncated = true;
                break;
            }
            body_bytes.extend_from_slice(&chunk);
        }

        // Convert to string (handle binary content gracefully)
        let body_str = String::from_utf8_lossy(&body_bytes);

        // Convert to requested format
        let format = fetch_args.format.unwrap_or_default();
        let content = match format {
            WebFetchFormat::Json => {
                // Check if response is actually JSON (API endpoint)
                let is_json_response = content_type
                    .as_ref()
                    .map(|ct| ct.contains("application/json"))
                    .unwrap_or(false);

                if is_json_response {
                    // Pretty-print JSON API response
                    if let Ok(json_value) = serde_json::from_str::<Value>(&body_str) {
                        serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| body_str.to_string())
                    } else {
                        body_str.to_string()
                    }
                } else {
                    // Extract clean content first
                    let clean_content = extract_article_content(&body_str, &final_url);
                    // Extract structured data from HTML, including clean content
                    let structured = extract_structured_json(&body_str, clean_content.as_ref());
                    serde_json::to_string_pretty(&structured).unwrap_or_else(|_| body_str.to_string())
                }
            }
            WebFetchFormat::Html => {
                // Try clean extraction, fallback to raw
                if let Some((clean_html, _, _)) = extract_article_content(&body_str, &final_url) {
                    clean_html
                } else {
                    body_str.to_string()
                }
            }
            WebFetchFormat::Text => {
                // Try clean text extraction, fallback to scraper
                if let Some((_, clean_text, _)) = extract_article_content(&body_str, &final_url) {
                    clean_text
                } else {
                    html_to_text(&body_str)
                }
            }
            WebFetchFormat::Markdown => {
                // Try clean extraction first
                if let Some((clean_html, _, title)) = extract_article_content(&body_str, &final_url) {
                    let md = htmd::convert(&clean_html).unwrap_or_default();
                    format!("# {}\n\n{}", title, md)
                } else {
                    // Fallback: convert raw HTML
                    htmd::convert(&body_str).unwrap_or_default()
                }
            }
        };

        // Extract links if requested
        let links = if fetch_args.extract_links.unwrap_or(false) {
            Some(extract_links(&body_str, &final_url))
        } else {
            None
        };

        let result = WebFetchResult {
            url: final_url,
            status_code,
            content_type,
            content,
            content_size: body_bytes.len(),
            elapsed_ms,
            links,
            truncated,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Convert HTML to plain text by stripping tags
fn html_to_text(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Try to get text content from body, fallback to entire document
    let selector = Selector::parse("body").ok();
    let text = if let Some(sel) = selector {
        document.select(&sel)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| html.to_string())
    } else {
        html.to_string()
    };

    // Clean up whitespace
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract clean article content using dom_smoothie (Mozilla Readability)
/// Returns (clean_html, clean_text, title) or None if extraction fails
fn extract_article_content(html: &str, url: &str) -> Option<(String, String, String)> {
    let config = ReadabilityConfig::default();
    let mut readability = Readability::new(html, Some(url), Some(config)).ok()?;

    match readability.parse() {
        Ok(article) => Some((
            article.content.to_string(),      // Clean HTML
            article.text_content.to_string(), // Plain text
            article.title,
        )),
        Err(_) => None,
    }
}

/// Extract links from HTML content
fn extract_links(html: &str, base_url: &str) -> Vec<WebLink> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let selector = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let base = match url::Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return Vec::new(),
    };

    let mut links = Vec::new();
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            let text = element.text().collect::<String>()
                .trim()
                .to_string();

            // Resolve relative URLs
            let full_url = if href.starts_with("http://") || href.starts_with("https://") {
                href.to_string()
            } else {
                base.join(href)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| href.to_string())
            };

            // Skip empty links and javascript:
            if !text.is_empty() && !full_url.starts_with("javascript:") {
                links.push(WebLink {
                    text,
                    url: full_url,
                });
            }
        }
    }

    // Deduplicate by URL
    let mut seen = std::collections::HashSet::new();
    links.retain(|link| seen.insert(link.url.clone()));

    links
}

/// Extract structured data from HTML as JSON
/// Includes: clean content (Readability), JSON-LD, Open Graph tags, Twitter cards, and meta tags
fn extract_structured_json(html: &str, clean_content: Option<&(String, String, String)>) -> Value {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut result = serde_json::Map::new();

    // Add clean content if available (from Readability extraction)
    if let Some((clean_html, clean_text, title)) = clean_content {
        if !title.is_empty() {
            result.insert("title".to_string(), Value::String(title.clone()));
        }
        if !clean_text.is_empty() {
            result.insert("content".to_string(), Value::String(clean_text.clone()));
        }
        if !clean_html.is_empty() {
            result.insert("content_html".to_string(), Value::String(clean_html.clone()));
        }
    } else {
        // Fallback: Extract title from HTML if no clean content
        if let Ok(title_selector) = Selector::parse("title") {
            if let Some(title_el) = document.select(&title_selector).next() {
                let title = title_el.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    result.insert("title".to_string(), Value::String(title));
                }
            }
        }
    }

    // Extract JSON-LD data (Schema.org)
    let mut json_ld_list = Vec::new();
    if let Ok(ld_selector) = Selector::parse(r#"script[type="application/ld+json"]"#) {
        for script in document.select(&ld_selector) {
            let json_text = script.text().collect::<String>();
            if let Ok(json_value) = serde_json::from_str::<Value>(&json_text) {
                json_ld_list.push(json_value);
            }
        }
    }
    if !json_ld_list.is_empty() {
        result.insert("json_ld".to_string(), Value::Array(json_ld_list));
    }

    // Extract Open Graph tags
    let mut og_data = serde_json::Map::new();
    if let Ok(og_selector) = Selector::parse(r#"meta[property^="og:"]"#) {
        for meta in document.select(&og_selector) {
            if let Some(property) = meta.value().attr("property") {
                if let Some(content) = meta.value().attr("content") {
                    let key = property.strip_prefix("og:").unwrap_or(property).to_string();
                    og_data.insert(key, Value::String(content.to_string()));
                }
            }
        }
    }
    if !og_data.is_empty() {
        result.insert("open_graph".to_string(), Value::Object(og_data));
    }

    // Extract Twitter cards
    let mut twitter_data = serde_json::Map::new();
    if let Ok(twitter_selector) = Selector::parse(r#"meta[name^="twitter:"]"#) {
        for meta in document.select(&twitter_selector) {
            if let Some(name) = meta.value().attr("name") {
                if let Some(content) = meta.value().attr("content") {
                    let key = name.strip_prefix("twitter:").unwrap_or(name).to_string();
                    twitter_data.insert(key, Value::String(content.to_string()));
                }
            }
        }
    }
    if !twitter_data.is_empty() {
        result.insert("twitter".to_string(), Value::Object(twitter_data));
    }

    // Extract standard meta tags
    let mut meta_data = serde_json::Map::new();
    let meta_names = ["description", "keywords", "author", "viewport", "robots"];
    for name in meta_names {
        if let Ok(meta_selector) = Selector::parse(&format!(r#"meta[name="{}"]"#, name)) {
            if let Some(meta) = document.select(&meta_selector).next() {
                if let Some(content) = meta.value().attr("content") {
                    meta_data.insert(name.to_string(), Value::String(content.to_string()));
                }
            }
        }
    }
    if !meta_data.is_empty() {
        result.insert("meta".to_string(), Value::Object(meta_data));
    }

    // Extract canonical URL
    if let Ok(canonical_selector) = Selector::parse(r#"link[rel="canonical"]"#) {
        if let Some(link) = document.select(&canonical_selector).next() {
            if let Some(href) = link.value().attr("href") {
                result.insert("canonical_url".to_string(), Value::String(href.to_string()));
            }
        }
    }

    // Extract main headings
    let mut headings = Vec::new();
    if let Ok(h_selector) = Selector::parse("h1, h2") {
        for h in document.select(&h_selector) {
            let text = h.text().collect::<String>().trim().to_string();
            if !text.is_empty() && text.len() < 200 {
                let tag = h.value().name().to_string();
                let mut heading_map = serde_json::Map::new();
                heading_map.insert("level".to_string(), Value::String(tag));
                heading_map.insert("text".to_string(), Value::String(text));
                headings.push(Value::Object(heading_map));
            }
            if headings.len() >= 10 {
                break;
            }
        }
    }
    if !headings.is_empty() {
        result.insert("headings".to_string(), Value::Array(headings));
    }

    Value::Object(result)
}
