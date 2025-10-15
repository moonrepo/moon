pub mod prompts;

use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{FlexDirection, View, element};
use miette::IntoDiagnostic;
use moon_common::{consts::CONFIG_DIRNAME, path::clean_components};
use moon_config::load_workspace_config_template;
use moon_console::{
    Console,
    ui::{Confirm, Container, Notice, StyledText, Variant},
};
use moon_vcs::{Vcs, git::Gitx};
use proto_core::PluginLocator;
use starbase::AppResult;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tera::{Context, Tera};
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct InitArgs {
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

fn render_workspace_template(context: &Context) -> miette::Result<String> {
    Tera::one_off(load_workspace_config_template(), context, false).into_diagnostic()
}

fn create_default_context() -> Context {
    let mut context = Context::new();
    context.insert("projects", &BTreeMap::<String, String>::new());
    context.insert("project_globs", &vec!["apps/*", "packages/*"]);
    context.insert("vcs_client", &"git");
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
        dir: clean_components(dest_dir),
        force: args.force,
        minimal: args.minimal,
        yes: args.yes,
    };

    // Extract template variables
    if verify_dest_dir(&session.console, &options).await?.is_none() {
        return Ok(None);
    }

    let git = Gitx::load(&options.dir, "master", &[])?;

    let mut context = create_default_context();
    context.insert("vcs_client", "git");
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
        context.insert("vcs_client", &"git");
        context.insert("vcs_default_branch", &"main");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_svn_vcs() {
        let mut context = create_default_context();
        context.insert("vcs_client", &"svn");
        context.insert("vcs_default_branch", &"trunk");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_gitlab() {
        let mut context = create_default_context();
        context.insert("vcs_client", &"git");
        context.insert("vcs_provider", &"gitlab");
        context.insert("vcs_default_branch", &"main");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }

    #[test]
    fn renders_bitbucket() {
        let mut context = create_default_context();
        context.insert("vcs_client", &"git");
        context.insert("vcs_provider", &"bitbucket");
        context.insert("vcs_default_branch", &"main");

        assert_snapshot!(render_workspace_template(&context).unwrap());
    }
}
