# Niteo Documentation

Niteo is a standalone Rust CLI for structural linting in TypeScript projects. It checks project shape, module boundaries, source structure, and unsafe TypeScript patterns. It does not format code.

## Contents

- [CLI reference](./cli.md)
- [Configuration](./configuration.md)
- [Rules](./rules.md)
- [Reports and output formats](./reports.md)
- [Benchmarks](./benchmarks.md)
- [Baselines](./baselines.md)
- [Suppressions](./suppressions.md)
- [CI usage](./ci.md)

## Quick Start

Run Niteo directly:

```sh
npx niteo-cli lint
```

Or install it globally:

```sh
npm i -g niteo-cli
niteo lint
```

Create a config file:

```sh
niteo init
```

For an existing codebase, create a baseline before enabling CI:

```sh
niteo baseline create
```

## Core Commands

```sh
niteo lint              # Scan for structural issues
niteo lint --watch      # Re-lint on every file change
niteo init              # Create niteo.toml
niteo init --preset     # Create config from a named preset
niteo config check      # Validate config file for errors
niteo config print      # Print resolved config source
niteo baseline create   # Snapshot current violations
niteo baseline prune    # Remove fixed violations from the baseline
niteo rules             # List rules and configured severities
niteo rules --preset    # Show what a preset would configure
niteo explain <rule>    # Explain a rule
niteo stats             # Show import graph statistics
niteo graph             # Output the import graph
```

## Common Options

| Option                  | Description                                                      |
| ----------------------- | ---------------------------------------------------------------- |
| `--root <path>`         | Project root to scan.                                            |
| `--scope <path>`        | Limit scanning to one path.                                      |
| `--verbose`             | Show every violation in text output.                             |
| `--git`                 | Scan changed TypeScript files only. Fails if git is unavailable. |
| `--format <format>`     | Output format: `text`, `json`, `sarif`, or `ndjson`.             |
| `--output <path>`       | Write output to a file.                                          |
| `--baseline <path>`     | Baseline file path.                                              |
| `--report-suppressions` | Report suppressed violations and stale ignore directives.        |
| `--watch`               | Re-run lint on file changes.                                                                     |
| `--cache`               | Enable caching of analysis results.                                                              |
| `--no-cache`            | Disable caching.                                                                               |
| `--clear-cache`         | Clear the cache before running.                                                                  |

## What Niteo Checks

- TypeScript and TSX files
- default exports, barrel files, export stars, and mutable exports
- oversized files and directories
- tiny directories and deep directory nesting
- hook and component file boundaries
- `any`, enums, namespaces, non-null assertions, `eval`, `debugger`, and console usage
- test code or test imports in production files
- circular imports and import graph structure

Import graph rules (circular imports, orphan files, test imports, barrel chains) use your project's `tsconfig.json` path aliases when present. See [Configuration](./configuration.md#typescript-path-aliases) for details.

See [Rules](./rules.md) for the full rule catalog.

## Monorepos

Niteo supports cascading configs. Place a root `niteo.toml` at the workspace level and additional `niteo.toml` files inside packages. Child configs merge on top of the root, overriding only declared fields. See [Cascading Configs](./configuration.md#cascading-configs) for details.
