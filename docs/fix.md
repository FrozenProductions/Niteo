# Autofix

Niteo can automatically fix a small, conservative set of structural violations. Autofix is always opt-in — it never runs during normal linting.

## Quick Start

```sh
niteo fix                 # Apply fixes to files
niteo fix --dry-run       # Preview fixes without writing
niteo lint --fix           # Lint, then apply fixes
```

## Pipeline

`niteo fix` runs the following pipeline:

1. **Analyze.** Calls `analysis::collect()` — the same full analysis pass used by `niteo lint` — to gather violations across the project. Discovery, config resolution, baseline loading, and suppression handling are all shared.
2. **Filter fixable files.** Collects the set of files that have violations from rules whose metadata has a non-`None` `fix_capability` (`Safe` or `Conditional`) and whose `[fix]` gate allows the rule. Files without allowed fixable violations are skipped entirely.
3. **Build rules per file.** For each fixable file, resolves the effective config and builds rule adapters. Only rules that are both enabled (severity not `off`) and support fix are invoked.
4. **Parse AST if needed.** If any fixable rule for the file requires an AST, the file is parsed once with oxc. The parsed program is shared across all rules for that file.
5. **Collect fixes.** Calls `fix::collect_fixes()`, which iterates the enabled fixable rules and calls `rule.fix(&ctx)`. Each rule returns `Vec<Fix>`; these are collected into one flat list.
6. **Apply fixes.** Calls `fix::apply_fixes()`, which groups edits by file, checks for overlaps, applies edits in reverse byte order, validates source hasn't changed on disk, and writes the result.
7. **Prune baseline.** Re-reads the baseline file and removes entries whose violations were fixed. This keeps the baseline accurate without a separate `niteo baseline prune` step.

### Architecture

```
cli.rs: Command::Fix { dry_run }
   │
   ▼
app.rs → commands::fix::fix_workspace()
   │
   ├── analysis::collect()           # full lint pass
   ├── baseline::read_baseline()     # filter known violations
   ├── filter fixable files          # from violation → rule metadata
   ├── for each file:
   │   ├── config_for_file()
   │   ├── build_file_rules()
   │   ├── parse AST
   │   ├── fix::collect_fixes()      # rule.fix() on enabled fixable rules
   │   └── accumulate Vec<Fix>
   ├── if dry_run → fix::report_dry_run()
   │   └── return
   ├── fix::apply_fixes()
   │   └── returns FixOutcome
   └── baseline::prune()             # remove stale entries
```

## How It Works

`niteo fix` runs a full analysis pass to collect violations, then filters to rules that support autofix and are allowed by `niteo.toml`. For each fixable violation, the rule produces one or more `TextEdit` values — byte offsets and replacement text. Niteo applies these edits to the source file and writes the result back to disk.

## Per-rule Gates

Use the `[fix]` table in `niteo.toml` to allow or block autofix per rule:

```toml
[fix]
no-any = false
no-non-null-assertion = false
```

Rules default to `true`, so existing projects keep the same behavior until a rule is explicitly set to `false`. A `false` entry disables only the autofix for that rule; diagnostics still run according to the rule's `[rules.<rule>]` severity.

The gate applies to both `niteo fix` and `niteo lint --fix`. In monorepos, child `niteo.toml` files inherit the parent `[fix]` table and can override individual rule entries:

```toml
# root niteo.toml
[fix]
no-any = false
no-non-null-assertion = false
```

```toml
# packages/app/niteo.toml
[fix]
no-any = true
```

With this setup, `no-any` autofix is disabled by default but re-enabled inside `packages/app`.

### Edit Model

Edits are applied in reverse byte order (from end of file to beginning) so that earlier edits do not shift the byte offsets of later edits. This ensures correct application even when multiple edits target the same file.

The core edit engine in `src/fix.rs` provides:

| Function                      | Purpose                                                                                                                                                          |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `apply_fixes(fixes, options)` | Groups edits by file, sorts by start offset, checks for overlap, applies edits, validates source freshness and parseability, writes files. Returns `FixOutcome`. |
| `apply_edits(source, edits)`  | Pure function that applies `TextEdit` values in reverse order to a source string.                                                                                |
| `collect_fixes(ctx, rules)`   | Iterates rules, calling `rule.fix()` only on enabled fixable rules.                                                                                              |
| `has_overlap(edits)`          | Checks if any adjacent edits in sorted-by-start order overlap.                                                                                                   |
| `report_dry_run(fixes)`       | Prints each edit as `file: rule\n  would replace bytes N-M with "replacement"`.                                                                                  |

