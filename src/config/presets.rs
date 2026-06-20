use crate::cli::PresetName;

pub fn default_config_for_preset(preset: PresetName) -> &'static str {
    match preset {
        PresetName::Balanced => BALANCED_PRESET,
        PresetName::Strict => STRICT_PRESET,
        PresetName::Migration => MIGRATION_PRESET,
        PresetName::React => REACT_PRESET,
        PresetName::Library => LIBRARY_PRESET,
        PresetName::NoBarrels => NO_BARRELS_PRESET,
    }
}

const BALANCED_PRESET: &str = r#"[project]
root = "src"
respect-gitignore = true

[rules]
[rules.no-console]
severity = "warn"
allow-patterns = []

[rules.no-debugger]
severity = "error"

[rules.no-eval]
severity = "error"

[rules.no-empty-interface]
severity = "error"

[rules.no-export-star]
severity = "warn"

[rules.no-focused-test]
severity = "error"

[rules.no-skipped-test]
severity = "warn"

[rules.no-test-code-in-production]
severity = "error"

[rules.no-mutable-exports]
severity = "error"

[rules.no-namespace]
severity = "error"

[rules.no-silent-catch]
severity = "warn"

[rules.no-non-null-assertion]
severity = "warn"

[rules.explicit-return-type]
severity = "warn"

[rules.no-circular-import]
severity = "error"

[rules.no-large-file]
severity = "info"
max-lines = 500

[rules.max-function-params]
severity = "warn"
max-params = 3

[rules.no-barrel-files]
severity = "off"

[rules.no-barrel-chain]
severity = "off"

[rules.no-logic-in-barrel]
severity = "off"
"#;

const STRICT_PRESET: &str = r#"[project]
root = "src"
respect-gitignore = true

[rules]
[rules.no-console]
severity = "warn"
allow-patterns = []

[rules.no-debugger]
severity = "error"

[rules.no-eval]
severity = "error"

[rules.no-empty-interface]
severity = "error"

[rules.no-export-star]
severity = "error"

[rules.no-focused-test]
severity = "error"

[rules.no-skipped-test]
severity = "error"

[rules.no-test-code-in-production]
severity = "error"

[rules.no-mutable-exports]
severity = "error"

[rules.no-namespace]
severity = "error"

[rules.no-silent-catch]
severity = "error"

[rules.no-non-null-assertion]
severity = "error"

[rules.no-type-assertion]
severity = "error"

[rules.explicit-return-type]
severity = "error"

[rules.no-circular-import]
severity = "error"

[rules.no-large-file]
severity = "warn"
max-lines = 300

[rules.max-function-params]
severity = "warn"
max-params = 3

[rules.no-barrel-files]
severity = "error"

[rules.no-barrel-chain]
severity = "error"

[rules.no-logic-in-barrel]
severity = "error"

[rules.no-magic-numbers]
severity = "warn"
allowed-numbers = ["0", "1", "-1"]

[rules.no-any]
severity = "error"
allowed-folders = []

[rules.no-abbreviations]
severity = "warn"
extra-abbreviations = []

[rules.prefer-readonly]
severity = "warn"

[rules.no-enums]
severity = "warn"

[rules.no-interface]
severity = "warn"
allow-declaration-merging = true

[rules.no-inline-types]
severity = "warn"

[rules.no-logic-in-domain]
severity = "warn"

[rules.max-directory-depth]
severity = "warn"
max-depth = 5
ignore-dirs = []

[rules.no-duplicate-file-names]
severity = "warn"
ignore-names = []

[rules.no-dump-files]
severity = "warn"
extra-names = []

[rules.no-empty-directories]
severity = "warn"
ignore-dirs = []

[rules.no-process-env]
severity = "error"

[rules.no-restricted-imports]
severity = "warn"
restricted = []
"#;

const MIGRATION_PRESET: &str = r#"[project]
root = "src"
respect-gitignore = true

[rules]
[rules.no-console]
severity = "off"

[rules.no-debugger]
severity = "error"

[rules.no-eval]
severity = "error"

[rules.no-empty-interface]
severity = "error"

[rules.no-export-star]
severity = "off"

[rules.no-focused-test]
severity = "error"

[rules.no-skipped-test]
severity = "off"

[rules.no-test-code-in-production]
severity = "error"

[rules.no-mutable-exports]
severity = "error"

[rules.no-namespace]
severity = "error"

[rules.no-silent-catch]
severity = "off"

[rules.no-non-null-assertion]
severity = "off"

[rules.no-type-assertion]
severity = "off"

[rules.explicit-return-type]
severity = "off"

[rules.no-circular-import]
severity = "error"

[rules.no-large-file]
severity = "info"
max-lines = 800

