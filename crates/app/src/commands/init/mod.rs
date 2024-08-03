mod bun;
mod node;
mod prompts;
mod rust;
mod typescript;

use crate::app_error::ExitCode;
use crate::helpers::create_theme;
use crate::session::CliSession;
use bun::init_bun;
use clap::{Args, ValueEnum};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use miette::IntoDiagnostic;
use moon_common::consts::{
    CONFIG_DIRNAME, CONFIG_TOOLCHAIN_FILENAME, CONFIG_WORKSPACE_FILENAME_YML,
};
use moon_common::is_test_env;
use moon_config::{load_toolchain_config_template, load_workspace_config_template};
use moon_console::Console;
use moon_vcs::{Git, Vcs};
use node::init_node;
use rust::init_rust;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use tracing::instrument;
use typescript::init_typescript;

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "lowercase")]
pub enum InitTool {
    Bun,
    Node,
    Rust,
    TypeScript,
}

#[derive(Args, Clone, Debug)]
pub struct InitArgs {
    #[arg(value_enum, help = "Specific tool to initialize")]
    tool: Option<InitTool>,

    #[arg(
        long = "to",
        help = "Destination to initialize into",
        default_value = "."
    )]
    dest: String,

    #[arg(long, help = "Overwrite existing configurations")]
    force: bool,

    #[arg(long, help = "Initialize with minimal configuration and prompts")]
    minimal: bool,

    #[arg(long, help = "Skip prompts and use default values")]
    yes: bool,
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
    console: &Console,
) -> AppResult {
    if !is_test_env() && !dest_dir.join(CONFIG_DIRNAME).exists() {
        console.err.write_line(format!(
            "moon has not been initialized! Try running {} first?",
            color::shell("moon init")
        ))?;

        return Err(ExitCode(1).into());
    }

    let tool_config = match tool {
        InitTool::Bun => init_bun(dest_dir, options, theme, console).await?,
        InitTool::Node => init_node(dest_dir, options, theme, console).await?,
        InitTool::Rust => init_rust(dest_dir, options, theme, console).await?,
        InitTool::TypeScript => init_typescript(dest_dir, options, theme, console).await?,
    };

    let toolchain_config_path = dest_dir
        .join(CONFIG_DIRNAME)
        .join(CONFIG_TOOLCHAIN_FILENAME);

    if !toolchain_config_path.exists() {
        fs::write_file(
            &toolchain_config_path,
            render_toolchain_template(&Context::new())?.trim(),
        )?;
    }

    fs::append_file(toolchain_config_path, format!("\n\n{}", tool_config.trim()))?;

    console.out.write_newline()?;

    console
        .out
        .write_line("Toolchain config has successfully been updated")?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn init(session: CliSession, args: InitArgs) -> AppResult {
    let options = InitOptions {
        force: args.force,
        minimal: args.minimal,
        yes: args.yes,
    };

    let theme = create_theme();
    let dest_path = PathBuf::from(&args.dest);
    let dest_dir = if args.dest == "." {
        session.working_dir.clone()
    } else if dest_path.is_absolute() {
        dest_path
    } else {
        session.working_dir.join(&args.dest)
    };

    // Initialize a specific tool and exit early
    if let Some(tool) = &args.tool {
        init_tool(&dest_dir, tool, &options, &theme, &session.console).await?;

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
            git.get_remote_default_branch().await?
        } else {
            git.get_default_branch().await?
        }
        .as_str(),
    );

    // Create workspace file
    fs::write_file(
        moon_dir.join(CONFIG_WORKSPACE_FILENAME_YML),
        render_workspace_template(&context)?,
    )?;

    // Append to ignore file
    fs::append_file(
        dest_dir.join(".gitignore"),
        r#"
# moon
.moon/cache
.moon/docker
"#,
    )?;

    let stdout = session.console.stdout();

    stdout.write_newline()?;

    stdout.write_line(format!(
        "Successfully initialized moon in {}!",
        color::path(&dest_dir),
    ))?;

    stdout.write_line("Get started with these next steps.")?;

    stdout.write_newline()?;

    stdout.write_line(format!(
        "  Learn more: {}",
        color::url("https://moonrepo.dev/docs")
    ))?;

    stdout.write_line(format!(
        "  Need help? {}",
        color::url("https://discord.gg/qCh9MEynv2")
    ))?;

    stdout.write_newline()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::assert_snapshot;

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
