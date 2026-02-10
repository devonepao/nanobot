# Providers Module

The providers module provides a unified interface for interacting with various LLM providers through HTTP API calls.

## Architecture

The module consists of three main components:

1. **base.rs** - Defines the `LLMProvider` trait and common types (`LLMResponse`, `ToolCallRequest`)
2. **registry.rs** - Provider registry containing metadata for all supported providers
3. **llm.rs** - `HttpLLMProvider` implementation that makes direct HTTP API calls

## Supported Providers

### Gateways
- **OpenRouter** - Multi-provider gateway (auto-detected by `sk-or-` key prefix)
- **AiHubMix** - OpenAI-compatible gateway

### Standard Providers
- **Anthropic** (Claude models)
- **OpenAI** (GPT models)
- **DeepSeek**
- **Gemini**
- **Zhipu AI** (GLM models)
- **DashScope** (Qwen models)
- **Moonshot** (Kimi models)
- **Groq**

### Local Deployments
- **vLLM** - OpenAI-compatible local server

## Usage

### Basic Chat

```rust
use nanobot::providers::{HttpLLMProvider, LLMProvider};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a provider
    let provider = HttpLLMProvider::new(
        Some("your-api-key".to_string()),
        None,                           // api_base (optional)
        "gpt-4o-mini".to_string(),      // default model
        HashMap::new(),                  // extra headers
        None,                            // provider name (optional)
    );

    // Send a chat request
    let messages = vec![
        json!({
            "role": "user",
            "content": "Hello!"
        }),
    ];

    let response = provider.chat(messages, None, None, 1000, 0.7).await?;
    println!("Response: {:?}", response.content);
    
    Ok(())
}
```

### Chat with Tool Calling

```rust
let tools = vec![
    json!({
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get the current weather",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    }
                },
                "required": ["location"]
            }
        }
    }),
];

let response = provider.chat(messages, Some(tools), None, 1000, 0.7).await?;

if response.has_tool_calls() {
    for tool_call in response.tool_calls {
        println!("Tool: {}", tool_call.name);
        println!("Arguments: {:?}", tool_call.arguments);
    }
}
```

### Using Different Providers

The provider is automatically detected based on:
1. Model name keywords (e.g., "claude" → Anthropic, "gpt" → OpenAI)
2. API key prefix (e.g., "sk-or-" → OpenRouter)
3. API base URL keywords (e.g., "openrouter" → OpenRouter)
4. Explicit provider name

#### DeepSeek Example
```rust
let provider = HttpLLMProvider::new(
    Some(deepseek_key),
    None,
    "deepseek-chat".to_string(),  // Automatically prefixed to "deepseek/deepseek-chat"
    HashMap::new(),
    None,
);
```

#### OpenRouter Gateway Example
```rust
let provider = HttpLLMProvider::new(
    Some(openrouter_key),
    Some("https://openrouter.ai/api/v1".to_string()),
    "anthropic/claude-3-haiku".to_string(),
    HashMap::new(),
    Some("openrouter".to_string()),  // Explicit provider name
);
```

#### Custom API Base Example
```rust
// Use a local vLLM server
let provider = HttpLLMProvider::new(
    Some("dummy-key".to_string()),
    Some("http://localhost:8000/v1".to_string()),
    "meta-llama/Llama-3-8B".to_string(),
    HashMap::new(),
    Some("vllm".to_string()),
);
```

## Provider Registry

The registry in `registry.rs` contains metadata for all providers:

- **Model prefixing** - Automatically adds provider-specific prefixes (e.g., "deepseek/" for DeepSeek)
- **Skip prefixes** - Avoids double-prefixing already-prefixed models
- **API base URLs** - Default API endpoints for each provider
- **Environment variables** - Required env var names for authentication
- **Model overrides** - Provider-specific parameter adjustments (e.g., Kimi K2.5 temperature >= 1.0)
- **Gateway detection** - Auto-detect gateway providers by key/URL patterns

### Adding a New Provider

To add a new provider:

1. Add a `ProviderSpec` to the `get_providers()` function in `registry.rs`
2. Fill in all fields (see existing examples as templates)
3. Done! The provider will automatically work with the HTTP client

## API Format

The module uses OpenAI-compatible HTTP API format:

**Request:**
```json
{
  "model": "gpt-4o-mini",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ],
  "max_tokens": 1000,
  "temperature": 0.7,
  "tools": [...],
  "tool_choice": "auto"
}
```

**Response:**
```json
{
  "choices": [
    {
      "message": {
        "content": "Hello! How can I help you?",
        "tool_calls": [...]
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 20,
    "total_tokens": 30
  }
}
```

## Error Handling

All methods return `anyhow::Result<T>` for flexible error handling:

```rust
match provider.chat(messages, None, None, 1000, 0.7).await {
    Ok(response) => {
        println!("Success: {:?}", response.content);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Testing

Run the tests:

```bash
cargo test --lib providers
```

Run the example:

```bash
export OPENAI_API_KEY="your-api-key"
cargo run --example provider_example
```

## Design Decisions

### Why HTTP API Calls Instead of LiteLLM?

The Rust version uses direct HTTP API calls instead of the Python LiteLLM library for several reasons:

1. **No Rust LiteLLM equivalent** - LiteLLM is Python-only
2. **Better control** - Direct HTTP calls give full control over requests/responses
3. **Fewer dependencies** - Only requires `reqwest` for HTTP
4. **OpenAI compatibility** - Most modern LLM providers use OpenAI-compatible APIs
5. **Transparency** - Clear what's being sent to each provider

### Registry-Driven Design

Provider-specific logic is centralized in the registry rather than scattered across the code:

- **Single source of truth** - All provider metadata in one place
- **Easy to extend** - Add new providers by adding a registry entry
- **No if-else chains** - Logic is data-driven
- **Consistent behavior** - All providers follow the same patterns

## Differences from Python Version

1. **Direct HTTP calls** instead of LiteLLM
2. **Strongly typed** responses with Rust structs
3. **Async/await** with Tokio instead of Python asyncio
4. **Error handling** with `anyhow::Result` instead of Python exceptions
5. **HashMap** for tool arguments instead of Python dicts
6. **No environment variable manipulation** - Rust doesn't need to set env vars for library calls

The API interface remains the same, so code using the providers module can be easily ported from Python to Rust.
