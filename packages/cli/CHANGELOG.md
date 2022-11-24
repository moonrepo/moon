# Changelog

## Unreleased

#### 🚀 Updates

- Added `vcs.remoteCandidates` to `.moon/workspace.yml` to customize the remotes for git to query
  against.
- Added support for `moduleSuffixes` and `moduleDetection` in TypeScript `tsconfig.json` compiler
  options.
- Added Google Cloud Build and AWS CodeBuild to the list of CI providers to detect. results.

##### Runner

- Updated the terminal output to include a shortened version of each task hash.
- Reworked the terminal output when running multiple tasks in parallel, or for long-running
  processes.
- Implemented a new file tree diffing algorithm that speeds up task output hydration by 10x.
- Updated pnpm to no longer run `pnpm prune` while deduping dependencies, as it produces unexpected

##### Generator

- Added `path_join` and `path_relative` template filters.
- Added pre-defined template variables for the working dir, destination, and workspace root.

#### 🐞 Fixes

- When writing JSON files, it will now respect the `indent_style = tab` setting in the closest
  `.editorconfig`.
- When writing YAML files, indentation and formatting will be inferred from the closest
  `.editorconfig` as best as possible.

#### ⚙️ Internal

- Migrated our json/yaml libraries to the official serde crates.
- Migrated to nextest for better testing performance.

## 0.19.1

#### 🚀 Updates

- Task `affectedFiles` will also be set via the `MOON_AFFECTED_FILES` env var.

#### 🐞 Fixes

- The runner will no longer attempt to install dependencies if running against affected files.
- Fixed some unexpected panics in relation to non-installed tools.

## 0.19.0

#### 💥 Breaking

- We've refactored how npm/pnpm/yarn work in the toolchain. Previously, they were installed as
  global packages (or via corepack) within the configured `~/.moon/tools/node` version. This
  approach worked but was susceptible to collisions, so now, these package managers are installed
  individually as their own tools at `~/.moon/tools/npm`, etc. This change should be transparent to
  you, but we're documenting it just in case something breaks!
- We've updated the dependency graph so that `InstallDeps` based actions use the task's `platform`
  instead of the project's `language` as the tool to install. This allows for granular control at
  the task level, and also unlocks the ability for project's to utilize multiple languages in the
  future.

#### 🚀 Updates

- When writing JSON files, indentation and formatting will be inferred from the closest
  `.editorconfig` as best as possible.
- When applicable, `moon ci` will group and collapse logs based on the current CI/CD environment.
- Updated webhook payloads to contain information about the current CI/CD environment under the
  `environment` field.

##### Runner

- Added an `affectedFiles` task option, allowing a filtered list of paths based on affected files to
  be passed as command line arguments. Perfect for git hooks!

##### Generator

- When generating files and a JSON or YAML file exists at the destination, you now have the option
  of merging files, instead of replacing entirely.

#### 🐞 Fixes

- Fixed an issue where passthrough args were incorrectly being passed to non-primary targets when
  using `moon run`.
- Fixed an issue where a root-level project was not being marked as affected based on touched files.
- Fixed an issue where tool version overrides at the project-level were not properly being set, and
  configuration that is root-only was being referenced in projects.
- Fixed some CLI arguments that should be ran mutually exclusive with other arguments.
- Task hashes will now properly invalidate if their dependencies hashes have also changed.

#### ⚙️ Internal

- Updated Rust to v1.65.

## 0.18.2

#### 🐞 Fixes

- Another attempt at fixing missing cache issues.
- Fixed an issue where moon would crash on old git versions (< 2.22.0) attempting to get the branch
  name. We encourage everyone to use v2.22 as the git minimum version.

## 0.18.1

#### 🚀 Updates

- Improved the resolution and hashing of `package.json` dependencies for Yarn and pnpm.

#### 🐞 Fixes

- Fixed an issue where caching would fail on missing `stdout.log` and `stderr.log` files.

## 0.18.0

#### 🚀 Updates

- Refactored `moon init` heavily for a better onboarding experience.
  - Each tool is now configured individually, with its own prompts. Tools can also be skipped.
  - Tools can now be initialized _after_ moon already exists, ala `moon init --tool node`.
  - Fixed many issues around the templates and rendering.
- Updated the `moon check` command to support an `--all` flag.
- Updated `moon migrate` commands to throw an error if the work tree is dirty. This can be bypassed
  with the new `--skipTouchedFilesCheck` option.
- Updated the `projects` setting in `.moon/workspace.yml` to support globs _and_ a map in unison.
- Updated default versions of tools:
  - node 16.17.0 -> 18.12.0
  - pnpm 7.12.1 -> 7.14.0
  - yarn 3.2.3 -> 3.2.4

##### Runner

- Added a `node.binExecArgs` setting to `.moon/workspace.yml`, so that additional `node` CLI
  arguments may be passed when executing the binary to run targets.
