# Configuration

Niteo reads `niteo.toml` from the current workspace.

Create a config:

```sh
niteo init                 # Full default config (all rules)
niteo init --preset strict # Focused rule set from a named preset
```

If no config file exists, Niteo uses its built-in defaults.

## TypeScript Path Aliases

Niteo reads `tsconfig.json` from the workspace root to resolve TypeScript path aliases for import graph rules.

```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"],
      "@features/*": ["src/features/*"]
    }
  }
}
```

Niteo uses `compilerOptions.baseUrl` and `compilerOptions.paths` when building the import graph. When a `tsconfig.json` is present, aliased imports such as `import { formatDate } from "@/shared/date"` are resolved to real files. This allows graph-based rules (circular import detection, orphan files, test imports, and barrel chains) to work with projects that use path aliases.

Supported path features:

- `baseUrl` — relative to the tsconfig location
- `paths` — one `*` wildcard per pattern
- Extensionless resolution (`.ts`, `.tsx`)
- Directory barrel resolution (`index.ts`, `index.tsx`)

If `tsconfig.json` is not present or has no `paths` configuration, Niteo falls back to its default resolution (relative imports only).

## Project Settings

```toml
[project]
root = "src"
respect-gitignore = true
history = true
baseline = "niteo-baseline.json"
```

| Setting             | Type    | Default                             | Description                                                        |
| ------------------- | ------- | ----------------------------------- | ------------------------------------------------------------------ |
| `root`              | string  | `src` when it exists, otherwise `.` | Project root to scan.                                              |
| `respect-gitignore` | boolean | `true`                              | Whether file discovery respects `.gitignore`.                      |
| `history`           | boolean | `true`                              | Whether `lint` appends `.niteo/history.jsonl` by default.          |
| `baseline`          | string  | `niteo-baseline.json`               | Baseline file path used by `lint`, `fix`, and `baseline` commands. |

`--root` overrides `[project].root`. `--baseline` overrides `[project].baseline`. `niteo lint --history` writes a history entry even when `[project].history` is `false`.

## Workspace Packages

Niteo discovers workspace packages from `package.json` `workspaces` (either a top-level array or `workspaces.packages`) and from `pnpm-workspace.yaml` `packages`. Each entry is a glob pattern relative to the workspace root.

Glob patterns use gitignore-style semantics:

- `*` matches a single path segment.
- `**` matches any number of path segments recursively.
- `!` patterns exclude previously matched paths.

Examples:

```json
[
  "packages/*",
  "packages/*/*",
  "apps/*/packages/*",
  "packages/**",
  "!packages/excluded"
]
```

Rules such as `no-package-cycle` and `no-private-package-import` rely on this discovery. They are disabled when no workspace is detected.

## Project Structure

Project structure settings tell Niteo how to identify domain-specific files such as components, hooks, types, constants, tests, and generated files.

Each domain supports:

| Field           | Type         | Description                                              |
| --------------- | ------------ | -------------------------------------------------------- |
| `folders`       | string array | Any file inside a matching folder belongs to the domain. |
| `file-suffixes` | string array | Any file with a matching suffix belongs to the domain.   |

Matching is additive. A file matches a domain if it is inside one of the configured folders or its file name ends with one of the configured suffixes.

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

[project.structure.tests]
folders = ["tests"]
file-suffixes = [".test.ts", ".tests.ts"]

[project.structure.generated]
folders = ["generated", "__generated__"]
file-suffixes = [".generated.ts", ".generated.tsx"]
```

Rules use these domains to avoid hard-coding one project layout. For example, `hook-prefix` checks files identified by `project.structure.hooks`, while `no-any` always exempts files identified by `project.structure.generated`.

## Architecture Layers

The `[architecture.layers]` section defines ordered architectural layers for the `layer-boundaries` rule. Each layer is a named group of files identified by folders or file suffixes.

```toml
[architecture.layers]
order = ["app", "features", "entities", "shared"]

[architecture.layers.app]
folders = ["app"]

[architecture.layers.features]
folders = ["features"]

[architecture.layers.entities]
folders = ["entities"]

