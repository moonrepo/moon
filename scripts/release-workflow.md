# Release workflow

This doc briefly outlines how we publish our `@moonrepo` npm packages, and indirectly the Rust
binary associated with it. It's based around Yarn's official
[release workflow](https://yarnpkg.com/features/release-workflow), but with additional steps and
requirements.

## Requiring version bumps in pull requests

Any change in a package (under `packages/`) requires a deferred version bump using Yarn. If no bump
has been defined, we have a CI check that will fail, forcing the author to bump it before it can be
merged. Bumping can be done by running the following command at the root:

```shell
yarn version:bump
```

This will spin up an interactive CLI in which to choose major/minor/patch for each affected package.

### Handling Rust changes

Since Rust code falls outside of the `packages/` directory, the version check above will not work.
We do however want the `@moonrepo/cli` and `@moonrepo/core-*` packages to be bumped when Rust code
changes, so we have an additional CI check to enforce it.

We also support another command for bumping these packages, which requires an explicit
major/minor/patch.

```shell
yarn version:bump:bin patch
```

## Releasing packages

Releasing is currently _not ideal_, but works for the time being. An administrator with push access
to master must run the following command from an up-to-date master branch.

```shell
yarn version:apply
```

This will apply all the deferred Yarn versions (found in `.yarn/versions`), add and commit changes,
and create a git tag for every affected package. This must then be pushed to upstream master.

```shell
git push origin master --tags
```

At this point, the actual "publishing to npm" is done through two GitHub workflows:

- [release.yml](https://github.com/moonrepo/moon/blob/master/.github/workflows/release.yml) -
  Publishes the `@moonrepo/cli` and `@moonrepo/core-*` packages. This is our most critical workflow,
  as it builds the Rust binary and copies it into the appropriate packages before publishing.
- [release-npm.yml](https://github.com/moonrepo/moon/blob/master/.github/workflows/release-npm.yml) -
  This workflow publishes all other npm packages.

Both of these workflows _must be manually triggered_ through GitHub's UI.

### Handling failed publishes

This hasn't happened yet, so nothing to document. At minimum, re-running the workflows should be our
first attempt at fixing the problem.
