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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;

    fn make_server(contracts_dir: &str) -> super::super::CddServer {
        super::super::CddServer::new(Config {
            contracts_dir: contracts_dir.to_string(),
            instructions: None,
        })
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cdd_list_test_{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_contract(dir: &std::path::Path, id: &str, domain: Option<&str>, tags: &[&str]) {
        let domain_line = domain
            .map(|d| format!("domain = \"{d}\"\n"))
            .unwrap_or_default();
        let tags_line = if tags.is_empty() {
            String::new()
        } else {
            let list = tags
                .iter()
                .map(|t| format!("\"{}\"", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("tags = [{list}]\n")
        };
        let content = format!(
            "id = \"{id}\"\nversion = \"1.0.0\"\nname = \"{id}\"\ndescription = \"desc\"\n{domain_line}{tags_line}"
        );
        fs::write(dir.join(format!("{id}.contract.toml")), content).unwrap();
    }

    #[tokio::test]
    async fn returns_all_when_no_filter() {
        let dir = temp_dir("all");
        write_contract(&dir, "contract-a", Some("core"), &["tag1"]);
        write_contract(&dir, "contract-b", Some("tools"), &["tag2"]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(&server, Params { domain: None, tags: None }).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 2);
    }

    #[tokio::test]
    async fn filters_by_domain_exact_match() {
        let dir = temp_dir("domain");
        write_contract(&dir, "contract-a", Some("core"), &[]);
        write_contract(&dir, "contract-b", Some("tools"), &[]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(
            &server,
            Params {
                domain: Some("core".to_string()),
                tags: None,
            },
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["contracts"][0]["id"], "contract-a");
    }

    #[tokio::test]
    async fn domain_filter_is_case_sensitive() {
        let dir = temp_dir("case_sensitive");
        write_contract(&dir, "contract-a", Some("Core"), &[]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(
            &server,
            Params {
                domain: Some("core".to_string()),
                tags: None,
            },
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 0, "Domain filter must be case-sensitive");
    }

    #[tokio::test]
    async fn filters_by_tags_uses_or_logic() {
        let dir = temp_dir("tags_or");
        write_contract(&dir, "contract-a", None, &["alpha", "beta"]);
        write_contract(&dir, "contract-b", None, &["gamma"]);
        write_contract(&dir, "contract-c", None, &["delta"]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(
            &server,
            Params {
                domain: None,
                tags: Some(vec!["alpha".to_string(), "gamma".to_string()]),
            },
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 2, "Tag filter must use OR logic");
    }

    #[tokio::test]
    async fn combined_domain_and_tags_both_must_match() {
        let dir = temp_dir("combined");
        write_contract(&dir, "contract-a", Some("core"), &["mcp"]);
        write_contract(&dir, "contract-b", Some("tools"), &["mcp"]);
        let server = make_server(dir.to_str().unwrap());
        let result = handle(
            &server,
            Params {
                domain: Some("core".to_string()),
                tags: Some(vec!["mcp".to_string()]),
            },
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["total"], 1, "Combined filters require AND logic");
        assert_eq!(json["contracts"][0]["id"], "contract-a");
    }
}
