use crate::{DbConn, error::{Error, Result}};
use crate::models::requests::{
    ToolResponse, WebSearchArgs, WebSearchResult, SearchResultItem,
};
use crate::services::storage::FileStorageService;
use crate::utils::safe_preview;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};
use regex::Regex;
use std::sync::OnceLock;

// Cache the VQD regex to avoid recompiling on every call
static VQD_REGEX: OnceLock<Regex> = OnceLock::new();

/// Default maximum number of results
const DEFAULT_MAX_RESULTS: usize = 10;

/// Web search tool using DuckDuckGo with multiple fallback strategies.
///
/// # Examples
///
/// ```text
/// // Simple search
/// {"query": "rust programming language"}
///
/// // Search with more results
/// {"query": "tokio async runtime", "max_results": 20}
/// ```
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        r#"Searches the web using DuckDuckGo.

Returns web search results for any query.

PARAMETERS: query (required), max_results (default 10), offset

EXAMPLE: {"query":"rust async programming","max_results":5}"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (required)"
                },
                "max_results": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum results to return. Default: 10. Accepts integer or string."
                },
                "offset": {
                    "type": ["integer", "string", "null"],
                    "description": "Result offset for pagination. Accepts integer or string."
                }
            },
            "required": ["query"],
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
        let search_args: WebSearchArgs = serde_json::from_value(args)?;

        let max_results = search_args.max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        let offset = search_args.offset.unwrap_or(0);

        // Try multiple search strategies in order
        let results = search_with_fallbacks(&search_args.query).await?;

        // Apply pagination
        let total = results.len();
        let paginated_results: Vec<SearchResultItem> = results
            .into_iter()
            .skip(offset)
            .take(max_results)
            .collect();

        let result = WebSearchResult {
            query: search_args.query,
            provider: "duckduckgo".to_string(),
            total,
            results: paginated_results,
            answer: None,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Search with multiple fallback strategies
async fn search_with_fallbacks(query: &str) -> Result<Vec<SearchResultItem>> {
    tracing::info!(query = %query, "Starting web search with fallbacks");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| Error::Internal(format!("Failed to create HTTP client: {}", e)))?;

    // Strategy 1: Try DuckDuckGo Lite (HTML parsing) - less protected
    match search_duckduckgo_lite(&client, query).await {
        Ok(results) if !results.is_empty() => {
            tracing::info!(query = %query, results_count = results.len(), strategy = "lite", "Search successful");
            return Ok(results);
        }
        Ok(_) => tracing::debug!(query = %query, "Lite search returned empty"),
        Err(e) => tracing::debug!(query = %query, error = %e, "Lite search failed"),
    }

    // Strategy 2: Try VQD token approach (may be blocked by anti-bot)
    match search_with_vqd_strategy(&client, query).await {
        Ok(results) if !results.is_empty() => {
            tracing::info!(query = %query, results_count = results.len(), strategy = "vqd", "Search successful");
            return Ok(results);
        }
        Ok(_) => tracing::debug!(query = %query, "VQD search returned empty"),
        Err(e) => tracing::debug!(query = %query, error = %e, "VQD search failed"),
    }

    // Strategy 3: Fallback to instant answer API (limited but always works)
    tracing::info!(query = %query, "Falling back to instant answer API");
    search_instant_answer(&client, query).await
}

