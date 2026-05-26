#![allow(clippy::disallowed_types)]

use super::map_miette_error;
use moon_app_context::AppContext;
use moon_codegen::CodeGenerator;
use moon_config::TemplateVariable;
use moon_process::{Command, output_to_trimmed_string};
use regex::Regex;
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;

async fn load_generator<'a>(
    app_context: &'a AppContext,
) -> Result<CodeGenerator<'a>, CallToolError> {
    let mut generator = CodeGenerator::new(
        &app_context.workspace_root,
        &app_context.workspace_config.generator,
        Arc::clone(&app_context.moon_env),
    );

    generator.load_templates().await.map_err(map_miette_error)?;

    Ok(generator)
}

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
pub struct GenerateTool {
    pub template: String,
    pub to: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<Variables>,
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub error: Option<String>,
    pub success: bool,
}

impl GenerateTool {
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

#[mcp_tool(
    name = "get_template",
    title = "Get template",
    description = "Describe a template's full variable schema, including types, defaults, \
                   prompts, required flags, and enum values. The `extends` chain is resolved \
                   so the returned schema matches what `generate` would actually prompt for. \
                   Internal variables are excluded."
)]
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct GetTemplateTool {
    pub id: String,
}

#[derive(Serialize)]
pub struct GetTemplateResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<String>,
    pub variables: BTreeMap<String, TemplateVariable>,
}

impl GetTemplateTool {
    pub async fn call_tool(
        &self,
        app_context: &AppContext,
    ) -> Result<CallToolResult, CallToolError> {
        let generator = load_generator(app_context).await?;
        let template = generator.get_template(&self.id).map_err(map_miette_error)?;

        let variables = template
            .config
            .variables
            .iter()
            .filter(|(_, var)| !var.is_internal())
            .map(|(name, var)| (name.clone(), var.clone()))
            .collect();

        let response = GetTemplateResponse {
            id: template.id.to_string(),
            title: template.config.title.clone(),
            description: template.config.description.clone(),
            destination: template.config.destination.clone(),
            extends: template
                .config
                .extends
                .to_list()
                .iter()
                .map(|id| id.to_string())
                .collect(),
            variables,
        };

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&response).map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}

#[mcp_tool(
    name = "get_templates",
    title = "Get templates",
    description = "List all available templates with their id, title, and description. \
                   Use this to discover templates before calling `generate`."
)]
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct GetTemplatesTool {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

#[derive(Serialize)]
pub struct TemplateSummary {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct GetTemplatesResponse {
    pub templates: Vec<TemplateSummary>,
}

impl GetTemplatesTool {
    pub async fn call_tool(
        &self,
        app_context: &AppContext,
    ) -> Result<CallToolResult, CallToolError> {
        let generator = load_generator(app_context).await?;

        let pattern = match self.filter.as_deref() {
            Some(filter) => Some(
                Regex::new(&format!("(?i){filter}"))
                    .map_err(|err| CallToolError::new(std::io::Error::other(err.to_string())))?,
            ),
            None => None,
        };

        let mut templates: Vec<TemplateSummary> = generator
            .templates
            .iter()
            .filter(|(id, _)| {
                pattern
                    .as_ref()
                    .is_none_or(|pattern| pattern.is_match(id.as_str()))
            })
            .map(|(id, template)| TemplateSummary {
                id: id.to_string(),
                title: template.config.title.clone(),
                description: template.config.description.clone(),
            })
            .collect();

        templates.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(CallToolResult::text_content(vec![TextContent::new(
            serde_json::to_string_pretty(&GetTemplatesResponse { templates })
                .map_err(CallToolError::new)?,
            None,
            None,
        )]))
    }
}
