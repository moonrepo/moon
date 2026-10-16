# Action graph builder: rewrite plan

A plan for replacing the action graph builder with a new implementation, written from scratch, that
produces the same nodes and edges under the same conditions. Written on 2026-10-15 against
`3.0-ag-new` at `20de2c6867`. Line references are as of that commit, and will drift.

[docs/ACTION_GRAPH.md](../ACTION_GRAPH.md) is the specification of the nodes, edges, and conditions
that must be preserved, and this plan doesn't repeat it. Estimates are marked as such. The phase 0
decisions were made on 2026-10-15 (see [Decisions](#decisions)), and phase 1 (the harness, run
against the current builder) was done the same day (see [Baseline](#baseline)). The new builder
hasn't been started.

## Summary

The current builder
([action_graph_builder.rs](../../crates/action-graph/src/action_graph_builder.rs)) is one struct of
about 2,200 lines (excluding tests) that interleaves task selection, gating, plugin queries, node
creation, edge linking, and edge repair, while inserting nodes one at a time. Much of its complexity
exists to make insertion order not matter: affected state is tracked up front, `--no-dependencies`
edges are linked at the end, serial edges are removed when a later dependency contradicts them,
recursion carries path-local cycle sets, and some node types re-link on reuse while others don't.

The new builder takes declarative requests, and builds in 4 stages with explicit intermediate data:

1. **Select** which task instances exist, and how they relate. Sync, no plugin calls.
2. **Resolve** which provisioning and sync actions exist. Plugin facts are gathered concurrently up
   front, then resolved synchronously.
3. **Assemble** nodes from typed keys, and edges from one rule function per edge kind.
   Deterministic, insertion order doesn't exist.
4. **Order and validate**: serial ordering last, cycle detection, then freeze into an `ActionGraph`.

Verification is a differential harness that runs every existing builder test, plus generated
workspaces, against both builders, and compares canonical graphs.

## Why rewrite

1. **One struct, many jobs.**
   [The builder's fields](../../crates/action-graph/src/action_graph_builder.rs#L147) hold affected
   tracking, the query, changed files, deferred `--no-dependencies` work, ignored dependencies,
   passthrough and primary targets, serial edge bookkeeping, and cleanup and wait targets, all
   mutated during insertion.
2. **Insertion order leaks into the result**, and is compensated for in 5 places:
   - affected state is tracked before insertion
     ([`track_affected`](../../crates/action-graph/src/action_graph_builder.rs#L342),
     [`run_tasks`](../../crates/action-graph/src/action_graph_builder.rs#L630)), as marking lazily
     starved transitive dependents;
   - `--no-dependencies` edges are deferred to `build()`
     ([`link_present_dependencies`](../../crates/action-graph/src/action_graph_builder.rs#L1702));
   - serial edges are skipped when they'd cycle
     ([`try_link_edge`](../../crates/action-graph/src/action_graph_builder.rs#L2184)), and removed
     newest-first when a later dependency contradicts them
     ([`unlink_serial_edges`](../../crates/action-graph/src/action_graph_builder.rs#L2131));
   - [`internal_setup_toolchain`](../../crates/action-graph/src/action_graph_builder.rs#L1527),
     [`internal_setup_environment`](../../crates/action-graph/src/action_graph_builder.rs#L1362),
     and [`internal_sync_project`](../../crates/action-graph/src/action_graph_builder.rs#L1622)
     carry path-local cycle sets;
   - node reuse is inconsistent:
     [`insert_node_if_missing!`](../../crates/action-graph/src/action_graph_builder.rs#L30) re-links
     edges, `insert_node_or_exit!` doesn't, so `SyncProject` edges are still decided by whichever
     caller reached the project first.
3. **Plugin calls are interleaved with graph mutation.** Every insertion awaits plugin functions
   (`locate_dependencies_root`, `define_requirements`, `has_func`, tier checks) while holding
   `&mut self`, through `Box::pin` async recursion. Results are memoized per input
   (`cache_func_with`), so nothing is called twice, but nothing runs concurrently either, and the
   control flow is hard to follow.
4. **Edge semantics are implicit.** Edges are passed around as `Vec<Option<NodeIndex>>` with "first
   available" and "fallback" rules, the install slot returns one of 3 node types, and nodes are
   inserted only to carry requirement edges.
5. **Dedupe is expensive.** `ActionNode`'s `Hash`
   ([action_node.rs:253](../../crates/action/src/action_node.rs#L253)) writes `label()`, which
   formats a new `String` on every lookup.
6. **The built graph is indirect.** It's a `Dag<NodeIndex, _>` whose weights index into a
   `FxHashMap<NodeIndex, ActionNode>`
   ([action_graph.rs:12](../../crates/action-graph/src/action_graph.rs#L12)), and `get_node_index`
   is a linear scan ([action_graph.rs:185](../../crates/action-graph/src/action_graph.rs#L185)).
7. **There was no benchmark** for building the action graph, so its cost was unknown. Phase 1 added
   one (see [Baseline](#baseline)).
8. **Deep dependency chains overflow the stack.** Insertion recurses through boxed futures once per
   task dependency and project dependency, and polling nests just as deep. A chain of 1,000 projects
   (one task each, depending on `^:build`) overflows both an 8MB main thread and a 2MB tokio worker,
   which is where starbase runs every CLI command, while 800 fits on both. Found by the phase 1
   benchmark, in the bench profile, and not verified against a release build of the CLI.

## The contract: what must not change

- **Nodes, identities, edges, and conditions**: everything in
  [docs/ACTION_GRAPH.md](../ACTION_GRAPH.md), including the install slot resolution table, required
  environments, the task gates and their order, the dependency type mapping (optional → required,
  cleanup reversed), serial ordering anchors (`prev` for persistent, `prev_standard` otherwise),
  present-dependency edges, affected gating (dependencies skip it), task selection (dependents walk
  skips cleanup edges, `--query` bounds it, partitioning after the affected filter), builder
  options, and the entry point behavior of each command.
- **Payload types**: `ActionNode` and its node structs, their labels (which appear in snapshots,
  `moon action-graph --dot|--json`, run reports, and webhooks), and `RunTaskNode.id` (used to stop
  the same action running in parallel).
- **What the pipeline reads**: edge weights (`Required`, `Wait`, `Cleanup`; never `Optional`),
  `cleanup_indices`, priorities, and every `ActionContext` field: primary targets, passthrough
  target states, ignored dependencies, cleanup and wait targets, changed files, and the affected
  state, including the marks made while gating tasks.
- **Errors**: the `ActionGraphError` variants and messages
  ([action_graph_error.rs](../../crates/action-graph/src/action_graph_error.rs)). Tests assert the
  `MissingToolchainRequirement` text, the job and plan errors, and `WouldCycle` naming a specific
  edge.
- **What commands read back**: the partition (`NodeIndex → Target`, and size) that `exec` prints and
  stores ([exec.rs:446](../../crates/app/src/commands/exec.rs#L446)), whether `setup_toolchain`
  inserted a node (`moon setup` counts them), and the node count.

## Decisions

These were artifacts of the current construction order, not part of the spec. They were decided on
2026-10-15 (phase 0), and they define what "equivalent" means for the differential harness: the new
builder may only produce a different graph where one of these decisions says so.

| #   | Behavior                                                                                             | Today                                                                                                             | Decision                                                                                                                                                                                                           |
| :-- | :--------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `SyncProject` dependency edges (ACTION_GRAPH.md, Notes)                                              | Linked only by the first caller to insert the node                                                                | Computed from the full set: once the synced projects are known, every synced project links each synced direct dependency                                                                                           |
| 2   | Cyclic project dependencies (cycles across scopes in the unioned project graph)                      | The project reached first gets the edge                                                                           | Production first: add production and peer edges first (that partition is acyclic), then development, build, and root edges in sorted order, skipping any that would close a cycle                                  |
| 3   | Serial ordering conflicts                                                                            | Serial edges are linked during insertion, and the newest is removed when a later dependency contradicts it        | Serial edges are added after every real edge exists, in declared order, skipping any that would cycle. Nothing is removed. Differences are limited to conflict cases, checked against the `serial_conflicts` tests |
| 4   | Mutual `for_setup_toolchain` requirements (2 toolchains that require each other)                     | Skipped along one recursion path, but a later request can fail with `WouldCycle`, depending on order (unverified) | Skip a requirement edge that would close a requirement cycle, in sorted order, which matches the intent of the path-local skip                                                                                     |
| 5   | Placeholder nodes for requirement-only toolchains (ACTION_GRAPH.md, Notes)                           | Inserted                                                                                                          | Keep                                                                                                                                                                                                               |
| 6   | Different `RunRequirements` per call                                                                 | Each call can pass its own; every command passes one per build                                                    | One `RunRequirements` per build, set on the builder                                                                                                                                                                |
| 7   | Persistent gate when a task is reached in several ways                                               | Decided per call, by the call's origin                                                                            | Union semantics: an instance exists if any path admits it, and it's primary only if a primary path admits it. Same results, verified with the `persistent_in_ci` tests                                             |
| 8   | Which edge `WouldCycle` names                                                                        | The edge being linked when the cycle formed                                                                       | `WouldCycle` for the first edge, in assembly order, that closes the cycle, with the full cycle chain appended to the message. Existing `contains` assertions keep passing                                          |
| 9   | `SetupProto` doesn't link `SyncWorkspace`, `moon sync` ignores `pipeline.*` (ACTION_GRAPH.md, Notes) | As described                                                                                                      | Keep for parity, and revisit separately, outside the rewrite                                                                                                                                                       |

## Design

### Requests

The builder stores requests, and only does work in `build()`, so the order requests are made in
doesn't matter:

```rust
let mut builder = ActionGraphBuilder::new(app_context, workspace_graph, options);

builder.set_requirements(reqs); // one per build (decision 6)
builder.set_query(query)?;
builder.set_changed_files(changed_files);
builder.track_affected(depth, ci_check);

let run = builder.run_tasks_with_plan(&plan); // returns a RequestId
builder.sync_workspace();

let output = builder.build().await?; // BuildOutput { context, graph, report }
let partition = output.report.partition(run);
```

| Request                        | Replaces                                                           | Used by                      |
| :----------------------------- | :----------------------------------------------------------------- | :--------------------------- |
| `RunTasks(locators\|plan)`     | `run_tasks`, `run_tasks_with_plan`, `run_task_by_target(_locator)` | `exec`, `action-graph`       |
| `RunTask(task)`                | `run_task`                                                         | `action-graph`, `sync`       |
| `SyncWorkspace`                | `sync_workspace`                                                   | `exec` in CI, `sync`, MCP    |
| `SyncProject(id)`              | `sync_project`                                                     | `sync`, `sync projects`, MCP |
| `SetupProto`                   | `setup_proto`                                                      | `setup`                      |
| `SetupToolchain(spec)`         | `setup_toolchain`                                                  | `setup`                      |
| `SetupEnvironmentRoot(spec)`   | `setup_environment_root`                                           | `setup`                      |
| `InstallDependencies(project)` | `install_dependencies_by_project`                                  | `docker setup`               |

`build()` becomes async and fallible. The test-only entry points (`run_task_with_config`,
`run_task_dependencies`, `install_dependencies_root`, project-scoped `setup_environment`) become
requests too, or test helpers.

### Stage 1: select

Turns the task requests into a `TaskPlan`. It reuses the current resolution logic (locators, tags,
the default project, `--query`, dependents expansion, affected tracking, job partitioning),
extracted from the builder into free functions.

```rust
struct TaskInstanceKey {
    target: Target,
    args: Vec<String>,
    env: Vec<(String, Option<String>)>, // sorted by key
}

struct TaskPlan {
    instances: IndexMap<TaskInstanceKey, TaskInstance>, // task, flags, priority, admission
    relations: Vec<TaskRelation>,                       // from, to, Required | Wait | Cleanup
    serial: Vec<SerialChain>,                           // parent, required deps in declared order
    primaries: Vec<TaskInstanceKey>,
    passthrough: FxHashSet<Target>,
    ignored_dependencies: FxHashMap<Target, FxHashSet<Target>>,
    cleanup_targets: FxHashSet<Target>,
    wait_targets: FxHashSet<Target>,
    partition: PartitionInfo,
}
```

- With one `RunRequirements` per build, the interactive and persistent flags are a function of the
  target, so they don't need to be in the key, while labels stay the same.
- The dependency closure is a worklist that applies the gates in the same order as today. Union
  semantics (decision 7) make the result independent of the order the worklist runs in.
- `--no-dependencies` is no longer deferred: once the closure is known, each instance's non-cleanup
  dependencies are filtered to the instances that exist, and the rest are recorded as ignored.
- Serial chains are recorded as data, not edges.
- Affected checks go through the same `AffectedTracker` calls as today, in a deterministic order, so
  `ActionContext.affected` gets the same marks.

### Stage 2: resolve

Decides which `SyncWorkspace`, `SetupProto`, `SetupToolchain`, `SetupEnvironment`,
`InstallDependencies`, and `SyncProject` nodes exist, producing a `ProvisionPlan`.

1. **Gather facts** with an async worklist. Unknown keys are fetched concurrently, then expanded
   synchronously, until no new keys appear (requirements can introduce new toolchains):
   - `ToolchainFacts`, keyed by toolchain id and config scope (a project, or the workspace): tier 2
     and 3 support, `install_dependencies` and `setup_environment` exports, and the
     `define_requirements` output;
   - `RootFacts`, keyed by toolchain id and project (or the workspace root): the located
     `DependenciesWorkspace`, and the project's role in it.

   Builder options and toolchain switches are applied before gathering, so disabled toolchains are
   never queried.

2. **Resolve** with pure functions over the facts: the install slot table, environment setup
   (requirements and required environments), toolchain setup (requirements), the proto rule, and the
   project sync set (owners of task instances, plus their dependency closure, with affected gating
   and switches).

Requirements are read exactly as they are today: `requires` with `for_setup_toolchain` and
`for_setup_environment`, and required environments regardless of the flags (see the `SetupToolchain`
and `SetupEnvironment` sections of [ACTION_GRAPH.md](../ACTION_GRAPH.md)).

### Stage 3: assemble

```rust
enum NodeKey {
    SyncWorkspace,
    SetupProto,
    SetupToolchain(ToolchainSpec),
    SetupEnvironment { toolchain: Id, root: WorkspaceRelativePathBuf, project: Option<Id> },
    InstallDependencies { toolchain: Id, root: WorkspaceRelativePathBuf, members: Option<Vec<String>>, project: Option<Id> },
    SyncProject(Id),
    RunTask(TaskInstanceKey),
}
```

- `NodeKey` derives `Hash` and `Eq`, so lookups don't format strings. Keys are mapped to their
  `ActionNode` payloads when freezing.
- Edges are collected into a set of `(from, to, TaskDependencyType, EdgeReason)`, deduped by
  `(from, to, weight)`. `EdgeReason` (workspace, proto, toolchain requirement, environment
  requirement, required environment, install slot, project dependency, task owner, task slot,
  fallback, task dependency, cleanup, serial, present dependency) is for diagnostics, and doesn't
  reach the pipeline.
- There's one rule function per edge kind, named after its section in ACTION_GRAPH.md, iterating
  keys in sorted order.
- `SyncProject → SyncProject` edges are computed from the full set of synced projects, production
  and peer edges first, then the other scopes in sorted order, skipping any that would close a cycle
  (decisions 1 and 2).
- Nodes are added in a documented, deterministic order (by stage, then key), so node indices no
  longer depend on how a command made its requests.

### Stage 4: order and validate

1. **Serial edges** are computed against the assembled graph: walk each dependency's subtree over
   `Required` edges between `RunTask` nodes, and add edges to the anchor, in chain order. An edge
   that would cycle is skipped. Since every real edge already exists, nothing is ever removed
   (decision 3).
2. **Preference edges** (serial, required environments, and requirement edges that would close a
   requirement cycle, per decision 4) are the only ones that may be skipped. Any other cycle is an
   error, reported as `WouldCycle` for the first edge that closes it, with the cycle chain appended
   (decision 8).
3. **Freeze** into an `ActionGraph`, `ActionContext`, and a `BuildReport` (each request's node
   indices, and the partition).

### Module layout

`crates/action-graph/src/builder/`: `mod.rs` (requests and the public API), `select.rs`,
`resolve.rs`, `assemble.rs`, `order.rs`, and `report.rs`. Estimated 1,600 to 2,000 lines, with most
of the reduction coming from dropping repair logic (serial edge removal, deferred linking, cycle
sets, and mixed reuse rules). This is an estimate.

## Verification

1. **Canonical form.** Sorted nodes (by `NodeKey`, which includes args and env, as labels collide
   for dependency variants), sorted edges (`from`, `to`, weight), cleanup nodes, and every context
   field, sorted. Two builds are equivalent when their canonical forms match. Done in phase 1:
   `CanonicalActionGraph` in [tests/utils.rs](../../crates/action-graph/tests/utils.rs), keyed by
   `get_node_key` (the label, plus args and sorted env for tasks, and the project and members that
   labels omit), with `with_partition`, a line-by-line `diff`, and `assert_canonical_eq`. Parallel
   edges with the same weight are merged, as the pipeline merges them, and a key held by more than
   one node is reported in `duplicate_nodes`.
2. **Differential harness.** `ActionGraphContainer`
   ([tests/utils.rs](../../crates/action-graph/tests/utils.rs)) builds with either builder, and
   every test in
   [action_graph_builder_test.rs](../../crates/action-graph/tests/action_graph_builder_test.rs) (224
   tests, 114 snapshots) also runs in a differential mode that asserts both builders produce the
   same canonical form, except for the documented decisions.
3. **Snapshots and `topo` assertions.** Node indices change with the new insertion order, which
   changes every dot snapshot and many `topo()` vectors, even for identical graphs. Snapshots are
   regenerated once, at the switch, and accepted only where the differential harness proves the
   graphs equivalent. `topo()` assertions move to canonical comparisons.
4. **Generated workspaces.** A seeded generator (extending `moon_bench_utils`, without a new
   dependency) that produces projects with dependencies (including cycles across scopes), tasks with
   every dependency type, serial parents, persistent and `runInCI` tasks, and `tc-*` toolchains.
   Compare both builders across many seeds, and check invariants: acyclic, every node except
   `SyncWorkspace` and `SetupProto` reaches `SyncWorkspace` when enabled, cleanup and wait edges
   only between tasks, and no edges from setup or sync nodes into tasks. Done in phase 1 for the
   current builder: `create_generated_workspace` in
   [generator.rs](../../crates/bench-utils/src/generator.rs), `find_invariant_violations` in
   [tests/utils.rs](../../crates/action-graph/tests/utils.rs), and
   [action_graph_invariants_test.rs](../../crates/action-graph/tests/action_graph_invariants_test.rs)
   (see [Baseline](#baseline)).
5. **Benchmark.** `crates/action-graph/benches/action_graph_builder.rs`, with `criterion` (the
   CodSpeed compat package) and `moon_bench_utils` (`create_simple_workspace`,
   `create_chained_workspace`), like `crates/affected/benches/affected_tracker.rs`, building every
   task at 100 and 1,000 projects (more locally), for both builders. Done in phase 1 for the current
   builder (see [Baseline](#baseline)).
6. **End to end.** The `moon_cli` tests for `run`, `ci`, `check`, `exec`, `setup`, `sync`, and
   `docker setup`.

## Phases

0. **Decide** the behaviors and open questions. Done on 2026-10-15, see [Decisions](#decisions) and
   [Resolved questions](#resolved-questions).
1. **Harness.** The canonical form, benchmark, and generator, run against the current builder only,
   as a baseline. Done on 2026-10-15, see [Baseline](#baseline).
2. **Tasks.** Stages 1, 3, and 4 for tasks only, behind a separate type, with the harness comparing
   the task subgraph (task nodes and the edges between them).
3. **Provisioning.** Stage 2, and project sync, with the harness comparing full graphs.
4. **Switch.** Move the commands, MCP, and test utilities to the request API, regenerate snapshots
   (reviewed against the harness), and update ACTION_GRAPH.md.
5. **Delete** the current builder.
6. **Follow-up:** store the built graph directly as `DiGraph<ActionNode, TaskDependencyType>`, which
   touches the action pipeline (`job_dispatcher.rs`, `action_pipeline.rs`), the `action-graph`
   command, and the graph traits. Decided: a separate change, after the switch. The `ActionGraph`
   API stays the same during the rewrite.

## Baseline

Phase 1, measured on 2026-10-15 against the current builder, on `3.0-ag-new` at `20de2c6867` plus
the harness changes.

### Benchmark

[benches/action_graph_builder.rs](../../crates/action-graph/benches/action_graph_builder.rs), run
with `cargo bench -p moon_action_graph --bench action_graph_builder` on an Apple M1 Pro (10 cores).
Each iteration creates a builder, calls `run_tasks` with every task as a target (`moon run` with
default requirements and options), and calls `build()`. The workspace graph is built, and every task
expanded, once outside the timing, and the app context is shared. The simple workspace has 4 tasks
per project (a chain of 3, plus `build`) and no project dependencies. The chained workspace is one
`build` task per project, with each project depending on the previous one. The 5,000 project case
only runs outside of CI and headless sessions, so it wasn't measured.

| Case                     | Projects | Tasks | Benched (worker) | Main thread (`block_on`) |
| :----------------------- | -------: | ----: | ---------------: | -----------------------: |
| `run_tasks/10`           |       11 |    44 |           191 µs |                   160 µs |
| `run_tasks/100`          |      101 |   404 |          1.63 ms |                  1.48 ms |
| `run_tasks/1000`         |    1,001 | 4,004 |          17.9 ms |                  16.4 ms |
| `chained/run_tasks/10`   |       11 |    11 |            73 µs |                    55 µs |
| `chained/run_tasks/100`  |      101 |   101 |           572 µs |                   503 µs |
| `chained/run_tasks/1000` |    1,001 | 1,001 |          7.72 ms |           stack overflow |

The benchmark spawns every build onto a runtime worker with a 256MB stack, as the 1,000 project
chain overflows the main thread (see [Why rewrite](#why-rewrite), item 8). Spawning measures 9% to
32% slower (the most for the smallest cases) than polling on the main thread, which is the second
column, measured before that change. Compare both builders with the same method. The cost grows
about linearly with the number of tasks: about 4µs per task in the simple workspace, and 5 to 8µs
per task in the chained workspace.

### Generated workspaces

[action_graph_invariants_test.rs](../../crates/action-graph/tests/action_graph_invariants_test.rs)
generates 64 workspaces by default (`ACTION_GRAPH_SEEDS=<count>` for more,
`ACTION_GRAPH_SEED=<seed>` to reproduce one), split across 4 test shards. Each has 3 to 10 projects
and 1 to 5 tasks per project, and is run from the workspace root, a project root, or a dependencies
workspace, as the tier 2 test toolchains locate the dependencies root from the working directory.
Each is built in 13 scenarios modeled on the commands: `run` (plain, `--affected`, `--no-deps` on
every other target, `--query` with `:task` locators, and with workspace sync, project dependency
sync, and installs disabled), `ci` (plain, deep affected depth and dependents, and partitioned into
2 jobs), `check --interactive`, `action-graph`, `sync`, `setup`, and `docker setup`.

- **Every invariant held for every seed and scenario.** No current-builder bug was found by them.
- Every build is also checked for: unique node identities, no `Optional` weights, every cleanup
  index being a task with an outgoing `Cleanup` edge. Each seed also builds the deep affected `ci`
  scenario twice, and requires identical canonical forms.
- A coverage check fails a shard if its seeds stop producing any of: every node type, dependency
  variants, a dependencies workspace (members), cleanup and wait edges, requirement edges between
  `SetupEnvironment` and between `SetupToolchain` nodes, `SyncProject` dependency edges, affected
  marks, ignored dependencies, and passthrough target states. Across the 208 graphs of one shard
  (16 seeds), 274 task nodes shared their label with another node (dependency variants), and there
  were 1,196 `SyncProject → SyncProject` edges.
- The suite takes about 25 seconds alone. Most of it is loading and dropping the toolchain plugins
  once per seed: dropping compiled plugins is slow on macOS (wasmtime deregisters the unwind info of
  every function), so each seed shares one plugin registry between its workspace graph and its
  builds.

## Risks

- **Hidden order dependence that users rely on.** Graphs that differ only in conflict cases
  (decisions 1 to 3), and `WouldCycle` messages (decision 8), will change. The harness surfaces
  them, and they go in the changelog.
- **Affected state.** Gating marks tasks lazily, and those marks are output through the context.
  Stage 1 must make the same calls, or reports and `affectedFiles` change.
- **Error precedence.** Gathering facts concurrently can change which plugin error surfaces first.
  Report the first error by key, not by completion order.
- **Plugin call volume.** Gathering facts up front can query toolchains whose nodes would have been
  gated out later. Gate before gathering, and compare call counts in the benchmark.
- **API churn.** `build()` becomes async and returns a report: 7 call sites in `crates/app` and
  `crates/mcp`, plus the test utilities.
- **Test churn.** 224 tests and 114 snapshots, most of it mechanical.

## Resolved questions

Decided on 2026-10-15. No questions are open.

1. **API:** request-based. Builder methods record requests and return a `RequestId`, and
   `build().await` returns the graph, the context, and a report (the partition, and each request's
   node).
2. **Requirements:** one `RunRequirements` per build (decision 6).
3. **Project cycles:** production and peer edges first (decision 2).
4. **Serial conflicts:** add serial edges last, and skip conflicts (decision 3).
5. **Cycle errors:** `WouldCycle` for the closing edge, with the chain appended (decision 8).
6. **Graph storage:** a separate change, after the switch (phase 6).
