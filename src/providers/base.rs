//! Base LLM provider interface.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tool call request from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRequest>,
    #[serde(default = "default_finish_reason")]
    pub finish_reason: String,
    #[serde(default)]
    pub usage: HashMap<String, i64>,
    pub reasoning_content: Option<String>, // Kimi, DeepSeek-R1 etc.
}

fn default_finish_reason() -> String {
    "stop".to_string()
}

impl LLMResponse {
    /// Check if response contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Abstract base trait for LLM providers.
///
/// Implementations should handle the specifics of each provider's API
/// while maintaining a consistent interface.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat completion request.
    ///
    /// # Arguments
    /// * `messages` - List of message dicts with 'role' and 'content'.
    /// * `tools` - Optional list of tool definitions.
    /// * `model` - Model identifier (provider-specific).
    /// * `max_tokens` - Maximum tokens in response.
    /// * `temperature` - Sampling temperature.
    ///
    /// # Returns
    /// LLMResponse with content and/or tool calls.
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f32,
    ) -> anyhow::Result<LLMResponse>;

    /// Get the default model for this provider.
    fn get_default_model(&self) -> &str;
}
