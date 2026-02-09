//! CLI commands for nanobot.
//!
//! This module implements the command-line interface using clap with derive macros.
//! It provides commands for onboarding, agent interaction, gateway management,
//! status reporting, channel management, and cron job scheduling.

use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const LOGO: &str = "🤖";

// =============================================================================
// CLI Structure
// =============================================================================

#[derive(Parser, Debug)]
#[command(
    name = "nanobot",
    version = VERSION,
    about = "🤖 nanobot - Personal AI Assistant",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize nanobot configuration and workspace
    Onboard,
    
    /// Interact with the agent directly
    Agent(AgentArgs),
    
    /// Start the nanobot gateway
    Gateway(GatewayArgs),
    
    /// Show nanobot status
    Status,
    
    /// Manage channels
    #[command(subcommand)]
    Channels(ChannelsCommands),
    
    /// Manage scheduled tasks
    #[command(subcommand)]
    Cron(CronCommands),
}

// =============================================================================
// Agent Command
// =============================================================================

#[derive(Args, Debug)]
pub struct AgentArgs {
    /// Message to send to the agent
    #[arg(short, long)]
    pub message: Option<String>,
    
    /// Session ID
    #[arg(short, long, default_value = "cli:default")]
    pub session: String,
    
    /// Render assistant output as Markdown
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub markdown: bool,
    
    /// Show nanobot runtime logs during chat
    #[arg(long, default_value = "false")]
    pub logs: bool,
}

// =============================================================================
// Gateway Command
// =============================================================================

#[derive(Args, Debug)]
pub struct GatewayArgs {
    /// Gateway port
    #[arg(short, long, default_value = "18790")]
    pub port: u16,
    
    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

// =============================================================================
// Channels Commands
// =============================================================================

#[derive(Subcommand, Debug)]
pub enum ChannelsCommands {
    /// Show channel status
    Status,
    
    /// Link device via QR code
    Login,
}

// =============================================================================
// Cron Commands
// =============================================================================

#[derive(Subcommand, Debug)]
pub enum CronCommands {
    /// List scheduled jobs
    List(CronListArgs),
    
    /// Add a scheduled job
    Add(CronAddArgs),
    
    /// Remove a scheduled job
    Remove(CronRemoveArgs),
    
    /// Enable or disable a job
    Enable(CronEnableArgs),
    
    /// Manually run a job
    Run(CronRunArgs),
}

#[derive(Args, Debug)]
pub struct CronListArgs {
    /// Include disabled jobs
    #[arg(short, long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct CronAddArgs {
    /// Job name
    #[arg(short, long)]
    pub name: String,
    
    /// Message for agent
    #[arg(short, long)]
    pub message: String,
    
    /// Run every N seconds
    #[arg(short, long)]
    pub every: Option<u64>,
    
    /// Cron expression (e.g. '0 9 * * *')
    #[arg(short, long)]
    pub cron: Option<String>,
    
    /// Run once at time (ISO format)
    #[arg(long)]
    pub at: Option<String>,
    
    /// Deliver response to channel
    #[arg(short, long)]
    pub deliver: bool,
    
    /// Recipient for delivery
    #[arg(long)]
    pub to: Option<String>,
    
    /// Channel for delivery (e.g. 'telegram', 'whatsapp')
    #[arg(long)]
    pub channel: Option<String>,
}

#[derive(Args, Debug)]
pub struct CronRemoveArgs {
    /// Job ID to remove
    pub job_id: String,
}

#[derive(Args, Debug)]
pub struct CronEnableArgs {
    /// Job ID
    pub job_id: String,
    
    /// Disable instead of enable
    #[arg(long)]
    pub disable: bool,
}

#[derive(Args, Debug)]
pub struct CronRunArgs {
    /// Job ID to run
    pub job_id: String,
    
