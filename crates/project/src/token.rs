use crate::errors::{ProjectError, TokenError};
use crate::file_group::FileGroup;
use moon_utils::regex::TOKEN_FUNC_PATTERN;
use std::collections::HashMap;

#[derive(PartialEq)]
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

#[derive(PartialEq)]
pub enum TokenType {
    // Var(String),

    // File groups: token, group name
    Dirs(String, String),
    Files(String, String),
    Globs(String, String),
    Root(String, String),
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
            TokenType::Root(_, _) => "@root",
        })
    }
}

pub struct TokenResolver<'a> {
    file_groups: &'a HashMap<String, FileGroup>,

    context: ResolverType,
}

impl<'a> TokenResolver<'a> {
    pub fn for_args(file_groups: &'a HashMap<String, FileGroup>) -> TokenResolver {
        TokenResolver {
            file_groups,
            context: ResolverType::Args,
        }
    }

    pub fn for_inputs(file_groups: &'a HashMap<String, FileGroup>) -> TokenResolver {
        TokenResolver {
            file_groups,
            context: ResolverType::Inputs,
        }
    }

    pub fn has_token(value: &str) -> bool {
        value.contains('@') || value.contains('$')
    }

    pub fn resolve(&self, value: &str) -> Result<Vec<String>, ProjectError> {
        if !Self::has_token(value) {
            return Ok(vec![value.to_owned()]);
        }

        self.replace_token(value)
    }

    fn replace_token(&self, value: &str) -> Result<Vec<String>, ProjectError> {
        if value.contains('@') && TOKEN_FUNC_PATTERN.is_match(value) {
            let matches = TOKEN_FUNC_PATTERN.captures(value).unwrap();
            let token = matches.get(0).unwrap().as_str(); // @name(arg)
            let func = matches.get(1).unwrap().as_str(); // name
            let arg = matches.get(2).unwrap().as_str(); // arg

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
        }

        Ok(vec![])
    }

    fn replace_file_group_tokens(
        &self,
        value: &str,
        token_type: TokenType,
    ) -> Result<Vec<String>, ProjectError> {
        token_type.check_context(&self.context)?;

        let mut files = vec![];

        let mut replace_token = |token: &str, replacement: &str| {
            files.push(String::from(value).replace(token, replacement));
        };

        let get_file_group = |token: &str, id: &str| match self.file_groups.get(id) {
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
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_utils::test::get_fixtures_dir;
    use std::collections::HashMap;

    fn create_file_groups() -> HashMap<String, FileGroup> {
        let project_root = get_fixtures_dir("base").join("files-and-dirs");
        let mut map = HashMap::new();

        map.insert(
            String::from("static"),
            FileGroup::new(
                "static",
                vec![
                    "file.ts".to_owned(),
                    "dir".to_owned(),
                    "dir/other.tsx".to_owned(),
                    "dir/subdir".to_owned(),
                    "dir/subdir/another.ts".to_owned(),
                ],
                &project_root,
            ),
        );

        map.insert(
            String::from("dirs_glob"),
            FileGroup::new("dirs_glob", vec!["**/*".to_owned()], &project_root),
        );

        map.insert(
            String::from("files_glob"),
            FileGroup::new(
                "files_glob",
                vec!["**/*.{ts,tsx}".to_owned()],
                &project_root,
            ),
        );

        map.insert(
            String::from("globs"),
            FileGroup::new(
                "globs",
                vec!["**/*.{ts,tsx}".to_owned(), "*.js".to_owned()],
                &project_root,
            ),
        );

        map.insert(
            String::from("no_globs"),
            FileGroup::new("no_globs", vec!["config.js".to_owned()], &project_root),
        );

        map
    }

    #[test]
    #[should_panic(expected = "UnknownFileGroup(\"@dirs(unknown)\", \"unknown\")")]
    fn errors_for_unknown_file_group() {
        let file_groups = create_file_groups();
        let resolver = TokenResolver::for_args(&file_groups);

        resolver.resolve("@dirs(unknown)").unwrap();
    }

    #[test]
    #[should_panic(expected = "NoGlobs(\"no_globs\")")]
    fn errors_if_no_globs_in_file_group() {
        let file_groups = create_file_groups();
        let resolver = TokenResolver::for_args(&file_groups);

        resolver.resolve("@globs(no_globs)").unwrap();
    }

    mod args {
        use super::*;

        #[test]
        fn supports_dirs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_args(&file_groups);

            assert_eq!(
                resolver.resolve("@dirs(static)").unwrap(),
                vec!["dir".to_owned(), "dir/subdir".to_owned(),]
            );
        }

        #[test]
        fn supports_dirs_with_globs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_args(&file_groups);

            assert_eq!(
                resolver.resolve("@dirs(dirs_glob)").unwrap(),
                vec!["dir".to_owned(), "dir/subdir".to_owned(),]
            );
        }

        #[test]
        fn supports_files() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_args(&file_groups);

            assert_eq!(
                resolver.resolve("@files(static)").unwrap(),
                vec![
                    "file.ts".to_owned(),
                    "dir/other.tsx".to_owned(),
                    "dir/subdir/another.ts".to_owned(),
                ]
            );
        }

