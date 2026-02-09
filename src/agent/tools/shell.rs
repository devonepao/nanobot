//! Shell execution tool.

use super::base::{get_optional_string, get_string_param, Tool};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Tool to execute shell commands.
pub struct ExecTool {
    timeout_secs: u64,
    working_dir: Option<String>,
    deny_patterns: Vec<Regex>,
    allow_patterns: Vec<Regex>,
    restrict_to_workspace: bool,
}

impl ExecTool {
    pub fn new(
        timeout_secs: u64,
        working_dir: Option<String>,
        deny_patterns: Option<Vec<String>>,
        allow_patterns: Option<Vec<String>>,
        restrict_to_workspace: bool,
    ) -> Result<Self> {
        let default_deny = vec![
            r"\brm\s+-[rf]{1,2}\b".to_string(),
            r"\bdel\s+/[fq]\b".to_string(),
            r"\brmdir\s+/s\b".to_string(),
            r"\b(format|mkfs|diskpart)\b".to_string(),
            r"\bdd\s+if=".to_string(),
            r">\s*/dev/sd".to_string(),
            r"\b(shutdown|reboot|poweroff)\b".to_string(),
            r":\(\)\s*\{.*\};\s*:".to_string(),
        ];

        let deny_list = deny_patterns.unwrap_or(default_deny);
        let deny_compiled: Result<Vec<Regex>> = deny_list
            .into_iter()
            .map(|p| Regex::new(&p).map_err(|e| anyhow!("Invalid deny pattern: {}", e)))
            .collect();

        let allow_list = allow_patterns.unwrap_or_default();
        let allow_compiled: Result<Vec<Regex>> = allow_list
            .into_iter()
            .map(|p| Regex::new(&p).map_err(|e| anyhow!("Invalid allow pattern: {}", e)))
            .collect();

        Ok(Self {
            timeout_secs,
            working_dir,
            deny_patterns: deny_compiled?,
            allow_patterns: allow_compiled?,
            restrict_to_workspace,
        })
    }
}

#[async_trait]
impl Tool for ExecTool {
    fn name(&self) -> &str {
        "exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output. Use with caution."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for the command"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let command = get_string_param(&params, "command")
            .ok_or_else(|| anyhow!("Missing required parameter: command"))?;
        let working_dir = get_optional_string(&params, "working_dir");

        let cwd = working_dir
            .as_deref()
            .or(self.working_dir.as_deref())
            .unwrap_or(".");

        // Safety guard
        if let Some(error) = self.guard_command(&command, cwd) {
            return Ok(error);
        }

        // Execute command
        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        };
        let shell_arg = if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        };

        let child = Command::new(shell)
            .arg(shell_arg)
            .arg(&command)
            .current_dir(cwd)
            .output();

        let result = match timeout(Duration::from_secs(self.timeout_secs), child).await {
            Ok(Ok(output)) => {
                let mut result_parts = Vec::new();

                if !output.stdout.is_empty() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    result_parts.push(stdout.to_string());
                }

                if !output.stderr.is_empty() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.trim().is_empty() {
                        result_parts.push(format!("STDERR:\n{}", stderr));
                    }
                }

                if !output.status.success() {
                    result_parts.push(format!("\nExit code: {}", output.status.code().unwrap_or(-1)));
                }

                let result = if result_parts.is_empty() {
                    "(no output)".to_string()
                } else {
                    result_parts.join("\n")
                };

                // Truncate very long output
                let max_len = 10000;
                if result.len() > max_len {
                    format!(
                        "{}\n... (truncated, {} more chars)",
                        &result[..max_len],
                        result.len() - max_len
                    )
                } else {
                    result
                }
            }
            Ok(Err(e)) => format!("Error executing command: {}", e),
            Err(_) => format!("Error: Command timed out after {} seconds", self.timeout_secs),
        };

        Ok(result)
    }
}

impl ExecTool {
    /// Best-effort safety guard for potentially destructive commands.
    fn guard_command(&self, command: &str, cwd: &str) -> Option<String> {
        let cmd = command.trim();
        let lower = cmd.to_lowercase();

        // Check deny patterns
        for pattern in &self.deny_patterns {
            if pattern.is_match(&lower) {
                return Some(
                    "Error: Command blocked by safety guard (dangerous pattern detected)"
                        .to_string(),
                );
            }
        }

        // Check allow patterns
        if !self.allow_patterns.is_empty() {
            let allowed = self.allow_patterns.iter().any(|p| p.is_match(&lower));
            if !allowed {
                return Some(
                    "Error: Command blocked by safety guard (not in allowlist)".to_string()
                );
            }
        }

        // Check workspace restriction
        if self.restrict_to_workspace {
            if cmd.contains("../") || cmd.contains("..\\") {
                return Some(
                    "Error: Command blocked by safety guard (path traversal detected)".to_string(),
                );
            }

            if let Ok(cwd_path) = PathBuf::from(cwd).canonicalize() {
                // Simple path extraction (not perfect but reasonable)
                let win_paths_re = Regex::new(r"[A-Za-z]:\\[^\\\x22\x27]+").unwrap();
                let posix_paths_re = Regex::new(r#"/[^\s"\x27]+"#).unwrap();

                for mat in win_paths_re.find_iter(cmd).chain(posix_paths_re.find_iter(cmd)) {
                    if let Ok(p) = PathBuf::from(mat.as_str()).canonicalize() {
                        if !p.starts_with(&cwd_path) && p != cwd_path {
                            return Some(
                                "Error: Command blocked by safety guard (path outside working dir)"
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }

        None
    }
}
