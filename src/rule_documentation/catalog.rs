use crate::config::rule_metadata::RuleCategory;
use crate::rule_documentation::summary::RuleSummaryFn;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleDocumentation {
    pub(crate) name: &'static str,
    pub(crate) intent: &'static str,
    pub(crate) examples: &'static [RuleExample],
    pub(crate) options: &'static [RuleOption],
    pub(crate) category: RuleCategory,
    pub(crate) conflicts: &'static [&'static str],
    pub(crate) fixable: bool,
    pub(crate) summarize: RuleSummaryFn,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleExample {
    pub(crate) label: &'static str,
    pub(crate) code: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleOption {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
}

pub(crate) const SEVERITY_OPTION: RuleOption = RuleOption {
    name: "severity",
    description: "One of off, info, warn, or error.",
};

const NO_OPTIONS: &[RuleOption] = &[SEVERITY_OPTION];

const RULE_DOCUMENTATION: &[RuleDocumentation] = &[
    RuleDocumentation {
        name: "boolean-prefix",
        intent: "Boolean variables should be prefixed with is, has, or can to signal intent.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const open = true;",
            },
            RuleExample {
                label: "Prefer",
                code: "const isOpen = true;",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "prefixes",
                description: "Custom list of allowed boolean prefixes.",
            },
            RuleOption {
                name: "ignore-constants",
                description: "When true, skips checking const declarations.",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::boolean_prefix_summary,
    },
    RuleDocumentation {
        name: "component-file-only-components",
        intent: "Component files should export components only. Move utilities, types, and hooks to separate files.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export function formatDate() {} // in Button.component.tsx",
            },
            RuleExample {
                label: "Prefer",
                code: "export function Button() {} // only components exported",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.components",
                description: "Folders and file suffixes that identify component files.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::component_file_only_components_summary,
    },
    RuleDocumentation {
        name: "hook-no-jsx",
        intent: "Keep hook files focused on state and effects. JSX belongs in components.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export function useMenu() { return <button />; }",
            },
            RuleExample {
                label: "Prefer",
                code: "export function useMenu() { return { isOpen, toggle }; }",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.hooks",
                description: "Folders and file suffixes that identify hook files.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::hook_no_jsx_summary,
    },
    RuleDocumentation {
        name: "hook-prefix",
        intent: "Hook functions in hook files should start with 'use' to follow React conventions.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export function authenticate() { return true; }",
            },
            RuleExample {
                label: "Prefer",
                code: "export function useAuth() { return { user: null }; }",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "prefixes",
                description: "Custom list of allowed hook prefixes.",
            },
            RuleOption {
                name: "project.structure.hooks",
                description: "Folders and file suffixes that identify hook files.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::hook_prefix_summary,
    },
    RuleDocumentation {
        name: "layer-boundaries",
        intent: "Enforce that imports respect an ordered set of architectural layers. Each layer may only import layers at or below its own position in the defined order.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "// src/shared/date.ts\nimport { getSession } from '@/features/auth/session';",
            },
            RuleExample {
                label: "Prefer",
                code: "// Define layers in niteo.toml\n[architecture.layers]\norder = [\"app\", \"features\", \"entities\", \"shared\"]\n// shared cannot import features, entities, or app",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "architecture.layers.order",
                description: "Ordered list of layer names from highest (app) to lowest (shared). Layers can only import from their own level or lower.",
            },
            RuleOption {
                name: "architecture.layers.<name>.folders",
                description: "Folder paths that identify files belonging to this layer.",
            },
            RuleOption {
                name: "architecture.layers.<name>.file-suffixes",
                description: "File name suffixes that identify files belonging to this layer.",
            },
        ],
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::layer_boundaries_summary,
    },
    RuleDocumentation {
        name: "max-directory-depth",
        intent: "Limit nested directories so project structure remains easy to scan.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/features/billing/screens/settings/forms/card/EditCard.tsx",
            },
            RuleExample {
                label: "Prefer",
                code: "src/features/billing/EditCard.tsx",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-depth",
                description: "Maximum allowed path depth below the configured project root.",
            },
            RuleOption {
                name: "ignore-dirs",
                description: "Directory names to skip when checking depth.",
            },
        ],
        category: RuleCategory::FileDirectory,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::max_directory_depth_summary,
    },
    RuleDocumentation {
        name: "max-file-exports",
        intent: "Keep each file's public surface small enough to understand and refactor.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export const a = 1; export const b = 2; // ...many more",
            },
            RuleExample {
                label: "Prefer",
                code: "Split unrelated exports into focused modules.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-exports",
                description: "Maximum number of exports allowed in a file.",
            },
        ],
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::max_file_exports_summary,
    },
    RuleDocumentation {
        name: "max-function-params",
        intent: "Limit function parameter count. Functions with many parameters are hard to call correctly; prefer an object parameter.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "function createUser(name: string, age: number, email: string, role: string) {}",
            },
            RuleExample {
                label: "Prefer",
                code: "function createUser(options: { name: string; age: number; email: string; role: string }) {}",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-params",
                description: "Maximum number of parameters allowed. Default is 3.",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::max_function_params_summary,
    },
    RuleDocumentation {
        name: "max-items-per-directory",
        intent: "Prevent directories from becoming oversized collections of unrelated files.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/components/ with more items than max-items",
            },
            RuleExample {
                label: "Prefer",
                code: "Group related files into narrower child directories.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-items",
                description: "Maximum allowed number of source items in a directory.",
            },
            RuleOption {
                name: "ignore-dirs",
                description: "Directory names to skip.",
            },
            RuleOption {
                name: "count-folders",
                description: "Whether child folders count toward the item limit.",
            },
        ],
        category: RuleCategory::FileDirectory,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::max_items_per_directory_summary,
    },
    RuleDocumentation {
        name: "min-items-per-directory",
        intent: "Find tiny directories that add navigation cost without enough structure benefit.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/features/search/ with only one source file",
            },
            RuleExample {
                label: "Prefer",
                code: "Merge the file into a sibling until the folder has a real boundary.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "min-items",
                description: "Minimum expected number of source items in a directory.",
            },
            RuleOption {
                name: "ignore-dirs",
                description: "Directory names to skip.",
            },
            RuleOption {
                name: "count-folders",
                description: "Whether child folders count toward the item total.",
            },
        ],
        category: RuleCategory::FileDirectory,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::min_items_per_directory_summary,
    },
    RuleDocumentation {
        name: "no-barrel-chain",
        intent: "Prevent index.ts barrel files from chaining through other barrel files.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export { Button } from './components'; // resolves to ./components/index.ts",
            },
            RuleExample {
                label: "Prefer",
                code: "export { Button } from './components/Button';",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_barrel_chain_summary,
    },
    RuleDocumentation {
        name: "no-circular-import",
        intent: "Detect circular import chains between modules that can cause runtime initialization issues.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "// a.ts\nimport { b } from './b';\n// b.ts\nimport { a } from './a';",
            },
            RuleExample {
                label: "Prefer",
                code: "Break the cycle by extracting shared logic to a third module.",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_circular_import_summary,
    },
    RuleDocumentation {
        name: "no-barrel-files",
        intent: "Avoid index.ts barrel files that hide import origins and public API shape.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export { Button } from './Button';",
            },
            RuleExample {
                label: "Prefer",
                code: "import { Button } from './Button';",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::ExportModuleShape,
        conflicts: &["directory-must-have-barrel"],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_barrel_files_summary,
    },
    RuleDocumentation {
        name: "no-comments",
        intent: "Discourage implementation comments that duplicate code instead of improving names.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "// increment count\ncount += 1;",
            },
            RuleExample {
                label: "Prefer",
                code: "Use clearer names or extract a function.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "allow-doc-comments",
                description: "Whether documentation comments are allowed.",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_comments_summary,
    },
    RuleDocumentation {
        name: "no-console",
        intent: "Keep debugging output out of application code except in explicitly allowed files.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "console.log(user);",
            },
            RuleExample {
                label: "Prefer",
                code: "Use the project's logger or remove the debug statement.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "allow-patterns",
                description: "Path substrings that may contain console statements.",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_console_summary,
    },
    RuleDocumentation {
        name: "no-side-effect-imports",
        intent: "Disallow bare side-effect imports such as `import \"./styles.css\"`; prefer importing named bindings or types.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "import \"./styles.css\";",
            },
            RuleExample {
                label: "Prefer",
                code: "import { styles } from \"./styles\";",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_side_effect_imports_summary,
    },
    RuleDocumentation {
        name: "sort-imports",
        intent: "Enforce consistent import ordering by module specifier. Groups separated by blank lines are sorted independently.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "import c from \"c\";\nimport a from \"a\";\nimport b from \"b\";",
            },
            RuleExample {
                label: "Prefer",
                code: "import a from \"a\";\nimport b from \"b\";\nimport c from \"c\";",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::sort_imports_summary,
    },
    RuleDocumentation {
        name: "no-debugger",
        intent: "Prevent committed debugger statements from stopping runtime execution.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "debugger;",
            },
            RuleExample {
                label: "Prefer",
                code: "Remove the statement before committing.",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::no_debugger_summary,
    },
    RuleDocumentation {
        name: "no-default-export",
        intent: "Prefer named exports so imports stay explicit and refactors are safer. When components-only is set, only component files are checked.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export default function Button() {}",
            },
            RuleExample {
                label: "Prefer",
                code: "export function Button() {}",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "components-only",
                description: "When true, only enforce named exports in component files.",
            },
            RuleOption {
                name: "project.structure.components",
                description: "Folders and file suffixes that identify component files (used when components-only is true).",
            },
        ],
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_default_export_summary,
    },
    RuleDocumentation {
        name: "no-duplicate-file-names",
        intent: "Avoid repeated file names that make stack traces and editor tabs ambiguous.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "features/user/Form.tsx and features/team/Form.tsx",
            },
            RuleExample {
                label: "Prefer",
                code: "UserForm.tsx and TeamForm.tsx",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "ignore-names",
                description: "File names allowed to repeat.",
            },
        ],
        category: RuleCategory::FileDirectory,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_duplicate_file_names_summary,
    },
    RuleDocumentation {
        name: "no-dump-files",
        intent: "Disallow generic file names like utils.ts, helpers.ts, and types.ts that become dumping grounds for unrelated code.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "utils.ts, helpers.ts, types.ts",
            },
            RuleExample {
                label: "Prefer",
                code: "authUtils.ts, dateHelpers.ts, userTypes.ts",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "extra-names",
                description: "Additional file stems to forbid (beyond utils, helpers, types).",
            },
        ],
        category: RuleCategory::FileDirectory,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_dump_files_summary,
    },
    RuleDocumentation {
        name: "no-empty-directories",
        intent: "Remove directories that no longer contain source files or useful modules.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/legacy/ with no source files",
            },
            RuleExample {
                label: "Prefer",
                code: "Delete the directory or move useful files into it.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "ignore-dirs",
                description: "Directory names to skip.",
            },
        ],
        category: RuleCategory::FileDirectory,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_empty_directories_summary,
    },
    RuleDocumentation {
        name: "directory-must-have-barrel",
        intent: "Non-leaf directories should provide a single import surface via an index.ts barrel.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/features/billing/components/Card.tsx with no src/features/billing/index.ts",
            },
            RuleExample {
                label: "Prefer",
                code: "src/features/billing/index.ts",
            },
        ],
        options: &[SEVERITY_OPTION],
        category: RuleCategory::FileDirectory,
        conflicts: &["no-barrel-files"],
        fixable: false,
        summarize: crate::rule_documentation::summary::directory_must_have_barrel_summary,
    },
    RuleDocumentation {
        name: "no-empty-interface",
        intent: "Avoid empty interfaces that add names without structure.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "interface Props {}",
            },
            RuleExample {
                label: "Prefer",
                code: "type Props = Record<string, never>;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::no_empty_interface_summary,
    },
    RuleDocumentation {
        name: "no-enums",
        intent: "Prefer union types or const objects over TypeScript enums.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "enum Status { Open = 'open' }",
            },
            RuleExample {
                label: "Prefer",
                code: "const STATUS = { Open: 'open' } as const;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_enums_summary,
    },
    RuleDocumentation {
        name: "no-eval",
        intent: "Block dynamic code execution through eval() and new Function().",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "eval(source);",
            },
            RuleExample {
                label: "Prefer",
                code: "Use a typed parser or explicit dispatch table.",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_eval_summary,
    },
    RuleDocumentation {
        name: "no-export-star",
        intent: "Make re-exported APIs explicit instead of hiding them behind export *.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export * from './Button';",
            },
            RuleExample {
                label: "Prefer",
                code: "export { Button } from './Button';",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_export_star_summary,
    },
    RuleDocumentation {
        name: "no-focused-test",
        intent: "Disallow focused test helpers (describe.only, it.only, test.only) that skip the rest of the suite.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "describe.only('auth', () => { it.only('logs in', () => {}); });",
            },
            RuleExample {
                label: "Prefer",
                code: "describe('auth', () => { it('logs in', () => {}); });",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::no_focused_test_summary,
    },
    RuleDocumentation {
        name: "no-inline-types",
        intent: "Keep exported contracts in colocated type files or accepted type folders.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export type User = { id: string }; // in Component.tsx",
            },
            RuleExample {
                label: "Prefer",
                code: "Move exported contracts to Component.type.ts or a types folder.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.types",
                description: "Folders and file suffixes that identify type files. Declaration files (.d.ts) are always allowed.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_inline_types_summary,
    },
    RuleDocumentation {
        name: "no-interface",
        intent: "Prefer type aliases unless interface declaration merging is intentional.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "interface User { id: string }",
            },
            RuleExample {
                label: "Prefer",
                code: "type User = { id: string };",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "allow-declaration-merging",
                description: "Whether repeated interface declarations are allowed.",
            },
        ],
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_interface_summary,
    },
    RuleDocumentation {
        name: "no-large-file",
        intent: "Keep files small enough to review and change without broad context.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "A file with more lines than max-lines.",
            },
            RuleExample {
                label: "Prefer",
                code: "Split by responsibility into focused modules.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-lines",
                description: "Maximum number of lines allowed in one file.",
            },
        ],
        category: RuleCategory::FileDirectory,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_large_file_summary,
    },
    RuleDocumentation {
        name: "no-logic-in-barrel",
        intent: "Keep index.ts barrel files limited to import/export forwarding.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const value = compute(); export { value };",
            },
            RuleExample {
                label: "Prefer",
                code: "export { value } from './value';",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_logic_in_barrel_summary,
    },
    RuleDocumentation {
        name: "no-logic-in-domain",
        intent: "Keep domain type and constant files free of runtime implementation logic.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export function buildUser() {} // in user.type.ts",
            },
            RuleExample {
                label: "Prefer",
                code: "Move implementation to a feature, service, or model module.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.types",
                description: "Folders and file suffixes that identify type domain files.",
            },
            RuleOption {
                name: "project.structure.constants",
                description: "Folders and file suffixes that identify constants domain files.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_logic_in_domain_summary,
    },
    RuleDocumentation {
        name: "no-abbreviations",
        intent: "Disallow abbreviated identifiers like btn, ctx, and mgr. Expanded names improve readability and self-document the code.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const btn = document.querySelector('button');",
            },
            RuleExample {
                label: "Prefer",
                code: "const button = document.querySelector('button');",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "extra-abbreviations",
                description: "Additional abbreviations to flag beyond the defaults (btn, ctx, mgr).",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_abbreviations_summary,
    },
    RuleDocumentation {
        name: "no-restricted-imports",
        intent: "Block imports from a configurable deny-list of packages or paths.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "import { merge } from 'lodash'; // when lodash is restricted",
            },
            RuleExample {
                label: "Prefer",
                code: "import merge from './utils/merge';",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "restricted",
                description: "List of package names or path prefixes to block.",
            },
        ],
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_restricted_imports_summary,
    },
    RuleDocumentation {
        name: "no-mutable-exports",
        intent: "Avoid exported mutable bindings that make module state unpredictable.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export let currentUser = null;",
            },
            RuleExample {
                label: "Prefer",
                code: "export const getCurrentUser = () => currentUser;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_mutable_exports_summary,
    },
    RuleDocumentation {
        name: "sort-exports",
        intent: "Enforce consistent export ordering by exported name. Default exports sort first. Groups separated by blank lines are sorted independently.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export const c = 1;\nexport const a = 2;\nexport const b = 3;",
            },
            RuleExample {
                label: "Prefer",
                code: "export const a = 2;\nexport const b = 3;\nexport const c = 1;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::sort_exports_summary,
    },
    RuleDocumentation {
        name: "no-nested-functions",
        intent: "Disallow functions defined inside other functions beyond a configured nesting depth.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "function outer() { function middle() { function inner() {} } }",
            },
            RuleExample {
                label: "Prefer",
                code: "function inner() {} function middle() { inner(); } function outer() { middle(); }",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-depth",
                description: "Maximum allowed function nesting depth. Default is 2.",
            },
            RuleOption {
                name: "contexts",
                description: "Which function-like constructs count as nesting levels. Each context represents a construct type: 'function', 'arrow', 'class-method', 'object-method'. Exclude a context to allow its construct without contributing to nesting depth. Default is all four.",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_nested_functions_summary,
    },
    RuleDocumentation {
        name: "no-orphan-files",
        intent: "Detect files not imported by any other file in the project, which may indicate dead code.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "// orphan.ts - not imported anywhere",
            },
            RuleExample {
                label: "Prefer",
                code: "Import the file from another module or mark it as an entry file.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "entry-files",
                description: "File stems that are expected entry points and not imported by other files. Defaults to main, app, layout, page.",
            },
        ],
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_orphan_files_summary,
    },
    RuleDocumentation {
        name: "no-package-cycle",
        intent: "Detect circular dependencies between workspace packages that can cause initialization deadlocks.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "// packages/a imports packages/b imports packages/c imports packages/a",
            },
            RuleExample {
                label: "Prefer",
                code: "Break the cycle by extracting shared logic to a new package or merging the packages.",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_package_cycle_summary,
    },
    RuleDocumentation {
        name: "no-private-package-import",
        intent: "Prevent importing internal files from other packages. Only public exports should be consumed across packages.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "import { helper } from '@scope/admin/src/internal/utils';",
            },
            RuleExample {
                label: "Prefer",
                code: "import { helper } from '@scope/admin'; // use public exports only",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_private_package_import_summary,
    },
    RuleDocumentation {
        name: "no-namespace",
        intent: "Prefer ES modules over TypeScript namespace declarations.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "namespace Utils { export function go() {} }",
            },
            RuleExample {
                label: "Prefer",
                code: "import { go } from './Utils';",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_namespace_summary,
    },
    RuleDocumentation {
        name: "no-silent-catch",
        intent: "Require catch blocks to log the error, rethrow it, or return a typed fallback.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "try { doWork(); } catch (e) {}",
            },
            RuleExample {
                label: "Prefer",
                code: "try { doWork(); } catch (e) { console.error(e); return null; }",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_silent_catch_summary,
    },
    RuleDocumentation {
        name: "no-skipped-test",
        intent: "Disallow skipped test helpers (describe.skip, it.skip, test.skip) that silently bypass the test suite.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "describe.skip('auth', () => { it.skip('logs in', () => {}); });",
            },
            RuleExample {
                label: "Prefer",
                code: "describe('auth', () => { it('logs in', () => {}); });",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::no_skipped_test_summary,
    },
    RuleDocumentation {
        name: "no-test-code-in-production",
        intent: "Disallow test code (describe, it, test, expect, and test library imports) outside test files.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "describe('auth', () => { it('works', () => { expect(true).toBe(true); }); }); // in src/auth.ts",
            },
            RuleExample {
                label: "Prefer",
                code: "Move test code to a test file (e.g. tests/auth.test.ts).",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.tests",
                description: "Folders and file suffixes that identify test files.",
            },
        ],
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_test_code_in_production_summary,
    },
    RuleDocumentation {
        name: "no-then-chain",
        intent: "Prefer async/await over .then() chains for better readability and error handling.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "fetch('/api').then(res => res.json()).then(data => process(data));",
            },
            RuleExample {
                label: "Prefer",
                code: "const res = await fetch('/api');",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "allow-single",
                description: "When true, a single .then() call without chaining is not flagged (default: true).",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_then_chain_summary,
    },
    RuleDocumentation {
        name: "no-upward-import",
        intent: "Avoid fragile ../../ imports in favor of local or project-root imports.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "import { Button } from '../../components/Button';",
            },
            RuleExample {
                label: "Prefer",
                code: "import { Button } from '@/components/Button';",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-depth",
                description: "Number of upward ../ segments allowed before reporting.",
            },
        ],
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_upward_import_summary,
    },
    RuleDocumentation {
        name: "prefer-satisfies",
        intent: "Prefer satisfies over as when validating a value against a type.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const config = value as Config;",
            },
            RuleExample {
                label: "Prefer",
                code: "const config = value satisfies Config;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::prefer_satisfies_summary,
    },
    RuleDocumentation {
        name: "prefer-readonly",
        intent: "Prefer readonly for array parameters in exported functions to prevent accidental mutation of caller data.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export function process(items: string[]) {}",
            },
            RuleExample {
                label: "Prefer",
                code: "export function process(items: readonly string[]) {}",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::ExportModuleShape,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::prefer_readonly_summary,
    },
    RuleDocumentation {
        name: "no-test-import",
        intent: "Production code may not import test files.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "import { helper } from './helper.test'; // in src/auth.ts",
            },
            RuleExample {
                label: "Prefer",
                code: "Move shared helpers out of test files into production modules.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.tests",
                description: "Folders and file suffixes that identify test files.",
            },
        ],
        category: RuleCategory::Import,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_test_import_summary,
    },
    RuleDocumentation {
        name: "entry-file-no-logic",
        intent: "Entry files like main.ts, app.tsx, layout.tsx, and page.tsx should delegate logic to dedicated modules instead of containing implementation details.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "function bootstrap() { /* complex setup logic */ } // in main.ts",
            },
            RuleExample {
                label: "Prefer",
                code: "import { bootstrap } from './bootstrap'; bootstrap(); // delegate to module",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "entry-files",
                description: "File stems to treat as entry files. Defaults to main, app, layout, page.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::entry_file_no_logic_summary,
    },
    RuleDocumentation {
        name: "explicit-return-type",
        intent: "Require explicit return type annotations on exported functions to make public API contracts clear.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export function add(a: number, b: number) { return a + b; }",
            },
            RuleExample {
                label: "Prefer",
                code: "export function add(a: number, b: number): number { return a + b; }",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::explicit_return_type_summary,
    },
    RuleDocumentation {
        name: "no-non-null-assertion",
        intent: "Disallow the non-null assertion operator (!) which bypasses TypeScript's null safety checks.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const value = obj!.prop;",
            },
            RuleExample {
                label: "Prefer",
                code: "const value = obj?.prop ?? 'default';",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::no_non_null_assertion_summary,
    },
    RuleDocumentation {
        name: "no-await-in-loop",
        intent: "Disallow await inside loop bodies. Extract to a separate async function or use Promise.all.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "for (const item of items) { await process(item); }",
            },
            RuleExample {
                label: "Prefer",
                code: "await Promise.all(items.map(item => process(item)));",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_await_in_loop_summary,
    },
    RuleDocumentation {
        name: "no-promise-executor-return",
        intent: "Disallow returning a value from a Promise executor. Values are discarded; use resolve() instead.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "new Promise((resolve, reject) => { return 42; });",
            },
            RuleExample {
                label: "Prefer",
                code: "new Promise((resolve, reject) => { resolve(42); });",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_promise_executor_return_summary,
    },
    RuleDocumentation {
        name: "no-unsafe-optional-chaining",
        intent: "Disallow optional chaining (`?.`) on expressions that are never null or undefined, such as literals, constructors, and assertions.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const result = 'hello'?.length;",
            },
            RuleExample {
                label: "Reports",
                code: "const instance = new Foo()?.bar;",
            },
            RuleExample {
                label: "Prefer",
                code: "const result = 'hello'.length;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_unsafe_optional_chaining_summary,
    },
    RuleDocumentation {
        name: "no-magic-numbers",
        intent: "Disallow numeric and string literals outside constants. Extract literals to named constants for clarity and maintainability.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "setTimeout(callback, 3000);",
            },
            RuleExample {
                label: "Prefer",
                code: "const TIMEOUT_MS = 3000;\nsetTimeout(callback, TIMEOUT_MS);",
            },
            RuleExample {
                label: "With enforce-strings",
                code: "fetch(\"/api/users\");",
            },
            RuleExample {
                label: "With enforce-strings, prefer",
                code: "const API_URL = \"/api/users\";\nfetch(API_URL);",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "allowed-numbers",
                description: "List of numeric literals to allow (e.g., [\"0\", \"1\", \"-1\"]).",
            },
            RuleOption {
                name: "enforce-strings",
                description: "When true, also flag inline string literals that should be named constants (default false).",
            },
        ],
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_magic_numbers_summary,
    },
    RuleDocumentation {
        name: "no-process-env",
        intent: "Prevent direct access to process.env. Use a centralized config module instead.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const key = process.env.API_KEY;",
            },
            RuleExample {
                label: "Prefer",
                code: "const key = config.apiKey;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::SourceHygiene,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::no_process_env_summary,
    },
    RuleDocumentation {
        name: "no-any",
        intent: "Disallow explicit `any` type annotations. Files in generated folders or configured allowed folders are exempt.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const value: any = getData();",
            },
            RuleExample {
                label: "Prefer",
                code: "const value: unknown = getData();",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "allowed-folders",
                description: "Folder names where `any` is permitted (e.g. legacy code).",
            },
            RuleOption {
                name: "project.structure.generated",
                description: "Folders and file suffixes that identify generated files (always exempt).",
            },
        ],
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: true,
        summarize: crate::rule_documentation::summary::no_any_summary,
    },
    RuleDocumentation {
        name: "no-type-assertion",
        intent: "Disallow `as` casts. Prefer type narrowing or `satisfies` for safer type checking.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const value = something as string;",
            },
            RuleExample {
                label: "Prefer",
                code: "const value = something satisfies string;",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_type_assertion_summary,
    },
    RuleDocumentation {
        name: "no-unnecessary-type-assertion",
        intent: "Disallow `as` casts where the expression already has the asserted type.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "const value = \"hello\" as string;",
            },
            RuleExample {
                label: "Prefer",
                code: "const value = \"hello\";",
            },
        ],
        options: NO_OPTIONS,
        category: RuleCategory::LanguageTypescript,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_unnecessary_type_assertion_summary,
    },
    RuleDocumentation {
        name: "no-empty-domain",
        intent: "Domain folders must contain real source files, not only barrel files that re-export.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/auth/ containing only index.ts with re-exports",
            },
            RuleExample {
                label: "Prefer",
                code: "Add implementation files to the domain or remove the folder.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "ignore-dirs",
                description: "Directory names to skip.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_empty_domain_summary,
    },
    RuleDocumentation {
        name: "no-anemic-domain",
        intent: "Domain folders with too few files add navigation cost without meaningful structure.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/features/search/ with only one source file",
            },
            RuleExample {
                label: "Prefer",
                code: "Flatten the file into a parent or sibling directory.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-files",
                description: "Maximum number of files below the threshold. Default is 1.",
            },
            RuleOption {
                name: "ignore-dirs",
                description: "Directory names to skip.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_anemic_domain_summary,
    },
    RuleDocumentation {
        name: "no-god-domain",
        intent: "Domain folders with too many files should be split into sub-domains for clarity.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "src/components/ with more files than max-files",
            },
            RuleExample {
                label: "Prefer",
                code: "Group related files into narrower child directories.",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "max-files",
                description: "Maximum number of files allowed. Default is 20.",
            },
            RuleOption {
                name: "ignore-dirs",
                description: "Directory names to skip.",
            },
        ],
        category: RuleCategory::Domain,
        conflicts: &[],
        fixable: false,
        summarize: crate::rule_documentation::summary::no_god_domain_summary,
    },
];

pub(crate) fn all_rules() -> &'static [RuleDocumentation] {
    RULE_DOCUMENTATION
}

pub(crate) fn find_rule(name: &str) -> Option<&'static RuleDocumentation> {
    RULE_DOCUMENTATION
        .iter()
        .find(|documentation| documentation.name == name)
}

pub(crate) fn available_rule_names() -> String {
    RULE_DOCUMENTATION
        .iter()
        .map(|documentation| documentation.name)
        .collect::<Vec<_>>()
        .join(", ")
}
