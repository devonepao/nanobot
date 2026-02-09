//! Utility functions for nanobot.
//!
//! This module provides helper functions for path management, string operations,
//! and date/time formatting used across the nanobot project.

use chrono::Local;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Ensure a directory exists, creating it if necessary.
///
/// # Arguments
///
/// * `path` - The directory path to ensure exists
///
/// # Returns
///
/// The path that was ensured to exist
///
/// # Errors
///
/// Returns an error if the directory cannot be created
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use nanobot::utils::ensure_dir;
///
/// let path = PathBuf::from("/tmp/test");
/// let result = ensure_dir(&path).unwrap();
/// assert_eq!(result, path);
/// ```
pub fn ensure_dir(path: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(path)?;
    Ok(path.to_path_buf())
}

/// Get the nanobot data directory (~/.nanobot).
///
/// # Returns
///
/// Path to the nanobot data directory
///
/// # Errors
///
/// Returns an error if the directory cannot be created or the home directory cannot be determined
///
/// # Examples
///
/// ```no_run
/// use nanobot::utils::get_data_path;
///
/// let data_path = get_data_path().unwrap();
/// println!("Data path: {}", data_path.display());
/// ```
pub fn get_data_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory")
    })?;
    let path = home.join(".nanobot");
    ensure_dir(&path)
}

/// Get the workspace path.
///
/// # Arguments
///
/// * `workspace` - Optional workspace path. Defaults to ~/.nanobot/workspace.
///
/// # Returns
///
/// Expanded and ensured workspace path
///
/// # Errors
///
/// Returns an error if the directory cannot be created or the home directory cannot be determined
///
/// # Examples
///
/// ```no_run
/// use nanobot::utils::get_workspace_path;
///
/// let workspace = get_workspace_path(None).unwrap();
/// println!("Workspace: {}", workspace.display());
///
/// let custom_workspace = get_workspace_path(Some("/tmp/workspace")).unwrap();
/// ```
pub fn get_workspace_path(workspace: Option<&str>) -> io::Result<PathBuf> {
    let path = if let Some(ws) = workspace {
        let ws_path = PathBuf::from(ws);
        // Expand tilde (~) in path
        if ws_path.starts_with("~") {
            let home = dirs::home_dir().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory")
            })?;
            home.join(ws_path.strip_prefix("~").expect("Path starts with ~"))
        } else {
            ws_path
        }
    } else {
        let home = dirs::home_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory")
        })?;
        home.join(".nanobot").join("workspace")
    };
    ensure_dir(&path)
}

/// Get the sessions storage directory.
///
/// # Returns
///
/// Path to the sessions directory
///
/// # Errors
///
/// Returns an error if the directory cannot be created
///
/// # Examples
///
/// ```no_run
/// use nanobot::utils::get_sessions_path;
///
/// let sessions = get_sessions_path().unwrap();
/// ```
pub fn get_sessions_path() -> io::Result<PathBuf> {
    let data_path = get_data_path()?;
    let path = data_path.join("sessions");
    ensure_dir(&path)
}

/// Get the memory directory within the workspace.
///
/// # Arguments
///
/// * `workspace` - Optional workspace path. If None, uses default workspace.
///
/// # Returns
///
/// Path to the memory directory
///
/// # Errors
///
/// Returns an error if the directory cannot be created
///
/// # Examples
///
/// ```no_run
/// use nanobot::utils::get_memory_path;
///
/// let memory = get_memory_path(None).unwrap();
/// ```
pub fn get_memory_path(workspace: Option<&Path>) -> io::Result<PathBuf> {
    let ws = if let Some(w) = workspace {
        w.to_path_buf()
    } else {
        get_workspace_path(None)?
    };
    let path = ws.join("memory");
    ensure_dir(&path)
}

/// Get the skills directory within the workspace.
///
/// # Arguments
///
/// * `workspace` - Optional workspace path. If None, uses default workspace.
///
/// # Returns
///
/// Path to the skills directory
///
/// # Errors
///
/// Returns an error if the directory cannot be created
///
/// # Examples
///
/// ```no_run
/// use nanobot::utils::get_skills_path;
///
/// let skills = get_skills_path(None).unwrap();
/// ```
pub fn get_skills_path(workspace: Option<&Path>) -> io::Result<PathBuf> {
    let ws = if let Some(w) = workspace {
        w.to_path_buf()
    } else {
        get_workspace_path(None)?
    };
    let path = ws.join("skills");
    ensure_dir(&path)
}

