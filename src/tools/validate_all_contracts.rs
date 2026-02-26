use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const SCHEMA_STR: &str = include_str!("../../schema/contract.schema.json");

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {}

#[derive(Serialize)]
struct Response {
    pass: bool,
    errors: Vec<Issue>,
    warnings: Vec<Issue>,
}

#[derive(Serialize)]
struct Issue {
    kind: &'static str,
    contract_id: Option<String>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
}

pub async fn handle(server: &super::CddServer, _params: Params) -> String {
    let (contracts, load_warnings) = super::loader::load_contracts(&server.config.contracts_dir);

    let mut errors: Vec<Issue> = Vec::new();
    let mut warnings: Vec<Issue> = load_warnings
        .into_iter()
        .map(|m| Issue { kind: "load_warning", contract_id: None, message: m, file: None })
        .collect();

    let schema_json: serde_json::Value = serde_json::from_str(SCHEMA_STR).unwrap();
    let validator = jsonschema::validator_for(&schema_json).expect("contract schema is valid JSON Schema");
    let contracts_dir = server.config.contracts_dir.trim_end_matches('/');

    for contract in &contracts {
        let cid = Some(contract.id.clone());

        // Schema validation
        let contract_json = serde_json::to_value(contract).unwrap();
        for error in validator.iter_errors(&contract_json) {
            errors.push(Issue {
                kind: "schema",
                contract_id: cid.clone(),
                message: format!("{} at '{}'", error, error.instance_path),
                file: None,
            });
        }

        // Missing files
        for path in contract.all_files() {
            if !std::path::Path::new(path).exists() {
                errors.push(Issue {
                    kind: "missing_file",
                    contract_id: cid.clone(),
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
                        contract_id: cid.clone(),
                        message: format!("Duplicate rule id: '{}'", b.id),
                        file: None,
                    });
                }
            }
        }

        // Filename-id consistency
        let expected_path = format!("{contracts_dir}/{}.contract.toml", contract.id);
        if !std::path::Path::new(&expected_path).exists() {
            warnings.push(Issue {
                kind: "filename_mismatch",
                contract_id: cid.clone(),
                message: format!(
                    "Contract id '{}' has no matching file at '{expected_path}'",
                    contract.id
                ),
                file: Some(expected_path),
            });
        }
    }

    let pass = errors.is_empty();
    serde_json::to_string(&Response { pass, errors, warnings }).unwrap()
}
