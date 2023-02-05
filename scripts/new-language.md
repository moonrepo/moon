# Implement a new language

Implementing a new language is _a lot_ of work, so this guide outlines all the necessary steps to do
so. Ideally these are done sequentially, as separate PRs, that correlate to our tiered language
support paradigm.

- INIT SCRIPT
- BIN COMMAND
- DOCKER PRUNE

## Tier 1

### Add variant to `ProjectLanguage` enum in `moon_config`

This allows projects to configure their primary language, and is utilized by editor extensions.

```rust
enum ProjectLanguage {
	// ...

	#[strum(serialize = "kotlin")]
	Kotlin,
}
```

- [ ] Updated enum
- [ ] Updated TypeScript types at `packages/types/src/project-config.ts`
- [ ] Verified all `match` callsites handle the new variant
- [ ] Ran `cargo make json-schemas` and updated the JSON schemas

### Add case to `PlatformType::from` in `moon_config`

At this stage, new languages will default to the system platform. Once they reach tier 2, they'll
have their own platform.

```rust
ProjectLanguage::Kotlin => PlatformType::System,
```

- [ ] Updated enum

### Create language crate

Every language will have a "lang" crate that defines metadata about the language, and helper
functions for interacting with its ecosystem (like parsing manifest and lockfiles).

Crate must exist at `crates/<language>/lang`. Feel free to copy an existing language crate and
update the implementation.

- [ ] Implemented `Language` struct
- [ ] Implemented `DependencyManager` struct (if applicable)
- [ ] Implemented `VersionManager` struct (if applicable)

#### Parsing manifests/lockfiles

When reading/writing the manifests/lockfiles, the `config_cache!` macro from the `moon_lang` crate
must be used. This macro handles concurrency (avoids race conditions) and caching.

The Node.js
[`package.json` implementation](https://github.com/moonrepo/moon/blob/master/crates/node/lang/src/package.rs)
can be used as a reference.

- [ ] Implemented manifests (if applicable)
- [ ] Implemented manifests (if applicable)

### Update `moon_platform_detector` crate

moon implements a lot of inference, detection, and automation, to avoid explicit configuration from
the developer. The `moon_platform_detector` handles this, and must be updated to support the new
language.

- [ ] Updated `detect_language_files`
- [ ] Updated `detect_project_language`

> The `detect_task_platform` can be skipped as it's required for tier 2.

### Add tests

Of course this should all be tested.

- [ ] Added fixture to `tests/fixtures/config-inheritance`
- [ ] Added fixture to `tests/fixtures/project-graph/langs`
- [ ] Updated `crates/core/config/tests/task_inheritance_test.rs`
- [ ] Updated `crates/core/project-graph/tests/projects_test.rs`

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

At minimum, create a new language struct at `crates/core/config/src/toolchain/<lang>.rs`. It's ok if
this struct is empty to start. Over time we will add toolchain support, settings to control
automation, and more.

```rust
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct KotlinConfig {
}
```

When ready, add a new field to the `ToolchainConfig` struct.

```rust
pub struct ToolchainConfig {
	// ...

	#[serde(skip_serializing_if = "Option::is_none")]
	#[validate]
	pub kotlin: Option<KotlinConfig>,
}
```

- [ ] Created language struct
- [ ] Updated `ToolchainConfig` struct
- [ ] Ran `cargo make json-schemas` and updated the JSON schemas

### Add variant to `PlatformType` enum in `moon_config`

This enum is the backbone of supporting language specific platforms.

```rust
enum PlatformType {
	// ...

	#[strum(serialize = "kotlin")]
	Kotlin,
}
```

- [ ] Updated enum
- [ ] Updated TypeScript types at `packages/types/src/common.ts`
- [ ] Verified all `match` callsites handle the new variant

### Update `PlatformType::from` case in `moon_config`

Now that the language has a platform, we should explicitly map it.

```rust
ProjectLanguage::Kotlin => PlatformType::Kotlin,
```

- [ ] Updated enum

### Add variant to `Runtime` enum in `moon_platform_runtime`

This determines the language + version of a tool to run within the platform.

```rust
pub enum Runtime {
	// ...
	Kotlin(Version),
}
```

- [ ] Updated enum
- [ ] Updated TypeScript types at `packages/types/src/common.ts`
- [ ] Verified all `match` callsites handle the new variant

### Update `moon_platform_detector` crate

Tasks run against the platform, so we can now attempt to detect this.

- [ ] Updated `detect_task_platform`
