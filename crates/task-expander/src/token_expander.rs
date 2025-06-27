use crate::token_expander_error::TokenExpanderError;
use moon_args::join_args;
use moon_common::{
    color,
    path::{self, WorkspaceRelativePathBuf},
};
use moon_config::{InputPath, OutputPath, ProjectMetadataConfig, patterns};
use moon_env_var::{EnvScanner, EnvSubstitutor, GlobalEnvBag};
use moon_graph_utils::GraphExpanderContext;
use moon_project::{FileGroup, Project};
use moon_task::Task;
use moon_time::{now_millis, now_timestamp};
use pathdiff::diff_paths;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::env;
use std::mem;
use std::path::Path;
use tracing::{debug, instrument, warn};

#[derive(Debug, Default, PartialEq)]
pub struct ExpandedResult {
    pub env: Vec<String>,
    pub files: Vec<WorkspaceRelativePathBuf>,
    pub globs: Vec<WorkspaceRelativePathBuf>,
    pub token: Option<String>,
    pub value: Option<String>,
}

#[derive(PartialEq)]
pub enum TokenScope {
    Command,
    Script,
    Args,
    Env,
    Inputs,
    Outputs,
}

impl TokenScope {
    pub fn label(&self) -> String {
        match self {
            TokenScope::Command => "commands",
            TokenScope::Script => "scripts",
            TokenScope::Args => "args",
            TokenScope::Env => "env",
            TokenScope::Inputs => "inputs",
            TokenScope::Outputs => "outputs",
        }
        .into()
    }
}

pub struct TokenExpander<'graph> {
    pub scope: TokenScope,

    pub context: &'graph GraphExpanderContext,

    pub project: &'graph Project,
}

impl<'graph> TokenExpander<'graph> {
    pub fn new(project: &'graph Project, context: &'graph GraphExpanderContext) -> Self {
        Self {
            scope: TokenScope::Args,
            context,
            project,
        }
    }

    pub fn has_token_function(&self, value: &str) -> bool {
        if value.contains('@') {
            if patterns::TOKEN_FUNC_DISTINCT.is_match(value) {
                return true;
            } else if patterns::TOKEN_FUNC.is_match(value) {
                if self.scope == TokenScope::Script {
                    return true;
                }

                warn!(
                    "Found a token function in `{}` with other content. Token functions *must* be used literally as the only value.",
                    value
                );
            }
        }

        false
    }

    pub fn has_token_variable(&self, value: &str) -> bool {
        value.contains('$') && patterns::TOKEN_VAR.is_match(value)
    }

    #[instrument(skip_all)]
    pub fn expand_command(&mut self, task: &mut Task) -> miette::Result<String> {
        self.scope = TokenScope::Command;

        let mut command = Cow::Owned(task.command.clone());

        if self.has_token_function(&command) {
            let result = self.replace_function(task, &command)?;

            if let (Some(token), Some(value)) = (result.token, result.value) {
                command = Cow::Owned(command.replace(&token, &value));
            }
        }

        self.replace_variables_and_scan(task, command)
    }

    #[instrument(skip_all)]
    pub fn expand_script(&mut self, task: &mut Task) -> miette::Result<String> {
        self.scope = TokenScope::Script;

        let mut script = Cow::Owned(task.script.clone().expect("Script not defined!"));

        while self.has_token_function(&script) {
            let result = self.replace_function(task, &script)?;

            self.infer_inputs_from_result(task, &result);

            if let Some(token) = result.token {
                let mut items = vec![];

                for file in result.files {
                    items.push(self.resolve_path_for_task(task, file)?);
                }

                for glob in result.globs {
                    items.push(self.resolve_path_for_task(task, glob)?);
                }

                if let Some(value) = result.value {
                    items.push(value);
                }

                script = Cow::Owned(script.replace(&token, &join_args(&items)));
            }
        }

        self.replace_variables_and_scan(task, script)
    }

