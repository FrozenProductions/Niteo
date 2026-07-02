use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Parse rule IDs from the catalog source as a fallback for external tests.
/// Only matches `name:` lines that belong to a `RuleDocumentation` block,
/// not `RuleOption` blocks.
fn rule_ids_from_catalog_source() -> HashSet<String> {
    let raw = include_str!("../src/rule_documentation/catalog.rs");
    let mut ids = HashSet::new();
    for block in catalog_rule_blocks(raw) {
        if let Some(id) = rule_id_from_catalog_block(block) {
            ids.insert(id);
        }
    }
    ids
}

fn catalog_rule_blocks(raw: &str) -> impl Iterator<Item = &str> {
    raw.split("const RULE_DOCUMENTATION")
        .nth(1)
        .unwrap_or("")
        .split("RuleDocumentation {")
        .skip(1)
        .filter_map(|block| block.split("\n    },").next())
}

fn rule_id_from_catalog_block(block: &str) -> Option<String> {
    for line in block.lines() {
        let trimmed = line.trim();
        let Some(name) = trimmed.strip_prefix("name: \"") else {
            continue;
        };
        let end = name.find('"')?;
        return Some(name[..end].to_string());
    }
    None
}

fn rule_ids_from_preset_source() -> Vec<String> {
    let raw = include_str!("../src/config/presets.rs");
    let mut ids = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("[rules.") {
            if let Some(end) = rest.find(']') {
                ids.push(rest[..end].to_string());
            }
        }
    }
    ids
}

fn known_rule_set() -> HashSet<String> {
    niteo::rules::known_rule_ids()
        .iter()
        .map(|id| id.to_string())
        .collect()
}

#[test]
fn every_known_rule_has_catalog_entry() -> Result<()> {
    let known = known_rule_set();
    let catalog_ids = rule_ids_from_catalog_source();
    for id in &known {
        assert!(
            catalog_ids.contains(id),
            "rule '{id}' is in known_rule_ids() but missing from catalog.rs"
        );
    }
    Ok(())
}

#[test]
fn every_catalog_rule_is_known() -> Result<()> {
    let known = known_rule_set();
    let catalog_ids = rule_ids_from_catalog_source();
    for id in &catalog_ids {
        assert!(
            known.contains(id),
            "rule '{id}' is documented in catalog.rs but missing from known_rule_ids()"
        );
    }
    Ok(())
}

#[test]
fn preset_refs_only_known_rules() -> Result<()> {
    let known = known_rule_set();
    let preset_ids = rule_ids_from_preset_source();
    let mut seen = HashSet::new();
    for id in preset_ids {
        if seen.insert(id.clone()) {
            assert!(known.contains(&id), "preset references unknown rule '{id}'");
        }
    }
    Ok(())
}

#[test]
fn docs_rules_md_mentions_every_known_rule() -> Result<()> {
    let known = known_rule_set();
    let docs = include_str!("../docs/rules.md");
    for id in &known {
        let in_table = docs.contains(&format!("| `{id}`"));
        let in_heading = docs.contains(&format!("### `{id}`"));
        assert!(
            in_table || in_heading,
            "rule '{id}' is in known_rule_ids() but not mentioned in docs/rules.md"
        );
    }
    Ok(())
}

#[test]
fn every_configurable_rule_has_metadata_entry() -> Result<()> {
    let known = known_rule_set();
    let catalog_ids = rule_ids_from_catalog_source();

    for id in &known {
        assert!(
            catalog_ids.contains(id),
            "rule '{id}' missing from merged rule catalog"
        );
    }
    Ok(())
}

#[test]
fn every_metadata_entry_has_catalog_entry() -> Result<()> {
    let known = known_rule_set();
    let catalog_ids = rule_ids_from_catalog_source();

    for id in &catalog_ids {
        assert!(
            known.contains(id),
            "rule '{id}' is in merged rule catalog but missing from known_rule_ids()"
        );
    }
    Ok(())
}

fn fixable_rule_ids_from_catalog() -> HashSet<String> {
    let raw = include_str!("../src/rule_documentation/catalog.rs");
    let mut ids = HashSet::new();
    for block in catalog_rule_blocks(raw) {
        if block.contains("fixable: true,") {
            if let Some(id) = rule_id_from_catalog_block(block) {
                ids.insert(id);
            }
        }
    }
    ids
}