    /// Run even if disabled
    #[arg(short, long)]
    pub force: bool,
}

// =============================================================================
// CLI Implementation
// =============================================================================

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Some(Commands::Onboard) => cmd_onboard().await,
            Some(Commands::Agent(args)) => cmd_agent(args).await,
            Some(Commands::Gateway(args)) => cmd_gateway(args).await,
            Some(Commands::Status) => cmd_status().await,
            Some(Commands::Channels(cmd)) => match cmd {
                ChannelsCommands::Status => cmd_channels_status().await,
                ChannelsCommands::Login => cmd_channels_login().await,
            },
            Some(Commands::Cron(cmd)) => match cmd {
                CronCommands::List(args) => cmd_cron_list(args).await,
                CronCommands::Add(args) => cmd_cron_add(args).await,
                CronCommands::Remove(args) => cmd_cron_remove(args).await,
                CronCommands::Enable(args) => cmd_cron_enable(args).await,
                CronCommands::Run(args) => cmd_cron_run(args).await,
            },
            None => {
                // Show help if no command provided
                println!("{} nanobot v{}", LOGO, VERSION);
                println!("\nUse --help for more information");
                Ok(())
            }
        }
    }
}

// =============================================================================
// Command Implementations
// =============================================================================

/// Initialize nanobot configuration and workspace
async fn cmd_onboard() -> Result<()> {
    use crate::config::{get_default_config_path, Config};
    use crate::utils::get_workspace_path;
    
    let config_path = get_default_config_path();
    
    if config_path.exists() {
        println!("{}", format!("Config already exists at {}", config_path.display()).yellow());
        print!("Overwrite? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            return Ok(());
        }
    }
    
    // Create default config
    let config = Config::default();
    config.save(None)?;
    println!("{} Created config at {}", "✓".green(), config_path.display());
    
    // Create workspace
    let workspace = get_workspace_path(None)?;
    println!("{} Created workspace at {}", "✓".green(), workspace.display());
    
    // Create default bootstrap files
    create_workspace_templates(&workspace)?;
    
    println!("\n{} nanobot is ready!", LOGO);
    println!("\nNext steps:");
    println!("  1. Add your API key to {}", "~/.nanobot/config.json".cyan());
    println!("     Get one at: https://openrouter.ai/keys");
    println!("  2. Chat: {}", "nanobot agent -m \"Hello!\"".cyan());
    println!("\n{}", "Want Telegram/WhatsApp? See: https://github.com/HKUDS/nanobot#-chat-apps".dimmed());
    
    Ok(())
}

/// Create default workspace template files
fn create_workspace_templates(workspace: &Path) -> Result<()> {
    let templates = vec![
        (
            "AGENTS.md",
            r#"# Agent Instructions

You are a helpful AI assistant. Be concise, accurate, and friendly.

## Guidelines

- Always explain what you're doing before taking actions
- Ask for clarification when the request is ambiguous
- Use tools to help accomplish tasks
- Remember important information in your memory files
"#,
        ),
        (
            "SOUL.md",
            r#"# Soul

I am nanobot, a lightweight AI assistant.

## Personality

- Helpful and friendly
- Concise and to the point
- Curious and eager to learn

## Values

- Accuracy over speed
- User privacy and safety
- Transparency in actions
"#,
        ),
        (
            "USER.md",
            r#"# User

Information about the user goes here.

## Preferences

- Communication style: (casual/formal)
- Timezone: (your timezone)
- Language: (your preferred language)
"#,
        ),
    ];
    
    for (filename, content) in templates {
        let file_path = workspace.join(filename);
        if !file_path.exists() {
            std::fs::write(&file_path, content)?;
            println!("  {} Created {}", "".dimmed(), filename.dimmed());
        }
    }
    
    // Create memory directory and MEMORY.md
    let memory_dir = workspace.join("memory");
    std::fs::create_dir_all(&memory_dir)?;
    let memory_file = memory_dir.join("MEMORY.md");
    if !memory_file.exists() {
        std::fs::write(
            &memory_file,
            r#"# Long-term Memory

This file stores important information that should persist across sessions.

## User Information

(Important facts about the user)

## Preferences

(User preferences learned over time)

## Important Notes

(Things to remember)
"#,
        )?;
        println!("  {} Created memory/MEMORY.md", "".dimmed());
    }
    
    Ok(())
}

