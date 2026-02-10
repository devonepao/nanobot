//! Message tool for sending messages to users.

use super::base::{get_optional_string, get_string_param, Tool};
use crate::bus::events::OutboundMessage;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

type SendCallback = Arc<dyn Fn(OutboundMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync>;

/// Tool to send messages to users on chat channels.
pub struct MessageTool {
    send_callback: Arc<Mutex<Option<SendCallback>>>,
    default_channel: Arc<Mutex<String>>,
    default_chat_id: Arc<Mutex<String>>,
}

impl MessageTool {
    pub fn new(
        send_callback: Option<SendCallback>,
        default_channel: String,
        default_chat_id: String,
    ) -> Self {
        Self {
            send_callback: Arc::new(Mutex::new(send_callback)),
            default_channel: Arc::new(Mutex::new(default_channel)),
            default_chat_id: Arc::new(Mutex::new(default_chat_id)),
        }
    }

    /// Set the current message context.
    pub async fn set_context(&self, channel: String, chat_id: String) {
        *self.default_channel.lock().await = channel;
        *self.default_chat_id.lock().await = chat_id;
    }

    /// Set the callback for sending messages.
    pub async fn set_send_callback(&self, callback: SendCallback) {
        *self.send_callback.lock().await = Some(callback);
    }
}

#[async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str {
        "message"
    }

    fn description(&self) -> &str {
        "Send a message to the user. Use this when you want to communicate something."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The message content to send"
                },
                "channel": {
                    "type": "string",
                    "description": "Optional: target channel (telegram, discord, etc.)"
                },
                "chat_id": {
                    "type": "string",
                    "description": "Optional: target chat/user ID"
                }
            },
            "required": ["content"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let content = get_string_param(&params, "content")
            .ok_or_else(|| anyhow!("Missing required parameter: content"))?;

        let channel = get_optional_string(&params, "channel")
            .unwrap_or_else(|| self.default_channel.try_lock().map(|g| g.clone()).unwrap_or_default());
        let chat_id = get_optional_string(&params, "chat_id")
            .unwrap_or_else(|| self.default_chat_id.try_lock().map(|g| g.clone()).unwrap_or_default());

        if channel.is_empty() || chat_id.is_empty() {
            return Ok("Error: No target channel/chat specified".to_string());
        }

        let callback_guard = self.send_callback.lock().await;
        let callback = match callback_guard.as_ref() {
            Some(cb) => cb,
            None => return Ok("Error: Message sending not configured".to_string()),
        };

        let msg = OutboundMessage::new(channel.clone(), chat_id.clone(), content);

        match callback(msg).await {
            Ok(_) => Ok(format!("Message sent to {}:{}", channel, chat_id)),
            Err(e) => Ok(format!("Error sending message: {}", e)),
        }
    }
}
