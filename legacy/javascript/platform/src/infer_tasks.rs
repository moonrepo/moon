use moon_args::split_args;
use moon_common::Id;
use moon_config::{
    NodePackageManager, OneOrMany, OutputPath, PartialTaskArgs, PartialTaskConfig,
    PartialTaskDependency,
};
use moon_node_lang::package_json::{PackageJsonCache, ScriptsMap};
use moon_target::Target;
use moon_toolchain::detect::is_system_command;
use moon_utils::regex::ID_CLEAN;
use moon_utils::{regex, string_vec};
use rustc_hash::FxHashMap;
use starbase_styles::color;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use tracing::{debug, warn};

static WIN_DRIVE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::create_regex(r#"^[A-Z]:"#).unwrap());

static ARG_ENV_VAR: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::create_regex(r#"^[A-Z0-9_]+="#).unwrap());

static ARG_OUTPUT_FLAG: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::create_regex(r#"^(-o|--(out|output|dist)(-{0,1}(?i:dir|file))?)$"#).unwrap()
});

static INFO_OPTIONS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::create_regex(r#"--(help|version)"#).unwrap());

// This isn't exhaustive but captures very popular tools
static DEV_COMMAND: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::create_regex(r#"(astro (dev|preview))|(concurrently)|(gatsby (new|dev|develop|serve|repl))|(next (dev|start))|(nuxt (dev|preview))|(packemon watch)|(parcel [^build])|(react-scripts start)|(snowpack dev)|(solid-start (dev|start))|(vite (dev|preview|serve))|(vue-cli-service serve)|(webpack (s|serve|server|w|watch|-))"#).unwrap()
});

static DEV_COMMAND_SOLO: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::create_regex(r#"^(npx |yarn dlx |pnpm dlx |bun x |bunx )?(parcel|vite|webpack)$"#)
        .unwrap()
});

static DEV_ONLY_NAME: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::create_regex(
        r#"(^(dev|start|serve|preview)$)|(^(start|serve|preview):)|(:(start|serve|preview)$)"#,
    )
    .unwrap()
});

// Special package manager handling
static PM_RUN_COMMAND: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::create_regex(r#"(?:npm|pnpm|yarn|bun) run ([a-zA-Z0-9:-_]+)([^&]+)?"#).unwrap()
});

static PM_LIFE_CYCLES: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::create_regex(r#"^(preprepare|prepare|postprepare|prepublish|prepublishOnly|publish|postpublish|prepack|pack|postpack|preinstall|install|postinstall|preversion|version|postversion|dependencies)$"#).unwrap()
});

// These patterns are currently not allowed
static INVALID_CD: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::create_regex(r"(^|\b|\s)cd ").unwrap());

static INVALID_REDIRECT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::create_regex(r"\s(<|<<|>>|>)\s").unwrap());

static INVALID_PIPE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::create_regex(r"\s\|\s").unwrap());

static INVALID_OPERATOR: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::create_regex(r"\s(\|\||;;)\s").unwrap());

fn is_bash_script(arg: &str) -> bool {
    arg.ends_with(".sh")
}

fn is_node_script(arg: &str) -> bool {
    arg.ends_with(".js") || arg.ends_with(".cjs") || arg.ends_with(".mjs")
}

pub fn should_run_in_ci(name: &str, script: &str) -> bool {
    if INFO_OPTIONS.is_match(script) {
        return true;
    }

    if script.contains("--watch")
        || DEV_ONLY_NAME.is_match(name)
        || DEV_COMMAND.is_match(script)
        || DEV_COMMAND_SOLO.is_match(script)
    {
        return false;
    }

    true
}

fn clean_env_var(pair: &str) -> (String, String) {
    let mut parts = pair.split('=');
    let key = parts.next().unwrap();
    let mut val = parts.next().unwrap_or_default();

    if val.ends_with(';') {
        val = &val[0..(val.len() - 1)];
    }

    (key.to_owned(), val.to_owned())
}

fn clean_output_path(target_id: &str, output: &str) -> miette::Result<String> {
    if output.starts_with("..") {
        Err(miette::miette!(
            "Task outputs must be project relative and cannot traverse upwards. Found {} in {}.",
            output,
            target_id,
        ))
    } else if output.starts_with('/') || WIN_DRIVE.is_match(output) {
        Err(miette::miette!(
            "Task outputs must be project relative and cannot be absolute. Found {} in {}.",
            output,
            target_id,
        ))
    } else if output.starts_with("./") || output.starts_with(".\\") {
        Ok(output[2..].to_owned())
    } else {
        Ok(output.to_owned())
    }
}

