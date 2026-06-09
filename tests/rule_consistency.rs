use std::collections::HashSet;

/// Parse rule IDs from the catalog source as a fallback for external tests.
/// Only matches `name:` lines that belong to a `RuleDocumentation` block,
/// not `RuleOption` blocks.
fn rule_ids_from_catalog_source() -> HashSet<String> {
    let raw = include_str!("../src/rule_documentation/catalog.rs");
    let mut ids = HashSet::new();
    let mut in_rule_doc = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "RuleDocumentation {" {
            in_rule_doc = true;
            continue;
        }
        if in_rule_doc {
            if trimmed == "}," || trimmed == "}" {
                in_rule_doc = false;
                continue;
            }
            if let Some(name) = trimmed.strip_prefix("name: \"") {
                if let Some(end) = name.find('"') {
                    ids.insert(name[..end].to_string());
                }
            }
        }
    }
    ids
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
fn every_known_rule_has_catalog_entry() {
    let known = known_rule_set();
    let catalog_ids = rule_ids_from_catalog_source();
    for id in &known {
        assert!(
            catalog_ids.contains(id),
            "rule '{id}' is in known_rule_ids() but missing from catalog.rs"
        );
    }
}

#[test]
fn every_catalog_rule_is_known() {
    let known = known_rule_set();
    let catalog_ids = rule_ids_from_catalog_source();
    for id in &catalog_ids {
        assert!(
            known.contains(id),
            "rule '{id}' is documented in catalog.rs but missing from known_rule_ids()"
        );
    }
}

#[test]
fn preset_refs_only_known_rules() {
    let known = known_rule_set();
    let preset_ids = rule_ids_from_preset_source();
    let mut seen = HashSet::new();
    for id in preset_ids {
        if seen.insert(id.clone()) {
            assert!(known.contains(&id), "preset references unknown rule '{id}'");
        }
    }
}

#[test]
fn docs_rules_md_mentions_every_known_rule() {
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
}

#[test]
fn every_configurable_rule_has_metadata_entry() {
    let known = known_rule_set();
    let raw = include_str!("../src/config/rule_metadata.rs");

    for id in &known {
        let quoted = format!("id: \"{id}\"");
        assert!(
            raw.contains(&quoted),
            "rule '{id}' missing from rule_metadata.rs"
        );
    }
}
