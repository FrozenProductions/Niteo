# CLI Reference

Niteo exposes one binary: `niteo`.

```sh
niteo [global options] [command]
```

If the command is omitted, Niteo runs `lint`.

## Global Options

These options are accepted by every command.

| Option                  | Short | Description                                                                                             |
| ----------------------- | ----- | ------------------------------------------------------------------------------------------------------- |
| `--root <path>`         |       | Project root to scan. Overrides `[project].root`.                                                       |
| `--scope <path>`        |       | Limit scanning to a path inside the project root.                                                       |
| `--verbose`             | `-v`  | Show every violation in text reports.                                                                   |
| `--git`                 |       | Scan changed TypeScript files only. Fails if git is unavailable.                                        |
| `--format <format>`     |       | Output format. Supported values: `text`, `json`, `sarif`, `ndjson`.                                     |
| `--output <path>`       | `-o`  | Write output to a file instead of stdout.                                                               |
| `--baseline <path>`     |       | Baseline file path. Defaults to `niteo-baseline.json`.                                                  |
| `--report-suppressions` |       | Include suppression counts and stale ignore directives.                                                 |
| `--watch`               |       | Re-run lint on file changes.                                                                            |
| `--cache`               |       | Enable caching of analysis results, parsed ASTs, and rule violations.                                   |
| `--no-cache`            |       | Disable caching even when it would otherwise be enabled.                                                |
| `--clear-cache`         |       | Clear the cache file before running.                                                                    |
| `--fail-on <threshold>` |       | Minimum severity that causes lint to fail. Supported values: `error`, `warn`, `any`. Defaults to `any`. |
| `--deny-child-configs`  |       | Fail when nested `niteo.toml` files are found inside the scan scope.                                    |

Not every command supports every output format. `rules`, `explain`, `stats`, `graph`, and `config` support `text` and `json`; they reject `sarif` and `ndjson`. `lint` supports `text`, `json`, `sarif`, and `ndjson`.

## `lint`

Scan the project for structural issues.

```sh
niteo lint
niteo lint --root src
niteo lint --scope src/components
niteo lint --verbose
niteo lint --format json --output niteo-report.json
niteo lint --format sarif --output niteo.sarif
```

`lint` reads `niteo.toml`, discovers `.ts` and `.tsx` files, applies enabled rules, applies ignore directives, filters known baseline violations, renders a report, and exits with a non-zero status if new violations meet the failure threshold.

Use `--fail-on error` to surface warnings without blocking CI. Use `--fail-on any` for strict mode.

When `--git` is not passed, Niteo attempts to detect changed TypeScript files via git. If detection succeeds and files are found, it prompts:

```text
Scan only changed files? [Y/n]
```

If git is unavailable or the detection fails, Niteo prints a warning and falls back to a full project scan. This makes interactive use resilient outside git repositories.

Use `--git` in scripts and CI when you want changed-file scanning without an interactive prompt. The `--git` flag is strict: it fails immediately if git is unavailable or returns an error.

