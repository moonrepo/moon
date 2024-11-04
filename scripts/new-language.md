# Implement a new language

Implementing a new language is _a lot_ of work, so this guide outlines all the necessary steps to do
so. Ideally these are done sequentially, as separate PRs, that correlate to our tiered language
support paradigm.

## Tier 1

### Add variant to `LanguageType` enum in `moon_config`

This allows projects to configure their primary language, and is utilized by editor extensions.

```rust
enum LanguageType {
  // ...
  Kotlin,
}
```

- [ ] Updated enum
- [ ] Verified all `match` callsites handle the new variant
- [ ] Ran `just schemas` and updated the JSON schemas/types

### Create language crate

Every language will have a "lang" crate that defines metadata about the language, and helper
functions for interacting with its ecosystem (like parsing manifest and lockfiles).

Crate must exist at `legacy/<language>/lang`. Feel free to copy an existing language crate and
update the implementation.

#### Parsing manifests/lockfiles

When reading/writing the manifests/lockfiles, the `config_cache_model!` macro from the `moon_lang`
crate must be used. This macro handles concurrency (avoids race conditions) and caching.

The Node.js
[`package.json` implementation](https://github.com/moonrepo/moon/blob/master/legacy/node/lang/src/package_json.rs)
can be used as a reference.

- [ ] Implemented manifests (if applicable)
- [ ] Implemented lockfiles (if applicable)
  - [ ] `load_lockfile_dependencies`

### Update `moon_toolchain` crate

moon implements a lot of inference, detection, and automation, to avoid explicit configuration from
the developer. The `moon_toolchain` handles this, and must be updated to support the new language.

- [ ] Updated `languages.rs`
- [ ] Updated `detect_language_files`
- [ ] Updated `detect_project_language`

> The `detect_task_platform` and `detect_project_platform` can be skipped as it's required for
> tier 2.

### Add tests

Of course this should all be tested.

- [ ] Added fixture to `crates/config/tests/__fixtures__/inheritance`
- [ ] Added fixture to `crates/project-builder/tests/__fixtures__/langs`
- [ ] Added fixture to `crates/task-builder/tests/__fixtures__/builder/platforms`
- [ ] Updated `crates/config/tests/inherited_tasks_config_test.rs`

### Create a pull request

Once everything is good, create a pull request and include it in the next release. Ideally tiers are
released separately!

## Tier 2

### Add toolchain configuration to `moon_config`

In moon, platforms _are not_ enabled unless the configuration field in `toolchain.yml` is defined,
even if it's an empty object. For example, this would enable the Kotlin platform:

```yaml
# .moon/toolchain.yml
kotlin: {}
```

At minimum, create a new language struct at `crates/config/src/toolchain/<lang>.rs`. It's ok if this
struct is empty to start. Over time we will add toolchain support, settings to control automation,
and more.

```rust
#[derive(Config)]
pub struct KotlinConfig {
}
```

When ready, add a new field to the `ToolchainConfig` struct.

```rust
pub struct ToolchainConfig {
  // ...

  #[setting(nested)]
  pub kotlin: Option<KotlinConfig>,
}
```

- [ ] Created language struct
- [ ] Created config template file
- [ ] Updated `ToolchainConfig` struct
- [ ] Ran `just schemas`
- [ ] Add `.prototools` support in `crates/config/src/toolchain_config.rs`
- [ ] Add tests to `crates/config/tests/toolchain_config_test.rs`

### Add variant to `PlatformType` enum in `moon_config`

This enum is the backbone of supporting language specific platforms.

```rust
enum PlatformType {
  // ...
  Kotlin,
}
```

- [ ] Updated enum
- [ ] Verified all `match` callsites handle the new variant
- [ ] Ran `just schemas`

### Update `moon_toolchain` crate

Tasks run against the platform, so we can now attempt to detect this.

- [ ] Updated `detect_task_platform`
- [ ] Updated `detect_project_platform`

### Create tool crate

Every language will have a "tool" crate that implements the moon `Tool` trait (and eventually the
proto `Tool` trait). This trait defines a handful of methods for how to install and execute the
tool.

```rust
#[derive(Debug)]
pub struct KotlinTool {
    pub config: KotlinConfig,
    pub global: bool,
}
```

This is required _even when not using_ the toolchain, as we fallback to a global binary available on
`PATH`.

Crate must exist at `legacy/<language>/tool`. Feel free to copy an existing tool crate and update
the implementation.

- [ ] Implemented `Tool` trait
- [ ] Implemented `PackageManager` trait (when applicable)
- [ ] Handled `global` binary

### Create platform crate

Every language will have a "platform" crate that implements the `Platform` trait. This trait defines
a ton of methods for interacting with the language's ecosystem, and how that interoperates with
moon.

Crate must exist at `legacy/<language>/platform`. Feel free to copy an existing platform crate and
update the implementation.

- [ ] Implemented `Platform` trait
- [ ] Implemented manifest hashing
- [ ] Implemented target hashing
- [ ] Implemented action handlers
- [ ] Implemented project graph bridge

### Update docs

At this point we should start updating docs, primarily these sections:

- Any configs
- Language handbook

### Create a pull request

Once everything is good, create a pull request and include it in the next release. Ideally tiers are
released separately!

## Tier 3

### Add tool to proto

Tier 3 requires a tool to be added to proto: https://github.com/moonrepo/proto

### Support `version` in `moon_config` for language

The toolchain requires an explicit version to function correctly, so the config struct pertaining to
the language must have a `version` field. This field must be `Option<String>`, which allows for the
toolchain to be disabled.

```rust
#[derive(Config)]
pub struct KotlinConfig {
  // ...
  pub version: Option<UnresolvedVersionSpec>,
}
```

Furthermore, when applicable, also add version support from `.prototools`.

```toml
kotlin = "1.2.3"
```

- [ ] Updated config struct: `crates/config/src/toolchain/<lang>.rs`
- [ ] Supported proto version in `crates/config/src/toolchain_config.rs`
- [ ] Ran `just schemas`

### Integrate proto tool into moon tool crate

Once the proto crate and configuration is ready, we can update the moon specific tool with proto's.

```rust
#[derive(Debug)]
pub struct KotlinTool {
    pub config: KotlinConfig,
    pub global: bool,
    pub tool: KotlinLanguage,
}
```

- [ ] Inherited `version` from applicable config
- [ ] Implemented `setup` and `teardown` methods
- [ ] Handled global binary

### Integrate moon tool into platform crate

When the moon tool has been integrated with proto's, we can update the platform crate to use the
`ToolManager` instance, and implement all necessary methods.

Refer to the Node.js implementation for examples (it can mostly be copied).

- [ ] Enabled `is_toolchain_enabled` method
- [ ] Updated `get_runtime_from_config` with `version` field
- [ ] Updated `setup_toolchain`, `setup_tool`, and `teardown_toolchain` methods
- [ ] Updated `create_run_target_command` to use the tool instance

### Support project-level config overrides

Different projects may have different version requirements, so we need to support this through
project-level toolchain overrides.

- [ ] Updated `crates/config/src/project/overrides_config.rs`
- [ ] Updated `get_runtime_from_config` in platform crate

### Integrate `--profile` option

When applicable, the run target command should handle the `--profile` option and the CPU/heap
variants.

### Update `bin` command

The `moon bin` command uses a hard-coded tool list, and is not based on the `PlatformType` or
`LanguageType` enums. Because of this, tools will need to be handled manually.

- [ ] Updated `crates/app/src/commands/bin.rs`

### Update `docker prune` and `docker scaffold` commands

By default these commands will do their best to handle languages/platforms, but each tool is
different and may require custom logic.

- [ ] Updated `crates/app/src/commands/docker/scaffold.rs` (mainly `scaffold_workspace` function)
- [ ] Updated `crates/app/src/commands/docker/prune.rs` (added another prune function)

### Add runner tests

The biggest thing to test besides the tool and platform, is that running tasks for the language work
correctly. There are many cases to test for: error handling, exit codes, stdout, stderr, etc. Refer
to Node.js for a complete example.

- [ ] Added `crates/cli/tests/run_<lang>_test.rs`

### Update docs

At this point we should start updating docs, primarily these sections:

- Any configs
- Language handbook

- [ ] `website/docs/__partials__/`

### Create a pull request

Once everything is good, create a pull request and include it in the next release. Ideally tiers are
released separately!