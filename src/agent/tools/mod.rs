//! Agent tools module.

pub mod base;
pub mod cron;
pub mod filesystem;
pub mod message;
pub mod registry;
pub mod shell;
pub mod spawn;
pub mod web;

pub use base::Tool;
pub use cron::CronTool;
pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use message::MessageTool;
pub use registry::ToolRegistry;
pub use shell::ExecTool;
pub use spawn::SpawnTool;
pub use web::{WebFetchTool, WebSearchTool};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();

        let read_tool = Arc::new(ReadFileTool::new(None));
        registry.register(read_tool);

        assert!(registry.has("read_file"));
        assert_eq!(registry.len(), 1);

        let definitions = registry.get_definitions();
        assert_eq!(definitions.len(), 1);
    }

    #[tokio::test]
    async fn test_tool_validation() {
        let read_tool = ReadFileTool::new(None);

        // Valid params
        let valid_params = json!({"path": "/tmp/test.txt"});
        let errors = read_tool.validate_params(&valid_params);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);

        // Invalid params - missing required field
        let invalid_params = json!({});
        let errors = read_tool.validate_params(&invalid_params);
        assert!(!errors.is_empty());
    }

    #[tokio::test]
    async fn test_file_tool_execution() {
        use tempfile::NamedTempFile;
        use tokio::fs;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();

        // Write tool
        let write_tool = WriteFileTool::new(None);
        let params = json!({"path": path, "content": "Hello, world!"})
            .as_object()
            .unwrap()
            .clone();
        let result = write_tool.execute(params).await.unwrap();
        assert!(result.contains("Successfully wrote"));

        // Read tool
        let read_tool = ReadFileTool::new(None);
        let params = json!({"path": path}).as_object().unwrap().clone();
        let result = read_tool.execute(params).await.unwrap();
        assert_eq!(result, "Hello, world!");

        // Edit tool
        let edit_tool = EditFileTool::new(None);
        let params = json!({
            "path": path,
            "old_text": "world",
            "new_text": "Rust"
        })
        .as_object()
        .unwrap()
        .clone();
        let result = edit_tool.execute(params).await.unwrap();
        assert!(result.contains("Successfully edited"));

        // Verify edit
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "Hello, Rust!");
    }
}

