# Rules

Niteo rules focus on structural TypeScript conventions: file shape, export style, project layout, imports, React hooks/components, and unsafe language features.

List configured rules:

```sh
niteo rules
niteo rules --format json
```

Explain one rule:

```sh
niteo explain no-console
niteo explain no-console --format json
```

## Table of Contents

- [Language And TypeScript Rules](#language-and-typescript-rules)
- [Source Hygiene Rules](#source-hygiene-rules)
- [Export And Module Shape Rules](#export-and-module-shape-rules)
- [File And Directory Rules](#file-and-directory-rules)
- [Domain Rules](#domain-rules)
- [Import Rules](#import-rules)

## Rule Reference

| Rule                             | Default severity | Purpose                                                                              | Fix                                                      | Options                        |
| -------------------------------- | ---------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------- | ------------------------------ |
| `boolean-prefix`                 | `info`           | Boolean variables should be prefixed with names such as `is`, `has`, or `can`.       | —                                                        | `prefixes`, `ignore-constants` |
| `component-file-only-components` | `warn`           | Component files should export components only.                                       | `project.structure.components`                           |
| `directory-must-have-barrel`     | `warn`           | Non-leaf directories must expose a barrel file.                                      | `barrel-names`                                           |
| `entry-file-no-logic`            | `warn`           | Entry files should delegate implementation logic to dedicated modules.               | `entry-files`                                            |
| `hook-no-jsx`                    | `warn`           | Hook files should not return or contain JSX.                                         | `project.structure.hooks`                                |
| `explicit-return-type`           | `warn`           | Require explicit return types on exported functions.                                 | `severity`                                               |
| `hook-prefix`                    | `info`           | Hook functions in hook files should use an allowed prefix, usually `use`.            | `prefixes`, `project.structure.hooks`                    |
| `layer-boundaries`               | `off`            | Enforce that imports respect ordered architectural layers.                           | `[architecture.layers]`                                  |
| `max-directory-depth`            | `warn`           | Limit nested directories below the configured project root.                          | `max-depth`, `ignore-dirs`                               |
| `max-file-exports`               | `warn`           | Limit the number of exports from one file.                                           | `max-exports`, `count-default`                           |
| `max-function-params`            | `warn`           | Limit function parameter count; prefer an object parameter.                          | `max-params`                                             |
| `max-items-per-directory`        | `warn`           | Prevent directories from becoming oversized collections.                             | `max-items`, `ignore-dirs`, `count-folders`              |
| `min-items-per-directory`        | `warn`           | Find tiny directories that add navigation cost without enough structure.             | `min-items`, `ignore-dirs`, `count-folders`              |
| `no-abbreviations`               | `info`           | Disallow abbreviated identifiers like `btn`, `ctx`, and `mgr`.                       | `extra-abbreviations`, `allow-abbreviations`             |
| `no-any`                         | `warn`           | Disallow explicit `any` type annotations outside generated or allowed folders.       | `allowed-folders`, `project.structure.generated`         |
| `no-await-in-loop`               | `warn`           | Forbid `await` inside loops — a common performance trap.                             | —                                                        | `severity`                     |
| `no-barrel-chain`                | `info`           | Prevent barrel files from re-exporting through other barrels.                        | `severity`                                               |
| `no-barrel-files`                | `info`           | Avoid barrel files.                                                                  | `barrel-names`                                           |
| `no-circular-import`             | `error`          | Detect circular import chains between modules.                                       | `severity`, `report-all-nodes`                           |
| `no-comments`                    | `info`           | Discourage implementation comments that duplicate code.                              | `allow-doc-comments`                                     |
| `no-console`                     | `warn`           | Keep debugging output out of application code.                                       | `severity`, `allow-patterns`                             |
| `no-debugger`                    | `error`          | Prevent committed `debugger` statements from stopping runtime execution.             | `severity`                                               |
| `no-default-export`              | `info`           | Prefer named exports. Set `components-only = true` to scope to component files only. | `components-only`, `project.structure.components`        |
| `no-duplicate-file-names`        | `warn`           | Avoid repeated file names that make editor tabs and stack traces ambiguous.          | `ignore-names`                                           |
| `no-dump-files`                  | `warn`           | Disallow generic file names such as `utils.ts`, `helpers.ts`, and `types.ts`.        | `extra-names`                                            |
| `no-empty-directories`           | `warn`           | Remove directories that no longer contain source files.                              | `ignore-dirs`                                            |
| `no-empty-domain`                | `warn`           | Domain folder must contain real source, not only barrels.                            | `ignore-dirs`                                            |
| `no-anemic-domain`               | `warn`           | Domain with too few files should be flattened.                                       | `max-files`, `ignore-dirs`                               |
| `no-god-domain`                  | `warn`           | Domain with too many files should be split.                                          | `max-files`, `ignore-dirs`                               |
| `no-empty-interface`             | `error`          | Avoid empty interfaces.                                                              | `severity`                                               |
| `no-enums`                       | `info`           | Prefer union types or const objects over TypeScript enums.                           | `severity`                                               |
| `no-eval`                        | `error`          | Block dynamic code execution through `eval()` and `new Function()`.                  | `severity`                                               |
| `no-export-star`                 | `warn`           | Require explicit re-exports instead of `export *`.                                   | `severity`                                               |
| `no-focused-test`                | `error`          | Disallow focused tests (`describe.only`, `it.only`, `test.only`).                    | `severity`                                               |
| `no-inline-types`                | `info`           | Keep exported contracts in type files or type folders.                               | `project.structure.types`                                |
| `no-interface`                   | `info`           | Prefer type aliases unless declaration merging is intentional.                       | `allow-declaration-merging`                              |
| `no-large-file`                  | `warn`           | Keep files under a configured line count.                                            | `max-lines`                                              |
| `no-logic-in-barrel`             | `warn`           | Keep barrel files limited to import/export forwarding.                               | `barrel-names`                                           |
| `no-logic-in-domain`             | `warn`           | Keep type and constants domains free of runtime implementation logic.                | `project.structure.types`, `project.structure.constants` |
| `no-mutable-exports`             | `error`          | Avoid exported mutable bindings.                                                     | `severity`                                               |
| `no-nested-functions`            | `warn`           | Disallow functions nested beyond a configured depth.                                 | `max-depth`, `contexts`                                  |
| `no-orphan-files`                | `warn`           | Detect files not imported by any other file in the project.                          | `entry-files`                                            |
| `no-promise-executor-return`     | `warn`           | Forbid returning a value from a Promise executor. Values are discarded.              | —                                                        | `severity`                     |
| `no-namespace`                   | `error`          | Prefer ES modules over TypeScript namespaces.                                        | `severity`                                               |
| `no-unsafe-optional-chaining`    | `warn`           | Forbid `?.` on expressions that are never null or undefined.                         | —                                                        | `severity`                     |
| `no-non-null-assertion`          | `error`          | Disallow the non-null assertion operator.                                            | `severity`                                               |
| `no-magic-numbers`               | `info`           | Disallow numeric and string literals outside constants.                              | `allowed-numbers`, `allowed-strings`, `enforce-strings`  |
| `no-package-cycle`               | `error`          | Detect circular dependencies between workspace packages.                             | `severity`                                               |
| `no-private-package-import`      | `warn`           | Prevent importing internal files from other packages.                                | `severity`                                               |
| `no-process-env`                 | `warn`           | Prevent direct access to `process.env`.                                              | `severity`                                               |
| `no-restricted-imports`          | `warn`           | Block imports from a configurable deny-list of packages or paths.                    | `restricted`                                             |
| `no-side-effect-imports`         | `info`           | Disallow bare side-effect imports like `import "./styles.css"`.                      | `severity`                                               |
| `no-silent-catch`                | `error`          | Require catch blocks to log, rethrow, or return a fallback.                          | `severity`                                               |
| `no-skipped-test`                | `error`          | Disallow skipped tests (`describe.skip`, `it.skip`, `test.skip`).                    | `severity`                                               |
| `no-test-code-in-production`     | `error`          | Disallow test globals and test library imports outside test files.                   | `project.structure.tests`                                |
| `no-test-import`                 | `error`          | Production code may not import test files.                                           | `project.structure.tests`                                |
| `no-then-chain`                  | `info`           | Prefer `async`/`await` over `.then()` chains.                                        | `severity`, `allow-single`                               |
| `no-type-assertion`              | `warn`           | Disallow `as` casts. Prefer type narrowing or `satisfies`.                           | `severity`                                               |
| `no-unnecessary-type-assertion`  | `warn`           | Flag `as T` when the expression is already typed as `T`.                             | `severity`                                               |
| `no-upward-import`               | `warn`           | Limit fragile `../` imports.                                                         | `max-depth`, `allow-patterns`                            |
| `prefer-satisfies`               | `info`           | Prefer `satisfies` over `as` when validating a value against a type.                 | `severity`                                               |
| `prefer-readonly`                | `warn`           | Prefer `readonly` for array parameters in exported functions.                        | `severity`                                               |
| `sort-exports`                   | `info`           | Enforce consistent export ordering by exported name. Fixable.                        | `severity`                                               |
| `sort-imports`                   | `info`           | Enforce consistent import ordering by module specifier. Fixable.                     | `severity`                                               |

## Language And TypeScript Rules

### `no-await-in-loop`

Reports `await` inside loop bodies (`for`, `for...of`, `for...in`, `while`, `do...while`). Await inside a loop serializes operations that could often be parallelized, which is a common performance trap.

```ts
async function process(items: Item[]) {
  for (const item of items) {
    await save(item);
  }
}
```

Prefer extracting to a separate function or using `Promise.all`:

```ts
async function process(items: Item[]) {
  await Promise.all(items.map((item) => save(item)));
}
```

### `no-any`

Reports explicit `any` type annotations.

```ts
const value: any = getData();
```

Prefer:

```ts
const value: unknown = getData();
```

Generated files are always exempt when they match `project.structure.generated`. Additional folders can be allowed with `allowed-folders`.

### `no-empty-interface`

Reports interfaces with no members.

```ts
interface Props {}
```

Prefer a type alias such as:

```ts
type Props = Record<string, never>;
```

### `no-interface`

Reports interfaces when type aliases should be used instead.

```ts
interface User {
  id: string;
}
```

Prefer:

```ts
type User = {
  id: string;
};
```

Set `allow-declaration-merging = true` to allow repeated interface declarations used for declaration merging.

### `no-enums`

Reports TypeScript enums.

```ts
enum Status {
  Open = "open",
}
```

Prefer const objects or union types.

### `no-namespace`

Reports TypeScript namespace declarations. Prefer ES modules.

### `no-non-null-assertion`

Reports the non-null assertion operator.

```ts
const value = obj!.prop;
```

Prefer optional chaining, guards, or explicit narrowing.

### `no-unsafe-optional-chaining`

Reports optional chaining (`?.`) on expressions that are never null or undefined. These include literal values, `new` expressions, arrow/function/class expressions, `this`, `new.target`, and non-null assertions.

```ts
const result = "hello"?.length;
const instance = new Foo()?.bar;
const value = onClick!?.();
```

Prefer removing the unnecessary `?.`:

```ts
const result = "hello".length;
const instance = new Foo().bar;
const value = onClick();
```

### `no-type-assertion`

Reports `as` casts. Type assertions bypass TypeScript's type checking and can hide bugs.

```ts
const value = something as string;
const config = { port: 3000 } as Config;
```

Prefer type narrowing or `satisfies`:

```ts
const value = something satisfies string;
const config = { port: 3000 } satisfies Config;

// Or use proper type narrowing
if (typeof value === "string") {
  // value is narrowed to string
}
```

### `no-unnecessary-type-assertion`

Reports `as T` casts where the expression is already typed as `T`, making the assertion redundant.

```ts
const name = "hello" as string;
const count = 42 as number;
const flag = true as boolean;
const empty = null as null;
const missing = undefined as undefined;
const big = 0n as bigint;
const message = `hello` as string;
```

All of these assertions are no-ops — the expression's type already matches the asserted type. Remove the `as` cast:

```ts
const name = "hello";
const count = 42;
const flag = true;
const empty = null;
const missing = undefined;
const big = 0n;
const message = `hello`;
```

### `prefer-satisfies`

Reports `as` casts where `satisfies` better expresses validation against a type.

```ts
const config = value as Config;
```

Prefer:

```ts
const config = value satisfies Config;
```

## Source Hygiene Rules

### `explicit-return-type`

Reports exported functions that lack an explicit return type annotation. Applies to named function exports, exported arrow functions, exported function expressions, and default exports.

```ts
export function add(a: number, b: number) {
  return a + b;
}

export const multiply = (a: number, b: number) => a * b;
```

Prefer:

```ts
export function add(a: number, b: number): number {
  return a + b;
}

export const multiply = (a: number, b: number): number => a * b;
```

### `no-console`

Reports console statements.

```ts
console.log(user);
```

Use `allow-patterns` with glob patterns for file paths that may contain console statements, such as scripts or logger adapters:

```toml
[rules.no-console]
severity = "warn"
allow-patterns = ["**/*.service.ts", "scripts/**"]
```

### `no-debugger`

Reports `debugger` statements.

**Supports autofix.** `niteo fix` removes `debugger` statements, their trailing semicolons, and trailing whitespace. See [Autofix](./fix.md) for details.

### `no-eval`

Reports `eval()` and `new Function()`.

### `no-promise-executor-return`

Reports returning a value from a Promise executor function. Return values from Promise executors are silently discarded; use `resolve()` or `reject()` instead.

```ts
new Promise((resolve, reject) => {
  return 42;
});

new Promise((resolve) => 42);
```

Prefer:

```ts
new Promise((resolve, reject) => {
  resolve(42);
});
```

### `no-process-env`

Reports direct access to `process.env`.

```ts
const key = process.env.API_KEY;
```

Prefer a centralized config module:

```ts
const key = config.apiKey;
```

### `no-comments`

Reports implementation comments. Documentation comments are allowed by default.

```toml
[rules.no-comments]
severity = "warn"
allow-doc-comments = true
```

### `no-silent-catch`

Reports catch blocks that silently ignore errors.

```ts
try {
  doWork();
} catch (error) {}
```

Catch blocks should log, rethrow, or return an intentional typed fallback.

### `no-abbreviations`

Reports identifiers that contain common abbreviations.

```ts
const btn = document.querySelector("button");
const ctx = getContext();
const mgr = new Manager();
```

Prefer fully spelled-out names:

```ts
const button = document.querySelector("button");
const context = getContext();
const manager = new Manager();
```

Add project-specific abbreviations:

```toml
[rules.no-abbreviations]
severity = "warn"
extra-abbreviations = ["req", "res", "tmp"]
```

Remove built-in abbreviations that are valid in your project:

```toml
[rules.no-abbreviations]
severity = "warn"
allow-abbreviations = ["btn", "ctx"]
```

### `no-then-chain`

Reports `.then()` chains. With `allow-single` enabled (default), only flags `.then()` calls that are part of a chain (follow another `.then()`) or followed by `.catch()` / `.finally()`. Set `allow-single = false` to flag every `.then()` call.

```ts
fetch("/api")
  .then((r) => r.json())
  .then((d) => process(d));
```

Prefer:

```ts
const r = await fetch("/api");
const d = await r.json();
process(d);
```

### `no-focused-test`

Reports focused test helpers that cause the test runner to skip the rest of the suite. Focused tests are useful during development but should never be committed.

```ts
describe.only("auth", () => {
  it.only("logs in", () => {});
});
```

Prefer:

```ts
describe("auth", () => {
  it("logs in", () => {});
});
```

### `no-skipped-test`

Reports skipped test helpers that silently bypass tests. Skipped tests hide failures and should be removed or fixed rather than left in the suite.

```ts
describe.skip("auth", () => {
  it.skip("logs in", () => {});
});
```

Prefer:

```ts
describe("auth", () => {
  it("logs in", () => {});
});
```

### `no-nested-functions`

Reports functions defined inside other functions beyond `max-depth` nesting levels. Deeply nested functions are hard to test and reason about. Extract them to module scope.

```ts
function outer() {
  function middle() {
    function inner() {} // reported when max-depth = 2
  }
}
```

Prefer:

```ts
function inner() {}
function middle() {
  inner();
}
function outer() {
  middle();
}
```

Configure the allowed nesting depth and which function-like constructs count toward it:

```toml
[rules.no-nested-functions]
severity = "warn"
max-depth = 2
contexts = ["function", "arrow", "class-method", "object-method"]
```

The `contexts` option controls which constructs increment the nesting counter. Each construct type you include counts as one nesting level. Exclude a context to allow that construct without contributing to depth — useful in React codebases where arrow callbacks in `.map()` or methods in object/class literals shouldn't force refactors.

Available contexts: `function`, `arrow`, `class-method`, `object-method`. Default is all four.

## Export And Module Shape Rules

### `no-default-export`

Reports default exports. Prefer named exports.

```ts
export default function Button() {}
```

Prefer:

```ts
export function Button() {}
```

Set `components-only` to limit enforcement to component files only (identified by `project.structure.components`):

```toml
[rules.no-default-export]
severity = "warn"
components-only = true
```

### `no-mutable-exports`

Reports exported mutable bindings.

```ts
export let currentUser = null;
```

### `max-file-exports`

Reports files with more exports than `max-exports`. When `count-default` is `false`,
default exports are excluded from the count.

```toml
[rules.max-file-exports]
severity = "warn"
max-exports = 10
count-default = true
```

### `max-function-params`

Reports functions with more parameters than `max-params`. Functions with many parameters are hard to call correctly and often benefit from an object parameter.

```ts
function createUser(name: string, age: number, email: string, role: string) {}
```

Prefer:

```ts
function createUser(options: {
  name: string;
  age: number;
  email: string;
  role: string;
}) {}
```

Configure the allowed parameter count:

```toml
[rules.max-function-params]
severity = "warn"
max-params = 3
```

### `prefer-readonly`

Reports array parameters in exported functions that are not marked `readonly`. Exported functions form a public API; marking array parameters `readonly` prevents accidental mutation of caller data.

```ts
export function process(items: string[]) {}
export function merge(a: Array<number>) {}
```

Prefer:

```ts
export function process(items: readonly string[]) {}
export function merge(a: ReadonlyArray<number>) {}
```

### `no-export-star`

Reports `export *`.

```ts
export * from "./Button";
```

Prefer explicit re-exports.

### `no-barrel-files`

Reports barrel files. Barrel file names are configurable via `barrel-names`.

### `no-barrel-chain`

Reports barrel files that re-export through another barrel.

### `no-logic-in-barrel`

Reports runtime logic in barrel files. Barrel files should only forward imports and exports. Barrel file names are configurable via `barrel-names`.

### `sort-exports`

Reports export declarations not in alphabetical order by exported name. Default exports sort first. Groups separated by blank lines are sorted independently. This rule supports autofix.

```ts
// ❌ Reports
export const c = 1;
export const a = 2;
export const b = 3;
```

```ts
// ✅ Prefer
export const a = 2;
export const b = 3;
export const c = 1;
```

## File And Directory Rules

### `no-large-file`

Reports files longer than `max-lines`.

```toml
[rules.no-large-file]
severity = "warn"
max-lines = 500
```

### `no-dump-files`

Reports generic dumping-ground file names such as `utils.ts`, `helpers.ts`, and `types.ts`.

Add project-specific names:

```toml
[rules.no-dump-files]
severity = "warn"
extra-names = ["misc", "common"]
```

### `no-duplicate-file-names`

Reports repeated file names in different directories.

```toml
[rules.no-duplicate-file-names]
severity = "warn"
ignore-names = ["index.ts"]
```

### `no-empty-directories`

Reports directories that do not contain source files.

### `directory-must-have-barrel`

Reports non-leaf directories (those containing child folders) that do not have a direct barrel file. Barrel files provide a single import surface for the directory's public API. Barrel file names are configurable via `barrel-names`.

```toml
[rules.directory-must-have-barrel]
severity = "warn"
barrel-names = ["index.ts", "index.tsx"]
```

### `max-items-per-directory`

Reports directories with more source items than `max-items`.

```toml
[rules.max-items-per-directory]
severity = "warn"
max-items = 20
ignore-dirs = ["__tests__"]
count-folders = false
```

### `min-items-per-directory`

Reports directories with fewer source items than `min-items`.

### `max-directory-depth`

Reports files nested deeper than `max-depth` below the project root.

```toml
[rules.max-directory-depth]
severity = "warn"
max-depth = 5
ignore-dirs = ["generated"]
```

### `no-empty-domain`

Reports domain folders that contain only barrel files (`index.ts` with re-exports) and no real source files. Such folders add navigation overhead without meaningful content.

```toml
[rules.no-empty-domain]
severity = "warn"
ignore-dirs = []
```

### `no-anemic-domain`

Reports domain folders with too few source files. A folder with only one or two files often adds navigation cost without a clear structural boundary.

```toml
[rules.no-anemic-domain]
severity = "warn"
max-files = 1
ignore-dirs = []
```

### `no-god-domain`

Reports domain folders with too many source files. Oversized domains are hard to navigate and often benefit from sub-grouping.

```toml
[rules.no-god-domain]
severity = "warn"
max-files = 20
ignore-dirs = []
```

## Domain Rules

### `no-inline-types`

Reports exported type contracts outside type files or type folders. Declaration files (`.d.ts`) are allowed.

### `no-logic-in-domain`

Reports runtime implementation logic in files identified as type or constants domain files.

### `component-file-only-components`

Reports non-component exports from component files.

### `hook-no-jsx`

Reports JSX in hook files.

### `hook-prefix`

Reports functions in hook files that do not start with an allowed prefix.

```toml
[rules.hook-prefix]
severity = "warn"
prefixes = ["use"]
```

### `boolean-prefix`

Reports boolean variables that do not start with an allowed prefix.

```toml
[rules.boolean-prefix]
severity = "warn"
prefixes = ["is", "has", "can"]
ignore-constants = false
```

### `entry-file-no-logic`

Reports implementation logic in entry files. Default entry file stems are `main`, `app`, `layout`, and `page`.

```toml
[rules.entry-file-no-logic]
severity = "warn"
entry-files = ["main", "app", "layout", "page"]
```

## Import Rules

### `no-circular-import`

Reports circular import chains between modules. Circular imports can cause runtime initialization issues and make module dependencies harder to reason about. By default, only one file per cycle is reported (the alphabetically first). Set `report-all-nodes = true` to flag every file participating in a cycle.

```ts
// a.ts
import { b } from "./b";
// b.ts
import { a } from "./a";
```

Prefer breaking the cycle by extracting shared logic to a third module:

```ts
// shared.ts
export const shared = compute();
// a.ts
import { shared } from "./shared";
// b.ts
import { shared } from "./shared";
```

### `no-upward-import`

Reports imports with more upward `../` segments than `max-depth`.

```toml
[rules.no-upward-import]
severity = "warn"
max-depth = 0
allow-patterns = []
```

To permit upward imports from generated directories: `allow-patterns = ["**/generated/**"]`.

### `layer-boundaries`

Enforces that imports respect an ordered set of architectural layers. Each layer may only import from layers at or below its position. Layers are defined in the `[architecture.layers]` section of `niteo.toml`.

```ts
// src/shared/date.ts — layer "shared" (lowest position)
import { getSession } from "@/features/auth/session"; // ❌ shared cannot import features
```

This rule is disabled by default. To use it, configure your layers and enable the rule:

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

[rules.layer-boundaries]
severity = "warn"
```

With this configuration, `app` can import from `features`, `entities`, and `shared`, while `shared` may not import from any higher layer.

### `no-package-cycle`

Reports circular dependencies between workspace packages. Requires a workspace configuration (`package.json` workspaces or `pnpm-workspace.yaml`). Disabled when no workspace is detected.

```toml
[rules.no-package-cycle]
severity = "warn"
```

A cycle exists when package A depends on package B and package B depends (directly or transitively) on package A. These cycles can cause initialization deadlocks and make dependency relationships unclear.

```
packages/a -> packages/b -> packages/c -> packages/a  ❌
```

### `no-private-package-import`

Reports imports from another package's internal files rather than its public API. Requires a workspace configuration. Disabled when no workspace is detected.

```ts
import { helper } from "@scope/admin/src/internal/utils"; // ❌
import { helper } from "@scope/admin"; // ✅ public exports only
```

Each package's public API is determined by its `exports`, `main`, or `module` fields in `package.json`, falling back to `src/index.ts` or `index.ts`.

```toml
[rules.no-private-package-import]
severity = "warn"
```

### `no-restricted-imports`

Reports imports from packages or paths listed in the `restricted` deny-list. Supports glob patterns, named-import restrictions, and per-pattern custom messages. Matches exact names and sub-paths (e.g. `lodash` also blocks `lodash/fp`).

```ts
import { merge } from "lodash";
```

Prefer an allowed alternative:

```ts
import merge from "./utils/merge";
```

Configure the deny-list:

```toml
[rules.no-restricted-imports]
severity = "warn"
restricted = ["lodash", "moment", "@internal/legacy"]
```

Use **glob patterns** with `*`, `**`, or `?` for more flexible matching:

```toml
[rules.no-restricted-imports]
severity = "warn"
restricted = ["internal/*", "@scope/**", "lib-v?"]
```

Restrict **specific named imports** from a module without blocking the entire package:

```toml
[[rules.no-restricted-imports.restricted]]
pattern = "my-lib"
named = ["deprecated", "oldFn"]
```

Add a **custom message** per pattern to suggest an alternative:

```toml
[[rules.no-restricted-imports.restricted]]
pattern = "lodash"
named = ["merge"]
message = "Use our custom merge utility instead."
```

The rule also checks re-exports (`export { x } from '...'` and `export * from '...'`).

### `no-test-import`

Reports production files importing test files.

### `no-test-code-in-production`

Reports test globals such as `describe`, `it`, `test`, and `expect`, plus test library imports outside files identified by `project.structure.tests`.

### `no-orphan-files`

Reports files that are not imported by any other file in the project. Orphan files may indicate dead code or missing imports.

Entry files (such as `main.ts`, `app.tsx`) and test files are exempt by default. Configure additional entry file stems:

```toml
[rules.no-orphan-files]
severity = "warn"
entry-files = ["main", "app", "layout", "page", "index"]
```