fn clean_script_name(name: &str) -> String {
    ID_CLEAN.replace_all(name, "-").to_string()
}

pub enum TaskContext {
    ConvertToTask,
    WrapRunScript,
}

pub fn create_task(
    target_id: &str,
    script_name: &str,
    script: &str,
    context: TaskContext,
    toolchain: &Id,
    pm: NodePackageManager,
) -> miette::Result<PartialTaskConfig> {
    let is_wrapping = matches!(context, TaskContext::WrapRunScript);
    let script_args = split_args(script)?;
    let mut task_config = PartialTaskConfig::default();
    let mut args = vec![];
    let mut outputs = vec![];
    let mut env = FxHashMap::default();

    for (index, arg) in script_args.iter().enumerate() {
        if arg == ";" {
            continue;
        }

        // Extract environment variables
        if ARG_ENV_VAR.is_match(arg) {
            let (key, val) = clean_env_var(arg);

            env.insert(key, val);

            continue;
        }

        // Detect possible outputs
        if ARG_OUTPUT_FLAG.is_match(arg) {
            if let Some(output) = script_args.get(index + 1) {
                outputs.push(OutputPath::ProjectFile(clean_output_path(
                    target_id, output,
                )?));
            }
        }

        if !is_wrapping {
            args.push(arg.to_owned());
        }
    }

    if is_wrapping {
        task_config.toolchain = Some(OneOrMany::One(toolchain.to_owned()));
        task_config.command = Some(PartialTaskArgs::List(string_vec![
            match pm {
                NodePackageManager::Bun => "bun",
                NodePackageManager::Npm => "npm",
                NodePackageManager::Pnpm => "pnpm",
                NodePackageManager::Yarn => "yarn",
            },
            "run",
            script_name
        ]));
    } else {
        if let Some(command) = args.first() {
            if is_bash_script(command) {
                args.insert(0, "bash".to_owned());
            } else if is_node_script(command) {
                args.insert(0, toolchain.as_str().to_owned());
            } else {
                // Already there
            }
        } else {
            args.insert(0, "noop".to_owned());
        }

        task_config.toolchain = Some(OneOrMany::One(
            if is_system_command(&args[0]) || &args[0] == "noop" {
                Id::raw("system")
            } else {
                toolchain.to_owned()
            },
        ));
        task_config.command = Some(if args.len() == 1 {
            PartialTaskArgs::String(args.remove(0))
        } else {
            PartialTaskArgs::List(args)
        });
    }

    if !env.is_empty() {
        task_config.env = Some(env);
    }

    if !outputs.is_empty() {
        task_config.outputs = Some(outputs);
    }

    #[allow(deprecated)]
    if !should_run_in_ci(script_name, script) {
        task_config.local = Some(true);
    }

    debug!(
        "Creating task {} {}",
        color::label(target_id),
        color::muted_light(format!("(for script {})", color::symbol(script_name)))
    );

    Ok(task_config)
}

pub struct ScriptParser<'a> {
    /// Life cycle events like "prepublishOnly".
    life_cycles: ScriptsMap,

    /// Script names -> task IDs.
    names_to_ids: FxHashMap<String, Id>,

    /// Scripts that started with "post".
    post: ScriptsMap,

    /// Scripts that started with "pre".
    pre: ScriptsMap,

    /// The project being parsed.
    project_id: &'a str,

    /// Scripts being parsed.
    scripts: ScriptsMap,

    /// Tasks that have been parsed and converted from scripts.
    pub tasks: BTreeMap<Id, PartialTaskConfig>,

    /// Scripts that ran into issues while parsing.
    unresolved_scripts: ScriptsMap,

    toolchain: Id,
    pm: NodePackageManager,
}

impl<'a> ScriptParser<'a> {
    pub fn new(project_id: &'a str, toolchain: Id, pm: NodePackageManager) -> Self {
        ScriptParser {
            life_cycles: ScriptsMap::default(),
            names_to_ids: FxHashMap::default(),
            post: ScriptsMap::default(),
            pre: ScriptsMap::default(),
            project_id,
            scripts: ScriptsMap::default(),
            tasks: BTreeMap::new(),
            unresolved_scripts: ScriptsMap::default(),
            toolchain,
            pm,
        }
    }

