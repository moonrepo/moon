# `moon docker scaffold` omits `.gitignore`, so installed dependencies look untracked

## Precondition

This only bites when the git directory is reachable from the image. The Docker
guide suggests excluding it, and with it excluded the VCS layer is disabled and
nothing below applies.

It is reachable whenever a build keeps `.git` on purpose, which is what
`hasher.walkStrategy: vcs` (the default) and any use of `git` during the build
require. That combination is the case reported here.

## Summary

`moon docker scaffold` copies manifests, lockfiles and moon configuration into
the configs skeleton, but not `.gitignore`. The generated build stage starts
from the base image and copies only that skeleton, so it has no ignore rules.

`moon docker setup` then installs dependencies into that same directory. Git now
sees a vendor directory that nothing excludes, so `--exclude-standard` has
nothing to exclude with, and every installed file is reported as untracked.

## Evidence

Measured inside the build stage, after the sources copy and before
`moon run`, in a large pnpm monorepo:

```
git status --porcelain --untracked-files --ignore-submodules -z

bytes=31141105        # 31.1 MB of output
entries=279856        # records in that output
index_files=49626     # tracked files in the index
```

Entries are 5.6 times the tracked file count. The excess is `node_modules`,
which the repository's own `.gitignore` excludes everywhere except inside the
scaffolded image.

A single `git status` takes 4.2s in that state.

## Impact

Every task hash calls `get_changed_files`, so the inflated set is walked
repeatedly during a run. In the build measured here the hash phase dominated
the whole `moon run`, and the run phase of the image build was 781s with 175
tasks that were all cache hits.

The effect is not limited to hashing. Any VCS-backed file discovery inside the
image operates on the inflated set.

## Suggested fix

Include `.gitignore` in the configs phase by default, at the workspace root and
per project, in the same way manifests and lockfiles are already included. The
files are small, and copying them restores the ignore semantics the repository
has everywhere else.

A workaround exists for affected users today:

```yaml
docker:
  scaffold:
    configsPhaseGlobs:
      - .gitignore
      - "**/.gitignore"
```

That the workaround is a one-line config change is an argument for the default
rather than against the fix: without it, the image silently disagrees with the
repository about which files are tracked, and nothing in the output says so.

## Related

Separately, `get_changed_files` rebuilds its map on every call rather than once
per run, which is what turns a large changed-file set into a per-task cost.
Filed separately. The two compound.
