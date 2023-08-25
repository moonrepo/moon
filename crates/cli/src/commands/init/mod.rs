mod node;
mod rust;
mod typescript;

use clap::{Args, ValueEnum};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use miette::IntoDiagnostic;
use moon_common::consts::{CONFIG_DIRNAME, CONFIG_TOOLCHAIN_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_config::{load_toolchain_config_template, load_workspace_config_template};
use moon_node_lang::NPM;
use moon_rust_lang::CARGO;
use moon_terminal::{create_theme, safe_exit};
use moon_utils::path;
use moon_vcs::{Git, Vcs};
use node::init_node;
use rust::init_rust;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;
use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use typescript::init_typescript;

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "lowercase")]
pub enum InitTool {
    Node,
    Rust,
    TypeScript,
}

#[derive(Args, Clone, Debug)]
pub struct InitArgs {
    #[arg(help = "Destination to initialize in", default_value = ".")]
    dest: String,

    #[arg(long, help = "Overwrite existing configurations")]
    force: bool,

    #[arg(long, help = "Initialize with minimal configuration and prompts")]
    minimal: bool,

    #[arg(long, help = "Skip prompts and use default values")]
    yes: bool,

    #[arg(long, value_enum, help = "Specific tool to initialize")]
    tool: Option<InitTool>,
}

fn render_toolchain_template(context: &Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_config_template(), context, false).into_diagnostic()
}

fn render_workspace_template(context: &Context) -> AppResult<String> {
    Tera::one_off(load_workspace_config_template(), context, false).into_diagnostic()
}

fn create_default_context() -> Context {
    let mut context = Context::new();
    context.insert("projects", &BTreeMap::<String, String>::new());
    context.insert("project_globs", &vec!["apps/*", "packages/*"]);
    context.insert("vcs_manager", &"git");
    context.insert("vcs_provider", &"github");
    context.insert("vcs_default_branch", &"master");
    context
}

fn detect_vcs_provider(repo_root: PathBuf) -> String {
    if repo_root.join(".gitlab").exists() {
        "gitlab".into()
    } else if repo_root.join("bitbucket-pipelines.yml").exists() {
        "bitbucket".into()
    } else {
        "github".into()
    }
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
) -> AppResult<Option<PathBuf>> {
    if options.yes
        || Confirm::with_theme(theme)
            .with_prompt(format!("Initialize moon into {}?", color::path(dest_dir)))
            .interact()
            .into_diagnostic()?
    {
        let moon_dir = dest_dir.join(CONFIG_DIRNAME);

        if !options.force
            && moon_dir.exists()
            && !Confirm::with_theme(theme)
                .with_prompt("moon has already been initialized, overwrite it?")
                .interact()
                .into_diagnostic()?
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
) -> AppResult {
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
        InitTool::Rust => init_rust(dest_dir, options, theme, None).await?,
        InitTool::TypeScript => init_typescript(dest_dir, options, theme).await?,
    };

    let mut file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(workspace_config_path)
        .into_diagnostic()?;

    writeln!(file, "\n\n{}", tool_config.trim()).into_diagnostic()?;

    println!("\nWorkspace config has successfully been updated");

    Ok(())
}

pub async fn init(args: InitArgs) -> AppResult {
    let options = InitOptions {
        force: args.force,
        minimal: args.minimal,
        yes: args.yes,
    };

    let theme = create_theme();
    let working_dir = env::current_dir().expect("Failed to determine working directory.");
    let dest_path = PathBuf::from(&args.dest);
    let dest_dir = if args.dest == "." {
        working_dir
    } else if dest_path.is_absolute() {
        dest_path
    } else {
        working_dir.join(&args.dest)
    };
    let dest_dir = path::normalize(&dest_dir);

    // Initialize a specific tool and exit early
    if let Some(tool) = &args.tool {
        init_tool(&dest_dir, tool, &options, &theme).await?;

        return Ok(());
    }

    // Extract template variables
    let Some(moon_dir) = verify_dest_dir(&dest_dir, &options, &theme)? else {
        return Ok(());
    };
    let mut context = create_default_context();

    let git = Git::load(&dest_dir, "master", &[])?;
    context.insert("vcs_manager", "git");
    context.insert(
        "vcs_provider",
        &detect_vcs_provider(git.get_repository_root().await?),
    );
    context.insert(
        "vcs_default_branch",
        if git.is_enabled() {
            git.get_local_branch().await?
        } else {
            git.get_default_branch().await?
        },
    );

    // Initialize all tools
    let mut toolchain_configs = VecDeque::new();

    if dest_dir.join(NPM.manifest).exists()
        || !options.yes
            && Confirm::with_theme(&theme)
                .with_prompt("Initialize Node.js?")
                .interact()
                .into_diagnostic()?
    {
        toolchain_configs
            .push_back(init_node(&dest_dir, &options, &theme, Some(&mut context)).await?);

        if dest_dir.join("tsconfig.json").exists()
            || !options.yes
                && Confirm::with_theme(&theme)
                    .with_prompt("Initialize TypeScript?")
                    .interact()
                    .into_diagnostic()?
        {
            toolchain_configs.push_back(init_typescript(&dest_dir, &options, &theme).await?);
        }
    }

    // For other languages, avoid enabling them by default
    if dest_dir.join(CARGO.manifest).exists()
        || !options.yes
            && Confirm::with_theme(&theme)
                .with_prompt("Initialize Rust?")
                .interact()
                .into_diagnostic()?
    {
        toolchain_configs
            .push_back(init_rust(&dest_dir, &options, &theme, Some(&mut context)).await?);
    }

    toolchain_configs.push_front(render_toolchain_template(&context)?);

    // Create config files
    fs::write_file(
        moon_dir.join(CONFIG_TOOLCHAIN_FILENAME),
        toolchain_configs
            .into_iter()
            .map(|c| c.trim().to_owned())
            .collect::<Vec<String>>()
            .join("\n\n"),
    )?;

    fs::write_file(
        moon_dir.join(CONFIG_WORKSPACE_FILENAME),
        render_workspace_template(&context)?,
    )?;

    // Append to ignore file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest_dir.join(".gitignore"))
        .into_diagnostic()?;

    writeln!(
        file,
        r#"
# moon
.moon/cache
.moon/docker"#
    )
    .into_diagnostic()?;

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

    #[test]
    fn renders_gitlab() {
        let mut context = create_default_context();
        context.insert("vcs_manager", &"git");
        context.insert("vcs_provider", &"gitlab");
        context.insert("vcs_default_branch", &"main");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_bitbucket() {
        let mut context = create_default_context();
        context.insert("vcs_manager", &"git");
        context.insert("vcs_provider", &"bitbucket");
        context.insert("vcs_default_branch", &"main");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }
}
