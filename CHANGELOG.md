# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic Versioning].

## Unreleased

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
