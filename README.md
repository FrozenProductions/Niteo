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

This project is not published as a package yet. Run it from source:

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
cargo run -- lint
```

Generate a starter config:

```sh
cargo run -- init
```

Scan a specific root:

```sh
cargo run -- lint --root src
```

Restrict the scan to a path:

```sh
cargo run -- lint --scope src/components
```

Show help:

```sh
cargo run -- --help
```

## Configuration

Niteo looks for `niteo.toml` in the current workspace.

You can generate a starter config with:

```sh
cargo run -- init
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

## License

No license has been added yet.
