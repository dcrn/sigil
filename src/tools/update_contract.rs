use rmcp::schemars;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use toml;

const SCHEMA_STR: &str = include_str!("../../schema/contract.schema.json");

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// The id of the contract to update.
    pub contract_id: String,
    /// Fields to update. Unspecified top-level fields are preserved from the original.
    /// Providing a list field (e.g., rules) replaces the entire list.
    pub updates: serde_json::Value,
    /// If provided, a changelog entry is appended with the current contract version,
    /// today's date, and this message as the description.
    pub changelog_message: Option<String>,
}

#[derive(Serialize)]
struct Response {
    path: String,
    diff: String,
    warnings: Vec<String>,
}

pub async fn handle(server: &super::SigilServer, params: Params) -> String {
    if let Err(e) = server.require_read("sigil_update_contract", &params.contract_id) {
        return e;
    }

    let contracts_dir = server.config.contracts_dir.trim_end_matches('/');
    let old_path = format!("{contracts_dir}/{}.contract.toml", params.contract_id);

    let old_yaml = match std::fs::read_to_string(&old_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return super::error_response(format!(
                "Contract '{}' not found. Use sigil_create_contract to create it.",
                params.contract_id
            ));
        }
        Err(e) => return super::error_response(format!("Failed to read '{old_path}': {e}")),
    };

    // Parse existing contract as JSON Value for merging
    let mut merged: serde_json::Value = match toml::from_str(&old_yaml) {
        Ok(v) => v,
        Err(e) => return super::error_response(format!("Failed to parse existing contract: {e}")),
    };

    // Shallow merge: updates overwrite top-level fields
    if let (Some(base), Some(updates)) = (merged.as_object_mut(), params.updates.as_object()) {
        for (k, v) in updates {
            base.insert(k.clone(), v.clone());
        }
    } else {
        return super::error_response("'updates' must be a JSON object");
    }

    // Append changelog entry if changelog_message is provided
    if let Some(message) = &params.changelog_message {
        let version = merged
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();
        let today = chrono::Local::now().date_naive().to_string();
        let entry = serde_json::json!({
            "version": version,
            "date": today,
            "description": message,
        });
        match merged.get_mut("changelog") {
            Some(serde_json::Value::Array(arr)) => arr.push(entry),
            _ => {
                merged
                    .as_object_mut()
                    .unwrap()
                    .insert("changelog".to_string(), serde_json::json!([entry]));
            }
        }
    }

    // Schema validation
    let schema_json: serde_json::Value = serde_json::from_str(SCHEMA_STR).unwrap();
    let validator = jsonschema::validator_for(&schema_json).expect("contract schema is valid JSON Schema");
    let schema_errors: Vec<String> = validator
        .iter_errors(&merged)
        .map(|e| format!("{} at '{}'", e, e.instance_path))
        .collect();
    if !schema_errors.is_empty() {
        return serde_json::json!({ "error": "Schema validation failed", "validation": schema_errors })
            .to_string();
    }

    // Determine output path (id may have changed)
    let new_id = merged
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(&params.contract_id)
        .to_string();
    let new_path = format!("{contracts_dir}/{new_id}.contract.toml");

    // Check for id collision if id changed
    if new_id != params.contract_id && std::path::Path::new(&new_path).exists() {
        return super::error_response(format!(
            "Cannot rename to '{new_id}': a contract with that id already exists at '{new_path}'."
        ));
    }

    // Serialize merged contract to TOML via typed struct to get consistent field order
    let new_toml = match serde_json::from_value::<crate::model::Contract>(merged.clone())
        .map_err(|e| e.to_string())
        .and_then(|c| toml::to_string_pretty(&c).map_err(|e| e.to_string()))
    {
        Ok(s) => s,
        Err(e) => return super::error_response(format!("Failed to serialize contract: {e}")),
    };

    // Write new file
    if let Err(e) = std::fs::write(&new_path, &new_toml) {
        return super::error_response(format!("Failed to write '{new_path}': {e}"));
    }

    // Remove old file if id changed
    if new_id != params.contract_id {
        let _ = std::fs::remove_file(&old_path);
    }

    // Build diff
    let diff_text = TextDiff::from_lines(&old_yaml, &new_toml)
        .iter_all_changes()
        .filter(|c| c.tag() != ChangeTag::Equal)
        .map(|c| {
            let prefix = match c.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            format!("{prefix}{c}")
        })
        .collect::<String>();
    let diff = if diff_text.is_empty() {
        "(no changes)".to_string()
    } else {
        diff_text
    };

    // Warn on missing files
    let mut warnings = Vec::new();
    if let Ok(contract) = serde_json::from_value::<crate::model::Contract>(merged) {
        for path in contract.all_files() {
            if !std::path::Path::new(path).exists() {
                warnings.push(format!("File does not exist: '{path}'"));
            }
        }
    }

    serde_json::to_string(&Response { path: new_path, diff, warnings }).unwrap()
}