/// Get today's date in YYYY-MM-DD format.
///
/// # Returns
///
/// Today's date as a string
///
/// # Examples
///
/// ```
/// use nanobot::utils::today_date;
///
/// let date = today_date();
/// assert_eq!(date.len(), 10); // YYYY-MM-DD format
/// ```
pub fn today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Get current timestamp in ISO 8601 format.
///
/// # Returns
///
/// Current timestamp as a string
///
/// # Examples
///
/// ```
/// use nanobot::utils::timestamp;
///
/// let ts = timestamp();
/// assert!(!ts.is_empty());
/// ```
pub fn timestamp() -> String {
    Local::now().to_rfc3339()
}

/// Truncate a string to max length, adding suffix if truncated.
///
/// # Arguments
///
/// * `s` - The string to truncate
/// * `max_len` - Maximum length in characters (not bytes)
/// * `suffix` - Suffix to add if truncated (default: "...")
///
/// # Returns
///
/// Truncated string
///
/// # Examples
///
/// ```
/// use nanobot::utils::truncate_string;
///
/// let short = truncate_string("hello", 100, "...");
/// assert_eq!(short, "hello");
///
/// let long = truncate_string("hello world", 8, "...");
/// assert_eq!(long, "hello...");
/// ```
pub fn truncate_string(s: &str, max_len: usize, suffix: &str) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let suffix_len = suffix.chars().count();
        let truncate_at = max_len.saturating_sub(suffix_len);
        let truncated: String = s.chars().take(truncate_at).collect();
        let mut result = String::with_capacity(truncated.len() + suffix.len());
        result.push_str(&truncated);
        result.push_str(suffix);
        result
    }
}

/// Convert a string to a safe filename.
///
/// Replaces unsafe characters with underscores and trims whitespace.
///
/// # Arguments
///
/// * `name` - The string to convert
///
/// # Returns
///
/// Safe filename string
///
/// # Examples
///
/// ```
/// use nanobot::utils::safe_filename;
///
/// let safe = safe_filename("hello/world.txt");
/// assert_eq!(safe, "hello_world.txt");
///
/// let safe2 = safe_filename("file:name<>?");
/// assert_eq!(safe2, "file_name___");
/// ```
pub fn safe_filename(name: &str) -> String {
    let unsafe_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    name.chars()
        .map(|c| if unsafe_chars.contains(&c) { '_' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Parse a session key into channel and chat_id.
///
/// # Arguments
///
/// * `key` - Session key in format "channel:chat_id"
///
/// # Returns
///
/// Tuple of (channel, chat_id)
///
/// # Errors
///
/// Returns an error if the key format is invalid
///
/// # Examples
///
/// ```
/// use nanobot::utils::parse_session_key;
///
/// let (channel, chat_id) = parse_session_key("telegram:12345").unwrap();
/// assert_eq!(channel, "telegram");
/// assert_eq!(chat_id, "12345");
///
/// let result = parse_session_key("invalid");
/// assert!(result.is_err());
/// ```
pub fn parse_session_key(key: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = key.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid session key: {}", key));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_date() {
        let date = today_date();
        assert_eq!(date.len(), 10);
        assert!(date.contains('-'));
    }

    #[test]
    fn test_timestamp() {
        let ts = timestamp();
        assert!(!ts.is_empty());
        assert!(ts.contains('T') || ts.contains('+') || ts.contains('-'));
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 100, "..."), "hello");
        assert_eq!(truncate_string("hello world", 8, "..."), "hello...");
        assert_eq!(truncate_string("test", 5, "..."), "test");
        assert_eq!(truncate_string("hello world", 5, "..."), "he...");
    }

    #[test]
    fn test_safe_filename() {
        assert_eq!(safe_filename("hello.txt"), "hello.txt");
        assert_eq!(safe_filename("hello/world.txt"), "hello_world.txt");
        assert_eq!(safe_filename("file:name"), "file_name");
        assert_eq!(safe_filename("test<>file"), "test__file");
        assert_eq!(safe_filename("  spaces  "), "spaces");
    }

    #[test]
    fn test_parse_session_key() {
        let (channel, chat_id) = parse_session_key("telegram:12345").unwrap();
        assert_eq!(channel, "telegram");
        assert_eq!(chat_id, "12345");

        let (channel2, chat_id2) = parse_session_key("discord:user:67890").unwrap();
        assert_eq!(channel2, "discord");
        assert_eq!(chat_id2, "user:67890");

        assert!(parse_session_key("invalid").is_err());
        assert!(parse_session_key("").is_err());
    }

    #[test]
    fn test_ensure_dir() {
        use std::env;
        let temp = env::temp_dir().join("nanobot_test_ensure_dir");
        let result = ensure_dir(&temp);
        assert!(result.is_ok());
        assert!(temp.exists());
        assert!(temp.is_dir());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_get_workspace_path() {
        let default_ws = get_workspace_path(None);
        assert!(default_ws.is_ok());

        let custom_ws = get_workspace_path(Some("/tmp/test_workspace"));
        assert!(custom_ws.is_ok());
    }
}
