use crate::errors::TokenError;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{InputPath, OutputPath};
use moon_logger::warn;
use moon_project::Project;
use moon_task::Task;
use moon_utils::regex::{
    matches_token_func, matches_token_var, TOKEN_FUNC_ANYWHERE_PATTERN, TOKEN_FUNC_PATTERN,
    TOKEN_VAR_PATTERN,
};
use moon_utils::{path, time};
use starbase_styles::color;
use starbase_utils::glob;
use std::path::Path;

type PathsGlobsResolved = (Vec<WorkspaceRelativePathBuf>, Vec<WorkspaceRelativePathBuf>);

#[derive(Debug, Eq, PartialEq)]
pub enum TokenContext {
    Command,
    Args,
    Inputs,
    Outputs,
}

impl TokenContext {
    pub fn context_label(&self) -> String {
        String::from(match self {
            TokenContext::Command => "command",
            TokenContext::Args => "args",
            TokenContext::Inputs => "inputs",
            TokenContext::Outputs => "outputs",
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum TokenType {
    Var(String),

    // File groups: token, group name
    Group(String, String),
    Dirs(String, String),
    Files(String, String),
    Globs(String, String),
    Root(String, String),

    // Inputs, outputs: token, index
    In(String, u8),
    Out(String, u8),
}

impl TokenType {
    pub fn check_context(&self, context: &TokenContext) -> miette::Result<()> {
        let allowed = match self {
            TokenType::Dirs(_, _)
            | TokenType::Files(_, _)
            | TokenType::Globs(_, _)
            | TokenType::Group(_, _)
            | TokenType::Root(_, _) => {
                matches!(
                    context,
                    TokenContext::Args | TokenContext::Inputs | TokenContext::Outputs
                )
            }
            TokenType::In(_, _) | TokenType::Out(_, _) => {
                matches!(context, TokenContext::Args)
            }
            TokenType::Var(_) => {
                matches!(
                    context,
                    TokenContext::Command | TokenContext::Args | TokenContext::Inputs
                )
            }
        };

        if !allowed {
            return Err(TokenError::InvalidTokenContext(
                self.token_label(),
                context.context_label(),
            )
            .into());
        }

        Ok(())
    }

    pub fn token_label(&self) -> String {
        match self {
            TokenType::Dirs(_, _) => "@dirs".into(),
            TokenType::Files(_, _) => "@files".into(),
            TokenType::Globs(_, _) => "@globs".into(),
            TokenType::Group(_, _) => "@group".into(),
            TokenType::In(_, _) => "@in".into(),
            TokenType::Out(_, _) => "@out".into(),
            TokenType::Root(_, _) => "@root".into(),
            TokenType::Var(name) => {
                if name.is_empty() {
                    "$var".into()
                } else {
                    format!("${name}")
                }
            }
        }
    }
}

pub struct TokenResolver<'task> {
    context: TokenContext,
    pub project: &'task Project,
    pub workspace_root: &'task Path,
}

impl<'task> TokenResolver<'task> {
    pub fn new(
        context: TokenContext,
        project: &'task Project,
        workspace_root: &'task Path,
    ) -> TokenResolver<'task> {
        TokenResolver {
            context,
            workspace_root,
            project,
        }
    }

    pub fn has_token_func(&self, value: &str) -> bool {
        if value.contains('@') {
            if matches_token_func(value) {
                return true;
            } else if TOKEN_FUNC_ANYWHERE_PATTERN.is_match(value) {
                warn!(
                    target: "moon:project:token",
                    "Found a token function in {} with other content. Token functions *must* be used literally as the only value.",
                    color::file(value)
                );
            }
        }

        false
    }

    pub fn has_token_var(&self, value: &str) -> bool {
        value.contains('$') && matches_token_var(value)
    }

    pub fn resolve_inputs(
        &self,
        inputs: &[&InputPath],
        task: &Task,
    ) -> miette::Result<PathsGlobsResolved> {
        let mut paths: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        for input in inputs {
            let mut is_glob = input.is_glob();
            let mut resolved;

            match input {
                InputPath::EnvVar(_) => {
                    continue;
                }
                InputPath::TokenFunc(func) => {
                    if self.has_token_func(func) {
                        let (resolved_paths, resolved_globs) = self.resolve_func(func, task)?;

                        paths.extend(resolved_paths);
                        globs.extend(resolved_globs);
                    }

                    continue;
                }
                InputPath::TokenVar(var) => {
                    resolved = WorkspaceRelativePathBuf::from(self.resolve_var(var, task)?);
                }
                other_input => {
                    resolved = WorkspaceRelativePathBuf::from(
                        self.resolve_vars(
                            other_input
                                .to_workspace_relative(&self.project.source)
                                .as_str(),
                            task,
                        )?,
                    );
                }
            };

            // This is a special case for inputs that converts "foo" to "foo/**/*",
            // when the input is a directory. This is necessary for VCS hashing.
            if resolved.to_path(self.workspace_root).is_dir() {
                is_glob = true;
                resolved = resolved.join("**/*");
            }

            if is_glob {
                globs.push(resolved);
            } else {
                paths.push(resolved);
            }
        }

        Ok((paths, globs))
    }

    pub fn resolve_outputs(
        &self,
        outputs: &[OutputPath],
        task: &Task,
    ) -> miette::Result<PathsGlobsResolved> {
        let mut paths: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        for output in outputs {
            match output {
                OutputPath::TokenFunc(func) => {
                    if self.has_token_func(func) {
                        let (resolved_paths, resolved_globs) = self.resolve_func(func, task)?;

                        paths.extend(resolved_paths);
                        globs.extend(resolved_globs);
                    }

                    continue;
                }
                other_output => {
                    let resolved = WorkspaceRelativePathBuf::from(
                        self.resolve_vars(
                            other_output
                                .to_workspace_relative(&self.project.source)
                                .unwrap()
                                .as_str(),
                            task,
                        )?,
                    );

                    if other_output.is_glob() {
                        globs.push(resolved);
                    } else {
                        paths.push(resolved);
                    }
                }
            };
        }

        Ok((paths, globs))
    }

    /// Cycle through the values, resolve any tokens, and return a list of absolute file paths.
    /// This should only be used for `inputs` and `outputs`.
    pub fn resolve(&self, values: &[String], task: &Task) -> miette::Result<PathsGlobsResolved> {
        let mut paths: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        for value in values {
            if self.has_token_func(value) {
                let (resolved_paths, resolved_globs) = self.resolve_func(value, task)?;

                paths.extend(resolved_paths);
                globs.extend(resolved_globs);
            } else {
                let resolved =
                    WorkspaceRelativePathBuf::from_path(path::expand_to_workspace_relative(
                        self.resolve_vars(value, task)?,
                        self.workspace_root,
                        &self.project.root,
                    ))
                    .unwrap();

                if glob::is_glob(value) {
                    globs.push(resolved);
                } else {
                    paths.push(resolved);
                }
            }
        }

        Ok((paths, globs))
    }

    pub fn resolve_command(&self, task: &Task) -> miette::Result<String> {
        if self.has_token_func(&task.command) {
            // Trigger validation only
            self.resolve_func(&task.command, task)?;

            return Ok(task.command.clone());
        }

        self.resolve_vars(&task.command, task)
    }

    pub fn resolve_func(&self, value: &str, task: &Task) -> miette::Result<PathsGlobsResolved> {
        let matches = TOKEN_FUNC_PATTERN.captures(value).unwrap();
        let token = matches.get(0).unwrap().as_str(); // @name(arg)
        let func = matches.get(1).unwrap().as_str(); // name
        let arg = matches.get(2).unwrap().as_str(); // arg

        match func {
            "dirs" => {
                self.replace_file_group_tokens(TokenType::Dirs(token.to_owned(), arg.to_owned()))
            }
            "files" => {
                self.replace_file_group_tokens(TokenType::Files(token.to_owned(), arg.to_owned()))
            }
            "globs" => {
                self.replace_file_group_tokens(TokenType::Globs(token.to_owned(), arg.to_owned()))
            }
            "group" => {
                self.replace_file_group_tokens(TokenType::Group(token.to_owned(), arg.to_owned()))
            }
            "in" => self.replace_input_token(
                TokenType::In(
                    token.to_owned(),
                    self.convert_string_to_u8(token, arg.to_owned())?,
                ),
                task,
            ),
            "out" => self.replace_output_token(
                TokenType::Out(
                    token.to_owned(),
                    self.convert_string_to_u8(token, arg.to_owned())?,
                ),
                task,
            ),
            "root" => {
                self.replace_file_group_tokens(TokenType::Root(token.to_owned(), arg.to_owned()))
            }
            _ => Err(TokenError::UnknownTokenFunc(token.to_owned()).into()),
        }
    }

    pub fn resolve_vars(&self, value: &str, task: &Task) -> miette::Result<String> {
        let mut value = value.to_owned();

        while self.has_token_var(&value) {
            value = self.resolve_var(&value, task)?;
        }

        Ok(value)
    }

    pub fn resolve_var(&self, value: &str, task: &Task) -> miette::Result<String> {
        let Some(matches) = TOKEN_VAR_PATTERN.captures(value) else {
            return Ok(value.to_owned());
        };

        let token = matches.get(0).unwrap().as_str(); // $var
        let var = matches.get(1).unwrap().as_str(); // var
        let workspace_root = &self.workspace_root;
        let project = self.project;

        TokenType::Var(var.to_owned()).check_context(&self.context)?;

        let var_value = match var {
            "workspaceRoot" => path::to_string(workspace_root)?,
            // Project
            "language" => project.language.to_string(),
            "project" => project.id.to_string(),
            "projectAlias" => project.alias.clone().unwrap_or_default(),
            "projectRoot" => path::to_string(&project.root)?,
            "projectSource" => project.source.to_string(),
            "projectType" => project.type_of.to_string(),
            // Task
            "target" => task.target.id.to_string(),
            "task" => task.id.to_string(),
            "taskPlatform" => task.platform.to_string(),
            "taskType" => task.type_of.to_string(),
            // Datetime
            "date" => time::now_timestamp().format("%F").to_string(),
            "datetime" => time::now_timestamp().format("%F_%T").to_string(),
            "time" => time::now_timestamp().format("%T").to_string(),
            "timestamp" => (time::now_millis() / 1000).to_string(),
            _ => {
                return Ok(value.to_owned());
            }
        };

        Ok(value.replace(token, &var_value))
    }

    fn convert_string_to_u8(&self, token: &str, value: String) -> miette::Result<u8> {
        match value.parse::<u8>() {
            Ok(i) => Ok(i),
            Err(_) => Err(TokenError::InvalidIndexType(token.to_owned(), value).into()),
        }
    }

    fn replace_file_group_tokens(
        &self,
        token_type: TokenType,
    ) -> miette::Result<PathsGlobsResolved> {
        token_type.check_context(&self.context)?;

        let mut paths: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];
        let file_groups = &self.project.file_groups;

        let get_file_group = |token: &str, id: &str| {
            file_groups
                .get(id)
                .ok_or_else(|| TokenError::UnknownFileGroup(token.to_owned(), id.to_owned()))
        };

        let workspace_root = self.workspace_root;
        let project_source = &self.project.source;

        match token_type {
            TokenType::Dirs(token, group) => {
                for dir in get_file_group(&token, &group)?.dirs(workspace_root)? {
                    paths.push(dir);
                }
            }
            TokenType::Files(token, group) => {
                for file in get_file_group(&token, &group)?.files(workspace_root)? {
                    paths.push(file);
                }
            }
            TokenType::Globs(token, group) => {
                for glob in get_file_group(&token, &group)?.globs()? {
                    globs.push(glob.to_owned());
                }
            }
            TokenType::Group(token, group) => {
                let group = get_file_group(&token, &group)?;

                for file in &group.files {
                    paths.push(file.to_owned());
                }

                for glob in &group.globs {
                    globs.push(glob.to_owned());
                }
            }
            TokenType::Root(token, group) => {
                paths.push(get_file_group(&token, &group)?.root(workspace_root, project_source)?);
            }
            _ => {}
        }

        Ok((paths, globs))
    }

    fn replace_input_token(
        &self,
        token_type: TokenType,
        task: &Task,
    ) -> miette::Result<PathsGlobsResolved> {
        token_type.check_context(&self.context)?;

        let mut paths: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        if let TokenType::In(token, index) = token_type {
            let error = TokenError::InvalidInIndex(token, index);

            let Some(input) = task.inputs.get(index as usize) else {
                return Err(error.into());
            };

            if input.is_glob() {
                match task
                    .input_globs
                    .iter()
                    .find(|g| g.ends_with(input.as_str()))
                {
                    Some(g) => {
                        globs.push(g.to_owned());
                    }
                    None => {
                        return Err(error.into());
                    }
                };
            } else {
                let rel = input.to_workspace_relative(&self.project.source);

                match task.input_paths.get(&rel) {
                    Some(p) => {
                        paths.push(p.clone());
                    }
                    None => {
                        return Err(error.into());
                    }
                };
            }
        }

        Ok((paths, globs))
    }

    fn replace_output_token(
        &self,
        token_type: TokenType,
        task: &Task,
    ) -> miette::Result<PathsGlobsResolved> {
        token_type.check_context(&self.context)?;

        let mut paths: Vec<WorkspaceRelativePathBuf> = vec![];
        let mut globs: Vec<WorkspaceRelativePathBuf> = vec![];

        if let TokenType::Out(token, index) = token_type {
            let error = TokenError::InvalidOutIndex(token.clone(), index);

            let Some(output) = task.outputs.get(index as usize) else {
                return Err(error.into());
            };

            if self.has_token_func(output.as_str()) {
                return Err(TokenError::InvalidOutNoTokenFunctions(token).into());
            }

            if output.is_glob() {
                match task
                    .output_globs
                    .iter()
                    .find(|g| g.ends_with(output.as_str()))
                {
                    Some(g) => {
                        globs.push(g.to_owned());
                    }
                    None => {
                        return Err(error.into());
                    }
                };
            } else {
                let rel = output
                    .to_workspace_relative(&self.project.source)
                    .unwrap_or_default();

                match task.output_paths.get(&rel) {
                    Some(p) => {
                        paths.push(p.clone());
                    }
                    None => {
                        return Err(error.into());
                    }
                };
            }
        }

        Ok((paths, globs))
    }
}
