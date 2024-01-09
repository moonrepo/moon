# Changelog

## Future roadmap

#### 🚀 Updates

- Rewrote the actions pipeline from the ground-up:
  - Increased performance.
  - Better concurrency handling and scheduling.
  - More accurately monitors signals (ctrl+c) and shutdowns.
  - Tasks can now be configured with a timeout.

## 1.19.3

This fixes a bad 1.19.2 release.

## 1.19.2

#### 🐞 Fixes

- Fixed another location where the `proto` binary was not available.

## 1.19.1

#### 🐞 Fixes

- Fixed `proto` binary not being available in a Dockerfile when running `moon docker` commands.
- Fixed our `@moonrepo/cli` postinstall script not working correctly for Bun Arm64.

## 1.19.0

#### 💥 Breaking

- Removed the `experiments.interweavedTaskInheritance` setting from `.moon/workspace.yml`.
  Interweaved inheritance is now always enabled (was previously true by default).
- Removed the `experiments.taskOutputBoundaries` setting from `.moon/workspace.yml`. We opted to
  remove boundaries entirely, as they caused more problems than solved. Task outputs may now overlap
  without issue.

#### 🚀 Updates

- Updated `implicitDeps` in `.moon/tasks.yml` and task `deps` in `moon.yml` to support arguments and
  environment variables for the dependency target.
- Updated the action graph and pipeline to _not_ run the same target (but with different arguments
  and environment variable variations) in parallel, to avoid unexpected collisions.
- Updated VS Code extension to support multiple VS Code workspace folders.
- Improved code generation and templates:
  - Added a `destination` field to `template.yml`, to customize a default location.
  - Added a `extends` field to `template.yml`, allowing templates to extend and inherit other
    templates.
  - Updated `[var]` syntax to support filters: `[var | camel_case]`.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.7.
- Updated `node_plugin` and `node_depman_plugin` to v0.7.
- Updated `rust_plugin` to v0.6.

#### ⚙️ Internal

- Updated Rust to v1.75.
- Updated proto to v0.26.4 (from v0.25).

## 1.18.5

#### 🚀 Updates

- Reworked duplicate project ID/alias detection to be more accurate. Will also now error instead of
  warn when a duplicate is detected.
  - For aliases, the error can be disabled with the new `experiments.strictProjectAliases` setting
    in `.moon/workspace.yml`.
  - For project IDs, the error can not be disabled, as conflicting IDs will cause issues with the
    project graph.

#### 🐞 Fixes

- Fixed glob based project locating to not log warnings when a file is found and it starts with `.`
  (ignore dotfiles).
- Fixed project aliases potentially overwriting a project with the same name.

## 1.18.4

#### 🚀 Updates

- Updated the proto installation step to download, unpack, and install using Rust, instead of
  relying on our Bash/PowerShell scripts. This should remove the requirement that openssl, tar, and
  other environment tools must exist.

#### ⚙️ Internal

- Updated proto to v0.25.3.

## 1.18.3

#### 🐞 Fixes

- Fixed more issues in relation to custom project IDs not resolving correctly.

#### ⚙️ Internal

- Improved some error messages with more information.

## 1.18.2

#### 🚀 Updates

- Silenced proto migration warnings when ran in the context of moon.

#### 🐞 Fixes

- Fixed an issue where `@dirs` and `@files` tokens didn't always work correctly in `outputs`.
- Fixed the `@moonrepo/cli` package pulling in different `@moonrepo/core-*` versions

#### ⚙️ Internal

- Updated proto to v0.25.2.

## 1.18.1

#### 🐞 Fixes

- Fixed an issue where we would install `proto` even when not required.
- Fixed an issue where implicit dependencies were not resolving correctly when projects were
  renamed.

## 1.18.0

#### 🚀 Updates

- Rewrote toolchain based task running to use a path based approach.
  - Instead of manually locating an executable, we now rely on `PATH` to locate the executable.
  - Non-system tasks can now be wrapped in a shell using the `shell` option.
  - This approach will now benefit from proto shims and binaries.
  - We'll also download and install the `proto` binary if it does not exist.
- Reworked the `moon init` command.
  - Will no longer scaffold the toolchain configuration by default.
  - The tool to scaffold into a toolchain can be passed as an argument.
  - The path to initialize in is now behined the `--to` option.
  - Added support for the `bun` tool.
  - Simplified the workflow overall.
