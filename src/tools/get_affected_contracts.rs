use crate::model::{Priority, Status};
use globset::Glob;
use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// File paths to check against contract files and applies_to glob patterns.
    pub files: Vec<String>,
}

#[derive(Serialize)]
struct Response {
    contracts: Vec<AffectedSummary>,
    total: usize,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct AffectedSummary {
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
    matched_files: MatchedFiles,
}

#[derive(Serialize)]
struct MatchedFiles {
    direct: Vec<String>,
    applies_to: Vec<AppliesMatch>,
}

#[derive(Serialize)]
struct AppliesMatch {
    pattern: String,
    matched_files: Vec<String>,
}

pub async fn handle(server: &super::CddServer, params: Params) -> String {
    let (contracts, mut warnings) = super::loader::load_contracts(&server.config.contracts_dir);
    server.mark_listed();

    // Normalize input files (forward slashes)
    let files: Vec<String> = params.files.iter().map(|f| f.replace('\\', "/")).collect();

    let mut summaries = Vec::new();

    for contract in &contracts {
        let contract_files: Vec<&str> = contract.all_files();

        // Direct matches
        let direct: Vec<String> = files
            .iter()
            .filter(|f| contract_files.contains(&f.as_str()))
            .cloned()
            .collect();

        // Glob (applies_to) matches
        let mut applies_to_matches = Vec::new();
        for pattern in contract.applies_to_patterns() {
            match Glob::new(pattern) {
                Ok(glob) => {
                    let matcher = glob.compile_matcher();
                    let matched_files: Vec<String> = files
                        .iter()
                        .filter(|f| matcher.is_match(f.as_str()))
                        .cloned()
                        .collect();
                    if !matched_files.is_empty() {
                        applies_to_matches.push(AppliesMatch {
                            pattern: pattern.to_string(),
                            matched_files,
                        });
                    }
                }
                Err(e) => {
                    warnings.push(format!(
                        "Contract '{}': invalid applies_to pattern '{}': {}",
                        contract.id, pattern, e
                    ));
                }
            }
        }

        if direct.is_empty() && applies_to_matches.is_empty() {
            continue;
        }

        let file_count = contract.all_files().len();

        summaries.push(AffectedSummary {
            id: contract.id.clone(),
            version: contract.version.clone(),
            name: contract.name.clone(),
            description: contract.description.clone(),
            priority: contract.priority.clone(),
            status: contract.status.clone(),
            domain: contract.domain.clone(),
            tags: contract.tags.clone(),
            trigger_type: contract.trigger.as_ref().and_then(|t| t.kind.clone()),
            file_count,
            matched_files: MatchedFiles {
                direct,
                applies_to: applies_to_matches,
            },
        });
    }

    let total = summaries.len();
    serde_json::to_string(&Response {
        contracts: summaries,
        total,
        warnings,
    })
    .unwrap()
}
