mod node;
mod typescript;

use crate::helpers::AnyError;
use clap::ValueEnum;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use moon_config::{
    load_tasks_config_template, load_toolchain_config_template, load_workspace_config_template,
};
use moon_constants::{
    CONFIG_DIRNAME, CONFIG_TASKS_FILENAME, CONFIG_TOOLCHAIN_FILENAME, CONFIG_WORKSPACE_FILENAME,
};
use moon_logger::color;
use moon_node_lang::NPM;
use moon_terminal::{create_theme, safe_exit};
use moon_utils::{fs, path};
use moon_vcs::detect_vcs;
use node::init_node;
use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tera::{Context, Error, Tera};
use typescript::init_typescript;

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "lowercase")]
pub enum InitTool {
    Node,
    TypeScript,
}

fn render_toolchain_template(context: &Context) -> Result<String, Error> {
    Tera::one_off(load_toolchain_config_template(), context, false)
}

fn render_workspace_template(context: &Context) -> Result<String, Error> {
    Tera::one_off(load_workspace_config_template(), context, false)
}

fn create_default_context() -> Context {
    let mut context = Context::new();
    context.insert("projects", &BTreeMap::<String, String>::new());
    context.insert("project_globs", &vec!["apps/*", "packages/*"]);
    context.insert("vcs_manager", &"git");
    context.insert("vcs_default_branch", &"master");
    context
}

pub struct InitOptions {
    pub force: bool,
    pub minimal: bool,
    pub yes: bool,
}

/// Verify the destination and return a path to the `.moon` folder
/// if all questions have passed.
fn verify_dest_dir(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> Result<Option<PathBuf>, AnyError> {
    if options.yes
        || Confirm::with_theme(theme)
            .with_prompt(format!("Initialize moon into {}?", color::path(dest_dir)))
            .interact()?
    {
        let moon_dir = dest_dir.join(CONFIG_DIRNAME);

        if !options.force
            && moon_dir.exists()
            && !Confirm::with_theme(theme)
                .with_prompt("moon has already been initialized, overwrite it?")
                .interact()?
        {
            return Ok(None);
        }

        fs::create_dir_all(&moon_dir)?;

        return Ok(Some(moon_dir));
    }

    Ok(None)
}

pub async fn init_tool(
    dest_dir: &Path,
    tool: &InitTool,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> Result<(), AnyError> {
    let workspace_config_path = dest_dir
        .join(CONFIG_DIRNAME)
        .join(CONFIG_WORKSPACE_FILENAME);

    if !workspace_config_path.exists() {
        eprintln!(
            "moon has not been initialized! Try running {} first?",
            color::shell("moon init")
        );

        safe_exit(1);
    }

    let tool_config = match tool {
        InitTool::Node => init_node(dest_dir, options, theme, None).await?,
        InitTool::TypeScript => init_typescript(dest_dir, options, theme).await?,
    };

    let mut file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(workspace_config_path)?;

    writeln!(file, "\n\n{}", tool_config.trim())?;

    println!("\nWorkspace config has successfully been updated");

    Ok(())
}

pub async fn init(
    dest: String,
    tool: Option<InitTool>,
    options: InitOptions,
) -> Result<(), AnyError> {
    let theme = create_theme();
    let working_dir = env::current_dir().expect("Failed to determine working directory.");
    let dest_path = PathBuf::from(&dest);
    let dest_dir = if dest == "." {
        working_dir
    } else if dest_path.is_absolute() {
        dest_path
    } else {
        working_dir.join(dest)
    };
    let dest_dir = path::normalize(&dest_dir);

    // Initialize a specific tool and exit early
    if let Some(tool) = &tool {
        init_tool(&dest_dir, tool, &options, &theme).await?;

        return Ok(());
    }

    // Extract template variables
    let Some(moon_dir) = verify_dest_dir(&dest_dir, &options, &theme)? else {
        return Ok(())
    };
    let mut context = create_default_context();

    let vcs = detect_vcs(&dest_dir).await?;
    context.insert("vcs_manager", &vcs.0.to_string());
    context.insert("vcs_default_branch", &vcs.1);

    // Initialize all tools
    let mut toolchain_configs = VecDeque::new();

    if options.yes
        || dest_dir.join(NPM.manifest).exists()
        || Confirm::with_theme(&theme)
            .with_prompt("Initialize Node.js?")
            .interact()?
    {
        toolchain_configs
            .push_back(init_node(&dest_dir, &options, &theme, Some(&mut context)).await?);

        if options.yes
            || dest_dir.join("tsconfig.json").exists()
            || Confirm::with_theme(&theme)
                .with_prompt("Initialize TypeScript?")
                .interact()?
        {
            toolchain_configs.push_back(init_typescript(&dest_dir, &options, &theme).await?);
        }
    }

    toolchain_configs.push_front(render_toolchain_template(&context)?);

    // Create config files
    fs::write(
        moon_dir.join(CONFIG_TOOLCHAIN_FILENAME),
        toolchain_configs
            .into_iter()
            .map(|c| c.trim().to_owned())
            .collect::<Vec<String>>()
            .join("\n\n"),
    )?;

    fs::write(
        moon_dir.join(CONFIG_WORKSPACE_FILENAME),
        render_workspace_template(&context)?,
    )?;

    if !options.minimal {
        fs::write(
            moon_dir.join(CONFIG_TASKS_FILENAME),
            Tera::one_off(load_tasks_config_template(), &context, false)?,
        )?;
    }

    // Append to ignore file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest_dir.join(".gitignore"))?;

    writeln!(
        file,
        r#"
# moon
.moon/cache
.moon/docker"#
    )?;

    println!(
        "\nmoon has successfully been initialized in {}",
        color::path(&dest_dir),
    );

    println!(
        "\nNot enjoying moon? Let us know with a 1 minute survey: {}",
        color::url("https://bit.ly/moon-survey")
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_test_utils::assert_snapshot;

    #[test]
    fn renders_default() {
        let context = create_default_context();

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_glob_list() {
        let mut context = create_default_context();
        context.insert("project_globs", &vec!["apps/*", "packages/*"]);

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_projects_map() {
        let mut context = create_default_context();
        context.insert("projects", &BTreeMap::from([("example", "apps/example")]));

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_git_vcs() {
        let mut context = create_default_context();
        context.insert("vcs_manager", &"git");
        context.insert("vcs_default_branch", &"main");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_svn_vcs() {
        let mut context = create_default_context();
        context.insert("vcs_manager", &"svn");
        context.insert("vcs_default_branch", &"trunk");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }
}
