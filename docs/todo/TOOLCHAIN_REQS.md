# Toolchain requirements: a typed model

A plan for replacing the `define_requirements` API (`requires` plus 2 boolean flags) with 3 explicit
relationships: per-action requirements, dependencies root delegation, and toolchain membership.
Written on 2026-10-15 against `3.0-ag-new` at `20de2c6867`, right after the action graph fixes in
that branch. Line references are as of that commit, and will drift. Plugin references are to
`moonrepo/plugins` at `f707d200`.

Nothing here has been implemented. Claims marked **unverified** come from reading the code, and need
a test before they're relied on.

## Summary

`DefineRequirementsOutput.requires` is one untyped list of toolchain ids, and moon reads it in 4
places with 4 different meanings: which toolchains a project or task uses, the order of toolchain
setup, the order of environment setup, and (since `20de2c6867`) which environments are setup in a
toolchain's dependencies root. The 2 flags that scope it apply to the whole list, default to
`false`, and are ignored by 3 of the 4 readers.

Because requirements are between toolchains, while ordering is between actions, valid plugin setups
look cyclic (Python and uv require each other), so the action graph carries path-local cycle sets
and edges that are skipped when they would cycle, instead of reporting real cycles.

The proposal:

1. **Requirements name the action they apply to.** Each requirement is one edge, from one of the
   toolchain's own actions to the required toolchain's setup. Cycle workarounds go away, and a real
   cycle becomes an error.
2. **Delegation is declared by the root's owner.** `locate_dependencies_root` returns the other
   toolchains that the located root belongs to, and moon sets up their environments there.
3. **Membership is its own, well-defined relation**, computed as a proper closure, with reverse
   inference (a `node` task gets `javascript`) derived from delegation instead of by scanning every
   plugin's requirements.

Old plugins keep working: the legacy fields are mapped onto the new model inside moon.

## The current model

### The API