    #[instrument(skip_all)]
    pub fn expand_args(&mut self, task: &mut Task) -> miette::Result<Vec<String>> {
        self.expand_args_with_task(task, None)
    }

    pub fn expand_args_with_task(
        &mut self,
        task: &mut Task,
        base_args: Option<Vec<String>>,
    ) -> miette::Result<Vec<String>> {
        self.scope = TokenScope::Args;

        let mut args = vec![];
        let base_args = base_args.unwrap_or_else(|| mem::take(&mut task.args));

        for arg in base_args {
            // Token functions
            if self.has_token_function(&arg) {
                let result = self.replace_function(task, &arg)?;

                self.infer_inputs_from_result(task, &result);

                for file in result.files {
                    args.push(self.resolve_path_for_task(task, file)?);
                }

                for glob in result.globs {
                    args.push(self.resolve_path_for_task(task, glob)?);
                }

                if let Some(value) = result.value {
                    args.push(value);
                }

                // Everything else
            } else {
                args.push(self.replace_variables_and_scan(task, arg)?);
            }
        }

        Ok(args)
    }

    #[instrument(skip_all)]
    pub fn expand_env(&mut self, task: &mut Task) -> miette::Result<FxHashMap<String, String>> {
        self.expand_env_with_task(task, None)
    }

    pub fn expand_env_with_task(
        &mut self,
        task: &mut Task,
        base_env: Option<FxHashMap<String, String>>,
    ) -> miette::Result<FxHashMap<String, String>> {
        self.scope = TokenScope::Env;

        let mut env = FxHashMap::default();
        let base_env = base_env.unwrap_or_else(|| mem::take(&mut task.env));

        for (key, value) in base_env {
            if self.has_token_function(&value) {
                let result = self.replace_function(task, &value)?;
                let mut items = vec![];

                self.infer_inputs_from_result(task, &result);

                for file in result.files {
                    items.push(self.resolve_path_for_task(task, file)?);
                }

                for glob in result.globs {
                    items.push(self.resolve_path_for_task(task, glob)?);
                }

                if let Some(value) = result.value {
                    items.push(value);
                }

                env.insert(
                    key.to_owned(),
                    items.into_iter().collect::<Vec<_>>().join(","),
                );
            } else {
                env.insert(
                    key.to_owned(),
                    self.replace_variables_and_substitute(task, value)?,
                );
            }
        }

        Ok(env)
    }

    #[instrument(skip_all)]
    pub fn expand_inputs(&mut self, task: &Task) -> miette::Result<ExpandedResult> {
        self.scope = TokenScope::Inputs;

        let mut result = ExpandedResult::default();
        let bag = GlobalEnvBag::instance();

        for input in &task.inputs {
            match input {
                InputPath::EnvVar(var) => {
                    result.env.push(var.to_owned());
                }
                InputPath::EnvVarGlob(var_glob) => {
                    let pattern =
                        Regex::new(format!("^{}$", var_glob.replace('*', "[A-Z0-9_]+")).as_str())
                            .unwrap();

                    bag.list(|var_name, _| {
                        if let Some(var_name) = var_name.to_str() {
                            if pattern.is_match(var_name) {
                                result.env.push(var_name.to_owned());
                            }
                        }
                    });
                }
                InputPath::TokenFunc(func) => {
                    let inner_result = self.replace_function(task, func)?;

                    result.env.extend(inner_result.env);
                    result.files.extend(inner_result.files);
                    result.globs.extend(inner_result.globs);
                    result.value = inner_result.value;
                }
                InputPath::TokenVar(var) => {
                    result.files.push(
                        self.project
                            .source
                            .join(self.replace_variable(task, Cow::Borrowed(var))?.as_ref()),
                    );
                }
                InputPath::ProjectFile(_) | InputPath::WorkspaceFile(_) => {
                    let file = self.create_path_for_task(
                        task,
                        input.to_workspace_relative(&self.project.source),
                    )?;

                    result.files.push(file);
                }
                InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_) => {
                    let glob = self.create_path_for_task(
                        task,
                        input.to_workspace_relative(&self.project.source),
                    )?;

                    result.globs.push(glob);
                }
            };
        }

