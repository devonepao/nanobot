//! Event types for the message bus.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message received from a chat channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Channel type: telegram, discord, slack, whatsapp
    pub channel: String,
    /// User identifier
    pub sender_id: String,
    /// Chat/channel identifier
    pub chat_id: String,
    /// Message text
    pub content: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Media URLs
    pub media: Vec<String>,
    /// Channel-specific data
    pub metadata: HashMap<String, serde_json::Value>,
}

impl InboundMessage {
    /// Create a new inbound message
    pub fn new(
        channel: String,
        sender_id: String,
        chat_id: String,
        content: String,
    ) -> Self {
        Self {
            channel,
            sender_id,
            chat_id,
            content,
            timestamp: Utc::now(),
            media: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Unique key for session identification.
    pub fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }
}

/// Message to send to a chat channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Channel type
    pub channel: String,
    /// Chat/channel identifier
    pub chat_id: String,
    /// Message text
    pub content: String,
    /// Message ID to reply to
    pub reply_to: Option<String>,
    /// Media URLs
    pub media: Vec<String>,
    /// Channel-specific data
    pub metadata: HashMap<String, serde_json::Value>,
}

impl OutboundMessage {
    /// Create a new outbound message
    pub fn new(channel: String, chat_id: String, content: String) -> Self {
        Self {
            channel,
            chat_id,
            content,
            reply_to: None,
            media: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}