[architecture.layers.shared]
folders = ["shared"]
```

Layers are ordered from highest (app-specific) to lowest (shared utilities). A file in a higher layer may import from any layer at or below its own position. Importing upward — a lower layer importing from a higher one — violates the boundary.

With the example above:

```text
app       -> can import from features, entities, shared
features  -> can import from entities, shared
entities  -> can import from shared
shared    -> cannot import from app, features, or entities
```

Each layer definition uses the same `folders` and `file-suffixes` matching as structure domains. When a file matches multiple layers, the most specific folder match wins. Files matching no layer are ignored by `layer-boundaries`.

The `layer-boundaries` rule defaults to `off`. Enable it after defining your layers:

```toml
[rules.layer-boundaries]
severity = "warn"
```

## Rule Severity

Every rule has a severity:

| Severity | Behavior                |
| -------- | ----------------------- |
| `off`    | Disable the rule.       |
| `info`   | Report as a suggestion. |
| `warn`   | Report as a warning.    |
| `error`  | Report as an error.     |

Run `niteo config check` to validate the config file. It detects unknown rule names, unknown options, invalid severities, and conflicting rule combinations. See the [CLI reference](./cli.md#config-check) for details.

> **Tip:** Severity values are validated strictly. A typo like `severity = "warning"` will fail with a clear error naming the invalid value and the allowed severities (`off`, `info`, `warn`, `error`).

Use a simple string when you only need severity:

```toml
[rules]
no-console = "error"
prefer-satisfies = "off"
```

Use a table when the rule has options:

```toml
[rules.no-large-file]
severity = "warn"
max-lines = 300
```

## Failure Thresholds

By default, `lint` exits with a non-zero status when any enabled rule reports a violation. You can change this with the `[fail-on]` section.

```toml
[fail-on]
default = "error"

[fail-on.rules]
no-console = "warn"

[fail-on.categories]
hygiene = "warn"
```

| Field                   | Type   | Default | Description                                                                |
| ----------------------- | ------ | ------- | -------------------------------------------------------------------------- |
| `default`               | string | `any`   | Minimum severity that causes lint to fail. Values: `error`, `warn`, `any`. |
| `rules.<rule>`          | string | —       | Override the default threshold for a specific rule.                        |
| `categories.<category>` | string | —       | Override the default threshold for every rule in a category.               |

Category names are: `typescript`, `hygiene`, `exports`, `files`, `domain`, `imports`. A rule override takes precedence over a category override, and both take precedence over `default`.

## Autofix Settings

The optional `[fix]` table controls whether each rule may apply autofixes when you run `niteo fix` or `niteo lint --fix`.

```toml
[fix]
no-debugger = true
no-empty-interface = false
```

Each value must be a boolean. Rules default to `true`, so omitting a rule keeps autofix enabled for rules that support it. Set a rule to `false` to keep reporting its diagnostics while blocking its edits.

Only rules with autofix support use this table. `niteo config check` reports unknown rule names, non-boolean values, and entries for rules that do not currently support autofix.

In cascading configs, child `[fix]` entries merge with the parent table and override only the rules they declare:

```toml
# niteo.toml
[fix]
no-debugger = false
no-focused-test = true
```

```toml
# packages/app/niteo.toml
[fix]
no-debugger = true
```

## Complete Example

```toml
[project]
root = "src"
respect-gitignore = true
baseline = "niteo-baseline.json"

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

[project.structure.tests]
folders = ["tests", "__tests__"]
file-suffixes = [".test.ts", ".tests.ts", ".spec.ts", ".spec.tsx"]

[project.structure.generated]
folders = ["generated", "__generated__"]
file-suffixes = [".generated.ts", ".generated.tsx"]

[rules.no-console]
severity = "error"
allow-patterns = ["logger", "scripts"]

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

[rules.no-any]
severity = "warn"
allowed-folders = ["legacy"]

