# Changelog

## Unreleased

#### 💥 Breaking

- Task outputs are now cached as `.tar.gz` archives, instead of being copied as-is. This shouldn't
  affect consumers, but we're raising awareness in case of any platform specific issues.
- Renamed the project-level `project.yml` file to `moon.yml`. The `.moon/project.yml` file has not
  changed.

#### 🚀 Updates

- Added a `runDepsInParallel` option to tasks, that controls whether task dependencies run in
  parallel or serial (in order).
- Updated tasks to automatically detect their `type` (when undefined) based on their defined
  `command`. Will attempt to match against common system commands, like `rm`, `mkdir`, etc.
- When in CI, Node.js will not install dependencies if they were already installed before moon runs.
  This should avoid unintentional and unnecessary double installs.
- Updated default versions of tools:
  - node 16.15.0 -> 16.16.0
  - npm 8.10.0 -> 8.16.0
  - pnpm 7.1.5 -> 7.9.0
  - yarn 3.2.1 -> 3.2.2

#### 🐞 Fixes

- Fixed some issues where task outputs were not being hydrated based on the state of the
  target/project.
- Fixed an issue where task outputs were not considered for hash generation.

## 0.9.1

#### 🐞 Fixes

- Fixed an issue where a root-level project cannot be configured with a glob. Updated `projects`
  glob matching to support `'.'`.
- Fixed an issue where moon was setup in a sub-folder. Updated git/svn to traverse upwards to find
  the applicable root (`.git`, etc).

## 0.9.0

#### 💥 Breaking

We've refactored our smart hashing layer to take into account different platforms (a task's type) in
an effort to be more accurate, which now results in different hashes for the same build. Any
previous builds are no longer valid and can be removed.

#### 🚀 Updates

- Updated task `type` to be automatically inferred when the value is unknown, based on the owning
  project's `language` (`javascript` = node, `bash` = system, etc).
- Updated `dependsOn` in `project.yml` to support an object form, where a scope (production,
  development, peer) can also be defined. This maps to the appropriate field in `package.json` when
  syncing.
- Added `batch` as a supported value for the project `language` setting (Windows counter-part to
  `bash`).
- Added a `cache` option to tasks, which will disable smart hashing and output caching.
- Added a `node.dependencyVersionFormat` setting to `.moon/workspace.yml`, to customize the version
  format when syncing dependencies.
- Added environment variable support to task `inputs` and `actionRunner.implicitInputs`, in the
  format of `$ENV_VAR`.

#### 🐞 Fixes

- Fixed an issue where pnpm didn't work with `node-linker=isolated` for nested node modules.
- Fixed an issue where failing processes would display an empty error message.

#### ⚙️ Internal

- Outputs are now copied to `.moon/cache/out` instead of being hardlinked.
- Package binaries are now resolved to their canonical path when a symlink.

### 0.8.1

#### 🐞 Fixes

- Fixed a crash when `node.packageManager` was set to "pnpm" or "yarn" but `node.pnpm` or
  `node.yarn` fields were not set.

## 0.8.0

This release was largely focused on interoperability with the Node.js ecosystem, specifically
`package.json` scripts. It's the first step in many steps, so stay tuned!

#### 🚀 Updates

- Added a special `noop` command for tasks.
- Added a `moon migrate from-package-json` command for migrating `package.json` scripts to
  `project.yml` tasks.
- Added a `moon sync` command for manually syncing all projects in the workspace to a healthy state.
- Added a `node.inferTasksFromScripts` setting to `.moon/workspace.yml`, that will automatically
  infer tasks from `package.json` scripts (with caveats).
- Added aliases for popular commands:
  - `moon r` -> `moon run`
  - `moon p` -> `moon project`
  - `moon pg` -> `moon project-graph`
  - `moon dg` -> `moon dep-graph`

#### 🐞 Fixes

- Fixed an issue where files being hashed with git were not being cached accordingly.

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
