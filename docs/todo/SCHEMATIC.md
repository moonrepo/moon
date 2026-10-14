# Schematic: shape enums and partial finalization plan

Changes to schematic that let moon load its last serde-only config types (`Input`, `Output`, and
`TaskCheck`) through partials, and drop the `deserialize_untagged_verbose_error` crates. Written on
2026-10-13 against moon `3.0-config` at `068523b0ff`, using schematic 0.20.10 (`schematic_core`
0.1.9). Schematic references point at the published crate sources (`schematic_core/src/...`,
`schematic/src/...`), so line numbers will drift, and the repository paths may differ.

Estimates are marked as such. Nothing here has been implemented in schematic yet. The moon side was
migrated afterwards, without schematic changes (uncommitted on top of `068523b0ff` when this was
updated), and surfaced steps 5 to 7.

## Summary

Nearly every moon config type loads through its schematic partial, with `#[setting(nested)]` on
every field and variant that references another `Config` type. The exceptions are 3 public enums
that serde parses directly, as they accept both a string and an object:

| Type        | Parsed from                               | Contains                                                                               |
| :---------- | :---------------------------------------- | :------------------------------------------------------------------------------------- |
| `Input`     | `InputShape` via `#[serde(try_from)]`     | `FileInput`, `FileGroupInput`, `GlobInput`, `ProjectInput`, `ManifestDepsInput`        |
| `Output`    | `OutputShape` via `#[serde(try_from)]`    | `FileOutput`, `GlobOutput`                                                             |
| `TaskCheck` | `TaskCheckShape` via `#[serde(try_from)]` | `TaskCheckConditionConfig`, `TaskCheckRequirementConfig`, `TaskCheckFingerprintConfig` |

Each public enum can't derive `Config` itself, because its string form is parsed into one of several
variants ([`Input::from_str`](../../crates/config/src/shapes/input.rs) handles `@token(...)`,
`$VAR`, `$VAR_*`, `file://`, `glob://`, `group://`, and `project://`). Schematic has no hook that
parses a string into a variant: `#[config(before_parse)]` only re-cases unit enums, and
`#[setting(transform)]` keeps the value's type.

The private `*Shape` enums can derive `Config` though, and all 3 now do:

- Each `*Shape` derives `Config` with `#[serde(untagged)]`, and its object variants are
  `#[setting(nested)]`. Only the partials are used, which get schematic's verbose untagged
  `Deserialize`, replacing `DeserializeUntaggedVerboseError` (now removed from moon).
- `Input`, `Output`, and `TaskCheck` deserialize with `#[serde(try_from = "Partial*Shape")]`, and
  convert partials with a local `finalize_partial` helper in
  [shapes/mod.rs](../../crates/config/src/shapes/mod.rs).
- Object forms now get their setting defaults, `#[setting(validate)]` validators, and `required`,
  which the serde-only path skipped.
  [full_struct_serde_test.rs](../../crates/config/tests/full_struct_serde_test.rs) round trips every
  string and object form, as the workspace graph cache reads them back through the same path.

This works without schematic changes, but has rough edges that schematic should fix, plus an
optional larger feature that would remove the shape enums entirely:

| #   | Change                                            | Estimate | Unblocks                                          |
| :-- | :------------------------------------------------ | :------- | :------------------------------------------------ |
| 1   | Use the user-facing name in untagged errors       | < 1 hour | Readable errors for every untagged `Config` enum  |
| 2   | Public API to finalize a standalone partial       | ~2 hours | Removes moon's helper and a `required` footgun    |
| 3   | String parsing hook for untagged enums (optional) | ~3 days  | `Input`, `Output`, `TaskCheck` as `Config` enums  |
| 4   | Generate `into_partial` (optional)                | ~3 days  | Hooks that return full types                      |
| 5   | Check required fields when matching variants      | ~4 hours | Untagged partial enums that don't depend on order |
| 6   | Support a bare `#[setting(default)]`              | ~1 hour  | Dropping serde `default`s that only feed schemas  |
| 7   | Don't emit unused partials of tagged variants     | ~2 hours | Clean TS output without exclude entries           |

## 1. Use the user-facing name in untagged errors

The generated `Deserialize` for untagged partial enums names the partial type in its error:

```
checks[0]: failed to parse as any variant of PartialTaskCheckShape:
- String: failed to deserialize; expected a string, found { "check": "unknown", "script": "test" }
- Tagged: unknown variant `unknown`, expected one of `condition`, `requirement`, `fingerprint`
```

