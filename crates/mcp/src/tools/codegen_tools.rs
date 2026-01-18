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

#[mcp_tool(
    name = "generate",
    title = "Generate",
    description = "Generate code or scaffold from a template."
)]
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct Generate {
    pub template: String,
    pub to: String,

    #[serde(default)]
    pub dry_run: bool,

    #[serde(default)]
    pub force: bool,

    #[serde(default)]
    pub variables: FxHashMap<String, serde_json::Value>,
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

        if self.dry_run {
            args.push("--dry-run".into());
        } else if self.force {
            args.push("--force".into());
        }

        if !self.variables.is_empty() {
            args.push("--".into());

            for (key, value) in &self.variables {
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
