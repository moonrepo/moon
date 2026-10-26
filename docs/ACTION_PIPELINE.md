# Action pipeline: actions, operations, jobs, and the runner

A specification of the action pipeline that moon runs once an action graph has been built: the data
model of actions and operations, how the pipeline schedules and dispatches jobs from the graph, how
each kind of action executes, how a task runs (hashing, caching, execution, archiving), and what the
pipeline reports. Every behavior listed here is one the implementation must preserve.

Written on 2026-10-24 against `develop-3.0` at `951a544abd`, from an audit of the code. The code is
the spec: where something looks surprising, it's called out in [Notes](#notes) instead of being
described as the intent. Graph _construction_ (which nodes and edges exist) is specified separately
in [ACTION_GRAPH.md](./ACTION_GRAPH.md), and isn't repeated here, except for the parts of the graph
that the pipeline reads.

Source files:

- [action/](../crates/action/src): `Action`, `ActionNode`, `Operation`, `OperationList`,
  `OperationMeta`, and the status enums.
- [action-context/](../crates/action-context/src/lib.rs): `ActionContext` and `TargetState`, the
  state shared by every job of a run.
- [exec-plan/](../crates/exec-plan/src/exec_plan.rs): the `ExecutionPlan` that `moon exec` runs.
- [action-pipeline/](../crates/action-pipeline/src): the pipeline, job dispatcher, jobs, abort
  state, event emitter, subscribers, and reports.
- [actions/](../crates/actions/src): the handler of each action type, shared hashing and locking
  utilities, plugin command execution, and the workspace sync operations.
- [task-runner/](../crates/task-runner/src): the task runner, executor, command builder, checks,
  output archiver, and output hydrater.
- [task-hasher/](../crates/task-hasher/src): what goes into a task's hash.
- [app/src/helpers.rs](../crates/app/src/helpers.rs) and
  [app/src/commands/exec.rs](../crates/app/src/commands/exec.rs): how commands configure and run the
  pipeline, and derive an exit code.

## Overview

```
ActionGraph + ActionContext
        │
        ▼
ActionPipeline::run_with_context
  ├─ register subscribers, emit PipelineStarted
  ├─ spawn signal monitor ──────────────────────────────┐
  ├─ spawn dispatcher task                              │ cancel_token
  │    JobDispatcher::next() ─► Job ─► run_action ─► handler (actions crate)
  │                                       │              └─ RunTask ─► TaskRunner
  │                                       └─ result ─► results channel
  ├─ receive results, abort / drain / complete
  └─ emit PipelineCompleted, return Vec<Action>
```

The pipeline is a consumer of two things the graph builder produced: the `ActionGraph` (nodes,
weighted edges, the cleanup index set, and the layout graph), and an `ActionContext` (primary
targets, passthrough states, cleanup and wait targets, changed files, affected state). It turns
every node into exactly one `Action`, runs them in dependency order with bounded concurrency, and
returns the actions in completion order.

Terminology:

| Term          | Meaning                                                                                                                      |
| :------------ | :--------------------------------------------------------------------------------------------------------------------------- |
| **Node**      | An `ActionNode` in the graph: what to do (`RunTask(app:build)`, `SetupToolchain(node 20)`, and so on).                       |
| **Action**    | The record of running one node once: status, timing, error, and its operations. Serialized into the run report.              |
| **Operation** | One step within an action (hash generation, a process execution, a cache hydration, a sync), with its own status and timing. |
| **Job**       | The pipeline's unit of scheduling: a node index plus the contexts needed to run it. One job per node, spawned as a future.   |
| **Target**    | A task's `project:task` identifier. Task state is tracked per target in the action context.                                  |

## Data model

### `ActionNode`

[action_node.rs](../crates/action/src/action_node.rs). The graph's node weight, and the action's
`node`. The variants and their payloads:

| Variant               | Payload                                                                                                    | Label                                               |
| :-------------------- | :--------------------------------------------------------------------------------------------------------- | :-------------------------------------------------- |
| `None`                | none (the enum default; never inserted by the builder)                                                     | `None`                                              |
| `InstallDependencies` | `toolchain_id`, `root` (workspace-relative), `members` (workspace globs), `project_id`, `owner_project_id` | `InstallDependencies(<id>[, <root>])`               |
| `RunTask`             | `target`, `args`, `env` (ordered map of `Option<String>`), `interactive`, `persistent`, `priority`, `id`   | `Run[Interactive\|Persistent]Task(<target>)`        |
| `SetupEnvironment`    | `toolchain_id`, `root`, `project_id`, `owner_project_id`                                                   | `SetupEnvironment(<id>[, <root>])`                  |
| `SetupProto`          | `version`                                                                                                  | `SetupProto(<version>)` (`1.2.3` under `MOON_TEST`) |
| `SetupToolchain`      | `toolchain` (a `ToolchainSpec`: id and optional version requirement)                                       | `SetupToolchain(<id> <req>)`                        |
| `SyncProject`         | `project_id`                                                                                               | `SyncProject(<id>)`                                 |
| `SyncWorkspace`       | none                                                                                                       | `SyncWorkspace`                                     |

Behaviors the pipeline relies on:

- **Identity.** `Hash`/`Eq` are the label plus the payload. For `RunTask`, `args` and `env` are part
  of the hash (env sorted by key), so the same target with different dependency args or env is a
  different node. `priority` and `id` are not hashed (`RunTaskNode` derives `Eq` over all fields,
  but `Hash` is manual).
