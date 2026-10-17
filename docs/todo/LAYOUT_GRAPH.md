# Layout graph: where projects live in dependencies roots

A design for a precomputed model of the repository layout: which dependencies root each project uses
for each of its toolchains, with which version and config, and what lives in each root. Written on
2026-10-16 against `3.0-layout-graph` at `65588e19aa` (the head of `develop-3.0`), and updated on
2026-10-17 with Miles's answers (see [Resolved questions](#resolved-questions)). Line references are
as of that commit, and will drift.

Nothing here has been implemented. Estimates are marked as such. Claims marked **unverified** come
from reading the code, and need a test before they're relied on.

This composes with [TOOLCHAIN_REQS.md](./TOOLCHAIN_REQS.md): delegation (its section 2) becomes a
field of a root in this model, and membership (its section 3) is an input to it. Caching composes
with [WS_GRAPH_NODE_CACHE.md](./WS_GRAPH_NODE_CACHE.md).

## Summary

Many consumers need to answer the same question: _for project P and toolchain T, which dependencies
root is used, what is P's role in it, and which version and config apply?_ No shared answer exists.
The action graph, docker prune, the task hasher, the command environment and the install action each
derive it themselves. They use 3 copies of the version logic, 4 different membership checks, and 2
config policies, and they call `locate_dependencies_root` once per project, even when hundreds of
projects end up in the same root.

The proposal is a **layout graph**, built from the workspace graph and the toolchain registry, that
owns that answer. The consumers read from it instead of locating roots themselves.

- **Shape:** a bipartite graph of projects and toolchain dependencies roots. The interesting data
  (role, version, merged config) belongs to a _(project, toolchain)_ pair, so it lives on the edge,
  not on either node. It's stored as arenas with typed indices, not as nested structs or a petgraph.
- **Config and versions:** included, at 2 levels.
  - Each root has one _install_ spec and config: what its environment setup and install run with.
    For a workspace root, that's the workspace's version. For a single-package root (the project's
    own root), it's that project's version, including an override.
  - Each binding has the project's own _run_ spec and merged config.
  - When a member's version differs from its root's, the layout records the difference. The root's
    actions still wait only on the root's version, and the member's tasks get an extra edge to their
    own. Neither depends on which projects are in the run.
- **Building:** `moon run` builds the layout lazily, only for the (project, toolchain) pairs its
  plan needs, between the select and assemble stages of the action graph builder. Docker, queries
  and validation build it in full. **Fields of a root depend only on the root, never on which
  members were seen**, so lazy and full builds agree.
- **Where:** a separate `moon_layout_graph` crate, exposed through the session like the action graph
  builder.

## Today

### Who derives what

