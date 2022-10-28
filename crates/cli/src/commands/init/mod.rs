mod node;
mod typescript;

use crate::helpers::AnyError;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use moon_config::{load_global_project_config_template, load_workspace_config_template};
use moon_constants::{CONFIG_DIRNAME, CONFIG_GLOBAL_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_lang_node::NPM;
use moon_logger::color;
use moon_terminal::create_theme;
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

fn render_template(context: &Context) -> Result<String, Error> {
    Tera::one_off(load_workspace_config_template(), context, false)
}

fn create_default_context() -> Context {
    let mut context = Context::new();
    context.insert("projects", &BTreeMap::<String, String>::new());
    context.insert("project_globs", &Vec::<String>::new());
    context.insert("vcs_manager", &"git");
    context.insert("vcs_default_branch", &"master");
    context
}

pub struct InitOptions {
    pub force: bool,
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
                .with_prompt("Moon has already been initialized, overwrite it?")
                .interact()?
        {
            return Ok(None);
        }

        return Ok(Some(moon_dir));
    }

    Ok(None)
}

pub async fn init(dest: &str, options: InitOptions) -> Result<(), AnyError> {
    let theme = create_theme();
    let working_dir = env::current_dir().expect("Failed to determine working directory.");
    let dest_path = PathBuf::from(dest);
    let dest_dir = if dest == "." {
        working_dir
    } else if dest_path.is_absolute() {
        dest_path
    } else {
        working_dir.join(dest)
    };

    // Extract template variables
    let dest_dir = path::normalize(&dest_dir);
    let moon_dir = match verify_dest_dir(&dest_dir, &options, &theme)? {
        Some(dir) => dir,
        None => return Ok(()),
    };
    let vcs = detect_vcs(&dest_dir).await?;

    // Initialize tools
    let mut workspace_config = VecDeque::new();
    let mut context = create_default_context();
    context.insert("vcs_manager", &vcs.0);
    context.insert("vcs_default_branch", &vcs.1);

    if dest_dir.join(NPM.manifest_filename).exists()
        || Confirm::with_theme(&theme)
            .with_prompt("Initialize Node.js?")
            .interact()?
    {
        workspace_config.push_back(init_node(&dest_dir, &options, &mut context, &theme).await?);

        if dest_dir.join("tsconfig.json").exists()
            || Confirm::with_theme(&theme)
                .with_prompt("Initialize TypeScript?")
                .interact()?
        {
            workspace_config.push_back(init_typescript(&dest_dir, &options, &theme).await?);
        }
    }

    workspace_config.push_front(render_template(&context)?);

    // Create config files
    fs::create_dir_all(&moon_dir).await?;

    fs::write(
        &moon_dir.join(CONFIG_WORKSPACE_FILENAME),
        workspace_config
            .into_iter()
            .collect::<Vec<String>>()
            .join("\n\n"),
    )
    .await?;

    fs::write(
        &moon_dir.join(CONFIG_GLOBAL_PROJECT_FILENAME),
        Tera::one_off(load_global_project_config_template(), &context, false)?,
    )
    .await?;

    // Append to ignore file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest_dir.join(".gitignore"))?;

    writeln!(
        file,
        r#"
 # Moon
 .moon/cache
 .moon/docker"#
    )?;

    println!(
        "Moon has successfully been initialized in {}",
        color::path(&dest_dir),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn renders_default() {
        let context = create_default_context();

        assert_snapshot!(render_template(&context).unwrap());
    }

    #[test]
    fn renders_glob_list() {
        let mut context = create_default_context();
        context.insert("project_globs", &vec!["apps/*", "packages/*"]);

        assert_snapshot!(render_template(&context).unwrap());
    }

    #[test]
    fn renders_projects_map() {
        let mut context = create_default_context();
        context.insert("projects", &BTreeMap::from([("example", "apps/example")]));

        assert_snapshot!(render_template(&context).unwrap());
    }

    #[test]
    fn renders_git_vcs() {
        let mut context = create_default_context();
        context.insert("vcs_manager", &"git");
        context.insert("vcs_default_branch", &"main");

        assert_snapshot!(render_template(&context).unwrap());
    }

    #[test]
    fn renders_svn_vcs() {
        let mut context = create_default_context();
        context.insert("vcs_manager", &"svn");
        context.insert("vcs_default_branch", &"trunk");

        assert_snapshot!(render_template(&context).unwrap());
    }
}