        #[test]
        fn supports_files_with_globs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_args(&file_groups);

            assert_eq!(
                resolver.resolve("@files(files_glob)").unwrap(),
                vec![
                    "file.ts".to_owned(),
                    "dir/subdir/another.ts".to_owned(),
                    "dir/other.tsx".to_owned(),
                ]
            );
        }

        #[test]
        fn supports_globs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_args(&file_groups);

            assert_eq!(
                resolver.resolve("@globs(globs)").unwrap(),
                vec!["**/*.{ts,tsx}".to_owned(), "*.js".to_owned()],
            );
        }

        #[test]
        fn supports_root() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_args(&file_groups);

            assert_eq!(
                resolver.resolve("@root(static)").unwrap(),
                vec!["dir".to_owned()],
            );
        }
    }

    mod inputs {
        use super::*;

        #[test]
        fn supports_dirs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_inputs(&file_groups);

            assert_eq!(
                resolver.resolve("@dirs(static)").unwrap(),
                vec!["dir".to_owned(), "dir/subdir".to_owned(),]
            );
        }

        #[test]
        fn supports_dirs_with_globs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_inputs(&file_groups);

            assert_eq!(
                resolver.resolve("@dirs(dirs_glob)").unwrap(),
                vec!["dir".to_owned(), "dir/subdir".to_owned(),]
            );
        }

        #[test]
        fn supports_files() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_inputs(&file_groups);

            assert_eq!(
                resolver.resolve("@files(static)").unwrap(),
                vec![
                    "file.ts".to_owned(),
                    "dir/other.tsx".to_owned(),
                    "dir/subdir/another.ts".to_owned(),
                ]
            );
        }

        #[test]
        fn supports_files_with_globs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_inputs(&file_groups);

            assert_eq!(
                resolver.resolve("@files(files_glob)").unwrap(),
                vec![
                    "file.ts".to_owned(),
                    "dir/subdir/another.ts".to_owned(),
                    "dir/other.tsx".to_owned(),
                ]
            );
        }

        #[test]
        fn supports_globs() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_inputs(&file_groups);

            assert_eq!(
                resolver.resolve("@globs(globs)").unwrap(),
                vec!["**/*.{ts,tsx}".to_owned(), "*.js".to_owned()],
            );
        }

        #[test]
        fn supports_root() {
            let file_groups = create_file_groups();
            let resolver = TokenResolver::for_inputs(&file_groups);

            assert_eq!(
                resolver.resolve("@root(static)").unwrap(),
                vec!["dir".to_owned()],
            );
        }
    }
}
