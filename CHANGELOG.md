# Changelog

## Unreleased

#### 💥 Breaking

- **WASM API**
  - Renamed `ProjectFragment.alias` to `ProjectFragment.aliases` and changed its type from
    `Option<String>` to `Vec<String>`.

#### 🚀 Updates

- **WASM API**
  - Added `ExtendProjectGraphInput.extension_config` field.
  - Added `ExtendTaskCommandInput.extension_config` field.
  - Added `ExtendTaskScriptInput.extension_config` field.
  - Added `RegisterToolchainOutput.language` field.
  - Added `SyncProjectInput.extension_config` field.
  - Added `SyncWorkspaceInput.extension_config` field.