    pub fn infer_scripts(&mut self, package_json: &PackageJsonCache) -> miette::Result<()> {
        let scripts = match &package_json.data.scripts {
            Some(s) => s.clone(),
            None => {
                return Ok(());
            }
        };

        for (name, script) in &scripts {
            if PM_LIFE_CYCLES.is_match(name) || name.starts_with("pre") || name.starts_with("post")
            {
                continue;
            }

            let task_id = clean_script_name(name);
            let target_id = format!("{}:{}", self.project_id, task_id);

            self.tasks.insert(
                Id::new(&task_id)?,
                create_task(
                    &target_id,
                    name,
                    script,
                    TaskContext::WrapRunScript,
                    &self.toolchain,
                    self.pm,
                )?,
            );
        }

        Ok(())
    }

    pub fn update_package(&mut self, package_json: &mut PackageJsonCache) -> miette::Result<()> {
        let mut scripts = ScriptsMap::default();

        for (name, script) in &self.life_cycles {
            scripts.insert(name.to_owned(), self.replace_run_commands(script));
        }

        package_json.set_scripts(scripts);

        Ok(())
    }

    pub fn parse_scripts(&mut self, package_json: &PackageJsonCache) -> miette::Result<()> {
        let scripts = match &package_json.data.scripts {
            Some(s) => s.clone(),
            None => {
                return Ok(());
            }
        };

        // First pass:
        //  - Remove unsupported scripts
        //  - Extract hooks and life cycles
        //  - Convert stand-alone scripts
        //  - Retain && operators
        let mut standalone_scripts = FxHashMap::default();

        for (name, script) in &scripts {
            if PM_LIFE_CYCLES.is_match(name) {
                self.life_cycles.insert(name.clone(), script.clone());
                continue;
            }

            if let Some(real_name) = name.strip_prefix("pre") {
                self.pre.insert(real_name.to_owned(), script.clone());
                continue;
            }

            if let Some(real_name) = name.strip_prefix("post") {
                self.post.insert(real_name.to_owned(), script.clone());
                continue;
            }

            // Do not allow "cd ..."
            if INVALID_CD.is_match(script) {
                warn!(
                    "Changing directories (cd ...) is not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable to handle it: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name, self.project_id,
                );

                continue;
            }

            // Rust commands do not support redirects natively
            if INVALID_REDIRECT.is_match(script) {
                warn!(
                    "Redirects (<, >, etc) are not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable that does the redirect: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name, self.project_id,
                );

                continue;
            }

            // Rust commands do not support pipes natively
            if INVALID_PIPE.is_match(script) {
                warn!(
                    "Pipes (|) are not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable that does the piping: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name, self.project_id,
                );

                continue;
            }

            // Rust commands do not support operators natively
            if INVALID_OPERATOR.is_match(script) {
                warn!(
                    "OR operator (||) is not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable to handle it: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name, self.project_id,
                );

                continue;
            }

            // Defer "npm run", "yarn run", and any "&&" usage, etc till the next pass
            if PM_RUN_COMMAND.is_match(script) || script.contains("&&") {
                self.scripts.insert(name.clone(), script.clone());
                continue;
            }

            // Stand-alone script? Hopefully...
            standalone_scripts.insert(name.clone(), script.clone());
        }

        for (name, script) in &standalone_scripts {
            self.create_task(name, script)?;
        }

        // Second pass:
        //  - Convert scripts that use "npm run", etc
        //  - Retain && operators
        let mut multi_scripts = FxHashMap::default();
        let mut run_scripts = FxHashMap::default();

        self.scripts.retain(|name, script| {
            if script.contains("&&") {
                multi_scripts.insert(name.clone(), script.clone());
            } else {
                run_scripts.insert(name.clone(), script.clone());
            }

            false
        });

        for (name, script) in &run_scripts {
            self.create_task_from_run(name, script)?;
        }

        // Third pass:
        //  - Convert scripts that contain &&
        //  - These are quite complex and require special treatmeant
        for (name, script) in &multi_scripts {
            self.create_task_from_multiple(name, script)?;
        }

        // Last pass:
        //  - Try to parse unresolved scripts again.
        //  - Hook up pre/post hooks
        for (name, script) in self.unresolved_scripts.clone() {
            self.parse_script(&name, &script)?;
        }

        for (script_name, task_id) in self.names_to_ids.clone() {
            self.apply_pre_post_hooks(&script_name, &task_id)?;
        }

        Ok(())
    }

    pub fn parse_script<K: AsRef<str>, V: AsRef<str>>(
        &mut self,
        name: K,
        value: V,
    ) -> miette::Result<Option<Id>> {
        let name = name.as_ref();
        let value = value.as_ref();

        Ok(if value.contains("&&") {
            self.create_task_from_multiple(name, value)?
        } else if PM_RUN_COMMAND.is_match(value) {
            self.create_task_from_run(name, value)?
        } else {
            Some(self.create_task(name, value)?)
        })
    }

