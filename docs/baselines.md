# Baselines

A baseline records the current set of violations so Niteo can ignore them during future `lint` runs.

This is useful when introducing Niteo to an existing project. You can prevent new issues from being introduced without fixing every existing violation first.

## Create A Baseline

```sh
niteo baseline create
```

This writes `niteo-baseline.json` by default.

Use a custom path:

```sh
niteo baseline create --baseline config/niteo-baseline.json
```

Commit the baseline file. After that, `niteo lint` filters out baseline entries and reports only new violations.

## Lint With A Baseline

```sh
niteo lint
```

Niteo automatically reads `niteo-baseline.json` when it exists.

Use a custom path:

```sh
niteo lint --baseline config/niteo-baseline.json
```

If the baseline file does not exist, `lint` continues without one.

## Prune A Baseline

As code is fixed, baseline entries can become stale. Remove them with:

```sh
niteo baseline prune
```

With a custom path:

```sh
niteo baseline prune --baseline config/niteo-baseline.json
```

`baseline prune` compares the baseline with current violations and writes the remaining entries back to the same file. It fails if the baseline file does not exist.

## Baseline Identity

Baseline entries are matched by:

- file path relative to the project root
- line
- column
- rule
- message
- subject

The violation detail text is not part of the identity. This lets some count-based details change without turning the same structural issue into a new violation.

If a violation moves to another line, changes rule, or changes subject, it is treated as new.

## Recommended Workflow

1. Run `niteo lint --verbose` to inspect the current state.
2. Run `niteo baseline create`.
3. Commit `niteo-baseline.json`.
4. Run `niteo lint` in CI.
5. Periodically run `niteo baseline prune` after cleanup work.

Avoid refreshing the baseline automatically in CI. A baseline update should be intentional and reviewed.

