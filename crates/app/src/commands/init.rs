#![allow(clippy::disallowed_types)]

use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{FlexDirection, View, element};
use moon_common::{consts::CONFIG_DIRNAME, path::clean_components};
use moon_config::{VcsProvider, WorkspaceConfig};
use moon_console::ui::{Confirm, Container, Notice, StyledText, Variant};
use moon_vcs::{Vcs, git::Git};
use schematic::schema::{
    ArrayType, SchemaGenerator, SchemaType, StringType, TemplateOptions, YamlTemplateRenderer,
};
use starbase::AppResult;
use starbase_utils::{fs, string_vec};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct InitArgs {
    #[arg(help = "Destination to initialize into", default_value = ".")]
    dest: PathBuf,

    #[arg(long, help = "Overwrite existing configurations")]
    force: bool,

    #[arg(long, help = "Initialize with minimal configuration and prompts")]
    minimal: bool,

    #[arg(long, help = "Skip prompts and use default values")]
    yes: bool,
}

#[instrument(skip(session))]
pub async fn init(session: MoonSession, args: InitArgs) -> AppResult {
    let dest_dir = clean_components(if args.dest.is_absolute() {
        args.dest.clone()
    } else {
        session.working_dir.join(&args.dest)
    });

    // Verify destination directory
    let init = if args.yes {
        true
    } else {
        let mut value = false;

        session
            .console
            .render_interactive(element! {
                Confirm(
                    label: format!("Initialize moon into <path>{}</path>?", dest_dir.display()),
                    on_confirm: &mut value
                )
            })
            .await?;

        value
    };

    if init {
        let moon_dir = dest_dir.join(CONFIG_DIRNAME);

        if !args.force && moon_dir.exists() {
            let mut force = false;

            session
                .console
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
    }

    // Load VCS information
    let git = Git::load(&dest_dir, "master", &[])?;
    let git_root = git.get_repository_root().await?;

    let git_provider = if git_root.join(".gitlab").exists() {
        VcsProvider::GitLab
    } else if git_root.join("bitbucket-pipelines.yml").exists() {
        VcsProvider::Bitbucket
    } else {
        VcsProvider::GitHub
    };

    let default_branch = if git.is_enabled() {
        git.get_remote_default_branch().await?
    } else {
        git.get_default_branch().await?
    };

    // Create workspace file
    let mut generator = SchemaGenerator::default();
    generator.add::<WorkspaceConfig>();

    generator.generate(
        session
            .config_loader
            .get_workspace_files(&dest_dir)
            .remove(0),
        YamlTemplateRenderer::new(TemplateOptions {
            // TODO update schematic
            default_values: HashMap::from_iter([
                (
                    "projects".into(),
                    SchemaType::Array(Box::new(ArrayType::new(SchemaType::String(Box::new(
                        StringType::new("packages/*"),
                    ))))),
                ),
                (
                    "vcs.defaultBranch".into(),
                    SchemaType::String(Box::new(StringType::new(default_branch.as_str()))),
                ),
                (
                    "vcs.provider".into(),
                    SchemaType::String(Box::new(StringType::new(git_provider.to_string()))),
                ),
            ]),
            expand_fields: string_vec!["projects"],
            hide_fields: string_vec![
                "codeowners",
                "constraints",
                "docker",
                "experiments",
                "extends",
                "generator",
                "hasher",
                "notifier",
                "pipeline",
                "remote",
                "telemetry",
                "vcs.hookFormat",
                "vcs.hooks",
                "vcs.remoteCandidates",
                "vcs.sync",
                "versionConstraint",
            ],
            ..Default::default()
        }),
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

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "Successfully initialized moon in <path>{}</path>!", dest_dir.display(),
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
