use moon_app_context::AppContext;
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::{
    ExecCommandInput, Extend, ExtendCommandInput, ExtendCommandOutput, ExtendTaskCommandInput,
    ExtendTaskScriptInput, ExtendTaskScriptOutput,
};
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_toolchain::{
    get_version_env_key, get_version_env_value, is_using_global_toolchain,
    is_using_global_toolchains,
};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

// Order of operations:
// - For task based commands:
//   - Inherit exe, args, and env from task
//   - Extend with `extend_command` plugin calls
//   - Extend with `extend_task_command` or `extend_task_script` plugin calls
// - For general commands:
//   - Inherit exe, args, and env from caller
//   - Extend with `extend_command` plugin calls

// Path ordering:
// - Plugin `extend_*` injected paths
// - Toolchain executable paths
// - proto store/shims/bin paths
// - moon store paths

#[derive(Default)]
pub struct CommandAugment {
    pub exe: String,
    pub args: VecDeque<String>,
    pub env: FxHashMap<String, String>,
    pub env_remove: Vec<String>,
    pub paths: VecDeque<PathBuf>,
    pub paths_store: VecDeque<PathBuf>,
}

impl CommandAugment {
    pub fn apply_command_outputs(&mut self, outputs: Vec<ExtendCommandOutput>) {
        for output in outputs {
            if let Some(new_exe) = output.command {
                self.exe = new_exe;
            }

            if let Some(new_args) = output.args {
                self.extend_args(new_args);
            }

            self.remove_env(output.env_remove);
            self.extend_env(output.env);
            self.extend_paths(output.paths);
        }
    }

    pub fn apply_script_outputs(&mut self, outputs: Vec<ExtendTaskScriptOutput>) {
        for output in outputs {
            if let Some(new_script) = output.script {
                self.exe = new_script;
            }

            self.remove_env(output.env_remove);
            self.extend_env(output.env);
            self.extend_paths(output.paths);
        }
    }

    pub fn extend_args(&mut self, args: Extend<Vec<String>>) {
        match args {
            Extend::Empty => {
                self.args.clear();
            }
            Extend::Append(next) => {
                self.args.extend(next);
            }
            Extend::Prepend(next) => {
                for arg in next.into_iter().rev() {
                    self.args.push_front(arg);
                }
            }
            Extend::Replace(next) => {
                self.args.clear();
                self.args.extend(next);
            }
        }
    }

    pub fn extend_env(&mut self, env: FxHashMap<String, String>) {
        self.env.extend(env);
    }

    pub fn extend_paths(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        // Normalize separators since WASM always uses forward slashes
        #[cfg(windows)]
        let paths = paths.into_iter().map(|path| {
            PathBuf::from(moon_common::path::normalize_separators(
                path.to_string_lossy(),
            ))
        });

        for path in paths.into_iter().rev() {
            self.paths.push_front(path);
        }
    }

    pub fn remove_env(&mut self, env_remove: Vec<String>) {
        self.env_remove.extend(env_remove);
    }
}

pub struct CommandAugmenter<'app> {
    augment: CommandAugment,
    bag: &'app GlobalEnvBag,
    context: &'app AppContext,
}

impl<'app> CommandAugmenter<'app> {
    pub fn new(context: &'app AppContext, bag: &'app GlobalEnvBag, exe: impl AsRef<str>) -> Self {
        CommandAugmenter {
            augment: CommandAugment {
                exe: exe.as_ref().to_string(),
                ..Default::default()
            },
            bag,
            context,
        }
    }

    pub fn from_input(
        context: &'app AppContext,
        bag: &'app GlobalEnvBag,
        input: &ExecCommandInput,
    ) -> Self {
        let mut augment = Self::new(context, bag, &input.command);
        augment.args.extend(input.args.clone());
        augment.env.extend(input.env.clone());
        augment
    }

