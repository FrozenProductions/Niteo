use anyhow::{Result, bail};
use serde::Serialize;

use crate::config::{ProjectConfig, Severity};

pub struct ConfiguredRule {
    pub name: &'static str,
    pub severity: Severity,
}

pub struct RuleExplanation {
    pub name: &'static str,
    pub severity: Severity,
    pub intent: &'static str,
    pub examples: Vec<RuleExplanationExample>,
    pub options: Vec<RuleExplanationOption>,
    pub current_severity: Severity,
    pub current_options: Vec<String>,
}

pub struct RuleExplanationExample {
    pub label: &'static str,
    pub code: &'static str,
}

pub struct RuleExplanationOption {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct RuleDocumentation {
    name: &'static str,
    intent: &'static str,
    examples: &'static [RuleExample],
    options: &'static [RuleOption],
    kind: RuleKind,
}

#[derive(Debug, Clone, Copy)]
struct RuleExample {
    label: &'static str,
    code: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct RuleOption {
    name: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Copy)]
enum RuleKind {
    BooleanPrefix,
    HookNoJsx,
    HookPrefix,
    MaxDirectoryDepth,
    MaxFileExports,
    MaxItemsPerDirectory,
    MinItemsPerDirectory,
    NoBarrelChain,
    NoBarrelFiles,
    NoComments,
    NoConsole,
    NoDebugger,
    NoDefaultExport,
    NoDuplicateFileNames,
    NoDumpFiles,
    NoEmptyDirectories,
    NoEmptyInterface,
    NoEnums,
    NoEval,
    NoExportStar,
    NoInlineTypes,
    NoInterface,
    NoLargeFile,
    NoLogicInBarrel,
    NoLogicInDomain,
    NoMutableExports,
    NoNamespace,
    NoSilentCatch,
    NoThenChain,
    NoUpwardImport,
    PreferSatisfies,
}

#[derive(Debug, Clone)]
struct RuleConfigSummary {
    severity: Severity,
    options: Vec<String>,
}

const SEVERITY_OPTION: RuleOption = RuleOption {
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
        kind: RuleKind::BooleanPrefix,
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
        options: NO_OPTIONS,
        kind: RuleKind::HookNoJsx,
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
        ],
        kind: RuleKind::HookPrefix,
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
        kind: RuleKind::MaxDirectoryDepth,
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
        kind: RuleKind::MaxFileExports,
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
        kind: RuleKind::MaxItemsPerDirectory,
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
        kind: RuleKind::MinItemsPerDirectory,
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
        kind: RuleKind::NoBarrelChain,
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
        kind: RuleKind::NoBarrelFiles,
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
        kind: RuleKind::NoComments,
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
        kind: RuleKind::NoConsole,
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
        kind: RuleKind::NoDebugger,
    },
    RuleDocumentation {
        name: "no-default-export",
        intent: "Prefer named exports so imports stay explicit and refactors are safer.",
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
        options: NO_OPTIONS,
        kind: RuleKind::NoDefaultExport,
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
        kind: RuleKind::NoDuplicateFileNames,
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
        kind: RuleKind::NoDumpFiles,
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
        kind: RuleKind::NoEmptyDirectories,
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
        kind: RuleKind::NoEmptyInterface,
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
        kind: RuleKind::NoEnums,
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
        kind: RuleKind::NoEval,
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
        kind: RuleKind::NoExportStar,
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
        options: NO_OPTIONS,
        kind: RuleKind::NoInlineTypes,
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
        kind: RuleKind::NoInterface,
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
        kind: RuleKind::NoLargeFile,
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
        kind: RuleKind::NoLogicInBarrel,
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
                name: "extra-folders",
                description: "Additional folder names treated as domain-only folders.",
            },
            RuleOption {
                name: "extra-file-suffixes",
                description: "Additional file suffixes treated as domain-only files.",
            },
        ],
        kind: RuleKind::NoLogicInDomain,
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
        kind: RuleKind::NoMutableExports,
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
        kind: RuleKind::NoNamespace,
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
        kind: RuleKind::NoSilentCatch,
    },
    RuleDocumentation {
        name: "no-then-chain",
        intent: "Prefer async/await over .then() chains for better readability and error handling.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "fetch('/api').then(res => res.json());",
            },
            RuleExample {
                label: "Prefer",
                code: "const res = await fetch('/api');",
            },
        ],
        options: NO_OPTIONS,
        kind: RuleKind::NoThenChain,
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
        kind: RuleKind::NoUpwardImport,
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
        kind: RuleKind::PreferSatisfies,
    },
];

pub fn configured_rules(config: &ProjectConfig) -> Vec<ConfiguredRule> {
    RULE_DOCUMENTATION
        .iter()
        .map(|documentation| ConfiguredRule {
            name: documentation.name,
            severity: config_summary(config, documentation.kind).severity,
        })
        .collect()
}