- Updated the task `command` to default to "noop" when not defined.
- The stdout and stderr of ran targets are now stored as individual log files in
  `.moon/cache/states/<project>/<task>`. This allows CI environments to cache them as artifacts,
  upload/download them, or simply help developers debug broken jobs.
  - Also, these log files are now stored in the output tarballs.

#### ⚙️ Internal

- Timestamps have been updated to UTC _without timezone_.
- Implemented a benchmarking system to start capturing performance changes.
- Improved language and platform interoperability.
- Extended configurations will now be temporarily cached for 4 hours.

## 0.17.0

#### 💥 Breaking

- Refactored project and task name/id cleaning. Previously, unsupported characters were simply
  removed. Instead, we now replace them with dashes for better readability.
- The task `type` in `moon.yml` and `.moon/project.yml` has been renamed to `platform`.
- The `$taskType` token has been renamed to `$taskPlatform`.

#### 🚀 Updates

- All YAML configuration files can now use
  [aliases and anchors](https://support.atlassian.com/bitbucket-cloud/docs/yaml-anchors/)!
- The `moon check` command can now use the `--report` option.

##### Tasks

- When defining `deps` within the current project, the `~:` prefix is now optional. For example,
  `~:build` can now be written as simply `build`.

##### Generator

- Enum variables can now declare an object form for `values`, so that a custom label can be provided
  for each value item.
- Added JSON schema support for the `template.yml` config.

##### Notifier

- Implemented a new service for notifying you about events happening in moon. The first feature in
  this service is webhooks!
- Added a new `notifier.webhookUrl` setting to `.moon/workspace.yml`, in which the webhooks endpoint
  can be defined.

#### ⚡️ Performance

- Enabled [mimalloc](https://github.com/microsoft/mimalloc). This reduces memory cost and increases
  runtime performance.
- Enabled link-time optimization, increases runtime performance.

## 0.16.1

#### 🐞 Fixes

- Fixed an issue where `moon init` would generate a config with invalid settings.
- Fixed an issue where downloading a tool would fail, but moon would still continue.

## 0.16.0

#### 🚀 Updates

##### Projects

- Projects can now override the workspace configured Node.js version on a per-project basis using
  the new `workspace.node.version` setting in `moon.yml`. However, this does not override the
  package manager!
- Package managers workspaces (via `package.json`) are no longer required. When not enabled, or a
  project is not within the workspace, it will install dependencies directly within the project
  root, and will utilize its own lockfile.

##### TypeScript

- Added a new `typescript.routeOutDirToCache` setting to `.moon/workspace.yml`, that will update the
  `outDir` compiler option to route to `.moon/cache/types`.
- Added a new `typescript.syncProjectReferencesToPaths` setting to `.moon/workspace.yml`, that will
  map project references to compiler option `paths` aliases.

##### Generator

- Template files can now be suffixed with `.tera` or `.twig` for syntax highlighting.

##### Runner

- The running command will now be displayed when installing dependencies (npm install, etc). This
  can be toggled with the `runner.logRunningCommand` setting.
- The dedupe command will now be displayed when running if the `node.dedupeOnLockfileChange` setting
  is enabled.
- Added a new `runner.implicitDeps` setting to `.moon/workspace.yml`, that will add task `deps` to
  _all_ tasks.

#### 📚 Docs

- Config file settings will now link to their API types.

#### ⚙️ Internal

- We've renamed and restructured the `.moon/cache` directory. If you were relying on any of these
  files, you'll need to update your implementation.
- Updated Cargo dependencies. A big change was clap v3 -> v4, so if you encounter any CLI issues,
  please report.

## 0.15.0

#### 🚀 Updates

- When running multiple targets in parallel, we've reworked the output prefix to be uniform amongst
  all targets, and to be colored to uniquely identify each target.
- Added a new `moon docker scaffold` command for scaffolding a skeleton workspace for use within
  `Dockerfile`s.
- Added a new `moon docker prune` command for pruning the Docker environment for a build/deply.
- Added frontmatter support to all template files.
- Added a `node.yarn.plugins` setting to `.moon/workspace.yml`.
- Updated run reports (via `--report`) to include additional information, like the total duration,
  and estimated time savings.
- Updated default versions of tools:
  - node 16.16.0 -> 16.17.0
  - npm 8.16.0 -> 8.19.2
  - pnpm 7.9.0 -> 7.12.1
  - yarn 3.2.2 -> 3.2.3

#### 🐞 Fixes

- Added missing `.npmrc` to the list of pnpm config files.
- Improved the handling of Rust/Go binaries shipped in pnpm node modules.

#### ⚙️ Internal

- Updated Rust to v1.64.
- Windows:
  - Will always use PowerShell and avoids `cmd.exe` entirely.
  - Reworked commands that run through PowerShell to pass arguments via stdin.

## 0.14.1

#### 🐞 Fixes

- Fixed an issue where alias warnings were logged while scanning the dependency graph.
- Windows:
  - Updated `*.cmd` executions to run with PowerShell when available. This resolves issues around
    file paths with special characters or spaces.

## 0.14.0

#### 🎉 Release

- Released a new GitHub action,
  [`moonrepo/run-report-action`](https://github.com/marketplace/actions/moon-ci-run-reports)!

#### 💥 Breaking

- Reworked how caching/hashing works when running in a Docker container/image. If the VCS root
  cannot be found, we disable caching. This removes the requirement of mounting a `.git` volume for
  Docker.

#### 🚀 Updates

- Added a new `moon generate` command, for code generation and scaffolding.
- Added a `generator` setting to `.moon/workspace.yml`, for controlling aspects of the generator and
  its templates.
- Updated the project graph to scan and find implicit dependencies based on language specific
  semantics. For example, will determine moon project relationships based on `package.json` names
  and dependencies.
- Updated `moon setup` to also install Node.js dependencies.

#### 🐞 Fixes

- Fixed an issue where project and task names were not being cleaned/formatted properly.

## 0.13.0

#### 💥 Breaking

- The `node` setting in `.moon/workspace.yml` is now optional, allowing repos to opt-out of Node.js
  support (in preparation for future languages support). This shouldn't affect you if the setting is
  already explicitly defined.
- Renamed `actionRunner` setting to `runner` in `.moon/workspace.yml`.

#### 🚀 Updates

- Added a new `moon check` command, for running all build/test tasks for a project(s).
- Added a `hasher` setting to `.moon/workspace.yml`, for controlling aspects of smart hashing.
- Updated hashing to utilize the resolved version from the lockfile when applicable.
- Updated the action runner to fail when an output is defined and the output does not exist after
  being ran.
- Released a new `@moonrepo/types` npm package.

#### ⚙️ Internal

- The `SetupToolchain` action has been updated to be language/platform aware, and as such, was split
  into `SetupNodeTool` and `SetupSystemTool`.
- Output is now buffered when running a target. This should reduce tearing and increase performance.
- Upgraded all Cargo dependencies.

## 0.12.1

#### 🐞 Fixes

- Fixed `init` templates being populated with the wrong default values.
- Fixed the "creation time is not available for the filesystem" error when running in Docker.

## 0.12.0

#### 💥 Breaking

- The `typescript` setting in `.moon/workspace.yml` is now optional, allowing repos to opt-out of
  TypeScript support. This shouldn't affect you if the setting is already explicitly defined.

#### 🚀 Updates

- Added support for Linux ARM GNU (`aarch64-unknown-linux-gnu`).
- Added support for Linux ARM musl (`aarch64-unknown-linux-musl`).
- Added a `workspace.typescript` setting to `moon.yml`, allowing TypeScript support to be toggled
  per project.
- Added a `--report` option to the `moon run` command, for generating run reports for debugging.
- Added an `--affected` option to the `moon query projects` command.
- Updated the task `command` to also support inline arguments. You can now merge `command` and
  `args` into a single field.

## 0.11.1

#### 🐞 Fixes

- Fixed an issue where `system` tasks were hashing incorrect contents.
- Fixed an issue where `envFile` is enabled and the relevant `.env` file may not exist in CI.

## 0.11.0

#### 🚀 Updates

- Added a `moon clean` command for manually clearing the cache.
- Added an `actionRunner.cacheLifetime` setting to `.moon/workspace.yml`, for controlling the stale
  cache threshold.
- Added an `envFile` option to tasks, allowing `.env` files to be loaded for environment variables.
- Added a `local` setting to tasks, that marks the task for local development only.
- Updated the `outputStyle` task option with additional variants: `buffer`, `buffer-only-failure`,
  `hash`, `none`.
- Updated `moon run` to support running multiple targets concurrently.

#### 🐞 Fixes

- Fixed an issue where output hydration was bypassing "off" cache.
- Fixed an issue where parsing a node module binary would panic.
- Fixed an issue where moon would panic attempting to read non-JS code shipped in node modules (Rust
  or Go binaries).
- Fixed an issue where project globs would pickup dot folders (`.git`, `.moon`, etc) or
  `node_modules`.
- Fixed an issue where project names were stripping capital letters when using globs.

#### ⚙️ Internal

- Updated Rust to v1.63.

## 0.10.0

#### 💥 Breaking

- Task outputs are now cached as `.tar.gz` archives, instead of being copied as-is. This shouldn't
  affect consumers, but we're raising awareness in case of any platform specific issues.
- Renamed the project-level `project.yml` file to `moon.yml`. The `.moon/project.yml` file has not
  changed.

#### 🚀 Updates

- Projects now support language specific aliases, which can be used as a drop-in replacement for
  names within targets and dependencies.
- Project and tasks names now support forward slashes (`/`).
- Added a `node.aliasPackageNames` setting to `.moon/workspace.yml`, that aliases the `package.json`
  name to the respective project.
- Added an experimental `outputStyle` option to tasks, providing some control of how stdout/stderr
  is handled.
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

- Outputs are now copied to `.moon/cache/outputs` instead of being hardlinked.
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

#### 📚 Documentation

- Add "released in version" badges/labels to new features across all docs.

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
