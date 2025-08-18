use crate::app_error::AppError;
use crate::commands::init::prompts::*;
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_common::Id;
use moon_config::ToolchainConfig;
use moon_console::ui::{Container, Entry, Notice, Section, Style, StyledText, Variant};
use moon_pdk_api::{ConditionType, InitializeToolchainInput, SettingCondition, SettingPrompt};
use moon_toolchain_plugin::{ToolchainPlugin, ToolchainRegistry};
use proto_core::PluginLocator;
use starbase::AppResult;
use starbase_utils::fs;
use starbase_utils::json::JsonValue;
use starbase_utils::yaml::{self, YamlMapping, YamlNumber, YamlValue};
use std::collections::VecDeque;
use std::str::FromStr;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ToolchainAddArgs {
    #[arg(help = "ID of the toolchain to add")]
    id: Id,

    #[arg(help = "Plugin locator string to find and load the toolchain")]
    plugin: Option<PluginLocator>,

    #[arg(long, help = "Initialize with minimal configuration and prompts")]
    minimal: bool,

    #[arg(long, help = "Skip prompts and use default values")]
    yes: bool,
}

#[instrument(skip_all)]
pub async fn add(session: MoonSession, args: ToolchainAddArgs) -> AppResult {
    let Some(locator) = args
        .plugin
        .clone()
        .or_else(|| ToolchainConfig::get_plugin_locator(&args.id))
    else {
        return Err(AppError::PluginLocatorRequired.into());
    };

    // Load toolchain
    let toolchain_registry = session.get_toolchain_registry().await?;
    let toolchain = toolchain_registry
        .load_without_config(&args.id, &locator)
        .await?;

    // Render config template
    let template = init_toolchain(&session, &args, &toolchain_registry, &toolchain).await?;

    // Update toolchain file
    let toolchain_config_path = &session
        .config_loader
        .get_toolchain_files(&session.workspace_root)[0];

    if toolchain_config_path.exists() {
        fs::append_file(toolchain_config_path, format!("\n\n{template}"))?;
    } else {
        fs::write_file(toolchain_config_path, template)?;
    }

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "Added toolchain <id>{}</id> to <file>.moon/toolchain.yml</file>!", toolchain.id
                    )
                )
            }
        }
    })?;

    Ok(None)
}