pub fn explain_rule(rule_name: &str, config: &ProjectConfig) -> Result<RuleExplanation> {
    let Some(documentation) = RULE_DOCUMENTATION
        .iter()
        .find(|documentation| documentation.name == rule_name)
    else {
        let names = RULE_DOCUMENTATION
            .iter()
            .map(|documentation| documentation.name)
            .collect::<Vec<_>>()
            .join(", ");
        bail!("unknown rule '{rule_name}'. Available rules: {names}");
    };

    let summary = config_summary(config, documentation.kind);

    Ok(RuleExplanation {
        name: documentation.name,
        severity: summary.severity,
        intent: documentation.intent,
        examples: documentation
            .examples
            .iter()
            .map(|e| RuleExplanationExample {
                label: e.label,
                code: e.code,
            })
            .collect(),
        options: documentation
            .options
            .iter()
            .map(|o| RuleExplanationOption {
                name: o.name,
                description: o.description,
            })
            .collect(),
        current_severity: summary.severity,
        current_options: summary.options,
    })
}

pub fn render_explanation_text(explanation: &RuleExplanation) -> String {
    let mut output = String::new();
    output.push_str(explanation.name);
    output.push('\n');
    output.push_str(&format!("severity: {}\n\n", explanation.severity.as_str()));
    output.push_str("Intent\n");
    output.push_str(explanation.intent);
    output.push_str("\n\nExamples\n");
    for example in &explanation.examples {
        output.push_str(&format!("- {}: {}\n", example.label, example.code));
    }
    output.push_str("\nConfig options\n");
    for option in &explanation.options {
        output.push_str(&format!("- {}: {}\n", option.name, option.description));
    }
    output.push_str("\nCurrent config\n");
    output.push_str(&format!(
        "- severity: {}\n",
        explanation.current_severity.as_str()
    ));
    for option in &explanation.current_options {
        output.push_str(&format!("- {option}\n"));
    }
    output
}

fn config_summary(config: &ProjectConfig, kind: RuleKind) -> RuleConfigSummary {
    match kind {
        RuleKind::BooleanPrefix => RuleConfigSummary {
            severity: config.rules.boolean_prefix.severity,
            options: vec![
                format!("prefixes: {:?}", config.rules.boolean_prefix.prefixes),
                format!(
                    "ignore-constants: {}",
                    config.rules.boolean_prefix.ignore_constants
                ),
            ],
        },
        RuleKind::HookNoJsx => simple_summary(config.rules.hook_no_jsx.severity),
        RuleKind::HookPrefix => RuleConfigSummary {
            severity: config.rules.hook_prefix.severity,
            options: vec![format!("prefixes: {:?}", config.rules.hook_prefix.prefixes)],
        },
        RuleKind::MaxDirectoryDepth => RuleConfigSummary {
            severity: config.rules.max_directory_depth.severity,
            options: vec![
                format!("max-depth: {}", config.rules.max_directory_depth.max_depth),
                format!(
                    "ignore-dirs: {:?}",
                    config.rules.max_directory_depth.ignore_dirs
                ),
            ],
        },
        RuleKind::MaxFileExports => RuleConfigSummary {
            severity: config.rules.max_file_exports.severity,
            options: vec![format!(
                "max-exports: {}",
                config.rules.max_file_exports.max_exports
            )],
        },
        RuleKind::MaxItemsPerDirectory => RuleConfigSummary {
            severity: config.rules.max_items_per_directory.severity,
            options: vec![
                format!(
                    "max-items: {}",
                    config.rules.max_items_per_directory.max_items
                ),
                format!(
                    "ignore-dirs: {:?}",
                    config.rules.max_items_per_directory.ignore_dirs
                ),
                format!(
                    "count-folders: {}",
                    config.rules.max_items_per_directory.count_folders
                ),
            ],
        },
        RuleKind::MinItemsPerDirectory => RuleConfigSummary {
            severity: config.rules.min_items_per_directory.severity,
            options: vec![
                format!(
                    "min-items: {}",
                    config.rules.min_items_per_directory.min_items
                ),
                format!(
                    "ignore-dirs: {:?}",
                    config.rules.min_items_per_directory.ignore_dirs
                ),
                format!(
                    "count-folders: {}",
                    config.rules.min_items_per_directory.count_folders
                ),
            ],
        },
        RuleKind::NoBarrelChain => simple_summary(config.rules.no_barrel_chain.severity),
        RuleKind::NoBarrelFiles => simple_summary(config.rules.no_barrel_files.severity),
        RuleKind::NoComments => RuleConfigSummary {
            severity: config.rules.no_comments.severity,
            options: vec![format!(
                "allow-doc-comments: {}",
                config.rules.no_comments.allow_doc_comments
            )],
        },
        RuleKind::NoConsole => RuleConfigSummary {
            severity: config.rules.no_console.severity,
            options: vec![format!(
                "allow-patterns: {:?}",
                config.rules.no_console.allow_patterns
            )],
        },
        RuleKind::NoDebugger => simple_summary(config.rules.no_debugger.severity),
        RuleKind::NoDefaultExport => simple_summary(config.rules.no_default_export.severity),
        RuleKind::NoDuplicateFileNames => RuleConfigSummary {
            severity: config.rules.no_duplicate_file_names.severity,
            options: vec![format!(
                "ignore-names: {:?}",
                config.rules.no_duplicate_file_names.ignore_names
            )],
        },
        RuleKind::NoDumpFiles => RuleConfigSummary {
            severity: config.rules.no_dump_files.severity,
            options: vec![format!(
                "extra-names: {:?}",
                config.rules.no_dump_files.extra_names
            )],
        },
        RuleKind::NoEmptyDirectories => RuleConfigSummary {
            severity: config.rules.no_empty_directories.severity,
            options: vec![format!(
                "ignore-dirs: {:?}",
                config.rules.no_empty_directories.ignore_dirs
            )],
        },
        RuleKind::NoEmptyInterface => simple_summary(config.rules.no_empty_interface.severity),
        RuleKind::NoEnums => simple_summary(config.rules.no_enums.severity),
        RuleKind::NoEval => simple_summary(config.rules.no_eval.severity),
        RuleKind::NoExportStar => simple_summary(config.rules.no_export_star.severity),
        RuleKind::NoInlineTypes => simple_summary(config.rules.no_inline_types.severity),
        RuleKind::NoInterface => RuleConfigSummary {
            severity: config.rules.no_interface.severity,
            options: vec![format!(
                "allow-declaration-merging: {}",
                config.rules.no_interface.allow_declaration_merging
            )],
        },
        RuleKind::NoLargeFile => RuleConfigSummary {
            severity: config.rules.no_large_file.severity,
            options: vec![format!(
                "max-lines: {}",
                config.rules.no_large_file.max_lines
            )],
        },
        RuleKind::NoLogicInBarrel => simple_summary(config.rules.no_logic_in_barrel.severity),
        RuleKind::NoLogicInDomain => RuleConfigSummary {
            severity: config.rules.no_logic_in_domain.severity,
            options: vec![
                format!(
                    "extra-folders: {:?}",
                    config.rules.no_logic_in_domain.extra_folders
                ),
                format!(
                    "extra-file-suffixes: {:?}",
                    config.rules.no_logic_in_domain.extra_file_suffixes
                ),
            ],
        },
        RuleKind::NoMutableExports => simple_summary(config.rules.no_mutable_exports.severity),
        RuleKind::NoNamespace => simple_summary(config.rules.no_namespace.severity),
        RuleKind::NoSilentCatch => simple_summary(config.rules.no_silent_catch.severity),
        RuleKind::NoThenChain => simple_summary(config.rules.no_then_chain.severity),
        RuleKind::NoUpwardImport => RuleConfigSummary {
            severity: config.rules.no_upward_import.severity,
            options: vec![format!(
                "max-depth: {}",
                config.rules.no_upward_import.max_depth
            )],
        },
        RuleKind::PreferSatisfies => simple_summary(config.rules.prefer_satisfies.severity),
    }
}