- Updated `moon.yml` to support customizing the project name using the `id` field.
  - Can be used to override the project name derived in `.moon/workspace.yml`.
- Added a `MOON_INSTALL_DIR` environment variable, to control where the `moon` binary is installed
  to.

#### 🐞 Fixes

- Fixed `moon upgrade` failing when not ran in a moon workspace.
- Fixed `CODEOWNERS` being written with double trailing newlines.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.6.
- Updated `node_plugin` and `node_depman_plugin` to v0.6.1.
- Updated `rust_plugin` to v0.5.

#### ⚙️ Internal

- Improved string allocation and performance for queries, task tokens, and process commands.
- Improved remote caching flow and handling.
- Updated proto to v0.25.

## 1.17.4

#### 🐞 Fixes

- Fixed an issue where executing moon (and indirectly proto) would run into privilege access issues
  on Windows.
- Fixed `typescript.includeProjectReferenceSources` and `typescript.syncProjectReferencesToPaths`
  settings not including project references that were manually added (not auto-synced).
- Fixed the "a project already exists with alias" warnings when using Bun and Node together.

#### ⚙️ Internal

- Added canary release support.
- Enabled wasmtime caching, which should improve performance of WASM plugins by 10-20%.
- Updated proto to v0.23.7.

## 1.17.3

#### 🐞 Fixes

- Fixed an issue where we would fail to find Cargo binaries on Windows.

#### ⚙️ Internal

- Updated proto to v0.23.3.

## 1.17.2

#### 🐞 Fixes

- Fixed an issue where `cargo-binstall` would error when trying to install it and it already exists.

## 1.17.1

#### 🐞 Fixes

- Fixed the wrong version being reported by the CLI.

## 1.17.0

#### 🚀 Updates

- Integrated full Bun support (tier 1-3).
  - Will download and install Bun into the toolchain when a `version` is configured.
  - Will parse the `bun.lockb` lockfile to extract and resolve dependencies.
  - Will hash manifests and inputs for Bun specific caching.
  - Added a `bun` setting to `.moon/toolchain.yml`.
  - Added a `toolchain.bun` setting to `moon.yml`.
  - Updated `moon bin` and `moon docker` commands to support Bun.
  - Updated task `platform` to support "bun".
- Improved TypeScript support.
  - Added a `typescript.root` setting to denote the TypeScript root.
  - Added a `typescript.includeSharedTypes` setting, for syncing a shared types path to all
    project's `include`.
  - Added a `typescript.includeProjectReferenceSources` setting, for syncing project reference files
    to all project's `include`.
  - Updated `typescript.syncProjectReferencesToPaths` setting to always include the wildcard, and
    not require an index file.
  - Improved project reference syncing and edge case handling.
- Improved JavaScript support.
  - Added `bun.rootPackageOnly` and `node.rootPackageOnly` settings to support the "one version
    rule" pattern.
  - Updated automatic dependency linking to use the `build` scope instead of `peer` scope. This
    should alleviate some of the pain points with `package.json` syncing.

## 1.16.5

#### 🐞 Fixes

- Fixed an issue where codegen would merge JSON/YAML files with the incorrect source.
- Updated file traversal to not walk outside of the workspace root.

#### ⚙️ Internal

- Updated Rust to v1.74.
- Updated proto to v0.23.0.
- Updated dependencies.
- Updated logs to now include nanoseconds.

## 1.16.4

#### 🚀 Updates

- Update project graph hashing to include git ignored `moon.yml` files.

#### 🐞 Fixes

- Fixed Yarn v1.22.x download not unpacking correctly.

#### 🧩 Plugins

- Updated Yarn `node_depman_plugin` to v0.5.1.

## 1.16.2/3

#### 🚀 Updates

- Updated `projects` globs to support ending in `moon.yml`.
- Updated `node.dependencyVersionFormat` to fallback to a supported format when the chosen
  `node.packageManager` does not support the configured (or default) version format.
- Updated to proto v0.22.0.

#### 🐞 Fixes

- Fixed an issue where dependencies were being injected into the root `package.json`, when a
  root-level project was dependending on non-root project tasks.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.5.
