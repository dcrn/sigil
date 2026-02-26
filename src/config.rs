use anyhow::{Context, Result};
use serde::Deserialize;

/// Default agent instructions embedded at compile time from docs/agent-instructions.md.
pub const DEFAULT_INSTRUCTIONS: &str =
    include_str!("../docs/agent-instructions.md");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Directory containing contract TOML files.
    #[serde(default = "default_contracts_dir")]
    pub contracts_dir: String,

    /// Override the agent instructions delivered via MCP ServerInfo.
    /// When absent, the instructions compiled into the binary are used.
    pub instructions: Option<String>,
}

impl Config {
    /// Load from `cdd.config.toml` in the current directory.
    /// Returns default config if the file is missing; errors if it is malformed.
    pub fn load() -> Result<Self> {
        let path = "cdd.config.toml";
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content)
                .with_context(|| format!("Failed to parse {path}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Self::default())
            }
            Err(e) => Err(e).with_context(|| format!("Failed to read {path}")),
        }
    }

    /// Returns the instructions to deliver to agents: config override if set,
    /// otherwise the compile-time default.
    pub fn instructions(&self) -> &str {
        self.instructions
            .as_deref()
            .unwrap_or(DEFAULT_INSTRUCTIONS)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            contracts_dir: default_contracts_dir(),
            instructions: None,
        }
    }
}

fn default_contracts_dir() -> String {
    "contracts/".to_string()
}
