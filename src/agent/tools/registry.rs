//! Tool registry for dynamic tool management.

use super::base::Tool;
use anyhow::Result;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for agent tools.
///
/// Allows dynamic registration and execution of tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Unregister a tool by name.
    pub fn unregister(&mut self, name: &str) {
        self.tools.remove(name);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool is registered.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool definitions in OpenAI format.
    pub fn get_definitions(&self) -> Vec<Value> {
        self.tools.values().map(|tool| tool.to_schema()).collect()
    }

    /// Execute a tool by name with given parameters.
    ///
    /// Returns the tool execution result as a string.
    pub async fn execute(&self, name: &str, params: Value) -> String {
        let tool = match self.tools.get(name) {
            Some(tool) => tool,
            None => return format!("Error: Tool '{}' not found", name),
        };

        let params_obj = match params.as_object() {
            Some(obj) => obj.clone(),
            None => {
                return format!(
                    "Error: Invalid parameters for tool '{}': expected object",
                    name
                )
            }
        };

        // Validate parameters
        let errors = tool.validate_params(&params);
        if !errors.is_empty() {
            return format!(
                "Error: Invalid parameters for tool '{}': {}",
                name,
                errors.join("; ")
            );
        }

        // Execute tool
        match tool.execute(params_obj).await {
            Ok(result) => result,
            Err(e) => format!("Error executing {}: {}", name, e),
        }
    }

    /// Get list of registered tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
