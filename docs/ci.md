# CI Usage

Use Niteo in CI to prevent new structural issues from being introduced.

## Basic CI Command

```sh
npx niteo-cli lint
```

`niteo lint` exits with a non-zero status when it reports unsuppressed, non-baselined violations.

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

      - uses: dtolnay/rust-toolchain@stable

      - run: npx niteo-cli lint
```

The npm package builds the Rust binary during installation, so the workflow must install Rust before running `npx niteo-cli`.

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

Only `lint` supports SARIF output.

## Changed Files

Use `--git` to scan changed TypeScript files only:

```sh
npx niteo-cli lint --git
```

`--git` uses:

```sh
git diff --name-only HEAD
git diff --name-only --cached
```

Only `.ts` and `.tsx` files are included.

For full protection on main branches, prefer scanning the whole configured root. Changed-file scanning is best for local development or fast pull request feedback.

## Monorepos

Niteo currently resolves one workspace config at a time. For multiple packages, run Niteo once per package root:

```sh
npx niteo-cli lint --root packages/web/src
npx niteo-cli lint --root packages/admin/src
```

Use separate baselines when packages have independent migration paths:

```sh
npx niteo-cli lint --root packages/web/src --baseline packages/web/niteo-baseline.json
```

