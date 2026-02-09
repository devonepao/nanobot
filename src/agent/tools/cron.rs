//! Cron tool for scheduling reminders and tasks.

use super::base::{get_int_param, get_optional_string, get_string_param, Tool};
use crate::cron::{CronSchedule, CronService};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tool to schedule reminders and recurring tasks.
pub struct CronTool {
    cron_service: Arc<CronService>,
    channel: Arc<Mutex<String>>,
    chat_id: Arc<Mutex<String>>,
}

impl CronTool {
    pub fn new(cron_service: Arc<CronService>) -> Self {
        Self {
            cron_service,
            channel: Arc::new(Mutex::new(String::new())),
            chat_id: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Set the current session context for delivery.
    pub async fn set_context(&self, channel: String, chat_id: String) {
        *self.channel.lock().await = channel;
        *self.chat_id.lock().await = chat_id;
    }

    async fn add_job(
        &self,
        message: String,
        every_seconds: Option<i64>,
        cron_expr: Option<String>,
    ) -> String {
        if message.is_empty() {
            return "Error: message is required for add".to_string();
        }

        let channel = self.channel.lock().await.clone();
        let chat_id = self.chat_id.lock().await.clone();

        if channel.is_empty() || chat_id.is_empty() {
            return "Error: no session context (channel/chat_id)".to_string();
        }

        // Build schedule
        let schedule = if let Some(seconds) = every_seconds {
            CronSchedule::every(seconds * 1000)
        } else if let Some(expr) = cron_expr {
            CronSchedule::cron(expr)
        } else {
            return "Error: either every_seconds or cron_expr is required".to_string();
        };

        let job_name = if message.len() > 30 {
            message[..30].to_string()
        } else {
            message.clone()
        };

        match self
            .cron_service
            .add_job(
                job_name,
                schedule,
                message,
                true,
                Some(channel),
                Some(chat_id),
                false,
            )
            .await
        {
            Ok(job) => format!("Created job '{}' (id: {})", job.name, job.id),
            Err(e) => format!("Error creating job: {}", e),
        }
    }

    async fn list_jobs(&self) -> String {
        let jobs = self.cron_service.list_jobs(false).await;

        if jobs.is_empty() {
            return "No scheduled jobs.".to_string();
        }

        let lines: Vec<String> = jobs
            .iter()
            .map(|j| {
                format!(
                    "- {} (id: {}, {:?})",
                    j.name,
                    j.id,
                    j.schedule.kind
                )
            })
            .collect();

        format!("Scheduled jobs:\n{}", lines.join("\n"))
    }

    async fn remove_job(&self, job_id: Option<String>) -> String {
        let job_id = match job_id {
            Some(id) => id,
            None => return "Error: job_id is required for remove".to_string(),
        };

        match self.cron_service.remove_job(&job_id).await {
            Ok(true) => format!("Removed job {}", job_id),
            Ok(false) => format!("Job {} not found", job_id),
            Err(e) => format!("Error removing job: {}", e),
        }
    }
}

#[async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Schedule reminders and recurring tasks. Actions: add, list, remove."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "list", "remove"],
                    "description": "Action to perform"
                },
                "message": {
                    "type": "string",
                    "description": "Reminder message (for add)"
                },
                "every_seconds": {
                    "type": "integer",
                    "description": "Interval in seconds (for recurring tasks)"
                },
                "cron_expr": {
                    "type": "string",
                    "description": "Cron expression like '0 9 * * *' (for scheduled tasks)"
                },
                "job_id": {
                    "type": "string",
                    "description": "Job ID (for remove)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let action = get_string_param(&params, "action")
            .ok_or_else(|| anyhow!("Missing required parameter: action"))?;

        let result = match action.as_str() {
            "add" => {
                let message = get_string_param(&params, "message").unwrap_or_default();
                let every_seconds = get_int_param(&params, "every_seconds");
                let cron_expr = get_optional_string(&params, "cron_expr");
                self.add_job(message, every_seconds, cron_expr).await
            }
            "list" => self.list_jobs().await,
            "remove" => {
                let job_id = get_optional_string(&params, "job_id");
                self.remove_job(job_id).await
            }
            _ => format!("Unknown action: {}", action),
        };

        Ok(result)
    }
}
