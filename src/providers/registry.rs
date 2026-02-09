//! Provider Registry — single source of truth for LLM provider metadata.
//!
//! Adding a new provider:
//!   1. Add a ProviderSpec to PROVIDERS below.
//!   2. Add a field to ProvidersConfig in config/schema.
//!   Done. Env vars, prefixing, config matching, status display all derive from here.
//!
//! Order matters — it controls match priority and fallback. Gateways first.
//! Every entry writes out all fields so you can copy-paste as a template.

use serde::Serialize;
use std::collections::HashMap;

/// One LLM provider's metadata. See PROVIDERS below for real examples.
///
/// Placeholders in env_extras values:
///   {api_key}  — the user's API key
///   {api_base} — api_base from config, or this spec's default_api_base
#[derive(Debug, Clone, Serialize)]
pub struct ProviderSpec {
    // identity
    pub name: String,                       // config field name, e.g. "dashscope"
    pub keywords: Vec<String>,              // model-name keywords for matching (lowercase)
    pub env_key: String,                    // env var, e.g. "DASHSCOPE_API_KEY"
    pub display_name: String,               // shown in `nanobot status`

    // model prefixing
    pub litellm_prefix: String,             // "dashscope" → model becomes "dashscope/{model}"
    pub skip_prefixes: Vec<String>,         // don't prefix if model already starts with these

    // extra env vars
    pub env_extras: Vec<(String, String)>,  // e.g. [("ZHIPUAI_API_KEY", "{api_key}")]

    // gateway / local detection
    pub is_gateway: bool,                   // routes any model (OpenRouter, AiHubMix)
    pub is_local: bool,                     // local deployment (vLLM, Ollama)
    pub detect_by_key_prefix: String,       // match api_key prefix, e.g. "sk-or-"
    pub detect_by_base_keyword: String,     // match substring in api_base URL
    pub default_api_base: String,           // fallback base URL

    // gateway behavior
    pub strip_model_prefix: bool,           // strip "provider/" before re-prefixing

