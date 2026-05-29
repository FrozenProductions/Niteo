# Configuration

Niteo reads `niteo.toml` from the current workspace.

Create a default config:

```sh
niteo init
```

If no config file exists, Niteo uses its built-in defaults.

## Project Settings

```toml
[project]
root = "src"
respect-gitignore = true
```

| Setting | Type | Default | Description |
| --- | --- | --- | --- |
| `root` | string | `src` when it exists, otherwise `.` | Project root to scan. |
| `respect-gitignore` | boolean | `true` | Whether file discovery respects `.gitignore`. |

`--root` overrides `[project].root`.

## Project Structure

Project structure settings tell Niteo how to identify domain-specific files such as components, hooks, types, constants, tests, and generated files.

Each domain supports:

| Field | Type | Description |
| --- | --- | --- |
| `folders` | string array | Any file inside a matching folder belongs to the domain. |
| `file-suffixes` | string array | Any file with a matching suffix belongs to the domain. |

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

## Rule Severity

Every rule has a severity:

| Severity | Behavior |
| --- | --- |
| `off` | Disable the rule. |
| `info` | Report as a suggestion. |
| `warn` | Report as a warning. |
| `error` | Report as an error. |

Unknown severity strings are treated as `warn`.

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

## Complete Example

```toml
[project]
root = "src"
respect-gitignore = true

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
```

## Rule Options

| Rule | Options |
| --- | --- |
| `boolean-prefix` | `prefixes`, `ignore-constants` |
| `entry-file-no-logic` | `entry-files` |
| `hook-prefix` | `prefixes` |
| `max-directory-depth` | `max-depth`, `ignore-dirs` |
| `max-file-exports` | `max-exports` |
| `max-items-per-directory` | `max-items`, `ignore-dirs`, `count-folders` |
| `min-items-per-directory` | `min-items`, `ignore-dirs`, `count-folders` |
| `no-any` | `allowed-folders` |
| `no-comments` | `allow-doc-comments` |
| `no-console` | `allow-patterns` |
| `no-dump-files` | `extra-names` |
| `no-duplicate-file-names` | `ignore-names` |
| `no-empty-directories` | `ignore-dirs` |
| `no-interface` | `allow-declaration-merging` |
| `no-large-file` | `max-lines` |
| `no-upward-import` | `max-depth` |

Every rule also supports `severity`.

## Cascading Configs

Niteo supports nested `niteo.toml` files for monorepos and multi-package projects. When Niteo discovers a `niteo.toml` inside a subdirectory, it merges that config on top of the root config.

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

The default config generated by `niteo init` enables the current rule set. Most rules default to `warn`. `no-empty-interface` defaults to `error`, and `prefer-satisfies` defaults to `info`.

Use `niteo rules` to see the effective severity of every rule after config resolution.

