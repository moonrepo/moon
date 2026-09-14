# fix(docker): scaffold .gitignore into the skeleton

Fixes the issue described in `ISSUE.md`.

## Problem

The configs skeleton carries manifests, lockfiles and moon configuration, but no
ignore rules. The generated build stage starts from the base image and copies
only that skeleton, so a build that keeps the git directory reachable has
nothing for `--exclude-standard` to apply.

`moon docker setup` then installs dependencies into that same directory, and git
reports every installed file as untracked.

Measured in one such build of a large monorepo:

```
git status --porcelain --untracked-files --ignore-submodules -z

bytes=31141105        # 31.1 MB
entries=279856
index_files=49626     # tracked files in the index
```

Entries are 5.6 times the tracked file count. The excess is the vendor
directory, which the repository's own `.gitignore` excludes everywhere except
inside the scaffolded image.

Task hashing walks the changed-file set, so the cost lands on every task.

## Change

Include `.gitignore` the same way `moon.yml` is already included, as a glob
matched per copied directory:

```rust
globs.insert(".gitignore".into());
```

Each project contributes its own file at its own root, and the root workspace
contributes its own. Nested files are therefore covered without a recursive
pattern, so no full tree walk is added to a phase that already iterates every
project.

## Scope

This only changes builds where the git directory is reachable. The Docker guide
suggests excluding it, and with it excluded the VCS layer is disabled and
nothing changes.

It is reachable whenever a build keeps it deliberately, which
`hasher.walkStrategy: vcs`, the default, needs in order to discover inputs.

## Why a default rather than documentation

A user can already work around this:

```yaml
docker:
  scaffold:
    configsPhaseGlobs:
      - .gitignore
      - "**/.gitignore"
```

That the workaround is one line is an argument for changing the default. Without
it the image silently disagrees with the repository about which files are
tracked, nothing in the output says so, and the cost appears as slow hashing
rather than as a configuration problem.

Note that the workaround above uses a recursive pattern, because workspace-level
globs are the fallback for every project. The fix avoids that cost.

## Verification

`cargo check -p moon_app` passes.
