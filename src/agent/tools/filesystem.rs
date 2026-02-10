//! File system tools: read, write, edit, list directory.

use super::base::{get_optional_string, get_string_param, Tool};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Resolve path and optionally enforce directory restriction.
fn resolve_path(path: &str, allowed_dir: Option<&PathBuf>) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path);
    let resolved = if path_buf.is_absolute() {
        path_buf
    } else {
        std::env::current_dir()?.join(path_buf)
    };

    let canonical = resolved.canonicalize().unwrap_or(resolved);

    if let Some(allowed) = allowed_dir {
        let allowed_canonical = allowed.canonicalize()?;
        if !canonical.starts_with(&allowed_canonical) {
            return Err(anyhow!(
                "Path {} is outside allowed directory {:?}",
                path,
                allowed
            ));
        }
    }

    Ok(canonical)
}

/// Tool to read file contents.
pub struct ReadFileTool {
    allowed_dir: Option<PathBuf>,
}

impl ReadFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let path = get_string_param(&params, "path")
            .ok_or_else(|| anyhow!("Missing required parameter: path"))?;

        let file_path = match resolve_path(&path, self.allowed_dir.as_ref()) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e)),
        };

        if !file_path.exists() {
            return Ok(format!("Error: File not found: {}", path));
        }

        if !file_path.is_file() {
            return Ok(format!("Error: Not a file: {}", path));
        }

        match fs::read_to_string(&file_path).await {
            Ok(content) => Ok(content),
            Err(e) => Ok(format!("Error reading file: {}", e)),
        }
    }
}

/// Tool to write content to a file.
pub struct WriteFileTool {
    allowed_dir: Option<PathBuf>,
}

impl WriteFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file at the given path. Creates parent directories if needed."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to write to"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let path = get_string_param(&params, "path")
            .ok_or_else(|| anyhow!("Missing required parameter: path"))?;
        let content = get_string_param(&params, "content")
            .ok_or_else(|| anyhow!("Missing required parameter: content"))?;

        let file_path = match resolve_path(&path, self.allowed_dir.as_ref()) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e)),
        };

        if let Some(parent) = file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return Ok(format!("Error creating directories: {}", e));
            }
        }

        match fs::write(&file_path, &content).await {
            Ok(_) => Ok(format!(
                "Successfully wrote {} bytes to {}",
                content.len(),
                path
            )),
            Err(e) => Ok(format!("Error writing file: {}", e)),
        }
    }
}

/// Tool to edit a file by replacing text.
pub struct EditFileTool {
    allowed_dir: Option<PathBuf>,
}

impl EditFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing old_text with new_text. The old_text must exist exactly in the file."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to edit"
                },
                "old_text": {
                    "type": "string",
                    "description": "The exact text to find and replace"
                },
                "new_text": {
                    "type": "string",
                    "description": "The text to replace with"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let path = get_string_param(&params, "path")
            .ok_or_else(|| anyhow!("Missing required parameter: path"))?;
        let old_text = get_string_param(&params, "old_text")
            .ok_or_else(|| anyhow!("Missing required parameter: old_text"))?;
        let new_text = get_string_param(&params, "new_text")
            .ok_or_else(|| anyhow!("Missing required parameter: new_text"))?;

        let file_path = match resolve_path(&path, self.allowed_dir.as_ref()) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e)),
        };

        if !file_path.exists() {
            return Ok(format!("Error: File not found: {}", path));
        }

        let content = match fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => return Ok(format!("Error reading file: {}", e)),
        };

        if !content.contains(&old_text) {
            return Ok("Error: old_text not found in file. Make sure it matches exactly.".to_string());
        }

        let count = content.matches(&old_text).count();
        if count > 1 {
            return Ok(format!(
                "Warning: old_text appears {} times. Please provide more context to make it unique.",
                count
            ));
        }

        let new_content = content.replacen(&old_text, &new_text, 1);

        match fs::write(&file_path, new_content).await {
            Ok(_) => Ok(format!("Successfully edited {}", path)),
            Err(e) => Ok(format!("Error editing file: {}", e)),
        }
    }
}

/// Tool to list directory contents.
pub struct ListDirTool {
    allowed_dir: Option<PathBuf>,
}

impl ListDirTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List the contents of a directory."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Map<String, Value>) -> Result<String> {
        let path = get_string_param(&params, "path")
            .ok_or_else(|| anyhow!("Missing required parameter: path"))?;

        let dir_path = match resolve_path(&path, self.allowed_dir.as_ref()) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e)),
        };

        if !dir_path.exists() {
            return Ok(format!("Error: Directory not found: {}", path));
        }

        if !dir_path.is_dir() {
            return Ok(format!("Error: Not a directory: {}", path));
        }

        let mut entries = match fs::read_dir(&dir_path).await {
            Ok(e) => e,
            Err(e) => return Ok(format!("Error listing directory: {}", e)),
        };

        let mut items = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(file_type) = entry.file_type().await {
                let prefix = if file_type.is_dir() { "📁 " } else { "📄 " };
                if let Some(name) = entry.file_name().to_str() {
                    items.push(format!("{}{}", prefix, name));
                }
            }
        }

        if items.is_empty() {
            return Ok(format!("Directory {} is empty", path));
        }

        items.sort();
        Ok(items.join("\n"))
    }
}
