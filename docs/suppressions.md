# Suppressions

Use inline ignore directives when a rule should not apply to a specific file or line.

Prefer configuration for broad project policy. Prefer suppressions for narrow exceptions.

## Ignore A File

```ts
// niteo-ignore-file
console.log("allowed in this file");
```

Limit the directive to specific rules:

```ts
// niteo-ignore-file: no-console, no-debugger
console.log("allowed in this file");
```

## Ignore The Next Line

```ts
// niteo-ignore-next-line: no-console
console.log("temporary debug output");
```

Without a rule list, the directive suppresses all rules for the next line:

```ts
// niteo-ignore-next-line
console.log("temporary debug output");
```

## Ignore The Current Line

```ts
console.log("temporary debug output"); // niteo-ignore-line: no-console
```

Without a rule list, the directive suppresses all rules for the current line:

```ts
console.log("temporary debug output"); // niteo-ignore-line
```

## Directive Syntax

| Directive | Scope |
| --- | --- |
| `niteo-ignore-file` | Suppresses matching violations anywhere in the file. |
| `niteo-ignore-next-line` | Suppresses matching violations on the next line. |
| `niteo-ignore-line` | Suppresses matching violations on the directive line. |

Rules are optional and comma-separated:

```ts
// niteo-ignore-next-line: no-console, no-debugger
```

When rules are omitted, the directive applies to all rules in its scope.

## Reporting Suppressions

Use `--report-suppressions` to see suppression activity and stale directives.

```sh
niteo lint --report-suppressions
niteo baseline create --report-suppressions
```

Text output reports:

- how many violations were suppressed
- which directives are stale
- the file and line for each stale directive

JSON output includes:

```json
{
  "suppressions": {
    "totalSuppressed": 1,
    "totalStale": 1,
    "files": [
      {
        "file": "src/app.ts",
        "suppressedCount": 1,
        "staleDirectives": [
          {
            "kind": "niteo-ignore-next-line",
            "line": 4,
            "rules": ["no-console"]
          }
        ]
      }
    ]
  }
}
```

## Best Practices

- Prefer rule-specific directives.
- Keep suppressions close to the code they suppress.
- Run `niteo lint --report-suppressions` before removing legacy suppressions.
- Do not use file-level suppressions for broad policy disagreement. Disable or tune the rule in `niteo.toml` instead.