[rules.max-function-params]
severity = "info"
max-params = 4

[rules.no-barrel-files]
severity = "off"

[rules.no-barrel-chain]
severity = "off"

[rules.no-logic-in-barrel]
severity = "off"

[rules.no-magic-numbers]
severity = "off"

[rules.no-any]
severity = "off"

[rules.no-abbreviations]
severity = "off"

[rules.prefer-readonly]
severity = "off"

[rules.no-enums]
severity = "off"

[rules.no-interface]
severity = "off"

[rules.no-inline-types]
severity = "off"

[rules.no-logic-in-domain]
severity = "off"

[rules.max-directory-depth]
severity = "off"

[rules.no-duplicate-file-names]
severity = "off"

[rules.no-dump-files]
severity = "off"

[rules.no-empty-directories]
severity = "off"

[rules.no-process-env]
severity = "off"

[rules.no-restricted-imports]
severity = "off"
"#;

const REACT_PRESET: &str = r#"[project]
root = "src"
respect-gitignore = true

[project.structure.hooks]
folders = ["hooks"]
file-suffixes = [".hook.ts", ".hooks.ts"]

[project.structure.components]
folders = ["components"]
file-suffixes = [".component.tsx", ".components.tsx"]

[rules]
[rules.no-console]
severity = "warn"
allow-patterns = []

[rules.no-debugger]
severity = "error"

[rules.no-eval]
severity = "error"

[rules.no-empty-interface]
severity = "error"

[rules.hook-no-jsx]
severity = "error"

[rules.hook-prefix]
severity = "error"
prefixes = ["use"]

[rules.component-file-only-components]
severity = "warn"

[rules.no-default-export]
severity = "warn"
components-only = true

[rules.no-barrel-files]
severity = "warn"

[rules.no-logic-in-barrel]
severity = "warn"

[rules.boolean-prefix]
severity = "warn"
prefixes = ["is", "has", "can"]
ignore-constants = false

[rules.no-silent-catch]
severity = "warn"

[rules.no-non-null-assertion]
severity = "error"

[rules.explicit-return-type]
severity = "warn"

[rules.no-circular-import]
severity = "error"

[rules.no-large-file]
severity = "warn"
max-lines = 400

[rules.max-function-params]
severity = "warn"
max-params = 3

[rules.entry-file-no-logic]
severity = "warn"
entry-files = ["main", "app", "layout", "page"]

[rules.no-orphan-files]
severity = "warn"
entry-files = ["main", "app", "layout", "page"]
"#;

const LIBRARY_PRESET: &str = r#"[project]
root = "src"
respect-gitignore = true

[rules]
[rules.no-console]
severity = "off"

[rules.no-debugger]
severity = "error"

[rules.no-eval]
severity = "error"

[rules.no-empty-interface]
severity = "error"

[rules.no-export-star]
severity = "off"

[rules.no-focused-test]
severity = "off"

[rules.no-skipped-test]
severity = "off"

[rules.no-test-code-in-production]
severity = "off"

[rules.no-mutable-exports]
severity = "error"

[rules.no-namespace]
severity = "error"

[rules.no-silent-catch]
severity = "warn"

[rules.no-non-null-assertion]
severity = "warn"

[rules.no-type-assertion]
severity = "warn"

[rules.explicit-return-type]
severity = "error"

[rules.no-circular-import]
severity = "error"

[rules.no-large-file]
severity = "warn"
max-lines = 500

[rules.max-function-params]
severity = "warn"
max-params = 3

[rules.no-barrel-files]
severity = "off"

[rules.no-barrel-chain]
severity = "off"

[rules.no-logic-in-barrel]
severity = "off"

[rules.no-magic-numbers]
severity = "warn"
allowed-numbers = ["0", "1", "-1"]

[rules.no-any]
severity = "warn"
allowed-folders = []

[rules.no-abbreviations]
severity = "warn"

[rules.prefer-readonly]
severity = "warn"

[rules.no-interface]
severity = "warn"
allow-declaration-merging = true

[rules.no-restricted-imports]
severity = "warn"
restricted = []
"#;

const NO_BARRELS_PRESET: &str = r#"[project]
root = "src"
respect-gitignore = true

[rules]
[rules.no-console]
severity = "warn"
allow-patterns = []

[rules.no-debugger]
severity = "error"

[rules.no-eval]
severity = "error"

[rules.no-empty-interface]
severity = "error"

[rules.no-barrel-files]
severity = "error"

[rules.no-barrel-chain]
severity = "error"

[rules.no-logic-in-barrel]
severity = "error"

[rules.directory-must-have-barrel]
severity = "off"

[rules.no-export-star]
severity = "error"

[rules.no-circular-import]
severity = "error"
"#;
