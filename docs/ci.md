# CI Usage

Use Niteo in CI to prevent new structural issues from being introduced.

## Basic CI Command

```sh
npx niteo-cli lint
```

`niteo lint` exits with a non-zero status when it reports unsuppressed, non-baselined violations that meet the configured failure threshold.

Parse failures always fail the run. If any discovered file cannot be parsed, the exit status is non-zero regardless of the `--fail-on` threshold; violations from unparseable files cannot be suppressed or baselined.

## Exit Thresholds

By default, `niteo lint` fails on any unsuppressed, non-baselined violation (`info`, `warn`, or `error`).

Use `--fail-on` to adjust the exit threshold:

```sh
npx niteo-cli lint --fail-on error
```

- `--fail-on error`: Fails only on `error` findings. Useful for surfacing warnings without blocking merges.
- `--fail-on warn`: Fails on `warn` and `error` findings.
- `--fail-on any`: Fails on `info`, `warn`, and `error` findings (default, strict mode).

The report will still display all findings regardless of the threshold.

## Existing Projects

For projects that already have violations, create and commit a baseline first:

```sh
npx niteo-cli baseline create
git add niteo-baseline.json
git commit -m "Add Niteo baseline"
```

Then run:

```sh
npx niteo-cli lint
```

CI will fail only for violations not present in the baseline.

See [Baselines](./baselines.md) for the full workflow and [Suppressions](./suppressions.md) for inline ignore directives (an alternative for per-line exceptions).

## GitHub Actions Example

```yaml
name: Niteo

on:
  pull_request:
  push:
    branches: [main]

jobs:
  niteo:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - run: npx niteo-cli lint
```

The npm package ships prebuilt binaries, so no Rust installation is required for standard usage.

## JSON Artifacts

Write JSON output for later processing:

```sh
npx niteo-cli lint --format json --output niteo-report.json
```

Upload `niteo-report.json` as a CI artifact if you want to inspect the complete report after a failed run.

## SARIF

Write SARIF for code scanning systems:

```sh
npx niteo-cli lint --format sarif --output niteo.sarif
```

Only `lint` supports SARIF and NDJSON output.

## Cache

Caching is opt-in. Use `--cache` to speed up repeated runs by storing import graph analysis, parsed AST results, and rule violations at `.niteo/cache.json`:

```sh
npx niteo-cli lint --cache
```

The cache is invalidated automatically when file contents, Niteo version, config, `tsconfig.json`, or the scanned file list changes. For CI, add `.niteo/` to `.gitignore` rather than committing it:

```gitignore
.niteo/
```

Use `--clear-cache` to remove the cache before a run:

```sh
npx niteo-cli lint --clear-cache
```

## Full Scan Vs. Changed Files

### Full scan

```sh
npx niteo-cli lint
```

Scans every TypeScript file under the configured root. Best for main branch CI where complete coverage matters. Detects issues in any file, including ones not modified in the current PR.

### Changed files

```sh
npx niteo-cli lint --git
npx niteo-cli lint --git origin/main..HEAD
npx niteo-cli lint --git-staged
npx niteo-cli lint --git-unstaged
```

Scans TypeScript files changed on this branch. Only `.ts` and `.tsx` files are included.

| Flag                  | Files included                                                                            |
| --------------------- | ----------------------------------------------------------------------------------------- |
| `--git`               | Working-tree diff vs HEAD, staged diff vs HEAD, and untracked files.                      |
| `--git RANGE`         | `git diff --name-only RANGE` (e.g. `main..HEAD`, `$BASE_SHA..HEAD`).                      |
| `--git-staged`        | Staged diff only (`git diff --name-only --cached`). Ideal for pre-commit hooks.           |
| `--git-unstaged`      | Working-tree diff vs index plus untracked files. Excludes staged changes.                 |

`--git`, `--git-staged`, and `--git-unstaged` are mutually exclusive.

**When to use full scan:** main branch CI, release pipelines, scheduled audits.

**When to use `--git RANGE`:** pull request CI — scope to commits in the PR (e.g. `${{ github.event.pull_request.base.sha }}..HEAD`).

**When to use `--git-staged`:** pre-commit hooks (e.g. lint-staged, husky).

The `--git*` flags are strict: they fail immediately if git is unavailable or returns an error. This ensures CI pipelines fail visibly when git context is missing rather than silently scanning the wrong files.

### Changed file detection without `--git`

When `--git` is not passed, Niteo attempts to detect changed TypeScript files via git. If detection succeeds and files are found, it prompts the user interactively. If git is unavailable, Niteo prints a warning and falls back to a full scan. This makes interactive use resilient outside git repositories but is not suitable for CI, where `--git` or a full scan should be chosen explicitly.

## Monorepos

Niteo supports cascading configs for monorepos. Place a `niteo.toml` at the workspace root and additional `niteo.toml` files inside individual packages. Child configs merge on top of the root config, overriding only the fields they declare.

```sh
npx niteo-cli lint --root packages
```

Niteo discovers every `niteo.toml` under the scan root and applies the nearest config to each file. See [Cascading Configs](./configuration.md#cascading-configs) for merge semantics.

For packages with independent migration paths, use separate baselines:

```sh
npx niteo-cli lint --root packages --baseline niteo-baseline.json
```

## Enforcing Config Policy

Use `--deny-child-configs` to prevent nested `niteo.toml` files from weakening or overriding root rules:

```sh
npx niteo-cli lint --deny-child-configs
```

When this flag is set, Niteo fails immediately if any `niteo.toml` is discovered under the scan scope. This is useful in CI to enforce that all packages share the same root policy without silent severity downgrades.

Combine with `--scope` to allow child configs outside the scanned area:

```sh
npx niteo-cli lint --deny-child-configs --scope src
```

This only rejects child configs found under `src/`.

See [Configuration](./configuration.md#cascading-configs) for cascading config semantics and [Configuration](./configuration.md#presets) for preset profiles.
