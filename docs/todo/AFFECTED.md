# Affected tracking: status and gaps

An audit of how moon determines which projects and tasks are affected by a change, compared with
other build systems and task runners (Nx, Turborepo, Rush, Bazel, Buck2, Pants). Written on
2026-10-06 against `3.0-async-exp` at `4a2b28f854`, after the tracker performance work in
moonrepo/moon#2746. Line references are as of that commit, and will drift.

## Summary

moon is a "git diff → owning task → relations" system, like Nx and Turborepo. It's ahead of them in
three places:

- It selects individual tasks by their inputs, by default. Turborepo only offers this behind the
  `futureFlags.affectedUsingTaskInputs` flag, and Nx selects whole projects.
- Its cache hashes each project's resolved lockfile dependencies.
- It supports opt-in early cutoff, through `cacheStrategy: outputs`.

Its gaps are changes that a diff against HEAD's graph can't see (lockfiles, task definitions), and
fallbacks that behave inconsistently.

## What's supported

### Change detection

- Git only ([crates/vcs/src/git](../../crates/vcs/src/git)).
- Diffs from the merge-base of base and head. When no merge-base can be resolved (shallow clones,
  unfetched bases), it diffs against the base directly, with a warning
  ([git_client.rs:556](../../crates/vcs/src/git/git_client.rs#L556)).
- Base and head come from `--base`/`--head`, `--affected=<base>:<head>`, `MOON_BASE`/`MOON_HEAD`, or
  exec plans. In CI, the PR base and head are detected automatically through `ci_env`
  ([exec.rs:348](../../crates/app/src/commands/exec.rs#L348)). Otherwise the base defaults to
  `vcs.defaultBranch`.
- In CI on the default branch, it compares `HEAD~1` against `HEAD`
  ([changed_files.rs:108](../../crates/app/src/queries/changed_files.rs#L108)).
- Outside of CI, `--affected` only considers the working tree (staged, unstaged, and untracked
  files). In CI it compares the branch against the base. `--affected=local` and `--affected=remote`
  override this.
- `--status` filters by kind of change (added, deleted, modified, staged, unstaged, untracked).
- `--stdin` accepts a newline-separated list or JSON, so external tools can supply the changed files
  ([changed_files.rs:201](../../crates/app/src/queries/changed_files.rs#L201)).
- Diffs run with `--no-renames`, so a move is a delete plus an add, and both paths count
  ([tree.rs:140](../../crates/vcs/src/git/tree.rs#L140)).
- Submodules are diffed between their recorded commits.

### Matching changes

- Tasks are matched individually, by their own inputs:
  - files and globs (output globs act as exclusions)
  - file groups
  - inputs from other projects (`project://dep`, or `^` for dependencies), filterable by group or
    glob
  - file inputs that only match when the file's content matches a regex
  - environment variables, including `$VAR_*` globs
  - `implicitInputs` in `.moon/tasks/*`
    ([inherited_tasks_config.rs:318](../../crates/config/src/inherited_tasks_config.rs#L318))
- Tasks without configured inputs default to `**/*` in their project. Root-level tasks in a monorepo
  default to no inputs, and `inputs: []` means never affected
  ([tasks_builder.rs:490](../../crates/task-builder/src/tasks_builder.rs#L490)).
- Projects are matched by directory, through an index of project sources. A root-level project is
  affected by any file that isn't dot-prefixed.
- `runInCI` (`always`, `only`, `skip`, `false`) bypasses the checks in CI.

### Propagation

- Projects (`dependsOn`) and tasks (`deps`) have separate scopes: upstream and downstream, each
  `none`, `direct`, or `deep`.
- Relationships are walked in one breadth-first pass, starting from every directly affected task or
  project.
- Dependencies of affected tasks are marked affected too, as `--upstream` defaults to `deep` for
  exec-based commands. Other tools run dependencies because the pipeline needs them, without calling
  them affected.
- Every mark records its reason (file, env var, dependency, dependent, or always affected), which
  `moon query affected` reports. This is similar to bazel-diff's directly vs. indirectly impacted
  targets.
- Relation marks always apply to `moon query`, but only apply to exec-based commands with
  `--include-relations` (`-g`).

### Caching

- The task hash covers the command, args, env, input file contents, input env values, outputs,
  toolchains, and dependency hashes
  ([task_fingerprint.rs](../../crates/task-hasher/src/task_fingerprint.rs)).
- Toolchains hash each project's manifest dependencies, plus the resolved lockfile versions when
  `hasher.optimization` is `accuracy`
  ([task_hashing.rs:243](../../crates/task-hasher/src/task_hashing.rs#L243)).
- Early cutoff is opt-in per dependency. `cacheStrategy: outputs` hashes the dependency's outputs
  instead of its hash, and `ignored` makes the dependency an ordering-only edge
  ([task_config.rs:181](../../crates/config/src/task_config.rs#L181)). The default is `hash`, or
  `ignored` when the dependency has no outputs.

### Hand-off to other tools

- The `affectedFiles` task option passes the matched files to the command as args or env, with glob
  filters, a project boundary toggle, and a fallback to the task's inputs
  ([task_options_config.rs:73](../../crates/config/src/task_options_config.rs#L73)). Test runners
  can then do their own selection (e.g. `jest --findRelatedTests`).

## Gaps

Ordered by impact.

### 1. Lockfile-only changes select nothing

In a monorepo, a root lockfile isn't in any project's default inputs, and toolchains only use
lockfiles for hashing, Docker scaffolding, and toolchain detection
([toolchain_plugin.rs:314](../../crates/toolchain-plugin/src/toolchain_plugin.rs#L314)). An
`npm update` or `cargo update` that only touches the lockfile marks no tasks as affected, so
`moon ci` runs nothing. Manifest changes inside a project are still detected, as the manifest is a
project file.

This errs in the unsafe direction. Nx and Turborepo mark everything affected on any lockfile change
by default, and Nx can narrow that down with
`pluginsConfig["@nx/js"].projectsAffectedByDependencyUpdates: "auto"`.

The precise version is half built:

- `ManifestDepsInput` (`manifest://<toolchain>?deps=...`, "dependencies to compare against when
  determining affected status") exists with tests, but is commented out of the `Input` enum
  ([input.rs:173](../../crates/config/src/shapes/input.rs#L173),
  [input.rs:302](../../crates/config/src/shapes/input.rs#L302)).
- The hasher already parses each project's manifests and resolved lockfile dependencies, through the
  toolchain plugins.

Fix:

- [ ] Parse the lockfile at both the base (`git show <base>:<path>`) and head, through the
      toolchain's `parse_lock`.
- [ ] Mark the tasks (and projects) whose resolved dependencies differ.
- [ ] Wire up `ManifestDepsInput` for explicit control, or apply it implicitly per toolchain.
- [ ] Until then, consider marking everything affected on lockfile changes, like Nx and Turborepo.

### 2. `moon ci` doesn't run dependents by default

`moon ci` sets `--downstream=direct` "for regression checks"
([ci.rs:50](../../crates/app/src/commands/ci.rs#L50)), and
[guides/ci.mdx:22](../../website/docs/guides/ci.mdx#L22) says dependents run. But relation marks
only count with `-g` ([exec.rs:698](../../crates/app/src/commands/exec.rs#L698), checked by the
builder's `is_task_affected` at
[action_graph_builder.rs:2070](../../crates/action-graph/src/action_graph_builder.rs#L2070)). A
dependent therefore only runs when its own inputs changed.

Related: project-level relations (`dependsOn`, or Cargo path dependencies added by the Rust plugin)
mark dependent projects, but never cause their tasks to run (moonrepo/moon#2499). Only task `deps`
edges propagate to tasks.

Fix:

- [ ] Turn on relations in `moon ci` whenever downstream isn't `none`, or count scope-derived marks.
- [ ] Decide whether project-level relations should promote tasks, or document task `deps` (e.g.
      `^:build`) and `project://` inputs as the way to propagate.

### 3. Default branch pushes only diff `HEAD~1`

`ci_env`'s GitHub provider returns no base on push events, so moon falls back to comparing `HEAD~1`.
Pushes that contain several commits, and commits whose previous CI run failed, are under-selected.

Prior art: Nx's `nx-set-shas` action uses the SHA of the last successful workflow run, and GitHub's
push event payload (`$GITHUB_EVENT_PATH`) includes the pre-push `before` SHA.

Fix:

- [ ] Read the push event's `before` SHA where the provider exposes it.
- [ ] Document `MOON_BASE` with a last-green-commit strategy for other providers.

### 4. Fallbacks are inconsistent

- A shallow clone without a base makes exec commands disable affected filtering and run everything
  ([exec.rs:433](../../crates/app/src/commands/exec.rs#L433); the `CiNoShallowHistory` error there
  is commented out). But `moon query affected`, `moon query projects`, and `moon query tasks`
  receive an empty file list and report nothing affected
  ([changed_files.rs:51](../../crates/app/src/queries/changed_files.rs#L51)), so scripts that gate
  on query output silently skip all work.
- When the base or head commit is missing (GitLab merge trains with shallow fetches), `git diff`
  fails and aborts the whole run, as `exec_diff` has no recovery (moonrepo/moon#2642).
- Bare `--affected` picks local vs. remote mode from CI detection alone, but `--affected=true` and
  `MOON_AFFECTED=true` go through `moon_common::is_local()`, which also treats SSH, devboxes, and
  headless sessions as remote ([app_options.rs:128](../../crates/app/src/app_options.rs#L128)). In
  those environments, the two spellings compare against different things (working tree vs. base
  branch).

Fix:

- [ ] Pick one policy for missing history, ideally a loud warning plus treating everything as
      affected, and apply it to both exec and query commands.
- [ ] Catch diff failures and degrade the same way.
- [ ] Make every spelling of `--affected` choose local vs. remote the same way, and print which mode
      was chosen.

### 5. Task definition edits aren't changes

Changing a task's definition changes its hash, but doesn't select it. For example:

- editing an inherited task in `.moon/tasks/*`
- bumping a toolchain version in `.moon/toolchains.*`
- editing a project's `moon.*` when the task has explicit inputs

Dot-prefixed paths don't even trigger the root-level project rule.

Fix:

- [ ] Coarse: treat the config files that define a task as implicit inputs.
- [ ] Precise: compare task definitions between base and head, like bazel-diff and
      target-determinator do with target hashes. For example, record hashes without file inputs on
      the base commit's CI run, and diff against them.

### 6. Env var inputs are presence-based

A `$VAR` input counts as changed whenever the variable is set and non-empty
([concepts/affected.mdx](../../website/docs/concepts/affected.mdx)). A variable that's always set in
CI keeps the task permanently affected, and a change in value is never detected.

Fix:

- [ ] Compare against the input env values recorded on the last run (or on the base commit's run).

### 7. Larger bets (not planned)

- Observed or sandboxed inputs. Inputs are declared only, so an undeclared input goes stale
  silently. Prior art: BuildXL's observed file accesses, tup, and `go test`'s recorded file and env
  reads.
- Coverage-based or predictive test selection. Only possible today by handing files to the test
  runner with `affectedFiles`.
- Any VCS other than Git.

## Docs drift

- [ ] [concepts/affected.mdx:18](../../website/docs/concepts/affected.mdx#L18) says changes come
      from `git status`, and mentions Mercurial and Subversion. Only Git is implemented, and the
      diff is merge-base based.
- [ ] [guides/ci.mdx:22](../../website/docs/guides/ci.mdx#L22) says dependents run (see gap 2).

## Suggested order

1. Gaps 2 and 4: small, and the docs already promise the behavior.
2. Gap 1: the biggest correctness hole, and half built.
3. Gap 3: a CI environment change.
4. Gap 5: needs a design discussion.

## Verification

All findings come from reading the code at `4a2b28f854`. Gap 2 and the merge-train failure in gap 4
were reproduced while investigating moonrepo/moon#2198 and moonrepo/moon#2642, and their code paths
haven't changed since. The rest were not reproduced.