| Consumer                                                                                                 | Toolchains                                                           | Version                                                                | Config                                         | Membership check                                       |
| :------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------- | :--------------------------------------------------------------------- | :--------------------------------------------- | :----------------------------------------------------- |
| Action graph [install slot](../../crates/action-graph/src/builder/assemble.rs#L683)                      | raw `task.toolchains` (EnterTask), raw `project.toolchains` (docker) | [`get_project_spec`][spec]                                             | merged to locate; workspace for a shared root  | `in_dependencies_workspace`                            |
| Docker prune [`gather_workspaces`](../../crates/app/src/commands/docker/prune.rs#L46)                    | `get_enabled_toolchains()`                                           | none                                                                   | merged to locate; workspace for `prune_docker` | focused: own locate; unfocused: focused roots' globs   |
| Task hasher [`apply_toolchain_dependencies`](../../crates/task-hasher/src/task_hashing.rs#L247)          | `get_enabled_toolchains_for_task`                                    | [own copy](../../crates/task-hasher/src/task_hashing.rs#L196)          | merged, twice per toolchain                    | **none**: hashes the lockfile of whatever root located |
| Command env [`inherit_from_toolchains`](../../crates/process-augment/src/augmented_command.rs#L333)      | `get_toolchain_ids` (task, project, or all plugins)                  | [own copy](../../crates/process-augment/src/augmented_command.rs#L338) | n/a                                            | n/a (plugins re-locate inside `extend_command`)        |
| Install action [`hash_manifest_contents`](../../crates/actions/src/actions/install_dependencies.rs#L315) | the node's                                                           | none                                                                   | workspace, merged if the node has a project    | walks `node.members` globs on disk                     |

[spec]: ../../crates/action-graph/src/builder/mod.rs#L68

Supporting definitions:

- [`DependenciesWorkspace { root, members }`](../../crates/toolchain/src/shapes.rs#L3) and
  `DependenciesWorkspaceRole { WorkspaceRoot, WorkspaceMember, PackageRoot }`, matched in
  [`in_dependencies_workspace`](../../crates/toolchain-plugin/src/toolchain_plugin.rs#L199) (`None`
  when the path is neither the root nor a member).
- [`create_merged_config`](../../crates/toolchain-plugin/src/toolchain_registry.rs#L98) is a JSON
  merge of the workspace config and the project's `toolchains.<id>` object, not memoized.
- Tasks can't override a toolchain's version or config. A task's
  [`toolchains`](../../crates/config/src/task_config.rs#L366) setting is a list of ids only.

### Inconsistencies

1. **3 copies of the version logic:** `get_project_spec`, the task hasher, and
   `inherit_from_toolchains`. They agree today, but nothing keeps them in sync.
2. **4 membership checks:**
   - The action graph and prune's focused pass call `in_dependencies_workspace`.
   - The hasher calls nothing, so a non-member (issue 2653's shape) hashes the workspace lockfile.
   - Prune's unfocused pass never locates. It attributes a project to a focused project's root by
     glob alone, even when the project's own locate would find a nested root.
   - The install action re-derives membership by walking member globs on disk. That walk is
     _correct_ there, as it must also catch packages that aren't moon projects.
3. **Toolchain lists:** raw `project.toolchains` / `task.toolchains` in the action graph and
   [`get_base_image`](../../crates/app/src/commands/docker/file.rs#L292), `get_enabled_toolchains*`
   everywhere else, and all plugins in docker scaffold's root and metadata passes.
4. **Config policy:** `define_requirements` gets the merged config in the project builder and the
   action graph, but the workspace config in
   [`expand_task_usage`](../../crates/task-builder/src/tasks_builder.rs#L965). `prune_docker` always
   gets the workspace config.
5. **Cost:** the plugin call cache is keyed by the whole input (starting dir plus merged config), so
   N projects in one root cost N `locate_dependencies_root` calls. The action graph's memo
   (`install_deps_indexes`,
   [assemble.rs:113](../../crates/action-graph/src/builder/assemble.rs#L113)) dedupes per project,
   not per root.

### Versions in a shared root

- `SetupToolchain` is keyed by spec, so each version override gets its own node.
- `SetupEnvironment` and `InstallDependencies` aren't keyed by version. The install slot passes the
  **requesting project's** spec down to the environment
  ([assemble.rs:736](../../crates/action-graph/src/builder/assemble.rs#L736)), which links that
  version's `SetupToolchain`, even though the node itself has no project. So the environment links
  the `SetupToolchain` of every version requested in this run by projects in the root
  ([assemble.rs:585](../../crates/action-graph/src/builder/assemble.rs#L585)).
- At runtime, a shared root's environment and install run with the **workspace** versions. The node
  has no project, so `exec_plugin_command` → `inherit_from_plugins(None, None)` →
  `inherit_from_toolchains` only sees `.moon/toolchains.yml`, and sets `PROTO_<ID>_VERSION` and the
  tool paths from it. A `PackageRoot` install runs with its project's versions.

So the graph and the runtime disagree about which version a shared root uses. The graph sets up
whatever the requesting members asked for. The command then runs with the workspace version, which
**nothing in the graph sets up** when only overriding members are in the run (unverified: traced
through the code, not tested). For example, take a Cargo workspace on Rust 1.85, where crate `a`
overrides it to 1.80. `moon run a:build` sets up 1.80 only, then runs the root's commands with
`PROTO_RUST_VERSION=1.85`.

This only affects the owner toolchain's own version. A delegated toolchain (like `node` under
`javascript`) is resolved with no project inside the environment, so the workspace version is always
linked.

## Concepts

- **Moon workspace:** the repository root. There is exactly one.
- **Toolchain dependencies root:** a directory located by a toolchain's `locate_dependencies_root`,
  identified by **(owner toolchain, root path)**. Under the existing ontology it's one of 2 things:
  - a _workspace_: a root plus member globs, where the root may also be a package;
  - a _package_: a single non-workspace root, with no members.

  There is no nested-member kind. 2 toolchains can share a directory (JavaScript and Cargo at `.`)
  and still be 2 roots.

- **Owner project:** the project whose root is the dependencies root, if any.
- **Binding:** a _(project, toolchain)_ pair, holding:
  - the project's spec and merged config;
  - a **placement**: in a root with a role, outside a root it located (not a member), or nowhere (no
    tier 2, or no root found).
- **Delegated toolchains:** toolchains whose environment is set up in another toolchain's root, like
  `node` and `pnpm` in a `javascript` root.

Mapping the first draft (`MoonWorkspace` and `ToolchainWorkspace`) onto this:

| Draft                                                 | Here                                                                                              |
| :---------------------------------------------------- | :------------------------------------------------------------------------------------------------ |
| `MoonWorkspace.projects` (not associated)             | Derived: projects whose bindings are all unplaced. Association is per toolchain, not per project. |
| `ToolchainWorkspace`                                  | `ToolchainDependenciesRoot`                                                                       |
| `ToolchainWorkspace.members` / `root`                 | `ToolchainDependenciesRoot.kind` / `root`                                                         |
| `ToolchainWorkspace.projects`                         | `ToolchainDependenciesRoot.members` (project ids), plus each project's binding                    |
| `primary_toolchain`                                   | `ToolchainDependenciesRoot.toolchain` (the owner)                                                 |
| `setup_environments` / `setup_toolchains` (`Vec<Id>`) | `ToolchainDependenciesRoot.delegates`: each with a **spec** and config, not 2 parallel id lists   |

**Naming:** "toolchain workspace config" already means the workspace-level toolchain config
([`toolchain_workspace_config`](../../crates/pdk-api/src/common.rs#L327)), so the root type is
`ToolchainDependenciesRoot` rather than `ToolchainWorkspace`. Docker prune's private
`ToolchainDependenciesWorkspace` ([prune.rs:16](../../crates/app/src/commands/docker/prune.rs#L16))
is replaced by it.

## Shape: a graph, not nested structs

Nested structs (`MoonWorkspace → ToolchainWorkspace → projects`) don't fit, for these reasons:

1. **Many to many.** A polyglot project is in a JavaScript root and a Cargo root. The root project
   at `.` is often in every root-level workspace.
2. **The data is on the edge.** Role, run spec and merged config belong to the (project, toolchain)
   pair. A `Vec<Arc<Project>>` under a root has nowhere to put them.
3. **Both directions are hot:**
   - project → roots: the install slot, the hasher, the command env;
   - root → members: install `packages`, docker prune, manifest hashing, affected.

   Nesting makes one direction a scan.

4. **Directory nesting isn't membership.** A package with its own lockfile inside a JavaScript
   workspace's directory is a separate root, not a child of the outer one.

The graph also needs no graph algorithms. Its depth is at most 2, it has no cycles, and ordering
belongs to the action graph. So it's stored as **arenas with typed indices plus 2 maps**, not a
petgraph. When DOT or JSON output is needed, a petgraph is built on demand through
`moon_graph_utils`.

Projects are referenced **by id, not by `Arc<Project>`**. That keeps the layout decoupled from the
workspace graph, and path-free and serializable for a later persisted cache.

## Data model

```rust
/// Identifies a toolchain dependencies root within a layout graph.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RootIndex(u32);

pub struct LayoutGraph {
    roots: Vec<ToolchainDependenciesRoot>,
    root_keys: FxHashMap<(Id, WorkspaceRelativePathBuf), RootIndex>,

    /// Per project, sorted by toolchain id.
    bindings: FxHashMap<Id, Vec<Binding>>,

    /// Located from the workspace root without a project (`moon setup`).
    workspace_bindings: Vec<Binding>,

    /// Whether every project was placed, so that `members` is complete.
    complete: bool,
}

pub struct ToolchainDependenciesRoot {
    /// The toolchain that located this root.
    pub toolchain: Id,
    pub root: WorkspaceRelativePathBuf,
    pub kind: RootKind,
    /// The project whose root is this root.
    pub owner_project: Option<Id>,

    /// What this root's environment setup and install run with.
    pub spec: ToolchainSpec,
    pub config: Arc<serde_json::Value>,

    /// Toolchains whose environment is setup in this root (TOOLCHAIN_REQS section 2).
    pub delegates: Vec<DelegatedToolchain>,

    pub files: RootFiles,
    pub capabilities: RootCapabilities,

    /// Projects placed in this root, sorted. Only complete for a full build.
    pub members: Vec<Id>,
}

pub enum RootKind {
    /// Member globs, relative to the root. Kept even though members are resolved
    /// to projects, as a dependencies workspace may have packages that aren't projects.
    Workspace { members: Vec<String> },
    Package,
}

pub struct DelegatedToolchain {
    pub spec: ToolchainSpec,
    pub config: Arc<serde_json::Value>,
}

/// Probed at the root from the toolchain's registered names.
pub struct RootFiles {
    /// Every registered lockfile name that exists, as more than one can.
    pub lockfiles: Vec<WorkspaceRelativePathBuf>,
    pub manifests: Vec<WorkspaceRelativePathBuf>,
    pub vendor_dir: Option<WorkspaceRelativePathBuf>,
}

pub struct RootCapabilities {
    pub setup_environment: bool,
    pub install_dependencies: bool,
}

pub struct Binding {
    pub toolchain: Id,
    /// The project's own spec, which its tasks run with.
    pub spec: ToolchainSpec,
    /// Workspace config merged with the project's.
    pub config: Arc<serde_json::Value>,
    pub placement: Placement,
}

pub enum Placement {
    Root { index: RootIndex, role: DependenciesWorkspaceRole },
    /// Located a root, but isn't a member of it (issue 2653's shape). No root is
    /// created for it, and it gets only toolchain setup, as today.
    Outside { root: WorkspaceRelativePathBuf },
    /// The toolchain doesn't support tier 2, or no root was located.
    None,
}
```

Queries:

- `binding(project, toolchain)` and `bindings(project)`;
- `root(index)` and `root_for(project, toolchain)`;
- `roots_of(toolchain)`, `members(index)`, and `iter_roots()`;
- `delegate_root(project, toolchain)`: the root in which a delegated toolchain lives for this
  project (see [reverse inference](#relationship-to-toolchain_reqsmd)).

### Root fields depend only on the root

This rule lets a lazy build and a full build agree.

- `spec` and `config` come from the workspace config, plus the owner project's config under the
  rules below.
- They never come from the members seen so far.
- Only `members` (and the diagnostics below) need a full build.

The owner project is found by path, through
[`ProjectGraph::sources`](../../crates/project-graph/src/project_graph.rs#L279) or an inverse map
built once.

## Config and versions

### Rules

| Root kind                | `spec` / `config` (install level)                        | Today                                         |
| :----------------------- | :------------------------------------------------------- | :-------------------------------------------- |
| Package (owner = P)      | P's spec (including an override), workspace + P's config | Same, via `project_id` on the node            |
| Workspace, no owner      | Workspace spec and config                                | Same                                          |
| Workspace, owner project | Workspace spec and config                                | Same, even when the root project overrides it |

The last row is decided as workspace-only for now. Merging the owner project's config into a
workspace root is more correct, as the root project's `moon.yml` is where someone would put a
root-level override. But under partial graph loading the owner may not be among the requested
projects, so it would have to be loaded too. That's a [follow-up](#follow-ups). The `Package` row
has no such cost, as the requesting project is the owner.

With these rules, every root's spec and config are known without seeing any member, so they satisfy
[the root-only rule](#root-fields-depend-only-on-the-root) for free.

Bindings always carry the project's own spec and merged config, computed once. That removes the
repeated `create_merged_config` calls and the 2 extra copies of the version logic.

### Divergence

A member whose spec for the owner toolchain or a delegate differs from the root's spec is
**divergent**. That's legitimate: tasks in a member can run on Node 20 against dependencies
installed with Node 22. The layout records it, and the action graph links it as follows (decided on
2026-10-17).

**Example:**

- A Cargo workspace at `cargo_root` uses Rust 1.90 (`.moon/toolchains.yml`), and has 3 crates: `a`,
  `b` and `c`.
- `a` overrides the version to 1.99 in its `moon.yml`.
- `moon run :build` runs all 3 crates.

| #   | Action                                  | Version                             |
| :-- | :-------------------------------------- | :---------------------------------- |
| 1   | `InstallDependencies(rust, cargo_root)` | workspace (1.90)                    |
| 2   | `SetupEnvironment(rust, cargo_root)`    | workspace (1.90)                    |
| 3   | `SetupToolchain(rust 1.90)`             | workspace, for the root's actions   |
| 4   | `SetupToolchain(rust 1.99)`             | `a`'s, only used to run `a`'s tasks |

| Edge          | Why                                                        |
| :------------ | :--------------------------------------------------------- |
| 1 → 2         | the install slot, as today                                 |
| 2 → 3         | the root's actions wait on the version they run with, only |
| `a:build` → 1 | every member waits on the install                          |
| `b:build` → 1 |                                                            |
| `c:build` → 1 |                                                            |
| `a:build` → 4 | an extra edge, as `a` diverges                             |

`SyncWorkspace` and `SetupProto` edges are unchanged, and left out of the tables.

Today the same run links 2 → 3 and 2 → 4, so the install waits on 1.99, which it doesn't use.
`moon run a:build` alone links 2 → 4 only, so 3 doesn't exist, while the root's commands still run
with `PROTO_RUST_VERSION=1.90` (unverified, see
[Versions in a shared root](#versions-in-a-shared-root)).

**Principle (decided on 2026-10-17):** an overridden version is only required to run that project's
tasks. It's never involved in a root's actions (`SetupEnvironment`, `InstallDependencies`, and the
toolchain setup they wait on), with one exception:

- **Package root** (the dependencies root is the project's own root, a single package and not a
  workspace): the project's overridden version is used for toolchain setup, environment setup and
  install, as today. The root has exactly one project, so no other project depends on its state.
- **Workspace root:** always the workspace's configured version, even when the root directory is
  also a project.

**Rejected:** running the root's actions with `a`'s version when `a` runs alone
(`moon run a:build`), so that only 1.99 is installed. The root's state would then depend on which
project ran last:

- `node_modules` with native addons, or a Python venv, would be built with whatever version ran
  last;
- both actions' fingerprints hash the node and the toolchain config, which includes `version`, so
  alternating `moon run a:build` and `moon run b:build` would reinstall every time;
- environment setup usually skips an environment that already exists (like Python's venv check), so
  a venv created on `a`'s version would stay for every member;
- the nodes would have to carry a version, and the command env would need a version override.

Installing both versions costs one extra toolchain download, which proto caches after the first
time.

**Rules:**

1. The root's actions link only `SetupToolchain(root spec)`, whichever projects are in the run. When
   the toolchain has no `setup_environment`, `InstallDependencies` links it directly. When it has no
   `install_dependencies`, the slot is `SetupEnvironment`, with the same edge.
2. A shared root's chain is resolved **without a project**: the root's spec, and requirement specs
   from the workspace config. A project's spec is only used for that project's own edge. Today the
   install slot calls `setup_toolchain(spec, Some(project))` first
   ([assemble.rs:702](../../crates/action-graph/src/builder/assemble.rs#L702)). A `PackageRoot`
   chain is resolved with its owner project, as today.
3. A task links `SetupToolchain(own spec)` **only when its binding diverges** from its root. A
   member on the root's spec reaches it through the install, so `b` and `c` get no extra edge.
4. Delegated toolchains follow the same rules. For example, take a `javascript` root on Node 22,
   whose member `a` overrides Node to 20:
   - the root's actions reach `SetupToolchain(node 22)` only;
   - `a`'s tasks link `SetupToolchain(node 20)`. They already do today, through the task's own
     `node` toolchain.

   Rule 2 is what removes Node 20 from the root's chain. Today `a`'s request reaches the shared
   `SetupToolchain(javascript)` placeholder with `a`'s project, and its `for_setup_toolchain`
   requirements then link `SetupToolchain(node 20)`. That node is keyed by spec only, so the root's
   install waits on Node 20 through it (unverified).

5. An `Outside` or unplaced binding has no root, so its task links `SetupToolchain(own spec)` as the
   install slot, as today.

Install-affecting config (like `javascript.packageManager` in a member) can also diverge, but moon
can't tell which keys matter. That would need a plugin hook, so it's out of scope here.

## Toolchain metadata per root

| Field            | Source                                                                                       | Consumers                                                                    |
| :--------------- | :------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------- |
| `kind`           | `locate_dependencies_root`                                                                   | install `packages`, manifest hashing, docker prune                           |
| `owner_project`  | path lookup                                                                                  | config and spec rules, node identity                                         |
| `spec`, `config` | rules above                                                                                  | `SetupEnvironment`, `InstallDependencies`, `prune_docker` inputs             |
| `delegates`      | TOOLCHAIN_REQS's `LocateDependenciesRootOutput.toolchains`; fallback: the owner's `requires` | environment delegation                                                       |
| `files`          | probe `lock_file_names`, `manifest_file_names` and `vendor_dir_name` at the root             | task hasher, docker scaffold, `first_install`, daemon and cache invalidation |
| `capabilities`   | `has_func`                                                                                   | install slot rows 5–6 without plugin calls                                   |
| `members`        | bindings                                                                                     | docker, install `packages`, queries                                          |

Files are probed, with no PDK change. Several package managers' lockfiles can exist at once
(`package-lock.json` and `pnpm-lock.yaml`), so `lockfiles` keeps all of them. That matches the task
hasher today, which parses every registered lockfile name that exists.

## Building it

### Where it lives

- A new crate, `crates/layout-graph` (`moon_layout_graph`), with `LayoutGraphBuilder` and the frozen
  `LayoutGraph`. It depends on `moon_workspace_graph` and `moon_toolchain_plugin`, and the action
  graph depends on it.
- It's hooked up through the session, like the action graph builder
  ([session.rs:109](../../crates/app/src/session.rs#L109)). `Session::build_layout_graph()` returns
  a `LayoutGraphBuilder` from the app context and the workspace graph. Docker, queries and
  validation call it for a full build.
- `build_action_graph_with_options` passes a `LayoutGraphBuilder` to `ActionGraphBuilder::new`. The
  action graph builds it lazily, and returns the frozen `Arc<LayoutGraph>` in `BuildOutput`, so the
  pipeline reads the same placements.
- It's not part of `WorkspaceGraph` or its cache, so commands that never touch toolchains never pay
  for locating.

### Inputs

- The workspace graph: projects, plus each task's enabled toolchains.
- The toolchain registry, and `.moon/toolchains.yml`.

The pairs to place are each project's enabled toolchains, **plus its tasks' enabled toolchains**, in
both lazy and full builds. Lazy builds get them from the task plan. Full builds load every project's
tasks to get them, so that a full layout agrees with `moon run` about `members`. Tasks can't
override a toolchain's version or config, so a binding is still per (project, toolchain): tasks only
add pairs.

### Steps

1. For each (project, toolchain) pair:
   - resolve the spec (the one shared helper) and the merged config;
   - for tier 2 toolchains, locate the root and compute the role.
2. Group placed bindings into roots by (toolchain, root), and find each root's owner project.
3. Per root:
   - resolve the spec, config and delegates, under the rules above;
   - probe its files, and read its capabilities.
4. Locate from the workspace root without a project, for `moon setup`.
5. In a full build only, run diagnostics:
   - **outside:** a project that located a root but isn't a member (info);
   - **shadowed:** a project placed in an inner root, while an enclosing root of the same toolchain
     matches it by glob. The outer install probably also installs it, so this is a warning.
   - **divergent:** a member's spec differs from its root's spec. This is info only and never warns,
     as it's often intentional.

### Lazy and full

Runs build lazily.

- **Lazy (`moon run`, `moon ci`, `moon exec`):**
  - The action graph builder's select stage already knows every task instance and project before
    assembly. A new stage between select and assemble collects the (project, toolchain) pairs from
    the `TaskPlan` and places them.
  - Locates run concurrently across toolchains. Today they're awaited one at a time inside `replay`.
  - The result is frozen before assembly, and install slot rows 3–4 become lookups.
  - Requirement calls remain, until TOOLCHAIN_REQS's model makes them prefetchable too.
- **Full (docker, `moon setup`, queries, validation):** every project.

Cost estimate (unverified): one WASM call is tens to a few hundred µs, plus WASI file walks. At 1000
projects × 2 tier-2 toolchains, a full build is ~100–400 ms with no persisted cache. That's fine for
docker and queries, but not for every `moon run`.

### Later: fewer calls, and persistence

- **Batch locate:** an optional `locate_dependencies_roots` PDK function that takes many starting
  dirs. It costs one WASM call per toolchain, instead of one per project.
- **Persisted layout:** a binding isn't a pure function of its project, as locate reads ancestor
  directories and the root manifest (for member globs). So its fingerprint is:
  - existence probes of the registered lockfile and manifest names in each ancestor, from the
    project root up to the workspace root;
  - the content of the located root's manifest;
  - the merged config.

  This fits WS_GRAPH_NODE_CACHE's layer 2, stored in the same keyed CAS store. Measure the lazy
  build first.

- **Daemon:** watch root lockfiles and manifests, and mark their roots dirty.

## Consumers after the change

- **Action graph:**
  - The install slot reads placements.
  - `NodeKey::SetupEnvironment` and `NodeKey::InstallDependencies` become `{ toolchain, root }`.
    `members` and `project_id` stay on the node payloads, but stop being identity, as they're
    functions of the root.
  - So one root can no longer have 2 identities. Today a single-package repo whose root is a project
    gets `(tc, "", None)` from `setup_environment_root` and `(tc, "", Some(root))` from a task. That
    only happens across different graphs today, so it's latent, not a race (unverified).
- **Docker prune and scaffold:**
  - Both use a full layout.
  - Unfocused projects get their own placements, instead of a focused project's globs.
  - Placements include task-only toolchains, so a member reached only through a task's toolchain now
    counts. For example, a `bash` project matched by a JavaScript workspace's member globs, whose
    task runs `node`. Prune only uses project toolchains today.
  - `packages` is drawn from root members.
  - The prune fixture needs a tier 2 toolchain (it uses `tc-tier1` today, so locate is untested).
- **Task hasher:** reads the binding. It hashes `root.files.lockfiles`, and decides explicitly
  whether an `Outside` project hashes the root's lockfiles (it does, implicitly, today).
- **Command env:** versions come from the binding's spec. Plugins still compute deps-root bin dirs
  in `extend_command`. Passing the root in `ExtendCommandInput` would let them stop re-locating, as
  a later PDK change.
- **Install action:** keeps walking member globs for manifest hashing, as non-project packages
  matter. It can take `packages` from members when installs are scoped (docker today).
- **Introspection (done, ahead of phase 5):** `moon layout-graph [id]` (`moon lg`) serves the
  visualizer, with projects (and the workspace) linking to roots, labeled with their role, sent in
  the visualizer's labeled format with a node `type` (`layout-workspace`, `layout-root`,
  `layout-project`) that it styles by. `--info` prints every root with its fields and members, the
  bindings that aren't in a root, and the workspace bindings. `--dot` renders the same graph, and
  `--json` serializes the `LayoutGraph` itself. Diagnostics can go in `moon toolchain info` or
  `moon check` later.

## Relationship to TOOLCHAIN_REQS.md

- **Delegation (its section 2)** becomes `ToolchainDependenciesRoot.delegates`. This is the layout's
  job: which toolchains live in a root, and with which specs.
- **Ordering (its section 1)** stays in the action graph, which combines placements with
  requirements. The layout holds no edges between toolchains.
- **Reverse inference (its open question 3)** gets a concrete source. A `node` task in project P
  installs into the `javascript` root that delegates `node` among P's bindings, so
  `delegate_root(P, node)` answers it. That happens at action graph time, without adding
  `javascript` to `task.toolchains` at graph build time.
- **Membership (its section 3)** is an input. The layout consumes project and task toolchain sets,
  and doesn't compute them.

## Phases

0. **Prep, independent (done):**
   - one `get_toolchain_spec(toolchains_config, id, project_config)` helper in `moon_toolchain`
     (`crates/toolchain/src/spec.rs`), used by the builder, the hasher and
     `inherit_from_toolchains`;
   - `moon docker file`'s base image uses the project's enabled toolchains. The action graph keeps
     its raw lists, as the helper already returns `None` for a disabled toolchain.
1. **Model and builder, no consumers (done):**
   - `crates/layout-graph` (`moon_layout_graph`): `LayoutGraph` and `LayoutGraphBuilder`, with
     `place(pairs)` (lazy), `place_project`, `place_all` (full, including task-only toolchains), and
     `place_workspace` (from the workspace root, for `moon setup`). Locates run concurrently, and
     bindings are inserted in request order, so root indices are deterministic.
   - `Session::build_layout_graph()`.
   - Tests in `crates/layout-graph/tests`, on a `layout` fixture: members, a workspace root, a
     package root, outside, nested (also matched by the outer globs), no tier 2, disabled, and lazy
     and full builds agreeing. `tc-tier2` gained an opt-in `testLocateRoots` config, which locates
     like a real plugin: the closest `tc.lock` from the project, with member globs in that root's
     `tc.root.cfg`. Without it, `tc-tier2` still decides the root from `context.working_dir`.
   - Not yet: `delegates` (phase 6), and diagnostics (phase 5).
   - `owner_project` is found through the workspace graph's sources, so a partial graph that doesn't
     include the owner leaves it `None`. Only a package root's spec and config depend on the owner.
     Its owner is the project placed at it, except from `place_workspace`.
   - For phase 2: from `place_workspace`, a single-package root at the workspace root uses the root
     project's spec and config when one exists. Today `moon setup` uses the workspace's
     (`setup_environment_root` passes no project), so phase 2 must choose.

2. **Action graph:**
   - add the placement stage between select and assemble;
   - turn the install slot into lookups, and switch to root-keyed node identity;
   - apply the [divergence rules](#divergence), so the root's actions link the root's spec only, and
     divergent tasks link their own;
   - first, add tests for the Rust example and the Node delegate case, so the snapshot changes are
     visible;
   - aim for unchanged snapshots otherwise, other than deduped identities;
   - benchmark against the generated workspaces in ACTION_GRAPH_BUILDER.md.
3. **Pipeline consumers:** the hasher and the command env read the frozen layout from `BuildOutput`.
4. **Docker:** prune and scaffold on a full layout, with a tier 2 prune fixture.
5. **Diagnostics:** outside, shadowed and divergent, also shown by `moon layout-graph`.
6. **TOOLCHAIN_REQS phases 3–4** on top: `delegates`, and reverse inference via the layout.
7. **Perf:** batch locate, persisted layout, daemon watching.

## Follow-ups

- **Owner config for a workspace root:** merge the root project's config into a workspace root's
  spec and config, and load the owner project when a partial graph doesn't include it.

## Resolved questions

Decided on 2026-10-17. No questions are open.

1. **Storage:** arenas with typed indices.
2. **Owner config for a workspace root:** workspace only, for now (see [Follow-ups](#follow-ups)).
3. **Build timing:** lazy per run, full for docker, queries and validation.
4. **Where it lives:** a separate crate, hooked up through the session like the action graph
   builder.
5. **Root files:** probed from the registered names, with no PDK change.
6. **Naming:** `ToolchainDependenciesRoot`.
7. **Overridden versions:** only required to run the project's tasks, never involved in a root's
   actions. In a shared root, the root's actions use the workspace version, and a divergent member's
   tasks get an extra edge to their own version, even when the member runs alone. A package root
   (the project's own root, not a workspace) uses its project's overridden version for setup and
   install, as today (see [Divergence](#divergence)).
8. **Divergence diagnostics:** info only, never a warning.
9. **Task-only toolchains:** placed in both lazy and full builds. Full builds load every project's
   tasks to get them.
