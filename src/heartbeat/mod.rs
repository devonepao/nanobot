//! Heartbeat service - periodic agent wake-up to check for tasks.
//!
//! The heartbeat service wakes the agent periodically to check for tasks in
//! `HEARTBEAT.md`. If the file has actionable content, the agent processes it.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};

/// Default heartbeat interval: 30 minutes
const DEFAULT_HEARTBEAT_INTERVAL_S: u64 = 30 * 60;

/// The prompt sent to agent during heartbeat
const HEARTBEAT_PROMPT: &str = r#"Read HEARTBEAT.md in your workspace (if it exists).
Follow any instructions or tasks listed there.
If nothing needs attention, reply with just: HEARTBEAT_OK"#;

/// Token that indicates "nothing to do"
const HEARTBEAT_OK_TOKEN: &str = "HEARTBEAT_OK";

/// Callback type for heartbeat execution
type HeartbeatCallback = Arc<
    dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
        + Send
        + Sync,
>;

/// Periodic heartbeat service that wakes the agent to check for tasks.
///
/// The agent reads `HEARTBEAT.md` from the workspace and executes any
/// tasks listed there. If nothing needs attention, it replies `HEARTBEAT_OK`.
pub struct HeartbeatService {
    workspace: PathBuf,
    on_heartbeat: Option<HeartbeatCallback>,
    interval: Duration,
    enabled: bool,
    running: Arc<RwLock<bool>>,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            on_heartbeat: None,
            interval: Duration::from_secs(DEFAULT_HEARTBEAT_INTERVAL_S),
            enabled: true,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Set the heartbeat interval
    pub fn with_interval(mut self, seconds: u64) -> Self {
        self.interval = Duration::from_secs(seconds);
        self
    }

    /// Set whether the heartbeat is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the callback function for heartbeat execution
    pub fn with_callback<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String>> + Send + 'static,
    {
        self.on_heartbeat = Some(Arc::new(move |prompt| Box::pin(callback(prompt))));
        self
    }

    /// Get the path to the HEARTBEAT.md file
    fn heartbeat_file(&self) -> PathBuf {
        self.workspace.join("HEARTBEAT.md")
    }

    /// Read HEARTBEAT.md content
    fn read_heartbeat_file(&self) -> Option<String> {
        let path = self.heartbeat_file();
        if path.exists() {
            std::fs::read_to_string(path).ok()
        } else {
            None
        }
    }

    /// Check if HEARTBEAT.md has actionable content
    fn is_heartbeat_empty(content: &Option<String>) -> bool {
        match content {
            None => true,
            Some(text) => {
                // Lines to skip: empty, headers, HTML comments, checkboxes
                let skip_patterns = ["- [ ]", "* [ ]", "- [x]", "* [x]"];

                for line in text.lines() {
                    let trimmed = line.trim();

                    // Skip empty lines
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Skip headers
                    if trimmed.starts_with('#') {
                        continue;
                    }

                    // Skip HTML comments
                    if trimmed.starts_with("<!--") {
                        continue;
                    }

                    // Skip checkbox patterns
                    if skip_patterns.iter().any(|&p| trimmed == p) {
                        continue;
                    }

                    // Found actionable content
                    return false;
                }

                true
            }
        }
    }

    /// Start the heartbeat service
    pub async fn start(&self) -> Result<()> {
        if !self.enabled {
            info!("Heartbeat disabled");
            return Ok(());
        }

        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        info!("Heartbeat started (every {}s)", self.interval.as_secs());

        // Spawn the main loop
        let service = self.clone_for_task();
        tokio::spawn(async move {
            service.run_loop().await;
        });

        Ok(())
    }

    /// Stop the heartbeat service
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Heartbeat stopped");
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Main heartbeat loop
    async fn run_loop(&self) {
        loop {
            // Check if we should stop
            if !*self.running.read().await {
                break;
            }

            // Wait for the interval
            sleep(self.interval).await;

            // Check again after sleep
            if !*self.running.read().await {
                break;
            }

            // Execute a tick
            if let Err(e) = self.tick().await {
                error!("Heartbeat error: {}", e);
            }
        }
    }

