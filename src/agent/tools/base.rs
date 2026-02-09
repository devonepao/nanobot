//! Base tool trait and validation logic.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Tool trait for agent capabilities.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name used in function calls.
    fn name(&self) -> &str;

    /// Description of what the tool does.
    fn description(&self) -> &str;

    /// JSON Schema for tool parameters.
    fn parameters(&self) -> Value;

    /// Execute the tool with given parameters.
    async fn execute(&self, params: Map<String, Value>) -> Result<String>;

    /// Validate tool parameters against JSON schema.
    fn validate_params(&self, params: &Value) -> Vec<String> {
        let schema = self.parameters();
        if let Value::Object(schema_obj) = &schema {
            if schema_obj.get("type").and_then(|v| v.as_str()) != Some("object") {
                return vec![format!(
                    "Schema must be object type, got {:?}",
                    schema_obj.get("type")
                )];
            }
            self.validate_value(params, &schema, "")
        } else {
            vec!["Schema must be an object".to_string()]
        }
    }

    /// Convert tool to OpenAI function schema format.
    fn to_schema(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters(),
            }
        })
    }

    /// Internal validation logic.
    fn validate_value(&self, val: &Value, schema: &Value, path: &str) -> Vec<String> {
        let mut errors = Vec::new();
        let schema_obj = match schema.as_object() {
            Some(obj) => obj,
            None => return errors,
        };

        let type_str = schema_obj
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("any");
        let label = if path.is_empty() {
            "parameter".to_string()
        } else {
            path.to_string()
        };

        // Type validation
        match type_str {
            "string" => {
                if !val.is_string() {
                    errors.push(format!("{} should be string", label));
                    return errors;
                }
                let s = val.as_str().unwrap();

                if let Some(min_len) = schema_obj.get("minLength").and_then(|v| v.as_u64()) {
                    if s.len() < min_len as usize {
                        errors.push(format!(
                            "{} must be at least {} chars",
                            label, min_len
                        ));
                    }
                }
                if let Some(max_len) = schema_obj.get("maxLength").and_then(|v| v.as_u64()) {
                    if s.len() > max_len as usize {
                        errors.push(format!("{} must be at most {} chars", label, max_len));
                    }
                }
            }
            "integer" => {
                if !val.is_i64() && !val.is_u64() {
                    errors.push(format!("{} should be integer", label));
                    return errors;
                }
                let num = val.as_i64().unwrap_or(0);

                if let Some(min) = schema_obj.get("minimum").and_then(|v| v.as_i64()) {
                    if num < min {
                        errors.push(format!("{} must be >= {}", label, min));
                    }
                }
                if let Some(max) = schema_obj.get("maximum").and_then(|v| v.as_i64()) {
                    if num > max {
                        errors.push(format!("{} must be <= {}", label, max));
                    }
                }
            }
            "number" => {
                if !val.is_number() {
                    errors.push(format!("{} should be number", label));
                    return errors;
                }
                let num = val.as_f64().unwrap_or(0.0);

                if let Some(min) = schema_obj.get("minimum").and_then(|v| v.as_f64()) {
                    if num < min {
                        errors.push(format!("{} must be >= {}", label, min));
                    }
                }
                if let Some(max) = schema_obj.get("maximum").and_then(|v| v.as_f64()) {
                    if num > max {
                        errors.push(format!("{} must be <= {}", label, max));
                    }
                }
            }
            "boolean" => {
                if !val.is_boolean() {
                    errors.push(format!("{} should be boolean", label));
                    return errors;
                }
            }
            "array" => {
                if !val.is_array() {
                    errors.push(format!("{} should be array", label));
                    return errors;
                }
                if let Some(items_schema) = schema_obj.get("items") {
                    if let Some(arr) = val.as_array() {
                        for (i, item) in arr.iter().enumerate() {
                            let item_path = if path.is_empty() {
                                format!("[{}]", i)
                            } else {
                                format!("{}[{}]", path, i)
                            };
                            errors.extend(self.validate_value(item, items_schema, &item_path));
                        }
                    }
                }
            }
            "object" => {
                if !val.is_object() {
                    errors.push(format!("{} should be object", label));
                    return errors;
                }
                let obj = val.as_object().unwrap();
                let properties = schema_obj.get("properties").and_then(|v| v.as_object());
                let required = schema_obj
                    .get("required")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                // Check required fields
                for req_field in required {
                    if !obj.contains_key(req_field) {
                        let field_path = if path.is_empty() {
                            req_field.to_string()
                        } else {
                            format!("{}.{}", path, req_field)
                        };
                        errors.push(format!("missing required {}", field_path));
                    }
                }

                // Validate present fields
                if let Some(props) = properties {
                    for (key, value) in obj {
                        if let Some(prop_schema) = props.get(key) {
                            let field_path = if path.is_empty() {
                                key.to_string()
                            } else {
                                format!("{}.{}", path, key)
                            };
                            errors.extend(self.validate_value(value, prop_schema, &field_path));
                        }
                    }
                }
            }
            _ => {}
        }

        // Enum validation
        if let Some(enum_vals) = schema_obj.get("enum").and_then(|v| v.as_array()) {
            if !enum_vals.contains(val) {
                errors.push(format!("{} must be one of {:?}", label, enum_vals));
            }
        }

        errors
    }
}

/// Helper to extract string parameter from JSON map.
pub fn get_string_param(params: &Map<String, Value>, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).map(String::from)
}

/// Helper to extract optional string parameter.
pub fn get_optional_string(params: &Map<String, Value>, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Helper to extract integer parameter.
pub fn get_int_param(params: &Map<String, Value>, key: &str) -> Option<i64> {
    params.get(key).and_then(|v| v.as_i64())
}

/// Helper to extract boolean parameter.
pub fn get_bool_param(params: &Map<String, Value>, key: &str) -> Option<bool> {
    params.get(key).and_then(|v| v.as_bool())
}
