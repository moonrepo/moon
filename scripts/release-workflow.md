# Release workflow

This doc briefly outlines how we publish our `@moonrepo` npm packages, and indirectly the Rust
binary associated with it. It's based around Yarn's official
[release workflow](https://yarnpkg.com/features/release-workflow), but with additional steps and
requirements.

## Define version bump in pull requests

Any change in a package (under `packages/`) requires a deferred version bump using Yarn. If no bump
has been defined, we have a CI check that will fail, forcing the developer to bump it before it can
be merged. Bumping can be done by running the following command at the root:

```shell
yarn version:bump
```

This will spin up an interactive CLI in which to choose major/minor/patch for each affected package.

### Handling Rust changes

Since Rust code falls outside of the `packages/` directory, the version check above will not work.
We do however want the cli/core packages to be bumped when Rust code change, so we have an
additional CI check to enforce it. We also support another command for bumping these packages:

```shell
yarn version:bump:cli
```