    /// Execute a single heartbeat tick
    async fn tick(&self) -> Result<()> {
        let content = self.read_heartbeat_file();

        // Skip if HEARTBEAT.md is empty or doesn't exist
        if Self::is_heartbeat_empty(&content) {
            debug!("Heartbeat: no tasks (HEARTBEAT.md empty)");
            return Ok(());
        }

        info!("Heartbeat: checking for tasks...");

        if let Some(ref callback) = self.on_heartbeat {
            let response = callback(HEARTBEAT_PROMPT.to_string())
                .await
                .context("Heartbeat execution failed")?;

            // Check if agent said "nothing to do"
            let response_normalized = response.to_uppercase().replace('_', "");
            let token_normalized = HEARTBEAT_OK_TOKEN.replace('_', "");

            if response_normalized.contains(&token_normalized) {
                info!("Heartbeat: OK (no action needed)");
            } else {
                info!("Heartbeat: completed task");
            }
        }

        Ok(())
    }

    /// Manually trigger a heartbeat
    pub async fn trigger_now(&self) -> Result<Option<String>> {
        if let Some(ref callback) = self.on_heartbeat {
            let response = callback(HEARTBEAT_PROMPT.to_string())
                .await
                .context("Manual heartbeat execution failed")?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    /// Helper to clone fields needed for the async task
    fn clone_for_task(&self) -> Self {
        Self {
            workspace: self.workspace.clone(),
            on_heartbeat: self.on_heartbeat.clone(),
            interval: self.interval,
            enabled: self.enabled,
            running: Arc::clone(&self.running),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    #[test]
    fn test_is_heartbeat_empty() {
        // None should be empty
        assert!(HeartbeatService::is_heartbeat_empty(&None));

        // Empty string should be empty
        assert!(HeartbeatService::is_heartbeat_empty(&Some(String::new())));

        // Only whitespace should be empty
        assert!(HeartbeatService::is_heartbeat_empty(&Some(
            "   \n  \n  ".to_string()
        )));

        // Only headers should be empty
        assert!(HeartbeatService::is_heartbeat_empty(&Some(
            "# Header\n## Another".to_string()
        )));

        // Only checkboxes should be empty
        assert!(HeartbeatService::is_heartbeat_empty(&Some(
            "- [ ]\n* [ ]\n- [x]".to_string()
        )));

        // Content should not be empty
        assert!(!HeartbeatService::is_heartbeat_empty(&Some(
            "Do something".to_string()
        )));

        // Mixed content should not be empty
        assert!(!HeartbeatService::is_heartbeat_empty(&Some(
            "# Header\nDo something\n- [ ]".to_string()
        )));
    }

    #[tokio::test]
    async fn test_heartbeat_creation() {
        let temp_dir = TempDir::new().unwrap();
        let service = HeartbeatService::new(temp_dir.path());

        assert_eq!(service.workspace, temp_dir.path());
        assert!(service.enabled);
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_with_interval() {
        let temp_dir = TempDir::new().unwrap();
        let service = HeartbeatService::new(temp_dir.path()).with_interval(60);

        assert_eq!(service.interval, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_heartbeat_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let service = HeartbeatService::new(temp_dir.path()).with_enabled(false);

        service.start().await.unwrap();
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_with_callback() {
        let temp_dir = TempDir::new().unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let service = HeartbeatService::new(temp_dir.path()).with_callback(move |prompt| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                assert!(prompt.contains("HEARTBEAT.md"));
                Ok("HEARTBEAT_OK".to_string())
            }
        });

        // Create a HEARTBEAT.md file with content
        let heartbeat_path = temp_dir.path().join("HEARTBEAT.md");
        std::fs::write(&heartbeat_path, "Do something").unwrap();

        // Trigger manually
        let result = service.trigger_now().await.unwrap();
        assert_eq!(result, Some("HEARTBEAT_OK".to_string()));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_heartbeat_file_reading() {
        let temp_dir = TempDir::new().unwrap();
        let service = HeartbeatService::new(temp_dir.path());

        // No file exists
        assert_eq!(service.read_heartbeat_file(), None);

        // Create file
        let heartbeat_path = temp_dir.path().join("HEARTBEAT.md");
        std::fs::write(&heartbeat_path, "Test content").unwrap();

        // Read file
        let content = service.read_heartbeat_file();
        assert_eq!(content, Some("Test content".to_string()));
    }

    #[tokio::test]
    async fn test_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let service = HeartbeatService::new(temp_dir.path())
            .with_interval(1) // 1 second for testing
            .with_callback(|_| async { Ok("OK".to_string()) });

        assert!(!service.is_running().await);

        service.start().await.unwrap();
        assert!(service.is_running().await);

        service.stop().await;
        assert!(!service.is_running().await);
    }
}