Users never see `Partial*` types, so the name should come from the full type.

**Where:** `schematic_core/src/container.rs`, `impl_partial_type_deserialize` (L557), which formats
the message with `stringify!(#partial_name)` (L593-596).

**Change:** format the message with `self.get_name()` (L126) instead. It already returns the
container's `#[config(rename)]` or `#[serde(rename)]` when set, and the type's ident otherwise, and
is used for schema references, so the error matches the schema name.

- Consider preferring `#[serde(expecting = "...")]` when set (it's already parsed into
  `SerdeContainerArgs::expecting`), as it's the serde-native way to describe what a type accepts.
  For example `#[serde(expecting = "a task check")]` would read "failed to parse as a task check:".
  Only worth doing if the phrasing works for both forms.
- Tests: update the existing untagged enum error tests and any snapshots of the expanded output.

**moon follow-up:** update the expected messages in
[task_config_test.rs](../../crates/config/tests/task_config_test.rs), for example
`failed to parse as any variant of PartialTaskArgs` and `PartialTaskCheckShape`.

## 2. Public API to finalize a standalone partial

`Config::from_partial` (`schematic/src/config/configs.rs`, L117) doesn't apply defaults or validate.
The generated per-field code uses `unwrap_or_default()` (`schematic_core/src/field.rs`, L567-607),
including for `#[setting(required)]` fields, so a missing required value silently becomes the type's
default unless validation ran first.

`ConfigLoader::load_with_context` (`schematic/src/config/loader.rs`, L131-153) already does this
correctly after merging its layers:

1. `partial.finalize(context)`. The generated `finalize` for structs
   (`schematic_core/src/container.rs`, `impl_partial_finalize`, L1227-1300) starts from
   `default_values()`, merges `self` on top, merges `env_values()` (with the `env` feature), and
   then finalizes nested fields. For enums, it finalizes the active variant.
2. `partial.validate(context, true)`, with the `validate` feature.
3. `T::from_partial(partial)`.

Anything that deserializes a partial on its own, like moon's shape enums, has to repeat this.

**Change:** add a provided method to `Config`:

```rust
/// Convert a partial configuration that was loaded on its own, instead of through
/// a [`ConfigLoader`], into a full configuration, by applying its default values
/// and environment variables, and validating it.
fn from_partial_finalized(
    partial: Self::Partial,
    context: &<Self::Partial as PartialConfig>::Context,
) -> Result<Self, ConfigError> {
    let partial = partial.finalize(context)?;

    #[cfg(feature = "validate")]
    partial.validate(context, true)?;

    Ok(Self::from_partial(partial))
}
```

- **Name:** alternatives are `PartialConfig::into_config(self, context)` or
  `Config::finalize_partial`. `from_partial_finalized` sits next to `from_partial` in docs and
  autocompletion, which is where people will look.
- **Env vars:** `finalize` merges `env_values()` for structs. That matches the loader, but a nested
  type loaded standalone could pick up env vars meant for the top-level config if it declares
  `#[setting(env)]`. moon's shapes declare none. Document it, or add a variant that skips env.
- **Error location:** the loader passes validation errors through `map_validator_error` to attach
  the source file. A standalone partial has no source, so `ConfigError::Validator` has an empty
  `location`. Consider an optional label argument, or document it.
- **Docs:** note on `from_partial` that it neither applies defaults nor validates, and that
  `required` settings fall back to their type's default. Point to the new method.
- **Tests:** a missing `required` field is an error, a `#[setting(default = ...)]` value is applied,
  a validator on a nested struct runs, and an untagged enum's partial finalizes its active variant.

**moon follow-up:** replace `crate::shapes::finalize_partial` with the new method. The moon helper
also merges into `default_partial()` before calling `finalize`, which applies the defaults twice.
It's harmless, but goes away with the switch.

## 3. String parsing hook for untagged enums (optional)

This lets `Input`, `Output`, and `TaskCheck` derive `Config` directly, and removes the shape enums
from moon.

**Container argument:** `#[config(parse_str = path)]`, only allowed on untagged enums with data. The
function receives the string and returns the partial:

```rust
fn parse_input(value: &str) -> Result<PartialInput, ParseError>;
```

