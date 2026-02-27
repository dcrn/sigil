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

pub async fn handle(server: &super::SigilServer, params: Params) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;

    fn make_server(contracts_dir: &str) -> super::super::SigilServer {
        super::super::SigilServer::new(Config {
            contracts_dir: contracts_dir.to_string(),
            instructions: None,
            notes: None,
        })
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sigil_affected_test_{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_with_files(dir: &std::path::Path, id: &str, files: &[&str]) {
        let list = files
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(", ");
        let files_line = if files.is_empty() {
            String::new()
        } else {
            format!("files = [{list}]\n")
        };
        let content = format!(
            "id = \"{id}\"\nversion = \"1.0.0\"\nname = \"{id}\"\ndescription = \"desc\"\n{files_line}"
        );
        fs::write(dir.join(format!("{id}.contract.toml")), content).unwrap();
    }

    fn write_with_applies_to(dir: &std::path::Path, id: &str, pattern: &str) {
        let content = format!(
            "id = \"{id}\"\nversion = \"1.0.0\"\nname = \"{id}\"\ndescription = \"desc\"\napplies_to = \"{pattern}\"\n"
        );
        fs::write(dir.join(format!("{id}.contract.toml")), content).unwrap();
    }

    #[tokio::test]
    async fn no_match_returns_empty() {
        let dir = temp_dir("no_match");
        write_with_files(&dir, "contract-a", &["src/foo.rs"]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(&server, Params { files: vec!["src/bar.rs".to_string()] }).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 0);
    }

    #[tokio::test]
    async fn matches_by_direct_file_path() {
        let dir = temp_dir("direct");
        write_with_files(&dir, "contract-a", &["src/foo.rs", "src/bar.rs"]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(&server, Params { files: vec!["src/foo.rs".to_string()] }).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["contracts"][0]["id"], "contract-a");
        let direct = &json["contracts"][0]["matched_files"]["direct"];
        assert!(direct.as_array().unwrap().contains(&serde_json::json!("src/foo.rs")));
    }

    #[tokio::test]
    async fn direct_match_is_exact_path_no_glob_expansion() {
        let dir = temp_dir("exact");
        write_with_files(&dir, "contract-a", &["src/foo.rs"]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(&server, Params { files: vec!["src/foo".to_string()] }).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 0, "Direct match must be exact path comparison");
    }

    #[tokio::test]
    async fn matches_by_applies_to_glob() {
        let dir = temp_dir("glob");
        write_with_applies_to(&dir, "contract-a", "src/**/*.rs");
        let server = make_server(dir.to_str().unwrap());
        let result = handle(&server, Params { files: vec!["src/tools/mod.rs".to_string()] }).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 1);
        let applies = &json["contracts"][0]["matched_files"]["applies_to"];
        assert!(!applies.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn wildcard_applies_to_matches_any_file() {
        let dir = temp_dir("wildcard");
        write_with_applies_to(&dir, "global-contract", "**");
        let server = make_server(dir.to_str().unwrap());
        let result = handle(&server, Params { files: vec!["anything/at/all.txt".to_string()] }).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 1);
    }
}