/// Interact with the agent directly
async fn cmd_agent(args: AgentArgs) -> Result<()> {
    use crate::config::Config;
    
    let config = Config::load(None)?;
    
    // Get provider configuration
    let _provider_cfg = config.get_provider(None)
        .ok_or_else(|| anyhow!("No API key configured. Set one in ~/.nanobot/config.json"))?;
    
    // TODO: Implement agent interaction once agent module is complete
    // This requires:
    // - providers::litellm::LiteLLMProvider
    // - agent::AgentLoop
    // - bus::queue::MessageBus
    
    if let Some(message) = args.message {
        // Single message mode
        println!("{} Agent module not yet implemented", "TODO:".yellow());
        println!("Message to send: {}", message);
    } else {
        // Interactive mode
        println!("{} Interactive mode (type {} or {} to quit)\n", LOGO, "exit".bold(), "Ctrl+C".bold());
        
        loop {
            print!("{} ", "You:".blue().bold());
            io::stdout().flush()?;
            
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) | Err(_) => {
                    println!("\nGoodbye!");
                    break;
                }
                Ok(_) => {}
            }
            
            let command = input.trim();
            if command.is_empty() {
                continue;
            }
            
            if is_exit_command(command) {
                println!("\nGoodbye!");
                break;
            }
            
            // TODO: Process through agent once implemented
            println!("{} Agent module not yet implemented", "TODO:".yellow());
            println!("Would process: {}", command);
        }
    }
    
    Ok(())
}

/// Print agent response with formatting
fn print_agent_response(response: &str, markdown: bool) -> Result<()> {
    println!("\n{} {}\n", LOGO, "nanobot".cyan().bold());
    
    // TODO: Implement markdown rendering when markdown=true
    // For now, just print as plain text
    let _ = markdown; // Suppress unused warning
    
    println!("{}\n", response);
    Ok(())
}

/// Check if command is an exit command
fn is_exit_command(command: &str) -> bool {
    matches!(
        command.to_lowercase().as_str(),
        "exit" | "quit" | "/exit" | "/quit" | ":q"
    )
}

/// Start the nanobot gateway
async fn cmd_gateway(args: GatewayArgs) -> Result<()> {
    use crate::config::{get_data_dir, Config};
    
    if args.verbose {
        tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )
        .ok();
    }
    
    println!("{} Starting nanobot gateway on port {}...", LOGO, args.port);
    
    let config = Config::load(None)?;
    
    // Get provider configuration
    let _provider_cfg = config.get_provider(None)
        .ok_or_else(|| anyhow!("No API key configured"))?;
    
    let _cron_store_path = get_data_dir().join("cron").join("jobs.json");
    
    // TODO: Implement gateway once all required modules are complete
    // This requires:
    // - providers::litellm::LiteLLMProvider
    // - agent::AgentLoop
    // - bus::queue::MessageBus
    // - session::SessionManager
    // - cron::CronService
    // - heartbeat::HeartbeatService
    // - channels::ChannelManager
    
    println!("{} Gateway module not yet implemented", "TODO:".yellow());
    println!("Configuration loaded successfully:");
    println!("  Port: {}", args.port);
    println!("  Workspace: {}", config.workspace_path().display());
    println!("  Model: {}", config.agents.defaults.model);
    
    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");
    
    Ok(())
}