fn simple_summary(severity: Severity) -> RuleConfigSummary {
    RuleConfigSummary {
        severity,
        options: Vec::new(),
    }
}

#[derive(Serialize)]
struct ConfiguredRuleJson {
    name: &'static str,
    severity: &'static str,
}

#[derive(Serialize)]
struct RuleExplanationJson {
    name: &'static str,
    severity: &'static str,
    intent: &'static str,
    examples: Vec<RuleExampleJson>,
    options: Vec<RuleOptionJson>,
    current_config: CurrentConfigJson,
}

#[derive(Serialize)]
struct RuleExampleJson {
    label: &'static str,
    code: &'static str,
}

#[derive(Serialize)]
struct RuleOptionJson {
    name: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct CurrentConfigJson {
    severity: &'static str,
    options: Vec<String>,
}

pub fn render_rules_json(rules: &[ConfiguredRule]) -> Result<String> {
    let rules_json: Vec<ConfiguredRuleJson> = rules
        .iter()
        .map(|r| ConfiguredRuleJson {
            name: r.name,
            severity: r.severity.as_str(),
        })
        .collect();

    Ok(serde_json::to_string_pretty(&rules_json)?)
}

pub fn render_explanation_json(explanation: &RuleExplanation) -> Result<String> {
    let json = RuleExplanationJson {
        name: explanation.name,
        severity: explanation.severity.as_str(),
        intent: explanation.intent,
        examples: explanation
            .examples
            .iter()
            .map(|e| RuleExampleJson {
                label: e.label,
                code: e.code,
            })
            .collect(),
        options: explanation
            .options
            .iter()
            .map(|o| RuleOptionJson {
                name: o.name,
                description: o.description,
            })
            .collect(),
        current_config: CurrentConfigJson {
            severity: explanation.current_severity.as_str(),
            options: explanation.current_options.clone(),
        },
    };

    Ok(serde_json::to_string_pretty(&json)?)
}
