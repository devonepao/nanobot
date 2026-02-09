//! Spawn tool for creating background subagents.

use super::base::{get_optional_string, get_string_param, Tool};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Callback type for spawning subagents.
type SpawnCallback = Arc<
    dyn Fn(String, Option<String>, String, String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
        + Send
        + Sync,
>;

/// Tool to spawn a subagent for background task execution.
///
/// The subagent runs asynchronously and announces its result back
/// to the main agent when complete.
pub struct SpawnTool {
    manager_callback: Arc<Mutex<Option<SpawnCallback>>>,
    origin_channel: Arc<Mutex<String>>,
    origin_chat_id: Arc<Mutex<String>>,
}

impl SpawnTool {
    pub fn new(manager_callback: Option<SpawnCallback>) -> Self {
        Self {
            manager_callback: Arc::new(Mutex::new(manager_callback)),
            origin_channel: Arc::new(Mutex::new("cli".to_string())),
            origin_chat_id: Arc::new(Mutex::new("direct".to_string())),
        }
    }

    /// Set the origin context for subagent announcements.
    pub async fn set_context(&self, channel: String, chat_id: String) {
        *self.origin_channel.lock().await = channel;
        *self.origin_chat_id.lock().await = chat_id;
    }

    /// Set the manager callback.
    pub async fn set_manager_callback(&self, callback: SpawnCallback) {
        *self.manager_callback.lock().await = Some(callback);
    }
}

#[async_trait]
impl Tool for SpawnTool {
    fn name(&self) -> &str {
        "spawn"
    }

    fn description(&self) -> &str {
        "Spawn a subagent to handle a task in the background. \
         Use this for complex or time-consuming tasks that can run independently. \
         The subagent will complete the task and report back when done."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The task for the subagent to complete"
                },
                "label": {
                    "type": "string",
                    "description": "Optional short label for the task (for display)"
                }
            },
            "required": ["task"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let task = get_string_param(&params, "task")
            .ok_or_else(|| anyhow!("Missing required parameter: task"))?;
        let label = get_optional_string(&params, "label");

        let callback_guard = self.manager_callback.lock().await;
        let callback = match callback_guard.as_ref() {
            Some(cb) => cb,
            None => return Ok("Error: Subagent manager not configured".to_string()),
        };

        let origin_channel = self.origin_channel.lock().await.clone();
        let origin_chat_id = self.origin_chat_id.lock().await.clone();

        match callback(task, label, origin_channel, origin_chat_id).await {
            Ok(result) => Ok(result),
            Err(e) => Ok(format!("Error spawning subagent: {}", e)),
        }
    }
}