    // per-model param overrides
    pub model_overrides: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl ProviderSpec {
    pub fn label(&self) -> &str {
        if self.display_name.is_empty() {
            &self.name
        } else {
            &self.display_name
        }
    }
}

/// Get the registry of all providers.
pub fn get_providers() -> Vec<ProviderSpec> {
    vec![
        // === Gateways (detected by api_key / api_base, not model name) =========
        // Gateways can route any model, so they win in fallback.

        // OpenRouter: global gateway, keys start with "sk-or-"
        ProviderSpec {
            name: "openrouter".to_string(),
            keywords: vec!["openrouter".to_string()],
            env_key: "OPENROUTER_API_KEY".to_string(),
            display_name: "OpenRouter".to_string(),
            litellm_prefix: "openrouter".to_string(),
            skip_prefixes: vec![],
            env_extras: vec![],
            is_gateway: true,
            is_local: false,
            detect_by_key_prefix: "sk-or-".to_string(),
            detect_by_base_keyword: "openrouter".to_string(),
            default_api_base: "https://openrouter.ai/api/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // AiHubMix: global gateway, OpenAI-compatible interface.
        // strip_model_prefix=true: it doesn't understand "anthropic/claude-3",
        // so we strip to bare "claude-3" then re-prefix as "openai/claude-3".
        ProviderSpec {
            name: "aihubmix".to_string(),
            keywords: vec!["aihubmix".to_string()],
            env_key: "OPENAI_API_KEY".to_string(),
            display_name: "AiHubMix".to_string(),
            litellm_prefix: "openai".to_string(),
            skip_prefixes: vec![],
            env_extras: vec![],
            is_gateway: true,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "aihubmix".to_string(),
            default_api_base: "https://aihubmix.com/v1".to_string(),
            strip_model_prefix: true,
            model_overrides: HashMap::new(),
        },

        // === Standard providers (matched by model-name keywords) ===============

        // Anthropic: recognizes "claude-*" natively, no prefix needed.
        ProviderSpec {
            name: "anthropic".to_string(),
            keywords: vec!["anthropic".to_string(), "claude".to_string()],
            env_key: "ANTHROPIC_API_KEY".to_string(),
            display_name: "Anthropic".to_string(),
            litellm_prefix: "".to_string(),
            skip_prefixes: vec![],
            env_extras: vec![],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://api.anthropic.com/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // OpenAI: recognizes "gpt-*" natively, no prefix needed.
        ProviderSpec {
            name: "openai".to_string(),
            keywords: vec!["openai".to_string(), "gpt".to_string()],
            env_key: "OPENAI_API_KEY".to_string(),
            display_name: "OpenAI".to_string(),
            litellm_prefix: "".to_string(),
            skip_prefixes: vec![],
            env_extras: vec![],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://api.openai.com/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // DeepSeek: needs "deepseek/" prefix for routing.
        ProviderSpec {
            name: "deepseek".to_string(),
            keywords: vec!["deepseek".to_string()],
            env_key: "DEEPSEEK_API_KEY".to_string(),
            display_name: "DeepSeek".to_string(),
            litellm_prefix: "deepseek".to_string(),
            skip_prefixes: vec!["deepseek/".to_string()],
            env_extras: vec![],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://api.deepseek.com/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // Gemini: needs "gemini/" prefix.
        ProviderSpec {
            name: "gemini".to_string(),
            keywords: vec!["gemini".to_string()],
            env_key: "GEMINI_API_KEY".to_string(),
            display_name: "Gemini".to_string(),
            litellm_prefix: "gemini".to_string(),
            skip_prefixes: vec!["gemini/".to_string()],
            env_extras: vec![],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://generativelanguage.googleapis.com/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // Zhipu: uses "zai/" prefix.
        // Also mirrors key to ZHIPUAI_API_KEY (some paths check that).
        ProviderSpec {
            name: "zhipu".to_string(),
            keywords: vec!["zhipu".to_string(), "glm".to_string(), "zai".to_string()],
            env_key: "ZAI_API_KEY".to_string(),
            display_name: "Zhipu AI".to_string(),
            litellm_prefix: "zai".to_string(),
            skip_prefixes: vec![
                "zhipu/".to_string(),
                "zai/".to_string(),
                "openrouter/".to_string(),
                "hosted_vllm/".to_string(),
            ],
            env_extras: vec![("ZHIPUAI_API_KEY".to_string(), "{api_key}".to_string())],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // DashScope: Qwen models, needs "dashscope/" prefix.
        ProviderSpec {
            name: "dashscope".to_string(),
            keywords: vec!["qwen".to_string(), "dashscope".to_string()],
            env_key: "DASHSCOPE_API_KEY".to_string(),
            display_name: "DashScope".to_string(),
            litellm_prefix: "dashscope".to_string(),
            skip_prefixes: vec!["dashscope/".to_string(), "openrouter/".to_string()],
            env_extras: vec![],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://dashscope.aliyuncs.com/api/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // Moonshot: Kimi models, needs "moonshot/" prefix.
        // Kimi K2.5 API enforces temperature >= 1.0.
        ProviderSpec {
            name: "moonshot".to_string(),
            keywords: vec!["moonshot".to_string(), "kimi".to_string()],
            env_key: "MOONSHOT_API_KEY".to_string(),
            display_name: "Moonshot".to_string(),
            litellm_prefix: "moonshot".to_string(),
            skip_prefixes: vec!["moonshot/".to_string(), "openrouter/".to_string()],
            env_extras: vec![("MOONSHOT_API_BASE".to_string(), "{api_base}".to_string())],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://api.moonshot.ai/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: {
                let mut m = HashMap::new();
                let mut overrides = HashMap::new();
                overrides.insert("temperature".to_string(), serde_json::json!(1.0));
                m.insert("kimi-k2.5".to_string(), overrides);
                m
            },
        },

        // === Local deployment (matched by config key, NOT by api_base) =========

        // vLLM / any OpenAI-compatible local server.
        ProviderSpec {
            name: "vllm".to_string(),
            keywords: vec!["vllm".to_string()],
            env_key: "HOSTED_VLLM_API_KEY".to_string(),
            display_name: "vLLM/Local".to_string(),
            litellm_prefix: "hosted_vllm".to_string(),
            skip_prefixes: vec![],
            env_extras: vec![],
            is_gateway: false,
            is_local: true,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },

        // === Auxiliary (not a primary LLM provider) ============================

        // Groq: mainly used for Whisper voice transcription, also usable for LLM.
        ProviderSpec {
            name: "groq".to_string(),
            keywords: vec!["groq".to_string()],
            env_key: "GROQ_API_KEY".to_string(),
            display_name: "Groq".to_string(),
            litellm_prefix: "groq".to_string(),
            skip_prefixes: vec!["groq/".to_string()],
            env_extras: vec![],
            is_gateway: false,
            is_local: false,
            detect_by_key_prefix: "".to_string(),
            detect_by_base_keyword: "".to_string(),
            default_api_base: "https://api.groq.com/openai/v1".to_string(),
            strip_model_prefix: false,
            model_overrides: HashMap::new(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Lookup helpers
// ---------------------------------------------------------------------------

/// Match a standard provider by model-name keyword (case-insensitive).
/// Skips gateways/local — those are matched by api_key/api_base instead.
pub fn find_by_model(model: &str) -> Option<ProviderSpec> {
    let model_lower = model.to_lowercase();
    get_providers()
        .into_iter()
        .find(|spec| {
            !spec.is_gateway 
                && !spec.is_local 
                && spec.keywords.iter().any(|kw| model_lower.contains(kw))
        })
}

/// Detect gateway/local provider.
///
/// Priority:
///   1. provider_name — if it maps to a gateway/local spec, use it directly.
///   2. api_key prefix — e.g. "sk-or-" → OpenRouter.
///   3. api_base keyword — e.g. "aihubmix" in URL → AiHubMix.
pub fn find_gateway(
    provider_name: Option<&str>,
    api_key: Option<&str>,
    api_base: Option<&str>,
) -> Option<ProviderSpec> {
    // 1. Direct match by config key
    if let Some(name) = provider_name {
        if let Some(spec) = find_by_name(name) {
            if spec.is_gateway || spec.is_local {
                return Some(spec);
            }
        }
    }

    // 2. Auto-detect by api_key prefix / api_base keyword
    for spec in get_providers() {
        if !spec.detect_by_key_prefix.is_empty() {
            if let Some(key) = api_key {
                if key.starts_with(&spec.detect_by_key_prefix) {
                    return Some(spec);
                }
            }
        }
        if !spec.detect_by_base_keyword.is_empty() {
            if let Some(base) = api_base {
                if base.contains(&spec.detect_by_base_keyword) {
                    return Some(spec);
                }
            }
        }
    }

    None
}

/// Find a provider spec by config field name, e.g. "dashscope".
pub fn find_by_name(name: &str) -> Option<ProviderSpec> {
    get_providers()
        .into_iter()
        .find(|spec| spec.name == name)
}