[`DefineRequirementsOutput`](../../crates/pdk-api/src/toolchain/tier2.rs#L26):

```rust
pub struct DefineRequirementsOutput {
    pub requires: Vec<String>,       // toolchain ids, `unstable_` prefixed when unstable
    pub for_setup_environment: bool, // default false
    pub for_setup_toolchain: bool,   // default false
}
```

`DefineRequirementsInput.toolchain_config` is the workspace config merged with the project's, when
there's a project.

### Who returns what

| Plugin (`moonrepo/plugins`)                                               | `requires`                                                                 | Flags                   | Locates a root | `setup_environment` |
| :------------------------------------------------------------------------ | :------------------------------------------------------------------------- | :---------------------- | :------------- | :------------------ |
| `javascript` (`toolchains/javascript/src/tier2.rs:201`)                   | `node` (unless the package manager is standalone), and the package manager | `for_setup_toolchain`   | yes (`:321`)   | yes                 |
| `node-depman` (npm, pnpm, yarn; `toolchains/node-depman/src/tier2.rs:10`) | `node` (unless nub)                                                        | `for_setup_toolchain`   | no             | yes (yarn plugins)  |
| `python` (`toolchains/python/src/tier2.rs:17`)                            | `unstable_<packageManager>`                                                | `for_setup_environment` | yes (`:167`)   | yes                 |
| `python-uv`, `python-pip`, `python-poetry` (`src/tier2.rs:7`)             | `unstable_python`                                                          | `for_setup_toolchain`   | no             | no                  |
| `deno`                                                                    | none                                                                       |                         | no             | yes (global bins)   |

`javascript`'s locate picks its lockfile names from its `packageManager` setting, so at the time it
locates a root it already knows which package manager (and runtime) the root belongs to.

### The 4 readers

1. **Project membership.**
   [`detect_project_toolchain_from_usage`](../../crates/toolchain-plugin/src/toolchain_registry_actions.rs#L116)
   (called from [project_builder.rs:219](../../crates/project-builder/src/project_builder.rs#L219))
   adds every requirement of every detected toolchain to the project. Flags are ignored.
2. **Task membership.**
   [`expand_task_usage`](../../crates/toolchain-plugin/src/toolchain_registry_actions.rs#L215)
   (called from [tasks_builder.rs:965](../../crates/task-builder/src/tasks_builder.rs#L965)) adds a
   toolchain and its requirements when the task uses the toolchain, **or any of its requirements**.
   That's reverse inference: a task that uses `node` gets `javascript` and its package manager.
   Flags are ignored.
3. **Toolchain setup ordering.**
   [`internal_setup_toolchain`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1527)
   links `SetupToolchain(self) → SetupToolchain(required)` when `for_setup_toolchain` is set
   ([loop](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1561)).
4. **Environment setup ordering and delegation.**
   [`internal_setup_environment`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1362)
   ([loop](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1397)):
   - with `for_setup_environment`, links `SetupEnvironment(self) → SetupToolchain(required)`;
   - **regardless of the flags**, links `SetupEnvironment(self) → SetupEnvironment(required)`, in
     this toolchain's root, via
     [`link_environment_edges`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1948),
     which skips an edge that would cycle. This is what makes `node-depman`'s and `deno`'s
     `setup_environment` run at all, as they never get a root of their own.

[docs/ACTION_GRAPH.md](../ACTION_GRAPH.md) describes the resulting edges in detail.

## Problems

1. **One list, 4 meanings.** Membership, 2 kinds of ordering, and delegation can't be expressed
   independently. A plugin can't require a toolchain for ordering without also adding it to every
   project that uses the plugin, or delegate a root without also ordering against it.
2. **The flags are list-wide.** A plugin can't say "`node` before my toolchain setup, but `uv`
   before my environment setup".
3. **Silent loss for old plugins.** The flags were added later and default to `false`, so a plugin
   built before them returns requirements that no ordering reader applies.
4. **The missing-toolchain policy depends on a flag.** A required toolchain without a spec errors
   with `MissingToolchainRequirement` on the flagged paths, but is skipped by the delegation path,
   because a plugin can't say whether a requirement is optional.
5. **Toolchain-level requirements make action-level DAGs look cyclic.** Python requires uv for its
   environment, and uv requires Python for its toolchain: a cycle between toolchains, but between
   actions it's `SetupEnvironment(python) → SetupToolchain(uv) → SetupToolchain(python)`, which is
   acyclic. The builder compensates with path-local cycle sets (on toolchain setup, environment
   setup, and project sync), and skip-on-cycle environment edges, so real plugin cycles are never
   reported, and the result can depend on insertion order.
6. **A latent `WouldCycle` (unverified).** `internal_setup_toolchain` re-links on reuse
   (`insert_node_if_missing!`) with a fresh path set per top-level call. If 2 toolchains require
   each other with `for_setup_toolchain`, setting up A first links `A → B` (B's recursion skips A),
   and a later direct request for B links `B → A`, which fails the build. No real plugin pair does
   this today.
7. **Delegation through `requires` points the wrong way for children.** `python-poetry` requires
   `python`, so if poetry ever located a root, Python's environment would be setup in it. Today
   that's only avoided because children don't locate roots.
8. **Placeholder nodes.** `SetupToolchain(javascript)` exists only so that `for_setup_toolchain`
   edges have something to hang off (javascript has no tool), and `SetupEnvironment` nodes are
   created for toolchains without `setup_environment` just to carry requirement edges. Their actions
   skip themselves at runtime.
9. **Membership is ill-defined.** `expand_task_usage` is a single pass over the plugin list, so
   reverse inference is one hop deep, and whether a toolchain is added can depend on iteration
   order. It also evaluates requirements against the **workspace** config, while project detection
   uses the project's merged config.
10. **Shared roots evaluate with workspace config.** A shared root's environment has no project, so
    its requirements come from the workspace config, even when projects in that root override a
    requirement-affecting setting (like `javascript.packageManager`).

## Proposal

### 1. Requirements name the action they apply to

```rust
pub struct DefineRequirementsOutput {
    pub requirements: Vec<ToolchainRequirement>,

    // Legacy, mapped by moon (see "Migration")
    pub requires: Vec<String>,
    pub for_setup_environment: bool,
    pub for_setup_toolchain: bool,
}

pub struct ToolchainRequirement {
    /// The required toolchain. Stable ids are accepted for unstable
    /// toolchains, as moon resolves the `unstable_` alias itself.
    pub toolchain: Id,

    /// The earliest of this toolchain's actions that needs the required
    /// toolchain to be setup. Later actions wait on it transitively.
    pub before: ToolchainStage,

    /// When the required toolchain isn't configured, skip the requirement
    /// instead of failing.
    pub optional: bool,
}

pub enum ToolchainStage {
    SetupToolchain,
    SetupEnvironment,
    InstallDependencies,
}
```

Each requirement is exactly one `Required` edge, from `<before>(self)` to
`SetupToolchain(required)`, attached to whichever node actually represents that action for that
toolchain (the install slot resolution in ACTION_GRAPH.md decides which nodes exist).

- **Cycles become errors.** Recursion is guarded by a path set keyed by `(toolchain, stage)`, not by
  toolchain, and a hit is an error naming the chain, instead of a silent skip. Python and uv no
  longer collide, as their requirements are on different stages.
- **No placeholder nodes.** `javascript` returns
  `{ node, before: InstallDependencies }, { pnpm, before: InstallDependencies }`, and moon links
  them from `InstallDependencies(javascript)`, so `SetupToolchain(javascript)` and empty
  `SetupEnvironment` nodes are no longer needed for ordering.
- **Missing toolchains** error unless `optional`.
- **Readiness** is always "the required toolchain is setup". No current plugin needs another
  toolchain's environment to be ready (that's what delegation is for), so a `ready` level is left
  out until something does.

### 2. The root's owner declares delegation

```rust
pub struct LocateDependenciesRootOutput {
    pub members: Option<Vec<String>>,
    pub root: Option<VirtualPath>,

    /// Other toolchains whose dependencies root this also is, like the runtime
    /// and package manager for JavaScript. `None` means the plugin doesn't
    /// declare delegation, and moon falls back to the legacy behavior.
    pub toolchains: Option<Vec<Id>>,
}
```

It flows into [`DependenciesWorkspace`](../../crates/toolchain/src/shapes.rs#L13) (converted in
[`ToolchainPlugin::locate_dependencies_root`](../../crates/toolchain-plugin/src/toolchain_plugin.rs#L566)).
For each listed toolchain with a spec, moon inserts `SetupEnvironment(child, root)` with the owner's
project association, and orders it before the owner's environment (as today). Children only get
environments through an owner, so delegation can't point the wrong way, or cycle.

`javascript` would return `[node, <packageManager>]` (or `[bun]`, `[deno]`); `python` would return
`[unstable_<packageManager>]`.

**Alternative:** declare it statically on the child, in `register_toolchain` (like
`dependencies_root_owner: "javascript"`). Simpler, but it can't vary per root, and the owner is the
one that already decides the root.

### 3. Membership is its own relation

- **Forward:** a project or task that uses a toolchain also uses its requirements, computed as a
  transitive closure (a fixpoint), not a single pass.
- **Reverse** (a `node` task gets `javascript`): derived from delegation (child → owner), not from
  scanning every plugin's requirements. If it turns out reverse inference isn't wanted, drop it.
- **Config:** project and task expansion should evaluate with the same config (the project's merged
  config).

Delegation is per root, while membership is per project and task, so reverse inference needs a
static form of the owner relation. If that's needed, it's an argument for also adding the
`register_toolchain` alternative from section 2, used only for membership.

## Migration

moon normalizes every `define_requirements` result into `Vec<ToolchainRequirement>` internally, so
the builder only ever sees the new model:

| Legacy output                           | Mapped to                                                                                          |
| :-------------------------------------- | :------------------------------------------------------------------------------------------------- |
| `requires` with `for_setup_toolchain`   | `before: SetupToolchain` per id                                                                    |
| `requires` with `for_setup_environment` | `before: SetupEnvironment` per id (both flags: both entries)                                       |
| `requires` with neither flag            | `before: SetupToolchain` per id, the behavior from before the flags existed, which fixes problem 3 |
| `requirements` set                      | used as is, legacy fields ignored                                                                  |

Legacy `optional` is `false`, except where the current code skips (the delegation path), which goes
away with section 2.

Delegation: when a plugin's locate returns `toolchains: None`, keep the current `requires`-based
environment delegation as a fallback, so third-party plugins built against the current PDK keep
working. Remove the fallback in a later major.

## Phases

1. **Verify.** Write a test for problem 6 (mutual `for_setup_toolchain` requirements, setting up A
   then B). The `tc-tier2-reqs` and `tc-tier3-reqs` fixtures can be extended with an opt-in config
   key, like `tc-tier2-setup-env`'s `testRequirements`.
2. **Internal model.** Add `ToolchainRequirement` and `ToolchainStage` to the PDK API, normalize
   legacy output in `ToolchainPlugin::define_requirements`, and rework the builder to stage edges:
   requirement edges move to the node for their stage, path sets are keyed by `(toolchain, stage)`
   and error on a hit, and the placeholder node rules go. Behavior for current plugins should be
   unchanged, other than errors for real cycles.
3. **Delegation.** Add `LocateDependenciesRootOutput.toolchains`, carry it through
   `DependenciesWorkspace`, and insert child environments from it. Keep the `requires` fallback for
   `None`.
4. **Membership.** Rework `detect_project_toolchain_from_usage` and `expand_task_usage` into a
   shared closure, with consistent config, and decide on reverse inference.
5. **Plugins.** Move `javascript`, `python`, `node-depman`, and `python-uv`/`pip`/`poetry` in
   `moonrepo/plugins` to `requirements` and locate `toolchains`.
6. **Later major.** Remove the legacy fields and the delegation fallback.

## What changes in this repo

- `crates/pdk-api/src/toolchain/tier2.rs`: the new types and fields, and docs on the legacy ones.
- `crates/toolchain-plugin`: normalization in `define_requirements`, `toolchains` in
  `locate_dependencies_root`, and the membership closure in `toolchain_registry_actions.rs`.
- `crates/toolchain/src/shapes.rs`: `DependenciesWorkspace.toolchains`.
- `crates/action-graph/src/action_graph_builder.rs`: `internal_setup_toolchain`,
  `internal_setup_environment`, `internal_install_dependencies`, and `link_environment_edges`
  (likely removed). The unit tests `keeps_toolchain_requirements_path_local_across_siblings`
  ([L2345](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L2345))
  and the `setup_env` requirement tests in `crates/action-graph/tests/action_graph_builder_test.rs`
  (`*_required_toolchain_environments*`,
  `doesnt_link_required_toolchain_environments_that_would_cycle`) encode current behavior and will
  need revisiting.
- `wasm/tc-tier2-setup-env` (keys `testEnvRequirements`, `testRequirements`), `wasm/tc-tier2-reqs`,
  `wasm/tc-tier3-reqs`, `wasm/tc-tier3-setup-env`: test plugins that return requirements.
- `docs/ACTION_GRAPH.md`: the `SetupToolchain` and `SetupEnvironment` sections, cycle guards, and
  Notes (requirement-only toolchains).
- `CHANGELOG.md`: PDK changes under Plugins, and graph changes under Action graph.

## Open questions

1. **Where delegation is declared:** the owner's locate output (recommended) or the child's
   registration, or both (locate for the graph, registration for membership)?
2. **Child environment order:** before the owner's environment (today), or only before the owner's
   install, so that they can run in parallel with the owner's environment?
3. **Reverse membership:** is "a `node` task gets `javascript`" intended? If so, from what?
4. **Stages:** is `InstallDependencies` needed as a requirement stage, or does `javascript` only
   need its install to wait on `node` and the package manager through task membership?
5. **PDK compatibility:** map the legacy fields (above), or break them in 3.0, given that
   third-party toolchain plugins are rare?
6. **Default for `optional`:** `false` (fail on a missing toolchain, like the flagged paths today)
   is the proposal.
