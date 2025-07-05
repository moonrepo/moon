mod bun;
mod node;
pub mod prompts;
mod rust;

use crate::session::MoonSession;
use bun::init_bun;
use clap::Args;
use clean_path::Clean;
use iocraft::prelude::{FlexDirection, View, element};
use miette::IntoDiagnostic;
use moon_common::{Id, consts::CONFIG_DIRNAME, is_test_env};
use moon_config::{load_toolchain_config_template, load_workspace_config_template};
use moon_console::{
    Console,
    ui::{Confirm, Container, Notice, StyledText, Variant},
};
use moon_vcs::{Git, Vcs};
use node::init_node;
use proto_core::PluginLocator;
use rust::init_rust;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tera::{Context, Tera};
use tracing::{instrument, warn};

#[derive(Args, Clone, Debug)]
pub struct InitArgs {
    #[arg(help = "Specific toolchain to initialize")]
    toolchain: Option<Id>,

    #[arg(help = "Plugin locator for the toolchain")]
    plugin: Option<PluginLocator>,

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

fn render_toolchain_template(context: &Context) -> miette::Result<String> {
    Tera::one_off(load_toolchain_config_template(), context, false).into_diagnostic()
}

fn render_workspace_template(context: &Context) -> miette::Result<String> {
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
    pub dir: PathBuf,
    pub force: bool,
    pub minimal: bool,
    pub yes: bool,
}

/// Verify the destination and return a path to the `.moon` folder
/// if all questions have passed.
async fn verify_dest_dir(
    console: &Console,
    options: &InitOptions,
) -> miette::Result<Option<PathBuf>> {
    let init = if options.yes {
        true
    } else {
        let mut value = false;

        console
            .render_interactive(element! {
                Confirm(
                    label: format!("Initialize moon into <path>{}</path>?", options.dir.display()),
                    on_confirm: &mut value
                )
            })
            .await?;

        value
    };

    if init {
        let moon_dir = options.dir.join(CONFIG_DIRNAME);

        if !options.force && moon_dir.exists() {
            let mut force = false;

            console
                .render_interactive(element! {
                    Confirm(
                        label: "moon has already been initialized, overwrite it?",
                        on_confirm: &mut force
                    )
                })
                .await?;

            if !force {
                return Ok(None);
            }
        }

        fs::create_dir_all(&moon_dir)?;

        return Ok(Some(moon_dir));
    }

    Ok(None)
}

pub async fn init_for_toolchain(
    session: &MoonSession,
    args: &InitArgs,
    options: &InitOptions,
) -> AppResult {
    let console = &session.console;
    let id = args.toolchain.as_ref().unwrap();

    if !is_test_env() && !options.dir.join(CONFIG_DIRNAME).exists() {
        console.err.write_line(format!(
            "moon has not been initialized! Try running {} first?",
            color::shell("moon init")
        ))?;

        return Ok(Some(1));
    }

    let tool_config = match id.as_str() {
        "bun" => init_bun(console, options).await?,
        "node" => init_node(console, options).await?,
        "rust" => init_rust(console, options).await?,
        _ => {
            warn!(
                "This command has been deprecated for toolchain plugins, use {} instead.",
                color::shell(format!("moon toolchain add {id}"))
            );

            return Ok(None);
        }
    };

    let toolchain_config_path = &session.config_loader.get_toolchain_files(&options.dir)[0];

    if !toolchain_config_path.exists() {
        fs::write_file(
            toolchain_config_path,
            render_toolchain_template(&Context::new())?.trim(),
        )?;
    }

    fs::append_file(toolchain_config_path, format!("\n\n{}", tool_config.trim()))?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: "Configuration <file>.moon/toolchain.yml</file> has successfully been updated!"
                )
            }
        }
    })?;

    Ok(None)
}

#[instrument(skip_all)]
pub async fn init(session: MoonSession, args: InitArgs) -> AppResult {
    let dest_path = PathBuf::from(&args.dest);
    let dest_dir = if args.dest == "." {
        session.working_dir.clone()
    } else if dest_path.is_absolute() {
        dest_path
    } else {
        session.working_dir.join(&args.dest)
    };

    let options = InitOptions {
        dir: dest_dir.clean(),
        force: args.force,
        minimal: args.minimal,
        yes: args.yes,
    };

    // Initialize a specific tool and exit early
    if args.toolchain.is_some() {
        init_for_toolchain(&session, &args, &options).await?;

        return Ok(None);
    }

    // Extract template variables
    if verify_dest_dir(&session.console, &options).await?.is_none() {
        return Ok(None);
    }

    let git = Git::load(&options.dir, "master", &[])?;

    let mut context = create_default_context();
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
        &session.config_loader.get_workspace_files(&options.dir)[0],
        render_workspace_template(&context)?,
    )?;

    // Append to ignore file
    fs::append_file(
        options.dir.join(".gitignore"),
        r#"
# moon
.moon/cache
.moon/docker
"#,
    )?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "Successfully initialized moon in <path>{}</path>!", options.dir.display(),
                    )
                )

                View(padding_top: 1, flex_direction: FlexDirection::Column) {
                    StyledText(content: "Learn more: <url>https://moonrepo.dev/docs</url>")
                    StyledText(content: "Need help?  <url>https://discord.gg/qCh9MEynv2</url>")
                }
            }
        }
    })?;

    Ok(None)
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
