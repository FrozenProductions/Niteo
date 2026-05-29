#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleDocumentation {
    pub(crate) name: &'static str,
    pub(crate) intent: &'static str,
    pub(crate) examples: &'static [RuleExample],
    pub(crate) options: &'static [RuleOption],
    pub(crate) kind: RuleKind,
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

macro_rules! define_rule_kinds {
    ( $( $variant:ident ),* $(,)? ) => {
        #[derive(Debug, Clone, Copy)]
        pub(crate) enum RuleKind {
            $( $variant, )*
        }
    };
}

define_rule_kinds! {
    BooleanPrefix,
    ComponentFileOnlyComponents,
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
    NoComponentDefaultExport,
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
    NoTestCodeInProduction,
    NoThenChain,
    NoUpwardImport,
    PreferSatisfies,
    NoTestImport,
    EntryFileNoLogic,
    NoNonNullAssertion,
    NoAny,
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
        kind: RuleKind::BooleanPrefix,
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
        kind: RuleKind::ComponentFileOnlyComponents,
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
            RuleOption {
                name: "project.structure.hooks",
                description: "Folders and file suffixes that identify hook files.",
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
        name: "no-component-default-export",
        intent: "Components must use named exports so imports stay explicit and refactorable.",
        examples: &[
            RuleExample {
                label: "Reports",
                code: "export default function Button() {} // in Button.tsx",
            },
            RuleExample {
                label: "Prefer",
                code: "export function Button() {}",
            },
        ],
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.components",
                description: "Folders and file suffixes that identify component files.",
            },
        ],
        kind: RuleKind::NoComponentDefaultExport,
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
        options: &[
            SEVERITY_OPTION,
            RuleOption {
                name: "project.structure.types",
                description: "Folders and file suffixes that identify type files. Declaration files (.d.ts) are always allowed.",
            },
        ],
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
                name: "project.structure.types",
                description: "Folders and file suffixes that identify type domain files.",
            },
            RuleOption {
                name: "project.structure.constants",
                description: "Folders and file suffixes that identify constants domain files.",
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
        kind: RuleKind::NoTestCodeInProduction,
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
        kind: RuleKind::NoTestImport,
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
        kind: RuleKind::EntryFileNoLogic,
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
        kind: RuleKind::NoNonNullAssertion,
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
        kind: RuleKind::NoAny,
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
