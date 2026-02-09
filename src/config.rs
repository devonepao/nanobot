//! Configuration schema and loading utilities for nanobot.
//!
//! This module provides the complete configuration structure for the nanobot
//! framework, including channels, providers, agents, and tools configuration.
//! Configuration is loaded from JSON files with camelCase field names and
//! converted to snake_case for internal use.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// =============================================================================
// Channel Configurations
// =============================================================================

/// WhatsApp channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_whatsapp_bridge_url")]
    pub bridge_url: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bridge_url: "ws://localhost:3001".to_string(),
            allow_from: Vec::new(),
        }
    }
}

fn default_whatsapp_bridge_url() -> String {
    "ws://localhost:3001".to_string()
}

/// Telegram channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
    #[serde(default)]
    pub proxy: Option<String>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allow_from: Vec::new(),
            proxy: None,
        }
    }
}

/// Feishu/Lark channel configuration using WebSocket long connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_secret: String,
    #[serde(default)]
    pub encrypt_key: String,
    #[serde(default)]
    pub verification_token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: String::new(),
            app_secret: String::new(),
            encrypt_key: String::new(),
            verification_token: String::new(),
            allow_from: Vec::new(),
        }
    }
}

/// DingTalk channel configuration using Stream mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DingTalkConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for DingTalkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            client_id: String::new(),
            client_secret: String::new(),
            allow_from: Vec::new(),
        }
    }
}

/// Discord channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
    #[serde(default = "default_discord_gateway_url")]
    pub gateway_url: String,
    #[serde(default = "default_discord_intents")]
    pub intents: i32,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allow_from: Vec::new(),
            gateway_url: default_discord_gateway_url(),
            intents: default_discord_intents(),
        }
    }
}

fn default_discord_gateway_url() -> String {
    "wss://gateway.discord.gg/?v=10&encoding=json".to_string()
}

fn default_discord_intents() -> i32 {
    37377 // GUILDS + GUILD_MESSAGES + DIRECT_MESSAGES + MESSAGE_CONTENT
}

/// Email channel configuration (IMAP inbound + SMTP outbound).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub consent_granted: bool,
    
    // IMAP (receive)
    #[serde(default)]
    pub imap_host: String,
    #[serde(default = "default_imap_port")]
    pub imap_port: u16,
    #[serde(default)]
    pub imap_username: String,
    #[serde(default)]
    pub imap_password: String,
    #[serde(default = "default_imap_mailbox")]
    pub imap_mailbox: String,
    #[serde(default = "default_true")]
    pub imap_use_ssl: bool,
    
    // SMTP (send)
    #[serde(default)]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default)]
    pub smtp_username: String,
    #[serde(default)]
    pub smtp_password: String,
    #[serde(default = "default_true")]
    pub smtp_use_tls: bool,
    #[serde(default)]
    pub smtp_use_ssl: bool,
    #[serde(default)]
    pub from_address: String,
    
    // Behavior
    #[serde(default = "default_true")]
    pub auto_reply_enabled: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u32,
    #[serde(default = "default_true")]
    pub mark_seen: bool,
    #[serde(default = "default_max_body_chars")]
    pub max_body_chars: usize,
    #[serde(default = "default_subject_prefix")]
    pub subject_prefix: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            consent_granted: false,
            imap_host: String::new(),
            imap_port: default_imap_port(),
            imap_username: String::new(),
            imap_password: String::new(),
            imap_mailbox: default_imap_mailbox(),
            imap_use_ssl: true,
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_use_tls: true,
            smtp_use_ssl: false,
            from_address: String::new(),
            auto_reply_enabled: true,
            poll_interval_seconds: default_poll_interval(),
            mark_seen: true,
            max_body_chars: default_max_body_chars(),
            subject_prefix: default_subject_prefix(),
            allow_from: Vec::new(),
        }
    }
}

fn default_imap_port() -> u16 { 993 }
fn default_smtp_port() -> u16 { 587 }
fn default_imap_mailbox() -> String { "INBOX".to_string() }
fn default_poll_interval() -> u32 { 30 }
fn default_max_body_chars() -> usize { 12000 }
fn default_subject_prefix() -> String { "Re: ".to_string() }
fn default_true() -> bool { true }

/// Slack DM policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackDMConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_slack_policy")]
    pub policy: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for SlackDMConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy: "open".to_string(),
            allow_from: Vec::new(),
        }
    }
}

fn default_slack_policy() -> String {
    "open".to_string()
}

