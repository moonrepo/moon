use crate::errors::TokenError;
use crate::file_group::FileGroup;
use crate::target::Target;
use crate::task::Task;
use moon_config::ProjectConfig;
use moon_logger::{color, warn};
use moon_utils::regex::{
    matches_token_func, matches_token_var, TOKEN_FUNC_ANYWHERE_PATTERN, TOKEN_FUNC_PATTERN,
    TOKEN_VAR_PATTERN,
};
use moon_utils::{glob, path};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum ResolverType {
    Args,
    Inputs,
    Outputs,
}

impl ResolverType {
    pub fn context_label(&self) -> String {
        String::from(match self {
            ResolverType::Args => "args",
            ResolverType::Inputs => "inputs",
            ResolverType::Outputs => "outputs",
        })
    }
}

#[derive(Debug, PartialEq)]
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
    pub fn check_context(&self, context: &ResolverType) -> Result<(), TokenError> {
        let allowed = match self {
            TokenType::Dirs(_, _)
            | TokenType::Files(_, _)
            | TokenType::Globs(_, _)
            | TokenType::Group(_, _)
            | TokenType::Root(_, _)
            | TokenType::Var(_) => {
                matches!(context, ResolverType::Args) || matches!(context, ResolverType::Inputs)
            }
            TokenType::In(_, _) | TokenType::Out(_, _) => {
                matches!(context, ResolverType::Args)
            }
        };

        if !allowed {
            return Err(TokenError::InvalidTokenContext(
                self.token_label(),
                context.context_label(),
            ));
        }

        Ok(())
    }

    pub fn token_label(&self) -> String {
        String::from(match self {
            TokenType::Dirs(_, _) => "@dirs",
            TokenType::Files(_, _) => "@files",
            TokenType::Globs(_, _) => "@globs",
            TokenType::Group(_, _) => "@group",
            TokenType::In(_, _) => "@in",
            TokenType::Out(_, _) => "@out",
            TokenType::Root(_, _) => "@root",
            TokenType::Var(_) => "$var",
        })
    }
}

pub struct TokenSharedData<'a> {
    pub file_groups: &'a HashMap<String, FileGroup>,

    pub project_config: &'a ProjectConfig,

    pub project_root: &'a Path,

    pub workspace_root: &'a Path,
}

impl<'a> TokenSharedData<'a> {
    pub fn new(
        file_groups: &'a HashMap<String, FileGroup>,
        workspace_root: &'a Path,
        project_root: &'a Path,
        project_config: &'a ProjectConfig,
    ) -> TokenSharedData<'a> {
        TokenSharedData {
            file_groups,
            project_config,
            project_root,
            workspace_root,
        }
    }
}

pub struct TokenResolver<'a> {
    context: ResolverType,

    pub data: &'a TokenSharedData<'a>,
}