**Codegen:** in `impl_partial_type_deserialize` (`schematic_core/src/container.rs`, L557), right
after `schematic::internal::buffer_untagged_content(deserializer)?`, strings go to the hook instead
of the variants. Buffered strings are `serde_content::Value::String(Cow<str>)`:

```rust
if let schematic::serde_content::Value::String(value) = &content {
    return #path(value).map_err(D::Error::custom);
}
```

- **Precedence:** strings go only to the hook, and variants never see them. This matches moon, where
  every string goes through `Input::from_str`. The alternative (hook first, then variants) is harder
  to reason about, as string variants like `EnvVar(String)` would match anything.
- **Validation:** panic in `validate_args` when used on a struct, a unit enum, or a non-untagged
  enum.

**Schema override:** the derive always generates `Schematic` for the full and partial types
(`impl_schematic`, L610), which conflicts with the hand-written impls on `Input`, `Output`, and
`TaskCheck`. The derived union would also repeat the `String` type for each string variant. Add a
container `#[config(schema = path)]` that the full type's `build_schema` calls, with the partial's
impl delegating to the full type's.

**Serialization:** the full types keep their hand-written `Serialize` impls, as the `Config` derive
doesn't add one. The partials derive `Serialize` with `untagged`, so a partial parsed from `$VAR`
serializes as `"VAR"`, not `"$VAR"`. Check whether anything serializes partials (template rendering,
caching, or the Pkl renderer). If so, document that `parse_str` partials don't round-trip.

**`TaskCheck` needs more:** it's a string or an object tagged with `check`. Tagged partials use
serde's derived `Deserialize` (container.rs around L503), which has nowhere to put the hook. Either:

- (a) when `parse_str` is set on a tagged enum, generate a manual `Deserialize` that buffers the
  content, sends strings to the hook, and otherwise deserializes into a private copy of the partial
  with the derived tagged impl, or
- (b) reshape `TaskCheck` in moon as untagged, with a nested `Tagged(TaggedTaskCheck)` variant. This
  changes its public shape for every caller.

**moon side:** the hook returns partials, so the parsers have to produce them. For example,
`FileInput::from_uri` would build a `PartialFileInput`. Without step 4, this duplicates the parsing
logic for partials.

## 4. Generate `into_partial` (optional)

A full to partial conversion, mirroring `from_partial` (`schematic_core/src/field.rs`, L567-607, and
`variant.rs`, L511). With it, `parse_str` hooks could return `Self`, and schematic would convert it,
so moon could keep its existing parsers.

- Plain fields become `Some(value)`, nested fields become `Some(value.into_partial())`, and the
  wrapper layers (`Option`, `Vec`, and map values of nested types) map over their items, matching
  the layers that `from_partial` already handles.
- Enums map each variant to its partial variant.
- Adding a required method to `Config` breaks manual impls. Generate it from the derive and put it
  on a separate trait (or a provided method that panics) to keep this semver compatible.
- It's useful beyond moon, for example to seed a `ConfigLoader` layer from a config built in code.

## 5. Check required fields when matching untagged variants

Found while migrating `InputShape`. Partial structs make every field optional, so an untagged
partial enum's variant matches any object without unknown fields (partials still use
`deny_unknown_fields`), and `required` is only checked when the result is finalized. Matching
therefore depends on variant order in a way the full types didn't.

`ProjectInput` (`project`, `filter`, `group`) and `FileGroupInput` (`group`, `format`) both have
`group`. With `Project` first, `{ group: 'sources' }` became a `PartialProjectInput`, then failed
with "project: this setting is required". moon now lists `FileGroup` before `Project`, with a
comment, and the `input_shape::parse_object::file_group` test fails if the order is swapped back.

**Change:** in the per-variant attempts (`schematic_core/src/variant.rs`,
`impl_partial_type_deserialize`, L569), treat a nested variant whose partial is missing a
`#[setting(required)]` field as a failed attempt, with an error like "missing required field
`project`", and move on to the next variant. This needs a generated helper on partials that reports
missing required fields without running validators, for example
`fn missing_required(&self) -> Vec<&'static str>`.

- Only top-level required fields of the variant matter for matching. Nested required fields are
  still reported by validation.
- An object that only matches through required fields, like `{}`, fails every variant and gets the
  combined error, instead of the first variant's "this setting is required".

## 6. Support a bare `#[setting(default)]`

