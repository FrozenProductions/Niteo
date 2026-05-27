# Niteo

Niteo is a standalone Rust CLI for structural linting in TypeScript projects.

It checks project shape and source structure rather than formatting. Niteo uses [oxc](https://github.com/oxc-project/oxc) for AST parsing.

## Status

Niteo is in alpha.

The rule set, output format, and configuration shape may change. It is useful for testing the direction of the tool and contributing early feedback.

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

For local development:

```sh
cargo run -- lint
cargo build
./target/debug/niteo lint
```

## Usage

### Commands

```sh
niteo lint              # Scan for structural issues
niteo init              # Create niteo.toml with default configuration
niteo baseline create   # Snapshot current violations into niteo-baseline.json
niteo rules             # List rules and their configured severities
niteo explain <rule>    # Explain a rule (e.g. niteo explain no-console)
```

### Options

| Flag                | Description                                         |
| ------------------- | --------------------------------------------------- |
| `--root <path>`     | Project root to scan                                |
| `--scope <path>`    | Limit scanning to this path                         |
| `--verbose`         | Show every violation (default groups and truncates) |
| `--git`             | Scan changed TypeScript files only                  |
| `--format <format>` | Output format: `text` (default), `json`, `sarif`    |
| `--output <path>`   | Write output to a file                              |
| `--baseline <path>` | Baseline file path (default: `niteo-baseline.json`) |

All options are global and work with any command.

### Examples

```sh
niteo lint --root src
niteo lint --scope src/components
niteo lint --git
niteo lint --format json --output report.json
niteo lint --format sarif --output report.sarif
niteo lint --verbose
niteo baseline create
niteo lint --baseline niteo-baseline.json
niteo rules --format json
niteo explain no-barrel-files
```

When `--git` is not passed but changed TypeScript files are detected, Niteo prompts to scan only those files.

## Output

The text report includes a health score (0-100), a status label, and a rule overview grouped by severity. Use `--verbose` to see all violations without truncation.

JSON and SARIF output formats are available via `--format`.

`niteo lint` exits with a non-zero status when it reports violations. If a baseline file exists, violations already captured in that baseline are ignored, so CI fails only when new violations are introduced.

## Baseline

Generate a baseline before adding Niteo to an existing codebase:

```sh
niteo baseline create
```

This writes `niteo-baseline.json` with the current violations. Commit that file, then run `niteo lint` in CI. Re-run `niteo baseline create` intentionally when you want to refresh the accepted set of existing violations.

## Configuration

Niteo reads `niteo.toml` from the current workspace. Generate a starter config with:

```sh
niteo init
```

### Project settings

```toml
[project]
root = "src"
respect-gitignore = true
```

### Project structure

Define conventions once under `[project.structure]`. Multiple rules share these definitions.

```toml
[project.structure.hooks]
folders = ["hooks"]
file-suffixes = [".hook.ts", ".hooks.ts"]

[project.structure.components]
folders = ["components"]
file-suffixes = [".component.tsx", ".components.tsx"]

[project.structure.types]
folders = ["types"]
file-suffixes = [".type.ts", ".types.ts"]

[project.structure.constants]
folders = ["constants"]
file-suffixes = [".constant.ts", ".constants.ts"]
```

### Configuration examples

Simple severity override:

```toml
[rules]
no-console = "error"
no-debugger = "error"
prefer-satisfies = "off"
```

Options table:

```toml
[rules.no-console]
severity = "error"
allow-patterns = ["logger", "debug"]

[rules.no-large-file]
severity = "warn"
max-lines = 300

[rules.max-file-exports]
severity = "warn"
max-exports = 5

[rules.boolean-prefix]
severity = "warn"
prefixes = ["is", "has", "can", "should"]
ignore-constants = true

[rules.max-items-per-directory]
severity = "warn"
max-items = 15
ignore-dirs = ["__tests__"]
count-folders = true

[rules.no-dump-files]
severity = "warn"
extra-names = ["misc", "common"]
```

## Suppressing violations

Use inline comment directives to suppress specific violations:

```ts
// niteo-ignore-file                    — suppress all rules in this file
// niteo-ignore-file: no-console        — suppress one rule in this file
// niteo-ignore-file: no-console, no-eval — suppress multiple rules

// niteo-ignore-next-line              — suppress all rules on the next line
// niteo-ignore-next-line: no-console  — suppress one rule on the next line

console.log("debug"); // niteo-ignore-line         — suppress all rules on this line
console.log("debug"); // niteo-ignore-line: no-console — suppress one rule on this line
```

## Development

```sh
cargo fmt
cargo check
cargo test
```

The project keeps the CLI, config loading, file discovery, rule checks, and reporting in separate modules.
