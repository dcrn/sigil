use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const SCHEMA_STR: &str = include_str!("../../schema/contract.schema.json");

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// The id of the contract to validate.
    pub contract_id: String,
}

#[derive(Serialize)]
struct Response {
    pass: bool,
    errors: Vec<Issue>,
    warnings: Vec<Issue>,
}

#[derive(Serialize)]
struct Issue {
    kind: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
}

pub async fn handle(server: &super::SigilServer, params: Params) -> String {
    let (contracts, load_warnings) = super::loader::load_contracts(&server.config.contracts_dir);

    let mut errors: Vec<Issue> = Vec::new();
    let mut warnings: Vec<Issue> = load_warnings
        .into_iter()
        .map(|m| Issue { kind: "load_warning", message: m, file: None })
        .collect();

    let Some(contract) = contracts.iter().find(|c| c.id == params.contract_id) else {
        return super::error_response(format!("Contract '{}' not found", params.contract_id));
    };

    // Schema validation
    let schema_json: serde_json::Value = serde_json::from_str(SCHEMA_STR).unwrap();
    let validator = jsonschema::validator_for(&schema_json).expect("contract schema is valid JSON Schema");
    let contract_json = serde_json::to_value(contract).unwrap();
    for error in validator.iter_errors(&contract_json) {
        errors.push(Issue {
            kind: "schema",
            message: format!("{} at '{}'", error, error.instance_path),
            file: None,
        });
    }

    // Missing files
    for path in contract.all_files() {
        if !std::path::Path::new(path).exists() {
            errors.push(Issue {
                kind: "missing_file",
                message: format!("Referenced file does not exist: '{path}'"),
                file: Some(path.to_string()),
            });
        }
    }

    // Unique rule ids
    if let Some(rules) = &contract.rules {
        let mut seen = HashSet::new();
        for b in rules {
            if !seen.insert(b.id.as_str()) {
                errors.push(Issue {
                    kind: "duplicate_rule_id",
                    message: format!("Duplicate rule id: '{}'", b.id),
                    file: None,
                });
            }
        }
    }

    // Filename-id consistency
    let expected_path = format!(
        "{}/{}.contract.toml",
        server.config.contracts_dir.trim_end_matches('/'),
        contract.id
    );
    if !std::path::Path::new(&expected_path).exists() {
        warnings.push(Issue {
            kind: "filename_mismatch",
            message: format!(
                "No file found at expected path '{expected_path}' for contract id '{}'",
                contract.id
            ),
            file: Some(expected_path),
        });
    }

    let pass = errors.is_empty();
    serde_json::to_string(&Response { pass, errors, warnings }).unwrap()
}
