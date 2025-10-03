# Changelog

## Unreleased

#### 💥 Breaking

- CLI
  - Removed scaffolding a toolchain from the `moon init` command. Use the `moon toolchain add`
    command instead.
  - Removed the `moon node` command and sub-commands.
  - Removed the `moon migrate from-package-json` command.
  - Removed the `moon migrate from-turborepo` command. Use the `migrate-turborepo` extension
    instead.
- Toolchains
  - Removed the legacy built-in platform system. WASM plugins have replaced their functionality.
- Webhooks
  - Removed the `tool.*` events. Use `toolchain.*` events instead.
  - Removed the `runtime` field from `dependencies.*` events. Use `toolchain` field instead.
