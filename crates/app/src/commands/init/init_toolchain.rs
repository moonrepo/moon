use super::InitOptions;
use super::prompts::prompt_version;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, Select};
use miette::IntoDiagnostic;
use moon_console::Console;
use moon_pdk_api::{
    ConditionType, InitializeToolchainInput, PromptType, SettingCondition, SettingPrompt,
};
use moon_toolchain_plugin::{ToolchainPlugin, ToolchainRegistry};
use schematic::color::apply_style_tags;
use starbase_styles::color;
use starbase_utils::json::JsonValue;
use starbase_utils::yaml::{self, YamlMapping, YamlNumber, YamlValue};
use std::collections::VecDeque;
use std::str::FromStr;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn init_toolchain(
    toolchain_registry: &ToolchainRegistry,
    toolchain: &ToolchainPlugin,
    options: &InitOptions,
    theme: &ColorfulTheme,
    console: &Console,
    include_locator: bool,
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

    if !options.yes {
        console.out.print_header(&toolchain.metadata.name)?;

        console.out.write_raw(|buffer| {
            buffer.extend_from_slice(
                format!(
                    "Toolchain: {}\n",
                    color::url("https://moonrepo.dev/docs/concepts/toolchain")
                )
                .as_bytes(),
            );

            if let Some(url) = &output.docs_url {
                buffer.extend_from_slice(format!("Handbook: {}\n", color::url(url)).as_bytes());
            }

            if let Some(url) = &output.config_url {
                buffer.extend_from_slice(format!("Config: {}\n\n", color::url(url)).as_bytes());
            }
        })?;

        console.out.flush()?;
    }

    // Gather built-in settings
    let mut settings = YamlMapping::new();

    if include_locator {
        settings.insert(
            YamlValue::String("plugin".into()),
            YamlValue::String(toolchain.locator.to_string()),
        );
    }

    if toolchain.has_func("detect_version_files").await {
        if let Some(version) = toolchain.detect_version(&options.dir).await? {
            settings.insert(
                YamlValue::String("version".into()),
                YamlValue::String(version.to_string()),
            );
        }
    }

    if !settings.contains_key("version") && toolchain.supports_tier_3().await {
        // TODO rewrite
        let version = prompt_version(&toolchain.metadata.name, options, theme, || {
            Ok(String::new())
        })?;

        if !version.is_empty() {
            settings.insert(
                YamlValue::String("version".into()),
                YamlValue::String(version),
            );
        }
    }

    // Gather user settings via prompts
    let prompts = output
        .prompts
        .into_iter()
        .filter(|p| if options.minimal { p.minimal } else { true })
        .collect::<Vec<_>>();

    for (key, value) in output.default_settings {
        inject_setting(key, value, &mut settings);
    }

    evaluate_prompts(&prompts, &mut settings, options, theme)?;

    // Render into a YAML string
    let config = YamlValue::Mapping(YamlMapping::from_iter([(
        YamlValue::String(toolchain.id.to_string()),
        YamlValue::Mapping(settings),
    )]));

    Ok(yaml::format(&config)?)
}

fn evaluate_prompts(
    prompts: &[SettingPrompt],
    settings: &mut YamlMapping,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> miette::Result<()> {
    for prompt in prompts {
        if let Some(condition) = &prompt.condition {
            if !evaluate_condition(condition, settings) {
                continue;
            }
        }

        if let Some(value) = render_prompt(prompt, options, theme)? {
            let falsy = is_json_falsy(&value);

            if prompt.skip_if_falsy && falsy {
                continue;
            }

            inject_setting(prompt.setting.clone(), value, settings);

            if !falsy {
                evaluate_prompts(&prompt.prompts, settings, options, theme)?;
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

fn render_prompt(
    prompt: &SettingPrompt,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> miette::Result<Option<JsonValue>> {
    match &prompt.ty {
        PromptType::None => Ok(None),
        PromptType::Confirm { default } => {
            let result = if options.yes {
                *default
            } else {
                let confirm = Confirm::with_theme(theme)
                    .with_prompt(apply_style_tags(&prompt.question))
                    .default(*default)
                    .show_default(true);

                if prompt.required {
                    confirm.interact().into_diagnostic()?
                } else {
                    confirm
                        .interact_opt()
                        .into_diagnostic()?
                        .unwrap_or(*default)
                }
            };

            Ok(Some(JsonValue::Bool(result)))
        }
        PromptType::Input { default } => {
            let result = if options.yes {
                default.to_owned()
            } else {
                Input::with_theme(theme)
                    .with_prompt(apply_style_tags(&prompt.question))
                    .default(default.to_owned())
                    .show_default(true)
                    .allow_empty(!prompt.required)
                    .interact_text()
                    .into_diagnostic()?
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
                let labels = items.iter().map(display_json_value).collect::<Vec<_>>();
                let select = Select::with_theme(theme)
                    .with_prompt(apply_style_tags(&prompt.question))
                    .items(&labels)
                    .default(*default_index);

                if prompt.required {
                    select.interact().into_diagnostic()?
                } else {
                    select
                        .interact_opt()
                        .into_diagnostic()?
                        .unwrap_or(*default_index)
                }
            };

            Ok(Some(items[index].clone()))
        }
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

fn display_json_value(value: &JsonValue) -> String {
    match value {
        // Remove quotes
        JsonValue::String(string) => string.to_owned(),
        other => other.to_string(),
    }
}
