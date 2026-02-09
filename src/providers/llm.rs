//! LLM provider implementation using direct HTTP API calls.

use super::base::{LLMProvider, LLMResponse, ToolCallRequest};
use super::registry::{find_by_model, find_gateway, ProviderSpec};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// OpenAI-compatible API request format.
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    max_tokens: i32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// OpenAI-compatible API response format.
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: Function,
}

#[derive(Debug, Deserialize)]
struct Function {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize, Default)]
struct Usage {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}

/// HTTP-based LLM provider for multi-provider support.
///
/// Supports OpenRouter, Anthropic, OpenAI, Gemini, and many other providers through
/// OpenAI-compatible HTTP APIs. Provider-specific logic is driven by the registry.
pub struct HttpLLMProvider {
    client: Client,
    api_key: Option<String>,
    api_base: Option<String>,
    default_model: String,
    extra_headers: HashMap<String, String>,
    gateway: Option<ProviderSpec>,
}

impl HttpLLMProvider {
    /// Create a new HTTP LLM provider.
    pub fn new(
        api_key: Option<String>,
        api_base: Option<String>,
        default_model: String,
        extra_headers: HashMap<String, String>,
        provider_name: Option<String>,
    ) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        // Detect gateway / local deployment
        let gateway = find_gateway(
            provider_name.as_deref(),
            api_key.as_deref(),
            api_base.as_deref(),
        );

        Self {
            client,
            api_key,
            api_base,
            default_model,
            extra_headers,
            gateway,
        }
    }

    /// Resolve model name by applying provider/gateway prefixes.
    fn resolve_model(&self, model: &str) -> String {
        if let Some(gateway) = &self.gateway {
            // Gateway mode: apply gateway prefix, skip provider-specific prefixes
            let mut resolved = model.to_string();
            if gateway.strip_model_prefix {
                resolved = resolved.split('/').last().unwrap_or(&resolved).to_string();
            }
            if !gateway.litellm_prefix.is_empty()
                && !resolved.starts_with(&format!("{}/", gateway.litellm_prefix))
            {
                resolved = format!("{}/{}", gateway.litellm_prefix, resolved);
            }
            return resolved;
        }

        // Standard mode: auto-prefix for known providers
        if let Some(spec) = find_by_model(model) {
            if !spec.litellm_prefix.is_empty() {
                let has_skip_prefix = spec
                    .skip_prefixes
                    .iter()
                    .any(|prefix| model.starts_with(prefix));
                if !has_skip_prefix {
                    return format!("{}/{}", spec.litellm_prefix, model);
                }
            }
        }

        model.to_string()
    }

    /// Apply model-specific parameter overrides from the registry.
    fn apply_model_overrides(&self, model: &str, kwargs: &mut HashMap<String, Value>) {
        let model_lower = model.to_lowercase();
        if let Some(spec) = find_by_model(model) {
            for (pattern, overrides) in &spec.model_overrides {
                if model_lower.contains(pattern) {
                    for (key, value) in overrides {
                        kwargs.insert(key.clone(), value.clone());
                    }
                    return;
                }
            }
        }
    }

    /// Get the API endpoint URL.
    fn get_api_url(&self, model: &str) -> Result<String> {
        // Use custom api_base if provided
        if let Some(ref base) = self.api_base {
            return Ok(format!("{}/chat/completions", base.trim_end_matches('/')));
        }

        // Try to find provider spec to get default_api_base
        if let Some(gateway) = &self.gateway {
            if !gateway.default_api_base.is_empty() {
                return Ok(format!(
                    "{}/chat/completions",
                    gateway.default_api_base.trim_end_matches('/')
                ));
            }
        }

        if let Some(spec) = find_by_model(model) {
            if !spec.default_api_base.is_empty() {
                return Ok(format!(
                    "{}/chat/completions",
                    spec.default_api_base.trim_end_matches('/')
                ));
            }
        }

        // Default to OpenAI
        Ok("https://api.openai.com/v1/chat/completions".to_string())
    }

    /// Parse the HTTP response into our standard format.
    fn parse_response(&self, response: ChatResponse) -> Result<LLMResponse> {
        let choice = response
            .choices
            .first()
            .context("No choices in response")?;

        let mut tool_calls = Vec::new();
        if let Some(tcs) = &choice.message.tool_calls {
            for tc in tcs {
                let args: HashMap<String, Value> =
                    serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| {
                        let mut map = HashMap::new();
                        map.insert("raw".to_string(), json!(tc.function.arguments));
                        map
                    });

                tool_calls.push(ToolCallRequest {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: args,
                });
            }
        }

        let mut usage = HashMap::new();
        usage.insert("prompt_tokens".to_string(), response.usage.prompt_tokens);
        usage.insert(
            "completion_tokens".to_string(),
            response.usage.completion_tokens,
        );
        usage.insert("total_tokens".to_string(), response.usage.total_tokens);

        Ok(LLMResponse {
            content: choice.message.content.clone(),
            tool_calls,
            finish_reason: choice.finish_reason.clone().unwrap_or_else(|| "stop".to_string()),
            usage,
            reasoning_content: choice.message.reasoning_content.clone(),
        })
    }
}