/// Search using DuckDuckGo Lite (HTML endpoint) - requires POST request
async fn search_duckduckgo_lite(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResultItem>> {
    tracing::debug!(query = %query, "Trying DuckDuckGo Lite search (POST)");

    let body_str = format!("q={}", urlencoding::encode(query));
    let response = client
        .post("https://lite.duckduckgo.com/lite/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "text/html")
        .body(body_str)
        .send()
        .await
        .map_err(|e| Error::Internal(format!("Lite search request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        tracing::debug!(status = %status, "Lite search returned non-success status");
        return Ok(Vec::new());
    }

    let body = response.text().await
        .map_err(|e| Error::Internal(format!("Failed to read lite response: {}", e)))?;

    tracing::debug!(body_len = body.len(), "Got lite response");

    parse_lite_html_response(&body)
}

/// Parse DuckDuckGo Lite HTML response
fn parse_lite_html_response(html: &str) -> Result<Vec<SearchResultItem>> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut results = Vec::new();

    // DuckDuckGo Lite uses <a class='result-link' href="..."> for search results
    let selector = match Selector::parse("a.result-link") {
        Ok(s) => s,
        Err(_) => return Ok(results),
    };

    for element in document.select(&selector) {
        let href = element.value().attr("href").unwrap_or("");
        let text = element.text().collect::<String>().trim().to_string();

        // Skip ad links (they go through duckduckgo.com/y.js)
        if href.contains("duckduckgo.com/y.js") || href.contains("ad_provider") {
            continue;
        }

        // Skip empty or internal links
        if href.is_empty() || text.is_empty() || href.starts_with('/') {
            continue;
        }

        // Only include external links (http/https)
        if href.starts_with("http://") || href.starts_with("https://") {
            results.push(SearchResultItem {
                title: text,
                url: href.to_string(),
                snippet: String::new(), // Lite version doesn't have snippets easily
                published_date: None,
            });
        }
    }

    tracing::info!(results_count = results.len(), "Parsed lite HTML response");
    Ok(results)
}

/// Search using VQD token strategy (original approach)
async fn search_with_vqd_strategy(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResultItem>> {
    // Step 1: Get VQD token
    let vqd = get_vqd(client, query).await?;
    tracing::debug!(query = %query, vqd = %vqd, "Got VQD token");

    // Step 2: Search with VQD token
    let results = search_with_vqd(client, query, &vqd).await?;
    tracing::info!(query = %query, results_count = results.len(), "Got results from d.js");

    Ok(results)
}

/// Get VQD token from DuckDuckGo
async fn get_vqd(client: &reqwest::Client, query: &str) -> Result<String> {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://duckduckgo.com/?q={}", encoded_query);
    tracing::debug!(url = %url, "Fetching VQD token");

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Internal(format!("Failed to get VQD: {}", e)))?;

    let body = response.text().await
        .map_err(|e| Error::Internal(format!("Failed to read VQD response: {}", e)))?;

    // Extract VQD from response (looks like vqd='...' or vqd":"...")
    let vqd_re = VQD_REGEX.get_or_init(|| {
        Regex::new(r#"vqd\s*[=:'"]+\s*['"]?([a-zA-Z0-9_-]+)['"]?"#).unwrap()
    });

    vqd_re.captures(&body)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .ok_or_else(|| {
            tracing::error!(query = %query, body_preview = %safe_preview(&body, 500), "Failed to extract VQD token");
            Error::Internal("Failed to extract VQD token from DuckDuckGo".to_string())
        })
}

/// Search DuckDuckGo with VQD token
async fn search_with_vqd(client: &reqwest::Client, query: &str, vqd: &str) -> Result<Vec<SearchResultItem>> {
    // Use DuckDuckGo's d.js endpoint which returns JSON
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://links.duckduckgo.com/d.js?q={}&vqd={}&kl=wt-wt",
        encoded_query, vqd
    );
    tracing::debug!(url = %url, "Searching with VQD token");

    let response = client
        .get(&url)
        .header("Accept", "application/javascript, */*")
        .send()
        .await
        .map_err(|e| Error::Internal(format!("Search request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        tracing::warn!(status = %status, "Search request returned non-success status");
        return Ok(Vec::new());
    }

    let body = response.text().await
        .map_err(|e| Error::Internal(format!("Failed to read search response: {}", e)))?;

    tracing::debug!(body_len = body.len(), body_preview = %safe_preview(&body, 200), "Got d.js response");

    // Parse the JSONP-like response
    parse_ddg_js_response(&body)
}

/// Parse DuckDuckGo d.js response (JSONP format)
fn parse_ddg_js_response(body: &str) -> Result<Vec<SearchResultItem>> {
    let mut results = Vec::new();

    // The response looks like: ddg_spice_search_results({...}); or similar
    // Extract JSON from the response
    let body = body.trim();

    // Try to find JSON array in the response
    let json_start = body.find('[').unwrap_or(0);
    let json_end = body.rfind(']').map(|i| i + 1).unwrap_or(body.len());

    if json_start >= json_end {
        tracing::warn!(body_preview = %safe_preview(body, 200), "No JSON array found in d.js response");
        return Ok(results);
    }

    let json_str = &body[json_start..json_end];

    // Parse as array of objects
    match serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        Ok(items) => {
            for item in items {
                if let (Some(title), Some(url), Some(snippet)) = (
                    item.get("t").and_then(|v| v.as_str()),
                    item.get("u").and_then(|v| v.as_str()),
                    item.get("a").and_then(|v| v.as_str()),
                ) {
                    results.push(SearchResultItem {
                        title: title.to_string(),
                        url: url.to_string(),
                        snippet: snippet.to_string(),
                        published_date: None,
                    });
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, json_preview = %safe_preview(json_str, 200), "Failed to parse d.js JSON");
        }
    }

    tracing::info!(results_count = results.len(), "Parsed d.js response");
    Ok(results)
}

/// Search using DuckDuckGo Instant Answer API (fallback)
async fn search_instant_answer(client: &reqwest::Client, query: &str) -> Result<Vec<SearchResultItem>> {
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1",
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Internal(format!("DuckDuckGo API request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::Internal(format!(
            "DuckDuckGo API returned status {}",
            response.status()
        )));
    }

    let json: DDGResponse = response
        .json()
        .await
        .map_err(|e| Error::Internal(format!("Failed to parse DuckDuckGo response: {}", e)))?;

    let mut results = Vec::new();

    // Extract abstract (main summary)
    if !json.abstract_text.is_empty() {
        results.push(SearchResultItem {
            title: json.heading.clone().unwrap_or_else(|| query.to_string()),
            url: json.abstract_url.clone().unwrap_or_default(),
            snippet: json.abstract_text.clone(),
            published_date: None,
        });
    }

    // Extract definition if available
    if !json.definition.is_empty() {
        results.push(SearchResultItem {
            title: format!("Definition: {}", query),
            url: json.definition_url.clone().unwrap_or_default(),
            snippet: json.definition.clone(),
            published_date: None,
        });
    }

    // Extract related topics
    for topic in json.related_topics {
        if let Some(ref text) = topic.text {
            if !text.is_empty() {
                results.push(SearchResultItem {
                    title: topic.first_url
                        .as_ref()
                        .map(|u| extract_title_from_url(u))
                        .unwrap_or_else(|| "Related Topic".to_string()),
                    url: topic.first_url.clone().unwrap_or_default(),
                    snippet: text.clone(),
                    published_date: None,
                });
            }
        }
    }

    tracing::info!(query = %query, results_count = results.len(), "Instant answer API completed");
    Ok(results)
}

/// Extract a title from a URL (uses the last path segment)
fn extract_title_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|segments| segments.last().map(|s| s.to_string()))
        })
        .map(|s| s.replace('_', " "))
        .unwrap_or_else(|| "Topic".to_string())
}

