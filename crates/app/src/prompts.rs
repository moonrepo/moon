use crate::app_error::AppError;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_common::Id;
use moon_console::{Console, ui::*};
use moon_pdk_api::{
    ConditionType, InitializePluginOutput, PromptType, SettingCondition, SettingPrompt,
};
use moon_process::ProcessRegistry;
use moon_task::Target;
use proto_core::UnresolvedVersionSpec;
use starbase_utils::json::{JsonMap, JsonValue};
use std::collections::VecDeque;

async fn select_identifiers_internal<'a, T: Clone>(
    console: &Console,
    ids: Vec<&T>,
    input: impl FnOnce() -> miette::Result<SelectProps<'a>>,
    output: impl Fn(String) -> miette::Result<T>,
) -> miette::Result<Vec<T>> {
    if !ids.is_empty() {
        return Ok(ids.into_iter().cloned().collect());
    }

    if !console.out.is_terminal() {
        return Err(AppError::RequiredIdNonTTY.into());
    }

    let mut index = 0;
    let mut indexes = vec![];
    let mut props = input()?;

    props.options.sort_by(|a, d| {
        let av = a.label.as_ref().unwrap_or(&a.value);
        let dv = d.label.as_ref().unwrap_or(&d.value);

        av.cmp(dv)
    });

    console
        .render_prompt(element! {
            Select(
                label: props.label,
                description: props.description,
                options: props.options.clone(),
                multiple: props.multiple,
                on_index: &mut index,
                on_indexes: &mut indexes,
            )
        })
        .await?;

    if let Ok(signal) = ProcessRegistry::instance().receive_signal().try_recv() {
        std::process::exit(128 + signal.get_code());
    }

    let mut ids = vec![];

    if props.multiple {
        for index in indexes {
            ids.push(output(props.options.get(index).cloned().unwrap().value)?);
        }
    } else {
        ids.push(output(props.options.remove(index).value)?);
    }

    Ok(ids)
}

pub async fn select_identifier<'a>(
    console: &Console,
    id: &'a Option<Id>,
    input: impl FnOnce() -> miette::Result<SelectProps<'a>>,
) -> miette::Result<Id> {
    select_identifiers_internal(
        console,
        id.as_ref().map_or(vec![], |id| vec![id]),
        input,
        |value| Id::new(value).into_diagnostic(),
    )
    .await
    .map(|mut ids| ids.remove(0))
}

pub async fn select_identifiers<'a>(
    console: &Console,
    ids: &'a [Id],
    input: impl FnOnce() -> miette::Result<SelectProps<'a>>,
) -> miette::Result<Vec<Id>> {
    select_identifiers_internal(console, Vec::from_iter(ids), input, |value| {
        Id::new(value).into_diagnostic()
    })
    .await
}

pub async fn select_target<'a>(
    console: &Console,
    target: &'a Option<Target>,
    input: impl FnOnce() -> miette::Result<SelectProps<'a>>,
) -> miette::Result<Target> {
    select_identifiers_internal(
        console,
        target.as_ref().map_or(vec![], |target| vec![target]),
        input,
        |value| Target::parse(&value),
    )
    .await
    .map(|mut targets| targets.remove(0))
}

pub async fn select_targets<'a>(
    console: &Console,
    targets: &'a [Target],
    input: impl FnOnce() -> miette::Result<SelectProps<'a>>,
) -> miette::Result<Vec<Target>> {
    select_identifiers_internal(console, Vec::from_iter(targets), input, |value| {
        Target::parse(&value)
    })
    .await
}

