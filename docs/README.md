# Niteo Documentation

Niteo is a standalone Rust CLI for structural linting in TypeScript projects. It checks project shape, module boundaries, source structure, and unsafe TypeScript patterns. It does not format code.

## Contents

- [CLI reference](./cli.md)
- [Configuration](./configuration.md)
- [Rules](./rules.md)
- [Reports and output formats](./reports.md)
- [Autofix](./fix.md)
- [Baselines](./baselines.md)
- [Suppressions](./suppressions.md)
- [CI usage](./ci.md)
- [Benchmarks](./benchmarks.md)

## Quick Start

```sh
npx niteo-cli lint
```

Or install globally:

```sh
npm i -g niteo-cli
niteo lint
```

Create a config file:

```sh
niteo init
```

## Pathways

Choose the path that matches what you are doing:

### First run

```sh
niteo init              # Create niteo.toml
niteo lint              # Scan for structural issues
```

See [CLI reference](./cli.md#lint) for options.

### Existing project with violations

```sh
niteo baseline create   # Snapshot current violations
git add niteo-baseline.json
niteo lint              # Reports only new violations
```

See [Baselines](./baselines.md) for the full workflow.

### CI setup

```sh
npx niteo-cli lint --fail-on error
```

See [CI usage](./ci.md) for exit thresholds, caching, monorepos, and GitHub Actions.

### Monorepo setup

Place a root `niteo.toml` at the workspace level and additional `niteo.toml` files inside packages. Child configs merge on top of the root. See [Cascading Configs](./configuration.md#cascading-configs).

### Understanding rule output

```sh
niteo rules             # List all rules with configured severity
niteo explain no-console # Explain a specific rule
niteo lint -v        # Show every finding
niteo lint -vv       # Show every finding + progress bars
```

See [Rules](./rules.md) and [Reports](./reports.md).

### Tuning rules

Edit `niteo.toml` to set severities and rule options. Validate changes:

```sh
niteo config check
```

See [Configuration](./configuration.md#rule-severity) for severity syntax and [Rules](./rules.md) for rule-specific options.

### Using baselines and suppressions

- **Baselines** record current violations so CI only catches new ones. See [Baselines](./baselines.md).
- **Suppressions** use inline `niteo-ignore` directives for per-line exceptions. See [Suppressions](./suppressions.md).

### Applying autofixes

```sh
niteo fix                 # Apply safe fixes
niteo fix --dry-run       # Preview without writing
niteo lint --fix          # Lint then fix
```

See [Autofix](./fix.md) for supported rules and safety guards.

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