#[instrument(skip_all)]
pub async fn init_toolchain(
    session: &MoonSession,
    args: &ToolchainAddArgs,
    toolchain_registry: &ToolchainRegistry,
    toolchain: &ToolchainPlugin,
) -> miette::Result<String> {
    // No instructions, so render an empty block
    if !toolchain.has_func("initialize_toolchain").await {
        return Ok(format!("{}: {{}}", toolchain.id));
    }

    // Extract information from the plugin
    let output = toolchain
        .initialize_toolchain(InitializeToolchainInput {
            context: toolchain_registry.create_context(),
        })
        .await?;

    if !args.yes {
        session.console.render(element! {
            Container {
                Section(title: &toolchain.metadata.name) {
                    Entry(
                        name: "Toolchain",
                        value: element! {
                            StyledText(
                                content: "https://moonrepo.dev/docs/concepts/toolchain",
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

    // Gather built-in settings
    let mut settings = YamlMapping::new();

    if args.plugin.is_some() {
        settings.insert(
            YamlValue::String("plugin".into()),
            YamlValue::String(toolchain.locator.to_string()),
        );
    }

    if toolchain.supports_tier_3().await {
        if toolchain.has_func("detect_version_files").await
            && let Some(version) = toolchain.detect_version(&session.working_dir).await?
        {
            settings.insert(
                YamlValue::String("version".into()),
                YamlValue::String(version.to_string()),
            );
        }

        if !settings.contains_key("version")
            && let Some(version) = render_version_prompt(
                &session.console,
                args.yes || args.minimal,
                &toolchain.metadata.name,
                || Ok(None),
            )
            .await?
        {
            settings.insert(
                YamlValue::String("version".into()),
                YamlValue::String(version.to_string()),
            );
        }
    }

    // Gather user settings via prompts
    for (key, value) in output.default_settings {
        inject_setting(key, value, &mut settings);
    }

    evaluate_prompts(session, args, &output.prompts, &mut settings).await?;

    // Render into a YAML string
    let config = YamlValue::Mapping(YamlMapping::from_iter([(
        YamlValue::String(toolchain.id.to_string()),
        YamlValue::Mapping(settings),
    )]));

    Ok(yaml::format(&config)?)
}

async fn evaluate_prompts(
    session: &MoonSession,
    args: &ToolchainAddArgs,
    prompts: &[SettingPrompt],
    settings: &mut YamlMapping,
) -> miette::Result<()> {
    for prompt in prompts
        .iter()
        .filter(|p| if args.minimal { p.minimal } else { true })
    {
        if let Some(condition) = &prompt.condition
            && !evaluate_condition(condition, settings)
        {
            continue;
        }

        if let Some(value) = render_prompt(&session.console, args.yes, prompt).await? {
            let falsy = is_json_falsy(&value);

            if prompt.skip_if_falsy && falsy {
                continue;
            }

            inject_setting(prompt.setting.clone(), value, settings);

            if !falsy {
                Box::pin(evaluate_prompts(session, args, &prompt.prompts, settings)).await?;
            }
        }
    }

    Ok(())
}

fn evaluate_condition(condition: &SettingCondition, settings: &YamlMapping) -> bool {
    let Some(value) = settings.get(&condition.setting) else {
        return condition.op == ConditionType::NotExists;
    };

    match (&condition.op, value) {
        (ConditionType::BoolEquals(expected), YamlValue::Bool(actual)) => expected == actual,
        (ConditionType::FloatEquals(expected), YamlValue::Number(number)) => {
            number.as_f64().is_some_and(|actual| expected == &actual)
        }
        (ConditionType::IntEquals(expected), YamlValue::Number(number)) => {
            number.as_i64().is_some_and(|actual| expected == &actual)
        }
        (ConditionType::StringContains(needle), YamlValue::String(haystack)) => {
            haystack.contains(needle)
        }
        (ConditionType::StringEquals(expected), YamlValue::String(actual)) => expected == actual,
        _ => false,
    }
}

fn inject_setting(key: String, value: JsonValue, settings: &mut YamlMapping) {
    let mut keys = key.split('.').collect::<VecDeque<_>>();

    inject_setting_nested(&mut keys, value, settings);
}

fn inject_setting_nested(keys: &mut VecDeque<&str>, value: JsonValue, settings: &mut YamlMapping) {
    let Some(key) = keys.pop_front() else { return };

    // If no other keys, this is the leaf
    if keys.is_empty() {
        settings.insert(YamlValue::String(key.into()), convert_json_to_yaml(value));
        return;
    }

    // If not an object, change it to one
    if !settings.contains_key(key) || settings.get(key).is_some_and(|inner| !inner.is_mapping()) {
        settings.insert(
            YamlValue::String(key.into()),
            YamlValue::Mapping(YamlMapping::new()),
        );
    }

    // Traverse another depth
    if let Some(YamlValue::Mapping(inner)) = settings.get_mut(key) {
        inject_setting_nested(keys, value, inner);
    }
}

fn convert_json_to_yaml(value: JsonValue) -> YamlValue {
    match value {
        JsonValue::Null => YamlValue::Null,
        JsonValue::Bool(boolean) => YamlValue::Bool(boolean),
        JsonValue::Number(number) => {
            YamlValue::Number(YamlNumber::from_str(&number.to_string()).unwrap())
        }
        JsonValue::String(string) => YamlValue::String(string),
        JsonValue::Array(list) => {
            YamlValue::Sequence(list.into_iter().map(convert_json_to_yaml).collect())
        }
        JsonValue::Object(map) => YamlValue::Mapping(YamlMapping::from_iter(
            map.into_iter()
                .map(|(key, value)| (YamlValue::String(key), convert_json_to_yaml(value))),
        )),
    }
}

fn is_json_falsy(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => true,
        JsonValue::Bool(boolean) => !(*boolean),
        JsonValue::Number(number) => number.as_f64().is_some_and(|no| no == 0.0),
        JsonValue::String(string) => string.is_empty(),
        JsonValue::Array(list) => list.is_empty(),
        JsonValue::Object(map) => map.is_empty(),
    }
}