pub async fn render_prompt(
    console: &Console,
    skip_prompts: bool,
    prompt: &SettingPrompt,
) -> miette::Result<Option<JsonValue>> {
    match &prompt.ty {
        PromptType::None => Ok(None),
        PromptType::Confirm { default } => {
            let result = if skip_prompts {
                *default
            } else {
                let mut value = *default;

                console
                    .render_prompt(element! {
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
            let result = if skip_prompts {
                default.to_owned()
            } else {
                let mut value = default.to_owned();
                let required = prompt.required;

                console
                    .render_prompt(element! {
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
            let index = if skip_prompts {
                *default_index
            } else {
                let mut index = *default_index;

                console
                    .render_prompt(element! {
                        Select(
                            label: &prompt.question,
                            description: prompt.description.clone(),
                            default_index: *default_index,
                            on_index: &mut index,
                            options: items
                                .iter()
                                .map(|i| SelectOption::new(display_json_value(i)))
                                .collect::<Vec<_>>()
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
    skip_prompts: bool,
    toolchain: &str,
    op: impl FnOnce() -> miette::Result<Option<UnresolvedVersionSpec>>,
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    let default_version = op()?;

    if skip_prompts {
        return Ok(default_version);
    }

    let mut confirmed = false;
    let mut value = String::new();

    console
        .render_prompt(element! {
            Confirm(
                label: if let Some(version) = &default_version {
                    format!(
                        "Manage {toolchain} {version} through <shell>moon</shell>? <muted>(recommended)</muted>"
                    )
                } else {
                    format!(
                        "Manage {toolchain} through <shell>moon</shell>? <muted>(recommended)</muted>"
                    )
                },
                description: "Will download and install on-demand.".to_string(),
                on_confirm: &mut confirmed,
            )
        })
        .await?;

    if confirmed {
        console
            .render_prompt(element! {
                Input(
                    label: format!("{toolchain} version?"),
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

pub async fn evaluate_plugin_initialize_prompts(
    console: &Console,
    title: &str,
    plugin: &str,
    url: &str,
    output: InitializePluginOutput,
    minimal: bool,
    skip_prompts: bool,
) -> miette::Result<JsonMap<String, JsonValue>> {
    if !skip_prompts {
        console.render(element! {
            Container {
                Section(title) {
                    Entry(
                        name: plugin,
                        value: element! {
                            StyledText(
                                content: url,
                                style: Style::Url
                            )
                        }.into_any()
                    )
                    #(output.docs_url.as_ref().map(|url| {
                        element! {
                            Entry(
                                name: "Handbook",
                                value: element! {
                                    StyledText(
                                        content: url,
                                        style: Style::Url
                                    )
                                }.into_any()
                            )
                        }
                    }))
                    #(output.config_url.as_ref().map(|url| {
                        element! {
                            Entry(
                                name: "Config",
                                value: element! {
                                    StyledText(
                                        content: url,
                                        style: Style::Url
                                    )
                                }.into_any()
                            )
                        }
                    }))
                }
            }
        })?;
    }

    let mut settings = JsonMap::from_iter(output.default_settings);

    evaluate_prompts(
        console,
        &output.prompts,
        &mut settings,
        minimal,
        skip_prompts,
    )
    .await?;

    Ok(settings)
}

pub async fn evaluate_prompts(
    console: &Console,
    prompts: &[SettingPrompt],
    settings: &mut JsonMap<String, JsonValue>,
    minimal: bool,
    skip_prompts: bool,
) -> miette::Result<()> {
    for prompt in prompts
        .iter()
        .filter(|p| if minimal { p.minimal } else { true })
    {
        if let Some(condition) = &prompt.condition
            && !evaluate_condition(condition, settings)
        {
            continue;
        }

        if let Some(value) = render_prompt(console, skip_prompts, prompt).await? {
            let falsy = is_json_falsy(&value);

            if prompt.skip_if_falsy && falsy {
                continue;
            }

            inject_setting(prompt.setting.clone(), value, settings);

            if !falsy {
                Box::pin(evaluate_prompts(
                    console,
                    &prompt.prompts,
                    settings,
                    minimal,
                    skip_prompts,
                ))
                .await?;
            }
        }
    }

    Ok(())
}

pub fn evaluate_condition(
    condition: &SettingCondition,
    settings: &JsonMap<String, JsonValue>,
) -> bool {
    let Some(value) = settings.get(&condition.setting) else {
        return condition.op == ConditionType::NotExists;
    };

    match (&condition.op, value) {
        (ConditionType::BoolEquals(expected), JsonValue::Bool(actual)) => expected == actual,
        (ConditionType::FloatEquals(expected), JsonValue::Number(number)) => {
            number.as_f64().is_some_and(|actual| expected == &actual)
        }
        (ConditionType::IntEquals(expected), JsonValue::Number(number)) => {
            number.as_i64().is_some_and(|actual| expected == &actual)
        }
        (ConditionType::StringContains(needle), JsonValue::String(haystack)) => {
            haystack.contains(needle)
        }
        (ConditionType::StringEquals(expected), JsonValue::String(actual)) => expected == actual,
        _ => false,
    }
}

pub fn inject_setting(key: String, value: JsonValue, settings: &mut JsonMap<String, JsonValue>) {
    let mut keys = key.split('.').collect::<VecDeque<_>>();

    inject_setting_nested(&mut keys, value, settings);
}

pub fn inject_setting_nested(
    keys: &mut VecDeque<&str>,
    value: JsonValue,
    settings: &mut JsonMap<String, JsonValue>,
) {
    let Some(key) = keys.pop_front() else { return };

    // If no other keys, this is the leaf
    if keys.is_empty() {
        settings.insert(key.into(), value);
        return;
    }

    // If not an object, change it to one
    if !settings.contains_key(key) || settings.get(key).is_some_and(|inner| !inner.is_object()) {
        settings.insert(key.into(), JsonValue::Object(JsonMap::default()));
    }

    // Traverse another depth
    if let Some(JsonValue::Object(inner)) = settings.get_mut(key) {
        inject_setting_nested(keys, value, inner);
    }
}

pub fn is_json_falsy(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => true,
        JsonValue::Bool(boolean) => !(*boolean),
        JsonValue::Number(number) => number.as_f64().is_some_and(|no| no == 0.0),
        JsonValue::String(string) => string.is_empty(),
        JsonValue::Array(list) => list.is_empty(),
        JsonValue::Object(map) => map.is_empty(),
    }
}

pub fn display_json_value(value: &JsonValue) -> String {
    match value {
        // Remove quotes
        JsonValue::String(string) => string.to_owned(),
        other => other.to_string(),
    }
}
