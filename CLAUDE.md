# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

### Rust Development
- `just build` - Build all Rust crates
- `just test` - Run tests with cargo-nextest
- `just test <name>` - Run specific test by name pattern
- `just lint` - Run clippy linter
- `just format` - Format code with rustfmt
- `just check` - Quick check compilation without building
- `just cov` - Generate code coverage report

### JavaScript/TypeScript Development
- `yarn install` - Install dependencies
- `yarn moon run <project>:build` - Build specific npm package
- `yarn moon run <project>:test` - Run tests for specific package
- `yarn moon run <project>:lint` - Lint specific package
- `yarn moon run <project>:typecheck` - Type check specific package
- `yarn moon run :lint` - Run lint in all projects
- `yarn moon run :test` - Run tests in all projects

### Release Commands
- `just bump <type>` - Bump version (patch/minor/major)
- `yarn version check --interactive` - Interactive version management

### Debugging
- `just mcp` - Run MCP inspector for debugging Model Context Protocol
- `just moon-check` - Run moon's self-check with trace logging

## Architecture Overview

moon is a Rust-based monorepo management tool with a modular crate architecture:

### Core Crates
- `cli` - Main entry point, command parsing with clap
- `app` - Application logic, session management, command handlers
- `action-graph` - DAG for task execution ordering
- `project-graph` - Project dependency management
- `task-runner` - Task execution engine with caching
- `workspace` - Workspace configuration and project discovery
- `cache` - Content-addressable caching system
- `config` - Configuration parsing and validation
- `plugin` - WASM-based plugin system
- `remote` - Remote caching via Bazel Remote Execution API
- `mcp` - Model Context Protocol support for AI agents
- `vcs` - Version control integration

### Key Concepts
- **Session-based architecture**: Components lazy-loaded via `MoonSession`
- **Graph-based execution**: Tasks executed in topological order
- **Multi-level caching**: Local and remote content-addressable caches
- **Plugin system**: Toolchain plugins (Proto-based) and extension plugins (WASM)
- **Event-driven**: Pipeline events for reporting, webhooks, and console output

### Testing Strategy
- Unit tests colocated with source files (`*.rs` → `#[cfg(test)]` modules)
- Integration tests in `tests/` directories
- Extensive fixtures in `/tests/fixtures/`
- Use `MOON_TEST=true` environment variable for test runs
- Coverage reports via `cargo llvm-cov`

### Configuration Files
- `moon.yml` - Project-level configuration
- `.moon/workspace.yml` - Workspace configuration
- `.moon/toolchain.yml` - Toolchain versions
- `.moon/tasks.yml` - Shared task definitions

### Development Tips
- moon uses itself for npm package management (dogfooding)
- Run `cargo run -- <command>` to test local moon binary
- Use `just` commands for common development tasks
- Enable trace logging with `--log trace` for debugging
- Check performance with `--summary` flag