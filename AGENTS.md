# Niteo Project Guidance

## Scope

- Niteo is a standalone Rust CLI for TypeScript structural linting.

## Rust guidelines

- Prioritize correctness and clarity over speed.
- Prefer existing files unless a new logical component is clearly needed.
- Avoid `unwrap()`, `expect()`, and panic-prone indexing; propagate errors with `?`.
- Never silently discard fallible results with `let _ =`.
- Use full words for variable names.
- Keep comments rare and only use them to explain non-obvious intent or constraints.
- Avoid `mod.rs`; prefer `src/name.rs` for modules.
- Keep changes small and local.

## Change habits

- Use explicit types where they improve clarity.
- Prefer one focused module over many tiny files.
- Do not add architectural churn unless the change requires it.
- Remove dead code and unused exports when touching a file.

## Build and verification

- Run `cargo fmt` before finishing Rust edits.
- Run `cargo check` after structural changes.
- Prefer `cargo test` when tests exist or are added.
- Keep unit tests inside the file they exercise under `#[cfg(test)]`.
- Put non-unit tests in the `tests/` directory.

## Product shape

- Keep the CLI standalone.
- Keep config, discovery, and reporting as separate concerns.
- Favor project-wide context over file-isolated assumptions when adding rules later.
