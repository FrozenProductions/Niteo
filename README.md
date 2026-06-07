<p align="center">
  <img src=".github/public/Niteo.png" alt="Niteo" width="128" />
</p>

<h1 align="center">Niteo</h1>

<p align="center">
  <a href="https://www.npmjs.com/package/niteo-cli"><img src="https://img.shields.io/npm/v/niteo-cli" alt="npm version" /></a>
  <a href="https://github.com/FrozenProductions/Niteo/actions/workflows/rust-ci.yml"><img src="https://github.com/FrozenProductions/Niteo/actions/workflows/rust-ci.yml/badge.svg" alt="Rust CI" /></a>
  <a href="https://github.com/FrozenProductions/Niteo/actions/workflows/npm-release.yml"><img src="https://github.com/FrozenProductions/Niteo/actions/workflows/npm-release.yml/badge.svg" alt="npm release" /></a>
  <a href="https://github.com/FrozenProductions/Niteo/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="license" /></a>
  <a href="https://www.npmjs.com/package/niteo-cli"><img src="https://img.shields.io/npm/dm/niteo-cli" alt="npm downloads" /></a>
</p>

---

Niteo is a standalone Rust CLI for structural linting in TypeScript projects.

It checks project shape, module boundaries, and source structure rather than formatting. Niteo ships 54 opinionated rules, builds a full import graph, and runs as a single binary with no plugins to configure. It uses [oxc](https://github.com/oxc-project/oxc) for AST parsing.

## Why Niteo?

| Tool                   | Focus                                                                                                                                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **ESLint**             | Code-level linting: style, correctness, and patterns within individual files.                                                                                                                                |
| **Knip**               | Unused exports, files, and dependencies. Finds dead code.                                                                                                                                                    |
| **dependency-cruiser** | Dependency rules between modules. Validates import constraints.                                                                                                                                              |
| **Niteo**              | Opinionated architecture and structural conventions. Enforces how a project is shaped: file layout, export style, module boundaries, domain boundaries, and import hygiene — all in one fast standalone CLI. |

Niteo complements ESLint and Knip rather than replacing them. Where ESLint checks what your code does and Knip checks what your code uses, Niteo checks how your project is organized.

## Features

- **54 structural rules** covering exports, barrel files, directory shape, imports, hooks, components, and unsafe TypeScript patterns. See the full [rule reference](docs/rules.md).
- **Baselines** — snapshot existing violations so CI only fails on new issues. Adopt Niteo incrementally without fixing every file first.
- **Suppressions** — inline `niteo-ignore-file`, `niteo-ignore-next-line`, and `niteo-ignore-line` directives for narrow exceptions. Stale directive detection keeps suppressions honest.
- **Monorepo configs** — cascading `niteo.toml` files. Set defaults at the workspace root and override per package. Rule options merge field-by-field.
- **Import graph** — `niteo stats` shows fan-out and most-imported files. `niteo graph` outputs DOT or JSON for visualization and tooling.
- **SARIF output** — `--format sarif` integrates with GitHub Code Scanning, Azure DevOps, and any SARIF-compatible dashboard.
- **NDJSON output** — `--format ndjson` produces one valid JSON object per line for streaming consumption.
- **Watch mode** — `niteo lint --watch` re-lints on every file change during development.
- **Cache** — `niteo lint --cache` caches import graph analysis for faster repeated runs. Automatically invalidated when files, config, or Niteo version changes.
- **Health score** — every run produces a 0–100 score so you can track structural health over time.
- **Git-aware scanning** — `--git` limits analysis to changed files, keeping feedback fast on large codebases. Interactive mode auto-detects changed files with best-effort fallback.

## Installation

With Homebrew:

```sh
brew install FrozenProductions/Niteo/niteo
```

Or tap the repository first:

```sh
brew tap FrozenProductions/Niteo https://github.com/FrozenProductions/Niteo
brew install niteo
```

Homebrew bottles are published from GitHub Actions when `Cargo.toml` version changes on `main`.

Run directly with `npx`:

```sh
npx niteo-cli lint
```

Or install globally:

```sh
npm i -g niteo-cli
niteo lint
```

The npm package ships prebuilt binaries for common platforms. Rust is only required when building from source or using an unsupported platform.

## Usage

```sh
niteo lint              # Scan for structural issues
niteo init              # Create niteo.toml
niteo baseline create   # Snapshot current violations
niteo rules             # List configured rules
niteo explain no-console
```

Examples:

```sh
niteo lint --root src
niteo lint --scope src/components
niteo lint --format json --output report.json
niteo lint --format sarif --output report.sarif
niteo lint --watch
niteo lint --git
niteo lint --fail-on error
niteo lint --cache
niteo lint --cache --watch
niteo stats
niteo graph
```

## Monorepos

Niteo supports cascading configs. Place a `niteo.toml` at the workspace root and additional `niteo.toml` files inside individual packages. Child configs merge on top of the root config, overriding only the fields they declare.

```toml
# niteo.toml (root)
[project]
root = "packages"

[rules.no-console]
severity = "warn"
```

```toml
# packages/admin/niteo.toml
[rules.no-console]
severity = "error"
```

See [Configuration](docs/configuration.md#cascading-configs) for merge semantics and examples.

## Documentation

See the full [documentation](docs/README.md).