- **Exclusive id.** `RunTaskNode::id` is an `FxHasher` of the target string, plus a marker byte when
  persistent (`100`) and when interactive (`50`). It's computed by `ActionNode::run_task`, and read
  by `get_id()` (`0` for every other variant). The dispatcher uses it to never run two standard
  instances of the same target at once (see [Exclusive targets](#exclusive-targets)).
- **Priority.** `get_priority()` is the task's `options.priority` level for `RunTask` (`0` critical,
  `1` high, `2` normal (default), `3` low), and `0` for every other variant.
- **Flags.** `is_interactive()` and `is_persistent()` are the `RunTask` flags (false otherwise).
  `is_standard()` is "not interactive and not persistent" for `RunTask`, and `true` for every other
  variant.
- `Display` is the label. Serializes as `{ "action": "<kebab-variant>", "params": {...} }`.

### `Action`

[action.rs](../crates/action/src/action.rs). Created by the job for its node, mutated by the action
runner and handler, and returned to the caller.

| Field            | Type               | Set by                                                                                                 |
| :--------------- | :----------------- | :----------------------------------------------------------------------------------------------------- |
| `node`           | `Arc<ActionNode>`  | `Action::new`.                                                                                         |
| `node_index`     | `usize`            | The job, from the graph index. `0` by default.                                                         |
| `label`          | `String`           | `Action::new`, from `node.label()`.                                                                    |
| `status`         | `ActionStatus`     | `Running` on creation; `finish(status)`, `abort()`.                                                    |
| `allow_failure`  | `bool`             | `run_task`, from `options.onFailure: allow`. `false` for every other action.                           |
| `expect_failure` | `bool`             | `run_task`, from `options.onFailure: expect`.                                                          |
| `flaky`          | `bool`             | `run_task`, from `OperationList::is_flaky`.                                                            |
| `operations`     | `OperationList`    | Handlers push operations; `run_task` replaces the list with the runner's.                              |
| `created_at`     | `NaiveDateTime`    | `Action::new`.                                                                                         |
| `started_at`     | `Option<..>`       | `start()`, called by the action runner right before the handler. `None` means it never started.        |
| `finished_at`    | `Option<..>`       | `finish()`.                                                                                            |
| `duration`       | `Option<Duration>` | `finish()`, elapsed since `start()`. `None` when `start()` was never called.                           |
| `error`          | `Option<String>`   | `fail(report)`, the report's display string. Serialized.                                               |
| `error_report`   | `Option<Report>`   | `fail(report)`. Not serialized. `get_error()` takes it (falls back to `error`, then "Unknown error!"). |
| `start_time`     | `Option<Instant>`  | `start()`. Not serialized.                                                                             |

Predicates:

| Method                | Result                                                                                                                                                                                                                             |
| :-------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `has_failed()`        | `status` is `Aborted`, `Failed`, or `TimedOut`.                                                                                                                                                                                    |
| `has_error()`         | `error` or `error_report` is set.                                                                                                                                                                                                  |
| `was_cached()`        | `status` is `Cached` or `CachedFromRemote`.                                                                                                                                                                                        |
| `should_bail()`       | `!allow_failure && has_failed()`.                                                                                                                                                                                                  |
| `should_abort()`      | `status == Aborted`, **or** the last `TaskExecution` operation has status `Aborted` (the execution injected when a task fails before its command spawns, see [Injected failure](#injected-failure)).                               |
| `get_exit_code()`     | The exit code of the last `TaskExecution` operation, if any (`None` for setup and sync actions, aborts, and signals).                                                                                                              |
| `get_changed_files()` | Every `changed_files` path of every `SetupOperation` and `SyncOperation` in `operations`.                                                                                                                                          |
| `get_prefix()`        | A kebab-case name per variant (`run-task`, `install-dependencies`, `setup-environment`, `setup-proto`, `setup-toolchain`, `sync-project`, `sync-workspace`, `unknown`), used for hasher labels, lock names, and metric attributes. |
| `get_duration()`      | Panics when `duration` is `None`. Only called after `finish()`.                                                                                                                                                                    |

`abort()` sets `status = Aborted` without touching timestamps. `finish(status)` sets `finished_at`,
`status`, and `duration`, and may be called more than once (each call overwrites the status).

### `ActionStatus`

The status of an action and of an operation. Serialized kebab-case; `get_type()` returns the same
string.

| Status             | Meaning                                                                                                               |
| :----------------- | :-------------------------------------------------------------------------------------------------------------------- |
| `Running`          | Default. In progress, or never finished.                                                                              |
| `Passed`           | Ran and succeeded.                                                                                                    |
| `Failed`           | Ran and failed, or errored before running.                                                                            |
| `Cached`           | A task whose outputs were reused from the previous run on disk, or hydrated from the local cache.                     |
| `CachedFromRemote` | A task whose outputs were hydrated from a remote cache backend.                                                       |
| `Skipped`          | Nothing happened: a gate was not passed, or there was nothing to do.                                                  |
| `Invalid`          | A sync action modified files in CI (which should have been committed); a hydration that missed; no process operation. |
| `TimedOut`         | A task execution attempt exceeded `options.timeout`.                                                                  |
| `Aborted`          | The pipeline aborted the action: before it started, while it ran (a sibling failed), or a provisioning action failed. |

Status groups used throughout: **failed** is `Aborted | Failed | TimedOut`, **passed** is
`Cached | CachedFromRemote | Passed`, **cached** is `Cached | CachedFromRemote`.

### `ActionPipelineStatus`

The status of a whole run: `Pending` (default), `Completed` (every result received), `Aborted` (an
action aborted the pipeline), `Interrupted` (`SIGINT`), `Terminated` (`SIGHUP` or `SIGTERM`). Other
signals report `Aborted`. See [Signals](#signals).

### `Operation`

[operation.rs](../crates/action/src/operation.rs). A step within an action.

| Field         | Type               | Notes                                                                                                            |
| :------------ | :----------------- | :--------------------------------------------------------------------------------------------------------------- |
| `meta`        | `OperationMeta`    | The kind of operation and its payload (below).                                                                   |
| `status`      | `ActionStatus`     | `Running` on creation.                                                                                           |
| `id`          | `Option<Id>`       | Set for setup and sync operations (the action prefix, or the plugin operation id in kebab-case).                 |
| `plugin`      | `Option<Id>`       | The plugin that produced it, for plugin commands and sync operations.                                            |
| `operations`  | `Vec<Operation>`   | Nested operations: a plugin sync's own operations are nested under its sync operation. Skipped when empty.       |
| `started_at`  | `NaiveDateTime`    | `new()` sets now; `new_finished()` sets started and finished to the same instant (no duration).                  |
| `finished_at` | `Option<..>`       | `finish()`.                                                                                                      |
| `duration`    | `Option<Duration>` | `finish()`, from `start_time`. `None` for `new_finished` operations and converted plugin operations without one. |
| `start_time`  | `Option<Instant>`  | Not serialized.                                                                                                  |

`OperationMeta` ([operation_meta.rs](../crates/action/src/operation_meta.rs)), tagged `type` in
kebab-case:

| Variant            | Group   | Payload                                                                    | Produced by                                                                      |
| :----------------- | :------ | :------------------------------------------------------------------------- | :------------------------------------------------------------------------------- |
| `NoOperation`      | process | none                                                                       | A `noop` task (`command` is `nop`, `noop`, or `no-op`, with no `script`).        |
| `OutputHydration`  | process | process output (`command`, `exit_code`, `stdout`, `stderr` from the cache) | A task's cache check and hydration.                                              |
| `ProcessExecution` | process | process output                                                             | A plugin command (install, dedupe, environment setup) or a task check.           |
| `SetupOperation`   | process | `changed_files`                                                            | The overall step of `SetupToolchain`, `SetupEnvironment`, `InstallDependencies`. |
| `SyncOperation`    | process | `changed_files`                                                            | Each sync of `SyncWorkspace` and `SyncProject`, and converted plugin operations. |
| `TaskExecution`    | process | process output                                                             | Each attempt of a task's command, and the skipped/injected placeholders.         |
| `ArchiveCreation`  | metric  | none                                                                       | A task's output archiving.                                                       |
| `HashGeneration`   | metric  | `hash`                                                                     | Every action that hashes (tasks, and the fingerprinted setup actions).           |
| `MutexAcquisition` | metric  | none                                                                       | A task with `options.mutex`.                                                     |

`OperationMetaProcessOutput`: `command` and `exit_code` are serialized; `exit_status`, `stdout`, and
`stderr` (as `Arc<String>`) are not. `set_stdout`/`set_stderr` ignore empty strings.
`get_exit_code()` defaults to `-1`.

Operation behaviors:

- **Constructors** (`archive_creation`, `hash_generation`, `no_operation`, `mutex_acquisition`,
  `output_hydration`, `process_execution(command)`, `task_execution(command)`,
  `setup_operation(id)`, `sync_operation(id)`) create a `Running` operation.
  `new_finished(meta, status)` creates one that's already done, used for skipped executions.
- **`finish_from_output(status, stdout, stderr)`** records the exit code and status on the process
  output, converts the bytes lossily to UTF-8 (process output may not be UTF-8), and finishes as
  `Passed` when the status is success, otherwise `Failed`. A `None` status finishes as `Failed`.
- **Trackers** (`track`, `track_with_check`, `track_async`, `track_async_with_check`,
  `track_async_changed`, and the `do_track*` forms that take `&mut`) run a closure and finish the
  operation from its result: `Ok` finishes `Passed`, or `Skipped` when a checker returns `false`;
  `Err` finishes `Failed` and propagates the error. `track_async_changed` additionally appends the
  returned paths to a setup/sync operation's `changed_files`.
- **Accessors.** `get_exec_output[_mut]()` for the three process-output variants.
  `get_file_state[_mut]()` for setup and sync operations. `get_exec_output_status()` renders
  `exit code N`, the exit status, or `skipped` / `timed out` / `unknown failure` by status.
- **Predicates.** `has_failed()`, `has_passed()`, `is_cached()` per the status groups above.
  `has_output()` is whether stdout or stderr is non-empty.

### `OperationList`

[operation_list.rs](../crates/action/src/operation_list.rs). A `Vec<Operation>` newtype with derefs,
plus:

| Method                 | Result                                                                                                                                                                                                                                    |
| :--------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_final_status()`   | The status of the last **process-based** operation (`NoOperation`, `OutputHydration`, `ProcessExecution`, `SyncOperation`, `TaskExecution`), or `Invalid` when there is none. Note `SetupOperation` isn't included (see [Notes](#notes)). |
| `get_last_process()`   | That operation.                                                                                                                                                                                                                           |
| `get_last_metric()`    | The last `ArchiveCreation`, `HashGeneration`, or `MutexAcquisition`.                                                                                                                                                                      |
| `get_last_execution()` | The last `TaskExecution`.                                                                                                                                                                                                                 |
| `get_hash()`           | The hash of the first `HashGeneration`.                                                                                                                                                                                                   |
| `has_executed_task()`  | Whether the task's command ran at least once: any `TaskExecution` with status `Passed`, `Failed`, or `TimedOut`, **or** a passed `NoOperation`. Skipped and aborted executions don't count. Drives cleanup scheduling.                    |
| `is_flaky()`           | At least one `TaskExecution`, at least one failed, and the last one passed.                                                                                                                                                               |
| `merge(other)`         | Appends.                                                                                                                                                                                                                                  |
| `take()`               | Moves the operations out, leaving an empty list.                                                                                                                                                                                          |

### `ActionContext` and `TargetState`

[action-context/lib.rs](../crates/action-context/src/lib.rs). One per run, built by the graph
builder, completed by the command, and shared (`Arc`) with every job. Serialized into the run report
(the `@mutable` fields and the token/mutex fields are skipped).

| Field                  | Set by                     | Read by                                                                                                                 |
| :--------------------- | :------------------------- | :---------------------------------------------------------------------------------------------------------------------- |
| `abort_token`          | pipeline (cancels)         | Tasks, so they don't start a command after a sibling failed.                                                            |
| `cancel_token`         | pipeline (cancels)         | Tasks, so they don't start a command after a signal.                                                                    |
| `affected`             | builder                    | Command builder (`affectedFiles` option), `install_dependencies` (skips when affected locally), `exec` display.         |
| `changed_files`        | builder                    | Command builder (affected files), `install_dependencies` (manifest changes).                                            |
| `cleanup_targets`      | builder                    | `should_always_run`, `should_stop`.                                                                                     |
| `wait_targets`         | builder                    | `should_always_run`.                                                                                                    |
| `ignored_dependencies` | builder                    | Dependency completeness and hashing, under `--no-dependencies`.                                                         |
| `initial_targets`      | command (`exec`)           | `should_inherit_args` (`:task` locators).                                                                               |
| `primary_targets`      | builder                    | `is_primary_target`, `get_target_prefix`, `should_inherit_args`, needless cleanup exemption, output streaming defaults. |
| `passthrough_args`     | command (`exec`)           | Command builder and hasher, for targets that inherit args.                                                              |
| `output_style`         | command (`exec`)           | Overrides every task's `options.outputStyle`.                                                                           |
| `named_mutexes`        | tasks (lazily)             | `get_or_create_mutex`.                                                                                                  |
| `target_states`        | builder, dispatcher, tasks | Dependency completeness, hashing, skip decisions.                                                                       |

`TargetState`, serialized as `{ state, hash }`:

| State                      | Meaning                                                                                                       | `is_complete` | `is_skipped` |
| :------------------------- | :------------------------------------------------------------------------------------------------------------ | :-----------: | :----------: |
| `Passed(hash)`             | Ran or was hydrated, with a hash.                                                                             |      yes      |      no      |
| `Passthrough`              | Ran without a hash (cache off, or a noop without one), a passthrough target, or a dispatched persistent task. |      yes      |      no      |
| `Failed`                   | Ran and failed.                                                                                               |      no       |      no      |
| `Skipped`                  | Did not run, because a dependency failed or was skipped, or the pipeline stopped.                             |      no       |     yes      |
| `SkippedConditional(hash)` | Did not run because every `condition` check passed; counts as complete for dependents.                        |      yes      |     yes      |

`TargetState::from_hash(Option<&str>)` is `Passed(hash)` or `Passthrough`.

Context methods:

- `get_or_create_mutex(name)`: a per-run `tokio::Mutex` per `options.mutex` name.
- `get_target_prefix(target)`: the target padded to the longest primary target id, for output
  prefixes.
- `is_primary_target(target)`, `is_dependency_ignored(target, dep)`.
- `should_always_run(target)`: the target is in `cleanup_targets` or `wait_targets`, so its command
  must run instead of being hydrated (another task depends on it running after it, or alongside it).
- `should_stop(target)`: `cancel_token` is cancelled, **or** `abort_token` is cancelled and the
  target isn't a cleanup target. Cleanup tasks still run after an abort.
- `should_inherit_args(target)`: `passthrough_args` is non-empty, and the target is primary, or an
  initial locator is a qualified `:task` with the same task id.
- `set_target_state(target, state)` (sync insert, overwrites) and `get_target_states()` (a snapshot
  map).

### `ExecutionPlan`

[exec_plan.rs](../crates/exec-plan/src/exec_plan.rs). The declarative input of `moon exec`, which
`run`, `ci`, and `check` build from their arguments. Every field is optional and falls back to the
CLI argument:

- `affected { base, head, depth, source, status[], stdin }`
- `graph { dependents: Depth, no_dependencies }`
- `pipeline { ci, concurrency, ignore_ci_checks, no_actions, on_failure: bail | continue, job, job_total }`
- `targets`: `Included([locators])`, `Filtered { include }`, or `Partitioned { jobs: [[locators]] }`
  (requires `job` and `job_total`, and the job count must match).

`Depth` ([graph-utils/depth.rs](../crates/graph-utils/src/depth.rs)) is `Limit(n)` (`NONE` is `0`,
`DIRECT` is `1`) or `Deep`.

## Pipeline inputs from the graph

The pipeline reads these from the `ActionGraph`
([action_graph.rs](../crates/action-graph/src/action_graph.rs)):

| API                                           | Used for                                                                                                                            |
| :-------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------- |
| `get_node_count()`, `get_nodes()`             | The results channel capacity, the total to wait for, and `PipelineStarted`.                                                         |
| `sort_topological()`                          | Dispatch order. Reverses petgraph's toposort so dependencies come first. Fails with `CycleDetected` naming the cycle (`A → B → C`). |
| `group_priorities(indices)`                   | Buckets the topological order by `get_priority()` into a `BTreeMap<u8, Vec<NodeIndex>>` (`0` first). Non-task nodes are all `0`.    |
| `get_inner_graph()`                           | The dispatcher reads edges directly.                                                                                                |
| `get_cleanup_indices()`, `is_cleanup_index()` | Nodes that run after the nodes they depend on (the source of a `Cleanup` edge).                                                     |
| `get_layout_graph()`                          | Shared with jobs through `JobContext`, so the task hasher reads the same dependencies roots the graph placed.                       |
| `get_node_from_index()`                       | The node to run for a dispatched index.                                                                                             |

Edge weights are `TaskDependencyType`: `Required` (and `Optional`, treated identically), `Wait`, and
`Cleanup`. An edge `A → B` means A runs after B. See [Relations](#relations) for how the dispatcher
interprets them.

## Pipeline

[action_pipeline.rs](../crates/action-pipeline/src/action_pipeline.rs).

### Configuration

| Field         | Default           | Set by (`run_action_pipeline` in [helpers.rs](../crates/app/src/helpers.rs))                                                          |
| :------------ | :---------------- | :------------------------------------------------------------------------------------------------------------------------------------ |
| `concurrency` | `num_cpus::get()` | `--concurrency`.                                                                                                                      |
| `bail`        | `false`           | `true` for `moon run` and `moon check`; `on_failure == bail` for `moon exec`; `false` for `moon ci`, `setup`, `sync`, `docker setup`. |
| `quiet`       | `false`           | Disables the console subscriber.                                                                                                      |
| `report_name` | `runReport.json`  | `ciReport.json` for `moon ci`.                                                                                                        |
| `summary`     | `None`            | The command's summary level; `Level::Two` for `setup` and `sync`.                                                                     |

`bail` means "abort the pipeline when any action fails that isn't allowed to", instead of only on
hard failures. See [Aborting](#aborting).

### Run flow

`run_with_context(graph, context)`:

1. Store the context, and register subscribers (see [Subscribers](#subscribers)).
2. Emit `PipelineStarted { actions_count, action_nodes, context }`.
3. `internal_run(graph)`, then take the collected actions.
4. Emit `PipelineCompleted { actions, context, duration, error, error_report, status }`, with the
   error when the run failed, and return the actions (or the error). The event is emitted on both
   paths, so subscribers always see the end of a run that started.

`internal_run`:

1. **Empty graph.** Zero nodes returns `Ok` immediately: no status change (stays `Pending`), no
   duration, no dispatcher, no signal monitor.
2. **Channels.** A bounded `mpsc` of `Action` results with capacity `node_count`, and an unbounded
   `mpsc` of `CompletedJob` for the dispatcher.
3. **Contexts.** A shared `AbortState`, and a `JobContext` (below) whose `abort_token` and
   `cancel_token` are the action context's, so tasks can check them.
4. **Signal monitor** (see [Signals](#signals)), spawned before dispatching.
5. **Dispatcher task** (see [Dispatch loop](#dispatch-loop)), which returns the job handles.
6. **Result loop** (see [Receiving results](#receiving-results)).
7. **Shutdown.** `abort_state.handle(false, None, None)` so a dispatcher waiting on the abort is
   released; capture the signal if cancelled (which sets `received_signal` and the status), else
   abort the monitor; `ProcessRegistry::wait_for_running_to_shutdown()`; then, unless the status is
   `Completed`, stop the remaining jobs: with `bail` the dispatcher task is aborted outright,
   otherwise it's awaited and its standard and persistent `JoinSet`s are shut down.
8. Store the actions and the duration. Return the first hard error, if any.

### `JobContext`

[job_context.rs](../crates/action-pipeline/src/job_context.rs). Cloned into every job.

| Field             | Purpose                                                                                                                        |
| :---------------- | :----------------------------------------------------------------------------------------------------------------------------- |
| `abort_token`     | Cancelled when an action aborts the pipeline. Jobs that haven't started finish `Aborted`.                                      |
| `cancel_token`    | Cancelled by the signal monitor.                                                                                               |
| `abort_state`     | See [AbortState](#abortstate).                                                                                                 |
| `bail`            | The pipeline's `bail`.                                                                                                         |
| `completed_queue` | Sends `CompletedJob { index, started, failed, executed }` to the dispatcher.                                                   |
| `result_sender`   | Sends the finished `Action` to the pipeline.                                                                                   |
| `running_jobs`    | `RwLock<FxHashMap<NodeIndex, u64>>` of dispatched standard jobs by exclusive id (see [Exclusive targets](#exclusive-targets)). |
| `semaphore`       | `concurrency` permits.                                                                                                         |
| `emitter`         | The event emitter.                                                                                                             |
| `daemon_client`   | Optional daemon connection, passed to the task runner.                                                                         |
| `layout_graph`    | From the graph.                                                                                                                |
| `workspace_graph` | Project and task lookups.                                                                                                      |

Methods:

- `is_aborted_or_cancelled()`: either token is cancelled.
- `should_abort(action)`: `action.should_abort() || (bail && action.should_bail())`.
- `send_result(action)`: the only path a finished job takes, with a fixed order:
  1. If `should_abort(action)`, cancel `abort_token` **first**, so a dependent that the dispatcher
     would release on this completion sees the abort before it starts.
  2. Build the `CompletedJob`: `started = started_at.is_some()`, `failed = has_failed()`,
     `executed = operations.has_executed_task()`.
  3. Send the action to the pipeline **before** marking the job complete, as the dispatcher stops
     once nothing is running, and the pipeline then stops receiving.
  4. `mark_completed_job`: remove from `running_jobs`, then send to the completed queue (a send
     error means the dispatcher has stopped, and is ignored).
- `mark_completed(index, started)`: a completion without a result (`failed: false`,
  `executed: false`), used for persistent jobs at dispatch and for missing nodes.

### Dispatch loop

`dispatch_jobs` sorts and groups the graph, then spawns one task that owns the `JobDispatcher` and
two `JoinSet`s (`standard` and `persistent`, never awaited together, as persistent jobs don't
complete). A `DrainedGuard` marks the abort state as drained when the task ends for any reason, so
the pipeline never waits on a drain that won't happen.

While `dispatcher.has_queued_jobs()` (not every node has been dispatched):

1. **Stop** if aborted or cancelled: break, and skip the exhaust step.
2. **Next.** `dispatcher.next()` (see [Job dispatcher](#job-dispatcher)). `None` means everything
   left is blocked on a running job, so wait for **one** standard job to finish
   (`standard.join_next()`), then loop. Only one, as some running jobs may only complete once a
   not-yet-dispatched job has run (a cleanup that stops what they wait on).
3. **Missing node** (should never happen): warn, `mark_completed(index, false)` so the loop doesn't
   hang, and continue.
4. **Needless cleanup.** A cleanup node that `is_needless_cleanup` (nothing to clean up after, and
   nothing depends on it), and that isn't a primary target, is not run: an `Action` with status
   `Skipped` (never started) is sent as its result.
5. **Pre-skip.** If the node is persistent or has wait dependents, and its task will be skipped
   because a dependency failed or was skipped (`will_skip_task`, which is
   `is_task_dependencies_complete` returning `Ok(false)`; an error means "not known", so not
   skipped), set its `TargetState::Skipped` now and `mark_never_started` on the dispatcher. This is
   done before dispatch because other jobs don't wait for this one to complete.
6. **Persistent.** `mark_completed(index, !will_be_skipped)` immediately (otherwise dependents that
   are persistent, or only wait on it, would never dispatch); set `TargetState::Passthrough` unless
   it will be skipped (so a persistent dependent doesn't error on a missing hash); spawn into
   `persistent` **without a permit** (they'd exhaust the pool).
7. **Has wait dependents.** Spawn into `standard` **without a permit**, as the dependents must be
   able to acquire one to run alongside it, and never in isolation.
8. **Standard.** Spawn `dispatch_job_with_permit` into `standard`. For a non-interactive node,
   continue immediately.
9. **Interactive.** Run in isolation: wait for this one job (through a oneshot, which also returns
   if the job panics) before dispatching anything else. Then re-check abort/cancel and stop.

After the loop, unless stopped: `exhaust_job_handles(standard)` waits for every standard job (and
detaches the set). Then, if the abort token is cancelled and the graph has cleanups, run
[`drain_cleanup_jobs`](#cleanup-after-an-abort). Return the handles.

`dispatch_job_with_permit`: a cleanup job dispatched after an abort first waits
`abort_state.wait_until_resumable(cancel_token)` **without** holding a permit. Then acquire a
permit, run the job, release.

### Job

[job.rs](../crates/action-pipeline/src/job.rs). `Job::dispatch`:

1. Create `Action::new(node)` with `node_index`.
2. `aborted_before_start = abort_token.is_cancelled()`. A cleanup job in that case waits
   `wait_until_resumable` (running processes must be terminated first, or its own would be too).
3. **Not started, aborted** (not a cleanup, and abort token cancelled): finish `Aborted`, send, and
   return. Cleanup jobs are never aborted this way, only cancelled.
4. **Not started, cancelled** (signal): finish with `status_when_signal_cancels(started = false)`,
   which is `Skipped`, send, return.
5. `run_action(...)` (boxed; never inside a `tokio::select!`, as cancelling the future would kill
   the child process and skip signal propagation). An `Err` from the runner itself (an emitter or
   graph lookup failure) finishes the action `Failed`.
6. **After running:**
   - If aborted (not a cleanup, and the abort token is cancelled), or a cleanup that was running
     when the abort happened (`!aborted_before_start`) and failed: `mark_casualty(node_index)` and
     finish `Aborted`. The job was terminated by the pipeline, not by its own fault.
   - Else if cancelled: finish `status_when_signal_cancels(started_at.is_some())`: `Failed` when the
     command was in flight, `Skipped` when it never started.
7. `send_result(action)`.

### Action runner

[action_runner.rs](../crates/action-pipeline/src/action_runner.rs). `run_action`:

1. `action.start()`, emit `ActionStarted { action, node }`.
2. Dispatch on the node variant, emitting the pair of events around the handler:

   | Node                  | Events                                                             | Handler                                                                   |
   | :-------------------- | :----------------------------------------------------------------- | :------------------------------------------------------------------------ |
   | `None`                | none                                                               | returns `Skipped`                                                         |
   | `SyncWorkspace`       | `WorkspaceSyncing` / `WorkspaceSynced { error }`                   | `sync_workspace`                                                          |
   | `SyncProject`         | `ProjectSyncing { project }` / `ProjectSynced`                     | `sync_project` (project looked up first; a lookup error fails the action) |
   | `SetupEnvironment`    | `EnvironmentInitializing { project, root, toolchain }` / `..ed`    | `setup_environment`                                                       |
   | `SetupProto`          | none                                                               | `setup_proto`                                                             |
   | `SetupToolchain`      | `ToolchainInstalling { spec }` / `ToolchainInstalled`              | `setup_toolchain`                                                         |
   | `InstallDependencies` | `DependenciesInstalling { project, root, toolchain }` / `..ed`     | `install_dependencies`                                                    |
   | `RunTask`             | `TaskRunning { node, target }` / `TaskRan { error, node, target }` | `run_task`                                                                |

   The `..ed` events carry `error: Option<String>` from the handler's result.

3. `Ok(status)`: `action.finish(status)`. `Err(error)`: `action.finish(Failed)`,
   `action.fail(error)`. Emit `ActionCompleted { action, error, error_report, node }` either way.
4. If the action failed and the node is `SetupProto`, `SetupToolchain`, `SetupEnvironment`, or
   `InstallDependencies`: `action.abort()`. Provisioning failures poison everything downstream
   (dependents dispatch on completion, not success), so they abort the pipeline regardless of
   `bail`.

### Receiving results

The pipeline loop receives one `Action` at a time:

1. **Abort, once.** If `job_context.should_abort(action)` and not yet aborted:
   `abort_state.mark_terminated()`, `ProcessRegistry::terminate_running()` (broadcasts `Terminate`
   to every child, like a signal), cancel `abort_token`, then
   `abort_state.handle(drain, resume_at, give_up_at)` with:
   - `drain` = the graph has cleanups **and** no signal was received (cleanups run after a failure,
     not after a signal);
   - `resume_at` = now + `threshold` + 250ms, `give_up_at` = now + 2 × `threshold` + 250ms, where
     `threshold` is the process registry's kill threshold (`pipeline.killProcessThreshold`, default
     2000ms); both `None` when the threshold is `0` (processes run to completion).
2. **First error.** If `action.should_abort()`, it has an error, no error is recorded yet, and the
   action isn't a casualty: record `action.get_error()`. The first hard failure is the root cause;
   casualties (jobs terminated because a sibling failed) may arrive earlier and are never the
   reported error.
3. Push the action.
4. **Status.**
   - Aborted: status `Aborted`. If `abort_state.should_drain()`, switch to **draining** (keep
     receiving until the cleanups are done), else close the receiver (remaining senders fail, and
     the loop ends once buffered results are drained).
   - Else cancelled: status `Interrupted` (replaced by the signal's status after the loop), close
     the receiver.
   - Else all `node_count` results received: status `Completed`, break.

While draining, `select!` (biased) prefers a result, then `wait_until_drained` or `cancelled`, both
of which close the receiver and take the last buffered result.

### Aborting

Two independent mechanisms stop a run:

| Mechanism | Trigger                                                                                                                          | Token          | Status                                   | Cleanups run? |
| :-------- | :------------------------------------------------------------------------------------------------------------------------------- | :------------- | :--------------------------------------- | :------------ |
| Abort     | A provisioning action failed; a task failed before its command spawned; or, with `bail`, any action failed that isn't `allow`ed. | `abort_token`  | `Aborted`                                | yes           |
| Cancel    | `SIGINT`, `SIGTERM`, `SIGHUP`, `SIGQUIT` (or Windows equivalents).                                                               | `cancel_token` | `Interrupted` / `Terminated` / `Aborted` | no            |

Without `bail`, a failed task does not abort: its dependents are dispatched, see the `Failed` target
state, and skip themselves; the run ends `Completed` with failed actions, and the command exits
non-zero (see [Exit codes](#exit-codes)).

#### `AbortState`

[abort_state.rs](../crates/action-pipeline/src/abort_state.rs). Shared by the pipeline, dispatcher,
and jobs:

| Member                                        | Semantics                                                                                                                                                                                        |
| :-------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `handle(drain, resume_at, give_up_at)`        | Only the first call has an effect. Stores the three values and cancels `handled`.                                                                                                                |
| `should_drain()`                              | Handled and `drain` is true.                                                                                                                                                                     |
| `wait_until_handled()`                        | Awaits the `handled` token.                                                                                                                                                                      |
| `wait_until_resumable(cancel)`                | Awaits `handled`, then sleeps until `resume_at` if set. Returns early if `cancel` is cancelled. New processes started before `resume_at` would be force killed by the registry's shutdown sweep. |
| `get_give_up_at()`                            | When to stop waiting on jobs that were running at the abort (they should have been killed by then).                                                                                              |
| `mark_drained()` / `wait_until_drained()`     | Set by the `DrainedGuard` when the dispatcher task ends.                                                                                                                                         |
| `mark_casualty(index)` / `is_casualty(index)` | Jobs aborted because of the pipeline, not themselves.                                                                                                                                            |
| `mark_terminated()` / `take_terminated()`     | A flag that the pipeline terminated processes itself, consumed once by the signal monitor.                                                                                                       |

### Signals

`monitor_signals` subscribes to `ProcessRegistry::receive_signal()` **before** spawning, so no
signal is missed, and loops:

- A `Terminate` while `take_terminated()` is true is the pipeline's own termination broadcast (from
  `terminate_running`), not an external signal: keep listening.
- Any other signal: cancel `cancel_token`, return the signal.
- `Lagged`: continue. `Closed`: return `Interrupt`.

After the loop, if cancelled, the pipeline awaits the monitor (it returns right after cancelling, so
this doesn't block) and sets `received_signal` and the status: `Interrupt` → `Interrupted`; `Hangup`
and `Terminate` → `Terminated` (the terminal closed, or an external stop, not a pipeline failure);
anything else → `Aborted`. `signal_exit_code()` is `128 + signal number` (`130` for `SIGINT`, `143`
for `SIGTERM`, `129` for `SIGHUP`, `131` for `SIGQUIT`), which the command sets as the process exit
code, overriding any task's exit code.

Running tasks observe the same broadcast through their child processes (the registry forwards the
signal, and force kills after the threshold), and through `has_received_signal` in the executor's
retry loop.

## Job dispatcher

[job_dispatcher.rs](../crates/action-pipeline/src/job_dispatcher.rs). Decides which node runs next.
Everything it needs is extracted from the graph once, in `new`, so dispatching never traverses the
graph.

### Relations

For every node, its outgoing edges are merged per target node into a
`Relation { required, wait, cleanup }` (`Required` and `Optional` set `required`; several edges
between the same pair are merged; self edges are dropped). The dependency is **released on
dispatch** instead of on completion only when the relation is `wait` alone
(`wait && !required && !cleanup`).

Per node: `dependencies: Vec<(NodeIndex, Relation)>` (in the graph's edge order, for determinism),
`dependents` (nodes released when this one completes), `wait_dependents` (released when this one is
dispatched), and `blocked_by` (the count of unreleased dependencies). `ready_nodes` counts nodes
with `blocked_by == 0` that haven't been dispatched.

Tracking sets: `visited` (dispatched), `completed`, `started` (completed and its action started),
`executed` (completed and ran its task command), `failed` (completed and failed or aborted),
`never_started` (dispatched but will skip, see pre-skip), `released` (its wait dependents were
unblocked).

### `next()`

1. `drain_completed_jobs()`: for each queued `CompletedJob`, the **first** completion of an index
   counts (a persistent job is marked at dispatch and again when it exits): record it in
   `completed`, and in `started`/`executed`/`failed` per its flags; release its wait dependents (a
   no-op if already released at dispatch); decrement `blocked_by` of its dependents, incrementing
   `ready_nodes` for each that reaches `0`.
2. If `ready_nodes == 0`: prune dispatched indices from the groups, return `None`.
3. `find_next_index()`: for each priority group (ascending) and each index in topological order:
   - `find_applicable_index`: skip if already traversed in this pass, visited, or completed. Return
     the index when `blocked_by == 0`. Otherwise, for groups `0` and `1` (critical, high) only,
     recurse into its dependencies looking for one that can run, so that a high-priority task's
     chain is started early. A `traversed` set avoids re-walking blocked subgraphs within a pass.
   - Skip a cleanup node once the abort token is cancelled (cleanups are then taken separately).
   - [Exclusive targets](#exclusive-targets) check.
   - Return the index.
4. On a hit: insert into `visited`, decrement `ready_nodes`, release its wait dependents, return it.
   On a miss: prune and return `None`.

`has_queued_jobs()` is `visited.len() < node_count`. `has_wait_dependents(index)`.

#### Exclusive targets

A node with `get_id() > 0` (a `RunTask`) that `is_standard()` (not interactive or persistent) is not
dispatched while `running_jobs` contains another index with the same id; the dispatcher logs and
moves on to the next candidate. Otherwise it records `(index, id)` in `running_jobs`, which
`mark_completed_job` removes. So two instances of the same target (different dependency args or env)
never run at once, but a persistent or interactive instance isn't subject to this.

### Cleanup after an abort

`drain_cleanup_jobs` (pipeline) runs once the dispatch loop has stopped and the abort token is
cancelled:

1. Wait for `abort_state` to be handled (or the cancel token). Return if cleanups shouldn't drain (a
   signal), or the run is cancelled.
2. Loop while not cancelled: `give_up` = `give_up_at` has passed. For each index in
   `take_cleanups_after_abort(give_up)`, spawn it (without a permit when it has wait dependents,
   else with). Break when `!is_waiting_on_cleanups(give_up)` or nothing is running. Otherwise wait
   for a standard job to complete, the cancel token, or the give-up deadline.

`take_cleanups_after_abort(give_up)` drains completions, then considers the pending cleanups (not
yet visited, sorted), and repeatedly takes those that are **needed** and **can run**, marking them
visited and releasing their wait dependents. When none can be taken, pending cleanups that relate to
nothing unfinished (everything they relate to has completed, or will never run) are dropped as dead,
and the loop retries; it stops when nothing is dead either. Like normal dispatch, two cleanups with
the same exclusive id don't run together (the second is deferred to a later call).

- **Needed**: it cleans up after a job that has something to clean up (`cleans_up_after_something`:
  a `cleanup` relation to a job in `executed`, or to a job that started a dependency it only waits
  on, which is still running or started and not `never_started`), **or** a needed pending cleanup
  requires or waits on it (transitively).
- **Can run** (`can_cleanup_after_abort`), for each dependency:
  - `required`: it must have started and completed, and if it isn't a task it must not have failed
    (only task dependencies are checked by the task itself when it runs);
  - `cleanup`: it must have completed, or be finished for good (not running, not another pending
    cleanup; when giving up, a running non-cleanup counts as finished);
  - `wait` only: it must have started, or still be running.
- `is_waiting_on_cleanups(give_up)`: a visited cleanup hasn't completed, or a pending cleanup has a
  running dependency (when giving up, only a running cleanup counts).

### Needless cleanups (normal dispatch)

`is_needless_cleanup(index)` is checked right after a cleanup node is dispatched normally (all its
dependencies have completed): it's needless when nothing requires or waits on it, and none of the
jobs it cleans up after has something to clean up (they were skipped, cached, or their wait
dependency never started). The pipeline then skips it, unless it's a primary target. A needless
cleanup that's skipped doesn't count as executed, so a cleanup of that cleanup is needless too.

## Action handlers

[actions/src/actions/](../crates/actions/src/actions). Every handler has the signature
`(action, action_context, app_context, [workspace_graph], node) -> Result<ActionStatus>`, records
its operations on the action, and returns the action's status. An `Err` fails the action.

### Shared utilities

[utils.rs](../crates/actions/src/utils.rs):

- **Skip env vars.** `should_skip_action(key)` matches the variable against `true`;
  `should_skip_action_matching(key, pattern)` matches it against an id (or `id:version` target). A
  value matches when it's `*`, `*:*`, `1`, `true`, equals the pattern, or (comma-separated, any part
  matching) for `id:version` patterns, `id`, `id:*`, `*:version`. Returns the value for logging.
- **Hashing.** `create_hasher(action, data)` pushes a passed `HashGeneration` operation with the
  hash of `data` (hasher label is the action prefix). `create_hash_and_return_lock(action, data)`
  additionally acquires a file lock named `<prefix>-<hash>` in the cache dir, so concurrent moon
  processes wait on an in-progress action.
  `create_hash_and_return_lock_if_changed(action, fingerprint, should_force)` does the same, then
  returns `None` when the digest already exists as a hash fingerprint in the objects storage (the
  action ran before with the same inputs) and `should_force()` is false. The check runs **after**
  locking.
- **`HashLock::persist_hash_fingerprint(storage)`** stores the fingerprint. It's deliberately not
  stored up front, so an action that fails or is killed leaves nothing claiming it succeeded. Every
  fingerprinted handler persists only at its very end.

[plugins/commands.rs](../crates/actions/src/plugins/commands.rs), for `ExecCommand`s returned by
plugins:

- `exec_plugin_commands(toolchain_id, commands, options)`: serial commands first, in order, then
  parallel ones concurrently. Every resulting operation gets `plugin = toolchain_id`.
- `exec_plugin_command(command, options)`: `1 + retry_count` attempts; stops at the first attempt
  whose operation is `Skipped` or exited `0`; an `Err` on the last attempt propagates. Each attempt
  is a `ProcessExecution` operation (command name as the label).
- Per attempt, by `command.cache`: `None` and `Memory` run the command; `Hash` runs it through
  `cache_engine.execute_if_changed(<prefix>:<label>, fingerprint)`, where the fingerprint is the
  label, the command input, and the `inputs` (env var values; file hashes via the VCS when enabled,
  else sizes; file sizes; file timestamps). Paths outside the workspace are warned about and
  ignored; missing files are ignored. A skipped run (unchanged) finishes the operation `Skipped`.
- The command is an `AugmentedCommand::from_input`, inherits plugin augmentations for the options'
  project (see [Command augmentation](#command-augmentation)), uses the input's `cwd` or the
  options' `working_dir`, caches process output in memory when `cache == Memory`, errors on non-zero
  unless `allow_failure`, prints the command when `pipeline.logRunningCommand`, inherits colors and
  proto, and holds a file lock `<prefix>-<cache key>` so commands that clobber shared state (rustup,
  cargo) don't collide. `on_exec` prints a `Setup` checkpoint with the label, and `attempt N of M`
  when retrying. Streams output when `command.stream`, else captures.

[plugins/operations.rs](../crates/actions/src/plugins/operations.rs):

- `convert_plugin_operation(plugin, op)`: a `SyncOperation` with `id` = the plugin operation id in
  kebab-case, `plugin`, its timestamps and duration, and status `Pending → Running`, `Failed`,
  `Passed`.
- `finalize_action_operations(action, plugin, op, plugin_ops, changed_files)`: sets `plugin` on the
  overall setup operation, finishes it `Passed` if still running, appends the converted plugin
  operations, records the changed files on it, and pushes it last.
- `finalize_sync_operation(result)`: the plugin call's own operation converted, with the output's
  operations nested under it, and its changed files.

### `SyncWorkspace`

[sync_workspace.rs](../crates/actions/src/actions/sync_workspace.rs).

1. Lock `sync-workspace`. Skip (`Skipped`) when `MOON_SKIP_SYNC_WORKSPACE` matches `true`.
2. Run these concurrently (a `JoinSet`), each as a tracked `SyncOperation`:

   | Operation id     | When                            | Runs                                                                                                                                                                                                                                | Passed when         |
   | :--------------- | :------------------------------ | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------ |
   | `config-schemas` | always                          | `sync_config_schemas`: generates JSON schemas (and Pkl schemas when `pkl` is on `PATH`) into `.moon/cache/schemas`, through `execute_if_changed("config-schemas", { files_exist, moon_version })`. Failures are warned, not errors. | it returned `true`  |
   | `codeowners`     | `codeowners.sync`               | `sync_codeowners`: a `CODEOWNERS` file from workspace and project `owners`, sorted by `codeowners.orderBy`; generated when the file is missing, or when the fingerprint changed.                                                    | a path was returned |
   | `vcs-hooks`      | `vcs.sync`                      | `sync_vcs_hooks`: generates hook scripts when they're missing or the fingerprint of `vcs.hooks` changed.                                                                                                                            | it returned `true`  |
   | per toolchain    | any toolchain plugin configured | `sync_workspace` of every toolchain, each finalized as a sync operation.                                                                                                                                                            | per plugin          |
   | per extension    | any extension plugin configured | `sync_workspace` of every extension.                                                                                                                                                                                                | per plugin          |

3. In CI, if any operation changed files: warn, and return `Invalid` (the changes should have been
   committed). Otherwise `Passed`.

### `SyncProject`

[sync_project.rs](../crates/actions/src/actions/sync_project.rs).

1. Load the project **with tasks**, and save its snapshot
   (`cache_engine.state.save_project_snapshot`) for tasks to reference through
   `MOON_PROJECT_SNAPSHOT`. This happens before the skip check.
2. Skip when `MOON_SKIP_SYNC_PROJECT` matches the project id.
3. Lock `sync-project-<id>`.
4. Collect the project's dependencies as fragments (with their scope).
5. Call `sync_project` on every toolchain **enabled for the project**, with the project fragment,
   its dependencies, the merged toolchain config (workspace config overridden by the project's), and
   the workspace config; then on every extension. Each result is a finalized sync operation.
6. In CI with changed files: `Invalid`. Else `Passed`.

### `SetupProto`

[setup_proto.rs](../crates/actions/src/actions/setup_proto.rs). Records no operations.

1. Set `PROTO_VERSION`, `PROTO_IGNORE_MIGRATE_WARNING=true`, `PROTO_VERSION_CHECK=false`, and
   `PROTO_LOOKUP_DIR=<inventory>/proto/<version>` on the global env bag, so lookups use this
   version.
2. `Skipped` when the install dir exists, or under `MOON_TEST`.
3. `Skipped` when proto isn't required (`requires_proto`: no toolchain has a version, or
   `MOON_TOOLCHAIN_FORCE_GLOBALS` applies).
4. Offline: `Skipped` if any `proto` binary exists in the store's bin dir (used even if another
   version), else error `RequiredInternetConnection`.
5. Lock `proto-install`, print a `Setup` checkpoint, load the built-in proto tool, resolve the
   version, `Skipped` if already installed, else install with prompts and UI disabled. `Passed` when
   a record was installed, `Skipped` otherwise.

### `SetupToolchain`

[setup_toolchain.rs](../crates/actions/src/actions/setup_toolchain.rs).

1. `Skipped` when the spec has no version (global), or `MOON_TOOLCHAIN_FORCE_GLOBALS` applies to the
   toolchain.
2. `Skipped` when `MOON_SKIP_SETUP_TOOLCHAIN` matches `<id>` or `<id>:<version>`.
3. Load the plugin; `Skipped` unless it supports tier 3.
4. `create_hash_and_return_lock` with fingerprint `{ action_node, installed_in_proto }`. This
   **always** runs the setup (no "unchanged" short-circuit): the plugin decides whether anything is
   installed.
5. Align the toolchain config's `version` with the spec being set up (a project may override the
   workspace version), then call `setup_toolchain` with `configured_version`, printing an
   `installing <label>` checkpoint through the callback, and a proto text reporter.
6. `finalize_action_operations` with the output's operations and changed files; persist the
   fingerprint. `Passed` when `output.installed`, else `Skipped`.

### `SetupEnvironment`

[setup_environment.rs](../crates/actions/src/actions/setup_environment.rs).

1. `Skipped` when `MOON_SKIP_SETUP_ENVIRONMENT` matches the toolchain id.
2. Load the plugin; `Skipped` unless it exports `setup_environment`.
3. Build the input: the root (as a virtual path), the project fragment when `project_id` is set, and
   the toolchain config merged with the config project's (`project_id`, else `owner_project_id`,
   else the workspace's). Call `setup_environment` to get the commands and operations to run.
4. `create_hash_and_return_lock_if_changed` with fingerprint
   `{ action_node, project, toolchain_config, commands, operations }`: the plugin's **output** is
   part of the fingerprint, so the environment is set up again only when what it would do changed.
   Unchanged → `Skipped`.
5. `exec_plugin_commands` in the root. `finalize_action_operations` with the output operations and
   changed files. Persist. `Skipped` when there were no commands and no operations, else `Passed`.

### `InstallDependencies`

[install_dependencies.rs](../crates/actions/src/actions/install_dependencies.rs). Gates, in order,
each returning `Skipped`:

1. `MOON_SKIP_INSTALL_DEPS` matches the toolchain id.
2. Offline (warned).
3. The cache is write-only (the user is force-updating the cache).
4. Running with `--affected` **outside** CI (installs interrupt the affected workflow and VCS
   hooks).
5. The plugin doesn't export `install_dependencies`.
6. In CI, and the vendor directory (`metadata.vendor_dir_name`) exists in the root and is non-empty
   (dependencies were installed before moon ran).

Then:

7. Build the input (root, project fragment when `project_id`, merged config of the config project).
   `first_install` = the plugin has a vendor dir name and the vendor dir is empty or missing
   (determined before anything runs).
8. `create_hash_and_return_lock_if_changed` with fingerprint
   `{ action_node, lockfile_hash, manifest_dependencies, manifest_paths, project, toolchain_config }`,
   forced when `first_install`:
   - `lockfile_hash`: the VCS hash of the first `metadata.lock_file_names` found in the root.
   - For each `metadata.manifest_file_names` that appears in the run's **changed files**: walk the
     root for manifests (by the `members` globs, excluding the vendor dir), always include the root
     manifest, parse each with `parse_manifest` concurrently, and record every dependency
     `name → versions` (all scopes) and the manifest paths. Unchanged manifests contribute nothing.
     Unchanged → `Skipped`.
9. Call `install_dependencies` for the install and dedupe commands. Run the install command (cache
   disabled, streamed unless `MOON_TEST_HIDE_INSTALL_OUTPUT`). Run the dedupe command only outside
   CI, and not on a first install (deduping a fresh install would rewrite a pristine lockfile).
10. `finalize_action_operations` (no changed files), persist. `Skipped` when the output had no
    install command, no dedupe command, and no operations; else `Passed`.

### `RunTask`

[run_task.rs](../crates/actions/src/actions/run_task.rs).

1. Look up the project and task (errors fail the action).
2. Set `action.allow_failure` and `action.expect_failure` from `options.onFailure` **before**
   running, so they hold even when an error bubbles up.
3. `TaskRunner::new(...).run(context, node)`.
4. `action.flaky = operations.is_flaky()`, `action.status = operations.get_final_status()`,
   `action.operations = operations`.
5. A failed action that's allowed to fail logs a warning.
6. Return the runner's error (`Err`), else `Ok(action.status)`. The action runner then overwrites
   the status: `finish(status)` on `Ok`, `finish(Failed)` on `Err`.

## Task runner

[task_runner.rs](../crates/task-runner/src/task_runner.rs). One `TaskRunner` per `RunTask` action,
holding the project, task, layout graph, the `TaskRunCacheState` cache item, an `OutputArchiver`, an
`OutputHydrater`, the `TaskRunState`, the `OperationList`, and a `TaskReportItem` for the console.

### State

`TaskRunCacheState` (`.moon/cache/states/<project>/<task>/lastRun.json`, with `tag-<tag>`
directories for tag-scoped targets): `exit_code`, `hash`, `last_run_time` (millis),
`output_globs_hash`, `target`. Loaded in `new`; `last_run_time` is updated and the item saved after
**every** run, success or failure.

`TaskRunState` (in-memory): `digest` (the task's fingerprint digest), `operation` (the last
execution operation, for the manifest), `target` (the final `TargetState`), and four cache flags:
`local_cache_readable/writable` = cache engine readable/writable and `options.cache` enables local;
`remote_cache_readable/writable` = the same plus `options.cache` enables remote and a remote backend
is enabled.

`is_cache_enabled()` = `options.cache` is enabled **and** the VCS is enabled (no VCS root, as in
some containers, disables caching rather than failing).

### `run`

1. Set the report's `output_prefix` (the padded target), `output_style` (the context's override,
   else the task's), and `primary`.
2. `internal_run`:
   1. If `!is_dependencies_complete` → `skip()` and return no hash.
   2. `hash()` always, even without caching.
   3. With caching enabled: if `context.should_always_run(target)` log and fall through (never
      hydrate a cleanup or wait target); else if `hydrate(hash)` returns `true`, return the hash.
      Else `execute()`, then `archive(hash)`.
   4. Without caching: `execute()`.
3. Save the cache item (`last_run_time`).
4. On `Ok`: set the target state to `state.target`, else `Passthrough`; set the report hash;
   `console.on_task_completed(target, operations, report, None)`; return `{ hash, operations }`.
5. On `Err`: set the target state to `state.target`, else `Failed`; `inject_failed_task_execution`;
   `console.on_task_completed(..., Some(error))`; return `{ error, operations }` (no hash).

### Dependency completeness

`is_task_dependencies_complete(task, context)`, also used by the dispatcher's pre-skip:

- No deps → `true`.
- Per dep: skip `cleanup` deps (they run after) and ignored deps (`--no-dependencies`). If the dep
  has a target state: complete → continue; else → `false` (the task will be skipped). If it has no
  state: a `wait` dep may still be running → continue; otherwise error `MissingDependencyHash` (the
  dependency hasn't run, which is a scheduling bug or misconfiguration).

### Hashing

`hash(context, node)` creates a `ContentHasher` labelled with the node label, and a `HashGeneration`
operation, then hashes three parts, stores the fingerprint in the objects storage
(`store_hash_fingerprint_with_hasher`, which yields the `Digest`), sets the report hash and
`state.digest`, and pushes the operation (`Passed`).

1. **Common contents** ([task_hashing.rs](../crates/task-hasher/src/task_hashing.rs)
   `hash_common_task_contents`), a `TaskFingerprint` of: `cacheKey`, `command`, `args`, `deps`,
   `env`, `envOverride` keys, `inputs` (file → hash), `input_env`, `outputs`, the project's
   `dependsOn` ids, `script`, `target`, `toolchains`, and a `version` constant to bump to invalidate
   every cache. Plus:
   - the passthrough args, when the task has no script and `should_inherit_args`;
   - the node's dependency `args` and `env`;
   - `deps`: only `required`-type deps contribute. An ignored dep contributes `passthrough`.
     Otherwise the value depends on the dep's `cache_strategy` and its target state
     (`dep_hash_input`): `Hash` (or unset) → the dep's hash for `Passed`/`SkippedConditional`,
     `passthrough` for `Passthrough`, nothing for `Failed`/`Skipped`; `Outputs` → `outputs`,
     `outputs-passthrough`, or nothing (invalidation comes from the dep's outputs being this task's
     inputs); `Ignored` → `ignored` always.
2. **Toolchain contents** (`hash_toolchain_task_contents`): for each toolchain enabled for the task,
   concurrently, a `TaskToolchainFingerprint { toolchain, version, contents, dependencies }`,
   included only when something was recorded: the resolved version (project override, else
   workspace); the dependencies of the project's manifest(s) in the task's dependencies root
   (located from the **layout graph**: the root the project was placed in, or the root it located
   without being a member), resolved against the lock file (parsed only with
   `hasher.optimization: accuracy`) by satisfying version then matching requirement, recording the
   lock hash, version, or meta, else the requirement string; inherited (`workspace:`) deps resolve
   through the root manifest; and the plugin's `hash_task_contents` output. Sorted by toolchain id.
3. **Fingerprint checks** (`hash_checks`): runs the task's `fingerprint` checks (see
   [Checks](#checks)); a hard error aborts; a failing check errors `FingerprintCheckFailed`; each
   check's output is filtered by its `hash` setting (`true` all, `false` none, `exitCode`, `stderr`,
   `stdout`) and hashed under the script.

### Cache check

`is_cached(hash) -> Option<HydrateFrom>`:

1. Parse `options.cacheLifetime` if set. A run is **stale** when a lifetime is set, there's a
   previous run time, and it's older than the lifetime.
2. **Previous outputs**: if the last run's `exit_code == 0`, its `hash` equals this hash, and
   `has_previous_outputs_been_created(output_globs_hash)` (every non-optional output file exists,
   and the hash of the files currently matched by output globs plus the optional files that exist
   equals the recorded one): stale → `None`, else `PreviousOutput` (reuse what's on disk, no
   hydration).
3. Cache not readable → `None`. Previous run failed (`exit_code > 0`) → `None`. Stale → `None`.
4. With a valid digest, `actions_storage.load_task_manifest(digest)` with the local/remote readable
   options: `Some(source)` → `Storage(source)`; `None` → miss; an error is **warned** and treated as
   a miss.

### Hydration

`hydrate(hash) -> bool`, with an `OutputHydration` operation:

- Not cached: operation `Skipped`, return `false`.
- From storage: clear the cached `hash` first, as hydrating deletes and replaces outputs, and may
  fail part way (the outputs would no longer be the previous hash's).
- `hydrater.hydrate(from, hash, state)`
  ([output_hydrater.rs](../crates/task-runner/src/output_hydrater.rs)):
  - `PreviousOutput` → `Hit` immediately.
  - `Storage`: `Skipped` when neither cache is readable. Validate every manifest file (with a
    digest) and symlink path is a declared output (a declared file, under a declared directory, or
    matching an output glob), else error `OutputFileNotDeclared`. Try the daemon's
    `hydrate_task_outputs` first (an RPC failure is warned and falls back in-process); in-process,
    `hydrate_task_manifest`, and only after all reads succeed **delete the existing outputs** and
    unpack. `HitFromStorage(manifest, is_remote)` or `Missed`.
- Outcome → operation: `Skipped` → `Skipped`; `Missed` → `Invalid`; `Hit` → `Cached`, with the
  command line, the cached exit code, and the `stdout.log`/`stderr.log` from the state dir;
  `HitFromStorage` → `Cached` or `CachedFromRemote`, with the exit code and logs from the manifest,
  and `persist_state(operation)`.
- When hydrated: record the output globs (unless from previous outputs), set the cached `hash`, and
  `state.target = Passed(hash)`.

### Execution

`execute(context, node)`:

1. **Noop task** → `skip_no_op`: a passed `NoOperation`, `state.target = from_hash(report.hash)`.
2. **Stopped** (`context.should_stop(target)`): the pipeline aborted or was interrupted while this
   task prepared → `skip_stopped`: a skipped `TaskExecution`, `state.target = Skipped`.
3. **Checks** (`execute_checks`, see [Checks](#checks)): a hard error aborts; a failing
   `requirement` errors `RequirementCheckFailed`; if there are `condition` checks and **all** pass →
   `skip_conditional`: a skipped `TaskExecution`, `state.target = SkippedConditional(hash)` (or
   `passthrough`); otherwise continue.
4. Build the command (see [Command builder](#command-builder)) and a `TaskExecutor`.
5. **Mutex**: with `options.mutex`, push a `MutexAcquisition` operation after acquiring the named
   mutex (held for the execution), and re-check `should_stop` once acquired → `skip_stopped`.
6. Execute. `persist_state(last attempt)` when there was an execution attempt. Merge the attempts
   into the operations. `state.target = run_state`.
7. The executor's error (a process that couldn't spawn), if any.
8. If the last attempt failed: with `onFailure: expect`, a code that isn't an expected failure
   errors `UnexpectedPass` (exit `0`) or `UnexpectedFailure` (126, 127, a signal, or abnormal);
   otherwise `RunFailed` wrapping `ExitNonZero` with the status and code.

#### Task executor

[task_executor.rs](../crates/task-runner/src/task_executor.rs). `attempt_total = retry_count + 1`.
`interactive`/`persistent` are the node's or the task's flags.

- **Output mode** (`prepare_state`): `stream` is the resolved output style (`context.output_style`,
  else `options.outputStyle`) being `stream`, or, without a style, "primary target and not CI".
  Interactive → `exec_stream_output` (inherits stdio); stream → `exec_stream_and_capture_output`;
  else `exec_capture_output`. The output prefix is removed when there's a single primary target and
  the task is persistent or has no deps. The report gets `attempt_total`, `output_streamed`,
  `primary`.
- A **running status** monitor prints `running for Ns` every 30s for non-persistent, non-interactive
  tasks.
- Per attempt: a `TaskExecution` operation with the command line; `MOON_TASK_RETRY_ATTEMPT` and
  `MOON_TASK_RETRY_TOTAL` env; `console.on_task_started`; print the command line when
  `pipeline.logRunningCommand` (with the workspace root and working dir); run under a timeout token
  from `options.timeout` seconds (biased `select!`: a timeout finishes the attempt `TimedOut`).
- On output: `finish_from_output`. With `onFailure: expect`, success is instead "the exit code is an
  expected failure (1–255 except 126/127 and 129–192) and the pipeline isn't stopping", and an exit
  `0` is an **unexpected pass**. `console.on_task_finished`, push the attempt.
  - Success: `run_state = from_hash(report.hash)`, stop.
  - Unexpected pass: stop without retrying (another attempt would only hide it if it failed).
  - Attempts left: retry, **unless** a signal was received (`has_received_signal`: anything on the
    registry broadcast, including the pipeline's own termination) or `should_stop`.
  - Out of attempts: stop. `run_state` stays `Failed`.
- On a spawn error: the attempt finishes `Failed`, `console.on_task_finished(error)`, return the
  error.

### Command builder

[command_builder.rs](../crates/task-runner/src/command_builder.rs). `build(context, node, hash)`, in
this order:

1. `AugmentedCommand::from_task` (the task's command, args, env, and shell settings), then
   `inherit_from_plugins(project, task)` (see [Command augmentation](#command-augmentation)).
2. `cwd` = the project root, or the workspace root with `options.runFromWorkspaceRoot`. Non-zero
   exits don't error (the runner handles them).
3. **Args**: the node's dependency `args` first, then the passthrough args when the task has no
   `script`, `should_inherit_args`, and there are any.
4. **Env**:
   - moon variables: `MOON_CACHE_DIR`, `MOON_PROJECT_ID`, `MOON_PROJECT_ROOT`,
     `MOON_PROJECT_SOURCE`, `MOON_PROJECT_SNAPSHOT`, `MOON_TASK_ID`, `MOON_TASK_HASH`,
     `MOON_TARGET`, `MOON_WORKSPACE_ROOT`, `MOON_WORKING_DIR`, `PWD` (the cwd). Applied last; they
     can't be overridden.
   - `options.envFile`: each file in order (missing files skipped; `*.local` files skipped in CI),
     resolving references against the system env, previous files, the task's `env`, and the moon
     variables. A value applies when the task set the key to `null` (opt out of system inheritance
     only), or when the command doesn't already have the key and the system doesn't either.
   - The node's dependency `env`, with the task's `envOverride` behavior per key.
5. **Shell**: `options.shell: true` selects `unixShell`/`windowsShell`; otherwise no shell.
6. **Affected files** (`options.affectedFiles`): the task's affected files from the changed files
   when running `--affected` (else none), falling back to the task's input files when
   `passInputsWhenNoMatch`; filtered to the project unless `ignoreProjectBoundary`, by `filter`
   globs, sorted, made relative to the cwd. As `MOON_AFFECTED_FILES` (OS path list; `.` or empty
   when none, per `passDotWhenNoResults`) and/or as args (`./`-prefixed, quoted when containing
   `* $ + [ ]`; `.` when none).
7. `inherit_colors` when `pipeline.inheritColorsForPipedTasks`.
8. `inherit_proto` last.

`build_check(check)` builds a check's command: `from_task_check`, `inherit_from_toolchains` only (no
plugin command extensions), the same cwd, shell always on, colors, proto.

#### Command augmentation

[augmented_command.rs](../crates/process-augment/src/augmented_command.rs).
`inherit_from_plugins(project, task)`:

1. `extend_command` on the task's toolchains (merged config) and every extension: args, env, paths.
2. With a project and task: for a `script`, clear the args and apply `extend_task_script` outputs
   (toolchains, then extensions); otherwise `extend_task_command` outputs.
3. `inherit_from_toolchains`: for each toolchain with a resolved version (project override, else
   workspace; none when forced global), set its version env var; unless forced to globals, apply
   `activate_environment` env (without overriding existing keys) and paths, in configured toolchain
   order, then the toolchains' command paths.

`inherit_proto` sets `PROTO_AUTO_INSTALL=false`, `PROTO_IGNORE_MIGRATE_WARNING=true`,
`PROTO_NO_PROGRESS=true`, `PROTO_VERSION`, `STARBASE_FORCE_TTY=true`, and adds the proto and moon
paths unless forced to globals.

### Checks

[checks_runner.rs](../crates/task-runner/src/checks_runner.rs) and
[check_executor.rs](../crates/task-runner/src/check_executor.rs). `ChecksRunner::execute(types)`
filters `task.checks` by type (`Condition`, `Requirement`, `Fingerprint`), builds each command, and
runs them **concurrently**. Each is a `ProcessExecution` operation with the command line, captured
output, the task's `timeout` (→ `TimedOut`), and `finish_from_output`. A spawn error is returned as
the result's `error` (the operation `Failed`), which the runner treats as a hard error.

### Archiving

`archive(hash)`: skipped (`false`, no operation) when the target state is skipped. Otherwise an
`ArchiveCreation` operation, `archiver.archive(hash, state)`
([output_archiver.rs](../crates/task-runner/src/output_archiver.rs)):

- A build-type task (`type: build`, or has outputs) whose outputs weren't all created errors
  `MissingOutputs`: every non-optional output file exists, and when there are output globs that can
  match (not all negated), they match at least one file unless every glob output is optional.
- Neither cache writable → `Skipped`.
- Build the `TaskManifest` on a blocking thread: every output file (including non-globs), the last
  execution operation (exit code, logs), and the source fingerprint blob from the objects storage
  when present.
- Try the daemon's `archive_task_outputs` (an RPC failure is warned and falls back in-process), else
  `archive_task_manifest` with the local/remote writable options. → `Queued`.

Then `record_output_globs` (the current `output_globs_hash`) and push the operation (`Skipped` or
`Passed`).

### Persisted state

`persist_state(operation)` sets `state.operation`, and when it has process output: `exit_code`,
`hash` (the digest when caching is enabled, else empty: the hash is recorded only once the task has
a result for it, so outputs on disk aren't attributed to a hash that never ran), and writes
`stdout.log` and `stderr.log` into the target's state dir. Without process output, the logs from a
previous run are removed.

### Skips and the injected failure

| Method               | Operation                             | Target state                                |
| :------------------- | :------------------------------------ | :------------------------------------------ |
| `skip()`             | `TaskExecution`, `Skipped`, no timing | `Skipped`                                   |
| `skip_stopped()`     | `TaskExecution`, `Skipped`            | `Skipped`                                   |
| `skip_conditional()` | `TaskExecution`, `Skipped`            | `SkippedConditional(hash or "passthrough")` |
| `skip_no_op()`       | `NoOperation`, `Passed`               | `from_hash(hash)`                           |

**Injected failure** (`inject_failed_task_execution`): when `run` fails and no `TaskExecution`
exists (the failure happened before the command spawned: hashing, a check, the command build, or a
plugin), a `TaskExecution` with exit code `-1` and status **`Aborted`** is pushed last, and reported
through `console.on_task_finished`. `Action::should_abort()` reads this, so such a failure aborts
the pipeline even without `bail`.

### Errors

[task_runner_error.rs](../crates/task-runner/src/task_runner_error.rs): `RunFailed`,
`UnexpectedPass`, `UnexpectedFailure`, `FingerprintCheckFailed`, `RequirementCheckFailed`,
`MissingDependencyHash`, `MissingOutputs`, `OutputSymlinkOutsideOfWorkspace`,
`OutputFileOutsideOfWorkspace`, `OutputFileNotDeclared`.

## Events and subscribers

[event_emitter.rs](../crates/action-pipeline/src/event_emitter.rs). `Event<'data>` borrows the
pipeline's data; `get_type()` is the webhook event name:

| Event                     | Type                       | Payload                                                             |
| :------------------------ | :------------------------- | :------------------------------------------------------------------ |
| `PipelineStarted`         | `pipeline.started`         | `actions_count`, `action_nodes`, `context`                          |
| `PipelineCompleted`       | `pipeline.completed`       | `actions`, `context`, `duration`, `error`, `error_report`, `status` |
| `ActionStarted`           | `action.started`           | `action`, `node`                                                    |
| `ActionCompleted`         | `action.completed`         | `action`, `error`, `error_report`, `node`                           |
| `DependenciesInstalling`  | `dependencies.installing`  | `project`, `root`, `toolchain`                                      |
| `DependenciesInstalled`   | `dependencies.installed`   | `error`, `project`, `root`, `toolchain`                             |
| `EnvironmentInitializing` | `environment.initializing` | `project`, `root`, `toolchain`                                      |
| `EnvironmentInitialized`  | `environment.initialized`  | `error`, `project`, `root`, `toolchain`                             |
| `ProjectSyncing`          | `project.syncing`          | `project`                                                           |
| `ProjectSynced`           | `project.synced`           | `error`, `project`                                                  |
| `TaskRunning`             | `task.running`             | `node`, `target`                                                    |
| `TaskRan`                 | `task.ran`                 | `error`, `node`, `target`                                           |
| `ToolchainInstalling`     | `toolchain.installing`     | `spec`                                                              |
| `ToolchainInstalled`      | `toolchain.installed`      | `error`, `spec`                                                     |
| `WorkspaceSyncing`        | `workspace.syncing`        | none                                                                |
| `WorkspaceSynced`         | `workspace.synced`         | `error`                                                             |

`EventEmitter::emit` holds a mutex over the subscribers and calls each `Subscriber::on_emit` in
registration order, sequentially; a subscriber error aborts the emit and propagates to the caller
(an action fails, or the pipeline run errors). Events from concurrent jobs are therefore serialized.

### Subscribers

Registered by `setup_subscribers`, in this order:

| Subscriber                | Registered when                                                                         | Behavior                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| :------------------------ | :-------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ConsoleSubscriber`       | `!quiet`                                                                                | Forwards `PipelineStarted`, `PipelineCompleted` (with a `PipelineReportItem { duration, summary, status }`), `ActionStarted`, `ActionCompleted` to the console reporter.                                                                                                                                                                                                                                                                                          |
| `ReportsSubscriber`       | always                                                                                  | On `PipelineCompleted` with a duration: writes `RunReport { actions, context, duration, comparison_estimate, status }` as `<report_name>` in the cache dir.                                                                                                                                                                                                                                                                                                       |
| `WebhooksSubscriber`      | `notifier.webhookUrl` set, and a remote (CI, SSH, devbox, headless) or test environment | Posts every event (by type) through `WebhooksNotifier` (via the daemon when connected), honoring `notifier.webhookAcknowledge`; waits for in-flight requests on `PipelineCompleted`.                                                                                                                                                                                                                                                                              |
| `NotificationsSubscriber` | `notifier.terminalNotifications` set                                                    | Terminal toasts: on `PipelineCompleted`, a success toast (`always`/`success`) when no error, a failure toast (`always`/`failure`) titled by status when there is one; on `TaskRan` with an error (`task-failure`). ANSI escapes are stripped. `never` disables.                                                                                                                                                                                                   |
| `CleanupSubscriber`       | `pipeline.autoCleanCache`                                                               | On `PipelineCompleted` with status `Completed` only: `clean_cache(cacheLifetime)` through the daemon, else `clean_stale_cache` in-process.                                                                                                                                                                                                                                                                                                                        |
| `MetricsSubscriber`       | `--otel` (`otel_enabled`)                                                               | On `ActionCompleted`: `moon.action.executions` (counter) and `moon.action.duration` (ms histogram) with `action`, `status`, and a `target`/`toolchain`/`project` dimension; `moon.operation.executions`/`duration` per operation (nested included) with `action`, `operation`, `status`, `id`, `plugin` (never the target, to bound cardinality); `moon.task.runs`/`duration` for tasks with `target`, `project`, `task`, sorted `toolchains`, `status`, `flaky`. |
| `TelemetrySubscriber`     | never (disabled; code retained)                                                         | Would track toolchain usage in CI on `PipelineStarted`.                                                                                                                                                                                                                                                                                                                                                                                                           |

### Run report and estimate

[reports/estimate.rs](../crates/action-pipeline/src/reports/estimate.rs). `Estimate::calculate`
estimates the time the run would have taken without moon: each action's duration (×10 when cached)
is bucketed by task id for `RunTask`, and summed as `install_duration` for `SetupToolchain` and
`InstallDependencies`; the comparison is the sum of 85% of each bucket (a rough parallelism
allowance) plus the install duration; `gain`/`loss` and `percent` compare it with the pipeline
duration; the install duration appears as task `*` with count `0`.

### Console hooks

The console reporter ([reporter.rs](../crates/console/src/reporter.rs)) is driven from two places:
the console subscriber (`on_pipeline_started`, `on_pipeline_completed`, `on_action_started`,
`on_action_completed`) and the task runner/executor directly (`on_task_started` per attempt,
`on_task_running` every 30s, `on_task_finished` per attempt, which prints captured output when not
streamed, and `on_task_completed`, which prints cached output and the final checkpoint).
`on_pipeline_completed` renders nothing for an empty run; for an aborted run it prints every failed
action's error unless a hard error was bubbled up (the app renders it); without a summary level it
prints stats; with one, `Review` (failures, level 3), `Summary` (level 2), and `Stats` (level 1).

## Exit codes

`run_action_pipeline` sets the session exit code to the signal code when a signal stopped the run.
The `exec` command then walks the results: any failed action that isn't `allow_failure` makes the
run fail. The exit code is, in priority: the signal code; the task's own exit code when a **single,
fully qualified** target was requested and it failed with a non-zero code; else `1`.

## Environment variables

Read by the pipeline and actions:

| Variable                                                         | Effect                                                                                |
| :--------------------------------------------------------------- | :------------------------------------------------------------------------------------ |
| `MOON_SKIP_SYNC_WORKSPACE`                                       | `SyncWorkspace` is skipped when `true`/`*`/`1`.                                       |
| `MOON_SKIP_SYNC_PROJECT`                                         | `SyncProject` skipped for matching project ids.                                       |
| `MOON_SKIP_SETUP_TOOLCHAIN`                                      | `SetupToolchain` skipped for matching `id` / `id:version`.                            |
| `MOON_SKIP_SETUP_ENVIRONMENT`                                    | `SetupEnvironment` skipped for matching toolchain ids.                                |
| `MOON_SKIP_INSTALL_DEPS`                                         | `InstallDependencies` skipped for matching toolchain ids.                             |
| `MOON_TOOLCHAIN_FORCE_GLOBALS`                                   | Skips toolchain setup, proto, and path/env injection; version env vars are still set. |
| `MOON_TEST_HIDE_INSTALL_OUTPUT`                                  | Install and dedupe commands capture instead of stream.                                |
| `MOON_PIPELINE_KILL_PROCESS_THRESHOLD`                           | `pipeline.killProcessThreshold`: the registry threshold used for abort timing.        |
| `MOON_PIPELINE_AUTO_CLEAN_CACHE`, `MOON_PIPELINE_CACHE_LIFETIME` | The cleanup subscriber's gate and lifetime.                                           |

Set for task processes: the `MOON_*` variables listed under [Command builder](#command-builder),
`MOON_TASK_RETRY_ATTEMPT`, `MOON_TASK_RETRY_TOTAL`, `MOON_AFFECTED_FILES`, the toolchain version
variables, and the `PROTO_*` variables from `inherit_proto`. `SetupProto` sets `PROTO_VERSION`,
`PROTO_IGNORE_MIGRATE_WARNING`, `PROTO_VERSION_CHECK`, and `PROTO_LOOKUP_DIR` on moon's own
environment.

## Daemon integration

When a daemon client is connected (`session.connect_to_daemon`), it's used as an optimization at
four points, each falling back to in-process work on an RPC failure (warned, never fatal): archiving
task outputs, hydrating task outputs, cleaning the stale cache, and sending webhooks.

## Entry points

| Command                                         | `bail`               | Report           | Summary          | Notes                                                                    |
| :---------------------------------------------- | :------------------- | :--------------- | :--------------- | :----------------------------------------------------------------------- |
| `moon run`                                      | `true`               | `runReport.json` | from `--summary` | `exec` with the arguments as an included plan.                           |
| `moon ci`                                       | `false`              | `ciReport.json`  | from `--summary` | CI checks always on; `skip_persistent` when it chose the targets itself. |
| `moon check`                                    | `true`               | `runReport.json` | from `--summary` | `skip_persistent` always.                                                |
| `moon exec`                                     | `on_failure == bail` | `runReport.json` | from `--summary` | Runs an `ExecutionPlan`.                                                 |
| `moon setup`, `moon sync`, `moon sync projects` | `false`              | `runReport.json` | `Level::Two`     |                                                                          |
| `moon docker setup`                             | `false`              | `runReport.json` | `None`           |                                                                          |

`exec` builds the graph with
`RunRequirements { ci, ci_check, no_dependencies, dependents, interactive, job, job_total, skip_affected: !affected || force, skip_persistent }`,
requests `sync_workspace` in CI, and runs `run_tasks_with_plan`. It then fills the context's
`initial_targets`, `output_style`, and `passthrough_args` before running the pipeline.

## Invariants

- Every node produces exactly one `Action` in the results, unless the run is stopped early (abort
  without drain, or a signal), in which case jobs never dispatched produce none. A run reports
  `Completed` only when the result count equals the node count.
- A job's result is sent before its completion is queued, and the abort token is cancelled before
  either, so a dependent never starts in a broken environment between the two.
- A dependency is released on completion regardless of its outcome, except `wait`-only relations
  (released on dispatch). Tasks protect themselves by checking dependency target states.
- A persistent node is "completed" at dispatch; its target state is `Passthrough` unless it's known
  to be skipped. A `Passthrough` dependency contributes `passthrough` to dependents' hashes.
- Standard instances of the same target never run concurrently.
- Cleanup tasks always run their command (never hydrated), run after the task they clean up
  including when it failed or the pipeline aborted, don't run after a signal, and don't run when
  there's nothing to clean up (unless requested).
- A task never starts its command (or another attempt) once the pipeline is aborted or interrupted,
  except a cleanup after an abort.
- Hash fingerprints for setup actions are persisted only after the action succeeds, under a file
  lock keyed by the hash, so concurrent moon processes serialize on the same action.
- A task's `hash` in its state file is only recorded once the task has a result for that hash.
- Provisioning failures abort the pipeline; task failures abort only with `bail`, or when the
  failure happened before the command spawned.

## Test coverage

Where the behaviors above are exercised:

- [action/tests](../crates/action/tests): node identity and labels, action predicates, operation
  tracking and statuses.
- [action-pipeline/src/job_dispatcher.rs](../crates/action-pipeline/src/job_dispatcher.rs) (unit):
  duplicate completions, dense graphs, wait release, cleanup dispatch, needless cleanups, and every
  after-abort cleanup rule.
- [action-pipeline/src/abort_state.rs](../crates/action-pipeline/src/abort_state.rs),
  [job.rs](../crates/action-pipeline/src/job.rs),
  [action_pipeline.rs](../crates/action-pipeline/src/action_pipeline.rs) (unit): handle-once,
  casualties, resumability, signal statuses and exit codes.
- [action-pipeline/tests/action_pipeline_test.rs](../crates/action-pipeline/tests/action_pipeline_test.rs):
  abort propagation, priorities, persistent tasks, scopes, `wait` and `cleanup` dependency types end
  to end, including bailing, termination, mutexes, and interactive deadlocks.
- [action-pipeline/tests/report_estimate_test.rs](../crates/action-pipeline/tests/report_estimate_test.rs),
  [metrics_subscriber.rs](../crates/action-pipeline/src/subscribers/metrics_subscriber.rs) (unit).
- [task-runner/tests](../crates/task-runner/tests): the runner's run paths, dependency completeness,
  hashing (including dependency types and passthrough args), cache checks (previous outputs, globs,
  optional files, lifetimes, local cache), hydration, archiving, checks, expected failures, stopped
  runs, the command builder, and the executor's retries and termination handling.
- [task-hasher/src/task_hashing.rs](../crates/task-hasher/src/task_hashing.rs) (unit): dependency
  hash inputs per cache strategy, lock file version matching.
- [actions/src/utils.rs](../crates/actions/src/utils.rs) (unit): skip patterns.

## Notes

Behaviors that follow from the code, and may be worth a second look:

1. **`TimedOut` is an operation status, not an action status, in practice.** The runner returns an
   error for a failed last attempt, and the action runner finishes an errored action as `Failed`.
   `ActionStatus::TimedOut` therefore appears on the `TaskExecution` operation, while the action
   reports `Failed`.
2. **`get_final_status` ignores setup operations.** `get_last_process` doesn't include
   `SetupOperation`, so an action whose only process operation is a setup would be `Invalid`. This
   is only called for `RunTask` actions today.
3. **No report for a run without a duration.** The reports subscriber requires a duration, which is
   `None` for an empty graph, and when the dispatcher fails before running (a cycle at sort time).
4. **A closed signal channel reports `Interrupt`.** The monitor returns `Interrupt` if the
   registry's broadcast closes, though the pipeline only reads it when the cancel token is
   cancelled.
5. **Skipped dependents still "complete".** Without `bail`, a failed task's dependents are
   dispatched and skip themselves; their `Skipped` executions don't count as executed, so their
   cleanups are needless.
6. **Dependency installs are skipped under `--affected` locally**, regardless of whether lockfiles
   changed, as they interrupt the affected and VCS hook workflows.
7. **Webhooks only fire in remote or test environments** (`is_remote()`: CI, SSH, devbox, or
   headless; or `is_test_env()`), even when `notifier.webhookUrl` is set.
8. **`SetupToolchain` has no unchanged short-circuit** (`create_hash_and_return_lock`, not
   `_if_changed`); the plugin is always asked to set up, and reports whether it installed.
9. **`SyncProject` snapshots before its skip gate**, so the project snapshot exists for tasks even
   when the sync is skipped by `MOON_SKIP_SYNC_PROJECT`.
10. **An interactive job's isolation only waits on itself**, not on every running job, so jobs
    dispatched before it may still be running while it has stdin.
11. **The exclusive-target check doesn't apply to persistent or interactive instances**, so a
    persistent task and a standard instance of the same target can run at once.
12. **`allow_failure` is only ever true for tasks**, so `bail` always aborts on a failed setup or
    sync action, and `should_bail` is irrelevant to them (provisioning aborts regardless; syncs
    abort only with `bail`).
