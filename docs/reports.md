# Reports And Output Formats

Niteo can write text, JSON, and SARIF reports.

```sh
niteo lint --format text
niteo lint --format json
niteo lint --format sarif
niteo lint --format json --output niteo-report.json
```

`lint` supports all three formats. `rules`, `explain`, `stats`, and `graph` support `text` and `json`.

## Text Reports

Text is the default output format.

```sh
niteo lint
```

The text report contains:

- a `Niteo Structure Health` header
- findings grouped by rule
- per-file locations
- a health score from 0 to 100
- a status label
- a rule overview grouped by severity

By default, text output is summarized:

- up to 6 rule groups
- up to 6 files per rule
- up to 8 locations per file

Use `--verbose` to show every finding:

```sh
niteo lint --verbose
```

## Health Score

The health score starts at 100.

Warnings count as 1 point of weight. Errors count as 2 points of weight. The weighted count is scaled by the number of scanned files.

Info findings do not lower the score.

Status labels are:

| Condition | Status |
| --- | --- |
| one or more errors | `Needs attention` |
| no errors, one or more warnings | `Review recommended` |
| no errors or warnings, one or more info findings | `Suggestions available` |
| no findings | `Healthy` |

## JSON Reports

```sh
niteo lint --format json
```

The JSON report contains:

```json
{
  "summary": {
    "filesScanned": 12,
    "violations": 3,
    "errors": 1,
    "warnings": 2,
    "info": 0,
    "score": 75,
    "status": "Needs attention"
  },
  "files": ["src/app.ts"],
  "violations": [
    {
      "file": "src/app.ts",
      "line": 4,
      "column": 1,
      "rule": "no-console",
      "message": "Unexpected console statement.",
      "severity": "warning",
      "detail": null,
      "subject": null
    }
  ]
}
```

When `--report-suppressions` is used, JSON output also includes `suppressions`.

## SARIF Reports

```sh
niteo lint --format sarif --output niteo.sarif
```

SARIF output uses version `2.1.0`. Use it for code scanning systems that understand SARIF.

Severity mapping:

| Niteo severity | SARIF level |
| --- | --- |
| `error` | `error` |
| `warn` | `warning` |
| `info` | `note` |
| `off` | `none` |

## Stats Output

```sh
niteo stats
niteo stats --format json
```

Stats output includes:

- `files`
- `import_edges`
- `unresolved_local_imports`
- `most_imported`
- `highest_fanout`

## Graph Output

```sh
niteo graph
niteo graph --format json
```

Text graph output is DOT:

```sh
niteo graph | dot -Tsvg > imports.svg
```

JSON graph output contains:

```json
{
  "nodes": [
    {
      "path": "src/app.ts",
      "is_barrel": false,
      "is_test": false
    }
  ],
  "edges": [
    {
      "source": "src/app.ts",
      "target": "src/bootstrap.ts",
      "specifier": "./bootstrap",
      "kind": "Value"
    }
  ]
}
```

