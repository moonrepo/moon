# Changelog

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
