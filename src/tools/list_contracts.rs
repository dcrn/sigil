use crate::model::{Contract, Priority, Status};
use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// Filter by domain (exact match, case-sensitive). Omit to return all domains.
    pub domain: Option<String>,
    /// Filter by tags (OR logic: contracts matching any provided tag are returned).
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
struct Response {
    contracts: Vec<Summary>,
    total: usize,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct Summary {
    id: String,
    version: String,
    name: String,
    description: String,
    priority: Priority,
    status: Status,
    domain: Option<String>,
    tags: Option<Vec<String>>,
    trigger_type: Option<String>,
    file_count: usize,
}

pub async fn handle(server: &super::CddServer, params: Params) -> String {
    let (contracts, mut warnings) = super::loader::load_contracts(&server.config.contracts_dir);
    server.mark_listed();

    let filtered: Vec<&Contract> = contracts
        .iter()
        .filter(|c| {
            if let Some(d) = &params.domain {
                if c.domain.as_deref() != Some(d.as_str()) {
                    return false;
                }
            }
            if let Some(filter_tags) = &params.tags {
                let contract_tags = c.tags.as_deref().unwrap_or(&[]);
                if !filter_tags.iter().any(|t| contract_tags.contains(t)) {
                    return false;
                }
            }
            true
        })
        .collect();

    let summaries: Vec<Summary> = filtered
        .iter()
        .map(|c| {
            // Warn on missing files
            for path in c.all_files() {
                if !std::path::Path::new(path).exists() {
                    warnings.push(format!("Contract '{}': missing file '{}'", c.id, path));
                }
            }

            let file_count = c.all_files().len();

            Summary {
                id: c.id.clone(),
                version: c.version.clone(),
                name: c.name.clone(),
                description: c.description.clone(),
                priority: c.priority.clone(),
                status: c.status.clone(),
                domain: c.domain.clone(),
                tags: c.tags.clone(),
                trigger_type: c.trigger.as_ref().and_then(|t| t.kind.clone()),
                file_count,
            }
        })
        .collect();

    let total = summaries.len();
    serde_json::to_string(&Response {
        contracts: summaries,
        total,
        warnings,
    })
    .unwrap()
}
