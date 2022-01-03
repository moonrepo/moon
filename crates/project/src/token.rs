use crate::errors::{ProjectError, TokenError};
use crate::file_group::FileGroup;
use moon_utils::regex::TOKEN_FUNC_PATTERN;
use std::collections::HashMap;

#[derive(PartialEq)]
pub enum ResolverType {
    Args,
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
    pub fn token(&self) -> String {
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

    type_of: ResolverType,
}

impl<'a> TokenResolver<'a> {
    pub fn for_args(file_groups: &'a HashMap<String, FileGroup>) -> TokenResolver {
        TokenResolver {
            file_groups,
            type_of: ResolverType::Args,
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
            let arg = matches.get(1).unwrap().as_str(); // arg

            return match arg {
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
        if self.type_of != ResolverType::Args {
            return Err(ProjectError::Token(TokenError::InvalidTokenInArgsContext(
                token_type.token(),
            )));
        }

        let mut files = vec![];
        let mut replace_token = |token: &str, replacement: &str| {
            files.push(String::from(value).replace(token, replacement));
        };
        let get_file_group = |id: &str| self.file_groups.get(id).unwrap();

        match token_type {
            TokenType::Dirs(token, group) => {
                for glob in get_file_group(&group).dirs()? {
                    replace_token(&token, &glob);
                }
            }
            TokenType::Files(token, group) => {
                for glob in get_file_group(&group).files()? {
                    replace_token(&token, &glob);
                }
            }
            TokenType::Globs(token, group) => {
                for glob in get_file_group(&group).globs()? {
                    replace_token(&token, &glob);
                }
            }
            TokenType::Root(token, group) => {
                replace_token(&token, &get_file_group(&group).root()?);
            }
        }

        Ok(files)
    }
}
