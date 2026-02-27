use crate::model::{Contract, Priority, Status};
use globset::Glob;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// The file paths that changed in this changeset.
    pub files: Vec<String>,
    /// Optional diff string for additional context (not interpreted by the server).
    pub diff: Option<String>,
}

#[derive(Serialize)]
struct Response {
    affected_contracts: Vec<AffectedEntry>,
    total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct AffectedEntry {
    id: String,
    version: String,
    name: String,
    description: String,
    priority: Priority,
    status: Status,
    domain: Option<String>,
    tags: Option<Vec<String>>,
    trigger_type: Option<String>,
    matched_files: Vec<String>,
    contract: Contract,
    file_contents: HashMap<String, FileContent>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum FileContent {
    Ok { contents: String },
    Missing,
    Error { message: String },
}

pub async fn handle(server: &super::SigilServer, params: Params) -> String {
    let (contracts, mut warnings) = super::loader::load_contracts(&server.config.contracts_dir);
    server.mark_listed();

    let files: Vec<String> = params.files.iter().map(|f| f.replace("\\", "/")).collect();
    let mut entries = Vec::new();

    for contract in contracts {
        let contract_files: Vec<&str> = contract.all_files();

        // Direct file matches
        let mut matched: Vec<String> = files
            .iter()
            .filter(|f| contract_files.contains(&f.as_str()))
            .cloned()
            .collect();

        // applies_to glob matches
        for pattern in contract.applies_to_patterns() {
            match Glob::new(pattern) {
                Ok(glob) => {
                    let matcher = glob.compile_matcher();
                    for f in &files {
                        if matcher.is_match(f.as_str()) && !matched.contains(f) {
                            matched.push(f.clone());
                        }
                    }
                }
                Err(e) => warnings.push(format!(
                    "Contract '{}': invalid applies_to pattern '{pattern}': {e}",
                    contract.id
                )),
            }
        }

        if matched.is_empty() {
            continue;
        }

        // Retrieve file contents
        let mut file_contents = HashMap::new();
        for path in contract.all_files() {
            let resolved = match std::fs::read_to_string(path) {
                Ok(contents) => FileContent::Ok { contents },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    warnings.push(format!("Contract '{}': missing file '{path}'", contract.id));
                    FileContent::Missing
                }
                Err(e) => FileContent::Error { message: e.to_string() },
            };
            file_contents.insert(path.to_string(), resolved);
        }

        server.mark_read(&contract.id);

        entries.push(AffectedEntry {
            id: contract.id.clone(),
            version: contract.version.clone(),
            name: contract.name.clone(),
            description: contract.description.clone(),
            priority: contract.priority.clone(),
            status: contract.status.clone(),
            domain: contract.domain.clone(),
            tags: contract.tags.clone(),
            trigger_type: contract.trigger.as_ref().and_then(|t| t.kind.clone()),
            matched_files: matched,
            contract,
            file_contents,
        });
    }

    let total = entries.len();
    serde_json::to_string(&Response {
        affected_contracts: entries,
        total,
        diff: params.diff,
        warnings,
    })
    .unwrap()
}
