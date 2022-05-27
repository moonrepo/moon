# Changelog

## Unreleased

#### 🚀 Updates

- Added support for macOS silicon (`aarch64-apple-darwin`).
- Added support for the `MOON_LOG` environment variable.
- Updated the JSON schemas to use the new package manager versions.
- Hashing will now ignore files that matched a pattern found in the root `.gitignore`.

#### 🐞 Fixes

- Fixed an issue with the `.moon/workspace.yml` template being generating with invalid whitespace
  during `moon init`.
