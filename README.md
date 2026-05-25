# Niteo

Niteo is a standalone Rust CLI for structural linting in TypeScript projects.

It is intended to check project shape and source structure rather than formatting. The current alpha build only covers a small part of that goal, so treat it as an experiment and development preview.

## Status

Niteo is in alpha.

Do not use it as a production quality linter yet. The rule set is small, the output format may change, configuration is not stable, and some intended project guidance is not implemented. It is useful right now for testing the direction of the tool and contributing early feedback.

## What It Does Today

Niteo is being built around a few core jobs:

- scan TypeScript project files
- apply structural linting rules
- read project-level configuration
- print terminal reports that are useful during development

Exact rules, configuration options, and report details are still changing during alpha.

## Installation

Run directly with `npx`:

```sh
npx niteo-cli lint
```

Or install globally and use the `niteo` command:

```sh
npm i -g niteo-cli
niteo lint
```

The npm package builds the Rust binary during installation, so Rust and Cargo must be installed on the machine running `npx` or `npm i -g`.

For local development, run it from source:

```sh
cargo run -- lint
```

For local development, you can also build the binary:

```sh
cargo build
```

Then run:

```sh
./target/debug/niteo lint
```

## Usage

Scan the default project root:

```sh
npx @frozenproductions/niteo lint
# or, after npm i -g @frozenproductions/niteo
niteo lint
```

Generate a starter config:

```sh
npx @frozenproductions/niteo init
# or
niteo init
```

List available rules and their configured severities:

```sh
niteo rules
```

Explain one rule:

```sh
niteo explain no-console
```

Scan a specific root:

```sh
npx @frozenproductions/niteo lint --root src
# or
niteo lint --root src
```

Restrict the scan to a path:

```sh
npx @frozenproductions/niteo lint --scope src/components
# or
niteo lint --scope src/components
```

Show help:

```sh
npx @frozenproductions/niteo --help
# or
niteo --help
```

## Configuration

Niteo looks for `niteo.toml` in the current workspace.

You can generate a starter config with:

```sh
npx @frozenproductions/niteo init
# or, after npm i -g @frozenproductions/niteo
niteo init
```

The config format is not stable yet, so prefer generating it from the CLI instead of copying old examples.

## Current Limitations

- The rule set is incomplete.
- Some checks may be shallow while the project is still being shaped.
- The config shape may change.
- The report format may change.
- It has not been tested across large real-world codebases.
- It should not replace ESLint, TypeScript, or existing CI checks.

## Development

Useful commands:

```sh
cargo fmt
cargo check
cargo test
```

The project currently keeps the CLI, config loading, file discovery, rule checks, and reporting in separate modules.
