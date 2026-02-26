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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_contracts_dir_is_contracts_slash() {
        let config = Config::default();
        assert_eq!(config.contracts_dir, "contracts/");
    }

    #[test]
    fn default_instructions_is_nonempty() {
        let config = Config::default();
        assert!(!config.instructions().is_empty());
        assert_eq!(config.instructions(), DEFAULT_INSTRUCTIONS);
    }

    #[test]
    fn instructions_override_returned_when_set() {
        let config = Config {
            contracts_dir: "contracts/".to_string(),
            instructions: Some("custom instructions".to_string()),
        };
        assert_eq!(config.instructions(), "custom instructions");
    }

    #[test]
    fn parse_valid_config_toml() {
        let content = r#"contracts_dir = "custom/path/""#;
        let config: Config = toml::from_str(content).unwrap();
        assert_eq!(config.contracts_dir, "custom/path/");
    }

    #[test]
    fn parse_unknown_field_fails() {
        let content = r#"unknown_field = "value""#;
        let result: Result<Config, _> = toml::from_str(content);
        assert!(result.is_err(), "Unknown fields must be rejected (deny_unknown_fields)");
    }

    #[test]
    fn parse_malformed_toml_fails() {
        let content = "not valid toml ===";
        let result: Result<Config, _> = toml::from_str(content);
        assert!(result.is_err());
    }

    #[test]
    fn missing_contracts_dir_field_defaults_to_contracts_slash() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.contracts_dir, "contracts/");
    }
}
