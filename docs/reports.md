# Reports And Output Formats

Niteo can write text, JSON, SARIF, and NDJSON reports.

```sh
niteo lint --format text
niteo lint --format json
niteo lint --format sarif
niteo lint --format ndjson
niteo lint --format json --output niteo-report.json
```

`lint` supports all four formats. `rules`, `explain`, `stats`, and `graph` support `text` and `json`. They reject `sarif` and `ndjson`.

## Text Reports

Text is the default output format. It is intended for human reading and is **not stable for programmatic parsing**. Use JSON, SARIF, or NDJSON for machine consumption.

```sh
niteo lint
```

The text report contains:

- a `Niteo Structure Health` header
- a `Diagnostics` section when operational warnings occur
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

The health score is informational. It is not a substitute for a structured quality model and may change in future versions.

Status labels are:

| Condition                                        | Status                  |
| ------------------------------------------------ | ----------------------- |
| one or more errors                               | `Needs attention`       |
| no errors, one or more warnings                  | `Review recommended`    |
| no errors or warnings, one or more info findings | `Suggestions available` |
| no findings                                      | `Healthy`               |

## JSON Reports

JSON output is machine-readable and stable across Niteo versions. Use it for CI artifacts, dashboards, and programmatic processing.

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
  ],
  "diagnostics": []
}
```

When `--report-suppressions` is used, JSON output also includes `suppressions`.

Operational warnings (for example, cache or workspace discovery failures) are collected in `diagnostics` rather than printed to `stderr`. Each diagnostic has a `category` (`cache`, `git`, or `workspace`) and a `message`.

## SARIF Reports

SARIF output is machine-readable and designed for code scanning systems (e.g., GitHub code scanning, SARIF-compatible CI platforms).

```sh
niteo lint --format sarif --output niteo.sarif
```

SARIF output uses version `2.1.0`. Use it for code scanning systems that understand SARIF.

Severity mapping:

| Niteo severity | SARIF level |
| -------------- | ----------- |
| `error`        | `error`     |
| `warn`         | `warning`   |
| `info`         | `note`      |
| `off`          | `none`      |

Operational diagnostics are emitted under `runs[0].invocations[0].toolExecutionNotifications` with level `warning` and a `descriptor.id` matching the diagnostic category.

## NDJSON Reports

NDJSON output is machine-readable and designed for streaming consumers. Each line is independently parseable.

```sh
niteo lint --format ndjson
niteo lint --format ndjson --output report.ndjson
```

NDJSON (newline-delimited JSON) outputs one valid JSON object per line.
Each line is independently parseable by streaming consumers.

NDJSON record order is: `summary` first, then `file` records, then `diagnostic` records, then `violation` records, then optionally a `suppressions` record. Consumers should not rely on any other ordering.

Every record has a `type` field:

| `type`         | Description                                             |
| -------------- | ------------------------------------------------------- |
| `summary`      | Overall run statistics (always first).                  |
| `file`         | One record per scanned file.                            |
| `diagnostic`   | One record per operational warning.                     |
| `violation`    | One record per lint violation.                          |
| `suppressions` | Suppression report (only with `--report-suppressions`). |

Example output:

```json
{"type":"summary","filesScanned":2,"violations":2,"errors":1,"warnings":1,"info":0,"score":50,"status":"Needs attention"}
{"type":"file","file":"src/console.ts"}
{"type":"file","file":"src/any.ts"}
{"type":"violation","file":"src/console.ts","line":4,"column":1,"rule":"no-console","message":"Unexpected console statement.","severity":"warning","detail":null,"subject":null}
{"type":"violation","file":"src/any.ts","line":1,"column":12,"rule":"no-any","message":"Avoid explicit any.","severity":"warning","detail":null,"subject":"value"}
```

Clean projects still produce useful output:

```json
{"type":"summary","filesScanned":2,"violations":0,"errors":0,"warnings":0,"info":0,"score":100,"status":"Healthy"}
{"type":"file","file":"src/app.ts"}
{"type":"file","file":"src/utils.ts"}
```

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
