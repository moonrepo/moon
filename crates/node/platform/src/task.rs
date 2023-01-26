use moon_config::{TaskCommandArgs, TaskConfig, TasksConfigsMap};
use moon_logger::{color, debug, warn};
use moon_node_lang::package::{PackageJson, ScriptsSet};
use moon_target::Target;
use moon_task::{PlatformType, TaskError, TaskID};
use moon_utils::regex::{UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};
use moon_utils::{lazy_static, process, regex, string_vec};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

const LOG_TARGET: &str = "moon:node-platform:tasks";

pub type ScriptsMap = FxHashMap<String, String>;

lazy_static! {
    pub static ref WIN_DRIVE: regex::Regex = regex::create_regex(r#"^[A-Z]:"#).unwrap();

    pub static ref ARG_ENV_VAR: regex::Regex = regex::create_regex(r#"^[A-Z0-9_]+="#).unwrap();

    pub static ref ARG_OUTPUT_FLAG: regex::Regex =
        regex::create_regex(r#"^(-o|--(out|output|dist)(-{0,1}(?i:dir|file))?)$"#).unwrap();

    pub static ref INFO_OPTIONS: regex::Regex =
        regex::create_regex(r#"--(help|version)"#)
            .unwrap();

    // This isn't exhaustive but captures very popular tools
    pub static ref DEV_COMMAND: regex::Regex =
        regex::create_regex(r#"(astro (dev|preview))|(concurrently)|(gatsby (new|dev|develop|serve|repl))|(next (dev|start))|(nuxt (dev|preview))|(packemon watch)|(parcel [^build])|(react-scripts start)|(snowpack dev)|(vite (dev|preview|serve))|(vue-cli-service serve)|(webpack (s|serve|server|w|watch|-))"#)
            .unwrap();

    pub static ref DEV_COMMAND_SOLO: regex::Regex =
            regex::create_regex(r#"^(npx |yarn dlx |pnpm dlx )?(parcel|vite|webpack)$"#)
                .unwrap();

    pub static ref DEV_ONLY_NAME: regex::Regex =
            regex::create_regex(r#"(^(dev|start|serve|preview)$)|(^(start|serve|preview):)|(:(start|serve|preview)$)"#)
                .unwrap();

    // Special package manager handling
    pub static ref PM_RUN_COMMAND: regex::Regex = regex::create_regex(r#"(?:npm|pnpm|yarn) run ([a-zA-Z0-9:-_]+)([^&]+)?"#)
        .unwrap();

    pub static ref PM_LIFE_CYCLES: regex::Regex = regex::create_regex(r#"^(preprepare|prepare|postprepare|prepublish|prepublishOnly|publish|postpublish|prepack|pack|postpack|preinstall|install|postinstall|preversion|version|postversion|dependencies)$"#)
        .unwrap();

    // These patterns are currently not allowed
    pub static ref INVALID_CD: regex::Regex = regex::create_regex(r#"(^|\b|\s)cd "#).unwrap();
    pub static ref INVALID_REDIRECT: regex::Regex = regex::create_regex(r#"\s(<|<<|>>|>)\s"#).unwrap();
    pub static ref INVALID_PIPE: regex::Regex = regex::create_regex(r#"\s\|\s"#).unwrap();
    pub static ref INVALID_OPERATOR: regex::Regex = regex::create_regex(r#"\s(\|\||;;)\s"#).unwrap();

    pub static ref TASK_ID_CHARS: regex::Regex = regex::create_regex(r#"[^a-zA-Z0-9-_]+"#).unwrap();
}

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

fn clean_output_path(target_id: &str, output: &str) -> Result<String, TaskError> {
    if output.starts_with("..") {
        Err(TaskError::NoParentOutput(
            output.to_owned(),
            target_id.to_owned(),
        ))
    } else if output.starts_with('/') || WIN_DRIVE.is_match(output) {
        Err(TaskError::NoAbsoluteOutput(
            output.to_owned(),
            target_id.to_owned(),
        ))
    } else if output.starts_with("./") || output.starts_with(".\\") {
        Ok(output[2..].to_owned())
    } else {
        Ok(output.to_owned())
    }
}

fn clean_script_name(name: &str) -> String {
    TASK_ID_CHARS.replace_all(name, "-").to_string()
}

fn detect_platform_type(command: &str) -> PlatformType {
    if UNIX_SYSTEM_COMMAND.is_match(command)
        || WINDOWS_SYSTEM_COMMAND.is_match(command)
        || command == "noop"
    {
        return PlatformType::System;
    }

    PlatformType::Node
}

fn add_task_dep(config: &mut TaskConfig, dep: String) {
    if let Some(deps) = &mut config.deps {
        deps.push(dep);
    } else {
        config.deps = Some(vec![dep]);
    }
}

pub enum TaskContext {
    ConvertToTask,
    WrapRunScript,
}

#[track_caller]
pub fn create_task(
    target_id: &str,
    script_name: &str,
    script: &str,
    context: TaskContext,
) -> Result<TaskConfig, TaskError> {
    let is_wrapping = matches!(context, TaskContext::WrapRunScript);
    let script_args = process::split_args(script)?;
    let mut task_config = TaskConfig::default();
    let mut args = vec![];
    let mut outputs = vec![];
    let mut env = FxHashMap::default();

    for (index, arg) in script_args.iter().enumerate() {
        // Extract environment variables
        if ARG_ENV_VAR.is_match(arg) {
            let (key, val) = clean_env_var(arg);

            env.insert(key, val);

            continue;
        }

        // Detect possible outputs
        if ARG_OUTPUT_FLAG.is_match(arg) {
            if let Some(output) = script_args.get(index + 1) {
                outputs.push(clean_output_path(target_id, output)?);
            }
        }

        if !is_wrapping {
            args.push(arg.to_owned());
        }
    }

    if is_wrapping {
        task_config.platform = PlatformType::Node;
        task_config.command = Some(TaskCommandArgs::Sequence(string_vec![
            "moon",
            "node",
            "run-script",
            script_name
        ]));
    } else {
        if let Some(command) = args.get(0) {
            if is_bash_script(command) {
                args.insert(0, "bash".to_owned());
            } else if is_node_script(command) {
                args.insert(0, "node".to_owned());
            } else {
                // Already there
            }
        } else {
            args.insert(0, "noop".to_owned());
        }

        task_config.platform = detect_platform_type(&args[0]);
        task_config.command = Some(if args.len() == 1 {
            TaskCommandArgs::String(args.remove(0))
        } else {
            TaskCommandArgs::Sequence(args)
        });
    }

    if !env.is_empty() {
        task_config.env = Some(env);
    }

    if !outputs.is_empty() {
        task_config.outputs = Some(outputs);
    }

    task_config.local = !should_run_in_ci(script_name, script);

    debug!(
        target: LOG_TARGET,
        "Creating task {} {}",
        color::target(target_id),
        color::muted_light(format!("(for script {})", color::symbol(script_name)))
    );

    Ok(task_config)
}

pub struct ScriptParser<'a> {
    /// Life cycle events like "prepublishOnly".
    life_cycles: ScriptsMap,

    /// Script names -> task IDs.
    names_to_ids: FxHashMap<String, String>,

    /// Scripts that started with "post".
    post: ScriptsMap,

    /// Scripts that started with "pre".
    pre: ScriptsMap,

    /// The project being parsed.
    project_id: &'a str,

    /// Scripts being parsed.
    scripts: ScriptsMap,

    /// Tasks that have been parsed and converted from scripts.
    pub tasks: TasksConfigsMap,

    /// Scripts that ran into issues while parsing.
    unresolved_scripts: ScriptsMap,
}

impl<'a> ScriptParser<'a> {
    pub fn new(project_id: &'a str) -> Self {
        ScriptParser {
            life_cycles: FxHashMap::default(),
            names_to_ids: FxHashMap::default(),
            post: FxHashMap::default(),
            pre: FxHashMap::default(),
            project_id,
            scripts: FxHashMap::default(),
            tasks: BTreeMap::new(),
            unresolved_scripts: FxHashMap::default(),
        }
    }

    pub fn infer_scripts(&mut self, package_json: &PackageJson) -> Result<(), TaskError> {
        let scripts = match &package_json.scripts {
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
            let target_id = Target::format(self.project_id, &task_id)?;

            self.tasks.insert(
                task_id,
                create_task(&target_id, name, script, TaskContext::WrapRunScript)?,
            );
        }

        Ok(())
    }

    pub fn update_package(&mut self, package_json: &mut PackageJson) -> Result<(), TaskError> {
        let mut scripts: ScriptsSet = BTreeMap::new();

        for (name, script) in &self.life_cycles {
            scripts.insert(name.to_owned(), self.replace_run_commands(script));
        }

        package_json.set_scripts(scripts);

        Ok(())
    }

    pub fn parse_scripts(&mut self, package_json: &PackageJson) -> Result<(), TaskError> {
        let scripts = match &package_json.scripts {
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

            if name.starts_with("pre") {
                self.pre
                    .insert(name.strip_prefix("pre").unwrap().to_owned(), script.clone());
                continue;
            }

            if name.starts_with("post") {
                self.post.insert(
                    name.strip_prefix("post").unwrap().to_owned(),
                    script.clone(),
                );
                continue;
            }

            // Do not allow "cd ..."
            if INVALID_CD.is_match(script) {
                warn!(
                    target: LOG_TARGET,
                    "Changing directories (cd ...) is not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable to handle it: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name,
                    self.project_id,
                );

                continue;
            }

            // Rust commands do not support redirects natively
            if INVALID_REDIRECT.is_match(script) {
                warn!(
                    target: LOG_TARGET,
                    "Redirects (<, >, etc) are not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable that does the redirect: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name,
                    self.project_id,
                );

                continue;
            }

            // Rust commands do not support pipes natively
            if INVALID_PIPE.is_match(script) {
                warn!(
                    target: LOG_TARGET,
                    "Pipes (|) are not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable that does the piping: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name,
                    self.project_id,
                );

                continue;
            }

            // Rust commands do not support operators natively
            if INVALID_OPERATOR.is_match(script) {
                warn!(
                    target: LOG_TARGET,
                    "OR operator (||) is not supported by moon, skipping script \"{}\" for project \"{}\". As an alternative, create an executable to handle it: https://moonrepo.dev/docs/faq#how-to-pipe-or-redirect-tasks",
                    name,
                    self.project_id,
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
    ) -> Result<Option<TaskID>, TaskError> {
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
    ) -> Result<TaskID, TaskError> {
        let name = name.as_ref();
        let value = value.as_ref();
        let task_id = clean_script_name(name);
        let target_id = Target::format(self.project_id, &task_id)?;

        self.names_to_ids.insert(name.to_owned(), task_id.clone());

        self.tasks.insert(
            task_id.clone(),
            create_task(&target_id, name, value, TaskContext::ConvertToTask)?,
        );

        Ok(task_id)
    }

    #[track_caller]
    pub fn create_task_from_multiple<T: AsRef<str>>(
        &mut self,
        name: T,
        value: &str,
    ) -> Result<Option<TaskID>, TaskError> {
        let name = name.as_ref();
        let scripts: Vec<_> = value.split("&&").map(|v| v.trim()).collect();
        let mut previous_task_id = String::new();

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
                        add_task_dep(task, format!("~:{previous_task_id}"));
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
    ) -> Result<Option<TaskID>, TaskError> {
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

    fn apply_pre_post_hooks(&mut self, script_name: &str, task_id: &str) -> Result<(), TaskError> {
        // Convert pre hooks as `deps`
        if self.pre.contains_key(script_name) {
            let pre = self.pre.remove(script_name).unwrap();

            if let Some(pre_task_id) = self.parse_script(format!("pre{script_name}"), pre)? {
                if let Some(task) = self.tasks.get_mut(task_id) {
                    add_task_dep(task, format!("~:{pre_task_id}"));
                }
            }
        }

        // Use this task as a `deps` for post hooks
        if self.post.contains_key(script_name) {
            let post = self.post.remove(script_name).unwrap();

            if let Some(post_task_id) = self.parse_script(format!("post{script_name}"), post)? {
                if let Some(task) = self.tasks.get_mut(&post_task_id) {
                    add_task_dep(task, format!("~:{task_id}"));
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
