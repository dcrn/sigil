use crate::model::Contract;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// The id of the contract to retrieve.
    pub contract_id: String,
    /// When true, includes the file contents of all files referenced in the contract.
    pub retrieve_file_contents: Option<bool>,
}

#[derive(Serialize)]
struct Response {
    contract: Contract,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_contents: Option<HashMap<String, FileContent>>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum FileContent {
    Ok { contents: String },
    Missing,
    Error { message: String },
}

pub async fn handle(server: &super::SigilServer, params: Params) -> String {
    if let Err(e) = server.require_listed("sigil_get_contract", &params.contract_id) {
        return e;
    }

    let (contracts, mut warnings) = super::loader::load_contracts(&server.config.contracts_dir);
    let Some(contract) = contracts.into_iter().find(|c| c.id == params.contract_id) else {
        return super::error_response(format!("Contract '{}' not found", params.contract_id));
    };

    server.mark_read(&params.contract_id);

    let file_contents = if params.retrieve_file_contents == Some(true) {
        let mut map = HashMap::new();
        for path in contract.all_files() {
            let resolved = match std::fs::read_to_string(path) {
                Ok(contents) => FileContent::Ok { contents },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    warnings.push(format!("Missing file: '{path}'"));
                    FileContent::Missing
                }
                Err(e) => {
                    warnings.push(format!("Error reading file '{path}': {e}"));
                    FileContent::Error {
                        message: e.to_string(),
                    }
                }
            };
            map.insert(path.to_string(), resolved);
        }
        Some(map)
    } else {
        // Still warn on missing files even without retrieval
        for path in contract.all_files() {
            if !std::path::Path::new(path).exists() {
                warnings.push(format!("Missing file: '{path}'"));
            }
        }
        None
    };

    serde_json::to_string(&Response {
        contract,
        file_contents,
        warnings,
    })
    .unwrap()
}