### Data Types

```rust
pub struct Fix {
    pub file: PathBuf,
    pub rule: RuleId,
    pub edits: Vec<TextEdit>,
}

pub struct TextEdit {
    pub start: usize,       // byte offset, inclusive
    pub end: usize,         // byte offset, exclusive
    pub replacement: String,
}

pub struct FixOutcome {
    pub fixed_files: Vec<PathBuf>,
    pub rejected_overlapping: usize,
    pub rejected_stale: usize,
}
```

Edits use byte offsets because oxc's AST spans use byte positions. This handles multibyte characters (Unicode) correctly as long as the offsets come from the parser's AST spans.

### Safety Guards

**Overlap rejection.** Edits for each file are sorted by start offset. If any adjacent pair has `edit_A.end > edit_B.start`, all edits for that file are rejected. This prevents corrupt output from conflicting changes. Niteo prints a warning with the count of rejected edits.

**Stale source detection.** Before writing, Niteo re-reads the file from disk. If the on-disk content differs from the source string that was analyzed, all edits for that file are rejected. This prevents applying edits computed against a stale version of the source (e.g. the file was modified externally between analysis and write).

**Dry run.** Pass `--dry-run` to see what fixes would be applied without writing any files. Each edit is printed with the rule that produced it:

```
src/foo.ts: no-debugger
  would replace bytes 12-20 with ""
src/bar.ts: no-focused-test
  would replace bytes 5-11 with ""
```

### Output

`niteo fix` prints one line per fixed file:

```
Fixed src/foo.ts
Fixed src/bar.ts
Fixed 2 file(s).
```

If overlaps, stale source, invalid edits, or parse validation failures are rejected, warnings go to stderr:

```
warning: rejected overlapping edits in src/foo.ts from no-empty-interface and prefer-satisfies
warning: rejected 2 invalid edits
warning: rejected 1 edit because fixed source would not parse
```

After all fixes are processed, a summary with the total number of rejected edits is also printed.

After fixing, if baseline entries were pruned:

```
Pruned 2 stale baseline entries
```

If no fixable violations are found:

```
No fixable violations found.
```

If all calculated fixes were rejected:

```
No fixes to apply.
```

## Fixable Rules

The following rules support autofix. Each rule is classified by a capability level:

- **Safe.** The fix is a local, mechanical removal that is unlikely to change semantics. These fixes are applied automatically.
- **Conditional.** The fix may change semantics in edge cases, so it is only applied when the rule can prove the narrow safe subset applies. The user should still review the result.

| Rule                    | Capability  | Fix behavior                                                                                                                                                                                                                                                             |
| ----------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `no-debugger`           | Safe        | Removes the `debugger` statement, plus its trailing semicolon if present and any trailing whitespace up to the next line. Surrounding code stays parseable.                                                                                                              |
| `no-focused-test`       | Safe        | Removes `.only` from `describe.only`, `it.only`, and `test.only` calls. Aliases and non-test calls are not modified.                                                                                                                                                     |
| `no-skipped-test`       | Safe        | Removes `.skip` from `describe.skip`, `it.skip`, and `test.skip` calls. The test body is preserved; aliases and non-test calls are not modified.                                                                                                                         |
| `no-empty-interface`    | Conditional | Converts a simple empty interface into a `type` alias using `Record<string, never>`. Exported interfaces keep their `export` keyword. Does not fix interfaces with `extends`, `.d.ts` files, ambient declarations, merged declarations, or bodies that contain comments. |
| `prefer-satisfies`      | Conditional | Replaces `as` with `satisfies` for literal expressions cast with `as`. Preserves the type annotation. Does not modify `as const`, `as any`, or `as unknown` casts.                                                                                                       |
| `no-any`                | Conditional | Replaces `any` type keyword with `unknown`. Applies to type annotations, generics, and type references. Skips generated files and configured allowed folders.                                                                                                            |
| `no-process-env`        | Conditional | Adds `// niteo-ignore-line: no-process-env` to the end of each line with a `process.env` access. Deduplicates multiple accesses on the same line.                                                                                                                        |
| `prefer-readonly`       | Safe        | Inserts `readonly` keyword before mutable array types (`string[]` → `readonly string[]`, `Array<T>` → `readonly Array<T>`) in exported function parameters and rest parameters.                                                                                          |
| `no-non-null-assertion` | Conditional | Removes the `!` non-null assertion operator. Converts `obj!.prop` to `obj.prop` and `fn()!` to `fn()`. Removes each `!` independently for nested assertions.                                                                                                             |
| `sort-exports`          | Safe        | Reorders export declarations alphabetically by exported name. Default exports sort first. Groups separated by blank lines are sorted independently.                                                                                                                      |
| `sort-imports`          | Safe        | Reorders import declarations alphabetically by module specifier. Groups separated by blank lines are sorted independently.                                                                                                                                               |

