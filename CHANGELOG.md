# Changelog

## Unreleased

#### 💥 Breaking

- **CLI**
  - Removed scaffolding a toolchain from the `moon init` command. Use the `moon toolchain add`
    command instead.
  - Removed the `moon node` command and sub-commands.
  - Removed the `moon migrate from-package-json` command.
  - Removed the `moon migrate from-turborepo` command. Use the `migrate-turborepo` extension
    instead.
  - Removed the `--platform` flag from all applicable commands. Use the `--toolchain` flag instead.
- **Configuration**
  - Removed the `$projectType` token. Use `$projectLayer` instead.
  - Removed the `$taskPlatform` token. Use `$taskToolchain` instead.
  - `moon.yml`
    - Removed the `type` alias. Use `layer` instead.
    - Removed the `platform` setting. Use `toolchain.default` instead.
    - Removed the `tasks.*.platform` setting. Use `tasks.*.toolchain` instead.
    - Removed the `toolchain.*.disabled` setting. Set the toolchain to null/false instead.
  - `.moon/toolchain.yml`
    - Removed the `node.addEnginesConstraint` setting.
  - `.moon/workspace.yml`
    - Removed the `constraints.enforceProjectTypeRelationships` alias. Use
      `enforceLayerRelationships` instead.
- **Projects**
  - The primary `language` is now detected from toolchains, instead of being a hardcoded
    implementation. The result may now differ, as the first toolchain in the list will be used.
    Additionally, languages that don't have a toolchain yet, like PHP or Ruby, will not be detected
    and must be explicitly configured.
- **Toolchains**
  - Removed the legacy built-in platform system. WASM plugins have replaced their functionality.
    - Some configuration settings may have changed. Refer to the documentation.
- **Webhooks**
  - Removed the `tool.*` events. Use `toolchain.*` events instead.
  - Removed the `runtime` field from `dependencies.*` events. Use `toolchain` field instead.
- **Other**
  - Removed the `projectType` and `taskPlatform` query properties. Use `projectLayer` and
    `taskToolchain` instead.

#### 🚀 Updates

- **WASM API**
  - Renamed `ProjectFragment.alias` to `ProjectFragment.aliases` and changed its type from
    `Option<String>` to `Vec<String>`.

#### 🚀 Updates

- **WASM API**
  - Added a `load_extension_config_by_id` host function.
  - Added `load_extension_config`, `parse_extension_config` and `parse_extension_config_schema`
    utility functions.
  - Added `ExtendProjectGraphInput.extension_config` field.
  - Added `ExtendTaskCommandInput.extension_config` field.
  - Added `ExtendTaskScriptInput.extension_config` field.
  - Added `RegisterToolchainOutput.language` field.
  - Added `SyncProjectInput.extension_config` field.
  - Added `SyncWorkspaceInput.extension_config` field.
