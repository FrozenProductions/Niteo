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

## Documentation

See the full [documentation](docs/README.md).
