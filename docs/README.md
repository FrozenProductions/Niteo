# Niteo Documentation

Niteo is a standalone Rust CLI for structural linting in TypeScript projects. It checks project shape, module boundaries, source structure, and unsafe TypeScript patterns. It does not format code.

## Contents

- [CLI reference](./cli.md)
- [Configuration](./configuration.md)
- [Rules](./rules.md)
- [Reports and output formats](./reports.md)
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
niteo init              # Create niteo.toml
niteo baseline create   # Snapshot current violations
niteo baseline prune    # Remove fixed violations from the baseline
niteo rules             # List rules and configured severities
niteo explain <rule>    # Explain a rule
niteo stats             # Show import graph statistics
niteo graph             # Output the import graph
```

## Common Options

| Option | Description |
| --- | --- |
| `--root <path>` | Project root to scan. |
| `--scope <path>` | Limit scanning to one path. |
| `--verbose` | Show every violation in text output. |
| `--git` | Scan changed TypeScript files only. |
| `--format <format>` | Output format: `text`, `json`, or `sarif`. |
| `--output <path>` | Write output to a file. |
| `--baseline <path>` | Baseline file path. |
| `--report-suppressions` | Report suppressed violations and stale ignore directives. |

## What Niteo Checks

- TypeScript and TSX files
- default exports, barrel files, export stars, and mutable exports
- oversized files and directories
- tiny directories and deep directory nesting
- hook and component file boundaries
- `any`, enums, namespaces, non-null assertions, `eval`, `debugger`, and console usage
- test code or test imports in production files

See [Rules](./rules.md) for the full rule catalog.

