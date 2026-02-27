mod create_contract;
mod delete_contract;
mod get_notes;
mod loader;
mod get_affected_contracts;
mod get_contract;
mod list_contracts;
mod review_changeset;
mod update_contract;
mod validate_all_contracts;
mod validate_contract;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};
use std::collections::HashSet;
use std::sync::Mutex;

use crate::config::Config;

#[derive(Default)]
struct SessionState {
    /// True once sigil_list_contracts or sigil_get_affected_contracts has been called.
    listed: bool,
    /// Contract ids for which sigil_get_contract has been called in this session.
    read_ids: HashSet<String>,
}

pub struct SigilServer {
    pub tool_router: ToolRouter<SigilServer>,
    pub config: Config,
    session: Mutex<SessionState>,
}

#[tool_handler]
impl ServerHandler for SigilServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(self.config.instructions().to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub(super) fn error_response(msg: impl std::fmt::Display) -> String {
    serde_json::json!({"error": msg.to_string()}).to_string()
}

impl SigilServer {
    pub(super) fn require_listed(&self, tool: &str, contract_id: &str) -> Result<(), String> {
        if !self.session.lock().unwrap().listed {
            Err(error_response(format!(
                "You must call sigil_list_contracts or sigil_get_affected_contracts before calling {tool} for '{contract_id}'."
            )))
        } else {
            Ok(())
        }
    }

    pub(super) fn mark_listed(&self) {
        self.session.lock().unwrap().listed = true;
    }

    pub(super) fn require_read(&self, tool: &str, contract_id: &str) -> Result<(), String> {
        if !self.session.lock().unwrap().read_ids.contains(contract_id) {
            Err(error_response(format!(
                "You must call sigil_get_contract for '{contract_id}' before calling {tool}."
            )))
        } else {
            Ok(())
        }
    }

    pub(super) fn mark_read(&self, contract_id: &str) {
        self.session
            .lock()
            .unwrap()
            .read_ids
            .insert(contract_id.to_string());
    }
}

#[tool_router]
impl SigilServer {
    pub fn new(config: Config) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config,
            session: Mutex::new(SessionState::default()),
        }
    }

    #[tool(description = "Return global project notes from the config file. Notes contain project-specific conventions and context that apply across all contracts.")]
    async fn sigil_get_notes(
        &self,
        Parameters(params): Parameters<get_notes::Params>,
    ) -> String {
        get_notes::handle(self, params).await
    }

    #[tool(description = "List all contracts with summary info. Starting point for planning. Supports optional filtering by domain and/or tags. Call this before sigil_get_contract.")]
    async fn sigil_list_contracts(
        &self,
        Parameters(params): Parameters<list_contracts::Params>,
    ) -> String {
        list_contracts::handle(self, params).await
    }

    #[tool(description = "Retrieve a single contract by id with full detail. When retrieve_file_contents is true, includes the file contents of all files referenced in the contract. Requires a prior sigil_list_contracts or sigil_get_affected_contracts call in the current session.")]
    async fn sigil_get_contract(
        &self,
        Parameters(params): Parameters<get_contract::Params>,
    ) -> String {
        get_contract::handle(self, params).await
    }

    #[tool(description = "Given a list of file paths, return all contracts that care about those files via files, applies_to glob patterns, or matching rules. Use this during planning to understand contract implications of a change.")]
    async fn sigil_get_affected_contracts(
        &self,
        Parameters(params): Parameters<get_affected_contracts::Params>,
    ) -> String {
        get_affected_contracts::handle(self, params).await
    }

    #[tool(description = "Validate a contract: checks schema compliance, missing files, and structural correctness. Returns pass/fail with categorized errors and warnings.")]
    async fn sigil_validate_contract(
        &self,
        Parameters(params): Parameters<validate_contract::Params>,
    ) -> String {
        validate_contract::handle(self, params).await
    }

    #[tool(description = "Create a new contract file. Validates the contract against the schema before writing. Derives the filename from the contract id field. Fails if a contract with that id already exists.")]
    async fn sigil_create_contract(
        &self,
        Parameters(params): Parameters<create_contract::Params>,
    ) -> String {
        create_contract::handle(self, params).await
    }

    #[tool(description = "Apply partial updates to an existing contract. Unspecified fields are preserved. List fields are replaced wholesale. Returns a diff of what changed. Requires a prior sigil_get_contract call for this contract_id in the current session.")]
    async fn sigil_update_contract(
        &self,
        Parameters(params): Parameters<update_contract::Params>,
    ) -> String {
        update_contract::handle(self, params).await
    }

    #[tool(description = "Delete a contract. Requires a prior sigil_get_contract call for this contract_id in the current session.")]
    async fn sigil_delete_contract(
        &self,
        Parameters(params): Parameters<delete_contract::Params>,
    ) -> String {
        delete_contract::handle(self, params).await
    }

    #[tool(description = "Fast validation of all contracts: checks missing files and schema validation errors. Returns pass/fail boolean plus categorized errors and warnings.")]
    async fn sigil_validate_all_contracts(
        &self,
        Parameters(params): Parameters<validate_all_contracts::Params>,
    ) -> String {
        validate_all_contracts::handle(self, params).await
    }

    #[tool(description = "Bundle context for a changeset review. Given changed files and optional diff, returns affected contracts with full context (contract content and file contents). The agent then performs the semantic review and produces verdicts.")]
    async fn sigil_review_changeset(
        &self,
        Parameters(params): Parameters<review_changeset::Params>,
    ) -> String {
        review_changeset::handle(self, params).await
    }
}