    pub fn from_task(context: &'app AppContext, bag: &'app GlobalEnvBag, task: &Task) -> Self {
        let mut augment = Self::new(
            context,
            bag,
            task.script.clone().unwrap_or_else(|| task.command.clone()),
        );

        if task.script.is_none() {
            augment.args.extend(task.args.clone());
        }

        augment.env.extend(task.env.clone());
        augment
    }

    pub fn create_command(self) -> Command {
        let mut command = Command::new(self.augment.exe.clone());

        self.augment_command(&mut command);

        command
    }

    pub fn augment_command(self, command: &mut Command) {
        command.args(self.augment.args);
        command.prepend_paths(self.augment.paths);
        command.append_paths(self.augment.paths_store);
        command.envs_if_not_global(self.augment.env);

        for key in self.augment.env_remove {
            command.env_remove(key);
        }
    }

    /// Inherit command augmentations (args, env, paths) from extension and toolchain plugins
    /// based on the provided project and task. Refer to the order of operations above.
    pub async fn inherit_from_plugins(
        &mut self,
        project: Option<&Project>,
        task: Option<&Task>,
    ) -> miette::Result<()> {
        let toolchain_ids = match (project, task) {
            (Some(p), Some(t)) => p.get_enabled_toolchains_for_task(t),
            (Some(p), None) => p.get_enabled_toolchains(),
            _ => vec![],
        };

        let current_dir = if let Some(p) = project {
            &p.root
        } else {
            &self.context.working_dir
        };

        // Inherit for shared
        self.augment.apply_command_outputs(
            self.context
                .toolchain_registry
                .extend_command_many(toolchain_ids.clone(), |registry, toolchain| {
                    ExtendCommandInput {
                        context: registry.create_context(),
                        command: self.augment.exe.clone(),
                        args: self.args.clone().into_iter().collect(),
                        current_dir: registry.to_virtual_path(current_dir),
                        toolchain_config: match project {
                            Some(p) => registry.create_merged_config(&toolchain.id, &p.config),
                            None => registry.create_config(&toolchain.id),
                        },
                        ..Default::default()
                    }
                })
                .await?,
        );

        self.augment.apply_command_outputs(
            self.context
                .extension_registry
                .extend_command_all(|registry, extension| ExtendCommandInput {
                    context: registry.create_context(),
                    command: self.augment.exe.clone(),
                    args: self.augment.args.clone().into_iter().collect(),
                    current_dir: registry.to_virtual_path(current_dir),
                    extension_config: registry.create_config(&extension.id),
                    ..Default::default()
                })
                .await?,
        );

        // Inherit for task specific
        if let Some(project) = project
            && let Some(task) = task
        {
            if task.script.is_some() {
                // Scripts don't use arguments
                self.args.clear();

                self.augment.apply_script_outputs(
                    self.context
                        .toolchain_registry
                        .extend_task_script_many(toolchain_ids, |registry, toolchain| {
                            ExtendTaskScriptInput {
                                context: registry.create_context(),
                                script: self.augment.exe.clone(),
                                project: project.to_fragment(),
                                task: task.to_fragment(),
                                toolchain_config: registry
                                    .create_merged_config(&toolchain.id, &project.config),
                                ..Default::default()
                            }
                        })
                        .await?,
                );

                self.augment.apply_script_outputs(
                    self.context
                        .extension_registry
                        .extend_task_script_all(|registry, extension| ExtendTaskScriptInput {
                            context: registry.create_context(),
                            script: self.augment.exe.clone(),
                            project: project.to_fragment(),
                            task: task.to_fragment(),
                            extension_config: registry.create_config(&extension.id),
                            ..Default::default()
                        })
                        .await?,
                );
            } else {
                self.augment.apply_command_outputs(
                    self.context
                        .toolchain_registry
                        .extend_task_command_many(toolchain_ids, |registry, toolchain| {
                            ExtendTaskCommandInput {
                                context: registry.create_context(),
                                command: self.augment.exe.clone(),
                                args: self.augment.args.clone().into_iter().collect(),
                                project: project.to_fragment(),
                                task: task.to_fragment(),
                                toolchain_config: registry
                                    .create_merged_config(&toolchain.id, &project.config),
                                ..Default::default()
                            }
                        })
                        .await?,
                );

                self.augment.apply_command_outputs(
                    self.context
                        .extension_registry
                        .extend_task_command_all(|registry, extension| ExtendTaskCommandInput {
                            context: registry.create_context(),
                            command: self.augment.exe.clone(),
                            args: self.augment.args.clone().into_iter().collect(),
                            project: project.to_fragment(),
                            task: task.to_fragment(),
                            extension_config: registry.create_config(&extension.id),
                            ..Default::default()
                        })
                        .await?,
                );
            }
        }

        self.inherit_from_toolchains(project).await?;

        Ok(())
    }

