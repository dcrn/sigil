use rmcp::schemars;
use serde::{Deserialize, Serialize};
use toml;

const SCHEMA_STR: &str = include_str!("../../schema/contract.schema.json");

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// The full contract content as an object matching the contract schema.
    pub contract: serde_json::Value,
}

#[derive(Serialize)]
struct Response {
    path: String,
    warnings: Vec<String>,
}

pub async fn handle(server: &super::CddServer, params: Params) -> String {
    // Schema validation
    let schema_json: serde_json::Value = serde_json::from_str(SCHEMA_STR).unwrap();
    let validator = jsonschema::validator_for(&schema_json).expect("contract schema is valid JSON Schema");
    let schema_errors: Vec<String> = validator
        .iter_errors(&params.contract)
        .map(|e| format!("{} at '{}'", e, e.instance_path))
        .collect();
    if !schema_errors.is_empty() {
        return serde_json::json!({ "error": "Schema validation failed", "validation": schema_errors })
            .to_string();
    }

    // Extract id
    let Some(id) = params.contract.get("id").and_then(|v| v.as_str()) else {
        return super::error_response("Contract must have an 'id' field");
    };
    let id = id.to_string();

    // Reject duplicate
    let contracts_dir = server.config.contracts_dir.trim_end_matches('/');
    let path = format!("{contracts_dir}/{id}.contract.toml");
    if std::path::Path::new(&path).exists() {
        return super::error_response(format!(
            "Contract '{id}' already exists at '{path}'. Use cdd_update_contract to modify it."
        ));
    }

    // Serialize to TOML via typed struct to get consistent field order
    let toml_str = match serde_json::from_value::<crate::model::Contract>(params.contract.clone())
        .map_err(|e| e.to_string())
        .and_then(|c| toml::to_string_pretty(&c).map_err(|e| e.to_string()))
    {
        Ok(s) => s,
        Err(e) => return super::error_response(format!("Failed to serialize contract: {e}")),
    };

    // Write file
    if let Err(e) = std::fs::write(&path, &toml_str) {
        return super::error_response(format!("Failed to write '{path}': {e}"));
    }

    // Warn on missing files
    let mut warnings = Vec::new();
    if let Ok(contract) = serde_json::from_value::<crate::model::Contract>(params.contract) {
        for path in contract.all_files() {
            if !std::path::Path::new(path).exists() {
                warnings.push(format!("File does not exist yet: '{path}'"));
            }
        }
    }

    serde_json::to_string(&Response { path, warnings }).unwrap()
}
