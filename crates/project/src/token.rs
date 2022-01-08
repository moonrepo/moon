use crate::errors::{ProjectError, TokenError};
use crate::file_group::FileGroup;
use crate::task::Task;
use moon_logger::{color, trace, warn};
use moon_utils::fs::{expand_root_path, is_glob};
use moon_utils::regex::{TOKEN_FUNC_ANYWHERE_PATTERN, TOKEN_FUNC_PATTERN};
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
    // Var(String),

    // File groups: token, group name
    Dirs(String, String),
    Files(String, String),
    Globs(String, String),
    Root(String, String),

    // Inputs, outputs: token, index
    In(String, u8),
    Out(String, u8),
}

impl TokenType {
    pub fn check_context(&self, context: &ResolverType) -> Result<(), ProjectError> {
        let allowed = match self {
            TokenType::Dirs(_, _) => {
                matches!(context, ResolverType::Args) || matches!(context, ResolverType::Inputs)
            }
            TokenType::Files(_, _) => {
                matches!(context, ResolverType::Args) || matches!(context, ResolverType::Inputs)
            }
            TokenType::Globs(_, _) => {
                matches!(context, ResolverType::Args) || matches!(context, ResolverType::Inputs)
            }
            TokenType::In(_, _) => {
                matches!(context, ResolverType::Args)
            }
            TokenType::Out(_, _) => {
                matches!(context, ResolverType::Args)
            }
            TokenType::Root(_, _) => {
                matches!(context, ResolverType::Args) || matches!(context, ResolverType::Inputs)
            }
        };

        if !allowed {
            return Err(ProjectError::Token(TokenError::InvalidTokenContext(
                self.token_label(),
                context.context_label(),
            )));
        }

        Ok(())
    }

    pub fn token_label(&self) -> String {
        String::from(match self {
            TokenType::Dirs(_, _) => "@dirs",
            TokenType::Files(_, _) => "@files",
            TokenType::Globs(_, _) => "@globs",
            TokenType::In(_, _) => "@in",
            TokenType::Out(_, _) => "@out",
            TokenType::Root(_, _) => "@root",
        })
    }
}

pub struct TokenSharedData<'a> {
    pub file_groups: &'a HashMap<String, FileGroup>,

    pub project_root: &'a Path,

    pub workspace_root: &'a Path,
}

