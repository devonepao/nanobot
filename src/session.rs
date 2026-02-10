//! Session management for conversation history.
//!
//! This module provides session management with:
//! - Message history storage
//! - JSONL file-based persistence
//! - Session loading and saving
//! - Async I/O operations

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, warn};

use crate::utils::safe_filename;

/// A message in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender (e.g., "user", "assistant", "system")
    pub role: String,
    /// Content of the message
    pub content: String,
    /// Timestamp when the message was created
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Message {
    /// Create a new message.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new message with additional metadata.
    pub fn with_metadata(
        role: impl Into<String>,
        content: impl Into<String>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
            metadata,
        }
    }

    /// Convert to LLM format (just role and content).
    pub fn to_llm_format(&self) -> serde_json::Value {
        serde_json::json!({
            "role": self.role,
            "content": self.content,
        })
    }
}

/// Metadata stored at the beginning of a session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetadata {
    #[serde(rename = "_type")]
    type_: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    metadata: HashMap<String, serde_json::Value>,
}

/// A conversation session.
///
/// Stores messages in JSONL format for easy reading and persistence.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session key (usually channel:chat_id)
    pub key: String,
    /// List of messages in the session
    pub messages: Vec<Message>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            key: key.into(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, role: impl Into<String>, content: impl Into<String>) {
        let msg = Message::new(role, content);
        self.messages.push(msg);
        self.updated_at = Utc::now();
    }

    /// Add a message with additional metadata.
    pub fn add_message_with_metadata(
        &mut self,
        role: impl Into<String>,
        content: impl Into<String>,
        metadata: HashMap<String, serde_json::Value>,
    ) {
        let msg = Message::with_metadata(role, content, metadata);
        self.messages.push(msg);
        self.updated_at = Utc::now();
    }

    /// Get message history for LLM context.
    ///
    /// Returns the most recent messages (up to max_messages) in LLM format
    /// (just role and content).
    pub fn get_history(&self, max_messages: usize) -> Vec<serde_json::Value> {
        let start_idx = if self.messages.len() > max_messages {
            self.messages.len() - max_messages
        } else {
            0
        };

        self.messages[start_idx..]
            .iter()
            .map(|m| m.to_llm_format())
            .collect()
    }

    /// Clear all messages in the session.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    /// Get the number of messages in the session.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

/// Information about a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session key
    pub key: String,
    /// When the session was created
    pub created_at: String,
    /// When the session was last updated
    pub updated_at: String,
    /// File path to the session
    pub path: String,
}

/// Manages conversation sessions.
///
/// Sessions are stored as JSONL files in the sessions directory.
pub struct SessionManager {
    /// Workspace path
    workspace: PathBuf,
    /// Directory where sessions are stored
    sessions_dir: PathBuf,
    /// In-memory cache of sessions
    cache: tokio::sync::RwLock<HashMap<String, Session>>,
}