[fix]
no-debugger = true
no-focused-test = true
no-skipped-test = true
no-empty-interface = false
```

## Rule Options

| Rule                         | Options                                     |
| ---------------------------- | ------------------------------------------- |
| `boolean-prefix`             | `prefixes`, `ignore-constants`              |
| `directory-must-have-barrel` | `barrel-names`                              |
| `entry-file-no-logic`        | `entry-files`                               |
| `hook-prefix`                | `prefixes`                                  |
| `layer-boundaries`           | configured via `[architecture.layers]`      |
| `max-directory-depth`        | `max-depth`, `ignore-dirs`                  |
| `max-file-exports`           | `max-exports`, `count-default`              |
| `max-function-params`        | `max-params`                                |
| `max-items-per-directory`    | `max-items`, `ignore-dirs`, `count-folders` |
| `min-items-per-directory`    | `min-items`, `ignore-dirs`, `count-folders` |
| `no-any`                     | `allowed-folders`                           |
| `no-barrel-files`            | `barrel-names`                              |
| `no-circular-import`         | `report-all-nodes`                          |
| `no-comments`                | `allow-doc-comments`                        |
| `no-console`                 | `allow-patterns`                            |
| `no-default-export`          | `components-only`                           |
| `no-dump-files`              | `extra-names`                               |
| `no-duplicate-file-names`    | `ignore-names`                              |
| `no-empty-directories`       | `ignore-dirs`                               |
| `no-empty-domain`            | `ignore-dirs`                               |
| `no-anemic-domain`           | `max-files`, `ignore-dirs`                  |
| `no-god-domain`              | `max-files`, `ignore-dirs`                  |
| `no-interface`               | `allow-declaration-merging`                 |
| `no-large-file`              | `max-lines`                                 |
| `no-logic-in-barrel`         | `barrel-names`                              |
| `no-magic-numbers`           | `allowed-numbers`, `enforce-strings`        |
| `no-nested-functions`        | `max-depth`, `contexts`                     |
| `no-orphan-files`            | `entry-files`                               |
| `no-restricted-imports`      | `restricted`                                |
| `no-then-chain`              | `allow-single`                              |
| `no-upward-import`           | `max-depth`, `allow-patterns`               |

Every rule also supports `severity`.

## Cascading Configs

Niteo supports nested `niteo.toml` files for monorepos and multi-package projects. When Niteo discovers a `niteo.toml` inside a subdirectory, it merges that config on top of its direct parent config. Multi-level nesting (`root -> packages/ -> packages/admin/`) cascades correctly — each child inherits from its immediate parent.

To enforce a single root policy and reject all nested overrides, use `--deny-child-configs`. See [CI Usage](./ci.md#enforcing-config-policy) for details.

### Discovery

Niteo walks the scan root and finds every `niteo.toml` file. Each config applies to files under its directory. The root `niteo.toml` (at the workspace level) provides defaults for everything.

### Merge Rules

- Child configs override only the fields they declare.
- Undeclared fields inherit from the parent config.
- Rule option tables merge field-by-field. A child can set `severity = "off"` without losing an inherited `max-lines` value.
- `[project] root` in a child config is ignored. The project root is always resolved from the workspace-level config or `--root`.

### Example

Root config at the workspace level:

```toml
# niteo.toml
[project]
root = "packages"

[rules.no-console]
severity = "warn"
```

Child config for the admin package:

```toml
# packages/admin/niteo.toml
[rules.no-console]
severity = "error"

[project.structure.tests]
folders = ["tests", "__tests__"]
file-suffixes = [".test.ts", ".spec.ts"]
```

Files under `packages/admin/` use `severity = "error"` for `no-console` and the overridden test structure. All other rules inherit from the root config. Files outside `packages/admin/` use the root config unchanged.

### Partial Rule Override

A child config can override just the severity of a rule without losing inherited options:

```toml
# Root niteo.toml
[rules.no-large-file]
severity = "warn"
max-lines = 300

# packages/admin/niteo.toml
[rules.no-large-file]
severity = "error"
```

The admin package enforces `no-large-file` as an error with the same `max-lines = 300` inherited from the root.

### Directory Rules

Directory-level rules (`no-empty-directories`, `max-items-per-directory`, `min-items-per-directory`, `max-directory-depth`) run per config scope. Parent config directory rules exclude child config directories to avoid double-reporting inside nested packages that have their own rules.

## Defaults

The default config generated by `niteo init` (without `--preset`) enables the current rule set. Safety-critical rules (`no-debugger`, `no-eval`, `no-circular-import`, `no-focused-test`, `no-skipped-test`, `no-silent-catch`, `no-mutable-exports`, `no-namespace`, `no-non-null-assertion`, `no-empty-interface`, `no-package-cycle`, `no-test-code-in-production`, `no-test-import`) default to `error`. Convention and style rules (`sort-imports`, `sort-exports`, `no-default-export`, `no-barrel-files`, `no-enums`, `no-abbreviations`, `no-magic-numbers`, `prefer-satisfies`, and others) default to `info`. The rest default to `warn`.

Use `niteo rules` to see the effective severity of every rule after config resolution.

## Presets

Presets provide focused rule profiles for common project types. Use them with `niteo init --preset <name>` to generate a `niteo.toml` scoped to the preset's rules.

| Preset       | Purpose                                                                                                                                 |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `balanced`   | Recommended for most teams. Catches common issues without being overly strict.                                                          |
| `strict`     | Maximum coverage. Enables most rules at `warn` or `error`.                                                                              |
| `migration`  | Lower-noise profile for legacy projects. Disables rules that produce many violations during initial adoption.                           |
| `react`      | React app conventions. Includes component and hook rules, entry-file checks, and barrel file policies.                                  |
| `library`    | Package/library conventions. Focuses on public API hygiene, explicit types, and restricted imports.                                     |
| `no-barrels` | Prefers direct imports. Disables barrel-requiring rules (`directory-must-have-barrel`) and enables barrel-prohibiting rules at `error`. |

See what a preset enables without writing a config file:

```sh
niteo rules --preset strict
```

Presets are stable starting points. After running `init --preset`, edit the generated `niteo.toml` to fine-tune severities and options for your project.
