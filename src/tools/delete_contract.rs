use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// The id of the contract to delete.
    pub contract_id: String,
}

#[derive(Serialize)]
struct Response {
    deleted: String,
}

pub async fn handle(server: &super::SigilServer, params: Params) -> String {
    if let Err(e) = server.require_read("sigil_delete_contract", &params.contract_id) {
        return e;
    }

    let contracts_dir = server.config.contracts_dir.trim_end_matches('/');
    let path = format!("{contracts_dir}/{}.contract.toml", params.contract_id);

    match std::fs::remove_file(&path) {
        Ok(()) => serde_json::to_string(&Response { deleted: path }).unwrap(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            super::error_response(format!("Contract '{}' not found at '{path}'", params.contract_id))
        }
        Err(e) => super::error_response(format!("Failed to delete '{path}': {e}")),
    }
}
