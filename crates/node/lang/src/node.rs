use crate::package_json::{PackageJson, PackageWorkspaces};
use crate::pnpm::workspace::PnpmWorkspace;
use crate::NODE;
use cached::proc_macro::cached;
use miette::IntoDiagnostic;
use moon_utils::{path, regex};
use once_cell::sync::Lazy;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};

static BIN_PATH_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    regex::create_regex("(?:(?:\\.+(?:\\\\|/)))+(?:(?:[.a-zA-Z0-9-_@+]+)(?:\\\\|/))+[a-zA-Z0-9-_]+(\\.((c|m)?js|exe))?").unwrap()
});

// https://nodejs.org/api/modules.html#loading-from-the-global-folders
#[inline]
pub fn extend_node_path<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref();
    let delimiter = if cfg!(windows) { ";" } else { ":" };

    match env::var("NODE_PATH") {
        Ok(old_value) => format!("{value}{delimiter}{old_value}"),
        Err(_) => value.to_owned(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinFile {
    Binary(PathBuf),        // Rust, Go
    Script(PathBuf),        // JavaScript
    Other(PathBuf, String), // Bash, Batch, etc.
}

/// Node module ".bin" files may be JavaScript, Bash, Go, or Rust.
///
/// npm:
///     - Unix: Symlinks to the bin file in node_modules.
///     - Windows: Creates a wrapping shell script AND .cmd that executes the bin file.
/// pnpm:
///     - Unix: Creates a wrapping shell script that executes the bin file.
///     - Windows: Creates a wrapping shell script AND .cmd that executes the bin file.
/// Yarn:
///     - Unix: Symlinks to the bin file in node_modules.
///     - Windows: Creates a wrapping shell script AND .cmd that executes the bin file.
#[cached(result)]
#[track_caller]
pub fn extract_canonical_node_module_bin(bin_path: PathBuf) -> miette::Result<BinFile> {
    // Resolve to the real file location if applicable
    let bin_path = if bin_path.is_symlink() {
        bin_path.canonicalize().into_diagnostic()?
    } else {
        bin_path
    };

    let buffer = fs::read_file_bytes(&bin_path)?;

    // Found a Rust or Go binary shipped in node modules, abort early
    if content_inspector::inspect(&buffer).is_binary() {
        return Ok(BinFile::Binary(bin_path));
    }

    let contents = String::from_utf8(buffer).into_diagnostic()?;
    let shebang = contents.starts_with("#!");

    // Found a JavaScript file, use as-is
    if has_shebang(&contents, "node") {
        return Ok(BinFile::Script(bin_path));
    }

    let is_pwsh = is_cmd_file(&contents) || has_shebang(&contents, "pwsh");
    let is_bash = has_shebang(&contents, "bash") || has_shebang(&contents, "sh");

    // Found a bash/batch script, extract the relative bin path from it
    if is_pwsh || is_bash {
        let Some(extracted_path) = parse_bin_file(contents) else {
            // No path found, might just be a regular Bash script
            return Ok(if shebang {
                BinFile::Binary(bin_path)
            } else if is_pwsh {
                BinFile::Other(bin_path, "powershell".into())
            } else {
                BinFile::Other(bin_path, "bash".into())
            });
        };

        let extracted_bin = path::normalize(bin_path.parent().unwrap().join(extracted_path));

        // Do another pass, as the extracted file may be a binary
        return extract_canonical_node_module_bin(extracted_bin);
    }

    Ok(BinFile::Script(bin_path))
}

#[inline]
#[track_caller]
pub fn find_package_bin<P: AsRef<Path>, B: AsRef<str>>(
    starting_dir: P,
    bin_name: B,
) -> miette::Result<Option<BinFile>> {
    let starting_dir = starting_dir.as_ref();
    let bin_name = bin_name.as_ref();
    let bin_path = starting_dir
        .join(NODE.vendor_bins_dir.unwrap())
        .join(get_bin_name_suffix(bin_name, "cmd", true));

    if bin_path.exists() {
        return Ok(Some(extract_canonical_node_module_bin(bin_path)?));
    }

    Ok(match starting_dir.parent() {
        Some(dir) => find_package_bin(dir, bin_name)?,
        None => None,
    })
}

#[inline]
pub fn find_package_manager_bin<P: AsRef<Path>, B: AsRef<str>>(
    install_dir: P,
    bin_name: B,
) -> PathBuf {
    install_dir
        .as_ref()
        .join(get_bin_name_suffix(bin_name, "cmd", false))
}

#[inline]
pub fn get_bin_name_suffix<T: AsRef<str>>(name: T, windows_ext: &str, flat: bool) -> String {
    let name = name.as_ref();

    if cfg!(windows) {
        format!("{name}.{windows_ext}")
    } else if flat {
        name.to_owned()
    } else {
        format!("bin/{name}")
    }
}

/// Extract the list of `workspaces` globs from the root `package.json`,
/// or if using pnpm, extract the globs from `pnpm-workspace.yaml`.
/// Furthermore, if the list is found, but is empty, return none.
#[cached(result)]
pub fn get_package_manager_workspaces(
    workspace_root: PathBuf,
) -> miette::Result<Option<Vec<String>>> {
    if let Some(pnpm_workspace) = PnpmWorkspace::read(workspace_root.clone())? {
        if !pnpm_workspace.packages.is_empty() {
            return Ok(Some(pnpm_workspace.packages));
        }
    }

    if let Some(package_json) = PackageJson::read(workspace_root)? {
        if let Some(workspaces) = package_json.workspaces {
            match workspaces {
                PackageWorkspaces::Array(globs) => {
                    if !globs.is_empty() {
                        return Ok(Some(globs));
                    }
                }
                PackageWorkspaces::Object(config) => {
                    if let Some(globs) = config.packages {
                        if !globs.is_empty() {
                            return Ok(Some(globs));
                        }
                    }
                }
            };
        }
    }

    Ok(None)
}

#[inline]
pub fn has_shebang(contents: &str, command: &str) -> bool {
    contents.starts_with(&format!("#!/usr/bin/env {command}"))
        || contents.starts_with(&format!("#!/usr/bin/{command}"))
        || contents.starts_with(&format!("#!/bin/{command}"))
}

#[inline]
pub fn is_cmd_file(contents: &str) -> bool {
    contents.contains("%~dp0")
        || contents.contains("%dp0%")
        || contents.contains("@SETLOCAL")
        || contents.contains("@ECHO")
}

#[inline]
#[track_caller]
pub fn parse_bin_file(contents: String) -> Option<PathBuf> {
    let Some(captures) = BIN_PATH_PATTERN.captures(&contents) else {
        return None;
    };

    Some(PathBuf::from(captures.get(0).unwrap().as_str()))
}

#[inline]
pub fn parse_package_name(package_name: &str) -> (Option<String>, String) {
    let scope;
    let name;

    if package_name.contains('/') {
        let mut parts = package_name.split('/');

        scope = Some(parts.next().unwrap().to_owned());
        name = parts.next().unwrap().to_owned();
    } else {
        scope = None;
        name = package_name.to_owned();
    }

    (scope, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_test_utils::{assert_fs::prelude::*, create_temp_dir, TempDir};

    fn create_cmd(path: &str) -> String {
        format!(
            r#"
@IF EXIST "%~dp0\node.exe" (
    "%~dp0\node.exe" "%~dp0\{path}" %*
) ELSE (
    SETLOCAL
    SET PATHEXT=%PATHEXT:;.JS;=;%
    node "%~dp0\{path}" %*
)"#
        )
    }

    fn create_node_modules_sandbox() -> TempDir {
        let sandbox = create_temp_dir();

        sandbox
            .child("node_modules/@scope/pkg-foo/package.json")
            .write_str("{}")
            .unwrap();

        sandbox
            .child("node_modules/pkg-bar/package.json")
            .write_str("{}")
            .unwrap();

        sandbox
            .child("node_modules/baz/bin.js")
            .write_str("{}")
            .unwrap();

        sandbox
            .child("node_modules/.bin/baz")
            .write_str(&create_cmd(r"../baz/bin.js"))
            .unwrap();

        sandbox
            .child("node_modules/.bin/baz.cmd")
            .write_str(&create_cmd(r"..\baz\bin.js"))
            .unwrap();

        sandbox.child("nested/file.js").write_str("{}").unwrap();

        sandbox
    }

    mod parse_bin_file {
        use super::*;

        #[test]
        fn basic_path() {
            assert_eq!(
                parse_bin_file(create_cmd(r"..\typescript\bin\tsc"),).unwrap(),
                PathBuf::from(r"..\typescript\bin\tsc")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"../typescript/bin/tsc"),).unwrap(),
                PathBuf::from(r"../typescript/bin/tsc")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"..\json5\lib\cli.js"),).unwrap(),
                PathBuf::from(r"..\json5\lib\cli.js")
            );
        }

        #[test]
        fn supports_periods() {
            assert_eq!(
                parse_bin_file(create_cmd(r"..\.dir\bin\bin"),).unwrap(),
                PathBuf::from(r"..\.dir\bin\bin")
            );
        }

        #[test]
        fn relative_paths() {
            assert_eq!(
                parse_bin_file(create_cmd(r".\eslint\bin\eslint"),).unwrap(),
                PathBuf::from(r".\eslint\bin\eslint")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"..\..\eslint\bin\eslint"),).unwrap(),
                PathBuf::from(r"..\..\eslint\bin\eslint")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"./eslint/bin/eslint"),).unwrap(),
                PathBuf::from(r"./eslint/bin/eslint")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"../../eslint/bin/eslint"),).unwrap(),
                PathBuf::from(r"../../eslint/bin/eslint")
            );
        }

        #[test]
        fn with_exts() {
            assert_eq!(
                parse_bin_file(create_cmd(r"..\babel\index.js"),).unwrap(),
                PathBuf::from(r"..\babel\index.js")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"../babel/index.js"),).unwrap(),
                PathBuf::from(r"../babel/index.js")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r".\webpack\dist\cli.cjs"),).unwrap(),
                PathBuf::from(r".\webpack\dist\cli.cjs")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r".\..\rollup\build\rollup.mjs"),).unwrap(),
                PathBuf::from(r".\..\rollup\build\rollup.mjs")
            );

            assert_eq!(
                parse_bin_file(create_cmd(
                    r"..\webpack-dev-server\bin\webpack-dev-server.js"
                ),)
                .unwrap(),
                PathBuf::from(r"..\webpack-dev-server\bin\webpack-dev-server.js")
            );
        }

        #[test]
        fn with_scopes() {
            assert_eq!(
                parse_bin_file(create_cmd(r"..\@scope\foo\bin"),).unwrap(),
                PathBuf::from(r"..\@scope\foo\bin")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"..\@scope\foo-bar\bin.js"),).unwrap(),
                PathBuf::from(r"..\@scope\foo-bar\bin.js")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"..\@scope-long\foo-bar\bin_file.js"),).unwrap(),
                PathBuf::from(r"..\@scope-long\foo-bar\bin_file.js")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"..\@docusaurus\core\bin\docusaurus.mjs"),).unwrap(),
                PathBuf::from(r"..\@docusaurus\core\bin\docusaurus.mjs")
            );

            assert_eq!(
                parse_bin_file(create_cmd(r"..\@babel\parser\bin\babel-parser.js"),).unwrap(),
                PathBuf::from(r"..\@babel\parser\bin\babel-parser.js")
            );
        }

        #[test]
        fn parses_pnpm() {
            assert_eq!(
                parse_bin_file(
                    r#"
#!/bin/sh
basedir=$(dirname "$(echo "$0" | sed -e 's,\\,/,g')")

case `uname` in
    *CYGWIN*) basedir=`cygpath -w "$basedir"`;;
esac

if [ -z "$NODE_PATH" ]; then
  export NODE_PATH="/Projects/moon/node_modules/.pnpm/node_modules"
else
  export NODE_PATH="$NODE_PATH:/Projects/moon/node_modules/.pnpm/node_modules"
fi
if [ -x "$basedir/node" ]; then
  exec "$basedir/node"  "$basedir/../typescript/bin/tsc" "$@"
else
  exec node  "$basedir/../typescript/bin/tsc" "$@"
fi
                    "#
                    .to_string(),
                )
                .unwrap(),
                PathBuf::from(r"../typescript/bin/tsc")
            );

            assert_eq!(
                parse_bin_file(
                    r#"
#!/bin/sh
basedir=$(dirname "$(echo "$0" | sed -e 's,\\,/,g')")

case `uname` in
    *CYGWIN*) basedir=`cygpath -w "$basedir"`;;
esac

if [ -z "$NODE_PATH" ]; then
  export NODE_PATH="C:\Projects\moon\node_modules\.pnpm\node_modules"
else
  export NODE_PATH="$NODE_PATH:C:\Projects\moon\node_modules\.pnpm\node_modules"
fi
if [ -x "$basedir\node" ]; then
  exec "$basedir\node"  "$basedir\..\typescript\bin\tsc" "$@"
else
  exec node  "$basedir\..\typescript\bin\tsc" "$@"
fi
                    "#
                    .to_string(),
                )
                .unwrap(),
                PathBuf::from(r"..\typescript\bin\tsc")
            );
        }

        #[test]
        fn parses_pnpm_isolated_linker() {
            assert_eq!(
                parse_bin_file(

                    r#"
#!/bin/sh
basedir=$(dirname "$(echo "$0" | sed -e 's,\\,/,g')")

case `uname` in
    *CYGWIN*) basedir=`cygpath -w "$basedir"`;;
esac

if [ -z "$NODE_PATH" ]; then
    export NODE_PATH="/Users/milesj/Projects/moon/node_modules/.pnpm/node_modules"
else
    export NODE_PATH="$NODE_PATH:/Users/milesj/Projects/moon/node_modules/.pnpm/node_modules"
fi
if [ -x "$basedir/node" ]; then
    exec "$basedir/node"  "$basedir/../../../node_modules/.pnpm/@docusaurus+core@2.0.0-beta.20_sfoxds7t5ydpegc3knd667wn6m/node_modules/@docusaurus/core/bin/docusaurus.mjs" "$@"
else
    exec node  "$basedir/../../../node_modules/.pnpm/@docusaurus+core@2.0.0-beta.20_sfoxds7t5ydpegc3knd667wn6m/node_modules/@docusaurus/core/bin/docusaurus.mjs" "$@"
fi
                    "#
                    .to_string(),
                ).unwrap(),
                PathBuf::from(r"../../../node_modules/.pnpm/@docusaurus+core@2.0.0-beta.20_sfoxds7t5ydpegc3knd667wn6m/node_modules/@docusaurus/core/bin/docusaurus.mjs")
            );
        }

        #[test]
        fn parses_moon_exe() {
            assert_eq!(
                parse_bin_file(
                    r#"
#!/bin/sh
basedir=$(dirname "$(echo "$0" | sed -e 's,\\,/,g')")

case `uname` in
    *CYGWIN*) basedir=`cygpath -w "$basedir"`;;
esac

exec "$basedir\..\@moonrepo\cli\moon.exe" "$@"
                    "#
                    .to_string(),
                )
                .unwrap(),
                PathBuf::from(r"..\@moonrepo\cli\moon.exe")
            );
        }

        #[test]
        fn literal_bash_scripts() {
            assert_eq!(
                parse_bin_file(
                    r#"
#!/usr/bin/env bash

# Forward all arguments to the local CLI (@expo/cli).
npm exec --no-install -- expo-internal "$@"
                    "#
                    .to_string(),
                ),
                None
            );

            assert_eq!(
                parse_bin_file(
                    r#"
#!/usr/bin/env sh

some-other command --that does nothing
                    "#
                    .to_string(),
                ),
                None
            );
        }
    }

    mod get_bin_name_suffix {
        use super::*;

        #[test]
        #[cfg(windows)]
        fn supports_cmd() {
            assert_eq!(
                get_bin_name_suffix("foo", "cmd", false),
                "foo.cmd".to_owned()
            );
        }

        #[test]
        #[cfg(windows)]
        fn supports_ps1() {
            assert_eq!(
                get_bin_name_suffix("foo", "ps1", false),
                "foo.ps1".to_owned()
            );
        }

        #[test]
        #[cfg(windows)]
        fn supports_exe() {
            assert_eq!(
                get_bin_name_suffix("foo", "exe", true),
                "foo.exe".to_owned()
            );
        }

        #[test]
        #[cfg(not(windows))]
        fn returns_nested_bin() {
            assert_eq!(
                get_bin_name_suffix("foo", "ext", false),
                "bin/foo".to_owned()
            );
        }

        #[test]
        #[cfg(not(windows))]
        fn returns_flat_bin() {
            assert_eq!(get_bin_name_suffix("foo", "ext", true), "foo".to_owned());
        }
    }

    mod find_package_bin {
        use super::*;

        #[test]
        fn returns_path_if_found() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package_bin(sandbox.path(), "baz");

            assert_eq!(
                path.unwrap().unwrap(),
                BinFile::Script(
                    sandbox
                        .path()
                        .join("node_modules")
                        .join("baz")
                        .join("bin.js")
                )
            );
        }

        #[test]
        fn returns_path_from_nested_file() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package_bin(sandbox.path().join("nested"), "baz");

            assert_eq!(
                path.unwrap().unwrap(),
                BinFile::Script(
                    sandbox
                        .path()
                        .join("node_modules")
                        .join("baz")
                        .join("bin.js")
                )
            );
        }

        #[test]
        fn returns_none_for_missing() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package_bin(sandbox.path(), "unknown-binary");

            assert_eq!(path.unwrap(), None);
        }
    }
}