    pub fn create_task<K: AsRef<str>, V: AsRef<str>>(
        &mut self,
        name: K,
        value: V,
    ) -> miette::Result<Id> {
        let name = name.as_ref();
        let value = value.as_ref();
        let task_id = Id::new(clean_script_name(name))?;
        let target_id = format!("{}:{}", self.project_id, task_id);

        self.names_to_ids.insert(name.to_owned(), task_id.clone());

        self.tasks.insert(
            task_id.clone(),
            create_task(
                &target_id,
                name,
                value,
                TaskContext::ConvertToTask,
                &self.toolchain,
                self.pm,
            )?,
        );

        Ok(task_id)
    }

    #[track_caller]
    pub fn create_task_from_multiple<T: AsRef<str>>(
        &mut self,
        name: T,
        value: &str,
    ) -> miette::Result<Option<Id>> {
        let name = name.as_ref();
        let scripts: Vec<_> = value.split("&&").map(|v| v.trim()).collect();
        let mut previous_task_id = Id::raw("");

        // Scripts need to be chained as deps instead of ran in parallel
        for (index, script) in scripts.iter().enumerate() {
            if let Some(task_id) = self.parse_script(
                if index == scripts.len() - 1 {
                    name.to_owned()
                } else {
                    format!("{}-dep{}", name, index + 1)
                },
                script,
            )? {
                if !previous_task_id.is_empty() {
                    if let Some(task) = self.tasks.get_mut(&task_id) {
                        task.deps
                            .get_or_insert(vec![])
                            .push(PartialTaskDependency::Target(Target::new_self(
                                previous_task_id,
                            )?));
                    }
                }

                previous_task_id = task_id;
            }
        }

        if previous_task_id.is_empty() {
            return Ok(None);
        }

        Ok(Some(previous_task_id))
    }

    #[track_caller]
    pub fn create_task_from_run<T: AsRef<str>>(
        &mut self,
        name: T,
        value: &str,
    ) -> miette::Result<Option<Id>> {
        let name = name.as_ref();

        let caps = PM_RUN_COMMAND.captures(value).unwrap();
        let run_script_name = caps.get(1).unwrap().as_str().to_owned();

        if self.names_to_ids.contains_key(&run_script_name) {
            let script = self.replace_run_commands(value);
            let task_id = self.create_task(name, script)?;

            return Ok(Some(task_id));
        }

        self.unresolved_scripts
            .insert(name.to_owned(), value.to_owned());

        Ok(None)
    }

    fn apply_pre_post_hooks(&mut self, script_name: &str, task_id: &Id) -> miette::Result<()> {
        // Convert pre hooks as `deps`
        if self.pre.contains_key(script_name) {
            let pre = self.pre.swap_remove(script_name).unwrap();

            if let Some(pre_task_id) = self.parse_script(format!("pre{script_name}"), pre)? {
                if let Some(task) = self.tasks.get_mut(task_id) {
                    task.deps
                        .get_or_insert(vec![])
                        .push(PartialTaskDependency::Target(Target::new_self(
                            pre_task_id,
                        )?));
                }
            }
        }

        // Use this task as a `deps` for post hooks
        if self.post.contains_key(script_name) {
            let post = self.post.swap_remove(script_name).unwrap();

            if let Some(post_task_id) = self.parse_script(format!("post{script_name}"), post)? {
                if let Some(task) = self.tasks.get_mut(&post_task_id) {
                    task.deps
                        .get_or_insert(vec![])
                        .push(PartialTaskDependency::Target(Target::new_self(task_id)?));
                }
            }
        }

        Ok(())
    }

    fn replace_run_commands(&self, script: &str) -> String {
        PM_RUN_COMMAND
            .replace_all(script, |caps: &regex::Captures| {
                let run_script_name = caps.get(1).unwrap().as_str();
                let run_args = match caps.get(2) {
                    Some(v) => {
                        if v.as_str() == " --" {
                            ""
                        } else {
                            v.as_str()
                        }
                    }
                    None => "",
                };

                let has_delimiter = run_args.starts_with("-- ") || run_args.starts_with(" -- ");
                let has_args = !run_args.is_empty() && run_args != " ";

                match self.names_to_ids.get(run_script_name) {
                    Some(task_id) => format!(
                        "moon run {}:{}{}{}",
                        self.project_id,
                        task_id,
                        if !has_delimiter && has_args {
                            " --"
                        } else {
                            ""
                        },
                        run_args
                    ),
                    None => caps.get(0).unwrap().as_str().to_owned(),
                }
            })
            .to_string()
    }
}
