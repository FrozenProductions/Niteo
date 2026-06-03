# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic Versioning].

## Unreleased

## 0.1.4 - 2026-06-03

### Added

- `--fail-on` flag to control the exit code threshold by severity level.
- `no-process-env` rule to prevent direct `process.env` access.
- `no-abbreviations` rule to flag abbreviations in identifiers.
- `no-circular-import` rule to detect circular import chains.
- `no-restricted-imports` rule to block imports from a configurable deny-list.
- `no-nested-functions` rule to limit function nesting depth.
- `no-orphan-files` rule to detect files not imported anywhere.
- `no-focused-test` rule to block `describe.only` / `it.only` / `test.only`.
- `no-skipped-test` rule to block `describe.skip` / `it.skip` / `test.skip`.
- `explicit-return-type` rule for exported function contracts.
- `max-function-params` rule to limit function parameters.
- `no-type-assertion` rule to disallow `as` casts.
- `no-magic-numbers` rule to disallow numeric literals outside constants.
- `prefer-readonly` rule to enforce `readonly` array parameters in exported functions.
- `no-empty-domain` rule to detect domain folders containing only barrel files.
- `no-anemic-domain` rule to flag domain folders with too few files.
- `no-god-domain` rule to flag domain folders with too many files.

### Changed

- Merge `no-component-default-export` into `no-default-export` with a `components-only` option.

### Removed

- Dead tests and unused `jsx` module.

## 0.1.3 - 2026-05-31

### Added

- Homebrew formula for installing Niteo from the repository tap.
- GitHub Actions automation for Homebrew releases and prebuilt bottles on macOS Intel, macOS Apple Silicon, and Linux.

## 0.1.2 - 2026-05-29

### Added

- Watch mode via `--watch` flag. Runs an initial lint pass, then watches for changes to `.ts`, `.tsx`, and `niteo.toml` files and re-lints automatically with debounced filesystem notifications.
- Cascading config support for monorepos. Niteo now discovers and merges nested `niteo.toml` files. Child configs override only the fields they declare; undeclared fields inherit from the parent. Rule option tables merge field-by-field.
- Per-config-scope directory rules. Directory-level rules now run per config node, excluding child config directories to avoid double-reporting.
- Per-file config resolution. Each file is linted against the nearest `niteo.toml` in its ancestor directories.

## 0.1.0 - 2026-05-29

_First beta release._

### Added

- Ship the first beta release of Niteo.
- Structural linting for TypeScript projects using oxc AST parsing.
- Configuration via `niteo.toml` with project structure definitions.
- Baseline support for incremental adoption in existing codebases.
- Multiple output formats: text, JSON, and SARIF.
- Inline suppression directives with stale detection.
- Git-aware scanning for changed files only.
- Rule explanation command for documentation lookup.

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/