- Updated `deno_plugin` to v0.5.
- Updated `go_plugin` to v0.5.
- Updated `node_plugin` and `node_depman_plugin` to v0.5.
- Updated `python_plugin` to v0.2.
- Updated `rust_plugin` to v0.4.
- Updated `schema_plugin` (TOML) to v0.5.

## 1.16.1

#### 🐞 Fixes

- Fixed `moon ci` not treating dependents as "CI only" when running locally.
- Fixed the MQL parser failing on projects that contain a `.`.
- Fixed JSON comment stripping not handling docblock styled comments (`/** **/`).

## 1.16.0

#### 🚀 Updates

- Added Bun as a supported Node.js package manager: `node.packageManager: 'bun'`.
- Added components and targets support for the Rust toolchain.
  - Added `rust.components` and `rust.targets` settings to `.moon/toolchain.yml`.
  - Will automatically be installed with `rustup` when the pipeline is ran.
- Added a `MOON_TOOLCHAIN_FORCE_GLOBALS` environment variable, that will force all toolchain tools
  to use the global binary available on `PATH`, instead of downloading and installing.
- Added an improved task inheritance chain resolver.
  - Global and local tasks are now interweaved within the chain, where as previously global was
    built first, then local.
  - To fallback to the previous behavior, set `experiments.interweavedTaskInheritance: false` in
    `.moon/workspace.yml`.
- Added a new project type `automation`, for projects like E2E and integration testing.
- Updated action graph cycle detection to list all nodes in the cycle (when detectable).
- Updated all npx calls to use a package manager equivalent. For example: `yarn dlx`, `pnpm dlx`,
  `bunx`.
- Updated to support Yarn v4.

#### 🐞 Fixes

- Fixed an issue where `moon ci` and `git` would fail if there's only 1 commit on the base branch.
- Fixed an issue where `runInCI` was not respected when a task is a dependency of an affected task.
- Fixed an issue where the task `replace` merge strategy would not apply for empty values.

#### ⚙️ Internal

- Updated dependencies.
- Updated to proto v0.21.0.
- Pinned proto plugins to a fixed version instead of using latest.

## 1.15.4

#### 🐞 Fixes

- Fixed an issue where pnpm would fail to dedupe when its toolchain version is not using a
  fully-qualified version.
- Fixed an issue where `PROTO_OFFLINE` wouldn't use global binaries when available.

#### ⚙️ Internal

- Updated to proto v0.20.3.

## 1.15.3

#### 🐞 Fixes

- Fixed an issue where interactive/persistent flags weren't always bubbled up the the task runner.

#### ⚙️ Internal

- Updated to proto v0.20.

## 1.15.2

#### 🚀 Updates

- Updated `moon run --interactive` to allow more than 1 target.

#### 🐞 Fixes

- Fixed an issue where "raw" codegen files were sometimes being rendered, and failing with invalid
  syntax.
- Fixed an issue where task dependents for the non-primary targets were being included in the action
  graph.
- Fixed an issue with the project graph that would create duplicate nodes for deeply nested cycles.
- Fixed an issue where a cycle would be created in the action graph for the `SyncProject` action
  type.

## 1.15.1

#### 🚀 Updates

- Based on feedback, we've updated the automatic dependency linking to _not apply_ when the target
  is the root-level project. This should alleviate all unwanted cycles.

#### 🐞 Fixes

- Fixed an issue where Node.js dependency syncing would fail on `build` dependencies, and be over
  zealous with root-level projects.
- Improved detection of Rust `cargo-binstall` package.

## 1.15.0

#### 💥 Breaking

- Tasks that depend (via `deps`) on other tasks from arbitrary projects (the parent project doesn't
  implicitly or explicitly depend on the other project) will now automatically mark that other
  project as a "peer" dependency. For example, "b" becomes a peer dependency for "a".

#### 🎉 Release

- Rewrote the dependency graph from the ground-up:
  - Now known as the action graph.
  - All actions now depend on the `SyncWorkspace` action, instead of this action running
    arbitrarily.
  - Cleaned up dependency chains between actions, greatly reducing the number of nodes in the graph.
  - Renamed `RunTarget` to `RunTask`, including interactive and persistent variants.