Rules that **do not** support autofix today include structural changes like import path rewrites, default export conversion, broad interface-to-type conversion, and file moves. These are harder to implement safely and may never have autofix support.

## Baselines

After applying fixes, `niteo fix` prunes the baseline file. Any baseline entries for violations that were fixed are removed. This keeps the baseline accurate without requiring a separate `niteo baseline prune` step.

If a fix changes the violation in a way that doesn't match the original baseline entry (different line, column, or subject), the baseline entry becomes stale. The prune step handles this automatically.

## `lint --fix`

The `--fix` flag on `lint` runs a normal lint pass followed by autofix:

```sh
niteo lint --fix
```

The lint report prints first, then fixes are applied. The command exits with the same status code as lint would without `--fix`. If lint reports violations and `--fail-on` triggers, the command still fails — even if fixes were applied.

`lint --fix` delegates to the same `commands::fix::fix_workspace()` pipeline as `niteo fix`. It always runs with `dry_run: false`.

## Adding Fix Support To A Rule

Adding autofix to a rule touches three locations:

### 1. Implement `fix_file()` in the rule module

In `src/rules/<rule>.rs`, add a `pub fn fix_file()` that returns `Vec<Fix>`:

```rust
pub fn fix_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    source: &str,
    config: &RuleConfig,
) -> Vec<Fix> {
    // Skip if disabled
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    // Collect AST spans to fix
    let mut collector = MySpanCollector { spans: Vec::new() };
    collector.visit_program(program);

    // Build TextEdit values
    let edits: Vec<TextEdit> = collector.spans.iter().map(|span| {
        TextEdit {
            start: span.start as usize,
            end: span.end as usize,
            replacement: String::new(),
        }
    }).collect();

    if edits.is_empty() {
        return Vec::new();
    }

    vec![Fix {
        file: file.to_path_buf(),
        rule: MY_RULE_ID,
        edits,
    }]
}
```

The function receives the file path, the fully-parsed AST, the raw source text (useful for inspecting characters after a span), and the rule config. It returns `Vec<Fix>`, where each `Fix` contains the file path, rule ID, and a list of `TextEdit` values.

### 2. Use a fixable adapter macro

In `src/rule_adapters.rs`, replace `ast_rule_adapter!` with `fixable_ast_rule_adapter!`:

```rust
// Before — check only
ast_rule_adapter!(MyRuleAdapter, MY_RULE_ID, crate::config::RuleConfig, my_rule);

// After — check + fix
fixable_ast_rule_adapter!(
    MyRuleAdapter,
    MY_RULE_ID,
    crate::config::RuleConfig,
    my_rule,
    FixCapability::Safe
);
```

The fixable adapter automatically generates `fix_capability()` and `fix()` that calls your rule's `fix_file()`. Use `FixCapability::Safe` for mechanical local removals and `FixCapability::Conditional` for fixes that may change semantics in edge cases.

For text-only rules (no AST), a future `fixable_text_rule_adapter!` would follow the same pattern.

### 3. Set metadata

In `src/config/rule_metadata.rs`, set the rule's `fix_capability`:

```rust
RuleMetadata {
    id: "my-rule",
    // ...
    fix_capability: FixCapability::Safe,
}
```

Disabled rules are skipped before `fix_file()` is called. Your `fix_file()` function should still check `config.severity.is_enabled()` as a defensive guard.

### Testing

Add unit tests in the rule module that verify:

- Fix removes the expected code
- Fix preserves surrounding code
- Fix produces no edits when no violations exist
- Fix with `severity: off` returns empty

Run `cargo test` to confirm everything passes.

## Limitations

- **No formatter integration.** Fixed code may look awkward without a separate formatting pass.
- **Conservative scope.** Only safe, mechanical fixes are supported. Most rules will never have autofix.
- **Byte-offset model.** Edits use byte offsets from the original parse, which means multibyte characters (Unicode) are handled correctly as long as the offsets come from the parser's AST spans.
