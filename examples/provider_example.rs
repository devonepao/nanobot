//! Example of using the providers module.
//!
//! This demonstrates how to use the HttpLLMProvider to make API calls to various LLM providers.
//!
//! Run with:
//! ```bash
//! export OPENAI_API_KEY="your-api-key"
//! cargo run --example provider_example
//! ```

use nanobot::providers::{HttpLLMProvider, LLMProvider};
use serde_json::json;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get API key from environment
    let api_key = env::var("OPENAI_API_KEY").ok();
    
    if api_key.is_none() {
        println!("Warning: OPENAI_API_KEY not set. Set it to test the provider.");
        return Ok(());
    }

    // Create a provider
    let provider = HttpLLMProvider::new(
        api_key,
        None,                           // api_base (optional)
        "gpt-4o-mini".to_string(),      // default model
        HashMap::new(),                  // extra headers
        None,                            // provider name (optional)
    );

    println!("Testing HttpLLMProvider...\n");

    // Example 1: Simple chat without tools
    println!("=== Example 1: Simple Chat ===");
    let messages = vec![
        json!({
            "role": "user",
            "content": "What is the capital of France? Answer in one word."
        }),
    ];

    match provider.chat(messages, None, None, 100, 0.7).await {
        Ok(response) => {
            println!("Response: {:?}", response.content);
            println!("Finish reason: {}", response.finish_reason);
            println!("Usage: {:?}", response.usage);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    println!("\n=== Example 2: Chat with Tools ===");
    let messages = vec![
        json!({
            "role": "user",
            "content": "What's the weather like in San Francisco?"
        }),
    ];

    let tools = vec![
        json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the current weather for a location",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city name"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["celsius", "fahrenheit"],
                            "description": "The temperature unit"
                        }
                    },
                    "required": ["location"]
                }
            }
        }),
    ];

    match provider.chat(messages, Some(tools), None, 500, 0.7).await {
        Ok(response) => {
            if response.has_tool_calls() {
                println!("Tool calls requested:");
                for tool_call in &response.tool_calls {
                    println!("  - {}: {:?}", tool_call.name, tool_call.arguments);
                }
            } else {
                println!("Response: {:?}", response.content);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    println!("\n=== Example 3: Using Different Providers ===");
    
    // DeepSeek example (if you have a key)
    if let Ok(deepseek_key) = env::var("DEEPSEEK_API_KEY") {
        let deepseek_provider = HttpLLMProvider::new(
            Some(deepseek_key),
            None,
            "deepseek-chat".to_string(),
            HashMap::new(),
            None,
        );

        let messages = vec![
            json!({
                "role": "user",
                "content": "Say 'Hello from DeepSeek' in one line."
            }),
        ];

        match deepseek_provider.chat(messages, None, None, 50, 0.7).await {
            Ok(response) => {
                println!("DeepSeek Response: {:?}", response.content);
            }
            Err(e) => {
                println!("DeepSeek Error: {}", e);
            }
        }
    } else {
        println!("DEEPSEEK_API_KEY not set, skipping DeepSeek example");
    }

    // OpenRouter gateway example (if you have a key)
    if let Ok(openrouter_key) = env::var("OPENROUTER_API_KEY") {
        let openrouter_provider = HttpLLMProvider::new(
            Some(openrouter_key),
            Some("https://openrouter.ai/api/v1".to_string()),
            "anthropic/claude-3-haiku".to_string(),
            HashMap::new(),
            Some("openrouter".to_string()),
        );

        let messages = vec![
            json!({
                "role": "user",
                "content": "Say 'Hello from Claude via OpenRouter' in one line."
            }),
        ];

        match openrouter_provider.chat(messages, None, None, 50, 0.7).await {
            Ok(response) => {
                println!("OpenRouter Response: {:?}", response.content);
            }
            Err(e) => {
                println!("OpenRouter Error: {}", e);
            }
        }
    } else {
        println!("OPENROUTER_API_KEY not set, skipping OpenRouter example");
    }

    Ok(())
}
