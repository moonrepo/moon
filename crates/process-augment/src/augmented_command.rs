use moon_app_context::AppContext;
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::{
    ExecCommandInput, Extend, ExtendCommandInput, ExtendCommandOutput, ExtendTaskCommandInput,
    ExtendTaskScriptInput, ExtendTaskScriptOutput,
};
use moon_process::{Command, CommandEnvMode};
use moon_project::Project;
use moon_task::Task;
use moon_toolchain::{
    get_version_env_key, get_version_env_value, is_using_global_toolchain,
    is_using_global_toolchains,
};
use rustc_hash::FxHashMap;
use std::ffi::{OsStr, OsString};
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

pub struct AugmentedCommand<'app> {
    command: Command,
    bag: &'app GlobalEnvBag,
    context: &'app AppContext,
}

impl<'app> AugmentedCommand<'app> {
    pub fn new(context: &'app AppContext, bag: &'app GlobalEnvBag, bin: impl AsRef<OsStr>) -> Self {
        AugmentedCommand {
            command: Command::new(bin),
            bag,
            context,
        }
    }

    pub fn from_input(
        context: &'app AppContext,
        bag: &'app GlobalEnvBag,
        input: &ExecCommandInput,
    ) -> Self {
        let mut builder = Self::new(context, bag, &input.command);
        builder.args(&input.args);
        builder.envs(&input.env);
        builder
    }

    pub fn from_task(context: &'app AppContext, bag: &'app GlobalEnvBag, task: &Task) -> Self {
        let mut builder = Self::new(context, bag, &task.command);

        builder.env_order(vec![
            // Don't auto inherit parent env vars
            CommandEnvMode::NoParent,
            // Then set task env vars
            CommandEnvMode::Child,
            // Then inherit parent env vars as they take precedence
            CommandEnvMode::Parent,
        ]);

        if let Some(script) = &task.script {
            builder.set_script(script);
        } else {
            builder.args(&task.args);
        }

        for (key, value) in &task.env {
            builder.env_opt(key, value.as_deref());
        }

        builder
    }

    pub fn augment(self) -> Command {
        self.command
    }

    pub fn apply_command_outputs(&mut self, outputs: Vec<ExtendCommandOutput>) {
        for output in outputs {
            if let Some(new_bin) = output.command {
                self.set_bin(new_bin);
            }

            if let Some(new_args) = output.args {
                self.apply_args(new_args);
            }

            self.envs(output.env);
            self.envs_remove(output.env_remove);
            self.apply_paths(output.paths);
        }
    }

    pub fn apply_script_outputs(&mut self, outputs: Vec<ExtendTaskScriptOutput>) {
        for output in outputs {
            if let Some(new_script) = output.script {
                self.set_script(new_script);
            }

            self.envs(output.env);
            self.envs_remove(output.env_remove);
            self.apply_paths(output.paths);
        }
    }

    pub fn apply_args(&mut self, args: Extend<Vec<String>>) {
        match args {
            Extend::Empty => {
                self.args.clear();
            }
            Extend::Append(next) => {
                self.args(next);
            }
            Extend::Prepend(next) => {
                for arg in next.into_iter().rev() {
                    self.args.push_front(OsString::from(arg));
                }
            }
            Extend::Replace(next) => {
                self.args.clear();
                self.args(next);
            }
        }
    }

    pub fn apply_paths(&mut self, paths: Vec<PathBuf>) {
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

        self.append_paths(paths);
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
        self.apply_command_outputs(
            self.context
                .toolchain_registry
                .extend_command_many(toolchain_ids.clone(), |registry, toolchain| {
                    ExtendCommandInput {
                        context: registry.create_context(),
                        command: self.get_bin_name(),
                        args: self.get_args_list(),
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

        self.apply_command_outputs(
            self.context
                .extension_registry
                .extend_command_all(|registry, extension| ExtendCommandInput {
                    context: registry.create_context(),
                    command: self.get_bin_name(),
                    args: self.get_args_list(),
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

                self.apply_script_outputs(
                    self.context
                        .toolchain_registry
                        .extend_task_script_many(toolchain_ids, |registry, toolchain| {
                            ExtendTaskScriptInput {
                                context: registry.create_context(),
                                script: self.get_script(),
                                project: project.to_fragment(),
                                task: task.to_fragment(),
                                toolchain_config: registry
                                    .create_merged_config(&toolchain.id, &project.config),
                                ..Default::default()
                            }
                        })
                        .await?,
                );

                self.apply_script_outputs(
                    self.context
                        .extension_registry
                        .extend_task_script_all(|registry, extension| ExtendTaskScriptInput {
                            context: registry.create_context(),
                            script: self.get_script(),
                            project: project.to_fragment(),
                            task: task.to_fragment(),
                            extension_config: registry.create_config(&extension.id),
                            ..Default::default()
                        })
                        .await?,
                );
            } else {
                self.apply_command_outputs(
                    self.context
                        .toolchain_registry
                        .extend_task_command_many(toolchain_ids, |registry, toolchain| {
                            ExtendTaskCommandInput {
                                context: registry.create_context(),
                                command: self.get_bin_name(),
                                args: self.get_args_list(),
                                project: project.to_fragment(),
                                task: task.to_fragment(),
                                toolchain_config: registry
                                    .create_merged_config(&toolchain.id, &project.config),
                                ..Default::default()
                            }
                        })
                        .await?,
                );

                self.apply_command_outputs(
                    self.context
                        .extension_registry
                        .extend_task_command_all(|registry, extension| ExtendTaskCommandInput {
                            context: registry.create_context(),
                            command: self.get_bin_name(),
                            args: self.get_args_list(),
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
            self.env(get_version_env_key(id), get_version_env_value(version));
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

            self.append_paths(paths);
        }

        Ok(())
    }

    pub fn inherit_proto(&mut self) {
        let proto_version = self.context.toolchains_config.proto.version.to_string();

        // Inherit common proto env vars
        self.env("PROTO_AUTO_INSTALL", "false");
        self.env("PROTO_IGNORE_MIGRATE_WARNING", "true");
        self.env("PROTO_NO_PROGRESS", "true");
        self.env("PROTO_VERSION", &proto_version);
        self.env("STARBASE_FORCE_TTY", "true");

        // If not using globals, inherit proto and moon paths
        if !is_using_global_toolchains(self.bag) {
            let moon = &self.context.toolchain_registry.host_data.moon_env;
            let proto = &self.context.toolchain_registry.host_data.proto_env;

            self.append_paths([
                proto.store.inventory_dir.join("proto").join(proto_version),
                proto.store.shims_dir.clone(),
                proto.store.bin_dir.clone(),
                moon.store_root.join("bin"),
            ]);
        }
    }
}

impl Deref for AugmentedCommand<'_> {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl DerefMut for AugmentedCommand<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}