#[async_trait]
impl LLMProvider for HttpLLMProvider {
    async fn chat(
        &self,
        messages: Vec<Value>,
        tools: Option<Vec<Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f32,
    ) -> Result<LLMResponse> {
        let model = model.unwrap_or_else(|| self.default_model.clone());
        let resolved_model = self.resolve_model(&model);

        // Build parameter map for overrides
        let mut param_overrides = HashMap::new();
        self.apply_model_overrides(&model, &mut param_overrides);

        // Apply overrides to temperature if present
        let final_temperature = param_overrides
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(temperature);

        let request = ChatRequest {
            model: resolved_model,
            messages,
            tools: tools.clone(),
            tool_choice: if tools.is_some() {
                Some("auto".to_string())
            } else {
                None
            },
            max_tokens,
            temperature: final_temperature,
            stream: Some(false),
        };

        let url = self.get_api_url(&model)?;

        // Build request with headers
        let mut req = self.client.post(&url).json(&request);

        // Add authorization header
        if let Some(ref api_key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        // Add extra headers
        for (key, value) in &self.extra_headers {
            req = req.header(key, value);
        }

        // Send request
        let response = req
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "LLM API error ({}): {}",
                status,
                error_text
            ));
        }

        // Parse response
        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse LLM API response")?;

        self.parse_response(chat_response)
    }

    fn get_default_model(&self) -> &str {
        &self.default_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_model_with_prefix() {
        let provider = HttpLLMProvider::new(
            Some("test-key".to_string()),
            None,
            "gpt-4".to_string(),
            HashMap::new(),
            None,
        );

        // DeepSeek models should get prefix
        let resolved = provider.resolve_model("deepseek-chat");
        assert_eq!(resolved, "deepseek/deepseek-chat");

        // Already prefixed models should not get double prefix
        let resolved = provider.resolve_model("deepseek/deepseek-chat");
        assert_eq!(resolved, "deepseek/deepseek-chat");
    }

    #[test]
    fn test_resolve_model_without_prefix() {
        let provider = HttpLLMProvider::new(
            Some("test-key".to_string()),
            None,
            "gpt-4".to_string(),
            HashMap::new(),
            None,
        );

        // OpenAI models don't need prefix
        let resolved = provider.resolve_model("gpt-4");
        assert_eq!(resolved, "gpt-4");

        // Anthropic models don't need prefix
        let resolved = provider.resolve_model("claude-3-opus");
        assert_eq!(resolved, "claude-3-opus");
    }

    #[test]
    fn test_gateway_model_resolution() {
        let provider = HttpLLMProvider::new(
            Some("sk-or-test".to_string()),
            Some("https://openrouter.ai/api/v1".to_string()),
            "gpt-4".to_string(),
            HashMap::new(),
            None,
        );

        // Gateway should add openrouter prefix
        let resolved = provider.resolve_model("claude-3-opus");
        assert_eq!(resolved, "openrouter/claude-3-opus");
    }
}