For full protection on main branches, prefer scanning the whole configured root. See [CI Usage](./ci.md#changed-files) for a comparison of full scan vs. `--git`.

### Fix Mode

Apply autofixes after linting by passing `--fix`:

```sh
niteo lint --fix
```

The lint report prints first, then fixes are applied to files. The command exits with the same status code as lint would without `--fix`.

### Watch Mode

Run lint continuously during development:

```sh
niteo lint --watch
niteo lint --watch --root src
niteo lint --watch --scope src/components
```

Niteo runs a full lint pass on startup, then watches for changes to `.ts`, `.tsx`, and `niteo.toml` files. Each detected change triggers a re-lint after a short debounce. Press Ctrl+C to stop.

Watch mode disables the interactive changed-files prompt and always performs a full scan.

### Cache

`lint` can cache import graph analysis, parsed AST results, and rule violations to speed up repeated runs. Use `--cache` to opt in:

```sh
niteo lint --cache
niteo lint --cache --watch
```

Cached data is stored at `.niteo/cache.json` relative to the workspace root. The cache is invalidated conservatively when any of the following change:

- file content (hash-based)
- Niteo version
- `niteo.toml` config (or any nested child config)
- `tsconfig.json`
- the list of files being scanned
- cache schema version

The `.niteo/` directory should normally be added to `.gitignore`:

```gitignore
.niteo/
```

Use `--clear-cache` to remove the cache file before a run:

```sh
niteo lint --clear-cache
```

Use `--no-cache` to ensure caching is disabled even when it would otherwise be active.

## `fix`

Apply autofixes for rules that support them.

```sh
niteo fix
niteo fix --dry-run           # Preview without writing
niteo fix --root src           # Fix only src/
niteo fix --scope src/components
niteo fix --git                # Fix only changed files
niteo fix --baseline niteo-baseline.json
```

`fix` runs a full analysis pass, collects violations from rules that support autofix, applies the edits to source files, and prunes stale entries from the baseline.

Use `--dry-run` to preview fixes without writing files.

`fix` only addresses rules with a non-`None` `fix_capability` in their metadata (`Safe` or `Conditional`). Rules that support autofix today are `no-debugger`, `no-focused-test`, `no-skipped-test`, and `no-empty-interface`. Violations from other rules are left untouched.

Edits that overlap with other edits in the same file are rejected. Edits computed against source that has changed on disk since analysis are also rejected. These guards prevent corrupt output.

After applying fixes, `fix` prunes the baseline to remove entries that no longer match. If no baseline file exists, the baseline step is skipped.

Exit code is always `0` — the fix command does not fail on violations. Use `lint --fix` instead if you need lint's exit code behavior.

See [Autofix](./fix.md) for details on the edit model, safety guards, and how to add fix support to new rules.

## `init`

Create a `niteo.toml` in the current workspace.

```sh
niteo init                 # Full default config (all rules)
niteo init --preset strict # Use a predefined rule profile
```

Supported presets: `balanced`, `strict`, `migration`, `react`, `library`, `no-barrels`.

Without `--preset`, `init` writes the full default config with all rules enabled. With `--preset`, it writes a focused rule set based on the named profile. See [Presets](./configuration.md#presets) for details.

The command fails if `niteo.toml` already exists.

## `baseline create`

Create a baseline file from current violations.

```sh
niteo baseline create
niteo baseline create --baseline config/niteo-baseline.json
niteo baseline create --report-suppressions
```

Use this when adding Niteo to a project that already has known violations. `lint` ignores violations recorded in the baseline, so CI can fail only for newly introduced issues.

## `baseline prune`

Remove stale baseline entries that no longer match current violations.

```sh
niteo baseline prune
niteo baseline prune --baseline config/niteo-baseline.json
```

The command fails if the baseline file does not exist.

## `rules`

List all rules with their configured severities.

```sh
niteo rules                   # Uses current config or defaults
niteo rules --preset strict   # Show what a preset would configure
niteo rules --format json
niteo rules --output rules.txt
```

Without `--preset`, the command uses the current configuration, so severity overrides in `niteo.toml` are reflected in the output. With `--preset`, it shows the effective rule set for a named preset without reading any config file.

## `explain`

Print documentation for one rule.

```sh
niteo explain no-console
niteo explain no-console --format json
```

The explanation includes the rule intent, current severity, examples, and supported options.

## `stats`

Show import graph statistics.

```sh
niteo stats
niteo stats --format json
niteo stats --scope src/features/billing
```

Text output includes:

- file count
- import edge count
- unresolved local import count
- most imported files
- highest fan-out files

JSON output contains the same information in a machine-readable shape.

## `graph`

Output the project import graph.

```sh
niteo graph
niteo graph --format json
niteo graph --format json --output graph.json
```

Text output is DOT. It can be piped to Graphviz:

```sh
niteo graph | dot -Tsvg > imports.svg
```

JSON output contains `nodes` and `edges`. Nodes include `path`, `is_barrel`, and `is_test`. Edges include `source`, `target`, `specifier`, and `kind`.

## `config check`

Validate the config file and report diagnostics.

```sh
niteo config check
```

Reads `niteo.toml` from the workspace and checks for:

- **Unknown rule names** — typos in rule identifiers, with "did you mean?" suggestions.
- **Unknown rule options** — misspelled or unsupported option keys.
- **Invalid severities** — severity strings that are not `off`, `info`, `warn`, or `error`.
- **Conflicting rule combinations** — rules whose policies contradict each other (e.g., `directory-must-have-barrel` and `no-barrel-files` both enabled).

Output format:

```text
error  no-console.severity          unknown severity "warning"; use "warn"
warn   directory-must-have-barrel   conflicts with "no-barrel-files": one requires barrels while the other rejects them
```

The command exits with a non-zero status when any error-level diagnostic is reported. Warnings do not cause a failure exit code.

See [Configuration](./configuration.md#rule-severity) for severity syntax and [Rules](./rules.md) for rule-specific options.

## `config print`

Print the resolved config source to stdout.

```sh
niteo config print
```

If `niteo.toml` exists, prints its contents. Otherwise, prints the built-in default config. This is useful for inspecting what Niteo would use in CI or for checking the default config before running `init`.

See [Configuration](./configuration.md#cascading-configs) for how configs combine in monorepos.

## Path Resolution

`--root` is resolved relative to the workspace unless it is absolute.

`--scope` is resolved relative to the resolved project root. For example:

```sh
niteo lint --root packages/app/src --scope components
```

This scans `packages/app/src/components`.

`--output` and `--baseline` are resolved relative to the workspace unless they are absolute.
