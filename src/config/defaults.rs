pub const CONFIG_FILE_NAME: &str = "niteo.toml";

pub const DEFAULT_BASELINE_FILE: &str = "niteo-baseline.json";

pub const DEFAULT_CONFIG_SOURCE: &str = r#"[project]
root = "src"
respect-gitignore = true
history = true
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
folders = ["tests"]
file-suffixes = [".test.ts", ".tests.ts"]

[project.structure.generated]
folders = ["generated", "__generated__"]
file-suffixes = [".generated.ts", ".generated.tsx"]

[rules]
[rules.component-file-only-components]
severity = "warn"

[rules.boolean-prefix]
severity = "warn"
prefixes = ["is", "has", "can"]
ignore-constants = false

[rules.no-comments]
severity = "warn"
allow-doc-comments = true

[rules.no-logic-in-barrel]
severity = "warn"

[rules.no-default-export]
severity = "warn"
components-only = false

[rules.no-export-star]
severity = "warn"

[rules.no-focused-test]
severity = "warn"

[rules.no-inline-types]
severity = "warn"

[rules.max-file-exports]
severity = "warn"
max-exports = 10

[rules.max-function-params]
severity = "warn"
max-params = 3

[rules.no-upward-import]
severity = "warn"
max-depth = 0

[rules.no-large-file]
severity = "warn"
max-lines = 500

[rules.no-enums]
severity = "warn"

[rules.no-barrel-files]
severity = "warn"

[rules.no-barrel-chain]
severity = "warn"

[rules.no-circular-import]
severity = "warn"

[rules.no-console]
severity = "warn"
allow-patterns = []

[rules.no-debugger]
severity = "warn"

[rules.no-eval]
severity = "warn"

[rules.no-logic-in-domain]
severity = "warn"

[rules.no-empty-directories]
severity = "warn"
ignore-dirs = []

[rules.directory-must-have-barrel]
severity = "warn"

[rules.no-duplicate-file-names]
severity = "warn"
ignore-names = []

[rules.max-items-per-directory]
severity = "warn"
max-items = 20
ignore-dirs = []
count-folders = false

[rules.min-items-per-directory]
severity = "warn"
min-items = 3
ignore-dirs = []
count-folders = false

[rules.max-directory-depth]
severity = "warn"
max-depth = 5
ignore-dirs = []

[rules.no-empty-interface]
severity = "error"

[rules.no-interface]
severity = "warn"
allow-declaration-merging = true

[rules.no-magic-numbers]
severity = "warn"
allowed-numbers = []
enforce-strings = false

[rules.no-mutable-exports]
severity = "warn"

[rules.no-namespace]
severity = "warn"

[rules.no-silent-catch]
severity = "warn"

[rules.no-skipped-test]
severity = "warn"

[rules.no-test-code-in-production]
severity = "warn"

[rules.prefer-satisfies]
severity = "info"

[rules.prefer-readonly]
severity = "warn"

[rules.hook-no-jsx]
severity = "warn"

[rules.hook-prefix]
severity = "warn"
prefixes = ["use"]

[rules.no-dump-files]
severity = "warn"
extra-names = []

[rules.no-test-import]
severity = "warn"

[rules.entry-file-no-logic]
severity = "warn"
entry-files = ["main", "app", "layout", "page"]

[rules.explicit-return-type]
severity = "warn"

[rules.no-non-null-assertion]
severity = "warn"

[rules.no-type-assertion]
severity = "warn"

[rules.no-process-env]
severity = "warn"

[rules.no-abbreviations]
severity = "warn"
extra-abbreviations = []

[rules.no-any]
severity = "warn"
allowed-folders = []

[rules.no-restricted-imports]
severity = "warn"
restricted = []

[rules.no-side-effect-imports]
severity = "warn"

[rules.no-nested-functions]
severity = "warn"
max-depth = 2

[rules.no-orphan-files]
severity = "warn"
entry-files = ["main", "app", "layout", "page"]

[rules.no-package-cycle]
severity = "warn"

[rules.no-private-package-import]
severity = "warn"

[rules.no-empty-domain]
severity = "warn"
ignore-dirs = []

[rules.no-anemic-domain]
severity = "warn"
max-files = 1
ignore-dirs = []

[rules.no-god-domain]
severity = "warn"
max-files = 20
ignore-dirs = []

[rules.layer-boundaries]
severity = "off"

[fix]
no-debugger = true
no-focused-test = true
no-empty-interface = true
no-skipped-test = true
"#;