impl SessionManager {
    /// Create a new session manager.
    ///
    /// # Arguments
    /// * `workspace` - The workspace directory path
    pub async fn new(workspace: impl AsRef<Path>) -> Result<Self> {
        let workspace = workspace.as_ref().to_path_buf();
        let sessions_dir = dirs::home_dir()
            .context("Failed to get home directory")?
            .join(".nanobot")
            .join("sessions");

        // Ensure sessions directory exists
        fs::create_dir_all(&sessions_dir)
            .await
            .context("Failed to create sessions directory")?;

        debug!("Session manager initialized with sessions dir: {:?}", sessions_dir);

        Ok(Self {
            workspace,
            sessions_dir,
            cache: tokio::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Create a new session manager with custom sessions directory (for testing).
    #[cfg(test)]
    async fn with_sessions_dir(workspace: impl AsRef<Path>, sessions_dir: impl AsRef<Path>) -> Result<Self> {
        let workspace = workspace.as_ref().to_path_buf();
        let sessions_dir = sessions_dir.as_ref().to_path_buf();

        // Ensure sessions directory exists
        fs::create_dir_all(&sessions_dir)
            .await
            .context("Failed to create sessions directory")?;

        Ok(Self {
            workspace,
            sessions_dir,
            cache: tokio::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Get the file path for a session.
    fn get_session_path(&self, key: &str) -> PathBuf {
        // Replace colon with underscore before making safe (e.g., "telegram:123" -> "telegram_123")
        let safe_key = safe_filename(&key.replace(':', "_"));
        self.sessions_dir.join(format!("{}.jsonl", safe_key))
    }

    /// Get an existing session or create a new one.
    ///
    /// # Arguments
    /// * `key` - Session key (usually channel:chat_id)
    pub async fn get_or_create(&self, key: impl Into<String>) -> Result<Session> {
        let key = key.into();

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(session) = cache.get(&key) {
                debug!("Session {} found in cache", key);
                return Ok(session.clone());
            }
        }

        // Try to load from disk
        let session = match self.load(&key).await {
            Ok(Some(session)) => {
                debug!("Session {} loaded from disk", key);
                session
            }
            Ok(None) => {
                debug!("Creating new session {}", key);
                Session::new(&key)
            }
            Err(e) => {
                warn!("Failed to load session {}: {}", key, e);
                Session::new(&key)
            }
        };

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(key.clone(), session.clone());
        }

        Ok(session)
    }

    /// Load a session from disk.
    async fn load(&self, key: &str) -> Result<Option<Session>> {
        let path = self.get_session_path(key);

        if !path.exists() {
            return Ok(None);
        }

        let file = fs::File::open(&path)
            .await
            .context("Failed to open session file")?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut messages = Vec::new();
        let mut metadata = HashMap::new();
        let mut created_at = Utc::now();
        let mut updated_at = Utc::now();

        while let Some(line) = lines.next_line().await? {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(data) => {
                    if data.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
                        // Parse metadata line
                        if let Ok(meta) = serde_json::from_value::<SessionMetadata>(data) {
                            metadata = meta.metadata;
                            created_at = meta.created_at;
                            updated_at = meta.updated_at;
                        }
                    } else {
                        // Parse message line
                        if let Ok(msg) = serde_json::from_value::<Message>(data) {
                            messages.push(msg);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse line in session {}: {}", key, e);
                }
            }
        }

        Ok(Some(Session {
            key: key.to_string(),
            messages,
            created_at,
            updated_at,
            metadata,
        }))
    }

    /// Save a session to disk.
    ///
    /// # Arguments
    /// * `session` - The session to save
    pub async fn save(&self, session: &Session) -> Result<()> {
        let path = self.get_session_path(&session.key);

        let mut file = fs::File::create(&path)
            .await
            .context("Failed to create session file")?;

        // Write metadata first
        let metadata_line = SessionMetadata {
            type_: "metadata".to_string(),
            created_at: session.created_at,
            updated_at: session.updated_at,
            metadata: session.metadata.clone(),
        };
        let metadata_json = serde_json::to_string(&metadata_line)?;
        file.write_all(metadata_json.as_bytes()).await?;
        file.write_all(b"\n").await?;

        // Write messages
        for msg in &session.messages {
            let msg_json = serde_json::to_string(msg)?;
            file.write_all(msg_json.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(session.key.clone(), session.clone());
        }

        debug!("Session {} saved to disk", session.key);
        Ok(())
    }

    /// Delete a session.
    ///
    /// # Arguments
    /// * `key` - Session key
    ///
    /// # Returns
    /// `true` if the session was deleted, `false` if it didn't exist
    pub async fn delete(&self, key: &str) -> Result<bool> {
        // Remove from cache
        {
            let mut cache = self.cache.write().await;
            cache.remove(key);
        }

        // Remove file
        let path = self.get_session_path(key);
        if path.exists() {
            fs::remove_file(&path)
                .await
                .context("Failed to delete session file")?;
            debug!("Session {} deleted", key);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all sessions.
    ///
    /// # Returns
    /// A list of session information, sorted by update time (most recent first)
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let mut sessions = Vec::new();

        let mut entries = fs::read_dir(&self.sessions_dir)
            .await
            .context("Failed to read sessions directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }

            match self.read_session_info(&path).await {
                Ok(Some(info)) => sessions.push(info),
                Ok(None) => continue,
                Err(e) => {
                    warn!("Failed to read session info from {:?}: {}", path, e);
                    continue;
                }
            }
        }

        // Sort by updated_at (most recent first)
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    /// Read session information from a file.
    async fn read_session_info(&self, path: &Path) -> Result<Option<SessionInfo>> {
        let file = fs::File::open(path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        if let Some(first_line) = lines.next_line().await? {
            let first_line = first_line.trim();
            if !first_line.is_empty() {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(first_line) {
                    if data.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
                        let key = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .replace('_', ":");
                        let created_at = data
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let updated_at = data
                            .get("updated_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        return Ok(Some(SessionInfo {
                            key,
                            created_at,
                            updated_at,
                            path: path.to_string_lossy().to_string(),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get the sessions directory path.
    pub fn sessions_dir(&self) -> &Path {
        &self.sessions_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_message_creation() {
        let msg = Message::new("user", "Hello, world!");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, world!");
        assert!(msg.metadata.is_empty());
    }

    #[test]
    fn test_message_to_llm_format() {
        let msg = Message::new("assistant", "Hi there!");
        let llm_format = msg.to_llm_format();
        assert_eq!(llm_format["role"], "assistant");
        assert_eq!(llm_format["content"], "Hi there!");
    }

    #[test]
    fn test_session_creation() {
        let session = Session::new("test:123");
        assert_eq!(session.key, "test:123");
        assert_eq!(session.messages.len(), 0);
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new("test:123");
        session.add_message("user", "Hello");
        session.add_message("assistant", "Hi");

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[0].content, "Hello");
        assert_eq!(session.messages[1].role, "assistant");
        assert_eq!(session.messages[1].content, "Hi");
    }

    #[test]
    fn test_session_get_history() {
        let mut session = Session::new("test:123");
        for i in 0..10 {
            session.add_message("user", format!("Message {}", i));
        }

        let history = session.get_history(5);
        assert_eq!(history.len(), 5);
        assert_eq!(history[0]["content"], "Message 5");
        assert_eq!(history[4]["content"], "Message 9");
    }

    #[test]
    fn test_session_clear() {
        let mut session = Session::new("test:123");
        session.add_message("user", "Hello");
        session.add_message("assistant", "Hi");

        session.clear();
        assert_eq!(session.messages.len(), 0);
    }

    #[tokio::test]
    async fn test_session_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        let manager = SessionManager::with_sessions_dir(temp_dir.path(), &sessions_dir).await.unwrap();
        assert!(manager.sessions_dir().exists());
    }

    #[tokio::test]
    async fn test_session_manager_get_or_create() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        let manager = SessionManager::with_sessions_dir(temp_dir.path(), &sessions_dir).await.unwrap();

        let session = manager.get_or_create("test:123").await.unwrap();
        assert_eq!(session.key, "test:123");
        assert_eq!(session.messages.len(), 0);
    }

    #[tokio::test]
    async fn test_session_manager_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        let manager = SessionManager::with_sessions_dir(temp_dir.path(), &sessions_dir).await.unwrap();

        let mut session = Session::new("test:123");
        session.add_message("user", "Hello");
        session.add_message("assistant", "Hi there!");

        manager.save(&session).await.unwrap();

        // Clear cache to force load from disk
        {
            let mut cache = manager.cache.write().await;
            cache.clear();
        }

        let loaded_session = manager.get_or_create("test:123").await.unwrap();
        assert_eq!(loaded_session.messages.len(), 2);
        assert_eq!(loaded_session.messages[0].content, "Hello");
        assert_eq!(loaded_session.messages[1].content, "Hi there!");
    }

    #[tokio::test]
    async fn test_session_manager_delete() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        let manager = SessionManager::with_sessions_dir(temp_dir.path(), &sessions_dir).await.unwrap();

        let session = Session::new("test:123");
        manager.save(&session).await.unwrap();

        let deleted = manager.delete("test:123").await.unwrap();
        assert!(deleted);

        let deleted_again = manager.delete("test:123").await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_session_manager_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        let manager = SessionManager::with_sessions_dir(temp_dir.path(), &sessions_dir).await.unwrap();

        let mut session1 = Session::new("test:123");
        manager.save(&session1).await.unwrap();
        
        // Wait and add a message to ensure different timestamp
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let mut session2 = Session::new("test:456");
        session2.add_message("user", "test message");
        manager.save(&session2).await.unwrap();

        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        
        // Session 2 should be more recent
        assert!(sessions[0].updated_at > sessions[1].updated_at);
    }
}