impl<'a> TokenResolver<'a> {
    pub fn for_args(data: &'a TokenSharedData<'a>) -> TokenResolver<'a> {
        TokenResolver {
            context: ResolverType::Args,
            data,
        }
    }

    pub fn for_inputs(data: &'a TokenSharedData<'a>) -> TokenResolver<'a> {
        TokenResolver {
            context: ResolverType::Inputs,
            data,
        }
    }

    pub fn for_outputs(data: &'a TokenSharedData<'a>) -> TokenResolver<'a> {
        TokenResolver {
            context: ResolverType::Outputs,
            data,
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

    /// Cycle through the values, resolve any tokens, and return a list of absolute file paths.
    /// This should only be used for `inputs` and `outputs`.
    pub fn resolve(&self, values: &[String], task: &Task) -> Result<Vec<PathBuf>, TokenError> {
        let mut results: Vec<PathBuf> = vec![];

        for value in values {
            if self.has_token_func(value) {
                for resolved_value in self.resolve_func(value, task)? {
                    results.push(resolved_value);
                }
            } else {
                let has_var = self.has_token_var(value);

                if has_var {
                    TokenType::Var(String::new()).check_context(&self.context)?;
                }

                results.push(path::expand_root_path(
                    if has_var {
                        self.resolve_vars(value, task)?
                    } else {
                        value.to_owned()
                    },
                    self.data.workspace_root,
                    self.data.project_root,
                ));
            }
        }

        Ok(results)
    }

    pub fn resolve_func(&self, value: &str, task: &Task) -> Result<Vec<PathBuf>, TokenError> {
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
            _ => Err(TokenError::UnknownTokenFunc(token.to_owned())),
        }
    }

    pub fn resolve_vars(&self, value: &str, task: &Task) -> Result<String, TokenError> {
        let mut value = value.to_owned();

        while self.has_token_var(&value) {
            value = self.resolve_var(&value, task)?;
        }

        Ok(value)
    }

    pub fn resolve_var(&self, value: &str, task: &Task) -> Result<String, TokenError> {
        let matches = TOKEN_VAR_PATTERN.captures(value).unwrap();
        let token = matches.get(0).unwrap().as_str(); // $var
        let var = matches.get(1).unwrap().as_str(); // var

        let (project_id, task_id) = Target::parse(&task.target)?.ids()?;
        let workspace_root = self.data.workspace_root;
        let project_root = self.data.project_root;
        let project_config = self.data.project_config;

        let var_value = match var {
            "language" => project_config.language.to_string(),
            "project" => project_id,
            "projectRoot" => path::to_string(project_root)?,
            "projectSource" => path::to_string(project_root.strip_prefix(workspace_root).unwrap())?,
            "projectType" => project_config.type_of.to_string(),
            "target" => task.target.clone(),
            "task" => task_id,
            "taskType" => task.type_of.to_string(),
            "workspaceRoot" => path::to_string(workspace_root)?,
            _ => {
                warn!(
                    target: "moon:project:token",
                    "Found a token variable in \"{}\" that is not supported, but this may be intentional, so leaving it.",
                    value
                );

                return Ok(String::from(value));
            }
        };

        Ok(String::from(value).replace(token, &var_value))
    }

    fn convert_string_to_u8(&self, token: &str, value: String) -> Result<u8, TokenError> {
        match value.parse::<u8>() {
            Ok(i) => Ok(i),
            Err(_) => Err(TokenError::InvalidIndexType(token.to_owned(), value)),
        }
    }

    fn replace_file_group_tokens(&self, token_type: TokenType) -> Result<Vec<PathBuf>, TokenError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];
        let file_groups = self.data.file_groups;

        let get_file_group = |token: &str, id: &str| match file_groups.get(id) {
            Some(fg) => Ok(fg),
            None => Err(TokenError::UnknownFileGroup(
                token.to_owned(),
                id.to_owned(),
            )),
        };

        let workspace_root = self.data.workspace_root;
        let project_root = self.data.project_root;

        match token_type {
            TokenType::Dirs(token, group) => {
                results.extend(get_file_group(&token, &group)?.dirs(workspace_root, project_root)?);
            }
            TokenType::Files(token, group) => {
                results
                    .extend(get_file_group(&token, &group)?.files(workspace_root, project_root)?);
            }
            TokenType::Globs(token, group) => {
                results
                    .extend(get_file_group(&token, &group)?.globs(workspace_root, project_root)?);
            }
            TokenType::Group(token, group) => {
                results.extend(get_file_group(&token, &group)?.all(workspace_root, project_root)?);
            }
            TokenType::Root(token, group) => {
                results.push(get_file_group(&token, &group)?.root(project_root)?);
            }
            _ => {}
        }

        Ok(results)
    }

    fn replace_input_token(
        &self,
        token_type: TokenType,
        task: &Task,
    ) -> Result<Vec<PathBuf>, TokenError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];

        if let TokenType::In(token, index) = token_type {
            let error = TokenError::InvalidInIndex(token, index);
            let input = match task.inputs.get(index as usize) {
                Some(i) => i,
                None => {
                    return Err(error);
                }
            };

            if glob::is_glob(input) {
                match task.input_globs.iter().find(|g| g.ends_with(input)) {
                    Some(g) => {
                        results.push(PathBuf::from(g));
                    }
                    None => {
                        return Err(error);
                    }
                };
            } else {
                let workspace_root = self.data.workspace_root;
                let project_root = self.data.project_root;

                match task.input_paths.get(&path::expand_root_path(
                    input,
                    workspace_root,
                    project_root,
                )) {
                    Some(p) => {
                        results.push(p.clone());
                    }
                    None => {
                        return Err(error);
                    }
                };
            }
        }

        Ok(results)
    }

    fn replace_output_token(
        &self,
        token_type: TokenType,
        task: &Task,
    ) -> Result<Vec<PathBuf>, TokenError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];

        if let TokenType::Out(token, index) = token_type {
            let error = TokenError::InvalidOutIndex(token, index);
            let output = match task.outputs.get(index as usize) {
                Some(i) => i,
                None => {
                    return Err(error);
                }
            };

            let workspace_root = self.data.workspace_root;
            let project_root = self.data.project_root;

            match task.output_paths.get(&path::expand_root_path(
                output,
                workspace_root,
                project_root,
            )) {
                Some(p) => {
                    results.push(p.clone());
                }
                None => {
                    return Err(error);
                }
            };
        }

        Ok(results)
    }
}
