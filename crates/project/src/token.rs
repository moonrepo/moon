use crate::errors::{ProjectError, TokenError};
use crate::file_group::FileGroup;
use crate::task::Task;
use moon_logger::{color, trace, warn};
use moon_utils::fs::is_glob;
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

    pub fn expand_io_path(&self, file: &str) -> PathBuf {
        if file.starts_with('/') {
            self.data
                .workspace_root
                .join(file.strip_prefix('/').unwrap())
        } else {
            self.data.project_root.join(file)
        }
    }

    pub fn has_token(value: &str) -> bool {
        value.contains('@') || value.contains('$')
    }

    pub fn resolve(
        &self,
        values: &[String],
        task: Option<&Task>,
    ) -> Result<Vec<String>, ProjectError> {
        let mut results: Vec<String> = vec![];

        println!("resolve = {:?}", values);

        for value in values {
            if Self::has_token(value) {
                for resolved_value in self.replace_token(value, task)? {
                    results.push(resolved_value);
                }
            } else {
                results.push(value.to_owned());
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

    fn replace_token(&self, value: &str, task: Option<&Task>) -> Result<Vec<String>, ProjectError> {
        println!(
            "replace_token = {} {} {} {}",
            value,
            value.contains('@'),
            TOKEN_FUNC_PATTERN.is_match(value),
            TOKEN_FUNC_PATTERN.as_str()
        );

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

            println!("{}, {}, {}", token, func, arg);

            return match func {
                "dirs" => self.replace_file_group_tokens(
                    value,
                    TokenType::Dirs(token.to_owned(), arg.to_owned()),
                ),
                "files" => self.replace_file_group_tokens(
                    value,
                    TokenType::Files(token.to_owned(), arg.to_owned()),
                ),
                "globs" => self.replace_file_group_tokens(
                    value,
                    TokenType::Globs(token.to_owned(), arg.to_owned()),
                ),
                "in" => self.replace_input_token(
                    value,
                    TokenType::In(
                        token.to_owned(),
                        self.convert_string_to_u8(token, arg.to_owned())?,
                    ),
                    task,
                ),
                "root" => self.replace_file_group_tokens(
                    value,
                    TokenType::Root(token.to_owned(), arg.to_owned()),
                ),
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
        value: &str,
        token_type: TokenType,
    ) -> Result<Vec<String>, ProjectError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];
        let file_groups = self.data.file_groups;

        let mut replace_token = |token: &str, replacement: &str| {
            results.push(String::from(value).replace(token, replacement));
        };

        let get_file_group = |token: &str, id: &str| match file_groups.get(id) {
            Some(fg) => Ok(fg),
            None => Err(ProjectError::Token(TokenError::UnknownFileGroup(
                token.to_owned(),
                id.to_owned(),
            ))),
        };

        match token_type {
            TokenType::Dirs(token, group) => {
                for glob in get_file_group(&token, &group)?.dirs()? {
                    replace_token(&token, &glob);
                }
            }
            TokenType::Files(token, group) => {
                for glob in get_file_group(&token, &group)?.files()? {
                    replace_token(&token, &glob);
                }
            }
            TokenType::Globs(token, group) => {
                for glob in get_file_group(&token, &group)?.globs()? {
                    replace_token(&token, &glob);
                }
            }
            TokenType::Root(token, group) => {
                replace_token(&token, &get_file_group(&token, &group)?.root()?);
            }
            _ => {}
        }

        Ok(results)
    }

    fn replace_input_token(
        &self,
        value: &str,
        token_type: TokenType,
        task: Option<&Task>,
    ) -> Result<Vec<String>, ProjectError> {
        token_type.check_context(&self.context)?;

        let mut results = vec![];
        let task = task.unwrap();

        let mut replace_token = |token: &str, replacement: &str| {
            results.push(String::from(value).replace(token, replacement));
        };

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
                        replace_token(&token, g);
                    }
                    None => {
                        return Err(error);
                    }
                };
            } else {
                match task.input_paths.get(&self.expand_io_path(input)) {
                    Some(p) => {
                        replace_token(&token, p.to_str().unwrap());
                    }
                    None => {
                        return Err(error);
                    }
                };
            }
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
        let file_groups = create_file_groups(&project_root);
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
        let file_groups = create_file_groups(&project_root);
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
        let file_groups = create_file_groups(&project_root);
        let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
        let resolver = TokenResolver::for_args(&metadata);

        assert_eq!(
            resolver
                .resolve(&string_vec!["foo/@dirs(static)/bar"], None)
                .unwrap(),
            Vec::<String>::new()
        );
    }

    mod in_token {
        use super::*;

        #[test]
        #[should_panic(expected = "Token(InvalidIndexType(\"@in(abc)\", \"abc\"))")]
        fn errors_for_invalid_index_format() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
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
            let file_groups = create_file_groups(&project_root);
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

    mod args {
        use super::*;

        #[test]
        fn supports_dirs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(static)"], None)
                    .unwrap(),
                vec!["dir", "dir/subdir"]
            );
        }

        #[test]
        fn supports_dirs_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(dirs_glob)"], None)
                    .unwrap(),
                vec!["dir", "dir/subdir"]
            );
        }

        #[test]
        fn supports_files() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(static)"], None)
                    .unwrap(),
                vec!["file.ts", "dir/other.tsx", "dir/subdir/another.ts",]
            );
        }

        #[test]
        fn supports_files_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(files_glob)"], None)
                    .unwrap(),
                vec!["file.ts", "dir/subdir/another.ts", "dir/other.tsx",]
            );
        }

        #[test]
        fn supports_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@globs(globs)"], None)
                    .unwrap(),
                vec!["**/*.{ts,tsx}", "*.js"],
            );
        }

        #[test]
        fn supports_in_paths() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
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
                    .resolve(&string_vec!["arg", "@in(1)"], Some(&task))
                    .unwrap(),
                vec!["arg", project_root.join("file.ts").to_str().unwrap()],
            );
        }

        #[test]
        fn supports_in_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
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
                    .resolve(&string_vec!["arg", "@in(0)"], Some(&task))
                    .unwrap(),
                vec!["arg", project_root.join("src/**/*").to_str().unwrap()],
            );
        }

        #[test]
        fn supports_root() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_args(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@root(static)"], None)
                    .unwrap(),
                vec!["dir"],
            );
        }
    }

    mod inputs {
        use super::*;

        #[test]
        fn supports_dirs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(static)"], None)
                    .unwrap(),
                vec!["dir", "dir/subdir"]
            );
        }

        #[test]
        fn supports_dirs_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@dirs(dirs_glob)"], None)
                    .unwrap(),
                vec!["dir", "dir/subdir"]
            );
        }

        #[test]
        fn supports_files() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(static)"], None)
                    .unwrap(),
                vec!["file.ts", "dir/other.tsx", "dir/subdir/another.ts",]
            );
        }

        #[test]
        fn supports_files_with_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@files(files_glob)"], None)
                    .unwrap(),
                vec!["file.ts", "dir/subdir/another.ts", "dir/other.tsx",]
            );
        }

        #[test]
        fn supports_globs() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@globs(globs)"], None)
                    .unwrap(),
                vec!["**/*.{ts,tsx}", "*.js"],
            );
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@in\", \"inputs\")")]
        fn doesnt_support_in() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            resolver.resolve(&string_vec!["@in(0)"], None).unwrap();
        }

        #[test]
        fn supports_root() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_inputs(&metadata);

            assert_eq!(
                resolver
                    .resolve(&string_vec!["@root(static)"], None)
                    .unwrap(),
                vec!["dir"],
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
            let file_groups = create_file_groups(&project_root);
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
            let file_groups = create_file_groups(&project_root);
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
            let file_groups = create_file_groups(&project_root);
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
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver.resolve(&string_vec!["@in(0)"], None).unwrap();
        }

        #[test]
        #[should_panic(expected = "InvalidTokenContext(\"@root\", \"outputs\")")]
        fn doesnt_support_root() {
            let project_root = get_project_root();
            let workspace_root = get_workspace_root();
            let file_groups = create_file_groups(&project_root);
            let metadata = TokenSharedData::new(&file_groups, &workspace_root, &project_root);
            let resolver = TokenResolver::for_outputs(&metadata);

            resolver
                .resolve(&string_vec!["@root(static)"], None)
                .unwrap();
        }
    }
}
