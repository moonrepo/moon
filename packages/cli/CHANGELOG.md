# Changelog

## Unreleased

#### 🚀 Updates

- Added a special `noop` command for tasks.
- Added a `moon sync` command for manually syncing all projects in the workspace to a healthy state.

#### ⚙️ Internal

- Runfiles are no longer cleaned up when running tasks.
- Reworked `package.json` and `tsconfig.json` handling to avoid race conditions.

## 0.7.0

#### 💥 Breaking

- The `language` and `type` settings in `project.yml` now default to "unknown" when the setting is
  not defined, or the config does not exist. However, the language will also now be inferred
  (below).

#### 🚀 Updates

- Updated project `language` to be automatically inferred when the value is unknown, based on the
  existence of config files (`package.json` = javascript, `tsconfig.json` = typescript).
- Updated the `InstallNodeDeps` action to install dependencies when a `package.json` change is
  detected.
- Added a `moon dep-graph` command for displaying the entire dependency graph in DOT format.
- Added `--language` and `--type` filter options to `moon query projects`.
- Added `$language`, `$projectType`, and `$taskType` token variables.
- Added `dev` as a non-CI task identifier (alongside `start` and `serve`).
- Token variables can now be used within task `inputs`.
- Multiple token variables can now be used within the same string.

#### 🐞 Fixes

- Fixed an issue where package binaries would not execute on pnpm.

## 0.6.0

#### 🚀 Updates

- Added a new `@group` token that can be used be task `args` and `inputs`.
- Added a `moon query` command for querying information about moon, the environment, and more.
  - To start, `moon query touched-files` can be used to query touched files. The same files
    `moon ci` and `moon run` use.
  - Also `moon query projects` can be used to query about projects in the project graph.
- Added `bash` as a supported value for the project `language` setting.
- Added `typescript.createMissingConfig` and `typescript.rootOptionsConfigFileName` settings to
  `.moon/workspace.yml`.
- Updated TypeScript project reference syncing to automatically create missing `tsconfig.json`s.
- Updated `moon setup` and `moon teardown` to display spinners while running.

#### 🐞 Fixes

- Fixed an issue with a globally installed moon not being executable in PowerShell.
- Fixed an issue with empty files being passed to `git hash-object`.
- Fixed an issue where a `git merge-base` could not be resolved when base and head are provided.

#### ⚙️ Internal

- Updated Rust to v1.62.
- Refactored our action runner to support additional languages in the future.
- Refactored Windows to execute package binaries with `node.exe` directly, instead of with
  `cmd.exe` + the `.bin/*.cmd` file.

## 0.5.0

#### 🚀 Updates

- Added caching to our VCS layer which should greatly reduce the amount of `git` commands being
  executed.
- Updated `moon init` to detect `vcs.manager` and `vcs.defaultBranch` from the environment.

#### ⚙️ Internal

- We now detect the current Windows terminal using the `COMSPEC` environment variable, instead of
  defaulting to `cmd.exe`.
- Improved our configuration layer so that error messages include more metadata.
- Added `#[track_caller]` to more easily diagnose panics.

### 0.4.1

#### 🐞 Fixes

- Fixed logs unintentionally logging non-moon messages.

## 0.4.0

#### 🚀 Updates

- Added an `extends` setting to `.moon/workspace.yml` and `.moon/project.yml`.
- Added a `actionRunner.logRunningCommand` setting to `.moon/workspace.yml` for logging the task
  command being ran.
- Added a global `--logFile` option to the CLI. Also supports a new `MOON_LOG_FILE` environment
  variable.
- When targets are being ran in parallel, their output is now prefixed with the target name to
  differentiate. This is currently only enabled in CI.

#### 🐞 Fixes

- More fixes around terminal color output and handling.

#### ⚙️ Internal

- Temporarily disabling offline internet checks as it has issues with VPNs. Will revisit in the
  future.

### 0.3.1

#### 🐞 Fixes

- Fixed an issue where tasks referencing workspace relative files were not being marked as affected.
- Fixed some issues during `moon init` config generation.
- Improved offline checks by also verifying against Google's DNS.

## 0.3.0

#### 💥 Breaking

- Moved the `project.type` setting in `project.yml` to the top-level. Is now simply `type`.

#### 🚀 Updates

- Added support for a list of globs when configuring the `projects` setting in
  `.moon/workspace.yml`.
- Added a `actionRunner.inheritColorsForPipedTasks` setting to `.moon/workspace.yml` for inheriting
  terminal colors for piped tasks.
- Added a `language` setting to `project.yml` for defining the primary programming language of a
  project.
- Added a global `--color` option to the CLI. Also supports a new `MOON_COLOR` environment variable.

#### 🐞 Fixes

- Fixed many issues around terminal color output and handling.

## 0.2.0

#### 🚀 Updates

- Added support for macOS silicon (`aarch64-apple-darwin`).
- Added support for Linux musl (`x86_64-unknown-linux-musl`).
- Added support for the `MOON_LOG` environment variable.
- Added duration timestamps to all ran tasks in the terminal.
- Updated the JSON schemas to use the new package manager versions.
- Updated git file diffing to use `git merge-base` as the base reference.
- Updated `moon run` to exit early if there are no tasks for the provided target.
- Hashing will now ignore files that matched a pattern found in the root `.gitignore`.
- Passthrough args can now be defined for multi-target runs (`:target`).

#### 🐞 Fixes

- Fixed an issue with the `.moon/workspace.yml` template being generating with invalid whitespace
  during `moon init`.