    async fn inherit_from_toolchains(&mut self, project: Option<&Project>) -> miette::Result<()> {
        let mut map = FxHashMap::default();

        // First pass, gather all version enabled toolchains
        for (id, config) in &self.context.toolchains_config.plugins {
            if let Some(version) = &config.version
                && !is_using_global_toolchain(self.bag, id)
            {
                map.insert(id, version);
            }
        }

        // Second pass, gather and filter based on the project
        if let Some(project_config) = project.as_ref().map(|p| &p.config) {
            for (id, config) in &project_config.toolchains.plugins {
                if !config.is_enabled() {
                    map.remove(id);
                    continue;
                }

                if let Some(version) = config.get_version()
                    && !is_using_global_toolchain(self.bag, id)
                {
                    map.insert(id, version);
                }
            }
        }

        if map.is_empty() {
            return Ok(());
        }

        // Add each toolchain as an env var
        for (id, version) in &map {
            self.augment
                .env
                .insert(get_version_env_key(id), get_version_env_value(version));
        }

        // If forced to globals, don't inject any paths but keep env vars
        if is_using_global_toolchains(self.bag) {
            return Ok(());
        }

        // Add toolchain specific paths
        if !map.is_empty() {
            let paths = self
                .context
                .toolchain_registry
                .get_command_paths(map.keys().copied().collect(), |_, toolchain| {
                    map.get(&toolchain.id).map(|version| (*version).to_owned())
                })
                .await?;

            // They go after the extended paths above
            self.augment.paths.extend(paths);
        }

        // Inherit proto/moon last
        self.inherit_for_proto();

        Ok(())
    }

    pub fn inherit_for_proto(&mut self) {
        let proto_version = self.context.toolchains_config.proto.version.to_string();

        // Inherit common proto env vars
        self.env.insert("PROTO_AUTO_INSTALL".into(), "false".into());
        self.env
            .insert("PROTO_IGNORE_MIGRATE_WARNING".into(), "true".into());
        self.env.insert("PROTO_NO_PROGRESS".into(), "true".into());
        self.env
            .insert("PROTO_VERSION".into(), proto_version.clone());
        self.env.insert("STARBASE_FORCE_TTY".into(), "true".into());

        // If not using globals, inherit proto and moon paths
        if !is_using_global_toolchains(self.bag) {
            let moon = &self.context.toolchain_registry.host_data.moon_env;
            let proto = &self.context.toolchain_registry.host_data.proto_env;

            self.augment.paths_store.extend([
                proto.store.inventory_dir.join("proto").join(proto_version),
                proto.store.shims_dir.clone(),
                proto.store.bin_dir.clone(),
                moon.store_root.join("bin"),
            ]);
        }
    }
}

impl Deref for CommandAugmenter<'_> {
    type Target = CommandAugment;

    fn deref(&self) -> &Self::Target {
        &self.augment
    }
}

impl DerefMut for CommandAugmenter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.augment
    }
}