fn rule_id_constants() -> HashMap<String, String> {
    let raw = include_str!("../src/rules.rs");
    let mut map = HashMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(id_start) = trimmed.find("id: ") {
            if let Some(id_end) = trimmed[id_start..].find(',') {
                let id_const = trimmed[id_start + 4..id_start + id_end].trim().to_string();
                if let Some(value_start) = trimmed.find("value: ") {
                    let value_offset = value_start + 7;
                    if value_offset < trimmed.len() && trimmed.as_bytes()[value_offset] == b'"' {
                        let value_body = &trimmed[value_offset + 1..];
                        if let Some(value_end) = value_body.find('"') {
                            let value = value_body[..value_end].to_string();
                            map.insert(id_const, value);
                        }
                    }
                }
            }
        }
    }
    map
}

fn fixable_rule_ids_from_adapters() -> HashSet<String> {
    let raw = include_str!("../src/rule_adapters.rs");
    let constants = rule_id_constants();
    let mut ids = HashSet::new();
    let mut lines = raw.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with("fixable_ast_rule_adapter!(")
            || trimmed.starts_with("fixable_text_rule_adapter!(")
        {
            let mut macro_lines = vec![trimmed.to_string()];
            while let Some(next) = lines.next() {
                let next_trimmed = next.trim();
                macro_lines.push(next_trimmed.to_string());
                if next_trimmed.ends_with(");") {
                    break;
                }
            }
            let full = macro_lines.join(" ");
            if let Some(args) = full.strip_prefix("fixable_ast_rule_adapter!(") {
                if let Some(end) = args.rfind(");") {
                    let parts: Vec<&str> = args[..end].split(',').collect();
                    if parts.len() >= 2 {
                        let id = parts[1].trim();
                        if let Some(value) = constants.get(id) {
                            ids.insert(value.clone());
                        }
                    }
                }
            }
            if let Some(args) = full.strip_prefix("fixable_text_rule_adapter!(") {
                if let Some(end) = args.rfind(");") {
                    let parts: Vec<&str> = args[..end].split(',').collect();
                    if parts.len() >= 2 {
                        let id = parts[1].trim();
                        if let Some(value) = constants.get(id) {
                            ids.insert(value.clone());
                        }
                    }
                }
            }
        }
    }

    // `NoAnyAdapter` uses a manual `supports_fix`/`fix` impl (custom `check` signature
    // with `generated` field), so it is not detected by the macro scan above.
    if let Some(any) = constants.get("NO_ANY_RULE_ID") {
        ids.insert(any.clone());
    }
    // `SortImportsAdapter` uses a manual `supports_fix`/`fix` impl (custom `check`
    // signature with `source` parameter), so it is not detected by the macro scan above.
    if let Some(id) = constants.get("SORT_IMPORTS_RULE_ID") {
        ids.insert(id.clone());
    }
    // `SortExportsAdapter` uses a manual `supports_fix`/`fix` impl (custom `check`
    // signature with `source` parameter), so it is not detected by the macro scan above.
    if let Some(id) = constants.get("SORT_EXPORTS_RULE_ID") {
        ids.insert(id.clone());
    }

    ids
}

#[test]
fn every_fixable_metadata_rule_has_fixable_adapter() -> Result<()> {
    let metadata_ids = fixable_rule_ids_from_catalog();
    let adapter_ids = fixable_rule_ids_from_adapters();
    for id in &metadata_ids {
        assert!(
            adapter_ids.contains(id),
            "rule '{id}' is marked fixable in the catalog but has no fixable adapter"
        );
    }
    Ok(())
}

#[test]
fn every_fixable_adapter_rule_has_fixable_metadata() -> Result<()> {
    let metadata_ids = fixable_rule_ids_from_catalog();
    let adapter_ids = fixable_rule_ids_from_adapters();
    for id in &adapter_ids {
        assert!(
            metadata_ids.contains(id),
            "rule '{id}' has a fixable adapter but is not marked fixable in the catalog"
        );
    }
    Ok(())
}

#[test]
fn fix_docs_mentions_every_fixable_rule() -> Result<()> {
    let fixable_ids = fixable_rule_ids_from_catalog();
    let docs = include_str!("../docs/fix.md");
    for id in &fixable_ids {
        assert!(
            docs.contains(&format!("`{id}`")),
            "rule '{id}' supports fix but is not documented in docs/fix.md"
        );
    }
    Ok(())
}

#[test]
fn fix_capability_classifications_match_plan() -> Result<()> {
    let fixable_ids = fixable_rule_ids_from_catalog();
    let expected = [
        "no-debugger",
        "no-focused-test",
        "no-skipped-test",
        "no-empty-interface",
    ];

    for id in expected {
        assert!(
            fixable_ids.contains(id),
            "rule '{id}' missing fixable catalog entry"
        );
    }
    Ok(())
}
