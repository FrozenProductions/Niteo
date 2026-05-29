# Niteo

Niteo is a standalone Rust CLI for structural linting in TypeScript projects.

It checks project shape and source structure rather than formatting. Niteo uses [oxc](https://github.com/oxc-project/oxc) for AST parsing.

## Installation

Run directly with `npx`:

```sh
npx niteo-cli lint
```

Or install globally:

```sh
npm i -g niteo-cli
niteo lint
```

The npm package builds the Rust binary during installation, so Rust and Cargo must be installed on the machine running `npx` or `npm i -g`.

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