/// Show nanobot status
async fn cmd_status() -> Result<()> {
    use crate::config::{get_default_config_path, Config, PROVIDERS};
    
    let config_path = get_default_config_path();
    let config = Config::load(None)?;
    let workspace = config.workspace_path();
    
    println!("{} nanobot Status\n", LOGO);
    
    let config_status = if config_path.exists() { "✓".green() } else { "✗".red() };
    println!("Config: {} {}", config_path.display(), config_status);
    
    let workspace_status = if workspace.exists() { "✓".green() } else { "✗".red() };
    println!("Workspace: {} {}", workspace.display(), workspace_status);
    
    if config_path.exists() {
        println!("Model: {}", config.agents.defaults.model);
        
        // Check API keys from registry
        for spec in PROVIDERS {
            if let Some(p) = config.get_provider_by_name(spec.name) {
                if spec.is_local {
                    // Local deployments show api_base instead of api_key
                    if let Some(ref base) = p.api_base {
                        println!("{}: {} {}", spec.name, "✓".green(), base);
                    } else {
                        println!("{}: {}", spec.name, "not set".dimmed());
                    }
                } else {
                    let has_key = !p.api_key.is_empty();
                    let status = if has_key { "✓".green() } else { "not set".dimmed() };
                    println!("{}: {}", spec.name, status);
                }
            }
        }
    }
    
    Ok(())
}

/// Show channel status
async fn cmd_channels_status() -> Result<()> {
    use crate::config::Config;
    
    let config = Config::load(None)?;
    
    println!("{} Channel Status\n", LOGO);
    
    // WhatsApp
    let wa = &config.channels.whatsapp;
    let wa_status = if wa.enabled { "✓".green() } else { "✗".dimmed() };
    println!("WhatsApp: {} - {}", wa_status, wa.bridge_url);
    
    // Discord
    let dc = &config.channels.discord;
    let dc_status = if dc.enabled { "✓".green() } else { "✗".dimmed() };
    println!("Discord: {} - {}", dc_status, dc.gateway_url);
    
    // Telegram
    let tg = &config.channels.telegram;
    let tg_config = if !tg.token.is_empty() {
        format!("token: {}...", &tg.token[..10.min(tg.token.len())])
    } else {
        "not configured".dimmed().to_string()
    };
    let tg_status = if tg.enabled { "✓".green() } else { "✗".dimmed() };
    println!("Telegram: {} - {}", tg_status, tg_config);
    
    // Slack
    let slack = &config.channels.slack;
    let slack_config = if !slack.app_token.is_empty() && !slack.bot_token.is_empty() {
        "socket".to_string()
    } else {
        "not configured".dimmed().to_string()
    };
    let slack_status = if slack.enabled { "✓".green() } else { "✗".dimmed() };
    println!("Slack: {} - {}", slack_status, slack_config);
    
    Ok(())
}

/// Link device via QR code
async fn cmd_channels_login() -> Result<()> {
    use std::process::Command;
    
    let bridge_dir = get_bridge_dir()?;
    
    println!("{} Starting bridge...", LOGO);
    println!("Scan the QR code to connect.\n");
    
    let status = Command::new("npm")
        .arg("start")
        .current_dir(&bridge_dir)
        .status()
        .context("Failed to run npm start. Is Node.js installed?")?;
    
    if !status.success() {
        return Err(anyhow!("Bridge failed with exit code: {}", status));
    }
    
    Ok(())
}

