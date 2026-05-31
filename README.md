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

It checks project shape and source structure rather than formatting. Niteo uses [oxc](https://github.com/oxc-project/oxc) for AST parsing.

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
niteo lint --watch
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