        Ok(result)
    }

    #[instrument(skip_all)]
    pub fn expand_outputs(&mut self, task: &Task) -> miette::Result<ExpandedResult> {
        self.scope = TokenScope::Outputs;

        let mut result = ExpandedResult::default();

        for output in &task.outputs {
            match output {
                OutputPath::TokenFunc(func) => {
                    let inner_result = self.replace_function(task, func)?;

                    result.files.extend(inner_result.files);
                    result.globs.extend(inner_result.globs);
                    result.value = inner_result.value;
                }
                OutputPath::TokenVar(var) => {
                    result.files.push(
                        self.project
                            .source
                            .join(self.replace_variable(task, Cow::Borrowed(var))?.as_ref()),
                    );
                }
                _ => {
                    let path = self.create_path_for_task(
                        task,
                        output.to_workspace_relative(&self.project.source).unwrap(),
                    )?;

                    if output.is_glob() {
                        result.globs.push(path);
                    } else {
                        result.files.push(path);
                    }
                }
            };
        }

        Ok(result)
    }

    #[instrument(skip(self, task))]
    pub fn replace_function(&self, task: &Task, value: &str) -> miette::Result<ExpandedResult> {
        let matches = patterns::TOKEN_FUNC.captures(value).unwrap();
        let token = matches.get(0).unwrap().as_str(); // @name(arg)
        let func = matches.get(1).unwrap().as_str(); // name
        let arg = matches.get(2).unwrap().as_str(); // arg

        let mut result = ExpandedResult {
            token: Some(token.to_owned()),
            ..ExpandedResult::default()
        };

        let loose_check = matches!(self.scope, TokenScope::Outputs);
        let file_group = || -> miette::Result<&FileGroup> {
            self.check_scope(
                task,
                token,
                &[
                    TokenScope::Script,
                    TokenScope::Args,
                    TokenScope::Env,
                    TokenScope::Inputs,
                    TokenScope::Outputs,
                ],
            )?;

            Ok(self.project.file_groups.get(arg).ok_or_else(|| {
                TokenExpanderError::UnknownFileGroup {
                    group: arg.to_owned(),
                    token: token.to_owned(),
                }
            })?)
        };

        match func {
            // File groups
            "root" => {
                result
                    .files
                    .push(file_group()?.root(&self.context.workspace_root, &self.project.source)?);
            }
            "dirs" => {
                result
                    .files
                    .extend(file_group()?.dirs(&self.context.workspace_root, loose_check)?);
            }
            "files" => {
                result
                    .files
                    .extend(file_group()?.files(&self.context.workspace_root, loose_check)?);
            }
            "globs" => {
                result.globs.extend(file_group()?.globs()?.to_owned());
            }
            "group" => {
                let group = file_group()?;
                result.files.extend(group.files.clone());
                result.globs.extend(group.globs.clone());

                // Only inputs can use env vars, but instead of failing for other
                // scopes, just ignore them
                if self.scope == TokenScope::Inputs {
                    result.env.extend(group.env.clone());
                }
            }
            // Inputs, outputs
            "in" => {
                self.check_scope(task, token, &[TokenScope::Script, TokenScope::Args])?;

                let index = self.parse_index(task, token, arg)?;
                let input =
                    task.inputs
                        .get(index)
                        .ok_or_else(|| TokenExpanderError::MissingInIndex {
                            index,
                            target: task.target.to_string(),
                            token: token.to_owned(),
                        })?;

                match input {
                    InputPath::ProjectFile(_) | InputPath::WorkspaceFile(_) => {
                        result.files.push(self.create_path_for_task(
                            task,
                            input.to_workspace_relative(&self.project.source),
                        )?);
                    }
                    InputPath::ProjectGlob(_) | InputPath::WorkspaceGlob(_) => {
                        result.globs.push(self.create_path_for_task(
                            task,
                            input.to_workspace_relative(&self.project.source),
                        )?);
                    }
                    InputPath::TokenFunc(func) => {
                        let inner_result = self.replace_function(task, func)?;
                        result.files.extend(inner_result.files);
                        result.globs.extend(inner_result.globs);
                    }
                    _ => {
                        return Err(TokenExpanderError::InvalidTokenIndexReference {
                            target: task.target.to_string(),
                            token: token.to_owned(),
                        }
                        .into());
                    }
                };
            }
            "out" => {
                self.check_scope(task, token, &[TokenScope::Script, TokenScope::Args])?;

                let index = self.parse_index(task, token, arg)?;
                let output =
                    task.outputs
                        .get(index)
                        .ok_or_else(|| TokenExpanderError::MissingOutIndex {
                            index,
                            target: task.target.to_string(),
                            token: token.to_owned(),
                        })?;

                match output {
                    OutputPath::ProjectFile(_) | OutputPath::WorkspaceFile(_) => {
                        result.files.push(self.create_path_for_task(
                            task,
                            output.to_workspace_relative(&self.project.source).unwrap(),
                        )?);
                    }
                    OutputPath::ProjectGlob(_) | OutputPath::WorkspaceGlob(_) => {
                        result.globs.push(self.create_path_for_task(
                            task,
                            output.to_workspace_relative(&self.project.source).unwrap(),
                        )?);
                    }
                    OutputPath::TokenFunc(func) => {
                        let inner_result = self.replace_function(task, func)?;
                        result.files.extend(inner_result.files);
                        result.globs.extend(inner_result.globs);
                    }
                    _ => {
                        return Err(TokenExpanderError::InvalidTokenIndexReference {
                            target: task.target.to_string(),
                            token: token.to_owned(),
                        }
                        .into());
                    }
                };
            }
            // Misc
            "envs" => {
                self.check_scope(task, token, &[TokenScope::Inputs])?;

                result.env.extend(file_group()?.env()?.to_owned());
            }
            "meta" => {
                self.check_scope(
                    task,
                    token,
                    &[
                        TokenScope::Command,
                        TokenScope::Script,
                        TokenScope::Args,
                        TokenScope::Env,
                    ],
                )?;

                let project = self.project;
                let metadata = project.config.project.as_ref();

                result.value = match arg {
                    "channel" => metadata.and_then(|md| md.channel.clone()),
                    "description" => metadata.and_then(|md| {
                        if md.description.is_empty() {
                            None
                        } else {
                            Some(md.description.clone())
                        }
                    }),
                    "maintainers" => metadata.and_then(|md| {
                        if md.maintainers.is_empty() {
                            None
                        } else {
                            Some(md.maintainers.join(","))
                        }
                    }),
                    "name" => metadata.and_then(|md| md.name.clone()),
                    "owner" => metadata.and_then(|md| md.owner.clone()),
                    custom_field => metadata.and_then(|md| {
                        md.metadata.get(custom_field).map(|value| value.to_string())
                    }),
                };
            }
            _ => {
                return Err(TokenExpanderError::UnknownToken {
                    token: token.to_owned(),
                }
                .into());
            }
        };

        Ok(result)
    }

    pub fn replace_variables(&self, task: &Task, value: &str) -> miette::Result<String> {
        let mut value = Cow::Borrowed(value);

        while self.has_token_variable(&value) {
            value = self.replace_variable(task, value)?;
        }

        Ok(value.to_string())
    }

    #[instrument(skip(self, task))]
    pub fn replace_variable<'l>(
        &self,
        task: &Task,
        value: Cow<'l, str>,
    ) -> miette::Result<Cow<'l, str>> {
        let Some(matches) = patterns::TOKEN_VAR.captures(&value) else {
            return Ok(value);
        };

        let token_match = matches.get(0).unwrap(); // $var
        let variable = matches.get(1).unwrap().as_str(); // var
        let project = self.project;

        let get_metadata =
            |op: fn(md: &ProjectMetadataConfig) -> Option<&'_ str>| match &project.config.project {
                Some(metadata) => Cow::Borrowed(op(metadata).unwrap_or_default()),
                None => Cow::Owned(String::new()),
            };

        if variable == "taskPlatform" {
            debug!(
                task_target = task.target.as_str(),
                "The {} token variable is deprecated, use {} instead",
                color::property("taskPlatform"),
                color::property("taskToolchain"),
            );
        }

        let replaced_value = match variable {
            // Env
            "arch" => Cow::Borrowed(env::consts::ARCH),
            "os" => Cow::Borrowed(env::consts::OS),
            "osFamily" => Cow::Borrowed(env::consts::FAMILY),
            "workingDir" => Cow::Owned(self.stringify_path(&self.context.working_dir)?),
            "workspaceRoot" => Cow::Owned(self.stringify_path(&self.context.workspace_root)?),
            // Project
            "language" => Cow::Owned(project.language.to_string()),
            "project" => Cow::Borrowed(project.id.as_str()),
            "projectAlias" => match project.alias.as_ref() {
                Some(alias) => Cow::Borrowed(alias.as_str()),
                None => Cow::Owned(String::new()),
            },
            "projectChannel" => get_metadata(|md| md.channel.as_deref()),
            "projectLayer" | "projectType" => Cow::Owned(project.layer.to_string()),
            "projectName" => get_metadata(|md| md.name.as_deref()),
            "projectOwner" => get_metadata(|md| md.owner.as_deref()),
            "projectRoot" => Cow::Owned(self.stringify_path(&project.root)?),
            "projectSource" => Cow::Borrowed(project.source.as_str()),
            "projectStack" => Cow::Owned(project.stack.to_string()),
            // Task
            "target" => Cow::Borrowed(task.target.as_str()),
            "task" => Cow::Borrowed(task.id.as_str()),
            "taskPlatform" | "taskToolchain" => match task.toolchains.first() {
                Some(tc) => Cow::Borrowed(tc.as_str()),
                None => Cow::Owned("unknown".into()),
            },
            "taskToolchains" => Cow::Owned(task.toolchains.join(",")),
            "taskType" => Cow::Owned(task.type_of.to_string()),
            // Datetime
            "date" => Cow::Owned(now_timestamp().format("%F").to_string()),
            "datetime" => Cow::Owned(now_timestamp().format("%F_%T").to_string()),
            "time" => Cow::Owned(now_timestamp().format("%T").to_string()),
            "timestamp" => Cow::Owned((now_millis() / 1000).to_string()),
            // VCS
            "vcsBranch" => Cow::Borrowed(self.context.vcs_branch.as_ref().as_str()),
            "vcsRepository" => Cow::Borrowed(self.context.vcs_repository.as_ref().as_str()),
            "vcsRevision" => Cow::Borrowed(self.context.vcs_revision.as_ref().as_str()),
            _ => {
                return Ok(value);
            }
        };

        let mut inner = value.to_string();
        inner.replace_range(token_match.range(), &replaced_value);

        Ok(inner.into())
    }

    fn check_scope(&self, task: &Task, token: &str, allowed: &[TokenScope]) -> miette::Result<()> {
        if !allowed.contains(&self.scope) {
            return Err(TokenExpanderError::InvalidTokenScope {
                target: task.target.to_string(),
                token: token.to_owned(),
                scope: self.scope.label(),
            }
            .into());
        }

        Ok(())
    }

    fn parse_index(&self, task: &Task, token: &str, value: &str) -> miette::Result<usize> {
        Ok(value
            .parse::<usize>()
            .map_err(|_| TokenExpanderError::InvalidTokenIndex {
                target: task.target.to_string(),
                token: token.to_owned(),
                index: value.to_owned(),
            })?)
    }

    fn create_path_for_task(
        &self,
        task: &Task,
        path: WorkspaceRelativePathBuf,
    ) -> miette::Result<WorkspaceRelativePathBuf> {
        Ok(WorkspaceRelativePathBuf::from(
            self.replace_variables_and_substitute(task, path.as_str())?,
        ))
    }

    fn replace_variables_and_substitute<T: AsRef<str>>(
        &self,
        task: &Task,
        value: T,
    ) -> miette::Result<String> {
        Ok(EnvSubstitutor::new()
            .with_local_vars(&task.env)
            .substitute(self.replace_variables(task, value.as_ref())?))
    }

    fn replace_variables_and_scan<T: AsRef<str>>(
        &self,
        task: &mut Task,
        value: T,
    ) -> miette::Result<String> {
        let mut scanner = EnvScanner::default();
        let result = scanner.scan(self.replace_variables(task, value.as_ref())?);

        if task.options.infer_inputs && !scanner.found.is_empty() {
            self.infer_inputs_from_set(task, scanner.found);
        }

        Ok(result)
    }

    fn resolve_path_for_task(
        &self,
        task: &Task,
        path: WorkspaceRelativePathBuf,
    ) -> miette::Result<String> {
        // From workspace root to any file
        if task.options.run_from_workspace_root {
            Ok(format!("./{}", path))

            // From project root to project file
        } else if let Ok(proj_path) = path.strip_prefix(&self.project.source) {
            Ok(format!("./{}", proj_path))

            // From project root to non-project file
        } else {
            let abs_path = path.to_logical_path(&self.context.workspace_root);

            self.stringify_path(&diff_paths(&abs_path, &self.project.root).unwrap_or(abs_path))
        }
    }

    fn stringify_path(&self, orig_value: &Path) -> miette::Result<String> {
        let value = path::to_string(orig_value)?;

        // https://cygwin.com/cygwin-ug-net/cygpath.html
        #[cfg(windows)]
        {
            if GlobalEnvBag::instance()
                .get("MSYSTEM")
                .is_some_and(|value| value == "MINGW32" || value == "MINGW64")
            {
                let mut value = moon_common::path::standardize_separators(value);

                if orig_value.is_absolute() {
                    for drive in 'A'..='Z' {
                        if let Some(suffix) = value.strip_prefix(&format!("{drive}:/")) {
                            value = format!("/{}/{suffix}", drive.to_ascii_lowercase());
                            break;
                        }
                    }
                }

                return Ok(value);
            }
        }

        Ok(value)
    }

    fn infer_inputs_from_result(&self, task: &mut Task, result: &ExpandedResult) {
        if task.options.infer_inputs {
            task.input_files.extend(result.files.clone());
            task.input_globs.extend(result.globs.clone());
        }
    }

    fn infer_inputs_from_set(&self, task: &mut Task, mut set: FxHashSet<String>) {
        let mut blacklist = vec![
            "CI_",
            "GIT_",
            "BUILD_",
            "PR_",
            "PULL_",
            "COMMIT_HASH",
            "COMMIT_REF",
            "COMMIT_SHA",
            "HEAD",
            "BASE",
            "BRANCH",
        ];
        let ci = ci_env::get_environment();
        let cd = cd_env::get_environment();

        if let Some(ci_prefix) = ci.as_ref().and_then(|ci| ci.env_prefix.as_ref()) {
            blacklist.push(ci_prefix);
        }

        if let Some(cd_prefix) = cd.as_ref().and_then(|cd| cd.env_prefix.as_ref()) {
            blacklist.push(cd_prefix);
        }

        set.retain(|key| {
            blacklist
                .iter()
                .all(|prefix| key != prefix && !key.starts_with(prefix))
        });

        task.input_env.extend(set);
    }
}
