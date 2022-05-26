# Changelog

## Unreleased

#### 🚀 Updates

- Hashing will now ignore files that matched a pattern found in the root `.gitignore`.
- Added support for the `MOON_LOG` environment variable.
- Updated the JSON schemas to use the new package manager versions.

#### 🐞 Fixes

- Fixed an issue with the `.moon/workspace.yml` template being generating with invalid whitespace
  during `moon init`.
