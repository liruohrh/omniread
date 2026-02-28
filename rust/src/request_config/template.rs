use crate::request_config::error::{RequestConfigError, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

pub struct TemplateEngine;

impl TemplateEngine {
    pub fn render_string(template: &str, variables: &HashMap<String, String>) -> Result<String> {
        let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();
        let mut result = template.to_string();
        
        for cap in re.captures_iter(template) {
            let var_name = &cap[1];
            if let Some(value) = variables.get(var_name) {
                result = result.replace(&format!("{{{{{}}}}}", var_name), value);
            } else {
                return Err(RequestConfigError::VariableNotFound(var_name.to_string()));
            }
        }
        
        Ok(result)
    }

    pub fn render_json(value: &mut Value, variables: &HashMap<String, String>) -> Result<()> {
        match value {
            Value::String(s) => {
                *s = Self::render_string(s, variables)?;
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::render_json(item, variables)?;
                }
            }
            Value::Object(obj) => {
                for (_key, val) in obj {
                    Self::render_json(val, variables)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn extract_variables(template: &str) -> Vec<String> {
        let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();
        re.captures_iter(template)
            .map(|cap| cap[1].to_string())
            .collect()
    }
}
