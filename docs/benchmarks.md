# Benchmarks

Niteo uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for
performance measurement.

## Running Benchmarks

```sh
cargo bench
cargo bench --bench cli_benchmarks
cargo bench --bench parallelism_benchmark
```

Criterion stores results in `target/criterion/`. Open
`target/criterion/report/index.html` to browse the comparison report.

## Benchmark Groups

### `cli`

End-to-end subprocess benchmarks. Each benchmark creates a temporary generated
TypeScript project, then shells out to the `niteo` binary and measures
wall-clock time for the full invocation (process startup, config discovery,
filesystem traversal, linting, reporting).

#### Size matrix — react project

| Benchmark                        | Files | Directories |
| -------------------------------- | ----- | ----------- |
| `cli_lint_react_25_files_json`   | 25    | 4           |
| `cli_lint_react_100_files_json`  | 100   | 4           |
| `cli_lint_react_250_files_json`  | 250   | 4           |
| `cli_lint_react_1000_files_json` | 1000  | 4           |

All use `--format json`. The react project has components (TSX), utils (TS),
hooks (TS), barrel exports, and imports from `react`.

#### Cache size matrix — react project

| Benchmark                                   | Files | Cache state |
| ------------------------------------------- | ----- | ----------- |
| `cli_lint_react_25_files_json_cache_warm`   | 25    | warm        |
| `cli_lint_react_25_files_json_cache_cold`   | 25    | cold        |
| `cli_lint_react_100_files_json_cache_warm`  | 100   | warm        |
| `cli_lint_react_100_files_json_cache_cold`  | 100   | cold        |
| `cli_lint_react_250_files_json_cache_warm`  | 250   | warm        |
| `cli_lint_react_250_files_json_cache_cold`  | 250   | cold        |
| `cli_lint_react_1000_files_json_cache_warm` | 1000  | warm        |
| `cli_lint_react_1000_files_json_cache_cold` | 1000  | cold        |

Warm benchmarks run `lint --cache` once to populate `.niteo/cache.json`, then
measure repeated warm invocations. Cold benchmarks delete the cache file before
each measured invocation.

#### Output formats — react project

| Benchmark                        | Command                     |
| -------------------------------- | --------------------------- |
| `cli_lint_react_100_files_text`  | `niteo lint`                |
| `cli_lint_react_100_files_sarif` | `niteo lint --format sarif` |

All use a 100-file react project. JSON output is covered in the size matrix above.

#### Size matrix — import-heavy project

| Benchmark                              | Files | Imports per file |
| -------------------------------------- | ----- | ---------------- |
| `cli_lint_import_heavy_50_files_json`  | 50    | ~5–16            |
| `cli_lint_import_heavy_100_files_json` | 100   | ~5–16            |
| `cli_lint_import_heavy_200_files_json` | 200   | ~5–16            |

Import-heavy projects have a dense import graph with a barrel re-exporting every
file. This stresses import graph construction and resolver index size.

#### Cache size matrix — import-heavy project

| Benchmark                                         | Files | Cache state |
| ------------------------------------------------- | ----- | ----------- |
| `cli_lint_import_heavy_50_files_json_cache_warm`  | 50    | warm        |
| `cli_lint_import_heavy_50_files_json_cache_cold`  | 50    | cold        |
| `cli_lint_import_heavy_100_files_json_cache_warm` | 100   | warm        |
| `cli_lint_import_heavy_100_files_json_cache_cold` | 100   | cold        |
| `cli_lint_import_heavy_200_files_json_cache_warm` | 200   | warm        |
| `cli_lint_import_heavy_200_files_json_cache_cold` | 200   | cold        |

### `check_files_parallelism`

In-process benchmarks in `benches/parallelism_benchmark.rs` that measure the
`check_files_for_benchmark` function from `src/rules_runner.rs`. Each benchmark
builds a full project scaffold (files, config, import graph, workspace) once,
then times the linting pass with parallelism toggled on or off. This isolates
the Rayon parallelization overhead from CLI startup and reporting.

| Benchmark                          | Files | Mode           |
| ---------------------------------- | ----- | -------------- |
| `single_threaded_100`              | 100   | single-threaded|
| `multi_threaded_100`               | 100   | multi-threaded |
| `single_threaded_500`              | 500   | single-threaded|
| `multi_threaded_500`               | 500   | multi-threaded |
| `single_threaded_1000`             | 1000  | single-threaded|
| `multi_threaded_1000`              | 1000  | multi-threaded |
| `single_threaded_2000`             | 2000  | single-threaded|
| `multi_threaded_2000`              | 2000  | multi-threaded |

Run with:

```sh
cargo bench --bench parallelism_benchmark
```

## Fixture Design

All fixtures are generated inside `TempDir` before the timed loop so file
creation cost is excluded from measurements.

### React project (`write_react_project`)

- `src/components/`: N TSX component files with `react` imports, JSX, and
  local util imports.
- `src/utils/`: N/2 TS util files with arithmetic helpers.
- `src/hooks/`: N/4 TS hook files with `useState`/`useEffect`.
- Barrels: `src/components/index.ts` and `src/utils/index.ts` with re-export
  chains.
- One `niteo.toml` at the project root.

### Import-heavy project (`write_import_heavy_project`)

- `src/`: N TS util files, each with 5–16 imports to other files in the same
  directory.
- Barrel: `src/index.ts` re-exports every file.
- One `niteo.toml` at the project root.

## Local Workflow

1. Establish a baseline on `main`:

   ```sh
   git checkout main
   cargo bench --bench cli_benchmarks
   ```

2. Switch to your branch and run again:

   ```sh
   cargo bench --bench cli_benchmarks
   ```

3. Compare results:

   ```sh
   open target/criterion/report/index.html
   ```

## When To Add Benchmarks

Add a benchmark when:

- A rule's worst-case complexity could cause noticeable regression.
- A new feature changes the hot path (config resolution, file discovery, import
  graph construction, reporting).
- A refactor alters the algorithm behind an existing cost center.

## Future Work

- In-process benchmarks (`lint_pipeline`, `discovery`, `syntax`, `rules`,
  `reporting`, `graph` groups) once the internal API boundaries are clean
  enough.
- Additional fixture profiles: noisy (many violations), barrel chain (deep
  re-export chains), monorepo (nested configs), suppression-heavy.
- CI integration that stores benchmark trends without blocking PRs.
