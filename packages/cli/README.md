# @moonrepo/cli

The official CLI for [moon](https://moonrepo.dev), a build system and repo management tool for the
JavaScript ecosystem, written in Rust!

- [Documentation](https://moonrepo.dev/docs)
- [Getting started](https://moonrepo.dev/docs/install)

## Requirements

- Node.js >= 14.15

## Installation

moon can be installed with npm, pnpm, or yarn.

```bash
# Install the dependency
yarn add --dev @moonrepo/cli

# Initialize moon in the repo
npx @moonrepo/cli init
```

## Usage

Once [projects](https://moonrepo.dev/docs/create-project) and
[tasks](https://moonrepo.dev/docs/create-task) have been configured, you can run tasks with:

```bash
# Run `lint` in project `app`
moon run app:lint

# Run `lint` in all projects
moon run :lint
```
