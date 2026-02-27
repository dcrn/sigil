use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {}

#[derive(Serialize)]
struct Response {
    notes: Option<String>,
}

pub async fn handle(server: &super::CddServer, _params: Params) -> String {
    serde_json::to_string(&Response {
        notes: server.config.notes.clone(),
    })
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tools::CddServer;

    fn server_with_notes(notes: Option<&str>) -> CddServer {
        CddServer::new(Config {
            notes: notes.map(str::to_string),
            ..Config::default()
        })
    }

    #[tokio::test]
    async fn returns_notes_when_present() {
        let server = server_with_notes(Some("Use snake_case for all identifiers."));
        let result = handle(&server, Params {}).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["notes"], "Use snake_case for all identifiers.");
    }

    #[tokio::test]
    async fn returns_null_when_notes_absent() {
        let server = server_with_notes(None);
        let result = handle(&server, Params {}).await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(json["notes"].is_null(), "notes must be null when not configured");
    }

    #[tokio::test]
    async fn response_shape_is_consistent() {
        for notes in [Some("some notes"), None] {
            let server = server_with_notes(notes);
            let result = handle(&server, Params {}).await;
            let json: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert!(json.get("notes").is_some(), "notes field must always be present");
        }
    }
}