/// URL encoding/decoding utilities
mod urlencoding {
    use url::form_urlencoded;

    pub fn encode(s: &str) -> String {
        form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}

/// DuckDuckGo Instant Answer API response
#[derive(Debug, serde::Deserialize)]
struct DDGResponse {
    #[serde(rename = "AbstractText", default)]
    abstract_text: String,
    #[serde(rename = "AbstractURL", default)]
    abstract_url: Option<String>,
    #[serde(rename = "Heading", default)]
    heading: Option<String>,
    #[serde(rename = "Definition", default)]
    definition: String,
    #[serde(rename = "DefinitionURL", default)]
    definition_url: Option<String>,
    #[serde(rename = "RelatedTopics", default)]
    related_topics: Vec<DDGTopic>,
}

#[derive(Debug, serde::Deserialize)]
struct DDGTopic {
    #[serde(rename = "Text", default)]
    text: Option<String>,
    #[serde(rename = "FirstURL", default)]
    first_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("github"), "github");
        assert_eq!(urlencoding::encode("rust programming"), "rust+programming");
    }

    #[tokio::test]
    async fn test_search_popular_topic() {
        // "rust programming language" has knowledge graph entry
        let results = search_with_fallbacks("rust programming language").await.unwrap();
        println!("Results for 'rust programming language': {}", results.len());
        for r in &results {
            println!("  - {} -> {}", r.title, r.url);
        }
        assert!(!results.is_empty(), "Should return results for popular topic");
    }

    #[tokio::test]
    async fn test_search_with_vqd_returns_results() {
        // This test verifies the VQD flow works end-to-end
        // If d.js is blocked, it should fallback to instant answer API
        let results = search_with_fallbacks("python").await.unwrap();
        println!("Results for 'python': {}", results.len());
        for r in &results {
            println!("  - {} -> {}", r.title, r.url);
        }
        assert!(!results.is_empty(), "Should return results for 'python'");
    }

    #[tokio::test]
    async fn test_search_technical_query() {
        // Technical queries like "async await" may not have knowledge graph entries
        // This tests that we can still get results from actual web search
        let results = search_with_fallbacks("async await javascript").await.unwrap();
        println!("Results for 'async await javascript': {}", results.len());
        for r in &results {
            println!("  - {} -> {}", r.title, r.url);
        }
        // This test documents the current limitation:
        // - If d.js is blocked by anti-bot AND no knowledge graph entry exists
        // - Results will be empty
        // TODO: Implement alternative search endpoint when this fails
        if results.is_empty() {
            println!("WARNING: No results for technical query - d.js may be blocked by anti-bot");
        }
    }

    #[tokio::test]
    async fn test_vqd_extraction() {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap();

        let vqd = get_vqd(&client, "test query").await;
        println!("VQD extraction result: {:?}", vqd);
        assert!(vqd.is_ok(), "Should be able to extract VQD token");
        assert!(!vqd.unwrap().is_empty(), "VQD token should not be empty");
    }

    #[tokio::test]
    async fn test_lite_endpoint() {
        // This test checks what the lite endpoint returns
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap();

        let url = format!(
            "https://lite.duckduckgo.com/lite/?q={}",
            urlencoding::encode("async await javascript")
        );

        let response = client
            .get(&url)
            .header("Accept", "text/html")
            .send()
            .await
            .unwrap();

        println!("Lite endpoint status: {}", response.status());

        let body = response.text().await.unwrap();
        println!("Lite response length: {}", body.len());
        println!("Lite response preview:\n{}", &body[..body.len().min(2000)]);

        let results = parse_lite_html_response(&body).unwrap();
        println!("Parsed lite results: {}", results.len());
        for r in &results {
            println!("  - {} -> {}", r.title, r.url);
        }
    }

    #[tokio::test]
    async fn test_d_js_endpoint_response_format() {
        // This test checks what the d.js endpoint actually returns
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap();

        let vqd = get_vqd(&client, "rust programming").await.unwrap();
        println!("Got VQD: {}", vqd);

        let encoded_query = urlencoding::encode("rust programming");
        let url = format!(
            "https://links.duckduckgo.com/d.js?q={}&vqd={}&kl=wt-wt",
            encoded_query, vqd
        );

        let response = client
            .get(&url)
            .header("Accept", "application/javascript, */*")
            .send()
            .await
            .unwrap();

        let body = response.text().await.unwrap();
        println!("d.js response (first 500 chars):");
        println!("{}", &body[..body.len().min(500)]);

        // Check if response contains anti-bot JavaScript
        let is_antibot = body.contains("let jsa =") && body.contains("DDG.deep.initialize");
        println!("Is anti-bot response: {}", is_antibot);

        // If it's anti-bot, parsing should return empty
        let results = parse_ddg_js_response(&body).unwrap();
        println!("Parsed results: {}", results.len());
    }
}