- Updated the action graph to process using a topological queue, which executes actions on-demand in
  the thread pool when they are ready (dependencies have been met). Previously, we would sort
  topologically _into batches_, which worked, but resulted in many threads uselessly waiting for an
  action to run, which was blocked waiting for the current batch to complete.
  - For large graphs, this should result in a significant performance improvement.
  - Persistent tasks will still be ran as a batch, but since it's the last operation, it's fine.
- Released a new GitHub action,
  [`moonrepo/setup-toolchain`](https://github.com/marketplace/actions/setup-proto-and-moon-toolchains),
  that replaces both `setup-moon-action` and `setup-proto`.

#### 🚀 Updates

- Added a `moon action-graph` command.
- Added a `--dependents` argument to `moon action-graph`.
- Added the ability to skip non-`RunTask` actions using environment variables.
- Deprecated the `moon dep-graph` command.

#### 🐞 Fixes

- Fixed an issue where task dependents (via `moon ci` or `moon run --dependents`) wouldn't always
  locate all downstream tasks.

#### ⚙️ Internal

- Added in-memory caching to project graph file system lookup operations.
- Updated Rust to v1.72.

## 1.14.5

#### 🐞 Fixes

- Temporarily fixed the "A dependency cycle has been detected for (unknown)" issue.
- Fixed an issue where Git hooks were not created properly when using Git worktrees.
- Fixed a panic when attempting to clean/parse a JSON string.

## 1.14.4

#### 🐞 Fixes

- Fixed an issue where `moon docker scaffold` was too greedy and would copy files it shouldn't.
- Fixed some `PATH` inconsistencies when executing npm/pnpm/yarn binaries.
- Fixed codegen `lower_case` and `upper_case` stripping characters.

## 1.14.3

#### 🚀 Updates

- Updated `moon dep-graph` to support a task in closest project, similar to `moon run`.
- Updated to proto v0.19.

#### 🐞 Fixes

- Fixed an issue where local tasks could not extend global tasks using the `extends` setting.

## 1.14.2

#### 🐞 Fixes

- Fixed an issue where non-YAML files in `.moon/tasks` would be parsed as YAML configs.
- Fixed an issue where arguments were not passed to generated Git hooks.
- Fixed an issue where moonbase would fail to sign in in CI.
- Fixed an issue where a root project with aliases, that has self referential tasks, would trigger a
  stack overflow error.

## 1.14.1

#### 🐞 Fixes

- Fixed an issue when using a global version of npm/pnpm/yarn, and the wrong arguments were being
  passed to commands.
- Fixed the "running for 0s" message constantly logging for interactive tasks.

## 1.14.0

#### 🚀 Updates

- Added a `moon run` shorthand, where "run" can be omitted. For example, `moon run app:build` can be
  written as `moon app:build`.
  - This only works for targets that contain a `:`.
- Updated `moon ci` to support running an explicit list of targets, instead of running everything.
- Updated `node.version`, `npm.version`, `pnpm.version`, `yarn.version`, and `rust.version` to
  support partial versions and requirements/ranges like `1.2`, `1`, or `^1.2`.
- Updated `.moon/tasks` to support nested folders, for better organization of task files.

#### ⚙️ Internal

- Improved handling of certificates and proxies.
- Updated to proto v0.18.

## 1.13.5

#### 🐞 Fixes

- Fixed an issue where the `projectName` query would not work correctly.

## 1.13.4

#### ⚙️ Internal

- Updated to proto v0.17.

## 1.13.3

#### 🐞 Fixes

- Fixed an issue where tool globals directory was not being correctly located.
- Fixed a panic when using the `rust` toolchain and attempting to install `bins`.

## 1.13.2

#### 🐞 Fixes

- Fixed an issue where `pnpm` or `yarn` would panic based on configuration combination.

## 1.13.1

#### 🐞 Fixes

- Fixed an issue where tasks depending on arbitrary project tasks would fail to build a partial
  project graph.
- Fixed an issue where task `deps` within global tasks would not apply exclude/rename overrides.
- Fixed an issue with asset/binary files not working correctly with codegen templates.
- Fixed an issue where `moon upgrade` would require a workspace.

## 1.13.0

#### 🚀 Updates

- Added an `allowFailure` task option, allowing tasks to fail without bailing the entire run.
  - Tasks allowed to fail cannot be depended on.
- Added colors to command line `--help` menus.
- Updated `runner.archivableTargets` to support tag scoped targets.
- Updated `moon query tasks` to filter out projects with no tasks.
- Updated `moon query tasks --affected` to filter based on affected task, instead of affected
  project.
- Updated proto integration from v0.12 to v0.16:
  - proto tools are now powered by WASM plugins, which will be downloaded by moon on-demand.
  - Yarn v2+ will now download the requested version, and not downgrade to latest v1.
  - Please report any issues or degradations from this migration.
  - View entire [proto changelog](https://github.com/moonrepo/proto/blob/master/CHANGELOG.md#0160).

#### 🐞 Fixes

- Fixed `moon init` not using the remote default branch when scaffolding.

#### ⚙️ Internal

- Cleaned up moonbase and launchpad implementations.
- Updated Rust to v1.72.

## 1.12.1

#### 🐞 Fixes

- Improved failed task error messages by including information about the failing task.
- Fixed an issue where failed tasks would be cached.
- Fixed an issue where errors lost information while bubbling up the stack.

## 1.12.0

#### 🚀 Updates

- Added [git worktree](https://git-scm.com/docs/git-worktree) support (experimental).
- Added an `interactive` field to task options. This marks tasks as interactive, so they can use
  stdin.
- Added an `extends` field to task configurations. This allows tasks to extend and inherit settings
  from sibling tasks.
- Updated task `env` values to support token functions and variables.
- Updated task `outputs` to support negated globs.
- Will now log a warning to the console if a configuration file uses the `.yaml` extension.

#### 🐞 Fixes

- Fixed an issue where `moon ci` would no run affected targets based on touched files.

#### ⚙️ Internal

- Improved caching and hashing layers.

## 1.11.1

#### 🐞 Fixes

- Fixed an issue where tasks using output globs would not always hydrate from the cache.

## 1.11.0

#### 💥 Breaking

- To support the new project graph, the order and priority in which environment variables are
  resolved has changed. Previously it was task-level > .env file > project-level. Now it's
  task-level > project-level > .env file.

#### 🚀 Updates

- Rewrote the project graph from the ground-up:
  - Lazily built using a multi-pass approach.
  - Graph edges now indicate the type of relationship: development, production, build, peer.
  - Updated `moon project-graph --json` to include the fully expanded graph data.
- Identifiers (project names, file groups, etc) can now be prefixed with underscores (`_`).
- Added Poetry detection support for Python projects.
- Added an `experiments` setting to `.moon/workspace.yml`.
- **Tasks**
  - Environment variables in `command` and `args` are now substituted.
  - Task `deps` can now depend on tag targets (`#tag:task`).
  - Task `env` are now used when substituting values, alongside system-level.
  - Task `outputs` can now use token variables.
- **Codegen**
  - Templates can be used as-is without rendering with [Tera](https://tera.netlify.app) by appending
    a `.raw` extension.
- **Query language**
  - Updated `project` to query both project name AND alias.
  - Added `projectName` for only querying by name.

#### 🐞 Fixes

- Fixed an issue where newer moonbase secret keys would fail to sign in.
- Fixed an issue where `@files` token would not invalidate the project graph cache.
- Fixed an issue where changing `.env` would not invalidate the project graph cache.

#### ⚙️ Internal

- Updated to proto v0.13.
- Updated Rust to v1.71.

## 1.10.1

#### 🐞 Fixes

- Fixed an issue where `.gitignore` patterns weren't always applied correctly.
- Fixed an issue where `git hash-object` commands would fail if moon was setup in a sub-directory.
- Fixed an issue where our "upgrade moon" message would print when requesting JSON output
  (`--json`), resulting in JSON parsing errors.

## 1.10.0

#### 💥 Breaking

> These changes are fixing edge cases that should not have been allowed, but may break existing
> repos. If these changes become troublesome, we'll revert.

- Tasks that configure the same outputs will now error. This change was made as multiple tasks
  writing to the same output location will cause caching and hydration issues.
- If a dependency of a task failed to run or was skipped, then the parent task will now be skipped.

#### 🚀 Updates

- Added support for `MOON_BASE` and `MOON_HEAD` environment variables.
  - Will be used when diffing across branches or commits.
  - Works for both `moon ci` and `moon run`.
- Added `deno.bins` setting to `.moon/toolchain.yml`.
- Added `hasher.ignorePatterns` and `hasher.ignoreMissingPatterns` settings to
  `.moon/workspace.yml`.
- Updated `moon ci` to include a summary of all failed actions.
- Updated `moon run` to compare against the previous commit when running on the default branch and
  using `--remote`.
- Updated `rust.bins` in `.moon/toolchain.yml` to support an object for each bin entry.
  - Can denote bins as CI or local only.
  - Can force install bins.
- Updated the run report to include stderr/stdout for all attempts.

#### 🐞 Fixes

- Fixed an issue where failed target run attempts would not appear in the run report.

#### 📚 Documentation

- Added a new in-depth "Debugging a task" guide.

#### ⚙️ Internal

- Updated to proto v0.12.
- Modernized the code generator and project constraints implementation.
- Renamed runfile to snapshot throughout.

## 1.9.2

#### 🐞 Fixes

- Fixed a panic when attempting to execute an npm package who's binary is purely Bash.

## 1.9.1

#### 🐞 Fixes

- Fixed a panic when parsing the output of `git --version`.

## 1.9.0

#### 🚀 Updates

- Added VCS hooks management support.
  - Added `vcs.hooks` and `vcs.syncHooks` settings to `.moon/workspace.yml`.
  - Added `moon sync hooks` command.
- Added `--clean` and `--force` flags to `moon sync codeowners` command.
- Updated `moon init` to:
  - Detect an applicable VCS provider and set the `vcs.provider` setting.
  - Convert a detected tool version to a fully-qualified semantic version.
- **Node.js**
  - Moved syncing logic from `InstallNodeDeps` action to `SetupNodeTool` action. This includes
    syncing `packageManager`, `engines`, and version files. This should feel more natural.

#### 🐞 Fixes

- Fixed an issue where task hashes would be different between Windows and Unix machines.
  - Root cause is that arguments would use different path separators.
- Fixed an issue where `dev`, `start`, or `serve` tasks would not always be marked as `local`.
- Fixed an issue where inherited tasks parameters (inputs, deps, etc) would sometimes be lost based
  on the merge strategy.
- Fixed an issue with dependency graph cycle detection.

#### ⚙️ Internal

- Updated to proto v0.11.
- Dropped SVN support since it was never finished and doesn't work.
- Improved VCS file handling, caching, and performance.

## 1.8.3

#### 🐞 Fixes

- Fixed an issue where command line arguments were incorrectly escaped in Bash shells.

## 1.8.2

#### 🐞 Fixes

- Updated `CODEOWNERS` to take `.editorconfig` into account when generating.
- Fixed an issue where `git` branch commands would fail on <= v2.22.
- Fixed an issue where disabling moon's cache would not disable proto's cache.

## 1.8.1

#### 🐞 Fixes

- Fixed an issue where failed processes would not bubble up the original error.
- Fixed TypeScript type issues in `@moonrepo/types`.
- Fixed JSON schema issues.

#### ⚙️ Internal

- Updated to proto v0.10.5.

## 1.8.0

#### 🚀 Updates

- Added code owners (`CODEOWNERS`) support.
  - Added `owners` setting to `moon.yml`.
  - Added `codeowners` setting to `.moon/workspace.yml`.
  - Added `moon sync codeowners` command.
- Added `vcs.provider` setting to `.moon/workspace.yml`.
- Added a new action to the graph, `SyncWorkspace`, that'll be used for workspace-level checks.
- Added `workspace.syncing` and `workspace.synced` webhooks.
- Added `MOON_OUTPUT_STYLE` and `MOON_RETRY_COUNT` environment variables.
- **Rust**
  - Improved Cargo workspace root and members detection.

#### ⚙️ Internal

- Deprecated the `moon sync` command, use `moon sync projects` instead.
- Refactored task inputs, outputs, and file groups to be more accurate.
- Updated Rust to v1.70.

## 1.7.3

#### 🐞 Fixes

- Fixed an issue where glob task outputs were not invalidating a previous build.
- Fixed an issue where changing inputs would not mark a task as affected, when a moon workspace is
  nested within a repository.
- Improved handling of ctrl+c signal detection and shutting down processes.

## 1.7.2

#### 🐞 Fixes

- Node.js
  - Fixed an issue where some workers/packages would fail while inheriting parent args.
- Rust
  - Fixed an issue where `cargo generate-lockfile` would run in the wrong directory.

## 1.7.1

#### 🐞 Fixes

- Fixed some configuration bugs.
- Fixed initial bootstrap log messages not being logged.
- Fixed an issue where hydrated caches would be partially written.

## 1.7.0

#### 🚀 Updates

- Rewrote configuration from the ground-up:
  - Strict parsing to bubble up typos, invalid nesting, and more.
  - Recursive merging and validation.
  - And many more improvements.
- Rewrote error handling and rendering.
  - Improved error messages.
  - Added custom error messages for certain situations.
- Added support for npm lockfile v3 format.

#### 🐞 Fixes

- Fixed an issue where colors were not being forced when passing `--color`.
- Fixed an issue where `--log` or `MOON_LOG` would error when running nested `moon` commands.

#### ⚙️ Internal

- Updated to proto v0.10.
- Updated Cargo dependencies.

## 1.6.1

#### 🐞 Fixes

- Fixed poor argument parsing of command line operators like `;`, `&&`, etc.

## 1.6.0

#### 🚀 Updates

- Added support for persistent tasks.
  - Added `persistent` task option to `moon.yml` (is also set via `local`).
  - Persistent tasks _run last_ in the dependency graph.
- Updated long running processes to log a checkpoint indicating it's still running.
- Updated task `platform` detection to only use the platform if the toolchain language is enabled.
- Started migration to a newer/better logging implementation.

#### 🐞 Fixes

- Fixed an issue where a task would panic for missing outputs.

#### ⚙️ Internal

- Reworked file groups to use workspace relative paths, instead of project relative.
- Reworked processes to better handle command line arguments, shells, and piped stdin input.

## 1.5.1

#### 🐞 Fixes

- Fixed an issue where tasks would run in CI even though `runInCI` was false.
- Fixed an issue where npm, pnpm, and yarn shims were not being used from proto.

## 1.5.0

#### 🚀 Updates

- Added Rust tier 2 and 3 language support!
  - Added `rust` as a supported `platform` variant.
  - Added `rust` setting to `.moon/toolchain.yml`.
  - Added `toolchain.rust` setting to `moon.yml`.
  - Added support for `rust` setting in `.prototools`.
  - Updated `moon init` and `moon bin` commands to support Rust.
  - Updated `moon docker scaffold` command to scaffold Cargo files.
  - Updated `moon docker prune` command to delete the `target` directory.

#### 🐞 Fixes

- Fixed an issue where task type was `run` when it should be `test`.

#### ⚙️ Internal

- Reworked `moon init --yes` to not enable all tools, and instead enable based on file detection.
- Cleaned up `moon init` templates. Will no longer scaffold `.moon/tasks.yml`.

## 1.4.0

#### 🚀 Updates

- Added a new target scope for tags, `#tag:task`, which will run a task for all projects with the
  given tag.
- Updated `moon query projects` and `moon query tasks` to support MQL for filtering results.
- Deprecated `node.aliasPackageNames` setting. Aliases will always be loaded now.

#### ⚙️ Internal

- Upgraded to proto v0.8.
- Updated JSON schemas with missing fields.
- Rewrote ID handling for future features.

## 1.3.2

#### 🐞 Fixes

- Fixed an issue where a `pnpm-lock.yaml` with no packages would fail to parse.

## 1.3.1

#### 🐞 Fixes

- Fixed a few issues during input hashing:
  - Would attempt to include deleted files from `git status`, which would log a warning.
  - Would attempt to hash directories for root-level projects, which would log a warning.

#### ⚙️ Internal

- Upgraded to proto v0.7.2.

## 1.3.0

#### 🚀 Updates

- Introducing MQL, a custom query language for running advanced filters on the project graph.
- Added a `--query` option to the `moon run` command, allowing for advanced targeting.
- Updated config loading to be strict and error on unknown fields for non-root fields.

#### 🐞 Fixes

- Fixed an issue where proto would fail to parse `manifest.json`.

#### ⚙️ Internal

- Updated Rust to v1.69.
- Upgraded to proto v0.7.
- Improved accuracy of our globbing utilities, especially around dotfiles/dotfolders.

## 1.2.2

#### 🚀 Updates

- Added `node_modules/.bin/moon` as another lookup location for the `moon` binary when running
  globally.

#### 🐞 Fixes

- Fixed an issue where running tasks were not killed, resulting in background zombie processes.
- Fixed a few version comparisons between Yarn legacy and berry.
- Updated dependency deduping to not run if the manager version is unknown.

## 1.2.1

#### 🐞 Fixes

- Fixed an issue where `$projectAlias` token was not substituting correctly.

## 1.2.0

#### 🚀 Updates

- Added task inheritance based on tags in the form of `.moon/tasks/tag-<name>.yml`.

#### 🐞 Fixes

- Fixed an issue where setting `MOON_COLOR` would fail validation.

#### ⚙️ Internal

- Upgraded to proto v0.6.
- Improvements to file system operations.
- Minor improvements to performance.

## 1.1.1

#### 🐞 Fixes

- Fixed an issue where token function resolving would cause massive performance degradation.

## 1.1.0

#### 🚀 Updates

- Added token variable substitution support for task `command`s.
- Added a `moon task` command, for viewing resolved information about a task.
- Updated `moon run` to be able to run tasks in the closest project based on current working
  directory.
- Updated `noop` tasks to be cacheable, so that they can be used for cache hit early returns.

#### ⚙️ Internal

- Upgraded to proto v0.5.
- Support pnpm v8's new lockfile format.
- Better handling for task's that execute the `moon` binary.

## 1.0.3

#### 🚀 Updates

- Added `hasher.batchSize` to control the number of files to be hashed per batch.
- Updated new version checks to include an optional message.

#### 🐞 Fixes

- Fixed an issue where non-input matching files were being passed to `git hash-object` during the
  inputs collection process. For large projects, you'll see improved performance.
- Fixed an issue where root-level input globs were not matching correctly when `hasher.walkStrategy`
  was "vcs".
- Fixed a deadlock where some concurrent tasks via a parent `noop` task would not start or run in
  parallel.

#### ⚙️ Internal

- Upgraded to proto v0.4.
- Switched to a semaphore for restricting task concurrency.

## 1.0.2

#### 🐞 Fixes

- Fixed an issue where `moon run` or `moon check` would hang when not running in a workspace.
- Fixed an issue where workspace root finding will locate `~/.moon`.

## 1.0.1

#### 🐞 Fixes

- Updated `envFile` to log a warning instead of triggering an error when `.env.` is missing.
- Updated `envFile` to support workspace relative paths when prefixed with `/`.
- Fixed an issue where `.moon/tasks/*.yml` were not scaffolded into `Dockerfile`s.
- Fixed an issue where a CI environment wasn't detected for some CI providers.
- Fixed a project cache issue when running tasks inside and outside of a container.

## 1.0.0

#### 💥 Breaking

- Updated the installer scripts and the `moon upgrade` command to install the `moon` binary to
  `~/.moon/bin`.
- Removed Homebrew support.

#### 🚀 Updates

- Added a `constraints` setting to `.moon/workspace.yml`, allowing for project/dep relationships to
  be enforced.
- Added a `hasher.warnOnMissingInputs` setting to `.moon/workspace.yml`.
- Added a `shell` task option to `moon.yml` that will wrap system tasks in a shell.
- Added a `tags` setting to `moon.yml` for project categorization.
- Added a `--tags` option to the `moon query projects` command.
- Added a `telemetry` setting to `.moon/workspace.yml`.
- Added 5 new token variables: `$projectAlias`, `$date`, `$time`, `$datetime`, and `$timestamp`.
- Updated task `env` and `.env` files to support variable substitution using `${VAR_NAME}` syntax.
- Updated system tasks to now execute within a shell.

#### 🐞 Fixes

- Reworked how task inputs are resolved when empty `[]` is configured, and all `**/*` is inherited.

#### ⚙️ Internal

- Updated the new version check to only run on the `check`, `ci`, `run`, and `sync` commands.
- Will now detect 16 additional CI environments: Agola, AppCenter, Appcircle, Azure, Bamboo,
  Bitrise, Buddy, Cirrus, Codemagic, Heroku, Jenkins, Jenkins X, Netlify, TeamCity, Vela,
  Woodpecker.
- Will now attempt to detect CD environments for more accurate metrics.
- We now create a [cache directory tag](https://bford.info/cachedir) in `.moon/cache`.
