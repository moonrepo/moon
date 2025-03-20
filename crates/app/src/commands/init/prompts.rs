use super::InitOptions;
use iocraft::prelude::element;
use moon_console::{
    Console,
    ui::{Confirm, Input, Select, SelectOption},
};
use moon_pdk_api::{PromptType, SettingPrompt};
use proto_core::UnresolvedVersionSpec;
use starbase_utils::json::JsonValue;

pub fn fully_qualify_version(version: String) -> Option<String> {
    if version.is_empty() {
        return None;
    }

    let mut parts = version.split('.');

    Some(
        [
            parts.next().unwrap_or("0"),
            parts.next().unwrap_or("0"),
            parts.next().unwrap_or("0"),
        ]
        .join("."),
    )
}

pub async fn render_prompt(
    console: &Console,
    options: &InitOptions,
    prompt: &SettingPrompt,
) -> miette::Result<Option<JsonValue>> {
    match &prompt.ty {
        PromptType::None => Ok(None),
        PromptType::Confirm { default } => {
            let result = if options.yes {
                *default
            } else {
                let mut value = *default;

                console
                    .render_interactive(element! {
                        Confirm(
                            label: &prompt.question,
                            description: prompt.description.clone(),
                            on_confirm: &mut value
                        )
                    })
                    .await?;

                value
            };

            Ok(Some(JsonValue::Bool(result)))
        }
        PromptType::Input { default } => {
            let result = if options.yes {
                default.to_owned()
            } else {
                let mut value = default.to_owned();
                let required = prompt.required;

                console
                    .render_interactive(element! {
                        Input(
                            label: &prompt.question,
                            description: prompt.description.clone(),
                            default_value: default,
                            on_value: &mut value,
                            validate: move |input: String| {
                                if input.is_empty() && required {
                                    Some("Please provide a value".into())
                                } else {
                                    None
                                }
                            }
                        )
                    })
                    .await?;

                value
            };

            Ok(Some(JsonValue::String(result)))
        }
        PromptType::Select {
            default_index,
            options: items,
        } => {
            let index = if options.yes {
                *default_index
            } else {
                let mut index = *default_index;

                console
                    .render_interactive(element! {
                        Select(
                            label: &prompt.question,
                            description: prompt.description.clone(),
                            default_index: *default_index,
                            on_index: &mut index,
                            options: items.iter().map(|i| SelectOption::new(display_json_value(i))).collect::<Vec<_>>()
                        )
                    })
                    .await?;

                index
            };

            Ok(Some(items[index].clone()))
        }
    }
}

pub async fn render_version_prompt(
    console: &Console,
    options: &InitOptions,
    tool: &str,
    op: impl FnOnce() -> miette::Result<Option<UnresolvedVersionSpec>>,
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    let default_version = op()?;

    if options.yes || options.minimal {
        return Ok(default_version);
    }

    let mut confirmed = false;
    let mut value = String::new();

    console
        .render_interactive(element! {
            Confirm(
                label: if let Some(version) = &default_version {
                    format!(
                        "Manage {tool} {version} through <shell>moon</shell>? <muted>(recommended)</muted>"
                    )
                } else {
                    format!(
                        "Manage {tool} through <shell>moon</shell>? <muted>(recommended)</muted>"
                    )
                },
                description: "Will download and install on-demand.".to_string(),
                on_confirm: &mut confirmed,
            )
        })
        .await?;

    if confirmed {
        console
            .render_interactive(element! {
                Input(
                    label: format!("{tool} version?"),
                    default_value: default_version.map(|v| v.to_string()).unwrap_or_default(),
                    on_value: &mut value,
                    validate: move |input: String| {
                        if input.trim().is_empty() {
                            Some("Please provide a version".into())
                        } else if let Err(error) = UnresolvedVersionSpec::parse(&input) {
                            Some(error.to_string())
                        } else {
                            None
                        }
                    }
                )
            })
            .await?;
    }

    Ok(if value.is_empty() {
        None
    } else {
        UnresolvedVersionSpec::parse(value).ok()
    })
}

pub fn display_json_value(value: &JsonValue) -> String {
    match value {
        // Remove quotes
        JsonValue::String(string) => string.to_owned(),
        other => other.to_string(),
    }
}