/// Slack channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_slack_mode")]
    pub mode: String,
    #[serde(default = "default_slack_webhook_path")]
    pub webhook_path: String,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub app_token: String,
    #[serde(default = "default_true")]
    pub user_token_read_only: bool,
    #[serde(default = "default_slack_group_policy")]
    pub group_policy: String,
    #[serde(default)]
    pub group_allow_from: Vec<String>,
    #[serde(default)]
    pub dm: SlackDMConfig,
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "socket".to_string(),
            webhook_path: "/slack/events".to_string(),
            bot_token: String::new(),
            app_token: String::new(),
            user_token_read_only: true,
            group_policy: "mention".to_string(),
            group_allow_from: Vec::new(),
            dm: SlackDMConfig::default(),
        }
    }
}

fn default_slack_mode() -> String { "socket".to_string() }
fn default_slack_webhook_path() -> String { "/slack/events".to_string() }
fn default_slack_group_policy() -> String { "mention".to_string() }

/// QQ channel configuration using botpy SDK.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QQConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub secret: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for QQConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: String::new(),
            secret: String::new(),
            allow_from: Vec::new(),
        }
    }
}

/// Configuration for all chat channels.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChannelsConfig {
    #[serde(default)]
    pub whatsapp: WhatsAppConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub feishu: FeishuConfig,
    #[serde(default)]
    pub dingtalk: DingTalkConfig,
    #[serde(default)]
    pub email: EmailConfig,
    #[serde(default)]
    pub slack: SlackConfig,
    #[serde(default)]
    pub qq: QQConfig,
}

// =============================================================================
// Agent Configuration
// =============================================================================

/// Default agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefaults {
    #[serde(default = "default_workspace")]
    pub workspace: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: i32,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            workspace: default_workspace(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            max_tool_iterations: default_max_tool_iterations(),
        }
    }
}

fn default_workspace() -> String { "~/.nanobot/workspace".to_string() }
fn default_model() -> String { "anthropic/claude-opus-4-5".to_string() }
fn default_max_tokens() -> i32 { 8192 }
fn default_temperature() -> f32 { 0.7 }
fn default_max_tool_iterations() -> i32 { 20 }

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentsConfig {
    #[serde(default)]
    pub defaults: AgentDefaults,
}

// =============================================================================
// Provider Configuration
// =============================================================================

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(default)]
    pub extra_headers: Option<HashMap<String, String>>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base: None,
            extra_headers: None,
        }
    }
}

/// Configuration for all LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProvidersConfig {
    #[serde(default)]
    pub anthropic: ProviderConfig,
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub openrouter: ProviderConfig,
    #[serde(default)]
    pub deepseek: ProviderConfig,
    #[serde(default)]
    pub groq: ProviderConfig,
    #[serde(default)]
    pub zhipu: ProviderConfig,
    #[serde(default)]
    pub dashscope: ProviderConfig,
    #[serde(default)]
    pub vllm: ProviderConfig,
    #[serde(default)]
    pub gemini: ProviderConfig,
    #[serde(default)]
    pub moonshot: ProviderConfig,
    #[serde(default)]
    pub aihubmix: ProviderConfig,
}

// =============================================================================
// Gateway Configuration
// =============================================================================

/// Gateway/server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    #[serde(default = "default_gateway_host")]
    pub host: String,
    #[serde(default = "default_gateway_port")]
    pub port: u16,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: default_gateway_host(),
            port: default_gateway_port(),
        }
    }
}

fn default_gateway_host() -> String { "0.0.0.0".to_string() }
fn default_gateway_port() -> u16 { 18790 }

// =============================================================================
// Tools Configuration
// =============================================================================

/// Web search tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_max_results")]
    pub max_results: i32,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            max_results: default_max_results(),
        }
    }
}

fn default_max_results() -> i32 { 5 }

/// Web tools configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WebToolsConfig {
    #[serde(default)]
    pub search: WebSearchConfig,
}

/// Shell exec tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecToolConfig {
    #[serde(default = "default_exec_timeout")]
    pub timeout: i32,
}

impl Default for ExecToolConfig {
    fn default() -> Self {
        Self {
            timeout: default_exec_timeout(),
        }
    }
}

fn default_exec_timeout() -> i32 { 60 }

/// Tools configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsConfig {
    #[serde(default)]
    pub web: WebToolsConfig,
    #[serde(default)]
    pub exec: ExecToolConfig,
    #[serde(default)]
    pub restrict_to_workspace: bool,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            web: WebToolsConfig::default(),
            exec: ExecToolConfig::default(),
            restrict_to_workspace: false,
        }
    }
}