impl<'a> TokenSharedData<'a> {
    pub fn new(
        file_groups: &'a HashMap<String, FileGroup>,
        workspace_root: &'a Path,
        project_root: &'a Path,
    ) -> TokenSharedData<'a> {
        TokenSharedData {
            file_groups,
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

    pub fn has_token(&self, value: &str) -> bool {
        value.contains('@') || value.contains('$')
    }

    /// Cycle through the values, resolve any tokens, and return a list of absolute file paths.
    /// This should only be used for `inputs` and `outputs`.
    pub fn resolve(
        &self,
        values: &[String],
        task: Option<&Task>,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        let mut results: Vec<PathBuf> = vec![];

        for value in values {
            if self.has_token(value) {
                for resolved_value in self.replace_token(value, task)? {
                    results.push(resolved_value);
                }
            } else {
                results.push(expand_root_path(
                    value,
                    self.data.workspace_root,
                    self.data.project_root,
                ));
            }
        }

        Ok(results)
    }

    fn convert_string_to_u8(&self, token: &str, value: String) -> Result<u8, ProjectError> {
        match value.parse::<u8>() {
            Ok(i) => Ok(i),
            Err(_) => Err(ProjectError::Token(TokenError::InvalidIndexType(
                token.to_owned(),
                value,
            ))),
        }
    }

    fn replace_token(
        &self,
        value: &str,
        task: Option<&Task>,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        if value.contains('@') && TOKEN_FUNC_PATTERN.is_match(value) {
            let matches = TOKEN_FUNC_PATTERN.captures(value).unwrap();
            let token = matches.get(0).unwrap().as_str(); // @name(arg)
            let func = matches.get(1).unwrap().as_str(); // name
            let arg = matches.get(2).unwrap().as_str(); // arg

            trace!(
                target: "moon:project:token",
                "Resolving token {} for {} value {}",
                color::id(token),
                self.context.context_label(),
                color::path(value)
            );

            return match func {
                "dirs" => self
                    .replace_file_group_tokens(TokenType::Dirs(token.to_owned(), arg.to_owned())),
                "files" => self
                    .replace_file_group_tokens(TokenType::Files(token.to_owned(), arg.to_owned())),
                "globs" => self
                    .replace_file_group_tokens(TokenType::Globs(token.to_owned(), arg.to_owned())),
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
                "root" => self
                    .replace_file_group_tokens(TokenType::Root(token.to_owned(), arg.to_owned())),
                _ => {
                    return Err(ProjectError::Token(TokenError::UnknownTokenFunc(
                        token.to_owned(),
                    )))
                }
            };
        } else if value.contains('@') && TOKEN_FUNC_ANYWHERE_PATTERN.is_match(value) {
            warn!(
                target: "moon:project:token",
                "Found a token function in {} with other content. Token functions *must* be used literally as the only value.",
                color::path(value)
            );
        }

        Ok(vec![])
    }

    fn replace_file_group_tokens(
        &self,
        token_type: TokenType,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];
        let file_groups = self.data.file_groups;

        let get_file_group = |token: &str, id: &str| match file_groups.get(id) {
            Some(fg) => Ok(fg),
            None => Err(ProjectError::Token(TokenError::UnknownFileGroup(
                token.to_owned(),
                id.to_owned(),
            ))),
        };

        let workspace_root = self.data.workspace_root;
        let project_root = self.data.project_root;

        match token_type {
            TokenType::Dirs(token, group) => {
                for glob in get_file_group(&token, &group)?.dirs(workspace_root, project_root)? {
                    results.push(glob);
                }
            }
            TokenType::Files(token, group) => {
                for file in get_file_group(&token, &group)?.files(workspace_root, project_root)? {
                    results.push(file);
                }
            }
            TokenType::Globs(token, group) => {
                for dir in get_file_group(&token, &group)?.globs(workspace_root, project_root)? {
                    results.push(dir);
                }
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
        task: Option<&Task>,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];
        let task = task.unwrap();

        if let TokenType::In(token, index) = token_type {
            let error = ProjectError::Token(TokenError::InvalidInIndex(token.to_owned(), index));
            let input = match task.inputs.get(index as usize) {
                Some(i) => i,
                None => {
                    return Err(error);
                }
            };

            if is_glob(input) {
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

                match task
                    .input_paths
                    .get(&expand_root_path(input, workspace_root, project_root))
                {
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
        task: Option<&Task>,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];
        let task = task.unwrap();

        if let TokenType::Out(token, index) = token_type {
            let error = ProjectError::Token(TokenError::InvalidOutIndex(token.to_owned(), index));
            let output = match task.outputs.get(index as usize) {
                Some(i) => i,
                None => {
                    return Err(error);
                }
            };

            let workspace_root = self.data.workspace_root;
            let project_root = self.data.project_root;

            match task
                .output_paths
                .get(&expand_root_path(output, workspace_root, project_root))
            {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{create_expanded_task, create_file_groups};
    use moon_config::TaskConfig;
    use moon_utils::string_vec;
    use moon_utils::test::get_fixtures_dir;
    use std::path::PathBuf;

    fn get_workspace_root() -> PathBuf {
        get_fixtures_dir("base")
    }

    fn get_project_root() -> PathBuf {
        get_workspace_root().join("files-and-dirs")
    }

    #[test]
    #[should_panic(expected = "UnknownFileGroup(\"@dirs(unknown)\", \"unknown\")")]
    fn errors_for_unknown_file_group() {
        let project_root = get_project_root();
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
        let resolver = TokenResolver::for_args(&metadata);

        resolver
            .resolve(&string_vec!["@dirs(unknown)"], None)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "NoGlobs(\"no_globs\")")]
    fn errors_if_no_globs_in_file_group() {
        let project_root = get_project_root();
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
        let resolver = TokenResolver::for_args(&metadata);

        resolver
            .resolve(&string_vec!["@globs(no_globs)"], None)
            .unwrap();
    }

    #[test]
    fn doesnt_match_when_not_alone() {
        let project_root = get_project_root();
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
        let resolver = TokenResolver::for_args(&metadata);

        assert_eq!(
            resolver
                .resolve(&string_vec!["foo/@dirs(static)/bar"], None)
                .unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    mod in_token {
        use super::*;

        #[test]
        #[should_panic(expected = "Token(InvalidIndexType(\"@in(abc)\", \"abc\"))")]
        fn errors_for_invalid_index_format() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["dir/**/*", "file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            resolver
                .resolve(&string_vec!["@in(abc)"], Some(&task))
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "Token(InvalidInIndex(\"@in(5)\", 5))")]
        fn errors_for_index_out_of_bounds() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["dir/**/*", "file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            resolver
                .resolve(&string_vec!["@in(5)"], Some(&task))
                .unwrap();
        }
    }

    mod out_token {
        use super::*;

        #[test]
        #[should_panic(expected = "Token(InvalidIndexType(\"@out(abc)\", \"abc\"))")]
        fn errors_for_invalid_index_format() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    outputs: Some(string_vec!["dir", "file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            resolver
                .resolve(&string_vec!["@out(abc)"], Some(&task))
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "Token(InvalidOutIndex(\"@out(5)\", 5))")]
        fn errors_for_index_out_of_bounds() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    outputs: Some(string_vec!["dir", "file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            resolver
                .resolve(&string_vec!["@out(5)"], Some(&task))
                .unwrap();
        }
    }

    mod args {
        use super::*;

        #[test]
        fn supports_dirs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(static)"], None)
                    .unwrap(),
                vec![project_root.join("dir"), project_root.join("dir/subdir")]
            );
        }

        #[test]
        fn supports_dirs_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(dirs_glob)"], None)
                    .unwrap(),
                vec![project_root.join("dir"), project_root.join("dir/subdir")]
            );
        }

        #[test]
        fn supports_files() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(static)"], None)
                    .unwrap(),
                vec![
                    project_root.join("file.ts"),
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts")
                ]
            );
        }

        #[test]
        fn supports_files_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(files_glob)"], None)
                    .unwrap(),
                vec![
                    project_root.join("file.ts"),
                    project_root.join("dir/subdir/another.ts"),
                    project_root.join("dir/other.tsx")
                ]
            );
        }