`Field::is_optional` (`schematic_core/src/field.rs`) treats a field as optional when it has a
`#[setting(default = ...)]`, or a serde `default`. `#[setting(default)]` without a value isn't
accepted, as the argument is an `Option<Expr>`.

Partials already default missing non-`Option` fields to their type's `Default`, so the only
remaining use of serde `default` on moon's shape structs is marking the setting optional in schemas.
Without it, `FileGroupInput.format`, `ProjectInput.filter`, `ManifestDepsInput.deps`, and
`TaskCheckFingerprintConfig.hash` become required in the full TS interfaces, which the `Input` and
`TaskCheck` schemas reference, so `{ group: 'sources' }` would stop type checking in TS. moon keeps
`#[serde(default)]` on those 4 fields, with a "Marks the setting as optional in schemas" comment.

**Change:** accept a bare `#[setting(default)]` that means the type's `Default`, and count it in
`is_optional`. moon can then replace the 4 serde `default`s, so that serde attributes on `Config`
types only describe serialization.

## 7. Don't emit unused partials of tagged variants

Found after adding `#[setting(nested)]` to the private `TaggedTaskCheck` (internally tagged with
`check`). The TS renderer now emits `PartialTaskCheckConditionConfig`,
`PartialTaskCheckRequirementConfig`, and `PartialTaskCheckFingerprintConfig` (each with a `check?`
tag field) into both `tasks-config.ts` and `project-config.ts`, even though nothing references them.
`TaskCheck` still renders as `string | TaggedTaskCheck` over the full types.

moon excludes them in [typescript_types.rs](../../crates/config-schema/src/typescript_types.rs) for
now.

**Change:** find where the partials of nested variants get registered with the `SchemaGenerator`
when only the full enum is referenced, and only add them when the partial enum itself is reachable.
Confirm with an internally tagged enum fixture whose full type is referenced but partial isn't.

## Not planned

- **`dead_code` on partial-only types:** a `#[derive(Config)]` enum that only exists for its partial
  warns that its fields are never read, so moon's shapes need `#[allow(dead_code)]`. The derive
  can't annotate the user's item, and generating code that reads the fields only to silence the lint
  isn't worth it.
- **Verbose errors for the full types:** `config_untagged_enum!` types now use serde's plain
  `#[serde(untagged)]` for their full types, so code that deserializes them directly gets serde's
  opaque "data did not match any variant" error. In practice that's toolchain plugins that parse
  `BinEntry` with `parse_toolchain_config` (plain `serde_json`). That's a moon or PDK concern, not
  schematic's.

## Suggested order

1. Steps 1 and 2 in schematic, and release them (they're additive).
2. Steps 5 to 7 in schematic. Step 5 changes which variant matches, so it may be worth a minor
   version.
3. In moon (the migration itself is done), bump schematic and then:
   - Replace `finalize_partial` with step 2's method.
   - Update the expected messages in
     [task_config_test.rs](../../crates/config/tests/task_config_test.rs) from
     `failed to parse as any variant of Partial*` to the user-facing names (step 1).
   - Optionally restore the "most complex first" variant order in `InputShape` (step 5).
   - Replace the 4 schema-only serde `default`s with `#[setting(default)]` (step 6).
   - Remove the `PartialTaskCheck*Config` exclude entries from
     [typescript_types.rs](../../crates/config-schema/src/typescript_types.rs) (step 7).
4. Decide on steps 3 and 4 only if other schematic users have the same "string or object" shapes.
   For moon alone, the shape enums are enough.

## Verification

- **Schematic:** unit tests for each change, as listed in each step.
- **moon against a local schematic:** uncomment the path dependency in
  [Cargo.toml](../../Cargo.toml) (L143, `# schematic = { path = "../schematic/crates/schematic" }`),
  then:
  - `cargo nextest run -p moon_config -p moon_task -p moon_task_builder`, with
    `CARGO_TARGET_DIR=~/.cargo/shared-target`, `MOON_TEST=true`, and `STARBASE_TEST=true`.
  - [full_struct_serde_test.rs](../../crates/config/tests/full_struct_serde_test.rs), as the
    workspace graph cache deserializes `Task.inputs`, `outputs`, and `checks` through the same
    `try_from` path.
  - `just gen`, to confirm the JSON schemas, TS types, and Pkl modules don't change for steps 1
    and 2.
- **Errors:** spot check an invalid input, output, and check in YAML, and confirm that every
  variant's error is listed under the user-facing type name.
