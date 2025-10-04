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
- **Configuration**
  - `moon.yml`
    - Removed the `type` alias. Use `layer` instead.
    - Removed the `platform` setting. Use `toolchain.default` instead.
    - Removed the `tasks.*.platform` setting. Use `tasks.*.toolchain` instead.
    - Removed the `toolchain.*.disabled` setting. Set the toolchain to null/false instead.
  - `.moon/toolchain.yml`
    - Removed the `node.addEnginesConstraint` setting.
- **Projects**
  - The primary `language` is now detected from toolchains, instead of being a hardcoded
    implementation. The result may now differ, as the first toolchain in the list will be used.
  - Toolchains are no longer inferred from the project language. They are now detected based on
    files in the project.
- **Toolchains**
  - Removed the legacy built-in platform system. WASM plugins have replaced their functionality.
    - Some configuration settings may have changed. Refer to the documentation.
- **Webhooks**
  - Removed the `tool.*` events. Use `toolchain.*` events instead.
  - Removed the `runtime` field from `dependencies.*` events. Use `toolchain` field instead.

#### 🚀 Updates

- **WASM API**
  - Added `RegisterToolchainOutput.language` field.
