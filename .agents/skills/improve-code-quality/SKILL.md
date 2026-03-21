---
name: improve-code-quality
description:
  Analyze and improve code quality for any path in the monorepo. Use whenever the user asks to
  improve, audit, review, clean up, refactor, lint, or check code quality for a directory or file.
  Also trigger for requests about security review, performance optimization, robustness checks,
  dependency audits, or readability improvements. Activate for any mention of code quality, code
  health, code smells, tech debt, or cleanup tasks targeting a specific path. Even if the user just
  says "check this code" or "review crates/foo", this skill applies.
---

# Improve Code Quality

Perform a structured code quality audit on a target path in the monorepo. The workflow is: run
automated tools, do deeper manual analysis, present a report, then apply fixes only after the user
approves.

## Usage

```
/improve-code-quality <path>
```

The path can be a directory (e.g., `crates/task-runner`), a single file (e.g.,
`crates/task-runner/src/lib.rs`), or the name of a package (e.g., `moon_task_runner`).

## Step 1: Validate and detect language

First, verify the target path exists. If it doesn't, tell the user and stop.

Then detect what languages are present:

- **Rust**: path is under `crates/`, `wasm/`, or `legacy/`, or contains `.rs` files or a
  `Cargo.toml`

Tell the user what you detected before continuing.

If the target is a single file, skip the automated tool phase and go straight to manual analysis.

## Step 2: Manual analysis

Read the source files in the target path and look for issues that automated tools miss. Organize
findings by category:

### Security

- `unsafe` blocks without justification comments
- `.unwrap()` on user-facing code paths (not tests — unwrap is fine in tests)
- Path traversal or injection risks
- Hardcoded secrets or credentials
- Use of `std::collections::HashMap` or `std::collections::HashSet` (refer to conventions below)

### Performance

- Unnecessary `.clone()` where borrowing would work
- Unnecessary `.collect()` in iterator chains
- O(n^2) or worse algorithms with better alternatives
- Blocking calls inside async functions
- Large types on the stack that should be `Box`ed
- Avoid cloning in loops; use `.iter()` instead of `.into_iter()` for `Copy` types
- Prefer iterators over manual loops (when applicable); avoid intermediate `.collect()` calls
- Detect memory leaks or dangling pointers

### Readability & structure

- Functions longer than ~80 lines that should be split
- Deeply nested or complex match/if-else chains
- Dead code or unused imports
- Inconsistent naming
- Magic numbers that should be named constants

### Robustness

- Missing error handling (bare `.unwrap()` in non-test production code)
- Incomplete pattern matches
- Missing input validation at boundaries
- Race conditions in concurrent code

# Borrowing & ownership

- Prefer `&T` over `.clone()` unless ownership transfer is required
- Small `Copy` types (≤24 bytes) can be passed by value
- Use `Cow<'_, T>` when ownership is ambiguous

### Dependencies

- Unused dependencies in `Cargo.toml`
- Overly broad feature flags on dependencies

## Step 3: Run automated tools

Run the appropriate tools from the repository root (without asking for permission if possible).
Capture all output, including errors.

Find the crate name by reading the `Cargo.toml` in the target directory (the `[package] name`
field). If the target is a subdirectory within a crate, walk up to find the crate root.

```bash
# Type check
cargo check -p <crate_name> 2>&1

# Linting
cargo clippy -p <crate_name> --all-targets -- -D warnings 2>&1

# Format check
cargo fmt -p <crate_name> --check 2>&1

# Testing
cargo nextest run -p <crate_name> --no-fail-fast -j 4 2>&1
```

For crates under `wasm/`, run from the `wasm/` directory instead.

### If a tool fails

If clippy can't run because of build errors, capture those errors — they become the highest priority
findings. Don't abort the rest of the analysis.

### Large scope warning

If the target is the repo root or a very broad path, warn the user it will take a while and suggest
narrowing scope.

## Step 4: Present the report

Show all findings in a structured report:

```
# Code quality report: <target_path>

**Language(s):** Rust
**Files analyzed:** <count>

---

## Automated tool results

### Checking
<summarize warnings/errors>

### Linting
<summarize warnings/errors with file:line references>

### Formatting
<list files with issues, or "All files properly formatted">

### Testing
<list failing tests>

---

## Manual analysis

### Critical — Security
| # | File:Line | Finding |
|---|-----------|---------|
| 1 | path/file.rs:42 | Description |

### High — Performance
| # | File:Line | Finding |
|---|-----------|---------|

### Medium — Readability & Structure
| # | File:Line | Finding |
|---|-----------|---------|

### Low — Robustness
| # | File:Line | Finding |
|---|-----------|---------|

### Info — Dependencies
| # | File:Line | Finding |
|---|-----------|---------|

---

## Summary

- Critical: <n> | High: <n> | Medium: <n> | Low: <n> | Info: <n>
- **Total: <n>**

## Recommended fix order
1. <highest priority>
2. ...
```

If a category has no findings, show the header with "No issues found."

Severity guide:

- **Critical**: Security vulnerabilities, correctness bugs, data loss risks
- **High**: Performance problems, error handling gaps with production impact
- **Medium**: Readability issues, structural problems, maintainability concerns
- **Low**: Minor style issues, documentation gaps
- **Info**: Dependency notes, suggestions, non-actionable observations

## Step 5: Apply fixes (after approval)

After showing the report, ask: **"Which findings should I fix? (all / critical+high / specific
numbers / skip)"**

Wait for the user's response. Do not apply anything without explicit approval.

### Fixing process

1. **Run auto-fix tools first:**
   - Linting: `cargo clippy -p <crate> --all-targets --fix --allow-dirty --allow-staged`
   - Formatting: `cargo fmt -p <crate> -- --emit=files`

2. **Apply manual fixes one at a time**, stating what changed and why for each.

3. **Re-run automated tools** to verify no regressions were introduced.

4. **Do not commit.** Tell the user the changes are ready for their review.

## Step 6: Validate best practices

After applying fixes, check that the target path adheres to best practices by running the
`/rust-skills` skill. If this skill does not exist, skip this step.

If you notice any new issues or deviations from conventions, add them to the report and ask the user
if they want to address those as well.

## moon specific conventions

Always enforce these project rules during analysis:

- No `std::collections::HashMap`/`HashSet` — use `rustc_hash::FxHashMap`/`FxHashSet`
  - Exception when using a third-party crate that requires `std` collections in its public API
    (e.g., `indexmap`), but even then internal code should prefer `FxHashMap`/`FxHashSet`
- Rust edition/toolchain idioms
  - Infer toolchain from `rust-toolchain.toml`
- Unix newlines for Rust files
- `cargo-nextest` for running tests (`cargo nextest`, not `cargo test`)