/// Get the bridge directory, setting it up if needed
fn get_bridge_dir() -> Result<PathBuf> {
    use std::process::Command;
    
    // User's bridge location
    let user_bridge = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join(".nanobot")
        .join("bridge");
    
    // Check if already built
    if user_bridge.join("dist").join("index.js").exists() {
        return Ok(user_bridge);
    }
    
    // Check for npm
    if Command::new("npm").arg("--version").output().is_err() {
        return Err(anyhow!("npm not found. Please install Node.js >= 18."));
    }
    
    // Find source bridge (in repository)
    let current_dir = std::env::current_dir()?;
    let repo_bridge = current_dir.join("bridge");
    
    if !repo_bridge.join("package.json").exists() {
        return Err(anyhow!("Bridge source not found. Try reinstalling nanobot."));
    }
    
    println!("{} Setting up bridge...", LOGO);
    
    // Copy to user directory
    if user_bridge.exists() {
        std::fs::remove_dir_all(&user_bridge)?;
    }
    copy_dir_all(&repo_bridge, &user_bridge)?;
    
    // Install and build
    println!("  Installing dependencies...");
    let install_status = Command::new("npm")
        .arg("install")
        .current_dir(&user_bridge)
        .status()
        .context("Failed to run npm install")?;
    
    if !install_status.success() {
        return Err(anyhow!("npm install failed"));
    }
    
    println!("  Building...");
    let build_status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&user_bridge)
        .status()
        .context("Failed to run npm build")?;
    
    if !build_status.success() {
        return Err(anyhow!("npm build failed"));
    }
    
    println!("{} Bridge ready\n", "✓".green());
    
    Ok(user_bridge)
}

/// Copy directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        // Skip node_modules and dist
        if entry.file_name() == "node_modules" || entry.file_name() == "dist" {
            continue;
        }
        
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// List scheduled jobs
async fn cmd_cron_list(args: CronListArgs) -> Result<()> {
    use crate::config::get_data_dir;
    
    let _store_path = get_data_dir().join("cron").join("jobs.json");
    
    // TODO: Implement cron service once cron module is complete
    // This requires: cron::CronService
    
    println!("{} Cron module not yet implemented", "TODO:".yellow());
    println!("Would list jobs (include_disabled: {})", args.all);
    
    Ok(())
}

/// Add a scheduled job
async fn cmd_cron_add(args: CronAddArgs) -> Result<()> {
    use crate::config::get_data_dir;
    
    let _store_path = get_data_dir().join("cron").join("jobs.json");
    
    // TODO: Implement cron service once cron module is complete
    // This requires: cron::{CronService, CronSchedule}
    
    println!("{} Cron module not yet implemented", "TODO:".yellow());
    println!("Would add job:");
    println!("  Name: {}", args.name);
    println!("  Message: {}", args.message);
    
    if let Some(every) = args.every {
        println!("  Schedule: every {}s", every);
    } else if let Some(ref expr) = args.cron {
        println!("  Schedule: cron '{}'", expr);
    } else if let Some(ref at) = args.at {
        println!("  Schedule: at {}", at);
    }
    
    Ok(())
}

/// Remove a scheduled job
async fn cmd_cron_remove(args: CronRemoveArgs) -> Result<()> {
    use crate::config::get_data_dir;
    
    let _store_path = get_data_dir().join("cron").join("jobs.json");
    
    // TODO: Implement cron service once cron module is complete
    // This requires: cron::CronService
    
    println!("{} Cron module not yet implemented", "TODO:".yellow());
    println!("Would remove job: {}", args.job_id);
    
    Ok(())
}

/// Enable or disable a job
async fn cmd_cron_enable(args: CronEnableArgs) -> Result<()> {
    use crate::config::get_data_dir;
    
    let _store_path = get_data_dir().join("cron").join("jobs.json");
    
    // TODO: Implement cron service once cron module is complete
    // This requires: cron::CronService
    
    println!("{} Cron module not yet implemented", "TODO:".yellow());
    let action = if args.disable { "disable" } else { "enable" };
    println!("Would {} job: {}", action, args.job_id);
    
    Ok(())
}

/// Manually run a job
async fn cmd_cron_run(args: CronRunArgs) -> Result<()> {
    use crate::config::get_data_dir;
    
    let _store_path = get_data_dir().join("cron").join("jobs.json");
    
    // TODO: Implement cron service once cron module is complete
    // This requires: cron::CronService
    
    println!("{} Cron module not yet implemented", "TODO:".yellow());
    println!("Would run job: {} (force: {})", args.job_id, args.force);
    
    Ok(())
}