// =============================================================================
// Root Configuration
// =============================================================================

/// Root configuration for nanobot.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub channels: ChannelsConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
}

// =============================================================================
// Provider Specification (from registry.py)
// =============================================================================

/// Provider specification metadata.
#[derive(Debug, Clone)]
pub struct ProviderSpec {
    pub name: &'static str,
    pub keywords: &'static [&'static str],
    pub is_gateway: bool,
    pub is_local: bool,
    pub detect_by_key_prefix: &'static str,
    pub detect_by_base_keyword: &'static str,
    pub default_api_base: &'static str,
}

impl ProviderSpec {
    pub const fn new(
        name: &'static str,
        keywords: &'static [&'static str],
        is_gateway: bool,
        is_local: bool,
        detect_by_key_prefix: &'static str,
        detect_by_base_keyword: &'static str,
        default_api_base: &'static str,
    ) -> Self {
        Self {
            name,
            keywords,
            is_gateway,
            is_local,
            detect_by_key_prefix,
            detect_by_base_keyword,
            default_api_base,
        }
    }
}

/// Provider registry - order matters for matching priority.
pub static PROVIDERS: &[ProviderSpec] = &[
    // Gateways first
    ProviderSpec::new(
        "openrouter",
        &["openrouter"],
        true,
        false,
        "sk-or-",
        "openrouter",
        "https://openrouter.ai/api/v1",
    ),
    ProviderSpec::new(
        "aihubmix",
        &["aihubmix"],
        true,
        false,
        "",
        "aihubmix",
        "https://aihubmix.com/v1",
    ),
    // Standard providers
    ProviderSpec::new("anthropic", &["anthropic", "claude"], false, false, "", "", ""),
    ProviderSpec::new("openai", &["openai", "gpt"], false, false, "", "", ""),
    ProviderSpec::new("deepseek", &["deepseek"], false, false, "", "", ""),
    ProviderSpec::new("gemini", &["gemini"], false, false, "", "", ""),
    ProviderSpec::new("zhipu", &["zhipu", "glm", "zai"], false, false, "", "", ""),
    ProviderSpec::new("dashscope", &["qwen", "dashscope"], false, false, "", "", ""),
    ProviderSpec::new(
        "moonshot",
        &["moonshot", "kimi"],
        false,
        false,
        "",
        "",
        "https://api.moonshot.ai/v1",
    ),
    // Local deployment
    ProviderSpec::new("vllm", &["vllm"], false, true, "", "", ""),
    // Auxiliary
    ProviderSpec::new("groq", &["groq"], false, false, "", "", ""),
];

// =============================================================================
// Implementation
// =============================================================================