        #[test]
        fn supports_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@globs(globs)"], None)
                    .unwrap(),
                vec![
                    project_root.join("**/*.{ts,tsx}"),
                    project_root.join("*.js")
                ],
            );
        }

        #[test]
        fn supports_in_paths() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["dir/**/*", "file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@in(1)"], Some(&task))
                    .unwrap(),
                vec![project_root.join("file.ts")],
            );
        }

        #[test]
        fn supports_in_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["src/**/*", "file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@in(0)"], Some(&task))
                    .unwrap(),
                vec![project_root.join("src/**/*")],
            );
        }

        #[test]
        fn supports_out_paths() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    outputs: Some(string_vec!["dir/", "file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@out(0)", "@out(1)"], Some(&task))
                    .unwrap(),
                vec![project_root.join("dir"), project_root.join("file.ts"),],
            );
        }

        #[test]
        fn supports_root() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@root(static)"], None)
                    .unwrap(),
                vec![project_root.join("dir")],
            );
        }
    }

    mod inputs {
        use super::*;

        #[test]
        fn supports_dirs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(static)"], None)
                    .unwrap(),
                vec![project_root.join("dir"), project_root.join("dir/subdir")]
            );
        }

        #[test]
        fn supports_dirs_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(dirs_glob)"], None)
                    .unwrap(),
                vec![project_root.join("dir"), project_root.join("dir/subdir")]
            );
        }

        #[test]
        fn supports_files() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(static)"], None)
                    .unwrap(),
                vec![
                    project_root.join("file.ts"),
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts")
                ]
            );
        }

        #[test]
        fn supports_files_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(files_glob)"], None)
                    .unwrap(),
                vec![
                    project_root.join("file.ts"),
                    project_root.join("dir/subdir/another.ts"),
                    project_root.join("dir/other.tsx")
                ]
            );
        }

        #[test]
        fn supports_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@globs(globs)"], None)
                    .unwrap(),
                vec![
                    project_root.join("**/*.{ts,tsx}"),
                    project_root.join("*.js")
                ],
            );
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@in\", \"inputs\")")]
        fn doesnt_support_in() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            resolver.resolve(&string_vec!["@in(0)"], None).unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@out\", \"inputs\")")]
        fn doesnt_support_out() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            resolver.resolve(&string_vec!["@out(0)"], None).unwrap();
        }

        #[test]
        fn supports_root() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@root(static)"], None)
                    .unwrap(),
                vec![project_root.join("dir")],
            );
        }
    }

    mod outputs {
        use super::*;

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@dirs\", \"outputs\")")]
        fn doesnt_support_dirs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver
                .resolve(&string_vec!["@dirs(static)"], None)
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@files\", \"outputs\")")]
        fn doesnt_support_files() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver
                .resolve(&string_vec!["@files(static)"], None)
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@globs\", \"outputs\")")]
        fn doesnt_support_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver
                .resolve(&string_vec!["@globs(globs)"], None)
                .unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@in\", \"outputs\")")]
        fn doesnt_support_in() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver.resolve(&string_vec!["@in(0)"], None).unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@out\", \"outputs\")")]
        fn doesnt_support_out() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver.resolve(&string_vec!["@out(0)"], None).unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@root\", \"outputs\")")]
        fn doesnt_support_root() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups();
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver
                .resolve(&string_vec!["@root(static)"], None)
                .unwrap();
        }
    }
}
