#![allow(clippy::disallowed_types)]

use super::map_miette_error;
use moon_app_context::AppContext;
use moon_process::{Command, output_to_trimmed_string};
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;

/// Newtype wrapper around `FxHashMap<String, Value>` that provides a valid
/// JSON Schema (`"type": "object"`) instead of the `"type": "unknown"` that
/// the `rust-mcp-macros` `JsonSchema` derive emits for unrecognized generic types.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Variables(pub FxHashMap<String, serde_json::Value>);

impl Variables {
    pub fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map
    }
}

impl Deref for Variables {
    type Target = FxHashMap<String, serde_json::Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[mcp_tool(
    name = "generate",
    title = "Generate",
    description = "Generate code or scaffold from a template."
)]
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct Generate {
    pub template: String,
    pub to: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<Variables>,
}

impl Generate {
    pub async fn call_tool(
        &self,
        app_context: &AppContext,
    ) -> Result<CallToolResult, CallToolError> {
        let mut args = vec![
            "generate".into(),
            self.template.clone(),
            "--to".into(),
            self.to.clone(),
        ];

        if self.dry_run.unwrap_or_default() {
            args.push("--dry-run".into());
        } else if self.force.unwrap_or_default() {
            args.push("--force".into());
        }

        if let Some(vars) = &self.variables
            && !vars.is_empty()
        {
            args.push("--".into());

            for (key, value) in vars.iter() {
                let opt = format!("--{key}");

                match value {
                    Value::Null | Value::Object(_) => {
                        // Skip
                    }
                    Value::Bool(inner) => {
                        if *inner {
                            args.push(opt);
                        } else {
                            args.push(format!("--no-{key}"));
                        }
                    }
                    Value::Number(inner) => {
                        args.push(opt);
                        args.push(inner.to_string());
                    }
                    Value::String(_) => {
                        args.push(opt);
                        // Wraps in quotes and escapes
                        args.push(value.to_string());
                    }
                    Value::Array(inner) => {
                        for item in inner {
                            args.push(opt.clone());
                            args.push(item.to_string());
                        }
                    }
                }
            }
        }

        let output = Command::new("moon")
            .args(args)
            .cwd(&app_context.workspace_root)
            .exec_capture_output()
            .await
            .map_err(map_miette_error)?;

        let mut response = GenerateResponse {
            error: None,
            success: true,
        };

        if !output.success() {
            response.success = false;
            response.error = Some(output_to_trimmed_string(&output.stderr));
        }

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&response).map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub error: Option<String>,
    pub success: bool,
}