impl Config {
    /// Get expanded workspace path.
    pub fn workspace_path(&self) -> PathBuf {
        let path = &self.agents.defaults.workspace;
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path[2..]);
            }
        }
        PathBuf::from(path)
    }

    /// Match provider config and its registry name. Returns (config, spec_name).
    fn match_provider(&self, model: Option<&str>) -> (Option<&ProviderConfig>, Option<&'static str>) {
        let model_str = model.unwrap_or(&self.agents.defaults.model);
        let model_lower = model_str.to_lowercase();

        // Match by keyword (order follows PROVIDERS registry)
        for spec in PROVIDERS {
            if let Some(provider) = self.get_provider_by_name(spec.name) {
                if spec.keywords.iter().any(|kw| model_lower.contains(kw)) 
                    && !provider.api_key.is_empty() {
                    return (Some(provider), Some(spec.name));
                }
            }
        }

        // Fallback: gateways first, then others (follows registry order)
        for spec in PROVIDERS {
            if let Some(provider) = self.get_provider_by_name(spec.name) {
                if !provider.api_key.is_empty() {
                    return (Some(provider), Some(spec.name));
                }
            }
        }

        (None, None)
    }

    /// Get provider config by name.
    pub fn get_provider_by_name(&self, name: &str) -> Option<&ProviderConfig> {
        match name {
            "anthropic" => Some(&self.providers.anthropic),
            "openai" => Some(&self.providers.openai),
            "openrouter" => Some(&self.providers.openrouter),
            "deepseek" => Some(&self.providers.deepseek),
            "groq" => Some(&self.providers.groq),
            "zhipu" => Some(&self.providers.zhipu),
            "dashscope" => Some(&self.providers.dashscope),
            "vllm" => Some(&self.providers.vllm),
            "gemini" => Some(&self.providers.gemini),
            "moonshot" => Some(&self.providers.moonshot),
            "aihubmix" => Some(&self.providers.aihubmix),
            _ => None,
        }
    }

    /// Get matched provider config (api_key, api_base, extra_headers).
    pub fn get_provider(&self, model: Option<&str>) -> Option<&ProviderConfig> {
        self.match_provider(model).0
    }

    /// Get the registry name of the matched provider.
    pub fn get_provider_name(&self, model: Option<&str>) -> Option<&'static str> {
        self.match_provider(model).1
    }

    /// Get API key for the given model. Falls back to first available key.
    pub fn get_api_key(&self, model: Option<&str>) -> Option<&str> {
        self.get_provider(model).map(|p| p.api_key.as_str())
    }

    /// Get API base URL for the given model. Applies default URLs for known gateways.
    pub fn get_api_base(&self, model: Option<&str>) -> Option<String> {
        let (provider, name) = self.match_provider(model);
        
        if let Some(p) = provider {
            if let Some(ref base) = p.api_base {
                return Some(base.clone());
            }
        }

        // Only gateways get a default api_base here
        if let Some(provider_name) = name {
            if let Some(spec) = PROVIDERS.iter().find(|s| s.name == provider_name) {
                if spec.is_gateway && !spec.default_api_base.is_empty() {
                    return Some(spec.default_api_base.to_string());
                }
            }
        }

        None
    }

    /// Load configuration from file.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(get_default_config_path);

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .context(format!("Failed to read config file: {:?}", config_path))?;
            
            let mut config: Config = serde_json::from_str(&content)
                .context("Failed to parse config JSON")?;
            
            // Apply migrations
            migrate_config(&mut config);
            
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save configuration to file.
    pub fn save(&self, path: Option<&Path>) -> Result<()> {
        let config_path = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(get_default_config_path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(&config_path, json)
            .context(format!("Failed to write config file: {:?}", config_path))?;

        Ok(())
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get the default configuration file path.
pub fn get_default_config_path() -> PathBuf {
    dirs::home_dir()
        .map(|home| home.join(".nanobot").join("config.json"))
        .unwrap_or_else(|| PathBuf::from(".nanobot/config.json"))
}

/// Get the nanobot data directory.
pub fn get_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|home| home.join(".nanobot"))
        .unwrap_or_else(|| PathBuf::from(".nanobot"))
}

/// Apply configuration migrations.
fn migrate_config(_config: &mut Config) {
    // Migration logic can be added here if needed
    // For example, moving deprecated fields to new locations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.agents.defaults.model, "anthropic/claude-opus-4-5");
        assert_eq!(config.agents.defaults.max_tokens, 8192);
        assert_eq!(config.agents.defaults.temperature, 0.7);
        assert_eq!(config.gateway.port, 18790);
    }

    #[test]
    fn test_workspace_path_expansion() {
        let config = Config::default();
        let path = config.workspace_path();
        // Should expand ~/ to home directory
        assert!(!path.to_string_lossy().starts_with("~"));
    }

    #[test]
    fn test_provider_matching() {
        let mut config = Config::default();
        config.providers.anthropic.api_key = "test-key".to_string();
        
        let provider = config.get_provider(Some("claude-3-opus"));
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().api_key, "test-key");
    }

    #[test]
    fn test_provider_name_matching() {
        let mut config = Config::default();
        config.providers.deepseek.api_key = "test-key".to_string();
        
        let name = config.get_provider_name(Some("deepseek-chat"));
        assert_eq!(name, Some("deepseek"));
    }

    #[test]
    fn test_gateway_detection() {
        let mut config = Config::default();
        config.providers.openrouter.api_key = "sk-or-test".to_string();
        
        let provider = config.get_provider(Some("random-model"));
        assert!(provider.is_some());
    }

    #[test]
    fn test_api_base_default() {
        let mut config = Config::default();
        config.providers.openrouter.api_key = "test-key".to_string();
        
        let api_base = config.get_api_base(Some("openrouter/model"));
        assert_eq!(api_base, Some("https://openrouter.ai/api/v1".to_string()));
    }

    #[test]
    fn test_json_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.agents.defaults.model, config.agents.defaults.model);
    }

    #[test]
    fn test_camel_case_deserialization() {
        let json = r#"{
            "agents": {
                "defaults": {
                    "maxTokens": 4096,
                    "maxToolIterations": 10
                }
            }
        }"#;
        
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.agents.defaults.max_tokens, 4096);
        assert_eq!(config.agents.defaults.max_tool_iterations, 10);
    }
}
