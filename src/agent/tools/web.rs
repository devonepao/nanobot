//! Web tools: web_search and web_fetch.

use super::base::{get_int_param, get_optional_string, get_string_param, Tool};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::{Map, Value};
use std::env;
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7_2) AppleWebKit/537.36";
const MAX_REDIRECTS: usize = 5;

/// Remove HTML tags and decode entities.
fn strip_tags(text: &str) -> String {
    let script_re = Regex::new(r"(?i)<script[\s\S]*?</script>").unwrap();
    let style_re = Regex::new(r"(?i)<style[\s\S]*?</style>").unwrap();
    let tag_re = Regex::new(r"<[^>]+>").unwrap();

    let text = script_re.replace_all(text, "");
    let text = style_re.replace_all(&text, "");
    let text = tag_re.replace_all(&text, "");

    html_escape::decode_html_entities(&text).to_string()
}

/// Normalize whitespace.
fn normalize(text: &str) -> String {
    let space_re = Regex::new(r"[ \t]+").unwrap();
    let newline_re = Regex::new(r"\n{3,}").unwrap();

    let text = space_re.replace_all(text, " ");
    let text = newline_re.replace_all(&text, "\n\n");

    text.trim().to_string()
}

/// Validate URL: must be http(s) with valid domain.
fn validate_url(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url)?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(anyhow!(
            "Only http/https allowed, got '{}'",
            parsed.scheme()
        ));
    }

    if parsed.host_str().is_none() {
        return Err(anyhow!("Missing domain"));
    }

    Ok(())
}

/// Search the web using Brave Search API.
pub struct WebSearchTool {
    api_key: String,
    max_results: i64,
}

impl WebSearchTool {
    pub fn new(api_key: Option<String>, max_results: i64) -> Self {
        let key = api_key.unwrap_or_else(|| env::var("BRAVE_API_KEY").unwrap_or_default());
        Self {
            api_key: key,
            max_results,
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web. Returns titles, URLs, and snippets."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "count": {
                    "type": "integer",
                    "description": "Results (1-10)",
                    "minimum": 1,
                    "maximum": 10
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        if self.api_key.is_empty() {
            return Ok("Error: BRAVE_API_KEY not configured".to_string());
        }

        let query = get_string_param(&params, "query")
            .ok_or_else(|| anyhow!("Missing required parameter: query"))?;
        let count = get_int_param(&params, "count")
            .unwrap_or(self.max_results)
            .max(1)
            .min(10);

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let response = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .query(&[("q", &query), ("count", &count.to_string())])
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &self.api_key)
            .send()
            .await?;

        response.error_for_status_ref()?;
        let json: Value = response.json().await?;

        let results = json
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
            .ok_or_else(|| anyhow!("Unexpected API response format"))?;

        if results.is_empty() {
            return Ok(format!("No results for: {}", query));
        }

        let mut lines = vec![format!("Results for: {}\n", query)];

        for (i, item) in results.iter().take(count as usize).enumerate() {
            let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let desc = item.get("description").and_then(|d| d.as_str());

            lines.push(format!("{}. {}\n   {}", i + 1, title, url));
            if let Some(d) = desc {
                lines.push(format!("   {}", d));
            }
        }

        Ok(lines.join("\n"))
    }
}

/// Fetch and extract content from a URL.
pub struct WebFetchTool {
    max_chars: usize,
}

impl WebFetchTool {
    pub fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }

    fn to_markdown(&self, html: &str) -> String {
        let link_re = Regex::new(r#"(?i)<a\s+[^>]*href=["']([^"']+)["'][^>]*>([\s\S]*?)</a>"#).unwrap();
        let heading_re = Regex::new(r"(?i)<h([1-6])[^>]*>([\s\S]*?)</h\1>").unwrap();
        let li_re = Regex::new(r"(?i)<li[^>]*>([\s\S]*?)</li>").unwrap();
        let block_end_re = Regex::new(r"(?i)</(p|div|section|article)>").unwrap();
        let br_re = Regex::new(r"(?i)<(br|hr)\s*/?>").unwrap();

        let mut text = link_re
            .replace_all(html, |caps: &regex::Captures| {
                format!("[{}]({})", strip_tags(&caps[2]), &caps[1])
            })
            .to_string();

        text = heading_re
            .replace_all(&text, |caps: &regex::Captures| {
                let level: usize = caps[1].parse().unwrap_or(1);
                format!("\n{} {}\n", "#".repeat(level), strip_tags(&caps[2]))
            })
            .to_string();

        text = li_re
            .replace_all(&text, |caps: &regex::Captures| {
                format!("\n- {}", strip_tags(&caps[1]))
            })
            .to_string();

        text = block_end_re.replace_all(&text, "\n\n").to_string();
        text = br_re.replace_all(&text, "\n").to_string();

        normalize(&strip_tags(&text))
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch URL and extract readable content (HTML → markdown/text)."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                },
                "extractMode": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "description": "Extraction mode (default: markdown)"
                },
                "maxChars": {
                    "type": "integer",
                    "minimum": 100,
                    "description": "Maximum characters to return"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let url = get_string_param(&params, "url")
            .ok_or_else(|| anyhow!("Missing required parameter: url"))?;
        let extract_mode = get_optional_string(&params, "extractMode").unwrap_or_else(|| "markdown".to_string());
        let max_chars = get_int_param(&params, "maxChars")
            .map(|c| c as usize)
            .unwrap_or(self.max_chars);

        // Validate URL
        if let Err(e) = validate_url(&url) {
            return Ok(serde_json::json!({
                "error": format!("URL validation failed: {}", e),
                "url": url
            })
            .to_string());
        }

        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()?;

        let response = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(serde_json::json!({
                    "error": e.to_string(),
                    "url": url
                })
                .to_string())
            }
        };

        let final_url = response.url().to_string();
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = match response.text().await {
            Ok(b) => b,
            Err(e) => {
                return Ok(serde_json::json!({
                    "error": e.to_string(),
                    "url": url
                })
                .to_string())
            }
        };

        let (text, extractor) = if content_type.contains("application/json") {
            // JSON response
            (body, "json")
        } else if content_type.contains("text/html")
            || body.trim_start().to_lowercase().starts_with("<!doctype")
            || body.trim_start().to_lowercase().starts_with("<html")
        {
            // HTML response - use basic readability extraction
            let document = Html::parse_document(&body);

            // Try to extract title
            let title = document
                .select(&Selector::parse("title").unwrap())
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            // Extract main content (try common content selectors)
            let content_selectors = vec!["article", "main", ".content", "#content", "body"];
            let mut content_html = String::new();

            for selector_str in content_selectors {
                if let Ok(selector) = Selector::parse(selector_str) {
                    if let Some(element) = document.select(&selector).next() {
                        content_html = element.html();
                        break;
                    }
                }
            }

            if content_html.is_empty() {
                content_html = body;
            }

            let content = if extract_mode == "markdown" {
                self.to_markdown(&content_html)
            } else {
                strip_tags(&content_html)
            };

            let text = if !title.is_empty() {
                format!("# {}\n\n{}", title.trim(), content)
            } else {
                content
            };

            (text, "readability")
        } else {
            // Raw text
            (body, "raw")
        };

        let truncated = text.len() > max_chars;
        let final_text = if truncated {
            text.chars().take(max_chars).collect::<String>()
        } else {
            text
        };

        Ok(serde_json::json!({
            "url": url,
            "finalUrl": final_url,
            "status": status,
            "extractor": extractor,
            "truncated": truncated,
            "length": final_text.len(),
            "text": final_text
        })
        .to_string())
    }
}
