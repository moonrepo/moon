use super::InitOptions;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};
use miette::IntoDiagnostic;
use starbase_styles::color;

pub fn prompt_version(
    label: &str,
    options: &InitOptions,
    theme: &ColorfulTheme,
    op: impl FnOnce() -> miette::Result<String>,
) -> miette::Result<String> {
    let mut version = op()?;

    if options.yes || options.minimal {
        return Ok(version);
    }

    if Confirm::with_theme(theme)
        .with_prompt(if version.is_empty() {
            format!(
                "Manage {} through {}? {}",
                label,
                color::shell("moon"),
                color::muted("(recommended)")
            )
        } else {
            format!(
                "Manage {} {} through {}? {}",
                label,
                version,
                color::shell("moon"),
                color::muted("(recommended)")
            )
        })
        .interact()
        .into_diagnostic()?
    {
        if version.is_empty() {
            version = Input::with_theme(theme)
                .with_prompt(format!("{} version?", label))
                .allow_empty(false)
                .interact_text()
                .into_diagnostic()?;
        }
    } else {
        version = String::new();
    }

    Ok(version)
}
