# Changelog

## Unreleased

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
