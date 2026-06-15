use std::collections::{HashMap, HashSet};

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

#[test]
fn every_metadata_entry_has_catalog_entry() {
    let catalog_ids = rule_ids_from_catalog_source();
    let raw = include_str!("../src/config/rule_metadata.rs");

    // Collect rule IDs from metadata source
    let mut metadata_ids = std::collections::HashSet::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(id_val) = trimmed.strip_prefix("id: \"") {
            if let Some(end) = id_val.find('"') {
                metadata_ids.insert(id_val[..end].to_string());
            }
        }
    }

    for id in &metadata_ids {
        assert!(
            catalog_ids.contains(id),
            "rule '{id}' is in rule_metadata.rs but missing from catalog.rs"
        );
    }
}

fn fixable_rule_ids_from_metadata() -> HashSet<String> {
    let raw = include_str!("../src/config/rule_metadata.rs");
    let mut ids = HashSet::new();
    let mut current_id: Option<String> = None;
    let mut current_fix_capability: Option<String> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(id_val) = trimmed.strip_prefix("id: \"") {
            if let Some(end) = id_val.find('"') {
                current_id = Some(id_val[..end].to_string());
            }
        }
        if let Some(capability) = trimmed.strip_prefix("fix_capability: FixCapability::") {
            current_fix_capability = Some(capability.trim_end_matches(',').to_string());
        }
        if trimmed == "}," || trimmed == "}" {
            if let Some(capability) = current_fix_capability.take() {
                if capability != "None" {
                    if let Some(id) = current_id.take() {
                        ids.insert(id);
                    }
                }
            }
            current_id = None;
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
        if !trimmed.starts_with("fixable_ast_rule_adapter!(")
            && !trimmed.starts_with("fixable_text_rule_adapter!(")
        {
            continue;
        }
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
    ids
}

#[test]
fn every_fixable_metadata_rule_has_fixable_adapter() {
    let metadata_ids = fixable_rule_ids_from_metadata();
    let adapter_ids = fixable_rule_ids_from_adapters();
    for id in &metadata_ids {
        assert!(
            adapter_ids.contains(id),
            "rule '{id}' has a non-None fix_capability in metadata but no fixable adapter"
        );
    }
}

#[test]
fn every_fixable_adapter_rule_has_fixable_metadata() {
    let metadata_ids = fixable_rule_ids_from_metadata();
    let adapter_ids = fixable_rule_ids_from_adapters();
    for id in &adapter_ids {
        assert!(
            metadata_ids.contains(id),
            "rule '{id}' has a fixable adapter but None fix_capability in metadata"
        );
    }
}

#[test]
fn fix_docs_mentions_every_fixable_rule() {
    let fixable_ids = fixable_rule_ids_from_metadata();
    let docs = include_str!("../docs/fix.md");
    for id in &fixable_ids {
        assert!(
            docs.contains(&format!("`{id}`")),
            "rule '{id}' supports fix but is not documented in docs/fix.md"
        );
    }
}

fn rule_capabilities_from_metadata() -> HashMap<String, String> {
    let raw = include_str!("../src/config/rule_metadata.rs");
    let mut map = HashMap::new();
    let mut current_id: Option<String> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(id_val) = trimmed.strip_prefix("id: \"") {
            if let Some(end) = id_val.find('"') {
                current_id = Some(id_val[..end].to_string());
            }
        }
        if let Some(capability) = trimmed.strip_prefix("fix_capability: FixCapability::") {
            let capability = capability.trim_end_matches(',').to_string();
            if let Some(id) = current_id.take() {
                map.insert(id, capability);
            }
        }
        if trimmed == "}," || trimmed == "}" {
            current_id = None;
        }
    }
    map
}

#[test]
fn fix_capability_classifications_match_plan() {
    let capabilities = rule_capabilities_from_metadata();
    let expected = [
        ("no-debugger", "Safe"),
        ("no-focused-test", "Safe"),
        ("no-skipped-test", "Safe"),
        ("no-empty-interface", "Conditional"),
    ];

    for (id, expected_capability) in expected {
        let actual = capabilities
            .get(id)
            .unwrap_or_else(|| panic!("rule '{id}' missing fix_capability"));
        assert_eq!(
            actual, expected_capability,
            "rule '{id}' has unexpected fix capability"
        );
    }
}
