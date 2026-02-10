//! LLM providers module for multi-provider support.
//!
//! This module provides a unified interface for interacting with various LLM providers
//! including OpenAI, Anthropic, DeepSeek, Gemini, and others through HTTP API calls.

pub mod base;
pub mod llm;
pub mod registry;

pub use base::{LLMProvider, LLMResponse, ToolCallRequest};
pub use llm::HttpLLMProvider;
pub use registry::{find_by_model, find_by_name, find_gateway, get_providers, ProviderSpec};
